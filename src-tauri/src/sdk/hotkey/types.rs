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
    /// 创建一个新的 Hotkey，仅指定主键，所有修饰键默认为 false。
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    /// 启用 Ctrl 修饰键。
    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// 启用 Alt 修饰键。
    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// 启用 Shift 修饰键。
    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// 启用 Meta/Win 修饰键。
    pub fn with_meta(mut self) -> Self {
        self.meta = true;
        self
    }
}

/// 按键事件类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HotkeyEvent {
    /// 全局快捷键按下
    GlobalHotkey(Hotkey),
    /// 双击 Ctrl
    DoubleCtrl,
}

/// 事件过滤器，用于回调注册时指定关注的事件类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyEventFilter {
    /// 所有事件
    All,
    /// 特定快捷键
    GlobalHotkey(Hotkey),
    /// 双击 Ctrl
    DoubleCtrl,
}

/// 按键回调函数类型。
pub type HotkeyCallback = Arc<dyn Fn(HotkeyEvent) + Send + Sync>;

/// 回调注册信息。
pub struct CallbackRegistration {
    /// 回调 ID（用于注销）
    pub id: String,
    /// 关注的事件类型
    pub filter: HotkeyEventFilter,
    /// 回调函数
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
    /// 快捷键定义
    pub hotkey: Hotkey,
}

/// 按键监听配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HotkeyConfig {
    /// 全局快捷键列表
    pub hotkeys: Vec<HotkeyRegistration>,
    /// 是否启用双击 Ctrl
    pub double_ctrl_enabled: bool,
}
