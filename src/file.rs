use crate::compress::encode;
use crate::compress::encoding::Encoding;
use futures::stream::Stream;
use hyper::Body;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{self, AsyncRead};

pub trait BodyFromFile {
    fn file(file: File, size: usize, encoding: Encoding) -> Body;
}

impl BodyFromFile for Body {
    fn file(file: File, size: usize, encoding: Encoding) -> Self {
        let stream = FileBody::new(file, size, encoding);
        Body::wrap_stream(stream)
    }
}

pub struct FileBody {
    file: File,
    size: usize,
    encoding: Encoding,
}

impl FileBody {
    pub fn new(file: File, size: usize, encoding: Encoding) -> Self {
        Self {
            file,
            size,
            encoding,
        }
    }
}

impl Stream for FileBody {
    type Item = io::Result<Vec<u8>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0; self.size];
        let encoding = self.encoding;
        let poll = Pin::new(&mut self.get_mut().file).poll_read(cx, &mut buf);

        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => match res {
                Ok(n) => {
                    if n == 0 {
                        return Poll::Ready(None);
                    }

                    let mut fu = Box::pin(encode::auto(&buf[..n], encoding));
                    match Pin::new(&mut fu).poll(cx) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(res) => Poll::Ready(Some(res)),
                    }
                }
                Err(err) => Poll::Ready(Some(Err(err))),
            },
        }
    }
}
