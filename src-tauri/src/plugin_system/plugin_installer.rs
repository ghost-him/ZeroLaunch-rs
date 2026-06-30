//! PluginInstaller — 插件安装/扫描/验证。
//!
//! 从 PluginManager 中提取的安装职责域，负责 zip 安装、目录安装、
//! 插件目录扫描、安装目录安全性验证。

use std::io::Read;
use std::path::{Path, PathBuf};

use tracing::{debug, info};
use walkdir::WalkDir;
use zerolaunch_plugin_protocol::Manifest;

use super::plugin_info::InstallError;

/// 插件安装器，封装 zip/目录安装和目录扫描逻辑。
pub(crate) struct PluginInstaller {
    plugins_dir: PathBuf,
}

impl PluginInstaller {
    /// 创建安装器，指定插件根目录。
    pub(crate) fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    /// 扫描插件目录，返回包含 manifest.toml 的子目录列表。
    pub(crate) fn scan_plugins_dir(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        if !self.plugins_dir.exists() {
            return found;
        }
        if let Ok(entries) = std::fs::read_dir(&self.plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest = path.join("manifest.toml");
                    if manifest.exists() {
                        found.push(path);
                    }
                }
            }
        }
        found
    }

    /// 从 .zip 文件安装插件到 `plugins_dir/<plugin_id>/`。
    pub(crate) fn install_from_zip(&self, zip_path: &Path) -> Result<PathBuf, InstallError> {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        // 第一遍：找 manifest + 收集所有条目名（用于计算公共前缀）
        let mut manifest_content = String::new();
        let mut find_manifest = false;
        let mut names = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name == "manifest.toml" {
                entry.read_to_string(&mut manifest_content)?;
                find_manifest = true;
            }
            names.push(name);
        }

        if !find_manifest {
            return Err(InstallError::Manifest(format!(
                "manifest.toml not found in zip: {}",
                zip_path.to_string_lossy()
            )));
        }

        let manifest: Manifest = toml::from_str(&manifest_content)
            .map_err(|e| InstallError::Manifest(format!("invalid manifest: {}", e)))?;

        let plugin_id = &manifest.plugin.id;
        let target_dir = self.plugins_dir.join(plugin_id);

        if target_dir.exists() {
            return Err(InstallError::AlreadyInstalled(plugin_id.clone()));
        }

        std::fs::create_dir_all(&target_dir)?;

        let common_prefix = find_common_prefix(&names);

        // 第二遍：解压所有文件
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();

            let relative = if let Some(rest) = name.strip_prefix(&common_prefix) {
                let trimmed = rest.trim_start_matches('/');
                if trimmed.is_empty() {
                    continue;
                }
                trimmed
            } else {
                &name
            };

            if relative.is_empty() {
                continue;
            }

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

        verify_install_dir(&target_dir)?;

        info!(
            "Installed plugin {} from {} to {}",
            plugin_id,
            zip_path.display(),
            target_dir.display()
        );

        Ok(target_dir)
    }

    /// 从目录复制安装插件到 `plugins_dir/<plugin_id>/`。
    pub(crate) fn install_from_dir(&self, source_dir: &Path) -> Result<PathBuf, InstallError> {
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
        let target_dir = self.plugins_dir.join(plugin_id);

        if target_dir.exists() {
            return Err(InstallError::AlreadyInstalled(plugin_id.clone()));
        }

        copy_dir_recursive(source_dir, &target_dir)?;
        verify_install_dir(&target_dir)?;

        Ok(target_dir)
    }
}

// ── 私有辅助函数 ─────────────────────────────────────────────────

/// 校验安装目录内无符号链接和路径遍历（使用 walkdir 递归遍历）。
fn verify_install_dir(target_dir: &Path) -> Result<(), InstallError> {
    let canonical_target = target_dir
        .canonicalize()
        .map_err(|e| InstallError::Manifest(format!("cannot canonicalize target dir: {}", e)))?;

    for entry in WalkDir::new(target_dir).follow_links(false) {
        let entry = entry.map_err(|e| InstallError::Manifest(format!("walk error: {}", e)))?;
        let path = entry.path();

        if entry.file_type().is_symlink() {
            return Err(InstallError::Manifest(format!(
                "symlinks not allowed: {}",
                path.display()
            )));
        }

        let canonical = path.canonicalize().map_err(|e| {
            InstallError::Manifest(format!("cannot canonicalize {}: {}", path.display(), e))
        })?;
        if !canonical.starts_with(&canonical_target) {
            return Err(InstallError::Manifest(format!(
                "path traversal detected: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

/// 找到 zip 内所有条目的公共路径前缀。
///
/// 例如所有条目都以 `my-plugin/` 开头 → 返回 `"my-plugin"`，
/// 解压时会剥掉这层目录。如果条目没有公共前缀（比如所有文件都
/// 在 zip 根目录），返回空字符串，解压时不剥任何前缀。
fn find_common_prefix(names: &[String]) -> String {
    if names.is_empty() {
        return String::new();
    }

    let first = &names[0];
    let first_dir = match first.find('/') {
        Some(idx) => &first[..idx],
        None => {
            debug!("find_common_prefix: first entry has no '/', no prefix to strip ({first})");
            return String::new();
        }
    };

    let prefix_with_slash = format!("{}/", first_dir);
    for name in &names[1..] {
        if !name.starts_with(&prefix_with_slash) {
            debug!(
                "find_common_prefix: no common prefix (\"{name}\" does not start with \"{prefix_with_slash}\")"
            );
            return String::new();
        }
    }

    debug!("find_common_prefix: using prefix \"{first_dir}\"");
    first_dir.to_string()
}

/// 递归复制目录（使用 walkdir 避免手动递归）。
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(src).unwrap();
        let dst_path = dst.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
