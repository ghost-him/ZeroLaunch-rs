//! LSP Content-Length 帧的编码工具。
//!
//! 本模块提供纯同步函数，不依赖 tokio。异步 I/O 由各 crate 自行处理。

/// 单帧的最大字节数（16 MB）。
/// 超过此大小的帧视为无效，防止内存溢出。
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// LSP 帧头部的最大字节数限制（512 字节）。
/// 防止恶意或损坏的发送方发送无限长的头部行。
pub const MAX_HEADER_SIZE: usize = 512;

/// 将 payload 编码为完整的 LSP Content-Length 帧格式。
///
/// 返回 `Content-Length: N\r\n\r\n{payload}` 格式的字节序列，
/// 可直接写入任意 `AsyncWrite` 或 `Write`。
///
pub fn encode_frame(payload: &[u8]) -> Vec<u8> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    let mut frame = Vec::with_capacity(header.len() + payload.len());
    frame.extend_from_slice(header.as_bytes());
    frame.extend_from_slice(payload);
    frame
}
