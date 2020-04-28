use crate::config::default;
use crate::config::tls::{create_sni_server_config, TLSContent};
use crate::config::Force;
use crate::config::Logger;
use crate::util::*;
use crate::var::{ToVar, Var};
use crate::*;
use base64::encode;
use compress::Level;
use compress::{CompressLevel, CompressMode, Encoding};
use hyper::{Method, Uri};
use matcher::{HostMatcher, IpMatcher, LocationMatcher};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio_rustls::TlsAcceptor;
use yaml::YamlExtend;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug, Clone)]
pub enum Setting<T> {
    None,
    Off,
    Value(T),
}

impl<T> Setting<T> {
    pub fn is_none(&self) -> bool {
        match self {
            Setting::None => true,
            _ => false,
        }
    }

    pub fn is_off(&self) -> bool {
        match self {
            Setting::Off => true,
            _ => false,
        }
    }
}

impl<T> Default for Setting<T> {
    fn default() -> Self {
        Setting::None
    }
}

impl<T: Default> Setting<T> {
    pub fn unwrap_or_default(self) -> T {
        match self {
            Setting::Value(x) => x,
            _ => Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub tls: Option<TlsAcceptor>,
    pub sites: Vec<SiteConfig>,
}

#[derive(Clone, Debug)]
pub struct SiteConfig {
    pub sni_name: Option<String>,
    pub host: HostMatcher,
    pub root: Option<PathBuf>,
    pub echo: Setting<Var<String>>,
    pub file: Setting<PathBuf>,
    pub index: Setting<Vec<String>>,
    pub directory: Setting<Directory>,
    pub headers: Setting<HeaderMap>,
    pub rewrite: Setting<Rewrite>,
    pub compress: Setting<Compress>,
    pub methods: Setting<Vec<Method>>,
    pub auth: Setting<String>,
    pub extensions: Setting<Vec<String>>,
    pub status: StatusPage,
    pub proxy: Setting<Proxy>,
    pub log: Setting<Logger>,
    pub ip: Setting<IpMatcher>,
    pub buffer: Setting<usize>,
    pub location: Vec<Location>,
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
    pub headers: Setting<HeaderMap>,
    pub rewrite: Setting<Rewrite>,
    pub compress: Setting<Compress>,
    pub methods: Setting<Vec<Method>>,
    pub auth: Setting<String>,
    pub extensions: Setting<Vec<String>>,
    pub status: StatusPage,
    pub proxy: Setting<Proxy>,
    pub log: Setting<Logger>,
    pub ip: Setting<IpMatcher>,
    pub buffer: Setting<usize>,
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
            .unwrap_or_else(|| exit!("Could not find redirected url"));

        let status = split.next().map(RewriteStatus::from).unwrap_or_default();

        let val = location
            .to_string()
            .to_var()
            .map_none(|s| s.as_str().to_header_value());

        Rewrite {
            location: val,
            status,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Compress {
    pub modes: Vec<Encoding>,
    pub extensions: Vec<String>,
}

impl Compress {
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
pub struct StatusPage {
    pub _403: Setting<PathBuf>,
    pub _404: Setting<PathBuf>,
    pub _500: Setting<PathBuf>,
    pub _502: Setting<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Proxy {
    pub uri: Vec<Var<Uri>>,
    pub method: Option<Method>,
    pub timeout: Duration,
    pub headers: Setting<HeaderMap>,
}

macro_rules! setting_value {
    ($yaml: expr) => {
        setting_none!($yaml);
        setting_off!($yaml);
    };
}

macro_rules! setting_none {
    ($yaml: expr) => {
        if $yaml.is_badvalue() || $yaml.is_null() {
            return Setting::None;
        }
    };
    ($yaml: expr, $default: expr) => {
        if $yaml.is_badvalue() || $yaml.is_null() {
            return Setting::Value($default);
        }
    };
}

macro_rules! setting_off {
    ($yaml: expr) => {
        if let Some(val) = $yaml.as_bool() {
            if !val {
                return Setting::Off;
            }
        }
    };
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
                    "status",
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
            let https = parser.https(&config_dir);
            let sni_name = https.clone().map(|d| d.sni);

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
                sni_name,
                host: parser.host(),
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
                status: parser.status(&root),
                proxy: parser.proxy(),
                log: parser.log(&config_dir).await,
                ip: parser.ip(),
                buffer: parser.buffer(),
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
                    "status",
                    "proxy",
                    "log",
                    "ip",
                    "buffer",
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
                status: parser.status(&root),
                proxy: parser.proxy(),
                log: parser.log(&config_dir).await,
                ip: parser.ip(),
                buffer: parser.buffer(),
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

    fn https(&self, config_dir: &Path) -> Option<TLSContent> {
        let https = &self.yaml["https"];
        if https.is_badvalue() {
            return None;
        }

        self.yaml
            .check("https", &["cert", "key", "name"], &["cert", "key", "name"]);

        let cert = https.key_to_string("cert").absolute_path(config_dir);
        let key = https.key_to_string("key").absolute_path(config_dir);
        let sni = https.key_to_string("name");

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

        let var = self.yaml.key_to_string("echo").to_var();
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

    fn headers(&self) -> Setting<HeaderMap> {
        setting_value!(self.yaml["header"]);

        let hash = self.yaml.key_to_hash("header");
        let mut map = HeaderMap::new();

        for (i, (key, value)) in hash.iter().enumerate() {
            let key = key.to_string(format!("header[{}]", i));
            let value = value.to_string(&key);

            map.insert(
                key.as_str().to_header_name(),
                value.as_str().to_header_value(),
            );
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

    fn status(&self, root: &Option<PathBuf>) -> StatusPage {
        if self.yaml["status"].is_badvalue() {
            return StatusPage::default();
        }

        self.yaml
            .check("status", &["403", "404", "500", "502"], &[]);

        let parser = Parser::new(self.yaml["status"].clone());

        StatusPage {
            _403: parser.status_item(403, &root),
            _404: parser.status_item(404, &root),
            _500: parser.status_item(500, &root),
            _502: parser.status_item(502, &root),
        }
    }

    fn status_item(&self, status: usize, root: &Option<PathBuf>) -> Setting<PathBuf> {
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
            .check("proxy", &["uri", "method", "timeout", "header"], &["uri"]);

        let uri = proxy
            .key_to_multiple_string("uri")
            .iter()
            .map(|u| {
                let var = u.clone().to_var();
                var.map_none(|s| s.as_str().to_uri())
            })
            .collect::<Vec<Var<Uri>>>();

        let method = if proxy["method"].is_badvalue() {
            None
        } else {
            let method = proxy.key_to_string("method").as_str().to_method();
            Some(method)
        };

        let timeout = if proxy["timeout"].is_badvalue() {
            default::PROXY_TIMEOUT
        } else {
            proxy.key_to_string("timeout").as_str().to_duration()
        };

        let headers = Parser::new(self.yaml.clone()).headers();

        Setting::Value(Proxy {
            uri,
            method,
            timeout,
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

    fn buffer(&self) -> Setting<usize> {
        let buffer = &self.yaml["buffer"];
        setting_value!(buffer);

        let size = self.yaml.key_to_string("buffer").as_str().to_size();
        Setting::Value(size)
    }

    fn break_(&self) -> bool {
        if self.yaml["break"].is_badvalue() {
            return false;
        }

        self.yaml.key_to_bool("break")
    }
}
