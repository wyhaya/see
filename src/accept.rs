use crate::config::{ServerConfig, SiteConfig};
use crate::connect;
use futures::{ready, Stream, StreamExt};
use hyper::server::accept::from_stream;
use hyper::server::conn::Http;
use hyper::server::Builder;
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::IpAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::io::{self, AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;

pub async fn run(tcp: TcpListener, config: ServerConfig) {
    let stream = AcceptStream::new(tcp).filter_map(|rst| {
        let config = config.clone();

        async move {
            let (stream, ip) = match rst {
                Ok(r) => r,
                Err(_) => return None,
            };

            if let Some(tls) = &config.tls {
                let stream = match tls.clone().accept(stream).await {
                    Ok(s) => s,
                    // TLS connection failed
                    Err(_) => return None,
                };

                let (_, session) = stream.get_ref();
                let name = session.get_sni_hostname().unwrap();

                let c = config
                    .sites
                    .iter()
                    .find(|d| {
                        if let Some(n) = &d.sni_name {
                            return n == name;
                        }
                        false
                    })
                    .unwrap();

                // Use HTTPS response
                return Some(Ok::<_, hyper::Error>(Connect::TlsStream(
                    stream,
                    ip,
                    vec![c.clone()],
                )));
            }

            // Use HTTP response
            Some(Ok::<_, hyper::Error>(Connect::Stream(
                stream,
                ip,
                config.sites,
            )))
        }
    });

    let service = make_service_fn(|req: &Connect| {
        let (ip, configs) = match req {
            Connect::Stream(_, i, c) => (*i, c.clone()),
            Connect::TlsStream(_, i, c) => (*i, c.clone()),
        };

        async move { Ok::<_, Infallible>(service_fn(move |req| connect(req, ip, configs.clone()))) }
    });

    let http = Http::new();

    let _ = Builder::new(from_stream(stream), http).serve(service).await;
}

// Accept stream and remote ip from TcpListener
struct AcceptStream {
    listener: TcpListener,
}

impl AcceptStream {
    fn new(tcp: TcpListener) -> Self {
        Self { listener: tcp }
    }
}

impl Stream for AcceptStream {
    type Item = io::Result<(TcpStream, IpAddr)>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = self.get_mut().listener.poll_accept(cx);
        let rst = ready!(poll);
        let item = rst.map(|(stream, addr)| (stream, addr.ip()));

        Poll::Ready(Some(item))
    }
}

// Distinguish between http and https
pub enum Connect {
    Stream(TcpStream, IpAddr, Vec<SiteConfig>),
    TlsStream(TlsStream<TcpStream>, IpAddr, Vec<SiteConfig>),
}

impl AsyncRead for Connect {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Connect::Stream(stream, _, _) => Pin::new(stream).poll_read(cx, buf),
            Connect::TlsStream(stream, _, _) => Pin::new(stream).poll_read(cx, buf),
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
            Connect::Stream(stream, _, _) => Pin::new(stream).poll_write(cx, buf),
            Connect::TlsStream(stream, _, _) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Connect::Stream(stream, _, _) => Pin::new(stream).poll_flush(cx),
            Connect::TlsStream(stream, _, _) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Connect::Stream(stream, _, _) => Pin::new(stream).poll_shutdown(cx),
            Connect::TlsStream(stream, _, _) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
