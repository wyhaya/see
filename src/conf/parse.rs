use lazy_static::lazy_static;
use regex::Regex;
use std::fmt::{self, Display, Formatter};
use std::iter::Enumerate;
use std::ops::Index;
use std::str::{FromStr, Lines};

// A block containing multiple directive
// {
//    key value
// }
#[derive(Debug)]
pub struct Block {
    line: usize,
    directives: Vec<Directive>,
}

impl Block {
    fn new(line: usize) -> Self {
        Self {
            line,
            directives: Vec::new(),
        }
    }

    fn push(&mut self, name: String, value: Value, line: usize) {
        self.directives.push(Directive { line, name, value });
    }

    // Get the line number of the block
    pub fn line(&self) -> usize {
        self.line
    }

    // Get the first directive by name
    pub fn get(&self, name: &str) -> Option<&Directive> {
        self.directives.iter().find(|item| item.name == name)
    }

    // Get all directive by specific name
    pub fn get_all_by_name(&self, name: &str) -> Vec<&Directive> {
        self.directives
            .iter()
            .filter(|item| item.name == name)
            .collect::<Vec<&Directive>>()
    }

    // Get all directive by multiple names
    pub fn get_all_by_names(&self, names: &[&str]) -> Vec<&Directive> {
        self.directives
            .iter()
            .filter(|item| names.contains(&item.name()))
            .collect::<Vec<&Directive>>()
    }

    // Get all directives
    pub fn directives(&self) -> &Vec<Directive> {
        &self.directives
    }
}

// Parsing the 'str' to block
impl FromStr for Block {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = LineParse::new(s);
        parse(&mut lines, 0, None)
    }
}

impl Index<&str> for Block {
    type Output = Directive;
    fn index(&self, name: &str) -> &Self::Output {
        self.get(name).expect(&format!("'{}' doesn't exist", name))
    }
}

#[derive(Debug)]
pub struct Directive {
    line: usize,
    name: String,
    value: Value,
}

#[derive(Debug)]
pub enum Value {
    // name
    None,
    // name value
    String(String),
    // name on | name off
    Boolean(bool),
    // name { ... }
    Block(Block),
    // name value { ... }
    ValueBlock(String, Block),
}

impl Directive {
    // Get the name of the directive
    pub fn name(&self) -> &str {
        &self.name
    }

    // Get the line number of the directive
    pub fn line(&self) -> usize {
        self.line
    }

    pub fn is_string(&self) -> bool {
        matches!(self.value, Value::String(_))
    }

    pub fn is_on(&self) -> bool {
        match self.value {
            Value::Boolean(val) => val,
            _ => false,
        }
    }

    pub fn is_off(&self) -> bool {
        match self.value {
            Value::Boolean(val) => !val,
            _ => false,
        }
    }

    pub fn is_block(&self) -> bool {
        matches!(self.value, Value::Block(_))
    }

    pub fn as_source_str(&self) -> Option<&str> {
        match &self.value {
            Value::String(val) => Some(val),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.value {
            Value::Boolean(val) => Some(val),
            _ => None,
        }
    }

    pub fn as_block(&self) -> Option<&Block> {
        match &self.value {
            Value::Block(val) => Some(val),
            _ => None,
        }
    }

    pub fn as_value_block(&self) -> Option<(&str, &Block)> {
        match &self.value {
            Value::ValueBlock(val, block) => Some((val, block)),
            _ => None,
        }
    }
}

struct LineParse<'a> {
    iter: Enumerate<Lines<'a>>,
}

impl<'a> Iterator for LineParse<'a> {
    type Item = (usize, Line);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(n, s)| (n + 1, Self::parse_line(s)))
    }
}

#[derive(Debug)]
enum Line {
    Invalid,
    Error(Error),
    Name(String),
    NameValue(String, String),
    NameBlock(String),
    NameValueBlock(String, String),
    BlockEnd,
}

