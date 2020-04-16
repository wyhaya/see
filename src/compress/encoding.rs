use crate::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Encoding {
    Auto(u32),
    Gzip(u32),
    Deflate(u32),
    Br(u32),
    None,
}

impl Encoding {
    pub fn new(mode: &str, level: u32) -> Self {
        if level > 9 {
            exit!("Compress level should be an integer between 0-9");
        }
        match mode {
            "auto" => Encoding::Auto(level),
            "gzip" => Encoding::Gzip(level),
            "deflate" => Encoding::Deflate(level),
            "br" => Encoding::Br(level),
            _ => exit!(
                "Wrong compression mode `{}`, optional value: `auto` `gzip` `deflate` `br`",
                mode
            ),
        }
    }

    pub fn parse_mode(&self, modes: Vec<&str>) -> Self {
        match self {
            Encoding::Auto(level) => {
                for mode in modes {
                    match mode {
                        "gzip" => return Encoding::Gzip(*level),
                        "deflate" => return Encoding::Deflate(*level),
                        "br" => return Encoding::Br(*level),
                        _ => {}
                    };
                }
            }
            Encoding::Gzip(level) => {
                for mode in modes {
                    if mode == "gzip" {
                        return Encoding::Gzip(*level);
                    }
                }
            }
            Encoding::Deflate(level) => {
                for mode in modes {
                    if mode == "deflate" {
                        return Encoding::Deflate(*level);
                    }
                }
            }
            Encoding::Br(level) => {
                for mode in modes {
                    if mode == "br" {
                        return Encoding::Br(*level);
                    }
                }
            }
            _ => {}
        }
        Encoding::None
    }

    pub fn to_header_value(&self) -> HeaderValue {
        let s = match self {
            Encoding::Gzip(_) => "gzip",
            Encoding::Deflate(_) => "deflate",
            Encoding::Br(_) => "br",
            _ => "",
        };

        HeaderValue::from_static(s)
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {}
}
