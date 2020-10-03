use crate::exit;
use globset::{Glob, GlobMatcher};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Method, Uri};
use regex::Regex;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

// Force conversion of string to specified type

pub fn to_glob(s: &str) -> GlobMatcher {
    Glob::new(s)
        .unwrap_or_else(|err| exit!("Cannot parse `{}` to glob matcher\n{}", s, err))
        .compile_matcher()
}

pub fn to_header_name<S: AsRef<str>>(s: S) -> HeaderName {
    HeaderName::from_str(s.as_ref())
        .unwrap_or_else(|err| exit!("Cannot parse `{}` to http header name\n{}", s.as_ref(), err))
}

pub fn to_header_value<S: AsRef<str>>(s: S) -> HeaderValue {
    HeaderValue::from_str(s.as_ref()).unwrap_or_else(|err| {
        exit!(
            "Cannot parse `{}` to http header value\n{}",
            s.as_ref(),
            err
        )
    })
}

pub fn to_method<S: AsRef<str>>(s: S) -> Method {
    Method::from_str(s.as_ref())
        .unwrap_or_else(|err| exit!("Cannot parse `{}` to http method\n{}", s.as_ref(), err))
}

pub fn to_regex<S: AsRef<str>>(s: S) -> Regex {
    Regex::new(s.as_ref()).unwrap_or_else(|err| {
        exit!(
            "Cannot parse `{}` to regular expression\n{}",
            s.as_ref(),
            err
        )
    })
}

pub fn to_socket_addr<S: AsRef<str>>(s: S) -> SocketAddr {
    try_to_socket_addr(s.as_ref())
        .unwrap_or_else(|_| exit!("Cannot parse `{}` to SocketAddr", s.as_ref()))
}

pub fn to_ip_addr(s: &str) -> IpAddr {
    s.parse::<IpAddr>()
        .unwrap_or_else(|_| exit!("Cannot parse `{}` to IP addr", s))
}

pub fn to_strftime(s: &str) {
    time::now()
        .strftime(s)
        .unwrap_or_else(|err| exit!("Cannot parse `{}` to time format\n{}", s, err));
}

pub fn to_url<S: AsRef<str>>(s: S) -> Uri {
    s.as_ref()
        .parse::<Uri>()
        .unwrap_or_else(|err| exit!("Cannot parse `{}` to http url\n{}", s.as_ref(), err))
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
