mod base64;
mod compress;
mod config;
mod connector;
mod directory;
mod logger;
mod matcher;
mod mime;
mod process;
mod util;
mod var;
mod yaml;

use ace::App;
use bright::Colorful;
use compress::encoding::Encoding;
use config::default;
use config::{Directory, Proxy, RewriteStatus, ServerConfig, Setting, SiteConfig, StatusPage};
use config::{ForceTo, GetExtension, ToAbsolutePath};
use connector::Connector;
use dirs;
use futures::io;
use futures::StreamExt;
use hyper::body::Bytes;
use hyper::header::{
    HeaderName, HeaderValue, ACCEPT_ENCODING, ALLOW, AUTHORIZATION, CONTENT_ENCODING,
    CONTENT_LENGTH, CONTENT_TYPE, HOST, LOCATION, SERVER, WWW_AUTHENTICATE,
};
use hyper::http::response::Builder;
use hyper::server::accept::from_stream;
use hyper::server::conn::Http;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, HeaderMap, Method, Request, Response, StatusCode, Uri, Version};
use lazy_static::lazy_static;
use matcher::HostMatcher;
use process::ExitError;
use rand::prelude::*;
use regex::Regex;
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::task::{Context, Poll};
use tokio::fs::{self, File};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio_rustls::server::TlsStream;
use var::{ReplaceVar, Var};

#[tokio::main]
async fn main() {
    let mut app = App::new()
        .config(default::SERVER_NAME, default::VERSION)
        .cmd("start", "Quick start in the current directory")
        .cmd("stop", "Stop the daemon")
        .cmd("restart", "Restart the service program")
        .cmd("help", "Print help information")
        .cmd("version", "Print version information")
        .opt("-c", "Specify a configuration file")
        .opt("-d", "Running in the background")
        .opt("-t", "Test the config file for error");

    let mut configs = vec![];

    if let Some(cmd) = app.command() {
        match cmd.as_str() {
            "start" => {
                let arg = app.value("start").unwrap();
                let listen = match arg.is_empty() {
                    true => default::START_PORT.to_string(),
                    false => arg[0].clone(),
                };
                let addr = listen.as_str().to_socket_addr();
                let root = env::current_dir()
                    .unwrap_or_else(|err| exit!("Can't get working directory\n{:?}", err));

                configs = vec![default::quick_start_config(root, addr)];
            }
            "stop" => {
                return stop_daemon();
            }
            "restart" => exit!("Waiting for development"),
            "help" => {
                return app.print_help();
            }
            "version" => {
                return app.print_version();
            }
            _ => {
                return app.print_error_try("help");
            }
        }
    }

    if !app.is("start") {
        let config_path = match app.value("-c") {
            Some(values) => {
                if values.len() != 1 {
                    exit!("-c value: [CONFIG_FILE]");
                }
                values[0].clone()
            }
            None => {
                let config = home_dir()
                    .join(default::CONFIG_PATH[0])
                    .join(default::CONFIG_PATH[1]);
                if let Some(p) = config.parent() {
                    let _ = std::fs::create_dir_all(p);
                }
                config.display().to_string()
            }
        };

        configs = ServerConfig::new(&config_path).await;

        // Check configuration file
        if app.value("-t").is_some() {
            return println!("The configuration file '{}' syntax is ok", config_path);
        }
    }

    if app.value("-d").is_some() {
        return start_daemon("-d");
    }

    if app.is("start") {
        quick_start_info(configs[0].listen, configs[0].sites[0].root.clone());
    }

    bind_tcp(configs).await;
}

fn home_dir() -> PathBuf {
    match dirs::home_dir() {
        Some(home) => home,
        None => exit!("Can't get home directory"),
    }
}

