use crate::{default, exit};
use async_compression::stream::{BrotliEncoder, DeflateEncoder, GzipEncoder};
pub use async_compression::Level as CompressLevel;
use futures::stream::{self, StreamExt};
use hyper::body::{Body, Bytes};
use hyper::header::HeaderValue;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

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
            "fast" => CompressLevel::Fastest,
            "default" => CompressLevel::Default,
            "best" => CompressLevel::Best,
            _ => exit!(
                "Wrong compression level `{}`, optional value: `fast` `default` `best`",
                s
            ),
        }
    }
}

pub struct BodyStream {
    compress: Option<CompressMode>,
}

impl BodyStream {
    pub fn new(compress: Option<CompressMode>) -> Self {
        Self { compress }
    }

    pub fn file(self, file: File) -> Body {
        let file = FramedRead::with_capacity(file, BytesCodec::new(), default::BUF_SIZE)
            .map(|rst| rst.map(|bytes| bytes.freeze()));

        match self.compress {
            Some(mode) => match mode {
                CompressMode::Gzip(level) => {
                    Body::wrap_stream(GzipEncoder::with_quality(file, level))
                }
                CompressMode::Br(level) => {
                    Body::wrap_stream(BrotliEncoder::with_quality(file, level))
                }
                CompressMode::Deflate(level) => {
                    Body::wrap_stream(DeflateEncoder::with_quality(file, level))
                }
            },
            None => Body::wrap_stream(file),
        }
    }

    pub fn content(self, text: String) -> Body {
        match self.compress {
            Some(mode) => {
                let text = stream::once(async move { Ok(Bytes::from(text)) });
                match mode {
                    CompressMode::Gzip(level) => {
                        Body::wrap_stream(GzipEncoder::with_quality(text, level))
                    }
                    CompressMode::Br(level) => {
                        Body::wrap_stream(BrotliEncoder::with_quality(text, level))
                    }
                    CompressMode::Deflate(level) => {
                        Body::wrap_stream(DeflateEncoder::with_quality(text, level))
                    }
                }
            }
            None => Body::from(text),
        }
    }
}
