use crate::compress::CompressMode;
use async_compression::stream::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use futures::stream::Stream;
use hyper::body::Bytes;
use hyper::Body;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{self, AsyncRead};

pub fn create_file_body(file: File, buf_size: usize, mode: Option<CompressMode>) -> Body {
    match mode {
        Some(c) => {
            let f = FileBytesReader { file, buf_size };
            let encoder = match c {
                CompressMode::Gzip(l) => Encoder::Gzip(GzipEncoder::with_quality(f, l)),
                CompressMode::Br(l) => Encoder::Brotli(BrotliEncoder::with_quality(f, l)),
                CompressMode::Deflate(l) => {
                    Encoder::Deflate(DeflateEncoder::with_quality(f, l))
                }
            };
            Body::wrap_stream(EncodingReader { encoder })
        }
        None => Body::wrap_stream(FileReader { file, buf_size }),
    }
}


struct FileReader {
    file: File,
    buf_size: usize,
}

impl Stream for FileReader {
    type Item = io::Result<Vec<u8>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0; self.buf_size];
        let poll = Pin::new(&mut self.get_mut().file).poll_read(cx, &mut buf);

        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => match res {
                Ok(n) => {
                    if n == 0 {
                        Poll::Ready(None)
                    } else {
                        let data = buf[..n].to_vec();
                        Poll::Ready(Some(Ok(data)))
                    }
                }
                Err(err) => Poll::Ready(Some(Err(err))),
            },
        }
    }
}

struct FileBytesReader {
    file: File,
    buf_size: usize,
}

impl Stream for FileBytesReader {
    type Item = io::Result<Bytes>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0; self.buf_size];
        let poll = Pin::new(&mut self.get_mut().file).poll_read(cx, &mut buf);
        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => match res {
                Ok(n) => {
                    if n == 0 {
                        Poll::Ready(None)
                    } else {
                        let data = buf[..n].to_vec();
                        Poll::Ready(Some(Ok(Bytes::from(data))))
                    }
                }
                Err(err) => Poll::Ready(Some(Err(err))),
            },
        }
    }
}

enum Encoder {
    Gzip(GzipEncoder<FileBytesReader>),
    Brotli(BrotliEncoder<FileBytesReader>),
    Deflate(DeflateEncoder<FileBytesReader>),
}

struct EncodingReader {
    encoder: Encoder,
}

impl Stream for EncodingReader {
    type Item = io::Result<Vec<u8>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let encoder = &mut self.get_mut().encoder;
        let poll = match encoder {
            Encoder::Gzip(encoder) => Pin::new(encoder).poll_next(cx),
            Encoder::Brotli(encoder) => Pin::new(encoder).poll_next(cx),
            Encoder::Deflate(encoder) => Pin::new(encoder).poll_next(cx),
        };
        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(op) => {
                let op: Option<std::io::Result<Bytes>> = op;
                match op {
                    Some(res) => match res {
                        Ok(s) => Poll::Ready(Some(Ok(s.to_vec()))),
                        Err(err) => Poll::Ready(Some(Err(err))),
                    },
                    None => Poll::Ready(None),
                }
            }
        }
    }
}
