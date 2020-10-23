use super::{Block, Directive};
use crate::exit;

pub trait BlockExt {
    // Check within the current block
    // If the check fails, the process will exit
    fn check(&self, allow: &[&str], required: &[&str], repeat: &[&str]);
}

impl BlockExt for Block {
    fn check(&self, allow: &[&str], required: &[&str], repeat: &[&str]) {
        // Allowed values
        for directive in self.directives() {
            if !allow.contains(&directive.name()) {
                exit!(
                    "[line:{}] Unknown directive `{}`",
                    directive.line(),
                    directive.name()
                )
            }
        }

        // Required values
        for name in required {
            if self.get(name).is_none() {
                exit!("[line:{}] Missing directive `{}`", self.line(), name)
            }
        }

        // Repeated values
        for directive in self.directives() {
            if !repeat.contains(&directive.name()) {
                let all = self.get_all_by_name(directive.name());
                if all.len() > 1 {
                    let d = all[all.len() - 1];
                    exit!("[line:{}] Repeated directive `{}`", d.line(), d.name())
                }
            }
        }
    }
}

pub trait DirectiveExt {
    fn to_str(&self) -> &str;
    fn to_source_str(&self) -> &str;
    fn to_multiple_str(&self) -> Vec<&str>;
    fn to_bool(&self) -> bool;
    fn to_block(&self) -> &Block;
    fn to_value_block(&self) -> (&str, &Block);
}

impl DirectiveExt for Directive {
    fn to_str(&self) -> &str {
        if let Some(val) = self.as_source_str() {
            if val.split_whitespace().count() == 1 {
                return val;
            }
        }
        exit!(
            "[line:{}] Directive `{}` does not allow multiple values",
            self.line(),
            self.name()
        )
    }

    fn to_source_str(&self) -> &str {
        self.as_source_str().unwrap_or_else(|| {
            exit!(
                "[line:{}] Cannot convert `{}` to 'string'",
                self.line(),
                self.name()
            )
        })
    }

    // todo
    // allow block
    fn to_multiple_str(&self) -> Vec<&str> {
        self.to_source_str()
            .split_whitespace()
            .collect::<Vec<&str>>()
    }

    fn to_bool(&self) -> bool {
        if let Some(val) = self.as_bool() {
            return val;
        }
        exit!(
            "[line:{}] Cannot convert `{}` to 'boolean'",
            self.line(),
            self.name()
        )
    }

    fn to_block(&self) -> &Block {
        if let Some(val) = self.as_block() {
            return val;
        }
        exit!(
            "[line:{}] Cannot convert `{}` to 'block'",
            self.line(),
            self.name()
        )
    }

    fn to_value_block(&self) -> (&str, &Block) {
        if let Some(val) = self.as_value_block() {
            return val;
        }
        exit!(
            "[line:{}] Cannot convert `{}` to 'value block'",
            self.line(),
            self.name()
        )
    }
}
