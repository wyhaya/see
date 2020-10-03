use crate::exit;
use std::fmt::Display;
use yaml_rust::yaml::Hash;
use yaml_rust::Yaml;

pub trait YamlExtend {
    fn check(&self, name: &str, keys: &[&str], must: &[&str]);
    fn try_to_string(&self) -> Option<String>;
    fn to_string<T: Display>(&self, msg: T) -> String;
    fn key_to_bool(&self, key: &str) -> bool;
    fn key_to_hash(&self, key: &str) -> &Hash;
    fn key_to_number(&self, key: &str) -> u64;
    fn key_to_string(&self, key: &str) -> String;
    fn key_to_multiple_string(&self, key: &str) -> Vec<String>;
}

impl YamlExtend for Yaml {
    fn check(&self, name: &str, keys: &[&str], must: &[&str]) {
        let hash = self.key_to_hash(name);

        // Disallowed key
        for (key, _) in hash {
            let key = key.to_string(format!("{} 'key'", name));
            if !keys.contains(&key.as_str()) {
                exit!("Unknown directive `{}` in '{}'", key, name)
            }
        }

        // Required key
        for must in must {
            if self[name][*must].is_badvalue() {
                exit!("Missing '{}' in '{}'", must, name)
            }
        }
    }

    fn try_to_string(&self) -> Option<String> {
        if let Some(s) = self.as_str() {
            return Some(s.to_string());
        }
        if let Some(s) = self.as_i64() {
            return Some(s.to_string());
        }
        if let Some(s) = self.as_f64() {
            return Some(s.to_string());
        }
        None
    }

    fn to_string<T: Display>(&self, msg: T) -> String {
        self.try_to_string().unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'string', but found:\n{:#?}",
                msg,
                self
            )
        })
    }

    fn key_to_bool(&self, key: &str) -> bool {
        self[key].as_bool().unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'boolean', but found:\n{:#?}",
                key,
                self[key]
            )
        })
    }

    fn key_to_hash(&self, key: &str) -> &Hash {
        self[key].as_hash().unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'hash', but found:\n{:#?}",
                key,
                self[key]
            )
        })
    }

    fn key_to_number(&self, key: &str) -> u64 {
        self[key].as_i64().map(|n| n as u64).unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'number', but found:\n{:#?}",
                key,
                self[key]
            )
        })
    }

    fn key_to_string(&self, key: &str) -> String {
        self[key].try_to_string().unwrap_or_else(|| {
            exit!(
                "Cannot parse `{}`, It should be 'string', but found:\n{:#?}",
                key,
                self[key]
            )
        })
    }

    fn key_to_multiple_string(&self, key: &str) -> Vec<String> {
        if self[key].is_badvalue() {
            return vec![];
        }

        let mut result = vec![];
        match self[key].as_vec() {
            Some(items) => {
                for (i, item) in items.iter().enumerate() {
                    let s = item.to_string(format!("{}[{}]", key, i));
                    if !result.contains(&s) {
                        result.push(s);
                    }
                }
            }
            None => {
                for line in self.key_to_string(key).lines() {
                    for s in line.split_whitespace() {
                        if !result.contains(&s.to_string()) {
                            result.push(s.to_string());
                        }
                    }
                }
            }
        }

        result
    }
}
