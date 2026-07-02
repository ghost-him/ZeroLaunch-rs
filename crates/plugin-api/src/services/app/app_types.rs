use serde::{Deserialize, Serialize};

use crate::services::icon_request::IconRequest;

/// 应用信息，跨平台统一结构。
/// 各平台实现将平台特定的应用数据映射到此结构，插件层无需关心平台差异。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// 应用唯一标识。
    /// - Windows: AppUserModelID (UWP) 或 exe 路径
    /// - macOS: Bundle ID（预留）
    /// - Linux: .desktop 文件名或 Flatpak app-id（预留）
    pub app_id: String,
    /// 显示名称
    pub display_name: String,
    /// 图标路径或图标标识符
    pub icon: IconRequest,
    /// 安装路径（某些平台可能为空，如 UWP 沙箱应用）
    pub install_path: Option<String>,
}
