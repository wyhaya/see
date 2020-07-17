use crate::{default, exit};
use rand::prelude::Rng;
use rand::thread_rng;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

pub fn current_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|err| exit!("Can't get working directory\n{:?}", err))
}

pub fn home_dir() -> PathBuf {
    match dirs::home_dir() {
        Some(home) => home,
        None => exit!("Can't get home directory"),
    }
}

pub fn config_dir() -> PathBuf {
    home_dir().join(default::CONFIG_PATH[0])
}

pub fn config_path() -> PathBuf {
    home_dir()
        .join(default::CONFIG_PATH[0])
        .join(default::CONFIG_PATH[1])
}

pub fn pid_path() -> PathBuf {
    home_dir()
        .join(default::PID_PATH[0])
        .join(default::PID_PATH[1])
}

pub fn dedup<T: Eq>(vec: Vec<T>) -> Vec<T> {
    let mut new = vec![];
    for item in vec {
        if !new.contains(&item) {
            new.push(item);
        }
    }
    new
}

pub fn get_rand_item<'a, T>(vec: &'a [T]) -> &'a T {
    if vec.len() == 1 {
        &vec[0]
    } else {
        let i = thread_rng().gen_range(0, vec.len());
        &vec[i]
    }
}

#[derive(Debug)]
pub enum DigitalUnitError {
    NoNumber,
    NoUnit,
    ErrorNumber,
    ErrorUnit,
    Zero,
}

impl DigitalUnitError {
    pub fn description(&self) -> &str {
        match self {
            DigitalUnitError::NoNumber => "No number found",
            DigitalUnitError::NoUnit => "No number found",
            DigitalUnitError::ErrorNumber => "Wrong number",
            DigitalUnitError::ErrorUnit => "Wrong unit",
            DigitalUnitError::Zero => "Value cannot be 0",
        }
    }
}

// Parse time format into Duration
// format: 1d 1.2h 5s ...
pub fn try_parse_duration(text: &str) -> Result<Duration, DigitalUnitError> {
    let numbers = "0123456789.".chars().collect::<Vec<char>>();
    let i = text
        .chars()
        .position(|ch| !numbers.contains(&ch))
        .ok_or_else(|| DigitalUnitError::NoUnit)?;

    let time = &text[..i];
    let unit = &text[i..];

    if time.is_empty() {
        return Err(DigitalUnitError::NoNumber);
    }
    let n = time
        .parse::<f64>()
        .map_err(|_| DigitalUnitError::ErrorNumber)?;
    let ms = match unit {
        "d" => Ok(24_f64 * 60_f64 * 60_f64 * 1000_f64 * n),
        "h" => Ok(60_f64 * 60_f64 * 1000_f64 * n),
        "m" => Ok(60_f64 * 1000_f64 * n),
        "s" => Ok(1000_f64 * n),
        "ms" => Ok(n),
        _ => Err(DigitalUnitError::ErrorUnit),
    }? as u64;

    if ms == 0 {
        Err(DigitalUnitError::Zero)
    } else {
        Ok(Duration::from_millis(ms))
    }
}

// Parse size format to bytes
// format: 1m 1.2k 10b
pub fn try_parse_size(text: &str) -> Result<usize, DigitalUnitError> {
    let numbers = "0123456789.".chars().collect::<Vec<char>>();
    let i = text
        .chars()
        .position(|ch| !numbers.contains(&ch))
        .ok_or_else(|| DigitalUnitError::NoUnit)?;

    let num = &text[..i];
    let unit = &text[i..];

    if num.is_empty() {
        return Err(DigitalUnitError::NoNumber);
    }
    let n = num
        .parse::<f64>()
        .map_err(|_| DigitalUnitError::ErrorNumber)?;
    let size = match unit {
        "g" => Ok(n * 1024_f64 * 1024_f64 * 1024_f64),
        "m" => Ok(n * 1024_f64 * 1024_f64),
        "k" => Ok(n * 1024_f64),
        "b" => Ok(n),
        _ => Err(DigitalUnitError::ErrorUnit),
    }? as usize;

    if size == 0 {
        Err(DigitalUnitError::Zero)
    } else {
        Ok(size)
    }
}

pub fn try_to_socket_addr(text: &str) -> Result<SocketAddr, ()> {
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Ok(addr);
    }
    if let Ok(port) = text.parse::<i64>() {
        if let Ok(addr) = format!("0.0.0.0:{}", port).parse::<SocketAddr>() {
            return Ok(addr);
        }
    }

    Err(())
}

pub fn bytes_to_size(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    if bytes < 1024 {
        return format!("{} B", bytes);
    }
    let bytes = bytes as f64;
    let i = (bytes.ln() / 1024_f64.ln()) as i32;
    format!("{:.2} {}", bytes / 1024_f64.powi(i), UNITS[i as usize])
}

#[test]
fn test_dedup() {
    assert_eq!(dedup(vec![1, 1]), vec![1]);
    assert_eq!(dedup(vec![1, 2, 3]), vec![1, 2, 3]);
}

#[test]
fn test_try_parse_size() {
    assert_eq!(try_parse_size("1b").unwrap(), 1);
    assert_eq!(try_parse_size("1k").unwrap(), 1024);
    assert_eq!(try_parse_size("1.5m").unwrap(), 1572864);
}

#[test]
fn test_try_to_socket_addr() {
    assert_eq!(
        try_to_socket_addr("80").unwrap(),
        "0.0.0.0:80".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(
        try_to_socket_addr("0.0.0.0:80").unwrap(),
        "0.0.0.0:80".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(try_to_socket_addr("err"), Err(()));
}

#[test]
fn test_bytes_to_size() {
    assert_eq!(bytes_to_size(0), "0 B");
    assert_eq!(bytes_to_size(1), "1 B");
    assert_eq!(bytes_to_size(1023), "1023 B");
    assert_eq!(bytes_to_size(1024), "1.00 KB");
    assert_eq!(bytes_to_size(1 * 1024 * 1024), "1.00 MB");
    assert_eq!(bytes_to_size(1 * 1024 * 1024 * 1024 * 1024), "1.00 TB");
    assert_eq!(bytes_to_size(u64::max_value()), "16.00 EB");
}
