use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;

/// 安装监控事件，表示监控目录中发生了文件系统变化。
/// 跨进程作为 `plugin/installation.event` 载荷字段 `event` 推送（camelCase 键名）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationEvent {
    /// 发生变化的文件路径列表
    #[serde(rename = "changedPaths")]
    pub changed_paths: Vec<String>,
    /// 变化类型
    #[serde(rename = "kind")]
    pub kind: InstallationEventKind,
}

/// 文件系统变化类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallationEventKind {
    /// 文件/目录被创建（可能表示新程序安装）
    #[serde(rename = "created")]
    Created,
    /// 文件/目录被修改（可能表示程序更新）
    #[serde(rename = "modified")]
    Modified,
    /// 文件/目录被删除（可能表示程序卸载）
    #[serde(rename = "removed")]
    Removed,
    /// 其他或混合变化
    #[serde(rename = "other")]
    Other,
}

/// 安装监控回调函数类型。
/// 当监控目录发生变化时，所有已注册的回调将被依次调用。
pub type InstallationCallback = Arc<dyn Fn(InstallationEvent) + Send + Sync>;
