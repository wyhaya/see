use crate::config::{Headers, Setting, SiteConfig, Var};
use crate::{client, headers_merge, response_error_page};
use hyper::header::ACCEPT_ENCODING;
use hyper::{header::HOST, Body, Method, Request, Response, StatusCode, Uri};

#[derive(Debug, Clone)]
pub struct Proxy {
    pub url: Var<Uri>,
    pub method: Option<Method>,
    pub headers: Setting<Headers>,
}

impl Proxy {
    pub async fn request(self, mut req: Request<Body>, config: &SiteConfig) -> Response<Body> {
        let encoding = req.headers().get(ACCEPT_ENCODING).cloned();

        let url = self.url.map(|s, r| {
            let result = r.replace(s, &req);
            result.parse::<Uri>().unwrap()
        });
        *req.uri_mut() = url;

        if let Some(method) = self.method {
            *req.method_mut() = method;
        }

        // Delete the host header, by default hyper will add the correct host
        req.headers_mut().remove(HOST);

        // todo
        if let Setting::Value(headers) = self.headers {
            let mut h = req.headers().clone();
            headers_merge(&mut h, headers, &req);
            *req.headers_mut() = h;
        }

        match client::request(req).await {
            Ok(res) => res,
            Err(err) => {
                let status = if err.is_timeout() {
                    StatusCode::GATEWAY_TIMEOUT
                } else {
                    StatusCode::BAD_GATEWAY
                };
                response_error_page(encoding.as_ref(), &config, status).await
            }
        }
    }
}
