use crate::matcher::WildcardMatcher;
use crate::util;
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
    fn new(s: &str) -> Result<Self, String> {
        if s.contains('*') {
            Ok(Self::Wildcard(WildcardMatcher::new(s)))
        } else {
            Ok(Self::Ip(util::to_ip_addr(s)?))
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
    pub fn new(allow: Vec<&str>, deny: Vec<&str>) -> Result<Self, String> {
        let mut a = vec![];
        for item in allow {
            a.push(MatchMode::new(item)?);
        }
        let mut d = vec![];
        for item in deny {
            d.push(MatchMode::new(item)?);
        }
        Ok(Self { allow: a, deny: d })
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
