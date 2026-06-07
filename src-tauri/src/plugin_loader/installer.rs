//! Local plugin installer — handles .zip extraction and manifest validation.

use std::path::{Path, PathBuf};
use tracing::info;

use zerolaunch_plugin_protocol::manifest::Manifest;

/// Error type for plugin installation.
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Manifest error: {0}")]
    Manifest(String),
    #[error("Plugin already installed: {0}")]
    AlreadyInstalled(String),
}

/// Install a plugin from a .zip file.
/// Extracts to `plugins_dir/<plugin_id>/` after validating the manifest.
pub fn install_from_zip(zip_path: &Path, plugins_dir: &Path) -> Result<PathBuf, InstallError> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // First pass: find manifest.toml to get the plugin id
    let mut manifest_content = String::new();

    let mut find_manifest = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name == "manifest.toml" {
            use std::io::Read;
            entry.read_to_string(&mut manifest_content)?;
            find_manifest = true;
            break;
        }
    }

    // 这里还要判断是不是空的，如果当前目录下没有这个，那么需要报错
    if !find_manifest {
        return Err(InstallError::Manifest(format!(
            "manifest 的内容是空的或找不到对应的manifest: {}",
            zip_path.to_string_lossy()
        )));
    }

    let manifest: Manifest = toml::from_str(&manifest_content)
        .map_err(|e| InstallError::Manifest(format!("invalid manifest: {}", e)))?;

    let plugin_id = &manifest.plugin.id;
    let target_dir = plugins_dir.join(plugin_id);

    if target_dir.exists() {
        return Err(InstallError::AlreadyInstalled(plugin_id.clone()));
    }

    // Create target directory
    std::fs::create_dir_all(&target_dir)?;

    // Collect all entry names for common-prefix detection
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .collect();

    // Find the common directory prefix across all entries.
    // This handles both single-top-level-dir zips and flat zips correctly.
    let common_prefix = find_common_prefix(&names);

    // Extract all files
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        // Strip the common prefix directory if present
        let relative = if let Some(rest) = name.strip_prefix(&common_prefix) {
            let trimmed = rest.trim_start_matches('/');
            if trimmed.is_empty() {
                continue; // skip the top-level directory entry itself
            }
            trimmed
        } else {
            // No common prefix to strip — use the name as-is (flat zip)
            &name
        };

        if relative.is_empty() {
            continue;
        }

        // Reject path traversal: absolute paths or ParentDir components.
        let normalized = std::path::Path::new(relative);
        if normalized.is_absolute() {
            return Err(InstallError::Manifest("absolute path in zip".into()));
        }
        for c in normalized.components() {
            if matches!(c, std::path::Component::ParentDir) {
                return Err(InstallError::Manifest("parent-dir traversal in zip".into()));
            }
        }

        let out_path = target_dir.join(relative);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }

    info!(
        "Installed plugin {} from {} to {}",
        plugin_id,
        zip_path.display(),
        target_dir.display()
    );

    Ok(target_dir)
}

/// Install a plugin from a directory (copy to plugins dir).
pub fn install_from_dir(source_dir: &Path, plugins_dir: &Path) -> Result<PathBuf, InstallError> {
    let manifest_path = source_dir.join("manifest.toml");
    if !manifest_path.exists() {
        return Err(InstallError::Manifest(
            "manifest.toml not found in source directory".into(),
        ));
    }

    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let manifest: Manifest = toml::from_str(&manifest_content)
        .map_err(|e| InstallError::Manifest(format!("invalid manifest: {}", e)))?;

    let plugin_id = &manifest.plugin.id;
    let target_dir = plugins_dir.join(plugin_id);

    if target_dir.exists() {
        return Err(InstallError::AlreadyInstalled(plugin_id.clone()));
    }

    copy_dir_recursive(source_dir, &target_dir)?;

    Ok(target_dir)
}

/// Find the common path prefix among all entry names in a zip archive.
/// For a well-formed zip with a single top-level directory (e.g. `my-plugin/...`),
/// returns `"my-plugin"`. For a flat zip with entries at root, returns `""`.
fn find_common_prefix(names: &[String]) -> String {
    if names.is_empty() {
        return String::new();
    }

    // Use the first entry as the candidate prefix
    let first = &names[0];
    // Take only the directory part of the first entry (if it has a slash)
    let first_dir = match first.find('/') {
        Some(idx) => &first[..idx],
        None => return String::new(), // first entry has no directory → flat zip
    };

    // Verify that all other entries start with this prefix
    let prefix_with_slash = format!("{}/", first_dir);
    for name in &names[1..] {
        if !name.starts_with(&prefix_with_slash) {
            // Not all entries share this prefix → no common prefix to strip
            return String::new();
        }
    }

    first_dir.to_string()
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
