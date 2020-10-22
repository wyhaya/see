use async_compression::Level;
use hyper::header::HeaderValue;

#[derive(Copy, Clone, Debug)]
pub enum Encoding {
    Gzip(Level),
    Deflate(Level),
    Br(Level),
}

impl Encoding {
    // Response header content-encoding
    pub fn to_header_value(self) -> HeaderValue {
        let encoding = match self {
            Encoding::Gzip(_) => "gzip",
            Encoding::Deflate(_) => "deflate",
            Encoding::Br(_) => "br",
        };

        HeaderValue::from_static(encoding)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CompressMode {
    Auto(Level),
    Gzip(Level),
    Deflate(Level),
    Br(Level),
    None,
}

impl CompressMode {
    pub fn new(mode: &str, level: Level) -> Result<Self, String> {
        match mode {
            "auto" => Ok(CompressMode::Auto(level)),
            "gzip" => Ok(CompressMode::Gzip(level)),
            "deflate" => Ok(CompressMode::Deflate(level)),
            "br" => Ok(CompressMode::Br(level)),
            _ => Err(format!(
                "Wrong compression mode `{}`, optional value: `auto` `gzip` `deflate` `br`",
                mode
            )),
        }
    }

    pub fn encoding(self, modes: &[&str]) -> Option<Encoding> {
        match self {
            CompressMode::Auto(level) => {
                for mode in modes {
                    match *mode {
                        "gzip" => return Some(Encoding::Gzip(level)),
                        "deflate" => return Some(Encoding::Deflate(level)),
                        "br" => return Some(Encoding::Br(level)),
                        _ => {}
                    };
                }
            }
            CompressMode::Gzip(level) => {
                if modes.contains(&"gzip") {
                    return Some(Encoding::Gzip(level));
                }
            }
            CompressMode::Deflate(level) => {
                if modes.contains(&"deflate") {
                    return Some(Encoding::Deflate(level));
                }
            }
            CompressMode::Br(level) => {
                if modes.contains(&"br") {
                    return Some(Encoding::Br(level));
                }
            }
            _ => {}
        }

        None
    }
}
