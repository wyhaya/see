use crate::compress::{compress_data, CompressMode};
use futures::stream::Stream;
use hyper::Body;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{self, AsyncRead};

pub struct FileBody {
    file: File,
    size: usize,
    mode: Option<CompressMode>,
}

impl FileBody {
    pub fn new(file: File, size: usize, mode: Option<CompressMode>) -> Body {
        Body::wrap_stream(Self { file, size, mode })
    }
}

impl Stream for FileBody {
    type Item = io::Result<Vec<u8>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0; self.size];
        let mode = self.mode;
        let poll = Pin::new(&mut self.get_mut().file).poll_read(cx, &mut buf);

        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => match res {
                Ok(n) => {
                    if n == 0 {
                        return Poll::Ready(None);
                    }
                    match mode {
                        Some(mode) => {
                            let mut fu = Box::pin(compress_data(&buf[..n], mode));
                            match Pin::new(&mut fu).poll(cx) {
                                Poll::Pending => Poll::Pending,
                                Poll::Ready(res) => Poll::Ready(Some(res)),
                            }
                        }
                        None => {
                            let data = buf[..n].to_vec();
                            Poll::Ready(Some(Ok(data)))
                        }
                    }
                }
                Err(err) => Poll::Ready(Some(Err(err))),
            },
        }
    }
}
