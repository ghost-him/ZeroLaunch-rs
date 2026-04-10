use serde::{Deserialize, Serialize};

/// 配置动作定义，描述组件在配置面板中提供的辅助操作。
/// 如"自动检测浏览器"等一键式操作，无需参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigActionDef {
    /// 动作唯一标识符，如 "detect_browsers"
    pub action: String,
    /// 动作显示名称，用于 UI 按钮文本
    pub label: String,
    /// 动作描述
    pub description: String,
}
