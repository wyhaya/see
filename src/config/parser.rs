use super::{ErrorPage, Headers, Location, ServerConfig, SiteConfig};
use crate::conf::{Block, BlockExt, DirectiveExt};
use crate::util::{self, absolute_path};
use crate::{check_none, check_off, check_value, compress, config, exit, matcher, option};
use compress::CompressMode;
use config::tls::{create_sni_server_config, TLSContent};
use config::{default, Setting, Var};
use matcher::{HostMatcher, IpMatcher, LocationMatcher};
use option::{Auth, Compress, Directory, Index, Logger, Method, Proxy, Rewrite, RewriteStatus};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Display;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub trait ParseResultExt<T> {
    fn unwrap_exit(self, line: usize) -> T;
}

impl<T, E: Display> ParseResultExt<T> for Result<T, E> {
    fn unwrap_exit(self, line: usize) -> T {
        match self {
            Ok(data) => data,
            Err(err) => exit!("[line:{}] {}", line, err),
        }
    }
}

pub async fn parse_server<P: AsRef<Path>>(block: &Block, config_dir: P) -> Vec<ServerConfig> {
    block.check(&["server"], &["server"], &["server"]);

    let mut configs: Vec<ServerConfig> = vec![];
    let mut tls_configs: Vec<(SocketAddr, Vec<TLSContent>)> = vec![];
    for d in block.directives() {
        let server = d.to_block();
        server.check(
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
                "try",
                "error",
                "proxy",
                "log",
                "ip",
                // location
                "@",
                "~",
                "^",
                "$",
            ],
            &["listen"],
            &["@", "~", "^", "$"],
        );
        let listens = parse_listen(server);
        let host = parse_host(server);
        let https = parse_https(server, config_dir.as_ref(), host.get_raw());

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

        let root = parse_root(server, config_dir.as_ref());

        let site = SiteConfig {
            host,
            root: root.clone(),
            echo: parse_echo(server),
            file: parse_file(server, &config_dir),
            index: parse_index(server, true),
            directory: parse_directory(server),
            headers: parse_header(server),
            rewrite: parse_rewrite(server),
            compress: parse_compress(server),
            try_: parse_try(server),
            method: parse_method(server, true),
            error: parse_error(server, &root),
            proxy: parse_proxy(server),
            log: parse_log(server, &config_dir).await,
            ip: parse_ip(server),
            auth: parse_auth(server),
            location: parse_location(server, &config_dir, root).await,
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
        let i = configs
            .iter()
            .position(|item| item.listen == listen)
            .unwrap();
        // todo
        let t = create_sni_server_config(group).unwrap_exit(0);
        configs[i].tls = Some(t);
    }

    configs
}

async fn parse_location<P: AsRef<Path>>(
    block: &Block,
    config_dir: P,
    parent_root: Option<PathBuf>,
) -> Vec<Location> {
    let locations = block.get_all_by_names(&["@", "~", "^", "$"]);
    if locations.is_empty() {
        return vec![];
    }
    let root = match parse_root(&block, config_dir.as_ref()) {
        Some(p) => Some(p),
        None => parent_root.clone(),
    };
    let mut vec = vec![];
    for d in locations {
        let (route, location) = d.to_value_block();
        location.check(
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
                "try",
                "error",
                "proxy",
                "log",
                "ip",
            ],
            &[],
            &[],
        );

        vec.push(Location {
            location: match d.name() {
                "@" => LocationMatcher::glob(route).unwrap_exit(d.line()),
                "~" => LocationMatcher::regex(route).unwrap_exit(d.line()),
                "^" => LocationMatcher::start(route),
                "$" => LocationMatcher::end(route),
                _ => unreachable!(),
            },
            break_: parse_break(location),
            root: root.clone(),
            echo: parse_echo(location),
            file: parse_file(location, &config_dir),
            index: parse_index(location, false),
            directory: parse_directory(location),
            headers: parse_header(location),
            rewrite: parse_rewrite(location),
            compress: parse_compress(location),
            method: parse_method(location, false),
            auth: parse_auth(location),
            try_: parse_try(location),
            error: parse_error(location, &root),
            proxy: parse_proxy(location),
            log: parse_log(location, &config_dir).await,
            ip: parse_ip(location),
        });
    }

    vec
}

fn parse_method(block: &Block, set_default: bool) -> Setting<Method> {
    check_off!(block, "method");
    if set_default {
        check_none!(
            block,
            "method",
            Method::new(default::ALLOW_METHODS.to_vec())
        );
    } else {
        check_none!(block, "method");
    }
    let methods = block["method"]
        .to_multiple_str()
        .iter()
        .map(|s| util::to_method(s).unwrap_exit(block["method"].line()))
        .collect();
    Setting::Value(Method::new(methods))
}

fn parse_host(block: &Block) -> HostMatcher {
    HostMatcher::new(
        block
            .get("host")
            .map(|d| d.to_multiple_str())
            .unwrap_or_default(),
    )
}

