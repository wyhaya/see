use crate::exit;
use lazy_static::lazy_static;
use regex::Regex;
use std::ops::Index;

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

    fn push(&mut self, key: String, value: Value, line: usize) {
        self.directives.push(Directive { line, key, value });
    }

    // Check within the current block
    // If the check fails, the process will exit
    pub fn check(&self, allow: &[&str], required: &[&str], repeat: &[&str]) {
        // Allowed values
        for directive in &self.directives {
            if !allow.contains(&directive.key.as_str()) {
                exit!(
                    "[line: {}] Unknown directive `{}`",
                    directive.line,
                    directive.key
                )
            }
        }

        // Required values
        for key in required {
            if self.get(key).is_none() {
                exit!("[line: {}] Missing directive `{}`", self.line, key)
            }
        }

        // Repeated values
        for directive in &self.directives {
            if !repeat.contains(&directive.key()) {
                let all = self.get_all(directive.key());
                if all.len() > 1 {
                    let re = all[all.len() - 1];
                    exit!("[line: {}] Repeated directive `{}`", re.line, re.key)
                }
            }
        }
    }

    // Get directive based on key
    pub fn get(&self, key: &str) -> Option<&Directive> {
        self.directives.iter().find(|item| item.key == key)
    }

    // Get all directive with a specific key
    pub fn get_all(&self, key: &str) -> Vec<&Directive> {
        self.directives
            .iter()
            .filter(|item| key == item.key)
            .collect::<Vec<&Directive>>()
    }

    pub fn directives(&self) -> &Vec<Directive> {
        &self.directives
    }
}

impl Index<&str> for Block {
    type Output = Directive;
    fn index(&self, index: &str) -> &Self::Output {
        self.get(index).unwrap()
    }
}

#[derive(Debug)]
pub struct Directive {
    line: usize,
    key: String,
    value: Value,
}

impl Directive {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn is_string(&self) -> bool {
        matches!(self.value, Value::String(_))
    }

    pub fn is_bool(&self) -> bool {
        matches!(self.value, Value::Boolean(_))
    }

    pub fn is_block(&self) -> bool {
        matches!(self.value, Value::Block(_))
    }

    pub fn to_str(&self) -> &str {
        let val = self.to_source_str();
        if val.split_whitespace().count() != 1 {
            exit!(
                "[line: {}] Directive `{}` does not allow multiple values",
                self.line,
                self.key
            )
        }
        val
    }

    pub fn to_source_str(&self) -> &str {
        match &self.value {
            Value::String(val) => val,
            _ => exit!(
                "[line: {}] Cannot convert `{}` to 'string'",
                self.line,
                self.key
            ),
        }
    }

    pub fn to_multiple_str(&self) -> Vec<&str> {
        self.to_source_str()
            .split_whitespace()
            .collect::<Vec<&str>>()
    }

    pub fn to_bool(&self) -> bool {
        match &self.value {
            Value::Boolean(val) => *val,
            _ => exit!(
                "[line: {}] Cannot convert `{}` to 'boolean'",
                self.line,
                self.key
            ),
        }
    }

    pub fn to_block(&self) -> &Block {
        match &self.value {
            Value::Block(val) => val,
            _ => exit!(
                "[line: {}] Cannot convert `{}` to 'block'",
                self.line,
                self.key,
            ),
        }
    }

    pub fn to_value_block(&self) -> (&str, &Block) {
        match &self.value {
            Value::ValueBlock(val, block) => (val, block),
            _ => exit!(
                "[line: {}] Cannot convert `{}` to 'value block'",
                self.line,
                self.key
            ),
        }
    }

    pub fn try_to_bool(&self) -> Option<bool> {
        if self.is_bool() {
            Some(self.to_bool())
        } else {
            None
        }
    }
}

#[derive(Debug)]
enum Value {
    // key value
    String(String),
    // key on | off
    Boolean(bool),
    // key { ... }
    Block(Block),
    // key value { ... }
    ValueBlock(String, Block),
}

#[derive(Debug)]
pub struct ParseError(usize, ErrorType);

#[derive(Debug)]
enum ErrorType {
    BlockEnd,
    Error,
}

pub struct ConfParser;
impl ConfParser {
    pub fn parse(content: &str) -> Result<Block, ParseError> {
        let mut lines = content.lines().enumerate();
        parse(&mut lines, 0)
    }
}

fn parse<'a, I: Iterator<Item = (usize, &'a str)>>(
    iter: &mut I,
    index: usize,
) -> Result<Block, ParseError> {
    let mut block = Block::new(index);
    let mut in_block = None;

    while let Some((mut n, line)) = iter.next() {
        n += 1;
        // Skip invalid rows
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Block end
        if line == "}" {
            return Ok(block);
        }

        match Line::from(line) {
            Line::KeyValue(key, value) => {
                if value == "on" {
                    block.push(key, Value::Boolean(true), n);
                } else if value == "off" {
                    block.push(key, Value::Boolean(false), n);
                } else {
                    block.push(key, Value::String(value.to_string()), n);
                }
            }
            Line::KeyBlock(key) => {
                in_block = Some(n);
                let child = parse(iter, n)?;
                in_block = None;
                block.push(key, Value::Block(child), n);
            }
            Line::KeyValueBlock(key, value) => {
                in_block = Some(n);
                let child = parse(iter, n)?;
                in_block = None;
                block.push(key, Value::ValueBlock(value, child), n);
            }
            Line::Error => return Err(ParseError(n, ErrorType::Error)),
        }
    }

    if let Some(n) = in_block {
        return Err(ParseError(n, ErrorType::BlockEnd));
    }

    Ok(block)
}

#[derive(Debug)]
enum Line {
    KeyValue(String, String),
    KeyBlock(String),
    KeyValueBlock(String, String),
    Error,
}

lazy_static! {
    static ref COMMENT_REGEX: Regex = Regex::new("#.*$").unwrap();
    static ref KV_REGEX: Regex = Regex::new(r"^(?P<key>[^\s]+)\s+(?P<value>.+)$").unwrap();
}

impl From<&str> for Line {
    fn from(line: &str) -> Self {
        // Remove comment
        let line = COMMENT_REGEX.replace(line, "");
        let line = line.trim();

        if line.ends_with('{') {
            // Block
            let mut s = line.to_string();
            s.pop();
            let sp = s.split_whitespace();
            let count = sp.count();
            if count == 1 {
                let mut sp = s.split_whitespace();
                let key = sp.next().unwrap();
                return Line::KeyBlock(key.to_string());
            }
            if count == 2 {
                let mut sp = s.split_whitespace();
                let key = sp.next().unwrap();
                let value = sp.next().unwrap();
                return Line::KeyValueBlock(key.to_string(), value.to_string());
            }
            Line::Error
        } else {
            let cap = KV_REGEX.captures(line).unwrap();
            let key = cap.name("key").unwrap().as_str().to_string();
            let value = cap.name("value").unwrap().as_str().to_string();
            Line::KeyValue(key, value)
        }
    }
}
