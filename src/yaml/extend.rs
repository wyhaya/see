use crate::exit;
use yaml_rust::yaml::Hash;
use yaml_rust::Yaml;

pub trait YamlExtend {
    fn check(&self, name: &str, keys: &[&str], must: &[&str]);
    fn to_bool(&self) -> bool;
    fn to_hash(&self) -> &Hash;
    fn try_to_string(&self) -> Option<String>;
    fn to_string(&self) -> String;
    fn to_multiple_string(&self) -> Vec<String>;
}

impl YamlExtend for Yaml {
    fn check(&self, name: &str, keys: &[&str], must: &[&str]) {
        let hash = self[name].to_hash();

        // Disallowed key
        for (key, _) in hash {
            let key = key.to_string();
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

    fn to_bool(&self) -> bool {
        self.as_bool()
            .unwrap_or_else(|| exit!("Cannot parse `{:?}` to 'boolean'", self))
    }

    fn to_hash(&self) -> &Hash {
        self.as_hash()
            .unwrap_or_else(|| exit!("Cannot parse `{:?}` to 'hash'", self))
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

    fn to_string(&self) -> String {
        self.try_to_string()
            .unwrap_or_else(|| exit!("Cannot parse `{:?}` to 'string'", self))
    }

    fn to_multiple_string(&self) -> Vec<String> {
        if self.is_badvalue() {
            return vec![];
        }

        let mut result = vec![];
        match self.as_vec() {
            Some(items) => {
                for item in items {
                    let s = item.to_string();
                    if !result.contains(&s) {
                        result.push(s);
                    }
                }
            }
            None => {
                for line in self.to_string().lines() {
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
