pub mod default;
pub mod tls;
pub mod transform;

mod parser;
mod setting;
mod var;

pub use setting::*;
pub use var::Var;

use crate::conf::ConfParser;
use crate::exit;
use crate::matcher::{HostMatcher, IpMatcher, LocationMatcher};
use crate::option::{Auth, Compress, Directory, Index, Logger, Method, Proxy, Rewrite};
use hyper::header::{HeaderName, HeaderValue};
use hyper::StatusCode;
use parser::parse_server;
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio_rustls::TlsAcceptor;

pub type Headers = HashMap<HeaderName, Var<HeaderValue>>;
pub type ErrorPage = Setting<HashMap<StatusCode, Setting<PathBuf>>>;

#[derive(Clone)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub tls: Option<TlsAcceptor>,
    pub sites: Vec<SiteConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct SiteConfig {
    pub host: HostMatcher,
    pub root: Option<PathBuf>,
    pub echo: Setting<Var<String>>,
    pub file: Setting<PathBuf>,
    pub index: Setting<Index>,
    pub directory: Setting<Directory>,
    pub headers: Setting<Headers>,
    pub rewrite: Setting<Rewrite>,
    pub compress: Setting<Compress>,
    pub method: Setting<Method>,
    pub auth: Setting<Auth>,
    pub try_: Setting<Vec<Var<String>>>,
    pub error: ErrorPage,
    pub proxy: Setting<Proxy>,
    pub log: Setting<Logger>,
    pub ip: Setting<IpMatcher>,
    pub location: Vec<Location>,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub location: LocationMatcher,
    pub break_: bool,
    pub root: Option<PathBuf>,
    pub echo: Setting<Var<String>>,
    pub file: Setting<PathBuf>,
    pub index: Setting<Index>,
    pub directory: Setting<Directory>,
    pub headers: Setting<Headers>,
    pub rewrite: Setting<Rewrite>,
    pub compress: Setting<Compress>,
    pub method: Setting<Method>,
    pub auth: Setting<Auth>,
    pub try_: Setting<Vec<Var<String>>>,
    pub error: ErrorPage,
    pub proxy: Setting<Proxy>,
    pub log: Setting<Logger>,
    pub ip: Setting<IpMatcher>,
}

impl ServerConfig {
    pub async fn new(path: &str) -> Vec<Self> {
        let config_dir = Path::new(&path)
            .parent()
            .unwrap_or_else(|| exit!("Cannot get configuration file directory"));

        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| exit!("Read '{}' failed\n{:?}", path, err));

        let conf = ConfParser::parse(&content)
            .unwrap_or_else(|err| exit!("Parsing config file failed\n{:?}", err));

        parse_server(&conf, config_dir).await
    }
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
            if !item.method.is_none() {
                self.method = item.method;
            }
            if !item.auth.is_none() {
                self.auth = item.auth;
            }
            if !item.try_.is_none() {
                self.try_ = item.try_;
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
            if !item.error.is_none() {
                if item.error.is_off() {
                    self.error = Setting::Off;
                } else {
                    let mut hash = self.error.into_value();
                    hash.extend(item.error.into_value());
                    self.error = Setting::Value(hash);
                }
            }
            if item.break_ {
                break;
            }
        }

        self.location = Vec::with_capacity(0);
        self
    }
}