fn quick_start_info(listen: SocketAddr, path: PathBuf) {
    println!(
        "Serving path   : {}",
        path.display().to_string().yellow().bold()
    );
    let port = match listen.port() {
        80 => String::with_capacity(0),
        _ => format!(":{}", listen.port()),
    };
    println!(
        "Serving address: {}\x1b[0m",
        format!("http://{}{}", listen.ip(), port).green().bold()
    );
}

fn start_daemon(detach: &str) {
    use std::fs;
    let args = env::args().collect::<Vec<String>>();
    let cmd = args
        .iter()
        .filter(|item| item != &detach)
        .cloned()
        .collect::<Vec<String>>();

    let child = Command::new(&cmd[0]).args(&cmd[1..]).spawn();
    match child {
        Ok(child) => {
            use std::io::Write;
            let home = home_dir();
            let _ = fs::create_dir_all(home.join(default::PID_PATH[0]));
            let p = home.join(default::PID_PATH[0]).join(default::PID_PATH[1]);
            let mut pid = fs::File::create(p).unwrap();
            write!(pid, "{}", child.id()).unwrap();
        }
        Err(err) => exit!("Failed to run in the background\n{:?}", err),
    }
}

fn stop_daemon() {
    let pid_path = home_dir()
        .join(default::PID_PATH[0])
        .join(default::PID_PATH[1]);
    match std::fs::read_to_string(&pid_path) {
        Ok(pid) => {
            if let Err(err) = std::fs::remove_file(&pid_path) {
                exit!("Cannot delete pid file\n{:?}", err)
            }

            let pid = match pid.parse::<i32>() {
                Ok(pid) => pid,
                Err(_) => exit!("Cannot parse pid '{}'", pid),
            };

            if let Err(err) = process::kill(pid) {
                match err {
                    ExitError::None => exit!("Process does not exist"),
                    ExitError::Failure => exit!("Can't kill the process"),
                }
            }
        }
        Err(e) => {
            exit!("Open {:?} failed\n{:?}", pid_path, e);
        }
    }
}

enum Req {
    Stream(TcpStream, Vec<SiteConfig>),
    TlsStream(TlsStream<TcpStream>, Vec<SiteConfig>),
}

