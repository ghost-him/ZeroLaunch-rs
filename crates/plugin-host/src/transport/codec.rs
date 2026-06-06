use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use zerolaunch_plugin_protocol::ProtocolError;

/// Read the next LSP-style Content-Length framed message from the reader.
/// Returns the raw JSON bytes (without the header), or TransportClosed on EOF.
///
/// Requires an already-buffered reader (e.g. `BufReader<ChildStdout>`).
pub async fn read_frame<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, ProtocolError> {
    // Read header lines one at a time until the empty separator line.
    let mut content_length: Option<usize> = None;
    let mut total_header_len = 0usize;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(ProtocolError::TransportClosed);
        }
        total_header_len += n;
        if total_header_len > 512 {
            return Err(ProtocolError::InvalidFrame("header too long".into()));
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            // End of headers — the empty line (\r\n) terminates the header block
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
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

        let mut reader = tokio::io::BufReader::new(std::io::Cursor::new(buf));
        let read_back = read_frame(&mut reader).await.unwrap();
        assert_eq!(read_back, payload);
    }
}
