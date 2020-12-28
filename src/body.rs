use crate::compress::Encoding;
use crate::default;
use async_compression::tokio::bufread::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use hyper::body::Body;
use std::io::Cursor;
use tokio::fs::File;
use tokio::io::BufReader;
use tokio_util::codec::{BytesCodec, FramedRead};

pub struct BodyStream {
    compress: Option<Encoding>,
}

impl BodyStream {
    pub fn new(compress: Option<Encoding>) -> Self {
        Self { compress }
    }

    pub fn file(self, file: File) -> Body {
        let file = BufReader::with_capacity(default::BUF_SIZE, file);
        match self.compress {
            Some(encoding) => match encoding {
                Encoding::Gzip(level) => Body::wrap_stream(FramedRead::new(
                    GzipEncoder::with_quality(file, level),
                    BytesCodec::new(),
                )),
                Encoding::Br(level) => Body::wrap_stream(FramedRead::new(
                    BrotliEncoder::with_quality(file, level),
                    BytesCodec::new(),
                )),
                Encoding::Deflate(level) => Body::wrap_stream(FramedRead::new(
                    DeflateEncoder::with_quality(file, level),
                    BytesCodec::new(),
                )),
            },
            None => Body::wrap_stream(FramedRead::new(file, BytesCodec::new())),
        }
    }

    pub fn text(self, text: String) -> Body {
        match self.compress {
            Some(encoding) => {
                let data = Cursor::new(text.into_bytes());
                match encoding {
                    Encoding::Gzip(level) => Body::wrap_stream(FramedRead::new(
                        GzipEncoder::with_quality(data, level),
                        BytesCodec::new(),
                    )),
                    Encoding::Br(level) => Body::wrap_stream(FramedRead::new(
                        BrotliEncoder::with_quality(data, level),
                        BytesCodec::new(),
                    )),
                    Encoding::Deflate(level) => Body::wrap_stream(FramedRead::new(
                        DeflateEncoder::with_quality(data, level),
                        BytesCodec::new(),
                    )),
                }
            }
            None => Body::from(text),
        }
    }
}
