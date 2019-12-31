use crate::*;
use config::ForceTo;
use globset::GlobMatcher;
use regex::Regex;

const REGEX_WORD: char = '~';
const WILDCARD: char = '*';

// Whether to use regular expression matching
fn get_regex_str(text: &str) -> Option<String> {
    if text.starts_with(REGEX_WORD) {
        let s = text.replacen(REGEX_WORD, "", 1);
        Some(s.trim().to_owned())
    } else {
        None
    }
}

// Match path
#[derive(Debug, Clone)]
pub struct LocationMatcher(LocationMatchMode);

#[derive(Debug, Clone)]
enum LocationMatchMode {
    Glob(GlobMatcher),
    Regex(Regex),
}

impl LocationMatcher {
    pub fn new(location: &str) -> Self {
        match get_regex_str(location) {
            Some(raw) => {
                let reg = raw.as_str().to_regex();
                LocationMatcher(LocationMatchMode::Regex(reg))
            }
            None => {
                let glob = location.to_glob().compile_matcher();
                LocationMatcher(LocationMatchMode::Glob(glob))
            }
        }
    }

    pub fn is_match(&self, path: &str) -> bool {
        match &self.0 {
            LocationMatchMode::Glob(glob) => glob.is_match(path),
            LocationMatchMode::Regex(reg) => reg.is_match(path),
        }
    }
}

// Match http header 'host'
#[derive(Debug, Clone)]
pub struct HostMatcher(Vec<HostMatcherMode>);

#[derive(Debug, Clone)]
enum HostMatcherMode {
    Text(String),
    Wildcard(HostWildcardMatcher),
    Regex(Regex),
}

impl Default for HostMatcher {
    fn default() -> HostMatcher {
        HostMatcher(Vec::with_capacity(0))
    }
}

impl HostMatcher {
    pub fn new(items: Vec<String>) -> HostMatcher {
        let mut matcher = vec![];
        for item in items {
            // Use regex: ~^example\.com$
            if let Some(raw) = get_regex_str(&item) {
                let reg = raw.as_str().to_regex();
                matcher.push(HostMatcherMode::Regex(reg));
                continue;
            }

            // Use wildcard match: *.example.com
            let has_wildcard = item.chars().any(|ch| ch == WILDCARD);
            if has_wildcard {
                let wildcard = HostMatcherMode::Wildcard(HostWildcardMatcher::new(&item));
                matcher.push(wildcard);
                continue;
            }

            // Plain Text: example.com
            matcher.push(HostMatcherMode::Text(item));
        }

        HostMatcher(matcher)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn is_match(&self, host: &str) -> bool {
        // Host is not set, matches any value
        if self.is_empty() {
            return true;
        }

        for matcher in &self.0 {
            match matcher {
                HostMatcherMode::Text(text) => {
                    if text == host {
                        return true;
                    }
                }
                HostMatcherMode::Wildcard(wildcard) => {
                    if wildcard.is_match(host) {
                        return true;
                    }
                }
                HostMatcherMode::Regex(reg) => {
                    if reg.is_match(host) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[derive(Debug, Clone)]
struct HostWildcardMatcher {
    chars: Vec<char>,
}

impl HostWildcardMatcher {
    fn new(raw: &str) -> Self {
        let mut chars = Vec::with_capacity(raw.len());
        for ch in raw.chars() {
            chars.push(ch);
        }

        Self { chars }
    }

    fn is_match(&self, host: &str) -> bool {
        let mut chars = host.chars();
        let mut dot = false;

        for ch in &self.chars {
            match ch {
                '*' => {
                    match chars.next() {
                        Some(c) => {
                            if c == '.' {
                                return false;
                            }
                        }
                        None => return false,
                    }
                    while let Some(n) = chars.next() {
                        if n == '.' {
                            dot = true;
                            break;
                        }
                    }
                }
                word => {
                    if dot {
                        if word == &'.' {
                            dot = false;
                            continue;
                        } else {
                            return false;
                        }
                    }
                    match chars.next() {
                        Some(ch) => {
                            if word != &ch {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
            }
        }

        if dot {
            return false;
        }

        chars.next().is_none()
    }
}

#[cfg(test)]
mod test_matcher {
    use super::*;

    #[test]
    fn test_location_create() {}

    #[test]
    fn test_location_glob() {
        let matcher = LocationMatcher::new("/test/*");
        assert!(matcher.is_match("/test/a"));
        assert!(matcher.is_match("/test/a/b"));
    }

    #[test]
    fn test_location_regex() {
        let matcher = LocationMatcher::new(r"~/test/\.*");
        assert!(matcher.is_match("/test/a"));
        assert!(matcher.is_match("/test/a/b"));
    }

    macro_rules! host_matcher {
        ($host: expr) => {
            HostMatcher::new(vec![$host.to_string()])
        };
    }

    #[test]
    fn test_host_create() {}

    #[test]
    fn test_host_text() {
        let matcher = host_matcher!("example.com");
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("-example.com"));
        assert!(!matcher.is_match("example.com.cn"));
    }

    #[test]
    fn test_host_wildcard() {
        let matcher = host_matcher!("*");
        assert!(matcher.is_match("localhost"));
        assert!(!matcher.is_match(".localhost"));
        assert!(!matcher.is_match("localhost."));
        assert!(!matcher.is_match("local.host"));

        let matcher = host_matcher!("*.com");
        assert!(matcher.is_match("test.com"));
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("test.test"));
        assert!(!matcher.is_match(".test.com"));
        assert!(!matcher.is_match("test.com."));
        assert!(!matcher.is_match("test.test.com"));

        let matcher = host_matcher!("*.*");
        assert!(matcher.is_match("test.test"));
        assert!(!matcher.is_match(".test.test"));
        assert!(!matcher.is_match("test.test."));
        assert!(!matcher.is_match("test.test.test"));

        let matcher = host_matcher!("*.example.com");
        assert!(matcher.is_match("test.example.com"));
        assert!(matcher.is_match("example.example.com"));
        assert!(!matcher.is_match("test.example.com.com"));
        assert!(!matcher.is_match("test.test.example.com"));

        let matcher = host_matcher!("*.example.*");
        assert!(matcher.is_match("test.example.com"));
        assert!(matcher.is_match("example.example.com"));
        assert!(!matcher.is_match("test.test.example.test"));
        assert!(!matcher.is_match("test.example.test.test"));
    }

    #[test]
    fn test_host_regex() {
        let matcher = host_matcher!("~^example.com$");
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("test.example.com"));
    }
}
