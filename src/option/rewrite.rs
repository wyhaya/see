use crate::config::{transform, Var};
use crate::{exit, ResponseExtend};
use hyper::header::{HeaderValue, LOCATION};
use hyper::{Body, Request, Response, StatusCode};

#[derive(Debug, Clone)]
pub struct Rewrite {
    pub location: Var<HeaderValue>,
    pub status: RewriteStatus,
}

impl Rewrite {
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
            .set_status(status)
            .set_header(LOCATION, value);
    }
}

impl From<String> for Rewrite {
    fn from(s: String) -> Self {
        let mut split = s.split_whitespace();

        let location = split
            .next()
            .map(|s| Var::from(s).map_none(transform::to_header_value))
            .unwrap_or_else(|| exit!("Rewrite url cannot be empty"));

        let status = split.next().map(RewriteStatus::from).unwrap_or_default();

        Rewrite { location, status }
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
