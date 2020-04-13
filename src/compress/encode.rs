use crate::compress::encoding::Encoding;
use async_compression::flate2::Compression;
use async_compression::futures::write::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use futures::AsyncWriteExt;
use tokio::io::Result;

pub async fn auto(buf: &[u8], encoding: Encoding) -> Result<Vec<u8>> {
    match encoding {
        Encoding::Gzip(level) => gzip(buf, level).await,
        Encoding::Deflate(level) => deflate(buf, level).await,
        Encoding::Br(level) => br(buf, level).await,
        _ => Ok(buf.to_vec()),
    }
}

async fn gzip(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let mut e = GzipEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}

async fn deflate(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}

async fn br(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let mut e = BrotliEncoder::new(Vec::new(), level);
    e.write_all(&data).await?;
    e.flush().await?;

    Ok(e.into_inner())
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {}
}
