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

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name == "manifest.toml" || name.ends_with("/manifest.toml") {
            use std::io::Read;
            entry.read_to_string(&mut manifest_content)?;
            break;
        }
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

    // Extract all files
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        // Strip top-level directory prefix if present
        let relative = if let Some(idx) = name.find('/') {
            &name[idx + 1..]
        } else {
            &name
        };

        if relative.is_empty() {
            continue;
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
