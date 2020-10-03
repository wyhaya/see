use crate::config::transform;
use crate::matcher::{replace_match_keyword, WildcardMatcher, ANY_WORD, REGEX_WORD};
use regex::Regex;

// Match http header 'host'
#[derive(Debug, Clone)]
pub struct HostMatcher {
    modes: Vec<MatchMode>,
}

#[derive(Debug, Clone)]
enum MatchMode {
    Text(String),
    Wildcard(WildcardMatcher),
    Regex(Regex),
}

impl Default for HostMatcher {
    fn default() -> HostMatcher {
        HostMatcher {
            modes: Vec::with_capacity(0),
        }
    }
}

impl HostMatcher {
    pub fn new(items: Vec<String>) -> HostMatcher {
        let mut modes = vec![];

        for item in items {
            // Use regex: ~^example\.com$
            if let Some(raw) = replace_match_keyword(&item, REGEX_WORD) {
                let reg = transform::to_regex(raw);
                modes.push(MatchMode::Regex(reg));
                continue;
            }

            // Use wildcard match: *.example.com
            if item.contains(ANY_WORD) {
                let wildcard = MatchMode::Wildcard(WildcardMatcher::new(&item));
                modes.push(wildcard);
                continue;
            }

            // Plain Text: example.com
            modes.push(MatchMode::Text(item));
        }

        HostMatcher { modes }
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
                MatchMode::Regex(reg) => {
                    if reg.is_match(host) {
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

    macro_rules! host_matcher {
        ($host: expr) => {
            HostMatcher::new(vec![$host.to_string()])
        };
    }

    #[test]
    fn create() {}

    #[test]
    fn text() {
        let matcher = host_matcher!("example.com");
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("-example.com"));
        assert!(!matcher.is_match("example.com.cn"));
    }

    #[test]
    fn regex() {
        let matcher = host_matcher!("~^example.com$");
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("test.example.com"));
    }

    #[test]
    fn multiple() {}
}
