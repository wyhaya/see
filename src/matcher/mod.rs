mod host;
mod ip;
mod location;
mod wildcard;

pub use host::HostMatcher;
pub use ip::IpMatcher;
pub use location::LocationMatcher;
pub use wildcard::WildcardMatcher;

pub const REGEX_WORD: char = '~';
pub const START_WORD: char = '^';
pub const END_WORD: char = '$';
pub const ANY_WORD: char = '*';

pub fn replace_match_keyword(text: &str, start: char) -> Option<String> {
    if text.starts_with(start) {
        let s = text.replacen(start, "", 1);
        Some(s.trim().to_owned())
    } else {
        None
    }
}

#[test]
fn test_replace_match_keyword() {
    assert_eq!(
        replace_match_keyword("~123", REGEX_WORD),
        Some(String::from("123"))
    );
    assert_eq!(
        replace_match_keyword("^123", START_WORD),
        Some(String::from("123"))
    );
    assert_eq!(
        replace_match_keyword("$123", END_WORD),
        Some(String::from("123"))
    );
}
