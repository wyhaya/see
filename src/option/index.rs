use crate::util;
use std::path::PathBuf;
use tokio::fs::File;

#[derive(Debug, Clone)]
pub struct Index(Vec<String>);

impl Index {
    pub fn new(index: Vec<String>) -> Self {
        Self(index)
    }

    pub async fn from_directory(&self, dir: PathBuf) -> Option<(File, String)> {
        for filename in &self.0 {
            let mut path = dir.clone();
            path.push(filename);
            if util::is_file(&path).await {
                if let Ok(file) = File::open(&path).await {
                    if let Some(ext) = util::get_extension(&path) {
                        return Some((file, ext.to_string()));
                    }
                }
            }
        }
        None
    }
}
