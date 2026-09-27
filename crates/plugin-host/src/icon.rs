//! 插件图标载荷：图标字节 → data URL 的唯一实现。
//!
//! 宿主有两处消费插件图标：已安装插件（`manager::read_plugin_icon` 读插件目录
//! 内的图标文件）与插件市场（`plugin_market` 读 release 附带的图标资产字节，
//! 见 `src-tauri/src/plugin_market/mod.rs`）。MIME 推断与大小上限在此单点定义，
//! 避免两处各写一份映射后漂移。

use base64::Engine;
use std::path::Path;
use tracing::warn;

/// 图标载荷大小上限（1MB）：避免超大图标膨胀 IPC 载荷（plugin_list / 市场卡片）。
pub const MAX_ICON_BYTES: u64 = 1024 * 1024;

/// 根据图标文件扩展名推断 MIME 类型，未知扩展名回退 `image/png`。
pub fn mime_from_extension(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/png",
    }
}

/// 图标字节 → data URL；超过 [`MAX_ICON_BYTES`] 返回 None（只告警，不阻断调用方）。
pub fn to_data_url(mime: &str, bytes: &[u8]) -> Option<String> {
    if bytes.len() as u64 > MAX_ICON_BYTES {
        warn!("图标超过 {} 字节上限，忽略", MAX_ICON_BYTES);
        return None;
    }
    Some(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_from_extension_covers_known_and_unknown() {
        assert_eq!(
            mime_from_extension(Path::new("a/b/icon.svg")),
            "image/svg+xml"
        );
        assert_eq!(mime_from_extension(Path::new("icon.ico")), "image/x-icon");
        assert_eq!(mime_from_extension(Path::new("icon.jpeg")), "image/jpeg");
        assert_eq!(mime_from_extension(Path::new("icon.webp")), "image/webp");
        assert_eq!(mime_from_extension(Path::new("icon.gif")), "image/gif");
        // 未知扩展名与无扩展名都回退 png
        assert_eq!(mime_from_extension(Path::new("icon.bmp")), "image/png");
        assert_eq!(mime_from_extension(Path::new("icon")), "image/png");
    }

    #[test]
    fn to_data_url_encodes_bytes_and_rejects_oversize() {
        assert_eq!(
            to_data_url("image/svg+xml", b"<svg/>").as_deref(),
            Some("data:image/svg+xml;base64,PHN2Zy8+")
        );
        // 超限返回 None（不产出载荷）
        let oversize = vec![0u8; MAX_ICON_BYTES as usize + 1];
        assert!(to_data_url("image/png", &oversize).is_none());
    }
}
