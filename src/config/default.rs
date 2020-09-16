use crate::compress::CompressLevel;
use crate::util::home_dir;
use crate::{Directory, ServerConfig, Setting, SiteConfig};
use hyper::Method;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

// Package

pub const SERVER_NAME: &str = env!("CARGO_PKG_NAME");

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Config file

pub fn config_path() -> PathBuf {
    home_dir().join(".see.yml")
}

// Server config

pub const AUTH_MESSAGE: &str = "Basic realm=\"User Visible Realm\"";

pub const ALLOW_METHODS: [Method; 2] = [Method::GET, Method::HEAD];

pub const COMPRESS_LEVEL: CompressLevel = CompressLevel::Default;

pub const COMPRESS_EXTENSIONS: [&str; 5] = ["html", "css", "js", "json", "png"];

pub const INDEX: [&str; 2] = ["index.html", "index.htm"];

pub const DIRECTORY_TIME_FORMAT: &str = "%Y-%m-%d %H:%M";

pub const BUF_SIZE: usize = 16 * 1024;

pub const CONNECT_TIMEOUT: Duration = Duration::from_millis(5000);

// Should be synchronized with src/var.rs
pub const LOG_FORMAT: &str = "${method} ${header_host}${path}${query} ${header_user-agent}";

// Quick start

pub const START_PORT: u16 = 80;

pub fn quick_start_config(root: PathBuf, listen: SocketAddr) -> ServerConfig {
    let mut site = SiteConfig::default();
    site.root = Some(root);
    site.directory = Setting::Value(Directory {
        time: Some(DIRECTORY_TIME_FORMAT.to_string()),
        size: true,
    });
    site.methods = Setting::Value(ALLOW_METHODS.to_vec());

    ServerConfig {
        listen,
        tls: None,
        sites: vec![site],
    }
}
