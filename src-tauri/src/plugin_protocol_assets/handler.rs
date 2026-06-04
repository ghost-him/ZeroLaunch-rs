//! Custom URI scheme handler for `zlplugin://` protocol.
//!
//! Serves third-party plugin UI assets (ES modules, CSS, images) from the
//! plugin installation directory with path traversal protection.

use regex::Regex;
use std::path::PathBuf;
use std::sync::OnceLock;

static PLUGINS_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Set the plugins directory root. Called once during startup.
pub fn set_plugins_dir(dir: PathBuf) {
    let _ = PLUGINS_DIR.set(dir);
}

/// Validate a plugin ID string against the reverse-domain pattern.
fn is_valid_plugin_id(id: &str) -> bool {
    let re = Regex::new(r"^[a-z][a-z0-9]*(\.[a-z][a-z0-9_-]*)+$").unwrap();
    re.is_match(id)
}

/// Determine MIME type from file extension.
fn mime_from_extension(path: &std::path::Path) -> &'static str {
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

/// Handle a `zlplugin://` request.
///
/// URI format: `zlplugin://<plugin-id>/ui/<sub-path>`
pub fn handle(uri: &str) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    // Parse the URI
    let uri = uri
        .strip_prefix("zlplugin://")
        .ok_or("not a zlplugin URI")?;
    let (host, path) = uri.split_once('/').unwrap_or((uri, ""));

    // Validate plugin ID
    if host.is_empty() || !is_valid_plugin_id(host) {
        return Err("invalid plugin id".into());
    }

    // Validate sub-path starts with ui/
    if !path.starts_with("ui/") {
        return Err("access denied: only ui/ path allowed".into());
    }

    let plugins_dir = PLUGINS_DIR.get().ok_or("plugins dir not initialized")?;
    let plugin_dir = plugins_dir.join(host);
    let asset_path = plugin_dir.join(path);

    // Canonicalize to prevent path traversal
    let canonical = asset_path.canonicalize()?;
    let plugin_canonical = plugin_dir.canonicalize()?;
    if !canonical.starts_with(&plugin_canonical) {
        return Err("access denied: path traversal detected".into());
    }

    let bytes = std::fs::read(&canonical)?;
    let mime = mime_from_extension(&canonical).to_string();

    Ok((bytes, mime))
}
