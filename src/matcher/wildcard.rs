#[derive(Debug, Clone)]
pub struct WildcardMatcher {
    chars: Vec<char>,
}

impl WildcardMatcher {
    pub fn new(s: &str) -> Self {
        Self {
            chars: s.chars().collect::<Vec<char>>(),
        }
    }

    pub fn is_match(&self, s: &str) -> bool {
        let mut chars = s.chars();
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
mod test {
    use super::*;

    #[test]
    fn wildcard() {
        let matcher = WildcardMatcher::new("*");
        assert!(matcher.is_match("localhost"));
        assert!(!matcher.is_match(".localhost"));
        assert!(!matcher.is_match("localhost."));
        assert!(!matcher.is_match("local.host"));

        let matcher = WildcardMatcher::new("*.com");
        assert!(matcher.is_match("test.com"));
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("test.test"));
        assert!(!matcher.is_match(".test.com"));
        assert!(!matcher.is_match("test.com."));
        assert!(!matcher.is_match("test.test.com"));

        let matcher = WildcardMatcher::new("*.*");
        assert!(matcher.is_match("test.test"));
        assert!(!matcher.is_match(".test.test"));
        assert!(!matcher.is_match("test.test."));
        assert!(!matcher.is_match("test.test.test"));

        let matcher = WildcardMatcher::new("*.example.com");
        assert!(matcher.is_match("test.example.com"));
        assert!(matcher.is_match("example.example.com"));
        assert!(!matcher.is_match("test.example.com.com"));
        assert!(!matcher.is_match("test.test.example.com"));

        let matcher = WildcardMatcher::new("*.example.*");
        assert!(matcher.is_match("test.example.com"));
        assert!(matcher.is_match("example.example.com"));
        assert!(!matcher.is_match("test.test.example.test"));
        assert!(!matcher.is_match("test.example.test.test"));
    }
}
