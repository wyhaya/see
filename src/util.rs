use std::env;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use tokio::fs;

// Encountered a fatal error
// Print error message and exit the current process
#[macro_export]
macro_rules! exit {
    ($($arg:tt)*) => {
        {
            eprint!("{}", "[ERROR]: ");
            eprintln!($($arg)*);
            std::process::exit(1)
        }
    };
}

pub fn current_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|err| exit!("Can't get working directory\n{:?}", err))
}

pub fn home_dir() -> PathBuf {
    match dirs::home_dir() {
        Some(home) => home,
        None => exit!("Can't get home directory"),
    }
}

// Get the file extension from PathBuf
pub fn get_extension(path: &PathBuf) -> Option<&str> {
    path.extension()
        .map(|ext| ext.to_str())
        .unwrap_or_else(|| Some(""))
}

pub async fn is_file(path: &PathBuf) -> bool {
    fs::metadata(path)
        .await
        .map(|meta| meta.is_file())
        .unwrap_or(false)
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
    if let Ok(port) = text.parse::<u16>() {
        if let Ok(addr) = format!("0.0.0.0:{}", port).parse::<SocketAddr>() {
            return Ok(addr);
        }
    }

    Err(())
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

// Convert path to absolute path
pub fn absolute_path<P: AsRef<Path>, R: AsRef<Path>>(path: P, root: R) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.as_ref().join(path)
    }
}
