mod client;
mod compress;
mod config;
mod directory;
mod logger;
mod matcher;
mod server;
mod util;
mod var;
mod yaml;

use ace::App;
use compress::BodyStream;
use config::Headers;
use config::{
    default, mime, AbsolutePath, Force, RewriteStatus, ServerConfig, Setting, SiteConfig,
};
use futures_util::future::join_all;
use hyper::header::{
    HeaderName, HeaderValue, ACCEPT_ENCODING, ALLOW, CONTENT_ENCODING, CONTENT_LENGTH,
    CONTENT_TYPE, HOST, LOCATION, SERVER, WWW_AUTHENTICATE,
};
use hyper::Result as HyperResult;
use hyper::{Body, HeaderMap, Method, Request, Response, StatusCode, Uri, Version};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::net::TcpListener;
use tokio::runtime;
use var::Var;

fn main() {
    let mut runtime = runtime::Builder::new()
        .thread_name(default::SERVER_NAME)
        .threaded_scheduler()
        .enable_all()
        .core_threads(num_cpus::get())
        .max_threads(num_cpus::get() + 1)
        .build()
        .unwrap_or_else(|err| exit!("Cannot create async runtime\n{:?}", err));

    runtime.block_on(async_main());
}

async fn async_main() {
    let app = App::new()
        .config(default::SERVER_NAME, default::VERSION)
        .cmd("start", "Quick start in the current directory")
        .cmd("help", "Print help information")
        .cmd("version", "Print version information")
        .opt("-b", "Change the 'start' binding address")
        .opt("-p", "Change the 'start' root directory")
        .opt("-c", "Set configuration file")
        .opt("-t", "Test the config file for error");

    if let Some(cmd) = app.command() {
        match cmd.as_str() {
            "start" => {
                let addr = match app.value("-b") {
                    Some(values) => {
                        if values.len() != 1 {
                            exit!("-b value: [ADDRESS]");
                        }
                        values[0].clone()
                    }
                    None => default::START_PORT.to_string(),
                }
                .as_str()
                .to_socket_addr();

                let path = match app.value("-p") {
                    Some(values) => {
                        if values.len() != 1 {
                            exit!("-p value: [DIR]");
                        }
                        values[0].absolute_path(util::current_dir())
                    }
                    None => util::current_dir(),
                };

                let config = default::quick_start_config(path.clone(), addr);

                let port = match addr.port() {
                    80 => String::new(),
                    _ => format!(":{}", addr.port()),
                };

                println!("Serving path   : {}", path.display());
                println!(
                    "Serving address: {}",
                    format!("http://{}{}", addr.ip(), port)
                );

                return bind_tcp(vec![config]).await;
            }
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

    let config_path = match app.value("-c") {
        Some(values) => {
            if values.len() != 1 {
                exit!("-c value: [CONFIG_FILE]");
            }
            values[0].clone()
        }
        None => default::config_path().display().to_string(),
    };

    let configs = ServerConfig::new(&config_path).await;

    // Check configuration file
    if app.value("-t").is_some() {
        return println!(
            "There are no errors in the configuration file '{}'",
            config_path
        );
    }

    bind_tcp(configs).await;
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

fn get_match_config(
    req: &Request<Body>,
    configs: Vec<SiteConfig>,
) -> Result<Option<SiteConfig>, ()> {
    match req.version() {
        Version::HTTP_2 => {
            // todo
            return Ok(Some(configs[0].clone()));
        }
        Version::HTTP_11 => {
            let host = match req.headers().get(HOST) {
                Some(header) => {
                    // Delete port
                    header
                        .to_str()
                        .unwrap_or_default()
                        .split(':')
                        .next()
                        .unwrap_or_default()
                }
                // A Host header field must be sent in all HTTP/1.1 request messages
                // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Host
                None => return Err(()),
            };
            // Use the host in config to match the header host
            Ok(configs
                .into_iter()
                .find(|config| config.host.is_match(host)))
        }
        _ => {
            // No header host
            Ok(configs.into_iter().find(|config| config.host.is_empty()))
        }
    }
}

pub async fn connect(
    req: Request<Body>,
    remote: IpAddr,
    configs: Vec<SiteConfig>,
) -> HyperResult<Response<Body>> {
    let mut config = match get_match_config(&req, configs) {
        Ok(opt) => match opt {
            Some(config) => config,
            None => {
                return Ok(StatusResponse::from_status(StatusCode::FORBIDDEN));
            }
        },
        Err(_) => {
            return Ok(StatusResponse::from_status(StatusCode::BAD_REQUEST));
        }
    };

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
            return StatusResponse::from_status(StatusCode::FORBIDDEN);
        }
    }

    // HTTP auth
    if let Setting::Value(auth) = &config.auth {
        if !auth.check(&req) {
            return StatusResponse::new(StatusCode::UNAUTHORIZED)
                .header(
                    WWW_AUTHENTICATE,
                    HeaderValue::from_static(default::AUTH_MESSAGE),
                )
                .into();
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

                return StatusResponse::new(StatusCode::METHOD_NOT_ALLOWED)
                    .header(ALLOW, HeaderValue::from_str(&allow).unwrap())
                    .into();
            } else {
                return StatusResponse::from_status(StatusCode::METHOD_NOT_ALLOWED);
            }
        }
    }

    // echo: Output plain text
    if config.echo.is_value() {
        let echo = config.echo.into_value().map(|s, r| r.replace(s, &req));
        return Response::new(Body::from(echo))
            .set_header(CONTENT_TYPE, HeaderValue::from_static(mime::TEXT_PLAIN));
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

        return Response::new(Body::empty())
            .set_status(status)
            .set_header(LOCATION, value);
    }

    let cur_path = format!(".{}", req_path);
    let path = match &config.root {
        Some(root) => {
            if let Setting::Value(p) = &config.file {
                p.clone()
            } else {
                Path::new(root).join(&cur_path)
            }
        }
        None => {
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

    match FileRoute::new(&path, &req_path).await {
        FileRoute::Ok => {
            // .
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

        FileRoute::Redirect => {
            let location = match req.uri().query() {
                Some(query) => format!("{}/{}", &req_path, query),
                None => format!("{}/", &req_path),
            };

            return Response::new(Body::empty())
                .set_status(StatusCode::MOVED_PERMANENTLY)
                .set_header(LOCATION, HeaderValue::from_str(&location).unwrap());
        }

        FileRoute::Directory => {
            if let Setting::Value(directory) = &config.directory {
                return match directory.render(&path, &req_path).await {
                    Ok(html) => response_html(html, &req, &config).await,
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
                if let Some((file, ext)) = index.from_directory(path).await {
                    return response_file(
                        StatusCode::OK,
                        file,
                        Some(&ext),
                        req.headers().get(ACCEPT_ENCODING),
                        &config,
                    )
                    .await;
                }
            }
            response_error_page(
                req.headers().get(ACCEPT_ENCODING),
                &config,
                StatusCode::NOT_FOUND,
            )
            .await
        }

        FileRoute::Error => {
            // Use the 'config try' file to roll back
            if let Setting::Value(try_) = &config.try_ {
                if let Some((file, ext)) = try_files(&path, try_, &req).await {
                    return response_file(
                        StatusCode::OK,
                        file,
                        Some(&ext),
                        req.headers().get(ACCEPT_ENCODING),
                        &config,
                    )
                    .await;
                }
            }

            response_error_page(
                req.headers().get(ACCEPT_ENCODING),
                &config,
                StatusCode::NOT_FOUND,
            )
            .await
        }
    }
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

    let url = c.url.map(|s, r| {
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
        if util::is_file(path).await {
            if let Ok(f) = File::open(&path).await {
                return response_file(status, f, util::get_extension(&path), encoding, &config)
                    .await;
            }
        }
    }

    StatusResponse::from_status(status)
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
    let body = BodyStream::new(encoding).text(html);

    Response::new(body)
        .set_header(CONTENT_TYPE, HeaderValue::from_static(mime::TEXT_HTML))
        .set_header(header.0, header.1)
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

    Response::new(body)
        .set_status(status)
        .set_header(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::from_extension(ext.unwrap_or_default())),
        )
        .set_header(header.0, header.1)
}

async fn try_files(_: &PathBuf, _: &Vec<Var<String>>, _: &Request<Body>) -> Option<(File, String)> {
    todo!();
}

// Response content is the StatusCode
struct StatusResponse(Response<Body>);

impl StatusResponse {
    fn new(status: StatusCode) -> Self {
        let body = Body::from(status.to_string());
        let mut res = Response::new(body);
        *res.status_mut() = status;

        Self(res)
    }

    fn header(mut self, key: HeaderName, val: HeaderValue) -> Self {
        self.0.headers_mut().insert(key, val);
        self
    }

    fn into(self) -> Response<Body> {
        self.header(CONTENT_TYPE, HeaderValue::from_static(mime::TEXT_PLAIN))
            .0
    }

    fn from_status(status: StatusCode) -> Response<Body> {
        Self::new(status).into()
    }
}

trait ResponseExtend {
    fn set_status(self, status: StatusCode) -> Self;
    fn set_header(self, key: HeaderName, val: HeaderValue) -> Self;
}

impl ResponseExtend for Response<Body> {
    fn set_status(mut self, status: StatusCode) -> Self {
        *self.status_mut() = status;
        self
    }

    fn set_header(mut self, key: HeaderName, val: HeaderValue) -> Self {
        self.headers_mut().insert(key, val);
        self
    }
}

enum FileRoute {
    Error,
    Ok,
    Directory,
    Redirect,
}

impl FileRoute {
    async fn new(path: &PathBuf, req_path: &str) -> Self {
        match fs::metadata(path).await {
            Ok(meta) => {
                if meta.is_dir() {
                    if req_path.ends_with('/') {
                        Self::Directory
                    } else {
                        Self::Redirect
                    }
                } else {
                    Self::Ok
                }
            }
            Err(_) => Self::Error,
        }
    }
}
