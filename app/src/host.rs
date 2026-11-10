//! GPUI 宿主实现：把内核的窗口、通知与事件出口接到 GPUI/Win32 外壳。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use zerolaunch_plugin_api::services::window::MonitorInfo;
use zerolaunch_plugin_api::CandidateId;
use zerolaunch_rs_lib::core::host::{Host, UpdateMenuState};

use crate::platform;

/// 视图线程需要执行的宿主命令。
#[derive(Debug)]
pub enum HostCommand {
    /// 退出应用事件循环。
    Exit,
}

/// 内核推给 GPUI 视图的宿主事件。
#[derive(Debug, Clone)]
pub enum HostEvent {
    /// 会话投影变化（已序列化载荷）。
    SessionState(serde_json::Value),
    /// 候选项缓存预热完成。
    CandidatesReady,
    /// 搜索主窗口已显示，视图应恢复输入焦点。
    MainWindowShown,
    /// 配置组件变更。
    ConfigChanged {
        /// 组件 id。
        component_id: String,
        /// 组件类型（DataSource / ExclusionRule 等）。
        component_type: String,
    },
    /// 查询结果图标异步回填。
    IconUpdated {
        /// 图标所属候选缓存世代。
        candidate_generation: u64,
        /// 候选 id。
        id: CandidateId,
        /// 图标 data URL。
        icon: String,
    },
    /// 插件安装完成。
    PluginInstalled {
        /// 插件 id。
        plugin_id: String,
        /// 插件显示名。
        name: String,
        /// 插件版本。
        version: String,
    },
    /// 插件卸载完成。
    PluginUninstalled {
        /// 插件 id。
        plugin_id: String,
    },
    /// 系统主题变化。
    SystemThemeChanged(String),
    /// 检查器状态变化。
    InspectorStateUpdated,
}

/// GPUI 宿主：窗口操作走 Win32，跨线程事件走广播通道。
pub struct GpuiHost {
    /// 交给 GPUI 前台任务执行的命令通道。
    commands: mpsc::UnboundedSender<HostCommand>,
    /// 推送给视图的宿主事件广播。
    events: broadcast::Sender<HostEvent>,
    /// 顶层窗口句柄。
    hwnd: Mutex<isize>,
    /// 设置窗口句柄；未绑定前为 0。
    settings_hwnd: Mutex<isize>,
    /// 设置窗口标题；窗口绑定前暂存，绑定后立即应用。
    settings_title: Mutex<Option<String>>,
    /// 设置窗口物理 bounds（x, y, width, height）。
    settings_bounds: Mutex<Option<(i32, i32, i32, i32)>>,
    /// 最近一次显示的可见状态。
    visible: AtomicBool,
}

impl GpuiHost {
    /// 创建宿主。
    /// 参数：commands - 交给视图线程执行的宿主命令通道。
    /// 返回：可安装到 AppState 的宿主实例。
    pub fn new(commands: mpsc::UnboundedSender<HostCommand>) -> Self {
        let (events, _) = broadcast::channel(128);
        Self {
            commands,
            events,
            hwnd: Mutex::new(0),
            settings_hwnd: Mutex::new(0),
            settings_title: Mutex::new(None),
            settings_bounds: Mutex::new(None),
            visible: AtomicBool::new(false),
        }
    }

    /// 绑定已创建的 GPUI 窗口。
    /// 参数：hwnd - 顶层窗口句柄；visible - 是否按当前显示状态初始化。
    /// 返回：无。
    pub fn attach(&self, hwnd: isize, visible: bool) {
        *self.hwnd.lock().expect("host hwnd lock poisoned") = hwnd;
        self.visible.store(visible, Ordering::Release);
        platform::configure_launcher(hwnd);
    }

    /// 绑定设置窗口句柄。
    /// 参数：hwnd - 设置窗口顶层句柄。
    /// 返回：无；若标题已由内核设置则立即应用。
    pub fn attach_settings(&self, hwnd: isize) {
        *self
            .settings_hwnd
            .lock()
            .expect("settings hwnd lock poisoned") = hwnd;
        if let Some(title) = self
            .settings_title
            .lock()
            .expect("settings title lock poisoned")
            .as_deref()
        {
            platform::set_window_title(hwnd, title);
        }
    }

    /// 读取搜索主窗口所在显示器。
    /// 返回：显示器信息；主窗口尚未绑定时 None。
    pub fn main_window_monitor(&self) -> Option<MonitorInfo> {
        platform::window_monitor_info(self.hwnd())
    }

