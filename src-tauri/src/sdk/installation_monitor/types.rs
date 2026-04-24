use std::fmt::Debug;
use std::sync::Arc;

/// 安装监控事件，表示监控目录中发生了文件系统变化。
#[derive(Debug, Clone)]
pub struct InstallationEvent {
    /// 发生变化的文件路径列表
    pub changed_paths: Vec<String>,
    /// 变化类型
    pub kind: InstallationEventKind,
}

/// 文件系统变化类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationEventKind {
    /// 文件/目录被创建（可能表示新程序安装）
    Created,
    /// 文件/目录被修改
    Modified,
    /// 文件/目录被删除（可能表示程序卸载）
    Removed,
    /// 其他或混合变化
    Other,
}

/// 安装监控回调函数类型。
/// 当监控目录发生变化时，所有已注册的回调将被依次调用。
pub type InstallationCallback = Arc<dyn Fn(InstallationEvent) + Send + Sync>;

/// 回调注册信息（内部使用）。
pub(crate) struct CallbackRegistration {
    /// 回调 ID（用于注销）
    pub id: String,
    /// 回调函数
    pub callback: InstallationCallback,
}

impl Debug for CallbackRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackRegistration")
            .field("id", &self.id)
            .finish()
    }
}
