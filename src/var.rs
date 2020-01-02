use crate::*;

#[derive(Debug, Clone)]
pub enum Var<T> {
    None(T),
    Replace(String, Replace),
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

const REQUEST_PATH: &str = "${request_path}";
const REQUEST_QUERY: &str = "${request_query}";
const REQUEST_URI: &str = "${request_uri}";
const REQUEST_METHOD: &str = "${request_method}";

const REQUEST_QUERY_WORD: &str = "request_query_";
const REQUEST_HEADER_WORD: &str = "request_header_";

lazy_static! {
    static ref REGEX_QUERY: Regex = Regex::new(r"\$\{request_query_([\w|-]+)\}").unwrap();
    static ref REGEX_HEADER: Regex = Regex::new(r"\$\{request_header_([\w|-]+)\}").unwrap();
}

#[derive(Debug, Clone, Default)]
pub struct Replace {
    path: bool,
    query: bool,
    uri: bool,
    method: bool,
    query_key: bool,
    header_key: bool,
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

fn get_replace(raw: &str) -> Option<Replace> {
    let query: &Regex = &REGEX_QUERY;
    let header: &Regex = &REGEX_HEADER;
    let mut replace = Replace::default();

    if raw.contains(REQUEST_PATH) {
        replace.path = true;
    }
    if raw.contains(REQUEST_QUERY) {
        replace.query = true;
    }
    if raw.contains(REQUEST_URI) {
        replace.uri = true;
    }
    if raw.contains(REQUEST_METHOD) {
        replace.method = true;
    }
    if query.is_match(raw) {
        replace.query_key = true;
    }
    if header.is_match(raw) {
        replace.header_key = true;
    }

    if replace.path
        || replace.query
        || replace.uri
        || replace.method
        || replace.query_key
        || replace.header_key
    {
        Some(replace)
    } else {
        None
    }
}

pub trait ReplaceVar {
    fn replace_var(self, replace: &Replace, req: &Request<Body>) -> String;
}

impl ReplaceVar for &str {
    fn replace_var(self, replace: &Replace, req: &Request<Body>) -> String {
        let mut source = self.to_string();

        if replace.path {
            source = source.replace(REQUEST_PATH, req.uri().path());
        }

        if replace.query {
            match req.uri().query() {
                Some(query) => {
                    source = source.replace(REQUEST_QUERY, &format!("?{}", query));
                }
                None => {
                    source = source.replace(REQUEST_QUERY, "");
                }
            }
        }

        if replace.uri {
            let query = req
                .uri()
                .query()
                .map(|q| format!("?{}", q))
                .unwrap_or(String::with_capacity(0));
            let uri = format!("{}{}", req.uri().path(), query);
            source = source.replace(REQUEST_URI, &uri);
        }

        if replace.method {
            source = source.replace(REQUEST_METHOD, req.method().as_str());
        }

        if replace.query_key {
            let reg: &Regex = &REGEX_QUERY;
            for cap in reg.captures_iter(&source.clone()) {
                if let Some(m) = cap.get(1) {
                    let from = format!("${{{}{}}}", REQUEST_QUERY_WORD, m.as_str());
                    let to = req.uri().param(m.as_str()).unwrap_or_default();
                    source = source.replace(from.as_str(), to);
                }
            }
        }

        if replace.header_key {
            let reg: &Regex = &REGEX_HEADER;
            for cap in reg.captures_iter(&source.clone()) {
                if let Some(m) = cap.get(1) {
                    let from = format!("${{{}{}}}", REQUEST_HEADER_WORD, m.as_str());
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
