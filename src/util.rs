use crate::{default, exit};
use rand::prelude::Rng;
use rand::thread_rng;
use std::env;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

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

pub fn try_to_socket_addr(text: &str) -> Result<SocketAddr, ()> {
    // 0.0.0.0:80
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Ok(addr);
    }
    // 0.0.0.0
    if let Ok(ip) = text.parse::<Ipv4Addr>() {
        if let Ok(addr) = format!("{}:80", ip).parse::<SocketAddr>() {
            return Ok(addr);
        }
    }
    // 80
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

// Get the file extension from PathBuf
pub fn get_extension(p: &PathBuf) -> Option<&str> {
    p.extension()
        .map(|ext| ext.to_str())
        .unwrap_or_else(|| Some(""))
}

#[test]
fn test_dedup() {
    assert_eq!(dedup(vec![1, 1]), vec![1]);
    assert_eq!(dedup(vec![1, 2, 3]), vec![1, 2, 3]);
}

#[test]
fn test_try_to_socket_addr() {
    assert_eq!(
        try_to_socket_addr("80").unwrap(),
        "0.0.0.0:80".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(
        try_to_socket_addr("0.0.0.0").unwrap(),
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
