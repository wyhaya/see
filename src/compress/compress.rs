use crate::*;
use async_compression::futures::write::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use async_compression::Level;
use futures::AsyncWriteExt;
use hyper::header::HeaderValue;
use tokio::io::Result;

#[derive(Copy, Clone, Debug)]
pub enum CompressMode {
    Gzip(CompressLevel),
    Deflate(CompressLevel),
    Br(CompressLevel),
}

impl CompressMode {
    // Response header content-encoding
    pub fn to_header_value(&self) -> HeaderValue {
        let s = match self {
            CompressMode::Gzip(_) => "gzip",
            CompressMode::Deflate(_) => "deflate",
            CompressMode::Br(_) => "br",
        };

        HeaderValue::from_static(s)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum CompressLevel {
    Fastest,
    Best,
    Default,
}

impl CompressLevel {
    pub fn new(level: String) -> Self {
        match level.as_str() {
            "fastest" => CompressLevel::Fastest,
            "default" => CompressLevel::Default,
            "best" => CompressLevel::Best,
            _ => exit!(
                "Wrong compression level `{}`, optional value: `fastest` `default` `best`",
                level
            ),
        }
    }

    fn to_level(self) -> Level {
        match self {
            CompressLevel::Default => Level::Default,
            CompressLevel::Best => Level::Best,
            CompressLevel::Fastest => Level::Fastest,
        }
    }
}

pub async fn compress_data(buf: &[u8], mode: CompressMode) -> Result<Vec<u8>> {
    match mode {
        CompressMode::Gzip(level) => gzip(buf, level.to_level()).await,
        CompressMode::Deflate(level) => deflate(buf, level.to_level()).await,
        CompressMode::Br(level) => br(buf, level.to_level()).await,
    }
}

async fn gzip(data: &[u8], level: Level) -> Result<Vec<u8>> {
    let mut e = GzipEncoder::with_quality(Vec::new(), level);
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}

async fn deflate(data: &[u8], level: Level) -> Result<Vec<u8>> {
    let mut e = DeflateEncoder::with_quality(Vec::new(), level);
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}

async fn br(data: &[u8], level: Level) -> Result<Vec<u8>> {
    let mut e = BrotliEncoder::with_quality(Vec::new(), level);
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {}
}
