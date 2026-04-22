use crate::sdk::host_api::HostApiError;
use crate::sdk::hotkey::types::{Hotkey, HotkeyCallback, HotkeyEventFilter};
use async_trait::async_trait;

/// 按键管理器 trait，定义平台原语。
/// 平台实现者实现各原语方法，HostApi 通过注入的 HotkeyManager 委托调用。
/// 同时负责管理上层回调注册和事件分发。
#[async_trait]
pub trait HotkeyManager: Send + Sync {
    /// 注册全局快捷键。
    /// 参数：hotkey - 快捷键定义。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn register_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError>;

    /// 注销全局快捷键。
    /// 参数：hotkey - 快捷键定义。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn unregister_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError>;

    /// 注销所有快捷键。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn unregister_all(&self) -> Result<(), HostApiError>;

    /// 启用双击 Ctrl 监听。
    /// 参数：enabled - 是否启用。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn set_double_ctrl_enabled(&self, enabled: bool) -> Result<(), HostApiError>;

    /// 开始监听按键事件。
    /// 内部构建事件分发器，将按键事件路由到已注册的回调。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn start_listening(&self) -> Result<(), HostApiError>;

    /// 停止监听按键事件。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn stop_listening(&self) -> Result<(), HostApiError>;

    /// 检查是否正在监听。
    /// 返回：正在监听返回 true，否则返回 false。
    fn is_listening(&self) -> bool;

    /// 注册按键事件回调。
    /// 参数：id - 回调标识；filter - 事件过滤器；callback - 回调函数。
    fn register_callback(&self, id: &str, filter: HotkeyEventFilter, callback: HotkeyCallback);

    /// 注销按键事件回调。
    /// 参数：id - 回调标识。
    fn unregister_callback(&self, id: &str);
}