    /// 记录设置窗口的物理位置与尺寸。
    /// 参数：bounds - (x, y, width, height)，物理像素。
    /// 返回：无；下一次显示时应用。
    pub fn set_settings_bounds(&self, bounds: (i32, i32, i32, i32)) {
        *self
            .settings_bounds
            .lock()
            .expect("settings bounds lock poisoned") = Some(bounds);
    }

    /// 订阅宿主事件。
    /// 返回：广播接收端；视图在生命周期内持续消费。
    pub fn subscribe(&self) -> broadcast::Receiver<HostEvent> {
        self.events.subscribe()
    }

    fn hwnd(&self) -> isize {
        *self.hwnd.lock().expect("host hwnd lock poisoned")
    }

    fn settings_hwnd(&self) -> isize {
        *self
            .settings_hwnd
            .lock()
            .expect("settings hwnd lock poisoned")
    }

    fn publish(&self, event: HostEvent) {
        let _ = self.events.send(event);
    }
}

impl Host for GpuiHost {
    fn hide_main_window(&self) {
        platform::hide_window(self.hwnd());
        self.visible.store(false, Ordering::Release);
    }

    fn show_main_window(&self) {
        platform::show_window(self.hwnd());
        self.visible.store(true, Ordering::Release);
        self.publish(HostEvent::MainWindowShown);
    }

    fn is_main_window_visible(&self) -> bool {
        self.visible.load(Ordering::Acquire) && platform::is_window_visible(self.hwnd())
    }

    fn set_main_window_position(&self, x: i32, y: i32) {
        platform::set_window_position(self.hwnd(), x, y);
    }

    fn main_window_position(&self) -> Option<(i32, i32)> {
        platform::window_position(self.hwnd())
    }

    fn monitor_info(&self) -> Vec<MonitorInfo> {
        platform::monitor_info()
    }

    fn show_settings_window(&self) {
        let hwnd = self.settings_hwnd();
        if let Some((x, y, width, height)) = *self
            .settings_bounds
            .lock()
            .expect("settings bounds lock poisoned")
        {
            platform::set_window_bounds(hwnd, x, y, width, height);
        }
        platform::show_window(hwnd);
    }

    fn hide_settings_window(&self) {
        platform::hide_window(self.settings_hwnd());
    }

    fn set_settings_window_title(&self, title: &str) {
        *self
            .settings_title
            .lock()
            .expect("settings title lock poisoned") = Some(title.to_string());
        platform::set_window_title(self.settings_hwnd(), title);
    }

    fn notify(&self, title: &str, body: &str) {
        if let Err(error) = notify_rust::Notification::new()
            .summary(title)
            .body(body)
            .show()
        {
            tracing::warn!("发送系统通知失败: {error}");
        }
    }

    fn exit(&self) {
        let _ = self.commands.send(HostCommand::Exit);
    }

    fn emit_session_state(&self, event: serde_json::Value) {
        self.publish(HostEvent::SessionState(event));
    }

    fn emit_candidates_ready(&self) {
        self.publish(HostEvent::CandidatesReady);
    }

    fn emit_config_changed(&self, config_component_id: &str, component_type: &str) {
        self.publish(HostEvent::ConfigChanged {
            component_id: config_component_id.to_string(),
            component_type: component_type.to_string(),
        });
    }

    fn emit_inspector_state_updated(&self) {
        self.publish(HostEvent::InspectorStateUpdated);
    }

    fn emit_icon_updated(&self, candidate_generation: u64, id: CandidateId, icon: &str) {
        self.publish(HostEvent::IconUpdated {
            candidate_generation,
            id,
            icon: icon.to_string(),
        });
    }

    fn emit_plugin_installed(&self, plugin_id: &str, name: &str, version: &str) {
        self.publish(HostEvent::PluginInstalled {
            plugin_id: plugin_id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
        });
    }

    fn emit_plugin_uninstalled(&self, plugin_id: &str) {
        self.publish(HostEvent::PluginUninstalled {
            plugin_id: plugin_id.to_string(),
        });
    }

    fn emit_system_theme_changed(&self, theme: &str) {
        self.publish(HostEvent::SystemThemeChanged(theme.to_string()));
    }

    fn tray_set_visible(&self, _visible: bool) {}
    fn tray_update_language(&self) {}
    fn tray_set_game_mode(&self, _checked: bool) {}
    fn tray_set_update_state(&self, _state: UpdateMenuState) {}
}
