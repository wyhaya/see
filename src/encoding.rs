use async_compression::flate2::Compression;
use async_compression::futures::write::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use futures::AsyncWriteExt;
use tokio::io::Result;

pub async fn gzip(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let mut e = GzipEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}

pub async fn deflate(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}

pub async fn br(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let mut e = BrotliEncoder::new(Vec::new(), level);
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}
