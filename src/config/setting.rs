#[macro_export]
macro_rules! check_value {
    ($block: expr, $key: expr) => {
        check_none!($block, $key);
        check_off!($block, $key);
    };
}

#[macro_export]
macro_rules! check_none {
    ($block: expr, $key: expr) => {
        match $block.get($key) {
            Some(d) => {
                if d.is_block() {
                    if d.to_block().directives().is_empty() {
                        return Setting::None;
                    }
                }
            }
            None => {
                return Setting::None;
            }
        }
    };
    ($block: expr, $key: expr, $default: expr) => {
        match $block.get($key) {
            Some(d) => {
                if d.is_block() {
                    if d.to_block().directives().is_empty() {
                        return Setting::Value($default);
                    }
                }
            }
            None => {
                return Setting::Value($default);
            }
        }
    };
}

#[macro_export]
macro_rules! check_off {
    ($block: expr, $key: expr) => {
        if let Some(val) = $block.get($key) {
            if let Some(b) = val.try_to_bool() {
                if !b {
                    return Setting::Off;
                }
            }
        }
    };
}

#[derive(Debug, Clone)]
pub enum Setting<T> {
    None,
    Off,
    Value(T),
}

impl<T> Setting<T> {
    pub fn is_value(&self) -> bool {
        matches!(self, Setting::Value(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Setting::None)
    }

    pub fn is_off(&self) -> bool {
        matches!(self, Setting::Off)
    }

    pub fn into_value(self) -> T {
        match self {
            Setting::Value(val) => val,
            _ => panic!("into_value"),
        }
    }
}

impl<T> Default for Setting<T> {
    fn default() -> Self {
        Setting::None
    }
}

impl<T: Default> Setting<T> {
    pub fn unwrap_or_default(self) -> T {
        match self {
            Setting::Value(x) => x,
            _ => Default::default(),
        }
    }
}
