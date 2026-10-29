use crate::host::error::HostApiError;
use crate::services::hotkey::types::{Hotkey, HotkeyCallback, HotkeyEventFilter};
use async_trait::async_trait;

/// 按键管理器 trait，定义平台原语。
#[async_trait]
pub trait HotkeyManager: Send + Sync {
    /// 校验快捷键能否被当前平台注册为全局快捷键（不产生注册副作用）。
    /// 默认实现视为可注册；平台实现应覆盖以报告键名或修饰键限制。
    async fn validate_hotkey(&self, _hotkey: &Hotkey) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn register_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError>;
    async fn unregister_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError>;
    async fn unregister_all(&self) -> Result<(), HostApiError>;
    /// 以给定集合替换当前全部注册。
    /// 默认实现先注销再逐个注册；平台实现应在注册失败时恢复原有注册，
    /// 避免一次失败的替换让旧快捷键全部失效。
    async fn replace_all_hotkeys(&self, hotkeys: &[Hotkey]) -> Result<(), HostApiError> {
        self.unregister_all().await?;
        for hotkey in hotkeys {
            self.register_hotkey(hotkey).await?;
        }
        Ok(())
    }
    async fn set_double_ctrl_enabled(&self, enabled: bool) -> Result<(), HostApiError>;
    async fn start_listening(&self) -> Result<(), HostApiError>;
    async fn stop_listening(&self) -> Result<(), HostApiError>;
    fn is_listening(&self) -> bool;
    fn register_callback(&self, id: &str, filter: HotkeyEventFilter, callback: HotkeyCallback);
    fn unregister_callback(&self, id: &str);
}
