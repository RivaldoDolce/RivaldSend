use std::path::Path;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use crate::error::CoreError;

pub async fn write_chunk(path: &Path, offset: u64, data: &[u8]) -> Result<(), CoreError> {
    let mut file = tokio::fs::OpenOptions::new().create(true).write(true).truncate(false).open(path).await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    file.write_all(data).await?;
    file.sync_data().await?;
    Ok(())
}

pub async fn preallocate(path: &Path, size: u64) -> Result<(), CoreError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
        let available = {
            #[cfg(unix)]
            {
                let stat = nix::sys::statvfs::statvfs(parent).map_err(|e| CoreError::IoString(e.to_string()))?;
                stat.blocks_available() as u64 * stat.fragment_size() as u64
            }
            #[cfg(not(unix))]
            {
                u64::MAX
            }
        };
        if size > available {
            return Err(CoreError::IoString(format!("espace disque insuffisant: requis {size}, disponible {available}")));
        }
    }
    let file = tokio::fs::OpenOptions::new().create(true).write(true).truncate(false).open(path).await?;
    file.set_len(size).await?;
    file.sync_all().await?;
    Ok(())
}

pub async fn atomic_write(path: &Path, data: &[u8]) -> Result<(), std::io::Error> {
    let parent = path.parent().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "chemin sans parent"))?;
    tokio::fs::create_dir_all(parent).await?;
    let tmp = parent.join(format!(".rivaldsend-tmp-{}", uuid::Uuid::new_v4()));
    {
        let mut f = tokio::fs::File::create(&tmp).await?;
        f.write_all(data).await?;
        f.sync_all().await?;
    }
    tokio::fs::rename(&tmp, path).await?;
    #[cfg(unix)]
    {
        if let Ok(f) = std::fs::File::open(parent) {
            let _ = f.sync_all();
        }
    }
    Ok(())
}
