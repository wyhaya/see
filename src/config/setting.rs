#[macro_export]
macro_rules! check_value {
    ($yaml: expr) => {
        check_none!($yaml);
        check_off!($yaml);
    };
}

#[macro_export]
macro_rules! check_none {
    ($yaml: expr) => {
        if $yaml.is_badvalue() || $yaml.is_null() {
            return Setting::None;
        }
    };
    ($yaml: expr, $default: expr) => {
        if $yaml.is_badvalue() || $yaml.is_null() {
            return Setting::Value($default);
        }
    };
}

#[macro_export]
macro_rules! check_off {
    ($yaml: expr) => {
        if let Some(val) = $yaml.as_bool() {
            if !val {
                return Setting::Off;
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
