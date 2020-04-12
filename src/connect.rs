use crate::config::SiteConfig;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::io::{self, AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

pub enum Connect {
    Stream(TcpStream, Vec<SiteConfig>),
    TlsStream(TlsStream<TcpStream>, Vec<SiteConfig>),
}

impl AsyncRead for Connect {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Connect::Stream(stream, _) => Pin::new(stream).poll_read(cx, buf),
            Connect::TlsStream(stream, _) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Connect {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Connect::Stream(stream, _) => Pin::new(stream).poll_write(cx, buf),
            Connect::TlsStream(stream, _) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Connect::Stream(stream, _) => Pin::new(stream).poll_flush(cx),
            Connect::TlsStream(stream, _) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Connect::Stream(stream, _) => Pin::new(stream).poll_shutdown(cx),
            Connect::TlsStream(stream, _) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
