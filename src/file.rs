use crate::compress::CompressMode;
use async_compression::stream::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use futures::ready;
use futures::stream::Stream;
use hyper::body::Bytes;
use hyper::Body;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncRead, Result};

pub fn create_file_body(file: File, buf_size: usize, mode: Option<CompressMode>) -> Body {
    match mode {
        Some(c) => {
            let f = FileBytesReader { file, buf_size };
            let encoder = match c {
                CompressMode::Gzip(l) => Encoder::Gzip(GzipEncoder::with_quality(f, l)),
                CompressMode::Br(l) => Encoder::Brotli(BrotliEncoder::with_quality(f, l)),
                CompressMode::Deflate(l) => Encoder::Deflate(DeflateEncoder::with_quality(f, l)),
            };
            Body::wrap_stream(EncodingReader { encoder })
        }
        None => Body::wrap_stream(FileReader { file, buf_size }),
    }
}

// Get file content via stream
struct FileReader {
    file: File,
    buf_size: usize,
}

impl Stream for FileReader {
    type Item = Result<Vec<u8>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0; self.buf_size];
        let poll = Pin::new(&mut self.get_mut().file).poll_read(cx, &mut buf);

        match ready!(poll) {
            Ok(n) => {
                if n == 0 {
                    Poll::Ready(None)
                } else {
                    let data = buf[..n].to_vec();
                    Poll::Ready(Some(Ok(data)))
                }
            }
            Err(err) => Poll::Ready(Some(Err(err))),
        }
    }
}

// Get file content 'Bytes' via stream
struct FileBytesReader {
    file: File,
    buf_size: usize,
}

impl Stream for FileBytesReader {
    type Item = Result<Bytes>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0; self.buf_size];
        let poll = Pin::new(&mut self.get_mut().file).poll_read(cx, &mut buf);

        match ready!(poll) {
            Ok(n) => {
                if n == 0 {
                    Poll::Ready(None)
                } else {
                    let data = buf[..n].to_vec();
                    Poll::Ready(Some(Ok(Bytes::from(data))))
                }
            }
            Err(err) => Poll::Ready(Some(Err(err))),
        }
    }
}

enum Encoder {
    Gzip(GzipEncoder<FileBytesReader>),
    Brotli(BrotliEncoder<FileBytesReader>),
    Deflate(DeflateEncoder<FileBytesReader>),
}

// Get compressed content via streaming
struct EncodingReader {
    encoder: Encoder,
}

impl Stream for EncodingReader {
    type Item = Result<Vec<u8>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let encoder = &mut self.get_mut().encoder;

        let poll = match encoder {
            Encoder::Gzip(encoder) => Pin::new(encoder).poll_next(cx),
            Encoder::Brotli(encoder) => Pin::new(encoder).poll_next(cx),
            Encoder::Deflate(encoder) => Pin::new(encoder).poll_next(cx),
        };

        match ready!(poll) {
            Some(rst) => match rst {
                Ok(s) => Poll::Ready(Some(Ok(s.to_vec()))),
                Err(err) => Poll::Ready(Some(Err(err))),
            },
            None => Poll::Ready(None),
        }
    }
}
