use crate::config::Force;
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

impl From<&str> for MatchMode {
    fn from(raw: &str) -> Self {
        if raw.contains(ANY_WORD) {
            MatchMode::Wildcard(WildcardMatcher::new(raw))
        } else {
            MatchMode::Ip(raw.to_ip_addr())
        }
    }
}

impl MatchMode {
    fn is_match(&self, ip: &IpAddr) -> bool {
        match self {
            MatchMode::Ip(m) => m == ip,
            MatchMode::Wildcard(m) => m.is_match(&ip.to_string()),
        }
    }
}

impl IpMatcher {
    pub fn new(allow: Vec<String>, deny: Vec<String>) -> Self {
        Self {
            allow: allow
                .iter()
                .map(|item| MatchMode::from(item.as_str()))
                .collect(),
            deny: deny
                .iter()
                .map(|item| MatchMode::from(item.as_str()))
                .collect(),
        }
    }

    pub fn is_pass(&self, ip: IpAddr) -> bool {
        if !self.allow.is_empty() {
            return self.allow.iter().any(|m| m.is_match(&ip));
        }
        return self.deny.iter().all(|m| !m.is_match(&ip));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ip() {}
}
