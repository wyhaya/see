use crate::compress::{CompressMode, Encoding};
use hyper::header::HeaderValue;

#[derive(Debug, Clone)]
pub struct Compress {
    pub modes: Vec<Encoding>,
    pub extensions: Vec<String>,
}

impl Compress {
    pub fn get_compress_mode(&self, header: &HeaderValue, ext: &str) -> Option<CompressMode> {
        if self.extensions.iter().any(|item| *item == ext) {
            // accept-encoding: gzip, deflate, br
            let header: Vec<&str> = match header.to_str() {
                Ok(encoding) => encoding.split(", ").collect(),
                Err(_) => return None,
            };

            for encoding in &self.modes {
                if let Some(compress) = encoding.compress_mode(&header) {
                    return Some(compress);
                }
            }
        }

        None
    }
}
