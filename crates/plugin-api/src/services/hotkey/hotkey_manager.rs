use crate::host::error::HostApiError;
use crate::services::hotkey::types::{Hotkey, HotkeyCallback, HotkeyEventFilter};
use async_trait::async_trait;

/// 按键管理器 trait，定义平台原语。
#[async_trait]
pub trait HotkeyManager: Send + Sync {
    async fn register_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError>;
    async fn unregister_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError>;
    async fn unregister_all(&self) -> Result<(), HostApiError>;
    async fn set_double_ctrl_enabled(&self, enabled: bool) -> Result<(), HostApiError>;
    async fn start_listening(&self) -> Result<(), HostApiError>;
    async fn stop_listening(&self) -> Result<(), HostApiError>;
    fn is_listening(&self) -> bool;
    fn register_callback(&self, id: &str, filter: HotkeyEventFilter, callback: HotkeyCallback);
    fn unregister_callback(&self, id: &str);
}
