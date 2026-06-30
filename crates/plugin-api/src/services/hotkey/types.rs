use serde::{Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};

/// 按键组合定义。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Hotkey {
    /// 主键（如 "Space", "A", "F1"）
    pub key: String,
    /// Ctrl 修饰键
    pub ctrl: bool,
    /// Alt 修饰键
    pub alt: bool,
    /// Shift 修饰键
    pub shift: bool,
    /// Meta/Win 修饰键
    pub meta: bool,
}

impl Hotkey {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }
    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }
    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }
    pub fn with_meta(mut self) -> Self {
        self.meta = true;
        self
    }
}

/// 按键事件类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HotkeyEvent {
    GlobalHotkey(Hotkey),
    DoubleCtrl,
}

/// 事件过滤器，用于回调注册时指定关注的事件类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyEventFilter {
    All,
    GlobalHotkey(Hotkey),
    DoubleCtrl,
}

/// 按键回调函数类型。
pub type HotkeyCallback = Arc<dyn Fn(HotkeyEvent) + Send + Sync>;

/// 回调注册信息。
pub struct CallbackRegistration {
    pub id: String,
    pub filter: HotkeyEventFilter,
    pub callback: HotkeyCallback,
}

impl Debug for CallbackRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackRegistration")
            .field("id", &self.id)
            .field("filter", &self.filter)
            .finish()
    }
}

/// 单个快捷键注册信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyRegistration {
    pub hotkey: Hotkey,
}

/// 按键监听配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HotkeyConfig {
    pub hotkeys: Vec<HotkeyRegistration>,
    pub double_ctrl_enabled: bool,
}
