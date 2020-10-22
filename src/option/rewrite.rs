use crate::config::Var;
use crate::{util, ResponseExt};
use hyper::header::{HeaderValue, LOCATION};
use hyper::{Body, Request, Response, StatusCode};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Rewrite {
    location: Var<HeaderValue>,
    status: RewriteStatus,
}

impl Rewrite {
    pub fn new(location: &str, status: RewriteStatus) -> Result<Self, String> {
        let location = match Var::from(location) {
            Var::Some(s, r) => Var::Some(s, r),
            Var::None(s) => Var::None(util::to_header_value(&s)?),
        };

        Ok(Self { location, status })
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

impl FromStr for RewriteStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "301" => Ok(RewriteStatus::_301),
            "302" => Ok(RewriteStatus::_302),
            _ => Err(format!(
                "Cannot parse `{}` to rewrite status, optional value: '301' '302'",
                s
            )),
        }
    }
}

impl Default for RewriteStatus {
    fn default() -> Self {
        RewriteStatus::_302
    }
}