lazy_static! {
    static ref COMMENT_REGEX: Regex = Regex::new("#.*$").unwrap();
    static ref KV_REGEX: Regex = Regex::new(r"^(?P<name>[^\s]+)\s+(?P<value>.+)$").unwrap();
}

impl<'a> LineParse<'a> {
    fn new(content: &'a str) -> Self {
        Self {
            iter: content.lines().enumerate(),
        }
    }

    fn parse_line(line: &str) -> Line {
        // Remove comment and space
        let line = COMMENT_REGEX.replace(line, "");
        let line = line.trim();
        if line.is_empty() {
            return Line::Invalid;
        }

        // name value? {
        if line.ends_with('{') {
            let mut line = line.to_string();
            line.pop();
            match line.split_whitespace().count() {
                // name {
                1 => {
                    return Line::NameBlock(line.trim_end().to_string());
                }
                // name value {
                2 => {
                    let mut sp = line.split_whitespace();
                    let name = sp.next().unwrap().to_string();
                    let value = sp.next().unwrap().to_string();
                    return Line::NameValueBlock(name, value);
                }
                len => {
                    if len == 0 {
                        return Line::Error(Error::BlockStart);
                    } else {
                        return Line::Error(Error::ValueLength);
                    }
                }
            };
        }

        if line.starts_with('}') || line.ends_with('}') {
            // Closing brackets must be on a separate line
            if line != "}" {
                return Line::Error(Error::BlockEnd);
            } else {
                return Line::BlockEnd;
            }
        }

        // Only name
        if line.split_whitespace().count() == 1 {
            return Line::Name(line.to_string());
        }

        // name ... ...
        let cap = KV_REGEX.captures(line).unwrap();
        let name = cap.name("name").unwrap().as_str().to_string();
        let value = cap.name("value").unwrap().as_str().to_string();
        Line::NameValue(name, value)
    }
}

fn parse<I: Iterator<Item = (usize, Line)>>(
    iter: &mut I,
    index: usize,
    in_block: Option<usize>,
) -> Result<Block, ParseError> {
    let mut block = Block::new(index);

    while let Some((n, line)) = iter.next() {
        match line {
            Line::Invalid => {
                continue;
            }
            Line::Error(err) => {
                return Err(ParseError(n, err));
            }
            Line::BlockEnd => {
                return match in_block {
                    Some(_) => Ok(block),
                    None => Err(ParseError(n, Error::Redundant)),
                };
            }
            Line::Name(name) => {
                block.push(name, Value::None, n);
            }
            Line::NameValue(name, val) => {
                if val == "on" {
                    block.push(name, Value::Boolean(true), n);
                } else if val == "off" {
                    block.push(name, Value::Boolean(false), n);
                } else {
                    block.push(name, Value::String(val), n);
                }
            }
            Line::NameBlock(name) => {
                let child = parse(iter, n, Some(n))?;
                block.push(name, Value::Block(child), n);
            }
            Line::NameValueBlock(name, val) => {
                let child = parse(iter, n, Some(n))?;
                block.push(name, Value::ValueBlock(val, child), n);
            }
        }
    }

    if let Some(n) = in_block {
        return Err(ParseError(n, Error::Lack));
    }

    Ok(block)
}

#[derive(Debug)]
pub struct ParseError(usize, Error);

#[derive(Debug)]
enum Error {
    BlockStart,
    BlockEnd,
    ValueLength,
    Lack,
    Redundant,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let msg = match self.1 {
            Error::BlockStart => "'{' can only appear at the end of a line",
            Error::BlockEnd => "'}' Must be on a separate line",
            Error::ValueLength => {
                "The length of the value is wrong\nTry: 'name {' or 'name value {'"
            }
            Error::Lack => "Missing '}'",
            Error::Redundant => "Redundant '}'",
        };
        write!(f, "[line: {}] {}", self.0, msg)
    }
}
