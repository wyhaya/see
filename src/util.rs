use std::env;
use std::path::PathBuf;
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
