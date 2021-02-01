use crate::{config::ServerConfig, connect};
use futures_util::ready;
use futures_util::stream::{Stream, StreamExt};
use hyper::server::{accept::from_stream, conn::Http, Builder};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;

pub async fn run(tcp: TcpListener, config: ServerConfig) {
    let config = Arc::new(config);

    let stream = AcceptTcpStream::new(tcp).filter_map(|rst| {
        let config = config.clone();
        async move {
            let (stream, ip) = match rst {
                Ok(val) => val,
                // Failed to receive request
                Err(_) => return None,
            };

            // HTTPS
            if let Some(tls) = &config.tls {
                let stream = match tls.clone().accept(stream).await {
                    Ok(s) => s,
                    // TLS connection failed
                    Err(_) => return None,
                };

                let (_, session) = stream.get_ref();
                // TODO
                // Matching certificate
                let hostname = match session.get_sni_hostname() {
                    Some(name) => name,
                    None => return None,
                };

                let i = config
                    .sites
                    .iter()
                    .position(|site| site.host.is_match(hostname))
                    .unwrap();

                return Some(Ok::<_, hyper::Error>(HttpConnect::TlsStream(stream, ip, i)));
            }

            // HTTP
            Some(Ok::<_, hyper::Error>(HttpConnect::Stream(stream, ip)))
        }
    });

    let service = make_service_fn(|req: &HttpConnect| {
        let config = config.clone();
        let (remote, site_position) = match req {
            HttpConnect::Stream(_, ip) => (*ip, None),
            HttpConnect::TlsStream(_, ip, i) => (*ip, Some(*i)),
        };
        async move {
            let service = service_fn(move |req| {
                let sites = match site_position {
                    Some(i) => vec![config.sites[i].clone()],
                    None => config.sites.clone(),
                };
                connect(req, remote, sites)
            });
            Ok::<_, Infallible>(service)
        }
    });

    let http = Http::new();

    let _ = Builder::new(from_stream(stream), http).serve(service).await;
}

// Accept stream and remote ip from TcpListener
struct AcceptTcpStream {
    listener: TcpListener,
}

impl AcceptTcpStream {
    fn new(tcp: TcpListener) -> Self {
        Self { listener: tcp }
    }
}

impl Stream for AcceptTcpStream {
    type Item = Result<(TcpStream, IpAddr)>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = self.listener.poll_accept(cx);
        let rst = ready!(poll);
        let item = rst.map(|(stream, addr)| (stream, addr.ip()));
        Poll::Ready(Some(item))
    }
}

// Distinguish between http and https
pub enum HttpConnect {
    Stream(TcpStream, IpAddr),
    TlsStream(TlsStream<TcpStream>, IpAddr, usize),
}

impl AsyncRead for HttpConnect {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        match self.get_mut() {
            Self::Stream(stream, _) => Pin::new(stream).poll_read(cx, buf),
            Self::TlsStream(stream, _, _) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for HttpConnect {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        match self.get_mut() {
            Self::Stream(stream, _) => Pin::new(stream).poll_write(cx, buf),
            Self::TlsStream(stream, _, _) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self.get_mut() {
            Self::Stream(stream, _) => Pin::new(stream).poll_flush(cx),
            Self::TlsStream(stream, _, _) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self.get_mut() {
            Self::Stream(stream, _) => Pin::new(stream).poll_shutdown(cx),
            Self::TlsStream(stream, _, _) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
