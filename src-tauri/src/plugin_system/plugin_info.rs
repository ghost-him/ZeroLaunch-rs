//! PluginManager 核心数据类型。
//!
//! 定义插件统一视图 `PluginInfo`、插件种类和状态枚举。
//! 这些类型不依赖任何业务模块，可在 PluginManager 内部和 IPC 层间安全传递。

/// 插件的统一视图，供管理 UI 和 CLI 查询使用。
/// 同时涵盖内置组件和第三方插件。
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// 插件唯一标识符。内置组件使用 component_id，第三方插件使用 manifest 中的 plugin.id。
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 插件种类：内置组件或第三方插件
    pub kind: PluginKind,
    /// 当前运行时状态
    pub status: PluginStatus,
    /// 语义版本号（仅 ThirdParty）
    pub version: Option<String>,
    /// 描述文本（仅 ThirdParty）
    pub description: Option<String>,
    /// 作者（仅 ThirdParty）
    pub author: Option<String>,
    /// 该插件提供的组件数量（DataSource / Executor / Plugin 等）
    pub component_count: usize,
    /// 是否处于启用状态（所有组件均启用）
    pub enabled: bool,
}

/// 插件种类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginKind {
    /// 编译进二进制、自动发现的内置组件
    Builtin,
    /// 外部子进程加载的第三方插件
    ThirdParty,
}

/// 插件运行时状态。
#[derive(Debug, Clone)]
pub enum PluginStatus {
    /// 正常运行中
    Active,
    /// 已安装但未运行
    Inactive,
    /// 运行异常，携带错误描述
    Error(String),
}

impl PluginInfo {
    /// 为内置组件创建 PluginInfo 条目。
    pub fn builtin(id: &str, name: &str, component_count: usize, enabled: bool) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            kind: PluginKind::Builtin,
            status: PluginStatus::Active,
            version: None,
            description: None,
            author: Some("ZeroLaunch".to_string()),
            component_count,
            enabled,
        }
    }

    /// 为第三方插件创建 PluginInfo 条目。
    #[allow(clippy::too_many_arguments)]
    pub fn third_party(
        id: String,
        name: String,
        version: String,
        description: String,
        author: String,
        component_count: usize,
        enabled: bool,
        is_running: bool,
    ) -> Self {
        let status = if is_running {
            PluginStatus::Active
        } else {
            PluginStatus::Inactive
        };
        Self {
            id,
            name,
            kind: PluginKind::ThirdParty,
            status,
            version: Some(version),
            description: Some(description),
            author: Some(author),
            component_count,
            enabled,
        }
    }

    /// 更新组件数和启用状态（用于配置变更后同步）。
    pub fn update_state(&mut self, component_count: usize, enabled: bool) {
        self.component_count = component_count;
        self.enabled = enabled;
    }
}

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
