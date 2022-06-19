use anyhow::Result;
use std::path::Path;
use futures::AsyncReadExt;

pub async fn async_get_content(path: &Path) -> Result<String> {
    Ok(async_fs::read_to_string(path).await?)
}

pub async fn async_get_buffer(path: &Path) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut file = async_fs::File::open(path).await?;
    file.read_to_end(&mut data).await?;
    return Ok(data)
}