// todo
fn parse_try(block: &Block) -> Setting<Vec<Var<String>>> {
    check_value!(block, "try");
    let vec = block["try"]
        .to_multiple_str()
        .iter()
        .map(|s| Var::from(s))
        .collect::<Vec<Var<String>>>();

    Setting::Value(vec)
}

fn parse_auth(block: &Block) -> Setting<Auth> {
    check_value!(block, "auth");
    let auth = block["auth"].to_block();
    auth.check(&["user", "password"], &["user", "password"], &[]);
    Setting::Value(Auth::basic(
        auth["user"].to_source_str(),
        auth["password"].to_source_str(),
    ))
}

fn parse_break(block: &Block) -> bool {
    block.get("break").map(|d| d.to_bool()).unwrap_or_default()
}

fn parse_echo(block: &Block) -> Setting<Var<String>> {
    check_value!(block, "echo");
    Setting::Value(Var::from(block["echo"].to_source_str()))
}

fn parse_file<P: AsRef<Path>>(block: &Block, root: P) -> Setting<PathBuf> {
    check_value!(block, "file");
    let buf = absolute_path(block["file"].to_source_str(), root);
    Setting::Value(buf)
}

fn parse_rewrite(block: &Block) -> Setting<Rewrite> {
    check_value!(block, "rewrite");

    if block["rewrite"].is_string() {
        let r = Rewrite::new(block["rewrite"].to_str(), RewriteStatus::default())
            .unwrap_exit(block["rewrite"].line());
        return Setting::Value(r);
    }

    let rewrite = block["rewrite"].to_block();
    rewrite.check(&["location", "status"], &["location"], &[]);

    let status = rewrite
        .get("status")
        .map(|d| RewriteStatus::from_str(d.to_str()).unwrap_exit(d.line()))
        .unwrap_or_default();

    Setting::Value(
        Rewrite::new(rewrite["location"].to_str(), status).unwrap_exit(rewrite["location"].line()),
    )
}

// todo
// error line
fn parse_ip(block: &Block) -> Setting<IpMatcher> {
    check_value!(block, "ip");

    let ip = block["ip"].to_block();
    ip.check(&["allow", "deny"], &[], &[]);

    let allow = ip
        .get("allow")
        .map(|d| d.to_multiple_str())
        .unwrap_or_default();

    let deny = ip
        .get("deny")
        .map(|d| d.to_multiple_str())
        .unwrap_or_default();

    Setting::Value(IpMatcher::new(allow, deny).unwrap_exit(ip.line()))
}

fn parse_root<P: AsRef<Path>>(block: &Block, config_dir: P) -> Option<PathBuf> {
    if block.get("root").is_none() {
        return None;
    }
    let path = absolute_path(block["root"].to_str(), config_dir);
    Some(path)
}

fn parse_index(block: &Block, set_default: bool) -> Setting<Index> {
    check_off!(block, "index");
    if set_default {
        check_none!(
            block,
            "index",
            Index::new(default::INDEX.iter().map(|i| (*i).to_string()).collect())
        );
    } else {
        check_none!(block, "index");
    }
    let indexs = block["index"]
        .to_multiple_str()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    Setting::Value(Index::new(indexs))
}

fn parse_listen(block: &Block) -> Vec<SocketAddr> {
    block["listen"]
        .to_multiple_str()
        .iter()
        .map(|s| util::to_socket_addr(s).unwrap_exit(block["listen"].line()))
        .collect::<BTreeSet<SocketAddr>>()
        .into_iter()
        .collect()
}

fn parse_header(block: &Block) -> Setting<Headers> {
    check_value!(block, "header");
    let header = block["header"].to_block().directives();
    let mut map = HashMap::new();
    for d in header {
        let header_name = util::to_header_name(d.name()).unwrap_exit(d.line());
        let value = Var::from(d.to_multiple_str().join(" "));
        let header_value = value.map_none(|s| util::to_header_value(&s).unwrap_exit(d.line()));

        map.insert(header_name, header_value);
    }

    Setting::Value(map)
}

fn parse_directory(block: &Block) -> Setting<Directory> {
    check_value!(block, "directory");

    // directory on
    if block["directory"].is_on() {
        return Setting::Value(Directory {
            time: None,
            size: false,
        });
    }

    let directory = block["directory"].to_block();
    directory.check(&["time", "size"], &[], &[]);

    let time = match directory.get("time") {
        Some(d) => {
            if let Some(b) = d.as_bool() {
                if b {
                    Some(default::DIRECTORY_TIME_FORMAT.to_string())
                } else {
                    None
                }
            } else {
                let format = d.to_source_str();
                util::check_strftime(format).unwrap_exit(d.line());
                Some(format.to_string())
            }
        }
        None => None,
    };

    let size = directory
        .get("size")
        .map(|d| d.to_bool())
        .unwrap_or_default();

    Setting::Value(Directory { time, size })
}

