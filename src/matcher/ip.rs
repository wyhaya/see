use crate::config::transform;
use crate::matcher::{WildcardMatcher, ANY_WORD};
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct IpMatcher {
    allow: Vec<MatchMode>,
    deny: Vec<MatchMode>,
}

#[derive(Debug, Clone)]
enum MatchMode {
    Ip(IpAddr),
    Wildcard(WildcardMatcher),
}

impl MatchMode {
    fn new(raw: &str) -> Self {
        if raw.contains(ANY_WORD) {
            Self::Wildcard(WildcardMatcher::new(raw))
        } else {
            Self::Ip(transform::to_ip_addr(raw))
        }
    }

    fn is_match(&self, ip: &IpAddr) -> bool {
        match self {
            MatchMode::Ip(m) => m == ip,
            MatchMode::Wildcard(m) => m.is_match(&ip.to_string()),
        }
    }
}

impl IpMatcher {
    pub fn new(allow: Vec<&str>, deny: Vec<&str>) -> Self {
        Self {
            allow: allow.iter().map(|s| MatchMode::new(s)).collect(),
            deny: deny.iter().map(|s| MatchMode::new(s)).collect(),
        }
    }

    pub fn is_pass(&self, ip: IpAddr) -> bool {
        if !self.allow.is_empty() {
            return self.allow.iter().any(|m| m.is_match(&ip));
        }
        self.deny.iter().all(|m| !m.is_match(&ip))
    }
}

#[cfg(test)]
mod test {
    // use super::*;

    #[test]
    fn ip() {}
}
