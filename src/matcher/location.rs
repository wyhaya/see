use crate::util;
use globset::GlobMatcher;
use regex::Regex;

// Match location
#[derive(Debug, Clone)]
pub struct LocationMatcher(MatchMode);

#[derive(Debug, Clone)]
enum MatchMode {
    Glob(GlobMatcher),
    Regex(Regex),
    Start(String),
    End(String),
}

impl LocationMatcher {
    // Matching with glob expression
    pub fn glob(location: &str) -> Result<Self, String> {
        Ok(LocationMatcher(MatchMode::Glob(util::to_glob(location)?)))
    }

    // Matching using regular expression
    pub fn regex(location: &str) -> Result<Self, String> {
        Ok(LocationMatcher(MatchMode::Regex(util::to_regex(location)?)))
    }

    // Matching the start of a location with a string
    pub fn start(location: &str) -> Self {
        LocationMatcher(MatchMode::Start(location.to_string()))
    }

    // Matching the end of a location with a string
    pub fn end(location: &str) -> Self {
        LocationMatcher(MatchMode::End(location.to_string()))
    }

    pub fn is_match(&self, path: &str) -> bool {
        match &self.0 {
            MatchMode::Glob(glob) => glob.is_match(path),
            MatchMode::Regex(reg) => reg.is_match(path),
            MatchMode::Start(s) => path.starts_with(s),
            MatchMode::End(s) => path.ends_with(s),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create() {}

    #[test]
    fn start() {
        let matcher = LocationMatcher::start("/test/");
        assert!(matcher.is_match("/test/a"));
        assert!(matcher.is_match("/test/a/b"));
    }

    #[test]
    fn end() {
        let matcher = LocationMatcher::end(".png");
        assert!(matcher.is_match("/test/a.png"));
        assert!(matcher.is_match("/test/a/b.png"));
    }

    #[test]
    fn regex() {
        let matcher = LocationMatcher::regex(r"/test/.*").unwrap();
        assert!(matcher.is_match("/test/a"));
        assert!(matcher.is_match("/test/a/b"));
    }

    #[test]
    fn glob() {
        let matcher = LocationMatcher::glob("/test/*").unwrap();
        assert!(matcher.is_match("/test/a"));
        assert!(matcher.is_match("/test/a/b"));
    }
}
