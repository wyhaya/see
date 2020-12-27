use hyper::client::HttpConnector;
use hyper::{Body, Client, Error, Request, Response};
use hyper_rustls::HttpsConnector;
use lazy_static::lazy_static;

pub async fn request(req: Request<Body>) -> Result<Response<Body>, Error> {
    lazy_static! {
        static ref CLIENT: Client<HttpsConnector<HttpConnector>> =
            Client::builder().build(HttpsConnector::with_native_roots());
    }
    CLIENT.request(req).await
}
