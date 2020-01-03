use crate::var::{ToVar, Var};
use crate::*;
use base64::encode;
use globset::Glob;
use hyper::{Method, Uri};
use matcher::{HostMatcher, LocationMatcher};
use regex::Regex;
use std::fmt::Display;
use std::fs;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls::rustls::internal::pemfile::{certs, rsa_private_keys};
use tokio_rustls::rustls::{NoClientAuth, ServerConfig as Config};
use tokio_rustls::TlsAcceptor;
use yaml_rust::yaml::Hash;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug, Clone)]
pub enum Setting<T> {
    None,
    Off,
    Value(T),
}

impl<T> Setting<T> {
    pub fn has_value(&self) -> bool {
        match self {
            Setting::None => false,
            _ => true,
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
    pub sites: Vec<SiteConfig>,
}

#[derive(Clone)]
pub struct SiteConfig {
    pub https: Option<TLSConfig>,
    pub host: HostMatcher,
    pub root: PathBuf,
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
    pub location: Vec<Location>,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub location: LocationMatcher,
    pub _break: bool,
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
}

#[derive(Clone)]
pub struct TLSConfig {
    pub acceptor: TlsAcceptor,
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

impl Default for RewriteStatus {
    fn default() -> Self {
        RewriteStatus::_302
    }
}

impl Rewrite {
    fn parse(rewrite: String) -> Rewrite {
        let mut split = rewrite.split_whitespace();

        let location = split
            .next()
            .unwrap_or_else(|| exit!("Could not find redirected url"));

        let status = split
            .next()
            .map(|s| match s {
                "301" => RewriteStatus::_301,
                "302" => RewriteStatus::_302,
                _ => exit!("Wrong redirect type `{}`, optional value: `301` `302`", s),
            })
            .unwrap_or_default();

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
    pub mode: ContentEncoding,
    pub extensions: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ContentEncoding {
    Auto(u32),
    Gzip(u32),
    Deflate(u32),
    Br(u32),
    None,
}

impl ContentEncoding {
    pub fn from(mode: &str, level: u32) -> Self {
        match mode {
            "auto" => ContentEncoding::Auto(level),
            "gzip" => ContentEncoding::Gzip(level),
            "deflate" => ContentEncoding::Deflate(level),
            "br" => ContentEncoding::Br(level),
            _ => exit!(
                "Wrong compression mode `{}`, optional value: `auto` `gzip` `deflate` `br`",
                mode
            ),
        }
    }

    pub fn parse_mode(&self, modes: Vec<&str>) -> Self {
        match self {
            ContentEncoding::Auto(level) => {
                for mode in modes {
                    match mode {
                        "gzip" => return ContentEncoding::Gzip(*level),
                        "deflate" => return ContentEncoding::Deflate(*level),
                        "br" => return ContentEncoding::Br(*level),
                        _ => {}
                    };
                }
            }
            ContentEncoding::Gzip(level) => {
                for mode in modes {
                    if mode == "gzip" {
                        return ContentEncoding::Gzip(*level);
                    }
                }
            }
            ContentEncoding::Deflate(level) => {
                for mode in modes {
                    if mode == "deflate" {
                        return ContentEncoding::Deflate(*level);
                    }
                }
            }
            ContentEncoding::Br(level) => {
                for mode in modes {
                    if mode == "br" {
                        return ContentEncoding::Br(*level);
                    }
                }
            }
            _ => {}
        }
        ContentEncoding::None
    }

    pub fn to_header_value(&self) -> HeaderValue {
        let s = match self {
            ContentEncoding::Gzip(_) => "gzip",
            ContentEncoding::Deflate(_) => "deflate",
            ContentEncoding::Br(_) => "br",
            _ => "",
        };

        HeaderValue::from_static(s)
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

macro_rules! is_none {
    ($yaml: expr, $bad: expr) => {
        if $yaml.is_badvalue() {
            $bad;
        }
    };
    ($yaml: expr, $bad: expr, $ok: expr) => {{
        if $yaml.is_badvalue() {
            $bad
        } else {
            $ok
        }
    }};
}

pub trait ForceTo {
    fn to_glob(&self) -> Glob;
    fn to_header_name(&self) -> HeaderName;
    fn to_header_value(&self) -> HeaderValue;
    fn to_method(&self) -> Method;
    fn to_regex(&self) -> Regex;
    fn to_socket_addr(&self) -> SocketAddr;
    fn to_uri(&self) -> Uri;
    fn to_strftime(&self);
}

impl ForceTo for &str {
    fn to_glob(&self) -> Glob {
        Glob::new(self)
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to glob matcher\n{}", self, err))
    }

    fn to_header_name(&self) -> HeaderName {
        HeaderName::from_str(self)
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to http header name\n{}", self, err))
    }

    fn to_header_value(&self) -> HeaderValue {
        HeaderValue::from_str(self)
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to http header value\n{}", self, err))
    }

    fn to_method(&self) -> Method {
        Method::from_str(self)
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to http method\n{}", self, err))
    }

    fn to_regex(&self) -> Regex {
        Regex::new(self)
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to regular expression\n{}", self, err))
    }

    fn to_socket_addr(&self) -> SocketAddr {
        if let Ok(addr) = self.parse::<SocketAddr>() {
            return addr;
        }
        if let Ok(port) = self.parse::<i64>() {
            if let Ok(addr) = format!("0.0.0.0:{}", port).parse::<SocketAddr>() {
                return addr;
            }
        }
        exit!("Cannot parse `{}` to SocketAddr", self)
    }

    fn to_uri(&self) -> Uri {
        self.parse::<Uri>()
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to http uri\n{}", self, err))
    }

    fn to_strftime(&self) {
        time::now()
            .strftime(self)
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to time format\n{}", self, err));
    }
}

trait YamlExtend {
    fn check(&self, name: &str, keys: &[&str], must: &[&str]);
    fn try_to_string(&self) -> Option<String>;
    fn to_string<T: Display>(&self, msg: T) -> String;
    fn key_to_bool(&self, key: &str) -> bool;
    fn key_to_hash(&self, key: &str) -> &Hash;
    fn key_to_number(&self, key: &str) -> u64;
    fn key_to_string(&self, key: &str) -> String;
    fn key_to_multiple_string(&self, key: &str) -> Vec<String>;
}

impl YamlExtend for Yaml {
    fn check(&self, name: &str, keys: &[&str], must: &[&str]) {
        let hash = self.key_to_hash(name);

        // Disallowed key
        for (key, _) in hash {
            let key = key.to_string(format!("{} 'key'", name));
            let find = keys.iter().any(|item| *item == &key);
            if !find {
                exit!("Check failed, unknown directive `{}` in '{}'", key, name)
            }
        }

        // Required key
        for must in must {
            is_none!(
                self[name][must.clone()],
                exit!("Missing '{}' in '{}'", must, name)
            )
        }
    }

    fn try_to_string(&self) -> Option<String> {
        if let Some(s) = self.as_str() {
            return Some(s.to_string());
        }
        if let Some(s) = self.as_i64() {
            return Some(s.to_string());
        }
        if let Some(s) = self.as_f64() {
            return Some(s.to_string());
        }
        None
    }

    fn to_string<T: Display>(&self, msg: T) -> String {
        self.try_to_string().unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'string', but found:\n{:#?}",
                msg,
                self
            )
        })
    }

    fn key_to_bool(&self, key: &str) -> bool {
        self.as_bool().unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'boolean', but found:\n{:#?}",
                key,
                self[key]
            )
        })
    }

    fn key_to_hash(&self, key: &str) -> &Hash {
        self[key].as_hash().unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'hash', but found:\n{:#?}",
                key,
                self[key]
            )
        })
    }

    fn key_to_number(&self, key: &str) -> u64 {
        self[key].as_i64().map(|n| n as u64).unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'number', but found:\n{:#?}",
                key,
                self[key]
            )
        })
    }

    fn key_to_string(&self, key: &str) -> String {
        self[key].try_to_string().unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'string', but found:\n{:#?}",
                key,
                self[key]
            )
        })
    }

    fn key_to_multiple_string(&self, key: &str) -> Vec<String> {
        is_none!(self[key], return vec![]);

        let mut result = vec![];
        match self[key].as_vec() {
            Some(items) => {
                for (i, item) in items.iter().enumerate() {
                    let s = item.to_string(format!("{}[{}]", key, i));
                    result.push(s);
                }
            }
            None => {
                for line in self.key_to_string(key).lines() {
                    for s in line.split_whitespace() {
                        result.push(s.to_string());
                    }
                }
            }
        }

        dedup(result)
    }
}

