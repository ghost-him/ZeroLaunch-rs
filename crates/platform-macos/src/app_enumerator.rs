use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use zerolaunch_plugin_api::services::app::{AppEnumerator, AppInfo};
use zerolaunch_plugin_api::services::IconRequest;

/// Enumerates installed macOS application bundles for the candidate pipeline.
///
/// Used by the platform injection layer when collecting application candidates.
pub struct MacosAppEnumerator;

impl MacosAppEnumerator {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosAppEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

fn plist_value(bundle: &Path, key: &str) -> Option<String> {
    let info = bundle.join("Contents/Info.plist");
    let output = Command::new("plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(info)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn icon_path(bundle: &Path) -> String {
    let resources = bundle.join("Contents/Resources");
    let configured = plist_value(bundle, "CFBundleIconFile").unwrap_or_default();
    let configured = configured.strip_suffix(".icns").unwrap_or(&configured);
    let preferred = resources.join(format!("{configured}.icns"));
    if !configured.is_empty() && preferred.is_file() {
        return preferred.to_string_lossy().into_owned();
    }
    fs::read_dir(resources)
        .ok()
        .and_then(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|e| e == "icns"))
        })
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(crate) fn app_info_from_bundle(bundle: &Path) -> Option<AppInfo> {
    if !bundle.is_dir() || bundle.extension().is_none_or(|e| e != "app") {
        return None;
    }
    let display_name = plist_value(bundle, "CFBundleDisplayName")
        .or_else(|| plist_value(bundle, "CFBundleName"))
        .unwrap_or_else(|| {
            bundle
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
    let app_id = plist_value(bundle, "CFBundleIdentifier")
        .unwrap_or_else(|| bundle.to_string_lossy().into_owned());
    Some(AppInfo {
        app_id,
        display_name,
        icon: IconRequest::Path(icon_path(bundle)),
        install_path: Some(bundle.to_string_lossy().into_owned()),
    })
}

fn application_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Applications"));
    }
    roots
}

#[async_trait]
impl AppEnumerator for MacosAppEnumerator {
    async fn enumerate_apps(&self) -> Vec<AppInfo> {
        application_roots()
            .into_iter()
            .flat_map(|root| fs::read_dir(root).into_iter().flatten().flatten())
            .filter_map(|entry| app_info_from_bundle(&entry.path()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_application_bundles() {
        assert!(app_info_from_bundle(Path::new("/tmp/nope.txt")).is_none());
    }

    #[test]
    fn application_roots_include_system_applications() {
        assert_eq!(application_roots()[0], PathBuf::from("/Applications"));
    }
}
