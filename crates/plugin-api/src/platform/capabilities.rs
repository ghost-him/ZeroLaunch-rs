use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// 平台能力枚举。
/// 列举 Plugin SDK 可能暴露的所有平台能力，不同平台的支撑情况不同。
/// 跨进程经 `host/platform.capabilities` 以 camelCase 名称下发，插件据此降级。
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlatformCapability {
    /// 图标提取：从文件/网址/扩展名中提取图标
    #[serde(rename = "iconExtraction")]
    IconExtraction,
    /// Shell 打开：使用系统默认方式打开文件/网址/文件夹
    #[serde(rename = "shellOpen")]
    ShellOpen,
    /// 以管理员身份运行
    #[serde(rename = "runAsAdmin")]
    RunAsAdmin,
    /// 应用枚举：发现系统中已安装的应用（含沙箱/容器应用）
    #[serde(rename = "appEnumeration")]
    AppEnumeration,
    /// 应用启动：通过平台专属 API 启动应用
    #[serde(rename = "appLaunch")]
    AppLaunch,
    /// 窗口激活：根据进程名或标题激活已存在的窗口
    #[serde(rename = "windowActivation")]
    WindowActivation,
    /// 自启动管理：管理系统开机自启动
    #[serde(rename = "autoStart")]
    AutoStart,
    /// 按键监听：全局快捷键和双击 Ctrl 监听
    #[serde(rename = "hotkeyListening")]
    HotkeyListening,
    /// 安装监控：监控文件系统变化（如开始菜单），检测程序安装/卸载
    #[serde(rename = "installationMonitoring")]
    InstallationMonitoring,
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

    /// 获取按固定顺序排列的能力列表（供跨进程传输）。
    /// 参数：无。
    /// 返回：按枚举声明顺序排序的能力名列表。
    pub fn to_list(&self) -> Vec<PlatformCapability> {
        let mut list: Vec<PlatformCapability> = self.capabilities.iter().copied().collect();
        list.sort();
        list
    }
}