fn dedup<T: Eq>(vec: Vec<T>) -> Vec<T> {
    let mut new = vec![];
    for item in vec {
        let has = new.iter().any(|s| s == &item);
        if !has {
            new.push(item);
        }
    }
    new
}

pub trait ToAbsolutePath {
    fn to_absolute_path<P: AsRef<Path>>(&self, root: P) -> PathBuf;
}

impl ToAbsolutePath for String {
    fn to_absolute_path<P: AsRef<Path>>(&self, root: P) -> PathBuf {
        let path = PathBuf::from(self);
        if path.is_absolute() {
            path
        } else {
            root.as_ref().join(self)
        }
    }
}

impl ServerConfig {
    pub fn new(path: &str) -> Vec<ServerConfig> {
        let parent = Path::new(&path)
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

        for server in servers {
            server.check(
                "server",
                &[
                    "listen",
                    // site
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
                    "location",
                ],
                &["listen", "root"],
            );

            let parser = Parser::new(server["server"].clone());
            let root = parser.root(&parent);
            let listens = parser.listen();

            let site = SiteConfig {
                https: parser.https(&parent),
                host: parser.host(),
                root: root.clone(),
                echo: parser.echo(),
                file: parser.file(&root),
                index: parser.index(true),
                directory: parser.directory(),
                headers: parser.headers(),
                rewrite: parser.rewrite(),
                compress: parser.compress(),
                extensions: parser.extensions(),
                methods: parser.methods(true),
                status: parser.status(&root),
                proxy: parser.proxy(),
                auth: parser.auth(),
                location: parser.location(root.clone()),
            };

            for listen in listens {
                let find = configs.iter().position(|item| item.listen == listen);
                match find {
                    Some(i) => {
                        configs[i].sites.push(site.clone());
                    }
                    None => configs.push(ServerConfig {
                        listen,
                        sites: vec![site.clone()],
                    }),
                }
            }
        }

        configs
    }
}

