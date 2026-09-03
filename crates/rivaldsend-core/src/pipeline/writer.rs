use std::path::Path;
use tokio::io::AsyncWriteExt;
pub async fn write_chunk(path: &Path, offset: u64, data: &[u8]) -> Result<(), std::io::Error> {
    let mut file = tokio::fs::OpenOptions::new().create(true).write(true).open(path).await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    file.write_all(data).await?;
    file.sync_data().await?;
    Ok(())
}
pub async fn atomic_write(path: &Path, data: &[u8]) -> Result<(), std::io::Error> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = tokio::fs::File::create(&tmp).await?;
        f.write_all(data).await?;
        f.sync_all().await?;
    }
    tokio::fs::rename(&tmp, path).await?;
    #[cfg(unix)]
    {
        if let Some(parent) = path.parent() {
            if let Ok(f) = std::fs::File::open(parent) {
                let _ = f.sync_all();
            }
        }
    }
    Ok(())
}
use tokio::io::AsyncSeekExt;
