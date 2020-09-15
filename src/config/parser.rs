use crate::{
    compress, config, exit, logger, matcher, setting_none, setting_off, setting_value, util, var,
    yaml, Setting,
};
use base64::encode;
use compress::{CompressLevel, CompressMode, Encoding, Level};
use config::tls::{create_sni_server_config, TLSContent};
use config::{default, AbsolutePath, Force};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Method, Uri};
use logger::Logger;
use matcher::{HostMatcher, IpMatcher, LocationMatcher};
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio_rustls::TlsAcceptor;
use util::*;
use var::Var;
use yaml::YamlExtend;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Clone)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub tls: Option<TlsAcceptor>,
    pub sites: Vec<SiteConfig>,
}

pub type Headers = HashMap<HeaderName, Var<HeaderValue>>;

#[derive(Debug, Clone, Default)]
pub struct SiteConfig {
    pub host: HostMatcher,
    pub root: Option<PathBuf>,
    pub echo: Setting<Var<String>>,
    pub file: Setting<PathBuf>,
    pub index: Setting<Vec<String>>,
    pub directory: Setting<Directory>,
    pub headers: Setting<Headers>,
    pub rewrite: Setting<Rewrite>,
    pub compress: Setting<Compress>,
    pub methods: Setting<Vec<Method>>,
    pub auth: Setting<String>,
    pub extensions: Setting<Vec<String>>,
    pub error: ErrorPage,
    pub proxy: Setting<Proxy>,
    pub log: Setting<Logger>,
    pub ip: Setting<IpMatcher>,
    pub location: Vec<Location>,
}

