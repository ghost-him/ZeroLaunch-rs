use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use zerolaunch_plugin_protocol::ProtocolError;

/// Read the next LSP-style Content-Length framed message from the reader.
/// Returns the raw JSON bytes (without the header), or TransportClosed on EOF.
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, ProtocolError> {
    // Read header lines until the empty line (\r\n\r\n)
    let mut header_buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        let n = reader.read(&mut byte).await?;
        if n == 0 {
            return Err(ProtocolError::TransportClosed);
        }
        header_buf.push(byte[0]);
        let len = header_buf.len();
        if len >= 4
            && header_buf[len - 4] == b'\r'
            && header_buf[len - 3] == b'\n'
            && header_buf[len - 2] == b'\r'
            && header_buf[len - 1] == b'\n'
        {
            break;
        }
        if len > 512 {
            return Err(ProtocolError::InvalidFrame("header too long".into()));
        }
    }

    let header = std::str::from_utf8(&header_buf)
        .map_err(|e| ProtocolError::InvalidFrame(format!("non-utf8 header: {}", e)))?;

    let mut content_length: Option<usize> = None;
    for line in header.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| ProtocolError::InvalidFrame("bad Content-Length".into()))?,
            );
        }
    }

    let len = content_length
        .ok_or_else(|| ProtocolError::InvalidFrame("missing Content-Length".into()))?;

    if len > 16 * 1024 * 1024 {
        return Err(ProtocolError::InvalidFrame(format!(
            "Content-Length too large: {}",
            len
        )));
    }

    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await?;
    Ok(body)
}

/// Write an LSP-style Content-Length framed message to the writer.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> Result<(), ProtocolError> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_write_frame() {
        let payload = br#"{"jsonrpc":"2.0","id":1,"result":"ok"}"#.to_vec();
        let mut buf = Vec::new();
        write_frame(&mut buf, &payload).await.unwrap();

        let expected_header = format!("Content-Length: {}\r\n\r\n", payload.len());
        assert!(buf.starts_with(expected_header.as_bytes()));

        let mut cursor = std::io::Cursor::new(buf);
        let read_back = read_frame(&mut cursor).await.unwrap();
        assert_eq!(read_back, payload);
    }
}
