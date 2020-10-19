use crate::config::{transform, Var};
use crate::{exit, ResponseExtend};
use hyper::header::{HeaderValue, LOCATION};
use hyper::{Body, Request, Response, StatusCode};

#[derive(Debug, Clone)]
pub struct Rewrite {
    location: Var<HeaderValue>,
    status: RewriteStatus,
}

impl Rewrite {
    pub fn new(location: &str, status: RewriteStatus) -> Self {
        Self {
            location: Var::from(location).map_none(transform::to_header_value),
            status,
        }
    }

    pub fn response(self, req: &Request<Body>) -> Response<Body> {
        let value = self.location.map(|s, r| {
            let rst = r.replace(s, &req);
            HeaderValue::from_str(&rst).unwrap()
        });

        let status = match self.status {
            RewriteStatus::_301 => StatusCode::MOVED_PERMANENTLY,
            RewriteStatus::_302 => StatusCode::FOUND,
        };

        return Response::new(Body::empty())
            .status(status)
            .header(LOCATION, value);
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RewriteStatus {
    _301,
    _302,
}

impl From<&str> for RewriteStatus {
    fn from(s: &str) -> Self {
        match s {
            "301" => RewriteStatus::_301,
            "302" => RewriteStatus::_302,
            _ => exit!(
                "Cannot parse `{}` to rewrite status, optional value: '301' '302'",
                s
            ),
        }
    }
}

impl Default for RewriteStatus {
    fn default() -> Self {
        RewriteStatus::_302
    }
}