impl AsyncRead for Req {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Req::Stream(stream, _) => Pin::new(stream).poll_read(cx, buf),
            Req::TlsStream(stream, _) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Req {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Req::Stream(stream, _) => Pin::new(stream).poll_write(cx, buf),
            Req::TlsStream(stream, _) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Req::Stream(stream, _) => Pin::new(stream).poll_flush(cx),
            Req::TlsStream(stream, _) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Req::Stream(stream, _) => Pin::new(stream).poll_shutdown(cx),
            Req::TlsStream(stream, _) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

async fn bind_tcp(configs: Vec<ServerConfig>) {
    //    let mut servers = vec![];

    for config in configs {
        let listen = &config.listen;

        let mut listener = match TcpListener::bind(listen).await {
            Ok(listener) => listener,
            Err(err) => exit!("Cannot bind to address: {}\n{:?}", listen, err),
        };

        let stream = listener.incoming().filter_map(|stream| {
            let config = config.clone();
            async move {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => return None,
                };

                let has_https = config.sites.iter().any(|item| item.https.is_some());
                if !has_https {
                    return Some(Ok::<_, hyper::Error>(Req::Stream(stream, config.sites)));
                }
                let mut buf = [0; 1];
                let is_https = match stream.peek(&mut buf).await {
                    Ok(_) => buf[0] == 22,
                    Err(_) => return None,
                };
                if is_https {
                    // bug
                    let configs = config
                        .sites
                        .iter()
                        .filter(|item| item.https.is_some())
                        .cloned()
                        .collect::<Vec<SiteConfig>>();
                    let config = configs[0].clone();
                    let tls = config.https.as_ref().unwrap();
                    // can accept
                    if let Ok(stream) = tls.acceptor.accept(stream).await {
                        return Some(Ok::<_, hyper::Error>(Req::TlsStream(stream, configs)));
                    }
                } else {
                    let mut s = vec![];
                    for item in config.sites {
                        if item.https.is_none() {
                            s.push(item);
                        }
                    }
                    return Some(Ok::<_, hyper::Error>(Req::Stream(stream, s)));
                }
                None
            }
        });

        let http = Http::new();

        let service = make_service_fn(|req: &Req| {
            let config = match req {
                Req::Stream(_, c) => c.clone(),
                Req::TlsStream(_, c) => c.clone(),
            };
            async { Ok::<_, Infallible>(service_fn(move |req| connect(req, config.clone()))) }
        });

        let server = hyper::server::Builder::new(from_stream(stream), http)
            .serve(service)
            .await;

        //        servers.push(server);
    }
}

trait BuilderFromStatus {
    fn from_status(self, status: StatusCode) -> Response<Body>;
}
impl BuilderFromStatus for Builder {
    fn from_status(self, status: StatusCode) -> Response<Body> {
        self.status(status)
            .header(CONTENT_TYPE, mime::TEXT_PLAIN)
            .body(Body::from(status.to_string()))
            .unwrap()
    }
}

async fn connect(
    req: hyper::Request<Body>,
    configs: Vec<SiteConfig>,
) -> hyper::Result<Response<Body>> {
    response(req, configs).await.map(|mut res| {
        res.headers_mut()
            .insert(SERVER, HeaderValue::from_static(default::SERVER_NAME));
        res
    })
}

lazy_static! {
    static ref REGEX_PORT: Regex = Regex::new(r"(:\d+)$").unwrap();
}

async fn response(
    req: hyper::Request<Body>,
    configs: Vec<SiteConfig>,
) -> hyper::Result<Response<Body>> {
    if let Some(host) = req.headers().get(HOST) {
        let host = host.to_str().unwrap();
        let reg: &Regex = &REGEX_PORT;
        let host = &reg.replacen(host, 1, "").to_string();

        for config in configs.iter() {
            if config.host.is_match(host) {
                let mut config = config.clone();
                return Ok(handle(req, &mut config)
                    .await
                    .append_headers(config.headers));
            }
        }
    } else if req.version() == Version::HTTP_11 {
        // A Host header field must be sent in all HTTP/1.1 request messages
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Host

        return Ok(Response::builder().from_status(StatusCode::BAD_REQUEST));
    }

    for config in configs {
        if config.host.is_empty() {
            let mut config = config;
            return Ok(handle(req, &mut config)
                .await
                .append_headers(config.headers));
        }
    }

    Ok(Response::builder().from_status(StatusCode::FORBIDDEN))
}

trait AppendHeaders {
    fn append_headers(self, headers: Setting<HeaderMap>) -> Response<Body>;
}

impl AppendHeaders for Response<Body> {
    fn append_headers(mut self, headers: Setting<HeaderMap>) -> Response<Body> {
        if let Setting::Value(headers) = headers {
            self.headers_mut().extend(headers);
        }
        self
    }
}

fn rand_uri(uri: Vec<Var<Uri>>) -> Var<Uri> {
    if uri.len() == 1 {
        return uri[0].clone();
    } else {
        let i = rand::thread_rng().gen_range(0, uri.len());
        return uri[i].clone();
    }
}

async fn proxy_response(mut req: Request<Body>, c: Proxy, config: &SiteConfig) -> Response<Body> {
    let encoding = req.headers().get(ACCEPT_ENCODING).map(|d| d.clone());

    let uri = rand_uri(c.uri).unwrap_or_else(|s, r| {
        let result = s.as_str().replace_var(&r, &req);
        result.parse::<Uri>().unwrap()
    });

    *req.uri_mut() = uri;
    if let Some(method) = c.method {
        *req.method_mut() = method;
    }
    if let Setting::Value(headers) = c.headers {
        req.headers_mut().extend(headers);
    }

    let is_https = req.uri().scheme_str() == Some("https");
    let client = Client::builder().build::<_, Body>(Connector::new(is_https));

    match client.request(req).await {
        Ok(res) => res,
        Err(_) => {
            // 502
            output_error(encoding.as_ref(), &config, StatusCode::BAD_GATEWAY).await
        }
    }
}

async fn handle(req: Request<Body>, config: &mut SiteConfig) -> Response<Body> {
    // Merge location to config
    if !config.location.is_empty() {
        merge_location(req.uri().path(), config);
    }

    if let Setting::Value(log) = &mut config.log {
        log.logger.write(req.uri().path()).await;
    }

    if let Setting::Value(ip) = &config.ip {
        // if !ip.is_pass() {
        //     // 403
        //     return
        // }
    }

    // HTTP auth
    if let Setting::Value(auth) = &config.auth {
        let authorization = req.headers().get(AUTHORIZATION);
        const MESSAGE: &str = "Basic realm=\"User Visible Realm\"";
        if let Some(value) = authorization {
            if value != auth {
                return Response::builder()
                    .header(WWW_AUTHENTICATE, MESSAGE)
                    .from_status(StatusCode::UNAUTHORIZED);
            }
        } else {
            return Response::builder()
                .header(WWW_AUTHENTICATE, MESSAGE)
                .from_status(StatusCode::UNAUTHORIZED);
        }
    }

    // Proxy request
    if let Setting::Value(proxy) = &config.proxy {
        return proxy_response(req, proxy.clone(), &config).await;
    }

    // Not allowed method
    let allow = match &config.methods {
        Setting::Value(methods) => methods.iter().any(|m| m == req.method()),
        _ => false,
    };
    if !allow {
        if req.method() == Method::OPTIONS {
            let methods = match &config.methods {
                Setting::Value(values) => values
                    .iter()
                    .map(|m| m.as_str())
                    .collect::<Vec<&str>>()
                    .join(", "),
                _ => String::with_capacity(0),
            };
            return Response::builder()
                .header(ALLOW, methods)
                .from_status(StatusCode::METHOD_NOT_ALLOWED);
        } else {
            return Response::builder().from_status(StatusCode::METHOD_NOT_ALLOWED);
        }
    }

    // echo
    if let Setting::Value(echo) = &config.echo {
        let echo = echo
            .clone()
            .unwrap_or_else(|s, r| s.as_str().replace_var(&r, &req));

        return Response::builder()
            .header(CONTENT_TYPE, mime::TEXT_PLAIN)
            .body(Body::from(echo))
            .unwrap();
    }

    let cur_path = String::from(".") + req.uri().path();
    let mut path = Path::new(&config.root).join(&cur_path);

    // rewrite
    if let Setting::Value(rewrite) = &config.rewrite {
        let value = rewrite.location.clone().unwrap_or_else(|s, r| {
            let result = s.as_str().replace_var(&r, &req);
            HeaderValue::from_str(&result).unwrap()
        });

        let status = match rewrite.status {
            RewriteStatus::_301 => StatusCode::MOVED_PERMANENTLY,
            RewriteStatus::_302 => StatusCode::FOUND,
        };
        return Response::builder()
            .status(status)
            .header(LOCATION, value)
            .body(Body::empty())
            .unwrap();
    }

    // file
    if let Setting::Value(file) = &config.file {
        path = file.clone();
    }

    match fs::metadata(&path).await {
        Ok(meta) => {
            if meta.is_dir() {
                if req.uri().path().chars().last().unwrap_or('.') == '/' {
                    if let Setting::Value(option) = &config.directory {
                        let html = directory::render_dir_html(
                            &path,
                            &req.uri().path(),
                            &option.time,
                            option.size,
                        )
                        .await;
                        return match html {
                            Ok(html) => Response::builder()
                                .header(CONTENT_TYPE, mime::TEXT_HTML)
                                .body(Body::from(html))
                                .unwrap(),
                            Err(_) => {
                                output_error(
                                    req.headers().get(ACCEPT_ENCODING),
                                    &config,
                                    StatusCode::FORBIDDEN,
                                )
                                .await
                            }
                        };
                    }
                    if let Setting::Value(index) = &config.index {
                        if !index.is_empty() {
                            match index_back(&path, &index).await {
                                Some((file, ext)) => {
                                    return file_response(
                                        StatusCode::OK,
                                        file,
                                        Some(&ext),
                                        req.headers().get(ACCEPT_ENCODING),
                                        &config,
                                    )
                                    .await
                                }
                                None => {
                                    return output_error(
                                        req.headers().get(ACCEPT_ENCODING),
                                        &config,
                                        StatusCode::NOT_FOUND,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    return output_error(
                        req.headers().get(ACCEPT_ENCODING),
                        &config,
                        StatusCode::NOT_FOUND,
                    )
                    .await;
                } else {
                    let aims;
                    if let Some(query) = req.uri().query() {
                        aims = format!("{}/{}", req.uri().path(), query);
                    } else {
                        aims = format!("{}/", req.uri().path());
                    }
                    return Response::builder()
                        .status(StatusCode::MOVED_PERMANENTLY)
                        .header(LOCATION, aims)
                        .body(Body::empty())
                        .unwrap();
                }
            } else {
                match File::open(&path).await {
                    Ok(file) => {
                        return file_response(
                            StatusCode::OK,
                            file,
                            path.get_extension(),
                            req.headers().get(ACCEPT_ENCODING),
                            &config,
                        )
                        .await;
                    }
                    Err(_) => {
                        return output_error(
                            req.headers().get(ACCEPT_ENCODING),
                            &config,
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                        .await;
                    }
                }
            }
        }
        Err(_) => match fallbacks(&path, &config.extensions).await {
            Some((file, ext)) => {
                return file_response(
                    StatusCode::OK,
                    file,
                    Some(&ext),
                    req.headers().get(ACCEPT_ENCODING),
                    &config,
                )
                .await
            }
            None => {
                return output_error(
                    req.headers().get(ACCEPT_ENCODING),
                    &config,
                    StatusCode::NOT_FOUND,
                )
                .await;
            }
        },
    };
}

fn merge_location(route: &str, config: &mut SiteConfig) {
    for item in &config.location {
        if !item.location.is_match(route) {
            continue;
        }
        if let Some(root) = &item.root {
            config.root = root.clone();
        }
        if !item.echo.is_none() {
            config.echo = item.echo.clone();
        }
        if !item.file.is_none() {
            config.file = item.file.clone();
        }
        if !item.index.is_none() {
            config.index = item.index.clone();
        }
        if !item.directory.is_none() {
            config.directory = item.directory.clone();
        }
        if !item.headers.is_none() {
            if let Setting::Value(header) = &item.headers {
                let mut h = config.headers.clone().unwrap_or_default();
                h.extend(header.clone());
                config.headers = Setting::Value(h);
            } else if item.headers.is_off() {
                config.headers = Setting::Off;
            }
        }
        if !item.rewrite.is_none() {
            config.rewrite = item.rewrite.clone();
        }
        if !item.compress.is_none() {
            config.compress = item.compress.clone();
        }
        if !item.methods.is_none() {
            config.methods = item.methods.clone();
        }
        if !item.auth.is_none() {
            config.auth = item.auth.clone();
        }
        if !item.extensions.is_none() {
            config.extensions = item.extensions.clone();
        }
        if !item.proxy.is_none() {
            config.proxy = item.proxy.clone();
        }
        if !item.log.is_none() {
            config.log = item.log.clone();
        }
        if !item.ip.is_none() {
            config.ip = item.ip.clone();
        }
        if !item.status._403.is_none() {
            config.status._403 = item.status._403.clone();
        }
        if !item.status._404.is_none() {
            config.status._404 = item.status._404.clone();
        }
        if !item.status._500.is_none() {
            config.status._500 = item.status._500.clone();
        }
        if !item.status._502.is_none() {
            config.status._502 = item.status._502.clone();
        }

        if item._break {
            break;
        }
    }
    config.location.clear();
}

fn compress(encoding: Option<&HeaderValue>, config: &SiteConfig, ext: &str) -> Encoding {
    if let Setting::Value(compress) = &config.compress {
        if compress.extensions.iter().any(|item| *item == ext) {
            // gzip, deflate, br
            if let Some(encoding) = encoding {
                let a = encoding.to_str().unwrap();
                let modes: Vec<&str> = a.split(", ").collect();
                return compress.mode.parse_mode(modes);
            }
        }
    }
    Encoding::None
}

async fn output_error(
    encoding: Option<&HeaderValue>,
    config: &SiteConfig,
    status: StatusCode,
) -> Response<Body> {
    let path = match status {
        StatusCode::FORBIDDEN => &config.status._403,
        StatusCode::NOT_FOUND => &config.status._404,
        StatusCode::BAD_GATEWAY => &config.status._502,
        _ => &config.status._500,
    };

    if let Setting::Value(path) = path {
        if path.is_file() {
            if let Ok(f) = File::open(&path).await {
                return file_response(status, f, path.get_extension(), encoding, &config).await;
            }
        }
    }

    Response::builder().from_status(status)
}

async fn file_response(
    status: StatusCode,
    mut file: File,
    ext: Option<&str>,
    header: Option<&HeaderValue>,
    config: &SiteConfig,
) -> Response<Body> {
    let encoding = ext
        .map(|ext| compress(header, config, ext))
        .unwrap_or(Encoding::None);

    let (mut sender, body) = Body::channel();
    let mime = ext
        .map(|e| mime::from_extension(e))
        .unwrap_or(mime::DEFAULT);
    let mut res = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, mime)
        .body(body)
        .unwrap();

    if encoding == Encoding::None {
        if let Ok(meta) = file.metadata().await {
            res.headers_mut()
                .insert(CONTENT_LENGTH, HeaderValue::from(meta.len()));
        }
    } else {
        res.headers_mut()
            .insert(CONTENT_ENCODING, encoding.to_header_value());
    }

    tokio::spawn(async move {
        loop {
            let mut buf = [0; default::BUF_SIZE];
            match file.read(&mut buf).await {
                Ok(len) => {
                    if len == 0 {
                        break;
                    }
                    let data = match &encoding {
                        Encoding::Gzip(level) => compress::encode::gzip(&buf[..len], *level).await,
                        Encoding::Deflate(level) => {
                            compress::encode::deflate(&buf[..len], *level).await
                        }
                        Encoding::Br(level) => compress::encode::br(&buf[..len], *level).await,
                        _ => Ok(buf[..len].to_vec()),
                    };
                    let data = match data {
                        Ok(data) => data,
                        Err(_) => break,
                    };
                    if sender.send_data(Bytes::from(data)).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    res
}

async fn fallbacks(file: &PathBuf, exts: &Setting<Vec<String>>) -> Option<(File, String)> {
    if let Setting::Value(exts) = exts {
        let left = file.display().to_string();
        for ext in exts {
            let path = PathBuf::from(format!("{}.{}", &left, ext));
            if path.is_file() {
                if let Ok(file) = File::open(&path).await {
                    return Some((file, ext.to_string()));
                }
            }
        }
    }

    None
}

async fn index_back(root: &PathBuf, files: &Vec<String>) -> Option<(File, String)> {
    for name in files {
        let path = name.to_absolute_path(root);
        if path.is_file() {
            if let Ok(file) = File::open(&path).await {
                if let Some(s) = &path.get_extension() {
                    return Some((file, s.to_string()));
                }
            }
        }
    }
    None
}
