use std::path::Path;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::*;
use tokio::io::{self, Result};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Logger {
    file: Option<Arc<Mutex<File>>>,
    stdout: Option<Arc<Mutex<Stdout>>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            file: None,
            stdout: None,
        }
    }

    pub async fn file<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map(|file| Arc::new(Mutex::new(file)))?;
        self.file = Some(file);
        Ok(self)
    }

    pub fn stdout(mut self) -> Self {
        self.stdout = Some(Arc::new(Mutex::new(io::stdout())));
        self
    }

    pub async fn write<S: AsRef<str>>(&mut self, text: S) {
        if let Some(file) = self.file.clone() {
            let mut file = (&*file).lock().await;
            let text = format!("{}\n", text.as_ref());
            let _ = file.write(text.as_bytes()).await;
        }

        if let Some(stdout) = self.stdout.clone() {
            let mut stdout = (&*stdout).lock().await;
            let text = format!("{}\n", text.as_ref());
            let _ = stdout.write(text.as_bytes()).await;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn log() {}
}
