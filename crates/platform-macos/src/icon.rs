use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use zerolaunch_plugin_api::common::image_utils::ImageUtils;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::icon::IconExtractor;

/// Extracts and converts macOS application icons to PNG bytes.
///
/// Used by the icon cache and candidate presentation services.
pub struct MacosIconExtractor {
    /// Fallback icon path for applications without an embedded icon.
    default_app_icon_path: String,
    /// Fallback icon path for web and non-application targets.
    default_web_icon_path: String,
}
impl MacosIconExtractor {
    pub fn new(default_app_icon_path: String, default_web_icon_path: String) -> Self {
        Self {
            default_app_icon_path,
            default_web_icon_path,
        }
    }
}

fn source_icon(path: &Path) -> Option<PathBuf> {
    if path.extension().is_some_and(|e| e == "app") {
        return fs::read_dir(path.join("Contents/Resources"))
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|e| e == "icns"));
    }
    path.is_file().then(|| path.to_path_buf())
}

fn convert_to_png(path: &Path) -> Result<Vec<u8>, HostApiError> {
    if path
        .extension()
        .is_some_and(|e| matches!(e.to_str(), Some("png") | Some("PNG")))
    {
        return fs::read(path).map_err(|e| HostApiError::IconExtractionFailed {
            request: path.display().to_string(),
            reason: e.to_string(),
        });
    }
    let output = std::env::temp_dir().join(format!(
        "zerolaunch-icon-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = Command::new("sips")
        .args(["-s", "format", "png"])
        .arg(path)
        .args(["--out", output.to_string_lossy().as_ref()])
        .output();
    let bytes = result
        .ok()
        .filter(|r| r.status.success())
        .and_then(|_| fs::read(&output).ok())
        .ok_or_else(|| HostApiError::IconExtractionFailed {
            request: path.display().to_string(),
            reason: "sips could not convert icon to PNG".into(),
        });
    let _ = fs::remove_file(output);
    bytes
}

#[async_trait]
impl IconExtractor for MacosIconExtractor {
    async fn extract_from_path(&self, path: &str) -> Result<Vec<u8>, HostApiError> {
        source_icon(Path::new(path))
            .ok_or_else(|| HostApiError::IconExtractionFailed {
                request: path.into(),
                reason: "no macOS icon was found".into(),
            })
            .and_then(|p| convert_to_png(&p))
    }
    async fn extract_from_url(&self, url: &str) -> Result<Vec<u8>, HostApiError> {
        if !self.is_network_available() {
            return Err(HostApiError::IconExtractionFailed {
                request: url.into(),
                reason: "no network connection available".into(),
            });
        }
        let parsed = url::Url::parse(url).map_err(|e| HostApiError::IconExtractionFailed {
            request: url.into(),
            reason: e.to_string(),
        })?;
        let favicon = format!(
            "{}://{}/favicon.ico",
            parsed.scheme(),
            parsed
                .host_str()
                .ok_or_else(|| HostApiError::IconExtractionFailed {
                    request: url.into(),
                    reason: "URL has no host".into()
                })?
        );
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) ",
                "AppleWebKit/537.36 (KHTML, like Gecko) ",
                "Chrome/120.0.0.0 Safari/537.36"
            ))
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| HostApiError::IconExtractionFailed {
                request: url.into(),
                reason: format!("failed to build reqwest client: {e}"),
            })?;
        let response =
            client
                .get(&favicon)
                .send()
                .await
                .map_err(|e| HostApiError::IconExtractionFailed {
                    request: url.into(),
                    reason: e.to_string(),
                })?;
        let bytes = response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
            HostApiError::IconExtractionFailed {
                request: url.into(),
                reason: e.to_string(),
            }
        })?;
        // The favicon may be ICO while the IconExtractor contract requires
        // PNG; convert so downstream WebP encoding can decode it.
        ImageUtils::convert_image_to_png(bytes).await.map_err(|e| {
            HostApiError::IconExtractionFailed {
                request: url.into(),
                reason: format!("favicon is not a decodable image: {e}"),
            }
        })
    }
    async fn extract_from_extension(&self, _ext: &str) -> Result<Vec<u8>, HostApiError> {
        fs::read(&self.default_app_icon_path).map_err(|e| HostApiError::IconExtractionFailed {
            request: "extension".into(),
            reason: e.to_string(),
        })
    }
    fn default_app_icon_path(&self) -> &str {
        &self.default_app_icon_path
    }
    fn default_web_icon_path(&self) -> &str {
        &self.default_web_icon_path
    }
    fn is_network_available(&self) -> bool {
        // No pure-CrossPlatform reachability check on macOS; a missing
        // network surfaces as the 10s request timeout instead of a fast
        // failure. Acceptable baseline for URL icon extraction.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_bundle_icon_path() {
        assert!(source_icon(Path::new("/missing.app")).is_none());
    }
}
