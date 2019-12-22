use crate::*;
use config::ForceTo;
use globset::GlobMatcher;
use regex::Regex;

// Whether to use regular expression matching
fn get_regex_str(text: &str) -> Option<String> {
    if text.starts_with("~") {
        let s = text.replacen("~", "", 1);
        Some(s.trim().to_owned())
    } else {
        None
    }
}

fn is_wildcard_mode(text: &str) -> bool {
    const TEXT: &str = "abcdefghijklmnopqrstuvwxyz0123456789-.";
    let has_wildcard = text.chars().any(|c| c == '*');
    if has_wildcard {
        return text.chars().all(|item| TEXT.chars().any(|c| item == c));
    }
    false
}

// Match http header 'host'
#[derive(Debug, Clone)]
pub struct HostMatcher(Vec<HostMatcherMode>);

#[derive(Debug, Clone)]
enum HostMatcherMode {
    Text(String),
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
            // use regex
            if let Some(s) = get_regex_str(&item) {
                let reg = s.as_str().to_regex();
                matcher.push(HostMatcherMode::Regex(reg));
                continue;
            }

            // *.example.com
            if is_wildcard_mode(&item) {
                let s = format!("^{}$", item.replace(".", r"\.").replace("*", r"[^.]+"));
                let reg = s.as_str().to_regex();
                matcher.push(HostMatcherMode::Regex(reg));
                continue;
            }

            // example.com
            matcher.push(HostMatcherMode::Text(item));
        }

        HostMatcher(matcher)
    }

    pub fn is_match(&self, host: &str) -> bool {
        // Host is not set, matches any value
        if self.0.is_empty() {
            return true;
        }

        for item in &self.0 {
            match item {
                HostMatcherMode::Text(text) => {
                    if text == host {
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

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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
    pub fn new(location: &str) -> LocationMatcher {
        match get_regex_str(location) {
            Some(s) => {
                let reg = s.as_str().to_regex();
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
