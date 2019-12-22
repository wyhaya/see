use crate::*;

lazy_static! {
    static ref REGEX_BASE: Regex =
        Regex::new(r"\$\{request_(scheme|host|port|path|query|uri|method)\}").unwrap();
    static ref REGEX_QUERY: Regex = Regex::new(r"\$\{request_query_([\w|-]+)\}").unwrap();
    static ref REGEX_HEADER: Regex = Regex::new(r"\$\{request_header_([\w|-]+)\}").unwrap();
}

#[derive(Debug, Clone)]
pub enum Var<T> {
    None(T),
    Replace(String, Replace),
}

#[derive(Debug, Clone)]
pub struct Replace {
    pub base: bool,
    pub query: bool,
    pub header: bool,
}

impl<T> Var<T> {
    pub fn map_none<U, F: FnOnce(T) -> U>(self, f: F) -> Var<U> {
        match self {
            Var::None(x) => Var::None(f(x)),
            Var::Replace(s, r) => Var::Replace(s, r),
        }
    }

    pub fn unwrap_or_else<F: FnOnce(String, Replace) -> T>(self, f: F) -> T {
        match self {
            Var::None(x) => x,
            Var::Replace(s, r) => f(s, r),
        }
    }
}

trait GetParam {
    fn param(&self, name: &str) -> Option<&str>;
}

impl GetParam for Uri {
    fn param(&self, name: &str) -> Option<&str> {
        let split = self.query()?.split('&');
        for item in split {
            let mut item = item.split('=');
            if let Some(key) = item.next() {
                if key == name {
                    return Some(item.next().unwrap_or(""));
                }
            }
        }
        None
    }
}

pub trait ToVar {
    fn to_var(self) -> Var<String>;
}

impl ToVar for String {
    fn to_var(self) -> Var<String> {
        match get_replace(&self) {
            Some(rep) => Var::Replace(self, rep),
            None => Var::None(self),
        }
    }
}

pub trait ReplaceVar {
    fn replace_var(self, replace: &Replace, req: &Request<Body>) -> String;
}

impl ReplaceVar for &str {
    fn replace_var(self, replace: &Replace, req: &Request<Body>) -> String {
        let mut source = self.to_string();

        // request_${}
        if replace.base {
            let scheme = req.uri().scheme_str().unwrap_or("");
            let host = req.uri().host().unwrap_or("");
            let port = req
                .uri()
                .port()
                .map(|d| d.to_string())
                .unwrap_or(String::with_capacity(0));
            let path = req.uri().path();
            let query = req
                .uri()
                .query()
                .map(|q| format!("?{}", q))
                .unwrap_or(String::with_capacity(0));
            let method = req.method().as_str();

            source = source
                .replace("${request_scheme}", scheme)
                .replace("${request_host}", host)
                .replace("${request_port}", &port)
                .replace("${request_path}", path)
                .replace("${request_query}", &query)
                .replace("${request_uri}", &format!("{}{}", path, query))
                .replace("${request_method}", method);
        }

        // request_query_${}
        if replace.query {
            let reg: &Regex = &REGEX_QUERY;
            for cap in reg.captures_iter(&source.clone()) {
                if let Some(m) = cap.get(1) {
                    let from = format!("${{request_query_{}}}", m.as_str());
                    let to = req.uri().param(m.as_str()).unwrap_or("");
                    source = source.replace(from.as_str(), to);
                }
            }
        }

        // request_header_${}
        if replace.header {
            let reg: &Regex = &REGEX_HEADER;
            for cap in reg.captures_iter(&source.clone()) {
                if let Some(m) = cap.get(1) {
                    let from = format!("${{request_header_{}}}", m.as_str());
                    let to = match req.headers().get(m.as_str()) {
                        Some(h) => h.to_str().unwrap(),
                        None => "",
                    };
                    source = source.replace(from.as_str(), to);
                }
            }
        }

        source
    }
}

fn get_replace(s: &str) -> Option<Replace> {
    let base: &Regex = &REGEX_BASE;
    let query: &Regex = &REGEX_QUERY;
    let header: &Regex = &REGEX_HEADER;
    let mut r = Replace {
        base: false,
        query: false,
        header: false,
    };
    if base.is_match(s) {
        r.base = true;
    }
    if query.is_match(s) {
        r.query = true;
    }
    if header.is_match(s) {
        r.header = true;
    }
    if !r.base && !r.query && !r.header {
        return None;
    }
    Some(r)
}
