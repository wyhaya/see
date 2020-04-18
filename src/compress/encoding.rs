use crate::*;
use compress::{CompressLevel, CompressMode};

#[derive(Debug, Clone, Copy)]
pub enum Encoding {
    Auto(CompressLevel),
    Gzip(CompressLevel),
    Deflate(CompressLevel),
    Br(CompressLevel),
    None,
}

impl Encoding {
    pub fn new(mode: &str, level: CompressLevel) -> Self {
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

    pub fn get_compress_mode(&self, modes: &Vec<&str>) -> Option<CompressMode> {
        match self {
            Encoding::Auto(level) => {
                for mode in modes {
                    match mode {
                        &"gzip" => return Some(CompressMode::Gzip(*level)),
                        &"deflate" => return Some(CompressMode::Deflate(*level)),
                        &"br" => return Some(CompressMode::Br(*level)),
                        _ => {}
                    };
                }
            }
            Encoding::Gzip(level) => {
                if modes.contains(&"gzip") {
                    return Some(CompressMode::Gzip(*level));
                }
            }
            Encoding::Deflate(level) => {
                if modes.contains(&"deflate") {
                    return Some(CompressMode::Deflate(*level));
                }
            }
            Encoding::Br(level) => {
                if modes.contains(&"br") {
                    return Some(CompressMode::Br(*level));
                }
            }
            _ => {}
        }

        None
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {}
}
