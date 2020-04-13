use crate::config::ServerConfig;
use crate::connect;
use crate::connect::Connect;
use futures::{Stream, StreamExt};
use hyper::server::accept::from_stream;
use hyper::server::conn::Http;
use hyper::server::Builder;
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::IpAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

pub async fn run(tcp: TcpListener, config: ServerConfig) {
    let stream = AcceptStream::new(tcp).filter_map(|res| {
        let config = config.clone();

        async move {
            let (stream, ip) = if let Ok(res) = res { res } else { return None };

            return Some(Ok::<_, hyper::Error>(Connect::Stream(
                stream,
                ip,
                config.sites,
            )));
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

        match poll {
            Poll::Ready(res) => {
                let res = res.map(|(stream, addr)| (stream, addr.ip()));
                Poll::Ready(Some(res))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

// async fn run_server(mut tcp: TcpListener, config: ServerConfig) {
//
// let stream = listener.incoming().filter_map(|stream| {
//     let config = config.clone();
//     async move {
//         let mut stream = match stream {
//             Ok(s) => s,
//             Err(_) => return None,
//         };
//
//         let has_https = config.sites.iter().any(|item| item.https.is_some());
//         if !has_https {
//             return Some(Ok::<_, hyper::Error>(Req::Stream(stream, config.sites)));
//         }
//         let mut buf = [0; 1];
//         let is_https = match stream.peek(&mut buf).await {
//             Ok(_) => buf[0] == 22,
//             Err(_) => return None,
//         };
//         if is_https {
//             // bug
//             let configs = config
//                 .sites
//                 .iter()
//                 .filter(|item| item.https.is_some())
//                 .cloned()
//                 .collect::<Vec<SiteConfig>>();
//             let config = configs[0].clone();
//             let tls = config.https.as_ref().unwrap();
//             // can accept
//             if let Ok(stream) = tls.acceptor.accept(stream).await {
//                 return Some(Ok::<_, hyper::Error>(Req::TlsStream(stream, configs)));
//             }
//         } else {
//             let mut s = vec![];
//             for item in config.sites {
//                 if item.https.is_none() {
//                     s.push(item);
//                 }
//             }
//             return Some(Ok::<_, hyper::Error>(Req::Stream(stream, s)));
//         }
//         None
//     }
// });
// }
