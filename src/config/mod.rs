pub mod default;
pub mod mime;
mod parser;
pub mod tls;
pub mod transform;

pub use parser::*;
mod setting;
pub use setting::*;

use std::path::{Path, PathBuf};

// Convert path to absolute path
pub trait AbsolutePath {
    fn absolute_path<P: AsRef<Path>>(&self, root: P) -> PathBuf;
}

impl AbsolutePath for String {
    fn absolute_path<P: AsRef<Path>>(&self, root: P) -> PathBuf {
        let path = PathBuf::from(self);
        if path.is_absolute() {
            path
        } else {
            root.as_ref().join(self)
        }
    }
}
