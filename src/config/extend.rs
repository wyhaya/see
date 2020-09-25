use crate::exit;
use globset::Glob;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Method, Uri};
use regex::Regex;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;

// Convert path to absolute path
pub trait AbsolutePath {
    fn absolute_path<P: AsRef<Path>>(&self, root: P) -> PathBuf;
}

impl AbsolutePath for String {
    fn absolute_path<P: AsRef<Path>>(&self, root: P) -> PathBuf {
        let path = PathBuf::from(self);
        if path.is_absolute() {
            path
        } else {
            root.as_ref().join(self)
        }
    }
}

// Force conversion of string to specified type
pub trait Force {
    fn to_glob(&self) -> Glob;
    fn to_header_name(&self) -> HeaderName;
    fn to_header_value(&self) -> HeaderValue;
    fn to_method(&self) -> Method;
    fn to_regex(&self) -> Regex;
    fn to_socket_addr(&self) -> SocketAddr;
    fn to_ip_addr(&self) -> IpAddr;
    fn to_strftime(&self);
    fn to_url(&self) -> Uri;
}

impl Force for &str {
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

    fn to_url(&self) -> Uri {
        self.parse::<Uri>()
            .unwrap_or_else(|err| exit!("Cannot parse `{}` to http url\n{}", self, err))
    }
}

fn try_to_socket_addr(text: &str) -> Result<SocketAddr, ()> {
    // 0.0.0.0:80
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Ok(addr);
    }
    // 0.0.0.0
    if let Ok(ip) = text.parse::<Ipv4Addr>() {
        if let Ok(addr) = format!("{}:80", ip).parse::<SocketAddr>() {
            return Ok(addr);
        }
    }
    // 80
    if let Ok(port) = text.parse::<u16>() {
        if let Ok(addr) = format!("0.0.0.0:{}", port).parse::<SocketAddr>() {
            return Ok(addr);
        }
    }

    Err(())
}

#[test]
fn test_try_to_socket_addr() {
    assert_eq!(
        try_to_socket_addr("80").unwrap(),
        "0.0.0.0:80".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(
        try_to_socket_addr("0.0.0.0").unwrap(),
        "0.0.0.0:80".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(
        try_to_socket_addr("0.0.0.0:80").unwrap(),
        "0.0.0.0:80".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(try_to_socket_addr("err"), Err(()));
}
