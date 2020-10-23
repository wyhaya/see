use crate::matcher::WildcardMatcher;
use std::collections::BTreeSet;

// Match http header 'host'
#[derive(Debug, Clone)]
pub struct HostMatcher {
    modes: Vec<MatchMode>,
}

#[derive(Debug, Clone)]
enum MatchMode {
    Text(String),
    Wildcard(WildcardMatcher),
}

impl Default for HostMatcher {
    fn default() -> HostMatcher {
        HostMatcher {
            modes: Vec::default(),
        }
    }
}

impl HostMatcher {
    // Creating a collection of host matchers
    pub fn new(items: Vec<&str>) -> Self {
        Self {
            modes: items
                .into_iter()
                .collect::<BTreeSet<&str>>()
                .into_iter()
                .map(|item| {
                    if item.contains('*') {
                        // Use wildcard match: *.example.com
                        MatchMode::Wildcard(WildcardMatcher::new(item))
                    } else {
                        // Plain Text: example.com
                        MatchMode::Text(item.to_string())
                    }
                })
                .collect::<Vec<MatchMode>>(),
        }
    }

    pub fn get_raw(&self) -> Vec<&String> {
        let mut v = vec![];
        for item in &self.modes {
            match item {
                MatchMode::Text(s) => {
                    v.push(s);
                }
                _ => todo!(),
            }
        }
        v
    }

    pub fn is_empty(&self) -> bool {
        self.modes.is_empty()
    }

    pub fn is_match(&self, host: &str) -> bool {
        // Host is not set, matches any value
        if self.is_empty() {
            return true;
        }

        for matcher in &self.modes {
            match matcher {
                MatchMode::Text(text) => {
                    if text == host {
                        return true;
                    }
                }
                MatchMode::Wildcard(wildcard) => {
                    if wildcard.is_match(host) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create() {}

    #[test]
    fn text() {
        let matcher = HostMatcher::new(vec!["example.com"]);
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("-example.com"));
        assert!(!matcher.is_match("example.com.cn"));
    }

    #[test]
    fn wildcard() {
        let matcher = HostMatcher::new(vec!["*.com"]);
        assert!(matcher.is_match("a.com"));
        assert!(!matcher.is_match("a.cn"));
        assert!(!matcher.is_match("a.a.cn"));
    }
}
