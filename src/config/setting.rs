#[macro_export]
macro_rules! setting_value {
    ($yaml: expr) => {
        setting_none!($yaml);
        setting_off!($yaml);
    };
}

#[macro_export]
macro_rules! setting_none {
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
macro_rules! setting_off {
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
        match self {
            Setting::Value(_) => true,
            _ => false,
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            Setting::None => true,
            _ => false,
        }
    }

    pub fn is_off(&self) -> bool {
        match self {
            Setting::Off => true,
            _ => false,
        }
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