impl SiteConfig {
    pub fn merge(mut self, route: &str) -> Self {
        for item in self.location {
            if !item.location.is_match(route) {
                continue;
            }
            if item.root.is_some() {
                self.root = item.root;
            }
            if !item.echo.is_none() {
                self.echo = item.echo;
            }
            if !item.file.is_none() {
                self.file = item.file;
            }
            if !item.index.is_none() {
                self.index = item.index;
            }
            if !item.directory.is_none() {
                self.directory = item.directory;
            }
            if !item.headers.is_none() {
                if item.headers.is_value() {
                    let mut h = self.headers.clone().unwrap_or_default();
                    h.extend(item.headers.into_value());
                    self.headers = Setting::Value(h);
                } else if item.headers.is_off() {
                    self.headers = Setting::Off;
                }
            }
            if !item.rewrite.is_none() {
                self.rewrite = item.rewrite;
            }
            if !item.compress.is_none() {
                self.compress = item.compress;
            }
            if !item.methods.is_none() {
                self.methods = item.methods;
            }
            if !item.auth.is_none() {
                self.auth = item.auth;
            }
            if !item.extensions.is_none() {
                self.extensions = item.extensions;
            }
            if !item.proxy.is_none() {
                self.proxy = item.proxy;
            }
            if !item.log.is_none() {
                self.log = item.log;
            }
            if !item.ip.is_none() {
                self.ip = item.ip;
            }
            if !item.error._403.is_none() {
                self.error._403 = item.error._403;
            }
            if !item.error._404.is_none() {
                self.error._404 = item.error._404;
            }
            if !item.error._500.is_none() {
                self.error._500 = item.error._500;
            }
            if !item.error._502.is_none() {
                self.error._502 = item.error._502;
            }
            if !item.error._504.is_none() {
                self.error._504 = item.error._504;
            }
            if item.break_ {
                break;
            }
        }

        self.location = Vec::with_capacity(0);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Location {
    pub location: LocationMatcher,
    pub break_: bool,
    pub root: Option<PathBuf>,
    pub echo: Setting<Var<String>>,
    pub file: Setting<PathBuf>,
    pub index: Setting<Vec<String>>,
    pub directory: Setting<Directory>,
    pub headers: Setting<Headers>,
    pub rewrite: Setting<Rewrite>,
    pub compress: Setting<Compress>,
    pub methods: Setting<Vec<Method>>,
    pub auth: Setting<String>,
    pub extensions: Setting<Vec<String>>,
    pub error: ErrorPage,
    pub proxy: Setting<Proxy>,
    pub log: Setting<Logger>,
    pub ip: Setting<IpMatcher>,
}

#[derive(Debug, Clone)]
pub struct Directory {
    pub time: Option<String>,
    pub size: bool,
}

#[derive(Debug, Clone)]
pub struct Rewrite {
    pub location: Var<HeaderValue>,
    pub status: RewriteStatus,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RewriteStatus {
    _301,
    _302,
}

impl From<&str> for RewriteStatus {
    fn from(s: &str) -> Self {
        match s {
            "301" => RewriteStatus::_301,
            "302" => RewriteStatus::_302,
            _ => exit!("Wrong redirect type `{}`, optional value: `301` `302`", s),
        }
    }
}

impl Default for RewriteStatus {
    fn default() -> Self {
        RewriteStatus::_302
    }
}

impl From<String> for Rewrite {
    fn from(s: String) -> Self {
        let mut split = s.split_whitespace();

        let location = split
            .next()
            .map(|s| Var::from(s).map_none(|s| s.as_str().to_header_value()))
            .unwrap_or_else(|| exit!("Could not find redirected url"));

        let status = split.next().map(RewriteStatus::from).unwrap_or_default();

        Rewrite { location, status }
    }
}

#[derive(Debug, Clone)]
pub struct Compress {
    pub modes: Vec<Encoding>,
    pub extensions: Vec<String>,
}

impl Compress {
    // todo
    // remove ...
    pub fn get_html_compress_mode(&self, header: &HeaderValue) -> Option<CompressMode> {
        // accept-encoding: gzip, deflate, br
        let header: Vec<&str> = match header.to_str() {
            Ok(encoding) => encoding.split(", ").collect(),
            Err(_) => return None,
        };
        for encoding in &self.modes {
            if let Some(compress) = encoding.get_compress_mode(&header) {
                return Some(compress);
            }
        }
        None
    }

    pub fn get_compress_mode(&self, header: &HeaderValue, ext: &str) -> Option<CompressMode> {
        if self.extensions.iter().any(|item| *item == ext) {
            // accept-encoding: gzip, deflate, br
            let header: Vec<&str> = match header.to_str() {
                Ok(encoding) => encoding.split(", ").collect(),
                Err(_) => return None,
            };
            for encoding in &self.modes {
                if let Some(compress) = encoding.get_compress_mode(&header) {
                    return Some(compress);
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct ErrorPage {
    pub _403: Setting<PathBuf>,
    pub _404: Setting<PathBuf>,
    pub _500: Setting<PathBuf>,
    pub _502: Setting<PathBuf>,
    pub _504: Setting<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Proxy {
    pub url: Vec<Var<Uri>>,
    pub method: Option<Method>,
    pub headers: Setting<Headers>,
}

impl ServerConfig {
    pub async fn new(path: &str) -> Vec<Self> {
        let config_dir = Path::new(&path)
            .parent()
            .unwrap_or_else(|| exit!("Cannot get configuration file directory"));

        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| exit!("Read '{}' failed\n{:?}", path, err));

        let docs = YamlLoader::load_from_str(&content)
            .unwrap_or_else(|err| exit!("Parsing config file failed\n{:?}", err));

        if docs.is_empty() {
            exit!("Cannot parse `server` to array")
        }

        let servers = docs[0]
            .as_vec()
            .unwrap_or_else(|| exit!("Cannot parse `server` to array"));

        let mut configs: Vec<ServerConfig> = vec![];
        let mut tls_configs: Vec<(SocketAddr, Vec<TLSContent>)> = vec![];

        for server in servers {
            server.check(
                "server",
                &[
                    "listen",
                    "https",
                    "host",
                    "root",
                    "echo",
                    "file",
                    "index",
                    "directory",
                    "header",
                    "rewrite",
                    "compress",
                    "method",
                    "auth",
                    "extension",
                    "error",
                    "proxy",
                    "log",
                    "ip",
                    "buffer",
                    "location",
                ],
                &["listen"],
            );

            let parser = Parser::new(server["server"].clone());
            let listens = parser.listen();
            let host = parser.host();

            let https = parser.https(&config_dir, host.get_raw());

            if let Some(tls) = https {
                for listen in &listens {
                    let position = tls_configs.iter().position(|item| item.0 == *listen);
                    match position {
                        Some(i) => {
                            tls_configs[i].1.push(tls.clone());
                        }
                        None => {
                            tls_configs.push((*listen, vec![tls.clone()]));
                        }
                    }
                }
            }

            let root = parser.root(&config_dir);

            let site = SiteConfig {
                host,
                root: root.clone(),
                echo: parser.echo(),
                file: parser.file(&config_dir),
                index: parser.index(true),
                directory: parser.directory(),
                headers: parser.headers(),
                rewrite: parser.rewrite(),
                compress: parser.compress(),
                extensions: parser.extensions(),
                methods: parser.methods(true),
                error: parser.error(&root),
                proxy: parser.proxy(),
                log: parser.log(&config_dir).await,
                ip: parser.ip(),
                auth: parser.auth(),
                location: parser.location(&config_dir, root).await,
            };

            for listen in listens {
                let position = configs.iter().position(|item| item.listen == listen);
                match position {
                    Some(i) => {
                        configs[i].sites.push(site.clone());
                    }
                    None => configs.push(ServerConfig {
                        listen,
                        tls: None,
                        sites: vec![site.clone()],
                    }),
                }
            }
        }

        for (listen, group) in tls_configs {
            let group = dedup(group);
            let i = configs
                .iter()
                .position(|item| item.listen == listen)
                .unwrap();
            let t = create_sni_server_config(group);
            configs[i].tls = Some(t);
        }

        configs
    }
}

struct Parser {
    yaml: Yaml,
}

impl Parser {
    fn new(yaml: Yaml) -> Self {
        Self { yaml }
    }

    async fn location<P: AsRef<Path>>(
        &self,
        config_dir: P,
        parent_root: Option<PathBuf>,
    ) -> Vec<Location> {
        if self.yaml["location"].is_badvalue() {
            return vec![];
        }

        let hash = self.yaml.key_to_hash("location");
        let mut vec = vec![];

        for (i, (key, server)) in hash.iter().enumerate() {
            let route = key.to_string(format!("location[{}]", i));

            self.yaml["location"].check(
                &route,
                &[
                    "break",
                    "root",
                    "echo",
                    "file",
                    "index",
                    "directory",
                    "header",
                    "rewrite",
                    "compress",
                    "method",
                    "auth",
                    "extension",
                    "error",
                    "proxy",
                    "log",
                    "ip",
                ],
                &[],
            );

            let parser = Parser::new(server.clone());

            let root = match parser.root(config_dir.as_ref()) {
                Some(p) => Some(p),
                None => parent_root.clone(),
            };

            vec.push(Location {
                location: LocationMatcher::new(route.as_str()),
                break_: parser.break_(),
                root: root.clone(),
                echo: parser.echo(),
                file: parser.file(&config_dir),
                index: parser.index(false),
                directory: parser.directory(),
                headers: parser.headers(),
                rewrite: parser.rewrite(),
                compress: parser.compress(),
                methods: parser.methods(false),
                auth: parser.auth(),
                extensions: parser.extensions(),
                error: parser.error(&root),
                proxy: parser.proxy(),
                log: parser.log(&config_dir).await,
                ip: parser.ip(),
            });
        }
        vec
    }

    fn listen(&self) -> Vec<SocketAddr> {
        let vec = self
            .yaml
            .key_to_multiple_string("listen")
            .iter()
            .map(|s| s.as_str().to_socket_addr())
            .collect();

        dedup(vec)
    }

    fn https(&self, config_dir: &Path, hostname: Vec<&String>) -> Option<TLSContent> {
        let https = &self.yaml["https"];
        if https.is_badvalue() {
            return None;
        }

        self.yaml.check("https", &["cert", "key"], &["cert", "key"]);

        let cert = https.key_to_string("cert").absolute_path(config_dir);
        let key = https.key_to_string("key").absolute_path(config_dir);

        if hostname.is_empty() {
            exit!("Miss 'host'");
        }

        // todo
        let sni = hostname[0].clone();

        Some(TLSContent { cert, key, sni })
    }

    fn host(&self) -> HostMatcher {
        let vec = self.yaml.key_to_multiple_string("host");
        HostMatcher::new(vec)
    }

    fn root(&self, config_dir: &Path) -> Option<PathBuf> {
        if self.yaml["root"].is_badvalue() {
            return None;
        }

        let path = self.yaml.key_to_string("root").absolute_path(config_dir);
        Some(path)
    }

    fn echo(&self) -> Setting<Var<String>> {
        setting_value!(self.yaml["echo"]);

        let var = Var::from(self.yaml.key_to_string("echo"));
        Setting::Value(var)
    }

    fn file<P: AsRef<Path>>(&self, root: P) -> Setting<PathBuf> {
        setting_value!(self.yaml["file"]);

        let path = self.yaml.key_to_string("file").absolute_path(root);
        Setting::Value(path)
    }

    fn index(&self, set_default: bool) -> Setting<Vec<String>> {
        let index = &self.yaml["index"];
        setting_off!(index);

        if set_default {
            setting_none!(
                index,
                default::INDEX.iter().map(|i| (*i).to_string()).collect()
            );
        } else {
            setting_none!(index);
        }

        let vec = self.yaml.key_to_multiple_string("index");
        Setting::Value(vec)
    }

    fn directory(&self) -> Setting<Directory> {
        let directory = &self.yaml["directory"];
        setting_value!(directory);

        // directory: true
        if directory.as_bool().is_some() {
            return Setting::Value(Directory {
                time: None,
                size: false,
            });
        }

        self.yaml.check("directory", &["time", "size"], &[]);

        let time = if directory["time"].is_badvalue() {
            None
        } else {
            match directory["time"].as_bool() {
                Some(b) => {
                    if b {
                        Some(default::DIRECTORY_TIME_FORMAT.to_string())
                    } else {
                        None
                    }
                }
                None => {
                    let format = directory.key_to_string("time");
                    // check
                    format.as_str().to_strftime();
                    Some(format)
                }
            }
        };

        let size = if directory["size"].is_badvalue() {
            false
        } else {
            directory.key_to_bool("size")
        };

        Setting::Value(Directory { time, size })
    }

    fn headers(&self) -> Setting<Headers> {
        setting_value!(self.yaml["header"]);

        let hash = self.yaml.key_to_hash("header");
        let mut map = HashMap::new();

        for (i, (key, value)) in hash.iter().enumerate() {
            let key = key.to_string(format!("header[{}]", i));
            let header_name = key.as_str().to_header_name();

            let value = Var::from(value.to_string(&key));
            let header_value = value.map_none(|s| s.as_str().to_header_value());

            map.insert(header_name, header_value);
        }

        Setting::Value(map)
    }

    fn rewrite(&self) -> Setting<Rewrite> {
        setting_value!(self.yaml["rewrite"]);
        let value = self.yaml.key_to_string("rewrite");

        Setting::Value(Rewrite::from(value))
    }

    fn compress(&self) -> Setting<Compress> {
        let compress = &self.yaml["compress"];
        setting_value!(compress);

        // compress: true
        if compress.as_bool().is_some() {
            return Setting::Value(Compress {
                modes: vec![Encoding::Auto(default::COMPRESS_LEVEL)],
                extensions: default::COMPRESS_EXTENSIONS
                    .iter()
                    .map(|e| (*e).to_string())
                    .collect(),
            });
        }

        self.yaml
            .check("compress", &["mode", "level", "extension"], &[]);

        let level = if compress["level"].is_badvalue() {
            default::COMPRESS_LEVEL
        } else {
            CompressLevel::new(compress.key_to_string("level"))
        };

        let modes = if compress["mode"].is_badvalue() {
            vec![Encoding::Auto(level)]
        } else {
            let mode = compress.key_to_multiple_string("mode");
            mode.iter().map(|mode| Encoding::new(mode, level)).collect()
        };

        let extensions = if compress["extension"].is_badvalue() {
            default::COMPRESS_EXTENSIONS
                .iter()
                .map(|e| (*e).to_string())
                .collect()
        } else {
            compress.key_to_multiple_string("extension")
        };

        Setting::Value(Compress { modes, extensions })
    }

    fn extensions(&self) -> Setting<Vec<String>> {
        setting_value!(self.yaml["extension"]);

        let vec = self.yaml.key_to_multiple_string("extension");
        Setting::Value(vec)
    }

    fn methods(&self, set_default: bool) -> Setting<Vec<Method>> {
        let method = &self.yaml["method"];
        setting_off!(method);

        if set_default {
            setting_none!(method, default::ALLOW_METHODS.to_vec());
        } else {
            setting_none!(method);
        }

        let methods = self
            .yaml
            .key_to_multiple_string("method")
            .iter()
            .map(|m| m.as_str().to_method())
            .collect();

        Setting::Value(methods)
    }

    fn auth(&self) -> Setting<String> {
        let auth = &self.yaml["auth"];
        setting_value!(auth);

        self.yaml
            .check("auth", &["user", "password"], &["user", "password"]);

        let s = format!(
            "{}:{}",
            auth.key_to_string("user"),
            auth.key_to_string("password")
        );

        Setting::Value(format!("Basic {}", encode(&s)))
    }

    fn error(&self, root: &Option<PathBuf>) -> ErrorPage {
        if self.yaml["error"].is_badvalue() {
            return ErrorPage::default();
        }

        self.yaml
            .check("error", &["403", "404", "500", "502", "504"], &[]);

        let parser = Parser::new(self.yaml["error"].clone());

        ErrorPage {
            _403: parser.error_page(403, &root),
            _404: parser.error_page(404, &root),
            _500: parser.error_page(500, &root),
            _502: parser.error_page(502, &root),
            _504: parser.error_page(504, &root),
        }
    }

    fn error_page(&self, status: usize, root: &Option<PathBuf>) -> Setting<PathBuf> {
        setting_value!(self.yaml[status]);

        let s = self.yaml[status].to_string(status);
        let p = PathBuf::from(&s);

        if p.is_absolute() {
            Setting::Value(p)
        } else {
            let root = root.clone().unwrap_or_else(|| exit!("miss root"));
            Setting::Value(s.absolute_path(&root))
        }
    }

    fn proxy(&self) -> Setting<Proxy> {
        let proxy = &self.yaml["proxy"];
        setting_value!(proxy);

        self.yaml
            .check("proxy", &["url", "method", "timeout", "header"], &["url"]);

        let url = proxy
            .key_to_multiple_string("url")
            .iter()
            .map(|u| Var::from(u).map_none(|s| s.as_str().to_url()))
            .collect::<Vec<Var<Uri>>>();

        let method = if proxy["method"].is_badvalue() {
            None
        } else {
            let method = proxy.key_to_string("method").as_str().to_method();
            Some(method)
        };

        let headers = Parser::new(proxy.clone()).headers();

        Setting::Value(Proxy {
            url,
            method,
            headers,
        })
    }

    async fn log<P: AsRef<Path>>(&self, root: P) -> Setting<Logger> {
        let log = &self.yaml["log"];
        setting_value!(log);

        if let Some(path) = log.try_to_string() {
            let logger = Logger::new(default::LOG_FORMAT.to_string())
                .file(path.absolute_path(root))
                .await
                .unwrap_or_else(|err| exit!("Init logger failed:\n{:?}", err));

            return Setting::Value(logger);
        }

        self.yaml.check("log", &["mode", "file", "format"], &[]);

        let format_ = if log["format"].is_badvalue() {
            default::LOG_FORMAT.to_string()
        } else {
            log.key_to_string("format")
        };

        let mode = log.key_to_string("mode");

        match mode.as_ref() {
            "stdout" => Setting::Value(Logger::new(format_).stdout()),
            "file" => {
                let path = log.key_to_string("log file");
                let logger = Logger::new(format_)
                    .file(path.absolute_path(root))
                    .await
                    .unwrap_or_else(|err| exit!("Init logger failed:\n{:?}", err));

                Setting::Value(logger)
            }
            _ => exit!("Wrong log mode `{}`, optional value: `stdout` `file`", mode),
        }
    }

    fn ip(&self) -> Setting<IpMatcher> {
        let ip = &self.yaml["ip"];
        setting_value!(ip);

        self.yaml.check("ip", &["allow", "deny"], &[]);

        Setting::Value(IpMatcher::new(
            ip.key_to_multiple_string("allow"),
            ip.key_to_multiple_string("deny"),
        ))
    }

    fn break_(&self) -> bool {
        if self.yaml["break"].is_badvalue() {
            return false;
        }

        self.yaml.key_to_bool("break")
    }
}
