mod accept;
mod client;
mod compress;
mod config;
mod directory;
mod matcher;
mod process;
mod util;
mod var;
mod yaml;

use ace::App;
use bright::Colorful;
use compress::ComressBody;
use config::{
    default, mime, AbsolutePath, Directory, Force, PathExtension, Proxy, RewriteStatus,
    ServerConfig, Setting, SiteConfig, StatusPage,
};
use futures::future::join_all;
use hyper::header::{
    HeaderValue, ACCEPT_ENCODING, ALLOW, AUTHORIZATION, CONTENT_ENCODING, CONTENT_LENGTH,
    CONTENT_TYPE, HOST, LOCATION, SERVER, WWW_AUTHENTICATE,
};
use hyper::http::response::Builder;
use hyper::Result as HyperResult;
use hyper::{Body, HeaderMap, Method, Request, Response, StatusCode, Uri, Version};
use matcher::HostMatcher;
use process::{start_daemon, stop_daemon};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::net::TcpListener;
use tokio::runtime;

fn main() {
    let mut runtime = runtime::Builder::new()
        .thread_name(default::SERVER_NAME)
        .threaded_scheduler()
        .enable_all()
        .core_threads(num_cpus::get())
        .max_threads(num_cpus::get() + 1)
        .build()
        .unwrap_or_else(|err| exit!("Cannot create async runtime\n{:?}", err));

    runtime.block_on(start());
}

