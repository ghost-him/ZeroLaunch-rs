use crate::sdk::focus_monitor::types::FocusCallback;

/// 聚焦监控器 trait，定义平台无关的窗口焦点检测能力契约。
/// 各平台实现此 trait，处理平台特定的窗口事件监听。
/// 支持多个回调注册，窗口失去焦点时依次调用所有回调。
/// 推送式（push-based）服务，与 InstallationMonitor / HotkeyManager 一致。
///
/// FocusMonitor 是强制启用的系统服务，构造即开始监控，不可停止。
pub trait FocusMonitor: Send + Sync {
    /// 注册焦点事件回调。
    fn register_callback(&self, id: &str, callback: FocusCallback);

    /// 注销焦点事件回调。
    fn unregister_callback(&self, id: &str);
}
