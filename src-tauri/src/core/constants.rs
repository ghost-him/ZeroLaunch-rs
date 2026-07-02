//! 应用级常量定义
//!
//! 该模块定义了应用程序级别的常量，包括：
//! - 应用版本号
//! - 应用名称
//! - 配置相关常量

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