fn parse_proxy(block: &Block) -> Setting<Proxy> {
    check_value!(block, "proxy");
    let proxy = block["proxy"].to_block();
    proxy.check(&["url", "method", "header"], &["url"], &[]);

    let url_str = proxy["url"].to_str();
    let url = Var::from(url_str).map_none(|s| util::to_url(&s).unwrap_exit(proxy["url"].line()));

    let method = proxy
        .get("method")
        .map(|d| util::to_method(d.to_str()).unwrap_exit(d.line()));

    Setting::Value(Proxy {
        url,
        method,
        headers: parse_header(proxy),
    })
}

fn parse_compress(block: &Block) -> Setting<Compress> {
    check_value!(block, "compress");

    // compress on
    if block["compress"].is_on() {
        return Setting::Value(Compress {
            modes: vec![CompressMode::Auto(default::COMPRESS_LEVEL)],
            extensions: default::COMPRESS_EXTENSIONS
                .iter()
                .map(|e| (*e).to_string())
                .collect(),
        });
    }

    let compress = block["compress"].to_block();
    compress.check(&["mode", "level", "extension"], &[], &[]);

    let level = match compress.get("level") {
        Some(d) => util::to_compress_level(d.to_str()).unwrap_exit(d.line()),
        None => default::COMPRESS_LEVEL,
    };

    let modes = match compress.get("mode") {
        Some(d) => {
            let mode = d.to_multiple_str();
            mode.iter()
                .map(|mode| CompressMode::new(mode, level).unwrap_exit(d.line()))
                .collect()
        }
        None => vec![CompressMode::Auto(level)],
    };

    let extensions = match compress.get("extension") {
        Some(d) => d
            .to_multiple_str()
            .iter()
            .map(|e| (*e).to_string())
            .collect(),
        None => default::COMPRESS_EXTENSIONS
            .iter()
            .map(|e| (*e).to_string())
            .collect(),
    };

    Setting::Value(Compress { modes, extensions })
}

// todo
fn parse_https(block: &Block, config_dir: &Path, hostname: Vec<&String>) -> Option<TLSContent> {
    if block.get("https").is_none() {
        return None;
    }
    let https = block["https"].to_block();
    https.check(&["cert", "key"], &["cert", "key"], &[]);

    let cert = absolute_path(https["cert"].to_str(), config_dir);
    let key = absolute_path(https["key"].to_str(), config_dir);

    if hostname.is_empty() {
        exit!("Missing 'host'");
    }

    Some(TLSContent {
        cert,
        key,
        sni: hostname[0].clone(),
    })
}

async fn parse_log<P: AsRef<Path>>(block: &Block, root: P) -> Setting<Logger> {
    check_value!(block, "log");

    if block["log"].is_string() {
        let path = absolute_path(block["log"].to_str(), root);
        let logger = Logger::new(default::LOG_FORMAT.to_string())
            .file(path)
            .await
            .unwrap_or_else(|err| exit!("Init logger failed:\n{:?}", err));

        return Setting::Value(logger);
    }

    let log = block["log"].to_block();
    log.check(&["mode", "file", "format"], &["mode"], &[]);

    let format_ = match log.get("format") {
        Some(d) => d.to_str(),
        None => default::LOG_FORMAT,
    };

    let mode = log["mode"].to_str();
    match mode {
        "stdout" => Setting::Value(Logger::new(format_).stdout()),
        "file" => {
            let path = absolute_path(log["file"].to_str(), root);
            let logger = Logger::new(format_)
                .file(path)
                .await
                .unwrap_or_else(|err| exit!("Init logger failed:\n{:?}", err));

            Setting::Value(logger)
        }
        _ => exit!("Wrong log mode `{}`, optional value: `stdout` `file`", mode),
    }
}

fn parse_error(block: &Block, root: &Option<PathBuf>) -> ErrorPage {
    check_value!(block, "error");
    let error = block["error"].to_block();
    let mut pages = HashMap::new();
    for d in error.directives() {
        let status = util::to_status_code(d.name()).unwrap_exit(d.line());
        let val = parse_error_value(&block, d.name());
        match val {
            Setting::Value(s) => {
                let p = PathBuf::from(&s);
                if p.is_absolute() {
                    pages.insert(status, Setting::Value(p));
                } else {
                    let root = root.clone().unwrap_or_else(|| exit!("Missing root option"));
                    pages.insert(status, Setting::Value(absolute_path(s, root)));
                }
            }
            Setting::Off => {
                pages.insert(status, Setting::Off);
            }
            Setting::None => {}
        }
    }

    Setting::Value(pages)
}

fn parse_error_value(block: &Block, status: &str) -> Setting<String> {
    check_value!(block, status);
    Setting::Value(block[status].to_str().to_string())
}
