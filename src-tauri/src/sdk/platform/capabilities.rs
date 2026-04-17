use std::collections::HashSet;

/// 平台能力枚举。
/// 列举 Plugin SDK 可能暴露的所有平台能力，不同平台的支撑情况不同。
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum PlatformCapability {
    /// 图标提取：从文件/网址/扩展名中提取图标
    IconExtraction,
    /// Shell 打开：使用系统默认方式打开文件/网址/文件夹
    ShellOpen,
    /// 以管理员身份运行
    RunAsAdmin,
    /// UWP 应用启动（仅 Windows）
    UwpLaunch,
    /// 窗口激活：根据进程名或标题激活已存在的窗口
    WindowActivation,
}

/// 平台能力集合。
/// 封装当前平台支持的能力，提供便捷的查询接口。
#[derive(Clone)]
pub struct PlatformCapabilities {
    capabilities: HashSet<PlatformCapability>,
}

impl PlatformCapabilities {
    /// 创建包含指定能力集合的 PlatformCapabilities。
    /// 参数：capabilities - 平台支持的能力集合。
    /// 返回：初始化后的 PlatformCapabilities。
    pub fn new(capabilities: HashSet<PlatformCapability>) -> Self {
        Self { capabilities }
    }

    /// 查询当前平台是否支持指定能力。
    /// 参数：capability - 待查询的能力。
    /// 返回：支持返回 true，不支持返回 false。
    pub fn has(&self, capability: PlatformCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// 获取当前平台所有支持的能力集合。
    /// 参数：无。
    /// 返回：能力集合的不可变引用。
    pub fn all(&self) -> &HashSet<PlatformCapability> {
        &self.capabilities
    }
}

/// Windows 平台的完整能力集构造函数。
/// 仅在 Windows 平台实现中使用，不应作为通用默认值。
#[cfg(target_os = "windows")]
impl PlatformCapabilities {
    pub fn windows() -> Self {
        Self::new(HashSet::from([
            PlatformCapability::IconExtraction,
            PlatformCapability::ShellOpen,
            PlatformCapability::RunAsAdmin,
            PlatformCapability::UwpLaunch,
            PlatformCapability::WindowActivation,
        ]))
    }
}