struct Parser {
    yaml: Yaml,
}

impl Parser {
    fn new(yaml: Yaml) -> Self {
        Parser { yaml }
    }

    fn listen(&self) -> Vec<SocketAddr> {
        let vec = self
            .yaml
            .key_to_multiple_string("listen")
            .iter()
            .map(|s| s.as_str().to_socket_addr())
            .collect::<Vec<SocketAddr>>();

        dedup(vec)
    }

    fn https(&self, parent: &Path) -> Option<TLSConfig> {
        let https = &self.yaml["https"];
        is_none!(https, return None);

        self.yaml.check("https", &["cert", "key"], &["cert", "key"]);

        let certs = {
            let p = https.key_to_string("cert").to_absolute_path(parent);

            let file = fs::File::open(&p)
                .unwrap_or_else(|err| exit!("Cannot open file: {}\n{:?}", p.display(), err));

            certs(&mut BufReader::new(file))
                .unwrap_or_else(|_| exit!("invalid cert: {}", p.display()))
        };

        let mut keys = {
            let p = https.key_to_string("key").to_absolute_path(parent);

            let file = fs::File::open(&p)
                .unwrap_or_else(|err| exit!("Cannot open file: {}\n{:?}", p.display(), err));

            rsa_private_keys(&mut BufReader::new(file))
                .unwrap_or_else(|_| exit!("invalid key: {}", p.display()))
        };

        let mut config = Config::new(NoClientAuth::new());
        config
            .set_single_cert(certs, keys.remove(0))
            .unwrap_or_else(|err| exit!("TLSError: {:?}", err));

        Some(TLSConfig {
            acceptor: TlsAcceptor::from(Arc::new(config)),
        })
    }

    fn host(&self) -> HostMatcher {
        let vec = self.yaml.key_to_multiple_string("host");
        HostMatcher::new(vec)
    }

    fn root(&self, parent: &Path) -> PathBuf {
        self.yaml.key_to_string("root").to_absolute_path(parent)
    }

    fn echo(&self) -> Setting<Var<String>> {
        setting_value!(self.yaml["echo"]);
        let val = self.yaml.key_to_string("echo").to_var();

        Setting::Value(val)
    }

    fn file(&self, root: &PathBuf) -> Setting<PathBuf> {
        setting_value!(self.yaml["file"]);
        let val = self.yaml.key_to_string("file").to_absolute_path(root);

        Setting::Value(val)
    }

