use hyper::{Body, Request, Uri};
use lazy_static::lazy_static;
use regex::Regex;

const REQUEST_PATH: &str = "$`path`";
const REQUEST_QUERY: &str = "$`query`";
const REQUEST_METHOD: &str = "$`method`";
const REQUEST_VERSION: &str = "$`version`";

const REQUEST_QUERY_KEY: &str = "query_";
const REQUEST_HEADER_KEY: &str = "header_";

lazy_static! {
    static ref REGEX_QUERY: Regex = Regex::new(r"\$`query_([\w|-]+)`").unwrap();
    static ref REGEX_HEADER: Regex = Regex::new(r"\$`header_([\w|-]+)`").unwrap();
}

#[derive(Debug, Clone)]
pub enum Var<T> {
    None(T),
    Some(String, Replace),
}

impl<T: ToString> From<T> for Var<String> {
    fn from(text: T) -> Self {
        let text = text.to_string();
        match Replace::new(&text) {
            Some(rep) => Var::Some(text, rep),
            None => Var::None(text),
        }
    }
}

impl<T> Var<T> {
    pub fn map<F: FnOnce(String, Replace) -> T>(self, f: F) -> T {
        match self {
            Var::None(x) => x,
            Var::Some(s, r) => f(s, r),
        }
    }

    pub fn map_none<U, F: FnOnce(T) -> U>(self, f: F) -> Var<U> {
        match self {
            Var::None(x) => Var::None(f(x)),
            Var::Some(s, r) => Var::Some(s, r),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Replace {
    path: bool,
    query: bool,
    method: bool,
    version: bool,
    query_key: bool,
    header_key: bool,
}

impl Replace {
    pub fn new(text: &str) -> Option<Self> {
        let query: &Regex = &REGEX_QUERY;
        let header: &Regex = &REGEX_HEADER;
        let mut replace = Replace::default();

        if text.contains(REQUEST_PATH) {
            replace.path = true;
        }
        if text.contains(REQUEST_QUERY) {
            replace.query = true;
        }
        if text.contains(REQUEST_METHOD) {
            replace.method = true;
        }
        if text.contains(REQUEST_VERSION) {
            replace.version = true;
        }
        if query.is_match(text) {
            replace.query_key = true;
        }
        if header.is_match(text) {
            replace.header_key = true;
        }

        if replace.path
            || replace.query
            || replace.method
            || replace.version
            || replace.query_key
            || replace.header_key
        {
            Some(replace)
        } else {
            None
        }
    }

    pub fn replace(&self, mut source: String, req: &Request<Body>) -> String {
        if self.path {
            source = source.replace(REQUEST_PATH, req.uri().path());
        }

        if self.query {
            match req.uri().query() {
                Some(query) => {
                    source = source.replace(REQUEST_QUERY, &format!("?{}", query));
                }
                None => {
                    source = source.replace(REQUEST_QUERY, "");
                }
            }
        }

        if self.method {
            source = source.replace(REQUEST_METHOD, req.method().as_str());
        }

        if self.version {
            source = source.replace(REQUEST_VERSION, &format!("{:?}", req.version()));
        }

        if self.query_key {
            let reg: &Regex = &REGEX_QUERY;
            for cap in reg.captures_iter(&source.clone()) {
                if let Some(m) = cap.get(1) {
                    let from = format!("$`{}{}`", REQUEST_QUERY_KEY, m.as_str());
                    let to = req.uri().get_query(m.as_str()).unwrap_or_default();
                    source = source.replace(from.as_str(), to);
                }
            }
        }

        if self.header_key {
            let reg: &Regex = &REGEX_HEADER;
            for cap in reg.captures_iter(&source.clone()) {
                if let Some(m) = cap.get(1) {
                    let from = format!("$`{}{}`", REQUEST_HEADER_KEY, m.as_str());
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

trait GetQuery {
    fn get_query(&self, name: &str) -> Option<&str>;
}

impl GetQuery for Uri {
    fn get_query(&self, name: &str) -> Option<&str> {
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

#[test]
fn test_var() {
    let _ = Var::from("$`path`");
}
