//! 插件安装错误类型定义。
//!
//! 由 PluginInstaller 产生，在 PluginManager 层转换为 PluginManagerError，
//! 最终由 GPUI 设置页映射为用户可见文案。

// ── InstallError ──────────────────────────────────────────────────

/// 插件安装错误类型。
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip 错误: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Manifest 错误: {0}")]
    Manifest(String),
    #[error("插件已安装: {0}")]
    AlreadyInstalled(String),
}
