use hyper::client::HttpConnector;
use hyper::{Body, Client, Error, Request, Response};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use lazy_static::lazy_static;

pub async fn request(req: Request<Body>) -> Result<Response<Body>, Error> {
    lazy_static! {
        static ref CLIENT: Client<HttpsConnector<HttpConnector>> = Client::builder().build(
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build()
        );
    }
    CLIENT.request(req).await
}
