use crate::ResponseExt;
use hyper::header::{HeaderValue, ALLOW};
use hyper::{Body, Method as HttpMethod, Request, Response, StatusCode};

#[derive(Debug, Clone)]
pub struct Method {
    allow: Vec<HttpMethod>,
}

impl Method {
    pub fn new(allow: Vec<HttpMethod>) -> Self {
        Self { allow }
    }

    pub fn response(&self, req: &Request<Body>) -> Option<Response<Body>> {
        if !self.allow.contains(req.method()) {
            // Show allowed methods in header
            if req.method() == HttpMethod::OPTIONS {
                let allow = self
                    .allow
                    .iter()
                    .map(|m| m.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ");

                return Some(
                    Response::error(StatusCode::METHOD_NOT_ALLOWED)
                        .header(ALLOW, HeaderValue::from_str(&allow).unwrap()),
                );
            }

            return Some(Response::error(StatusCode::METHOD_NOT_ALLOWED));
        }

        None
    }
}
