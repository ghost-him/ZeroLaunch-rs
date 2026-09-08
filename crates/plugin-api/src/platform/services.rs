//! PlatformServices — 平台实现的统一集合（平台端口）。
//!
//! 只收纳 OS 特定、由平台 crate（platform-windows）提供实现的服务 trait 对象；
//! 宿主级服务（model/storage/timer/parameter_resolver/app_resource/icon_cache 等）
//! 与 OS 无关，不属于平台面，留在宿主侧。
//!
//! 由平台 crate 的工厂函数（如 platform-windows 的 `windows_platform_services`）
//! 一次性构造并注入宿主；测试通过 mock 桩构造。能力集（PlatformCapabilities）
//! 与实现集合在同一模块声明；新增平台能力时须同步扩展两者（同步链见
//! .omp/rules/add-platform-capability.md）。

use std::sync::Arc;

use super::capabilities::PlatformCapabilities;
use crate::services::app::app_enumerator::AppEnumerator;
use crate::services::app::app_launcher::AppLauncher;
use crate::services::autostart::AutoStartManager;
use crate::services::clipboard::ClipboardManager;
use crate::services::focus_monitor::FocusMonitor;
use crate::services::hotkey::HotkeyManager;
use crate::services::icon::icon_extractor::IconExtractor;
use crate::services::installation_monitor::InstallationMonitor;
use crate::services::parameter::provider::SystemParameterProvider;
use crate::services::path::path_resolver::PathResolver;
use crate::services::shell::lnk_resolver::LnkResolver;
use crate::services::shell::resource_loader::ResourceLoader;
use crate::services::shell::ShellExecutor;
use crate::services::theme::ThemeProvider;
use crate::services::window::{WindowManager, WindowPositioner};

/// 平台实现的统一集合（平台端口）。
/// 字段为平台服务 trait 对象；组装权归各平台 crate 的工厂，宿主按端口整体注入。
pub struct PlatformServices {
    /// 平台支持的能力集合（与实现集合在同一模块声明，新增能力须同步扩展）。
    pub capabilities: PlatformCapabilities,
    /// 图标提取器。
    pub icon_extractor: Arc<dyn IconExtractor>,
    /// Shell 执行器（打开/提权/执行命令）。
    pub shell_executor: Arc<dyn ShellExecutor>,
    /// 窗口管理器（按进程名/标题/PID 激活窗口）。
    pub window_manager: Arc<dyn WindowManager>,
    /// 窗口位置计算器。
    pub window_positioner: Arc<dyn WindowPositioner>,
    /// 路径解析器（KnownPath → 实际文件系统路径）。
    pub path_resolver: Arc<dyn PathResolver>,
    /// 应用枚举器（发现已安装应用）。
    pub app_enumerator: Arc<dyn AppEnumerator>,
    /// 应用启动器。
    pub app_launcher: Arc<dyn AppLauncher>,
    /// Lnk 快捷方式解析器。
    pub lnk_resolver: Arc<dyn LnkResolver>,
    /// 资源加载器（desktop.ini 本地化名称解析等）。
    pub resource_loader: Arc<dyn ResourceLoader>,
    /// 剪贴板系统参数提供者（`{clipboard}` 参数）。
    pub clipboard_provider: Arc<dyn SystemParameterProvider>,
    /// 窗口句柄系统参数提供者（`{hwnd}` 参数）。
    pub window_handle_provider: Arc<dyn SystemParameterProvider>,
    /// 选中文本系统参数提供者（`{selection}` 参数）。
    pub selection_provider: Arc<dyn SystemParameterProvider>,
    /// 开机自启动管理器。
    pub autostart_manager: Arc<dyn AutoStartManager>,
    /// 全局热键管理器。
    pub hotkey_manager: Arc<dyn HotkeyManager>,
    /// 安装监控器。
    pub installation_monitor: Arc<dyn InstallationMonitor>,
    /// 聚焦监控器。
    pub focus_monitor: Arc<dyn FocusMonitor>,
    /// 剪贴板管理器。
    pub clipboard_manager: Arc<dyn ClipboardManager>,
    /// 系统主题提供器。
    pub theme_provider: Arc<dyn ThemeProvider>,
}
