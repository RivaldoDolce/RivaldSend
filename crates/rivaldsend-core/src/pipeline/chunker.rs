use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
pub const SINGLE_SHOT_THRESHOLD: usize = 16 * 1024 * 1024;
pub const CHUNK_SIZE: usize = 4 * 1024 * 1024;
pub async fn read_chunk(file: &mut tokio::fs::File, offset: u64, buf: &mut BytesMut) -> Result<Option<bytes::Bytes>, std::io::Error> {
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    buf.clear();
    buf.resize(CHUNK_SIZE, 0);
    let mut read = 0usize;
    while read < CHUNK_SIZE {
        let n = file.read(&mut buf[read..]).await?;
        if n == 0 {
            break;
        }
        read += n;
    }
    if read == 0 {
        return Ok(None);
    }
    buf.truncate(read);
    Ok(Some(buf.split_to(read).freeze()))
}
