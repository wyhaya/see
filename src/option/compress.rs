use crate::compress::{CompressMode, Encoding};
use hyper::header::HeaderValue;

#[derive(Debug, Clone)]
pub struct Compress {
    pub modes: Vec<CompressMode>,
    pub extensions: Vec<String>,
}

impl Compress {
    pub fn get_compress_mode(&self, header: &HeaderValue, ext: &str) -> Option<Encoding> {
        if self.extensions.iter().any(|item| *item == ext) {
            // accept-encoding: gzip, deflate, br
            let header: Vec<&str> = match header.to_str() {
                Ok(encoding) => encoding.split(", ").collect(),
                Err(_) => return None,
            };

            for mode in &self.modes {
                if let Some(compress) = mode.encoding(&header) {
                    return Some(compress);
                }
            }
        }

        None
    }
}
