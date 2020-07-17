use futures::future::FutureExt;
use hyper::client::connect::{Connected, Connection};
use hyper::client::HttpConnector;
use hyper::service::Service;
use hyper::{Body, Client, Request, Response, Uri};
use lazy_static::lazy_static;
use std::future::Future;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{self, AsyncRead, AsyncWrite};
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::{ClientConfig, Session};
use tokio_rustls::webpki::DNSNameRef;
use tokio_rustls::TlsConnector;
use webpki_roots::TLS_SERVER_ROOTS;

pub async fn request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    lazy_static! {
        static ref CLIENT: Client<Connector<HttpConnector>> =
            Client::builder().build(Connector::new());
    }
    CLIENT.request(req).await
}

// from: https://github.com/ctz/hyper-rustls/blob/master/src/connector.rs
#[derive(Clone)]
pub struct Connector<T> {
    http: T,
    tls: Arc<ClientConfig>,
}

impl Connector<HttpConnector> {
    pub fn new() -> Self {
        let mut http = HttpConnector::new();
        http.enforce_http(false);

        let mut config = ClientConfig::new();
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        config
            .root_store
            .add_server_trust_anchors(&TLS_SERVER_ROOTS);

        Self {
            http,
            tls: Arc::new(config),
        }
    }
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

impl<T> Service<Uri> for Connector<T>
where
    T: Service<Uri>,
    T::Response: Connection + AsyncRead + AsyncWrite + Send + Unpin + 'static,
    T::Future: Send + 'static,
    T::Error: Into<BoxError>,
{
    type Response = Stream<T::Response>;
    type Error = BoxError;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, BoxError>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.http
            .poll_ready(cx)
            .map(|result| result.map_err(|err| err.into()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        let is_https = dst.scheme_str() == Some("https");
        if is_https {
            let config = self.tls.clone();
            let host = dst.host().unwrap_or_default().to_string();
            let connecting = self.http.call(dst);

            async move {
                let stream = connecting.await.map_err(Into::into)?;
                let connector = TlsConnector::from(config);
                let domain = DNSNameRef::try_from_ascii_str(&host)
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid dnsname"))?;

                let tls = connector
                    .connect(domain, stream)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                Ok(Stream::Https(tls))
            }
            .boxed()
        } else {
            let connecting = self.http.call(dst);
            async move {
                let stream = connecting.await.map_err(Into::into)?;
                Ok(Stream::Http(stream))
            }
            .boxed()
        }
    }
}

pub enum Stream<T> {
    Http(T),
    Https(TlsStream<T>),
}

impl<T: AsyncRead + AsyncWrite + Connection + Unpin> Connection for Stream<T> {
    fn connected(&self) -> Connected {
        match self {
            Stream::Http(s) => s.connected(),
            Stream::Https(s) => {
                let (tcp, tls) = s.get_ref();
                if tls.get_alpn_protocol() == Some(b"h2") {
                    tcp.connected().negotiated_h2()
                } else {
                    tcp.connected()
                }
            }
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for Stream<T> {
    #[inline]
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [MaybeUninit<u8>]) -> bool {
        match self {
            Stream::Http(s) => s.prepare_uninitialized_buffer(buf),
            Stream::Https(s) => s.prepare_uninitialized_buffer(buf),
        }
    }

    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, io::Error>> {
        match Pin::get_mut(self) {
            Stream::Http(s) => Pin::new(s).poll_read(cx, buf),
            Stream::Https(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl<T: AsyncWrite + AsyncRead + Unpin> AsyncWrite for Stream<T> {
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match Pin::get_mut(self) {
            Stream::Http(s) => Pin::new(s).poll_write(cx, buf),
            Stream::Https(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match Pin::get_mut(self) {
            Stream::Http(s) => Pin::new(s).poll_flush(cx),
            Stream::Https(s) => Pin::new(s).poll_flush(cx),
        }
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match Pin::get_mut(self) {
            Stream::Http(s) => Pin::new(s).poll_shutdown(cx),
            Stream::Https(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}
