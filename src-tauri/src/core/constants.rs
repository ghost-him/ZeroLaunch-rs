//! 应用级常量定义
//!
//! 该模块定义了应用程序级别的常量，包括：
//! - 应用版本号
//! - 应用名称
//! - 资源路径映射

use dashmap::DashMap;
use lazy_static::lazy_static;

// ============================================================================
// 应用基本信息
// ============================================================================

/// 应用名称
pub const APP_NAME: &str = "ZeroLaunch-rs";

/// 当前软件的版本号（从 Cargo.toml 读取）
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 应用作者
pub const APP_AUTHORS: &str = "ZeroLaunch Team";

/// 应用描述
pub const APP_DESCRIPTION: &str = "A fast application launcher for Windows";

// ============================================================================
// 资源路径
// ============================================================================

lazy_static! {
    /// 应用使用到的图片资源路径映射
    /// Key: 资源名称（如 "tray_icon"）
    /// Value: 资源文件的完整路径
    ///
    /// 该映射在程序初始化时通过 PathResolver 填充
    pub static ref APP_PIC_PATH: DashMap<String, String> = DashMap::new();
}

// ============================================================================
// 配置相关常量
// ============================================================================

/// 本地配置文件名称
/// 用于存储本地特定的设置（如存储后端配置）
pub const LOCAL_CONFIG_NAME: &str = "ZeroLaunch_local_config.json";

/// 远程配置文件名称
/// 用于存储用户的应用设置，可同步到云端
pub const REMOTE_CONFIG_NAME: &str = "ZeroLaunch_remote_config.json";

/// 配置文件版本号，用于迁移和兼容性检查
pub const CONFIG_VERSION: &str = "3";

// ============================================================================
// 功能开关
// ============================================================================

/// 是否为便携模式（编译时决定）
pub const fn is_portable_mode() -> bool {
    cfg!(feature = "portable")
}
