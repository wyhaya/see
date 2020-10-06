use super::{ErrorPage, Headers, Location, ServerConfig, SiteConfig};
use crate::util::AbsolutePath;
use crate::{check_none, check_off, check_value, compress, config, exit, matcher, option, yaml};
use compress::{CompressLevel, Encoding, Level};
use config::tls::{create_sni_server_config, TLSContent};
use config::{default, transform, Setting, Var};
use matcher::{HostMatcher, IpMatcher, LocationMatcher};
use option::{Auth, Compress, Directory, Index, Logger, Method, Proxy, Rewrite};
use std::collections::{BTreeSet, HashMap};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use yaml::YamlExtend;
use yaml_rust::Yaml;

pub struct Parser {
    yaml: Yaml,
}

impl Parser {
    pub fn new(yaml: Yaml) -> Self {
        Self { yaml }
    }

    pub async fn server<P: AsRef<Path>>(&self, config_dir: P) -> Vec<ServerConfig> {
        let mut configs: Vec<ServerConfig> = vec![];
        let mut tls_configs: Vec<(SocketAddr, Vec<TLSContent>)> = vec![];

        for server in self.yaml.as_vec().unwrap() {
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
                    "try",
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

            let https = parser.https(config_dir.as_ref(), host.get_raw());

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

            let root = parser.root(config_dir.as_ref());

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
                try_: parser.try_(),
                method: parser.method(true),
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
            let i = configs
                .iter()
                .position(|item| item.listen == listen)
                .unwrap();
            let t = create_sni_server_config(group);
            configs[i].tls = Some(t);
        }

        configs
    }

