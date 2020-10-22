use crate::compress::Encoding;
use crate::default;
use async_compression::stream::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use futures_util::stream::{self, StreamExt};
use hyper::body::{Body, Bytes};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

pub struct BodyStream {
    compress: Option<Encoding>,
}

impl BodyStream {
    pub fn new(compress: Option<Encoding>) -> Self {
        Self { compress }
    }

    pub fn file(self, file: File) -> Body {
        let file = FramedRead::with_capacity(file, BytesCodec::new(), default::BUF_SIZE)
            .map(|rst| rst.map(|bytes| bytes.freeze()));

        match self.compress {
            Some(mode) => match mode {
                Encoding::Gzip(level) => Body::wrap_stream(GzipEncoder::with_quality(file, level)),
                Encoding::Br(level) => Body::wrap_stream(BrotliEncoder::with_quality(file, level)),
                Encoding::Deflate(level) => {
                    Body::wrap_stream(DeflateEncoder::with_quality(file, level))
                }
            },
            None => Body::wrap_stream(file),
        }
    }

    pub fn text(self, text: String) -> Body {
        match self.compress {
            Some(mode) => {
                let text = stream::once(async move { Ok(Bytes::from(text)) });
                match mode {
                    Encoding::Gzip(level) => {
                        Body::wrap_stream(GzipEncoder::with_quality(text, level))
                    }
                    Encoding::Br(level) => {
                        Body::wrap_stream(BrotliEncoder::with_quality(text, level))
                    }
                    Encoding::Deflate(level) => {
                        Body::wrap_stream(DeflateEncoder::with_quality(text, level))
                    }
                }
            }
            None => Body::from(text),
        }
    }
}
