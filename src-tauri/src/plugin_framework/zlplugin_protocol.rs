//! zlplugin:// 协议处理器。
//!
//! 从 PluginManager 中提取的自定义 URI 协议处理职责域，
//! 处理 `zlplugin://<plugin-id>/ui/<sub-path>` 格式的请求。

use std::path::{Path, PathBuf};

use regex::Regex;

/// zlplugin:// 协议处理器。
pub(crate) struct ZlpluginProtocolHandler {
    plugins_dir: PathBuf,
}

impl ZlpluginProtocolHandler {
    /// 创建处理器，指定插件根目录。
    pub(crate) fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    /// 处理 `zlplugin://` 协议请求，返回 (文件字节, MIME 类型)。
    ///
    /// URI 格式：`zlplugin://<plugin-id>/ui/<sub-path>`
    pub(crate) fn handle(
        &self,
        uri: &str,
    ) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
        let uri = uri
            .strip_prefix("zlplugin://")
            .ok_or("not a zlplugin URI")?;
        let (host, path) = uri.split_once('/').unwrap_or((uri, ""));

        if host.is_empty() || !is_valid_plugin_id(host) {
            return Err("invalid plugin id".into());
        }

        if !path.starts_with("ui/") {
            return Err("access denied: only ui/ path allowed".into());
        }

        let plugin_dir = self.plugins_dir.join(host);
        let asset_path = plugin_dir.join(path);

        // Canonicalize 防路径遍历
        let canonical = asset_path.canonicalize()?;
        let plugin_canonical = plugin_dir.canonicalize()?;
        if !canonical.starts_with(&plugin_canonical) {
            return Err("access denied: path traversal detected".into());
        }

        let bytes = std::fs::read(&canonical)?;
        let mime = mime_from_extension(&canonical).to_string();

        Ok((bytes, mime))
    }
}

// ── 私有辅助函数 ─────────────────────────────────────────────────

/// 校验插件 ID 是否符合反向域名格式。
fn is_valid_plugin_id(id: &str) -> bool {
    use std::sync::LazyLock;
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(zerolaunch_plugin_protocol::manifest::PLUGIN_ID_RE).unwrap());
    RE.is_match(id)
}

/// 根据文件扩展名确定 MIME 类型。
fn mime_from_extension(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mjs") | Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("html") => "text/html",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") | Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
