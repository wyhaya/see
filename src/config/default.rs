use crate::option::{Directory, Method};
use crate::util::home_dir;
use crate::{ServerConfig, Setting, SiteConfig};
use async_compression::Level;
use clap::{crate_name, crate_version};
use hyper::Method as HttpMethod;
use std::net::SocketAddr;
use std::path::PathBuf;

// Package

pub const SERVER_NAME: &str = crate_name!();

pub const VERSION: &str = crate_version!();

// Config file

pub fn config_path() -> String {
    home_dir().join(".see.conf").display().to_string()
}

// Server config

pub const AUTH_MESSAGE: &str = "Basic realm=\"User Visible Realm\"";

pub const ALLOW_METHODS: [HttpMethod; 2] = [HttpMethod::GET, HttpMethod::HEAD];

pub const COMPRESS_LEVEL: Level = Level::Default;

pub const COMPRESS_EXTENSIONS: [&str; 5] = ["html", "css", "js", "json", "png"];

pub const INDEX: [&str; 1] = ["index.html"];

pub const DIRECTORY_TIME_FORMAT: &str = "%Y-%m-%d %H:%M";

pub const BUF_SIZE: usize = 16 * 1024;

// Should be synchronized with src/var.rs
pub const LOG_FORMAT: &str = "$`method` $`header_host`$`path`$`query` $`header_user-agent`";

// Quick start

pub fn bind_addr() -> SocketAddr {
    "0.0.0.0:80".parse::<SocketAddr>().unwrap()
}

pub fn quick_start_config(root: PathBuf, listen: SocketAddr) -> ServerConfig {
    let site = SiteConfig {
        root: Some(root),
        directory: Setting::Value(Directory {
            time: Some(DIRECTORY_TIME_FORMAT.to_string()),
            size: true,
        }),
        method: Setting::Value(Method::new(ALLOW_METHODS.to_vec())),
        ..Default::default()
    };

    ServerConfig {
        listen,
        tls: None,
        sites: vec![site],
    }
}
