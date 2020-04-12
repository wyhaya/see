use crate::config::ServerConfig;
use crate::connect;
use crate::connect::Connect;
use futures::StreamExt;
use hyper::server::accept::from_stream;
use hyper::server::conn::Http;
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use tokio::net::TcpListener;

pub async fn run(mut tcp: TcpListener, config: ServerConfig) {
    let stream = tcp.incoming().filter_map(|stream| {
        let config = config.clone();
        async move {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => return None,
            };
            return Some(Ok::<_, hyper::Error>(Connect::Stream(stream, config.sites)));
        }
    });

    let service = make_service_fn(|req: &Connect| {
        let config = match req {
            Connect::Stream(_, c) => c.clone(),
            Connect::TlsStream(_, c) => c.clone(),
        };
        async { Ok::<_, Infallible>(service_fn(move |req| connect(req, config.clone()))) }
    });

    let http = Http::new();

    hyper::server::Builder::new(from_stream(stream), http)
        .serve(service)
        .await;
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
//
// let http = Http::new();
//
// let service = make_service_fn(|req: &Req| {
//     let config = match req {
//         Req::Stream(_, c) => c.clone(),
//         Req::TlsStream(_, c) => c.clone(),
//     };
//     async { Ok::<_, Infallible>(service_fn(move |req| connect(req, config.clone()))) }
// });
//
// let server = hyper::server::Builder::new(from_stream(stream), http)
//     .serve(service)
//     .await;

//        servers.push(server);

// }