    async fn location<P: AsRef<Path>>(
        &self,
        config_dir: P,
        parent_root: Option<PathBuf>,
    ) -> Vec<Location> {
        if self.yaml["location"].is_badvalue() {
            return vec![];
        }

        let hash = self.yaml["location"].to_hash();
        let mut vec = vec![];

        for (key, server) in hash.iter() {
            let route = key.to_string();

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
                    "try",
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
                method: parser.method(false),
                auth: parser.auth(),
                try_: parser.try_(),
                error: parser.error(&root),
                proxy: parser.proxy(),
                log: parser.log(&config_dir).await,
                ip: parser.ip(),
            });
        }
        vec
    }

    fn listen(&self) -> Vec<SocketAddr> {
        self.yaml["listen"]
            .to_multiple_string()
            .iter()
            .map(transform::to_socket_addr)
            .collect::<BTreeSet<SocketAddr>>()
            .into_iter()
            .collect()
    }

    fn https(&self, config_dir: &Path, hostname: Vec<&String>) -> Option<TLSContent> {
        let https = &self.yaml["https"];
        if https.is_badvalue() {
            return None;
        }

        self.yaml.check("https", &["cert", "key"], &["cert", "key"]);

        let cert = https["cert"].to_string().absolute_path(config_dir);
        let key = https["key"].to_string().absolute_path(config_dir);

        if hostname.is_empty() {
            exit!("Miss 'host'");
        }

        // todo
        let sni = hostname[0].clone();

        Some(TLSContent { cert, key, sni })
    }

    fn host(&self) -> HostMatcher {
        let vec = self.yaml["host"].to_multiple_string();
        HostMatcher::new(vec)
    }

    fn root(&self, config_dir: &Path) -> Option<PathBuf> {
        if self.yaml["root"].is_badvalue() {
            return None;
        }

        let path = self.yaml["root"].to_string().absolute_path(config_dir);
        Some(path)
    }

    fn echo(&self) -> Setting<Var<String>> {
        check_value!(self.yaml["echo"]);

        let var = Var::from(self.yaml["echo"].to_string());
        Setting::Value(var)
    }

    fn file<P: AsRef<Path>>(&self, root: P) -> Setting<PathBuf> {
        check_value!(self.yaml["file"]);

        let path = self.yaml["file"].to_string().absolute_path(root);
        Setting::Value(path)
    }

    fn index(&self, set_default: bool) -> Setting<Index> {
        let index = &self.yaml["index"];
        check_off!(index);

        if set_default {
            check_none!(
                index,
                Index::new(default::INDEX.iter().map(|i| (*i).to_string()).collect())
            );
        } else {
            check_none!(index);
        }

        let vec = self.yaml["index"].to_multiple_string();
        Setting::Value(Index::new(vec))
    }

    fn directory(&self) -> Setting<Directory> {
        let directory = &self.yaml["directory"];
        check_value!(directory);

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
                    let format = directory["time"].to_string();
                    // check
                    let _ = transform::to_strftime(&format);
                    Some(format)
                }
            }
        };

        let size = if directory["size"].is_badvalue() {
            false
        } else {
            directory["size"].to_bool()
        };

        Setting::Value(Directory { time, size })
    }

    fn headers(&self) -> Setting<Headers> {
        check_value!(self.yaml["header"]);

        let hash = self.yaml["header"].to_hash();
        let mut map = HashMap::new();

        for (key, value) in hash {
            let key = key.to_string();
            let header_name = transform::to_header_name(&key);

            let value = Var::from(value.to_string());
            let header_value = value.map_none(transform::to_header_value);

            map.insert(header_name, header_value);
        }

        Setting::Value(map)
    }

    fn rewrite(&self) -> Setting<Rewrite> {
        check_value!(self.yaml["rewrite"]);
        let value = self.yaml["rewrite"].to_string();

        Setting::Value(Rewrite::from(value))
    }

    fn compress(&self) -> Setting<Compress> {
        let compress = &self.yaml["compress"];
        check_value!(compress);

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
            CompressLevel::new(compress["level"].to_string())
        };

        let modes = if compress["mode"].is_badvalue() {
            vec![Encoding::Auto(level)]
        } else {
            let mode = compress["mode"].to_multiple_string();
            mode.iter().map(|mode| Encoding::new(mode, level)).collect()
        };

        let extensions = if compress["extension"].is_badvalue() {
            default::COMPRESS_EXTENSIONS
                .iter()
                .map(|e| (*e).to_string())
                .collect()
        } else {
            compress["extension"].to_multiple_string()
        };

        Setting::Value(Compress { modes, extensions })
    }

    fn try_(&self) -> Setting<Vec<Var<String>>> {
        check_value!(self.yaml["try"]);

        let vec = self.yaml["try"]
            .to_multiple_string()
            .iter()
            .map(|s| Var::from(s))
            .collect::<Vec<Var<String>>>();

        Setting::Value(vec)
    }

    fn method(&self, set_default: bool) -> Setting<Method> {
        let method = &self.yaml["method"];
        check_off!(method);

        if set_default {
            check_none!(method, Method::new(default::ALLOW_METHODS.to_vec()));
        } else {
            check_none!(method);
        }

        let methods = self.yaml["method"]
            .to_multiple_string()
            .iter()
            .map(|s| transform::to_method(s))
            .collect();

        Setting::Value(Method::new(methods))
    }

    fn auth(&self) -> Setting<Auth> {
        let auth = &self.yaml["auth"];
        check_value!(auth);

        self.yaml
            .check("auth", &["user", "password"], &["user", "password"]);

        Setting::Value(Auth::basic(
            auth["user"].to_string(),
            auth["password"].to_string(),
        ))
    }

    fn error(&self, root: &Option<PathBuf>) -> ErrorPage {
        let error = &self.yaml["error"];
        check_value!(error);

        let mut pages = HashMap::new();

        for (key, val) in self.yaml["error"].to_hash() {
            let key = key.to_string();
            let status = transform::to_status_code(&key);
            let val = Self::get_error_value(&val);
            match val {
                Setting::Value(s) => {
                    let p = PathBuf::from(&s);
                    if p.is_absolute() {
                        pages.insert(status, Setting::Value(p));
                    } else {
                        let root = root.clone().unwrap_or_else(|| exit!("Miss root option"));
                        pages.insert(status, Setting::Value(s.absolute_path(&root)));
                    }
                }
                Setting::Off => {
                    pages.insert(status, Setting::Off);
                }
                Setting::None => break,
            }
        }

        Setting::Value(pages)
    }

    fn get_error_value(yaml: &Yaml) -> Setting<String> {
        check_value!(yaml);
        Setting::Value(yaml.to_string())
    }

    fn proxy(&self) -> Setting<Proxy> {
        let proxy = &self.yaml["proxy"];
        check_value!(proxy);

        self.yaml
            .check("proxy", &["url", "method", "header"], &["url"]);

        let url_str = proxy["url"].to_string();
        let url = Var::from(url_str).map_none(transform::to_url);

        let method = if proxy["method"].is_badvalue() {
            None
        } else {
            let method = transform::to_method(proxy["method"].to_string());
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
        check_value!(log);

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
            log["format"].to_string()
        };

        let mode = log["mode"].to_string();

        match mode.as_ref() {
            "stdout" => Setting::Value(Logger::new(format_).stdout()),
            "file" => {
                let path = log["file"].to_string();
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
        check_value!(ip);

        self.yaml.check("ip", &["allow", "deny"], &[]);

        Setting::Value(IpMatcher::new(
            ip["allow"].to_multiple_string(),
            ip["deny"].to_multiple_string(),
        ))
    }

    fn break_(&self) -> bool {
        if self.yaml["break"].is_badvalue() {
            return false;
        }

        self.yaml["break"].to_bool()
    }
}
