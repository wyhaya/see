use crate::*;
pub use async_compression::Level as CompressLevel;
use hyper::header::HeaderValue;

#[derive(Copy, Clone, Debug)]
pub enum CompressMode {
    Gzip(CompressLevel),
    Deflate(CompressLevel),
    Br(CompressLevel),
}

impl CompressMode {
    // Response header content-encoding
    pub fn to_header_value(self) -> HeaderValue {
        let encoding = match self {
            CompressMode::Gzip(_) => "gzip",
            CompressMode::Deflate(_) => "deflate",
            CompressMode::Br(_) => "br",
        };

        HeaderValue::from_static(encoding)
    }
}

pub trait Level {
    fn new(s: String) -> Self;
}

impl Level for CompressLevel {
    fn new(s: String) -> Self {
        match s.as_str() {
            "fastest" => CompressLevel::Fastest,
            "default" => CompressLevel::Default,
            "best" => CompressLevel::Best,
            _ => exit!(
                "Wrong compression level `{}`, optional value: `fastest` `default` `best`",
                s
            ),
        }
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {}
}
