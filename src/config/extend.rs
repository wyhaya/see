use crate::util::*;
use crate::*;
use globset::Glob;
use hyper::{Method, Uri};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

pub trait GetExtension {
    fn get_extension(&self) -> Option<&str>;
}

impl GetExtension for PathBuf {
    fn get_extension(&self) -> Option<&str> {
        if let Some(p) = self.extension() {
            if let Some(p) = p.to_str() {
                return Some(p);
            }
        }
        None
    }
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

pub trait ForceTo {
    fn to_duration(&self) -> Duration;
    fn to_glob(&self) -> Glob;
    fn to_header_name(&self) -> HeaderName;
    fn to_header_value(&self) -> HeaderValue;
    fn to_method(&self) -> Method;
    fn to_regex(&self) -> Regex;
    fn to_socket_addr(&self) -> SocketAddr;
    fn to_ip_addr(&self) -> IpAddr;
    fn to_strftime(&self);
    fn to_uri(&self) -> Uri;
}

impl ForceTo for &str {
    fn to_duration(&self) -> Duration {
        try_parse_duration(self).unwrap_or_else(|err| {
            exit!("Cannot parse `{}` to duration: {}", self, err.description())
        })
    }

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
        try_to_socket_addr(self).unwrap_or_else(|_| exit!("Cannot parse `{}` to SocketAddr", self))
    }

    fn to_ip_addr(&self) -> IpAddr {
        self.parse::<IpAddr>()
            .unwrap_or_else(|_| exit!("Cannot parse `{}` to IP addr", self))
    }

    fn to_strftime(&self) {
        time::now()
            .strftime(self)
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to time format\n{}", self, err));
    }

    fn to_uri(&self) -> Uri {
        self.parse::<Uri>()
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to http uri\n{}", self, err))
    }
}
