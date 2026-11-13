//! GPUI 宿主实现：把内核的窗口、通知与事件出口接到 GPUI/Win32 外壳。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use zerolaunch_plugin_api::services::window::MonitorInfo;
use zerolaunch_plugin_api::CandidateId;
use zerolaunch_rs_lib::core::host::{Host, UpdateMenuState};
use zerolaunch_rs_lib::core::i18n::I18nManager;

use crate::platform;
use crate::tray::{Tray, TrayLabels};

/// 系统通知使用的 AppUserModelID；与安装程序注册的应用标识一致。
const APP_ID: &str = "com.zerolaunch-rs.app";

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
    /// 插件脚本请求执行面板动作。
    PluginAction {
        /// 插件 id。
        plugin_id: String,
        /// 动作 id。
        action_id: String,
    },
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
    /// 系统托盘；初始化前或图标创建失败时为 None。
    tray: Mutex<Option<Tray>>,
    /// 托盘图标路径；托盘按需重建时复用。
    tray_icon: Mutex<String>,
    /// 当前游戏模式状态；托盘按需重建时复用。
    game_mode: AtomicBool,
    /// 后端翻译服务；用于托盘菜单文案。
    i18n: Mutex<Option<Arc<I18nManager>>>,
    /// 最近一次更新检查的托盘展示状态。
    update_state: Mutex<UpdateMenuState>,
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
            tray: Mutex::new(None),
            tray_icon: Mutex::new(String::new()),
            game_mode: AtomicBool::new(false),
            i18n: Mutex::new(None),
            update_state: Mutex::new(UpdateMenuState::Idle),
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

    /// 发布插件面板动作请求。
    /// 参数：plugin_id - 插件 id；action_id - 动作 id。
    /// 返回：无。
    pub fn plugin_action(&self, plugin_id: &str, action_id: &str) {
        self.publish(HostEvent::PluginAction {
            plugin_id: plugin_id.to_string(),
            action_id: action_id.to_string(),
        });
    }

    /// 安装后端翻译服务；托盘文案随语言变化时使用。
    /// 参数：i18n - 内核的翻译管理器。
    /// 返回：无。
    pub fn set_i18n(&self, i18n: Arc<I18nManager>) {
        *self.i18n.lock().expect("host i18n lock poisoned") = Some(i18n);
    }

    /// 创建系统托盘。
    /// 参数：icon_path - 托盘图标路径；visible - 是否显示；game - 游戏模式初始状态。
    /// 返回：无；创建失败只记录日志，不影响搜索窗口。
    pub fn init_tray(&self, icon_path: String, visible: bool, game: bool) {
        *self.tray_icon.lock().expect("host tray icon lock poisoned") = icon_path.clone();
        self.game_mode.store(game, Ordering::Release);
        let labels = self.tray_labels();
        match Tray::new(icon_path, labels, visible, game) {
            Ok(tray) => *self.tray.lock().expect("host tray lock poisoned") = Some(tray),
            Err(error) => tracing::warn!("启动系统托盘线程失败: {error}"),
        }
    }

    /// 按需创建托盘；图标加载失败后由可见性请求重试。
    /// 返回：创建成功为 true。
    fn try_create_tray(&self) -> bool {
        let (icon_path, game) = {
            (
                self.tray_icon
                    .lock()
                    .expect("host tray icon lock poisoned")
                    .clone(),
                self.game_mode.load(Ordering::Acquire),
            )
        };
        if icon_path.is_empty() {
            return false;
        }
        match Tray::new(icon_path, self.tray_labels(), true, game) {
            Ok(tray) => {
                *self.tray.lock().expect("host tray lock poisoned") = Some(tray);
                true
            }
            Err(error) => {
                tracing::warn!("重试创建系统托盘失败: {error}");
                false
            }
        }
    }

    /// 当前语言与更新状态对应的托盘文案。
    /// 返回：完整菜单文案；翻译服务尚未安装时回退为 i18n key。
    fn tray_labels(&self) -> TrayLabels {
        let update = self
            .update_state
            .lock()
            .expect("host update state lock poisoned")
            .clone();
        TrayLabels {
            show_settings: self.tray_text("tray.showSettings"),
            refresh: self.tray_text("tray.refreshDatabase"),
            reregister: self.tray_text("tray.reregisterHotkeys"),
            game_mode: self.tray_text("tray.gameMode"),
            update: self.tray_update_text(&update),
            exit: self.tray_text("tray.exitProgram"),
        }
    }

    /// 读取一条托盘译文。
    /// 参数：key - i18n key。
    /// 返回：当前语言译文；无翻译服务时返回 key。
    fn tray_text(&self, key: &str) -> String {
        let i18n = self.i18n.lock().expect("host i18n lock poisoned").clone();
        match i18n {
            Some(i18n) => {
                let language = i18n.current_language();
                i18n.t(&language, key)
            }
            None => key.to_string(),
        }
    }

    /// 按最近一次检查结果生成检查更新菜单文案。
    /// 参数：state - 检查状态。
    /// 返回：菜单文本；发现新版本时带版本号。
    fn tray_update_text(&self, state: &UpdateMenuState) -> String {
        match state {
            UpdateMenuState::Idle => self.tray_text("tray.checkUpdate"),
            UpdateMenuState::Checking => self.tray_text("tray.checking"),
            UpdateMenuState::UpToDate(version) => self
                .tray_text("tray.upToDate")
                .replace("{version}", version),
            UpdateMenuState::Available(version) => self
                .tray_text("tray.updateAvailable")
                .replace("{version}", version),
            UpdateMenuState::Failed => self.tray_text("tray.checkFailed"),
        }
    }

    /// 刷新托盘菜单文案。
    /// 返回：无；托盘尚未创建时忽略。
    fn refresh_tray_labels(&self) {
        let labels = self.tray_labels();
        if let Some(tray) = self.tray.lock().expect("host tray lock poisoned").as_ref() {
            tray.set_labels(labels);
        }
    }

    /// 广播一条宿主事件。
    /// 参数：event - 推送给视图的事件。
    /// 返回：无；无订阅者时忽略。
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
            .app_id(APP_ID)
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

    fn tray_set_visible(&self, visible: bool) {
        let missing = self.tray.lock().expect("host tray lock poisoned").is_none();
        if visible && missing && self.try_create_tray() {
            return;
        }
        if let Some(tray) = self.tray.lock().expect("host tray lock poisoned").as_ref() {
            tray.set_visible(visible);
        }
    }

    fn tray_update_language(&self) {
        self.refresh_tray_labels();
    }

    fn tray_set_game_mode(&self, checked: bool) {
        self.game_mode.store(checked, Ordering::Release);
        if let Some(tray) = self.tray.lock().expect("host tray lock poisoned").as_ref() {
            tray.set_game(checked);
        }
    }

    fn tray_set_update_state(&self, state: UpdateMenuState) {
        *self
            .update_state
            .lock()
            .expect("host update state lock poisoned") = state;
        self.refresh_tray_labels();
    }
}
