use crate::default;
use crate::ResponseExt;
use hyper::header::{HeaderValue, AUTHORIZATION, WWW_AUTHENTICATE};
use hyper::{Body, Request, Response, StatusCode};

#[derive(Debug, Clone)]
pub struct Auth(String);

impl Auth {
    pub fn basic(user: &str, password: &str) -> Self {
        let s = format!("{}:{}", user, password);
        Self(format!("Basic {}", base64::encode(&s)))
    }

    pub fn response(&self, req: &Request<Body>) -> Option<Response<Body>> {
        let check = req
            .headers()
            .get(AUTHORIZATION)
            .map(|val| val == &self.0)
            .unwrap_or(false);

        if !check {
            return Some(Response::error(StatusCode::UNAUTHORIZED).header(
                WWW_AUTHENTICATE,
                HeaderValue::from_static(default::AUTH_MESSAGE),
            ));
        }

        None
    }
}