    fn index(&self, set_default: bool) -> Setting<Vec<String>> {
        let index = &self.yaml["index"];
        if set_default {
            setting_none!(
                index,
                default::INDEX.iter().map(|i| i.to_string()).collect()
            );
        } else {
            setting_none!(index);
        }

        setting_off!(index);

        let val = self.yaml.key_to_multiple_string("index");

        Setting::Value(val)
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

        let time = is_none!(directory["time"], None, {
            match directory["time"].as_bool() {
                Some(b) => {
                    if b {
                        Some(default::TIME_FORMAT.to_string())
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
        });

        let size = is_none!(directory["size"], false, directory.key_to_bool("size"));

        Setting::Value(Directory { time, size })
    }

    fn headers(&self) -> Setting<HeaderMap> {
        setting_value!(self.yaml["header"]);

        let header = self.yaml.key_to_hash("header");
        let mut map = HeaderMap::new();

        for (i, (key, value)) in header.iter().enumerate() {
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
        let val = self.yaml.key_to_string("rewrite");

        Setting::Value(Rewrite::parse(val))
    }

    fn compress(&self) -> Setting<Compress> {
        let compress = &self.yaml["compress"];
        setting_value!(compress);

        // compress: true
        if compress.as_bool().is_some() {
            return Setting::Value(Compress {
                mode: ContentEncoding::Auto(default::COMPRESS_LEVEL),
                extensions: default::COMPRESS_EXTENSIONS
                    .iter()
                    .map(|e| e.to_string())
                    .collect(),
            });
        }

        self.yaml
            .check("compress", &["mode", "level", "extension"], &[]);

        let level = is_none!(compress["level"], default::COMPRESS_LEVEL, {
            let level = compress.key_to_number("level");
            if level > 9 {
                exit!("Compress level should be an integer between 0-9")
            }
            level as u32
        });

        let mode = is_none!(compress["mode"], ContentEncoding::Auto(level), {
            let mode = compress.key_to_string("mode");
            ContentEncoding::from(&mode, level)
        });

        let extensions = is_none!(
            compress["extension"],
            {
                default::COMPRESS_EXTENSIONS
                    .iter()
                    .map(|e| e.to_string())
                    .collect()
            },
            compress.key_to_multiple_string("extension")
        );

        Setting::Value(Compress { mode, extensions })
    }

    fn extensions(&self) -> Setting<Vec<String>> {
        setting_value!(self.yaml["extension"]);
        let extensions = self.yaml.key_to_multiple_string("extension");

        Setting::Value(extensions)
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
            .collect::<Vec<Method>>();

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

    fn status(&self, root: &PathBuf) -> StatusPage {
        is_none!(self.yaml["status"], return StatusPage::default());

        self.yaml
            .check("status", &["403", "404", "500", "502"], &[]);
        let parse = Parser::new(self.yaml["status"].clone());

        StatusPage {
            _403: parse.status_item(403, &root),
            _404: parse.status_item(404, &root),
            _500: parse.status_item(500, &root),
            _502: parse.status_item(502, &root),
        }
    }

    fn status_item(&self, status: usize, root: &PathBuf) -> Setting<PathBuf> {
        setting_value!(self.yaml[status]);
        let s = self.yaml[status].to_string(status);

        Setting::Value(s.to_absolute_path(root))
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

        let method = is_none!(proxy["method"], None, Some(proxy.key_to_string("method")))
            .map(|m| m.as_str().to_method());

        let timeout = is_none!(
            proxy["timeout"],
            default::PROXY_TIMEOUT,
            proxy.key_to_number("timeout")
        );

        let headers = Parser::new(self.yaml.clone()).headers();

        Setting::Value(Proxy {
            uri,
            method,
            timeout: Duration::from_millis(timeout),
            headers,
        })
    }

    fn location(&self, parent: PathBuf) -> Vec<Location> {
        is_none!(self.yaml["location"], return vec![]);

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
                ],
                &[],
            );

            let parser = Parser::new(server.clone());

            let lr = parser.location_root();
            let root = match &lr {
                Some(a) => a.to_absolute_path(&parent),
                None => parent.clone(),
            };
            let d = lr.map(|s| s.to_absolute_path(&parent));

            vec.push(Location {
                location: LocationMatcher::new(route.as_str()),
                _break: parser.location_break(),
                root: d,
                echo: parser.echo(),
                file: parser.file(&root),
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
            });
        }
        vec
    }

    fn location_break(&self) -> bool {
        is_none!(self.yaml["break"], return false);
        self.yaml.key_to_bool("break")
    }

    fn location_root(&self) -> Option<String> {
        is_none!(self.yaml["root"], return None);
        Some(self.yaml.key_to_string("root"))
    }
}