async fn start() {
    let mut app = App::new()
        .config(default::SERVER_NAME, default::VERSION)
        .cmd("start", "Quick start")
        .cmd("stop", "Stop the daemon")
        .cmd("restart", "Restart the service program")
        .cmd("help", "Print help information")
        .cmd("version", "Print version information")
        .opt("-b", "Change the quick start binding address")
        .opt("-p", "Change the quick start directory")
        .opt("-c", "Specify a configuration file")
        .opt("-d", "Running in the background")
        .opt("-t", "Test the config file for error");

    let mut configs = vec![];

    if let Some(cmd) = app.command() {
        match cmd.as_str() {
            "start" => {
                let path = app
                    .value("-p")
                    .map(|values| {
                        if values.len() != 1 {
                            exit!("-p value: [DIR]");
                        }
                        values[0].absolute_path(util::current_dir())
                    })
                    .unwrap_or_else(|| util::current_dir());

                let addr = app
                    .value("-b")
                    .map(|values| {
                        if values.len() != 1 {
                            exit!("-b value: [ADDRESS]");
                        }
                        values[0].clone()
                    })
                    .unwrap_or_else(|| default::START_PORT.to_string());
                let addr = addr.as_str().to_socket_addr();
                configs = vec![default::quick_start_config(path, addr)];
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
                let config = util::config_path();
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
        let root = configs[0].sites[0].root.clone().unwrap();
        quick_start_info(configs[0].listen, root);
    }

    bind_tcp(configs).await;
}

fn quick_start_info(listen: SocketAddr, path: PathBuf) {
    println!(
        "Serving path   : {}",
        path.display().to_string().yellow().bold()
    );
    let port = match listen.port() {
        80 => String::new(),
        _ => format!(":{}", listen.port()),
    };
    println!(
        "Serving address: {}\x1b[0m",
        format!("http://{}{}", listen.ip(), port).green().bold()
    );
}

async fn bind_tcp(configs: Vec<ServerConfig>) {
    let mut servers = Vec::with_capacity(configs.len());

    for config in configs {
        let listener = TcpListener::bind(&config.listen)
            .await
            .unwrap_or_else(|err| exit!("Cannot bind to address: '{}'\n{:?}", &config.listen, err));

        servers.push(accept::run(listener, config));
    }

    join_all(servers).await;
}

// Response content is the current status
trait IntoStatus {
    fn into_status(self, status: StatusCode) -> Response<Body>;
}
impl IntoStatus for Builder {
    fn into_status(self, status: StatusCode) -> Response<Body> {
        let body = Body::from(status.to_string());

        self.status(status)
            .header(CONTENT_TYPE, mime::TEXT_PLAIN)
            .body(body)
            .unwrap()
    }
}

// Handle client requests
pub async fn connect(
    req: Request<Body>,
    ip: IpAddr,
    configs: Vec<SiteConfig>,
) -> HyperResult<Response<Body>> {
    let mut res = response(req, ip, configs).await;

    // Add server name for all responses
    res.headers_mut()
        .insert(SERVER, HeaderValue::from_static(default::SERVER_NAME));

    Ok(res)
}

async fn response(req: Request<Body>, remote: IpAddr, configs: Vec<SiteConfig>) -> Response<Body> {
    let host = req.headers().get(HOST).map(|header| {
        // Delete port
        header
            .to_str()
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default()
    });

    // A Host header field must be sent in all HTTP/1.1 request messages
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Host
    if host.is_none() && req.version() == Version::HTTP_11 {
        return Response::builder().into_status(StatusCode::BAD_REQUEST);
    }

    let mut opt = None;
    match host {
        // Use the host in config to match the header host
        Some(host) => {
            for config in configs {
                if config.host.is_match(host) {
                    opt = Some(config);
                    break;
                }
            }
        }
        // No header host
        None => {
            for config in configs {
                if config.host.is_empty() {
                    opt = Some(config);
                    break;
                }
            }
        }
    };

    match opt {
        Some(config) => {
            let headers = config.headers.clone();
            handle(req, remote, config).await.append_headers(headers)
        }
        None => Response::builder().into_status(StatusCode::FORBIDDEN),
    }
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

async fn handle(req: Request<Body>, ip: IpAddr, mut config: SiteConfig) -> Response<Body> {
    // Decode request path
    let req_path = percent_encoding::percent_decode_str(req.uri().path())
        .decode_utf8_lossy()
        .to_string();

    // Merge location to config
    if !config.location.is_empty() {
        config = merge_location(&req_path, config);
    }

    // Record request log
    if let Setting::Value(logger) = &mut config.log {
        logger.write(&req).await;
    }

    // IP allow and deny
    if let Setting::Value(matcher) = &config.ip {
        if !matcher.is_pass(ip) {
            return Response::builder().into_status(StatusCode::FORBIDDEN);
        }
    }

    // HTTP auth
    if let Setting::Value(auth) = &config.auth {
        let authorization = req.headers().get(AUTHORIZATION);

        if let Some(value) = authorization {
            if value != auth {
                return Response::builder()
                    .header(WWW_AUTHENTICATE, default::AUTH_MESSAGE)
                    .into_status(StatusCode::UNAUTHORIZED);
            }
        } else {
            return Response::builder()
                .header(WWW_AUTHENTICATE, default::AUTH_MESSAGE)
                .into_status(StatusCode::UNAUTHORIZED);
        }
    }

    // Proxy request
    if let Setting::Value(proxy) = &config.proxy {
        return proxy_response(req, proxy.clone(), &config).await;
    }

    // Not allowed request method
    if let Setting::Value(methods) = &config.methods {
        if !methods.contains(req.method()) {
            // Show allowed methods in header
            if req.method() == Method::OPTIONS {
                let allow = methods
                    .iter()
                    .map(|m| m.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ");

                return Response::builder()
                    .header(ALLOW, allow)
                    .into_status(StatusCode::METHOD_NOT_ALLOWED);
            }

            return Response::builder().into_status(StatusCode::METHOD_NOT_ALLOWED);
        }
    }

    // echo: Output plain text
    if let Setting::Value(echo) = &config.echo {
        let echo = echo.clone().map(|s, r| r.replace(s, &req));

        return Response::builder()
            .header(CONTENT_TYPE, mime::TEXT_PLAIN)
            .body(Body::from(echo))
            .unwrap();
    }

    // rewrite
    if let Setting::Value(rewrite) = &config.rewrite {
        let value = rewrite.location.clone().map(|s, r| {
            let result = r.replace(s, &req);
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

    let cur_path = format!(".{}", req_path);
    let path = match &config.root {
        Some(root) => {
            // file
            if let Setting::Value(p) = &config.file {
                p.clone()
            } else {
                Path::new(root).join(&cur_path)
            }
        }
        None => {
            // file
            if let Setting::Value(p) = &config.file {
                p.clone()
            } else {
                return response_error_page(
                    req.headers().get(ACCEPT_ENCODING),
                    &config,
                    StatusCode::NOT_FOUND,
                )
                .await;
            }
        }
    };

    match fs::metadata(&path).await {
        Ok(meta) => {
            if meta.is_dir() {
                if req_path.ends_with('/') {
                    if let Setting::Value(option) = &config.directory {
                        let html =
                            directory::render_dir_html(&path, &req_path, &option.time, option.size)
                                .await;
                        return match html {
                            Ok(html) => return response_html(html, &req, &config).await,
                            Err(_) => {
                                response_error_page(
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
                                    return response_file(
                                        StatusCode::OK,
                                        file,
                                        Some(&ext),
                                        req.headers().get(ACCEPT_ENCODING),
                                        &config,
                                    )
                                    .await
                                }
                                None => {
                                    return response_error_page(
                                        req.headers().get(ACCEPT_ENCODING),
                                        &config,
                                        StatusCode::NOT_FOUND,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    return response_error_page(
                        req.headers().get(ACCEPT_ENCODING),
                        &config,
                        StatusCode::NOT_FOUND,
                    )
                    .await;
                } else {
                    let location = match req.uri().query() {
                        Some(query) => format!("{}/{}", &req_path, query),
                        None => format!("{}/", &req_path),
                    };
                    return Response::builder()
                        .status(StatusCode::MOVED_PERMANENTLY)
                        .header(LOCATION, location)
                        .body(Body::empty())
                        .unwrap();
                }
            } else {
                match File::open(&path).await {
                    Ok(file) => {
                        return response_file(
                            StatusCode::OK,
                            file,
                            path.get_extension(),
                            req.headers().get(ACCEPT_ENCODING),
                            &config,
                        )
                        .await;
                    }
                    Err(_) => {
                        return response_error_page(
                            req.headers().get(ACCEPT_ENCODING),
                            &config,
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                        .await;
                    }
                }
            }
        }
        Err(_) => {
            // Cannot access the corresponding file
            // Use the 'config extensions' file to roll back
            if let Some((file, ext)) = fallbacks(&path, &config.extensions).await {
                return response_file(
                    StatusCode::OK,
                    file,
                    Some(&ext),
                    req.headers().get(ACCEPT_ENCODING),
                    &config,
                )
                .await;
            }

            // 404
            return response_error_page(
                req.headers().get(ACCEPT_ENCODING),
                &config,
                StatusCode::NOT_FOUND,
            )
            .await;
        }
    };
}

fn merge_location(route: &str, mut config: SiteConfig) -> SiteConfig {
    for item in config.location {
        if !item.location.is_match(route) {
            continue;
        }
        if item.root.is_some() {
            config.root = item.root;
        }
        if !item.echo.is_none() {
            config.echo = item.echo;
        }
        if !item.file.is_none() {
            config.file = item.file;
        }
        if !item.index.is_none() {
            config.index = item.index;
        }
        if !item.directory.is_none() {
            config.directory = item.directory;
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
            config.rewrite = item.rewrite;
        }
        if !item.compress.is_none() {
            config.compress = item.compress;
        }
        if !item.methods.is_none() {
            config.methods = item.methods;
        }
        if !item.auth.is_none() {
            config.auth = item.auth;
        }
        if !item.extensions.is_none() {
            config.extensions = item.extensions;
        }
        if !item.proxy.is_none() {
            config.proxy = item.proxy;
        }
        if !item.log.is_none() {
            config.log = item.log;
        }
        if !item.ip.is_none() {
            config.ip = item.ip;
        }
        if !item.status._403.is_none() {
            config.status._403 = item.status._403;
        }
        if !item.status._404.is_none() {
            config.status._404 = item.status._404;
        }
        if !item.status._500.is_none() {
            config.status._500 = item.status._500;
        }
        if !item.status._502.is_none() {
            config.status._502 = item.status._502;
        }
        if item.break_ {
            break;
        }
    }

    config.location = Vec::with_capacity(0);
    config
}

async fn proxy_response(mut req: Request<Body>, c: Proxy, config: &SiteConfig) -> Response<Body> {
    let encoding = req.headers().get(ACCEPT_ENCODING).cloned();

    let uri = util::get_rand_item(&c.uri).clone().map(|s, r| {
        let result = r.replace(s, &req);
        result.parse::<Uri>().unwrap()
    });

    *req.uri_mut() = uri;
    if let Some(method) = c.method {
        *req.method_mut() = method;
    }
    if let Setting::Value(headers) = c.headers {
        req.headers_mut().extend(headers);
    }

    // todo
    // timeout

    match client::request(req).await {
        Ok(res) => res,
        Err(_) => {
            // 502
            response_error_page(encoding.as_ref(), &config, StatusCode::BAD_GATEWAY).await
        }
    }
}

async fn response_error_page(
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
                return response_file(status, f, path.get_extension(), encoding, &config).await;
            }
        }
    }

    Response::builder().into_status(status)
}

async fn response_html(html: String, req: &Request<Body>, config: &SiteConfig) -> Response<Body> {
    let encoding = match &config.compress {
        Setting::Value(compress) => match req.headers().get(ACCEPT_ENCODING) {
            Some(header) => compress.get_html_compress_mode(&header),
            None => None,
        },
        _ => None,
    };
    let header = match encoding {
        Some(encoding) => (CONTENT_ENCODING, encoding.to_header_value()),
        None => (CONTENT_LENGTH, HeaderValue::from(html.len())),
    };
    let body = ComressBody::new(encoding).content(html);
    let mut res = Response::builder()
        .header(CONTENT_TYPE, mime::TEXT_HTML)
        .body(body)
        .unwrap();
    res.headers_mut().insert(header.0, header.1);
    res
}

async fn response_file(
    status: StatusCode,
    file: File,
    ext: Option<&str>,
    header: Option<&HeaderValue>,
    config: &SiteConfig,
) -> Response<Body> {
    let encoding = match &config.compress {
        Setting::Value(compress) => ext
            .map(|ext| {
                header
                    .map(|header| compress.get_compress_mode(&header, ext))
                    .unwrap_or_default()
            })
            .unwrap_or_default(),
        _ => None,
    };

    let header = match encoding {
        Some(encoding) => (CONTENT_ENCODING, encoding.to_header_value()),
        None => {
            let meta = file.metadata().await.unwrap();
            (CONTENT_LENGTH, HeaderValue::from(meta.len()))
        }
    };

    let body = ComressBody::new(encoding).file(file);

    let mut res = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, mime::from_extension(ext.unwrap_or_default()))
        .body(body)
        .unwrap();

    res.headers_mut().insert(header.0, header.1);

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

async fn index_back(root: &PathBuf, files: &[String]) -> Option<(File, String)> {
    for name in files {
        let path = name.absolute_path(root);
        if path.is_file() {
            if let Ok(file) = File::open(&path).await {
                if let Some(ext) = path.get_extension() {
                    return Some((file, ext.to_string()));
                }
            }
        }
    }
    None
}
