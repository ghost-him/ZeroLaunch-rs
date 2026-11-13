//! 宿主端口：内核只依赖本 trait，不感知 GPUI 或平台类型。
//!
//! GPUI 宿主实现本 trait；无窗口场景使用 `NoopHost`。

use parking_lot::RwLock;
use std::sync::Arc;
use zerolaunch_plugin_api::services::window::MonitorInfo;
use zerolaunch_plugin_api::CandidateId;

/// 「检查更新」菜单项的展示状态。
///
/// 菜单项既是操作入口也是结果回显：已是最新/失败时保留「检查更新」字样，
/// 点击仍然重新检查。自动检查只在发现新版本时改文案，避免启动即显示「已是最新」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateMenuState {
    /// 尚未检查（显示「检查更新」）
    Idle,
    /// 用户主动发起的检查进行中
    Checking,
    /// 已是最新版本（携带当前版本号）
    UpToDate(String),
    /// 发现新版本（携带最新版本号）
    Available(String),
    /// 上次检查失败
    Failed,
}

/// 宿主能力端口：窗口、通知、托盘与前端事件出口。
pub trait Host: Send + Sync {
    /// 隐藏主窗口（含旧前端需要的失焦通知）。
    fn hide_main_window(&self);
    /// 显示并聚焦主窗口（含旧前端需要的显示通知）。
    fn show_main_window(&self);
    /// 主窗口当前是否可见。
    fn is_main_window_visible(&self) -> bool;
    /// 设置主窗口物理坐标。
    fn set_main_window_position(&self, x: i32, y: i32);
    /// 读取主窗口物理坐标；窗口不存在时返回 None。
    fn main_window_position(&self) -> Option<(i32, i32)>;
    /// 收集显示器信息供窗口定位。
    fn monitor_info(&self) -> Vec<MonitorInfo>;
    /// 显示并聚焦设置窗口。
    fn show_settings_window(&self);
    /// 隐藏设置窗口。
    fn hide_settings_window(&self);
    /// 更新设置窗口标题。
    fn set_settings_window_title(&self, title: &str);
    /// 发送系统通知。
    fn notify(&self, title: &str, body: &str);
    /// 退出程序。
    fn exit(&self);

    /// 推送会话投影（已序列化的事件载荷）。
    fn emit_session_state(&self, event: serde_json::Value);
    /// 通知前端候选缓存已预热完成。
    fn emit_candidates_ready(&self);
    /// 推送配置变更。
    fn emit_config_changed(&self, config_component_id: &str, component_type: &str);
    /// 通知检查器状态变化。
    fn emit_inspector_state_updated(&self);
    /// 推送查询结果图标回填。
    fn emit_icon_updated(&self, candidate_generation: u64, id: CandidateId, icon: &str);
    /// 推送插件安装完成。
    fn emit_plugin_installed(&self, plugin_id: &str, name: &str, version: &str);
    /// 推送插件卸载完成。
    fn emit_plugin_uninstalled(&self, plugin_id: &str);

    /// 设置托盘图标可见性。
    fn tray_set_visible(&self, visible: bool);
    /// 按当前语言重建托盘菜单。
    fn tray_update_language(&self);
    /// 同步游戏模式菜单勾选状态。
    fn tray_set_game_mode(&self, checked: bool);
    /// 更新「检查更新」菜单状态。
    fn tray_set_update_state(&self, state: UpdateMenuState);
}

/// 无窗口宿主：CLI、集成测试与内核初始化阶段使用。
#[derive(Default)]
pub struct NoopHost {
    visible: RwLock<bool>,
}

impl Host for NoopHost {
    fn hide_main_window(&self) {
        *self.visible.write() = false;
    }

    fn show_main_window(&self) {
        *self.visible.write() = true;
    }

    fn is_main_window_visible(&self) -> bool {
        *self.visible.read()
    }

    fn set_main_window_position(&self, _x: i32, _y: i32) {}

    fn main_window_position(&self) -> Option<(i32, i32)> {
        None
    }

    fn monitor_info(&self) -> Vec<MonitorInfo> {
        Vec::new()
    }

    fn show_settings_window(&self) {}

    fn hide_settings_window(&self) {}

    fn set_settings_window_title(&self, _title: &str) {}

    fn notify(&self, _title: &str, _body: &str) {}

    fn exit(&self) {}

    fn emit_session_state(&self, _event: serde_json::Value) {}
    fn emit_candidates_ready(&self) {}
    fn emit_config_changed(&self, _config_component_id: &str, _component_type: &str) {}
    fn emit_inspector_state_updated(&self) {}
    fn emit_icon_updated(&self, _candidate_generation: u64, _id: CandidateId, _icon: &str) {}
    fn emit_plugin_installed(&self, _plugin_id: &str, _name: &str, _version: &str) {}
    fn emit_plugin_uninstalled(&self, _plugin_id: &str) {}

    fn tray_set_visible(&self, _visible: bool) {}
    fn tray_update_language(&self) {}
    fn tray_set_game_mode(&self, _checked: bool) {}
    fn tray_set_update_state(&self, _state: UpdateMenuState) {}
}

/// 当前宿主实现（默认无窗口）。
pub type SharedHost = Arc<dyn Host>;
