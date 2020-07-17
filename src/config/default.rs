use crate::compress::CompressLevel;
use crate::{Directory, HostMatcher, ServerConfig, Setting, SiteConfig, StatusPage};
use hyper::Method;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

// Package

pub const SERVER_NAME: &str = env!("CARGO_PKG_NAME");

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Config file

pub const CONFIG_PATH: [&str; 2] = [".see", "config.yml"];

pub const PID_PATH: [&str; 2] = [".see", "pid"];

// Server config

pub const AUTH_MESSAGE: &str = "Basic realm=\"User Visible Realm\"";

pub const ALLOW_METHODS: [Method; 2] = [Method::GET, Method::HEAD];

pub const COMPRESS_LEVEL: CompressLevel = CompressLevel::Default;

pub const COMPRESS_EXTENSIONS: [&str; 5] = ["html", "css", "js", "json", "png"];

pub const INDEX: [&str; 2] = ["index.html", "index.htm"];

pub const DIRECTORY_TIME_FORMAT: &str = "%Y-%m-%d %H:%M";

pub const BUF_SIZE: usize = 16 * 1024;

// pub const PROXY_BUF_SIZE: usize = 8 * 1024;

// pub const TIMEOUT: u64 = 5000;

pub const PROXY_TIMEOUT: Duration = Duration::from_millis(5000);

// Should be synchronized with src/var.rs
pub const LOG_FORMAT: &str =
    "${request_method} ${request_header_host}${request_uri} ${request_header_user-agent}";

// Quick start

pub const START_PORT: i64 = 80;

pub fn quick_start_config(root: PathBuf, listen: SocketAddr) -> ServerConfig {
    ServerConfig {
        listen,
        tls: None,
        sites: vec![SiteConfig {
            sni_name: None,
            host: HostMatcher::default(),
            root: Some(root),
            echo: Setting::None,
            file: Setting::None,
            index: Setting::None,
            directory: Setting::Value(Directory {
                time: Some(DIRECTORY_TIME_FORMAT.to_string()),
                size: true,
            }),
            headers: Setting::None,
            rewrite: Setting::None,
            compress: Setting::None,
            methods: Setting::Value(ALLOW_METHODS.to_vec()),
            auth: Setting::None,
            extensions: Setting::None,
            status: StatusPage::default(),
            proxy: Setting::None,
            log: Setting::None,
            ip: Setting::None,
            location: Vec::with_capacity(0),
        }],
    }
}
