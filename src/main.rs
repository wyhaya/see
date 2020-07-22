mod client;
mod compress;
mod config;
mod directory;
mod logger;
mod matcher;
mod process;
mod server;
mod util;
mod var;
mod yaml;

use ace::App;
use bright::Colorful;
use compress::BodyStream;
use config::Headers;
use config::{
    default, mime, AbsolutePath, Directory, ErrorPage, Force, RewriteStatus, ServerConfig, Setting,
    SiteConfig,
};
use futures_util::future::join_all;
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
        print_start_info(configs[0].listen, root);
    }

    bind_tcp(configs).await;
}

fn print_start_info(listen: SocketAddr, path: PathBuf) {
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

        servers.push(server::run(listener, config));
    }

    join_all(servers).await;
}

// Response content is the StatusCode
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

fn get_match_config(host: Option<&str>, configs: Vec<SiteConfig>) -> Option<SiteConfig> {
    match host {
        // Use the host in config to match the header host
        Some(host) => {
            for config in configs {
                if config.host.is_match(host) {
                    return Some(config);
                }
            }
        }
        // No header host
        None => {
            for config in configs {
                if config.host.is_empty() {
                    return Some(config);
                }
            }
        }
    };

    None
}

// Handle client requests
pub async fn connect(
    req: Request<Body>,
    remote: IpAddr,
    configs: Vec<SiteConfig>,
) -> HyperResult<Response<Body>> {
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
        return Ok(Response::builder().into_status(StatusCode::BAD_REQUEST));
    }

    let mut res = match get_match_config(host, configs) {
        Some(mut config) => {
            // Decode request path
            let req_path = percent_encoding::percent_decode_str(req.uri().path())
                .decode_utf8_lossy()
                .to_string();

            // Merge location to config
            config = config.merge(&req_path);
            let mut header_map = HeaderMap::new();

            if let Setting::Value(headers) = config.headers.clone() {
                headers_merge(&mut header_map, headers, &req);
            }

            let mut res = handle(req, req_path, remote, config).await;
            res.headers_mut().extend(header_map);
            res
        }
        None => Response::builder().into_status(StatusCode::FORBIDDEN),
    };

    // Add server name for all responses
    res.headers_mut()
        .insert(SERVER, HeaderValue::from_static(default::SERVER_NAME));

    Ok(res)
}

async fn handle(
    req: Request<Body>,
    req_path: String,
    ip: IpAddr,
    mut config: SiteConfig,
) -> Response<Body> {
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
    if config.proxy.is_value() {
        return response_proxy(req, &config).await;
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
    if config.echo.is_value() {
        let echo = config.echo.into_value().map(|s, r| r.replace(s, &req));
        return Response::builder()
            .header(CONTENT_TYPE, mime::TEXT_PLAIN)
            .body(Body::from(echo))
            .unwrap();
    }

    // rewrite
    if config.rewrite.is_value() {
        let rewrite = config.rewrite.into_value();

        let value = rewrite.location.map(|s, r| {
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
                    StatusCode::FORBIDDEN,
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
                            util::get_extension(&path),
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

fn headers_merge(headers: &mut HeaderMap, new_headers: Headers, req: &Request<Body>) {
    for (name, value) in new_headers {
        let val = value.map(|s, r| {
            let v = r.replace(s, &req);
            HeaderValue::from_str(&v).unwrap()
        });
        headers.insert(name, val);
    }
}

async fn response_proxy(mut req: Request<Body>, config: &SiteConfig) -> Response<Body> {
    let c = config.proxy.clone().into_value();
    let encoding = req.headers().get(ACCEPT_ENCODING).cloned();

    let url = util::get_rand_item(&c.url).clone().map(|s, r| {
        let result = r.replace(s, &req);
        result.parse::<Uri>().unwrap()
    });

    *req.uri_mut() = url;
    if let Some(method) = c.method {
        *req.method_mut() = method;
    }
    if let Setting::Value(headers) = c.headers {
        let mut h = req.headers().clone();
        headers_merge(&mut h, headers, &req);
        *req.headers_mut() = h;
    }

    match client::request(req).await {
        Ok(res) => res,
        Err(err) => {
            let status = if err.is_timeout() {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::BAD_GATEWAY
            };
            response_error_page(encoding.as_ref(), &config, status).await
        }
    }
}

async fn response_error_page(
    encoding: Option<&HeaderValue>,
    config: &SiteConfig,
    status: StatusCode,
) -> Response<Body> {
    let path = match status {
        StatusCode::FORBIDDEN => &config.error._403,
        StatusCode::NOT_FOUND => &config.error._404,
        StatusCode::BAD_GATEWAY => &config.error._502,
        StatusCode::GATEWAY_TIMEOUT => &config.error._504,
        _ => &config.error._500,
    };

    if let Setting::Value(path) = path {
        if path.is_file() {
            if let Ok(f) = File::open(&path).await {
                return response_file(status, f, util::get_extension(&path), encoding, &config)
                    .await;
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
    let body = BodyStream::new(encoding).content(html);
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

    let body = BodyStream::new(encoding).file(file);

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
                if let Some(ext) = util::get_extension(&path) {
                    return Some((file, ext.to_string()));
                }
            }
        }
    }
    None
}
