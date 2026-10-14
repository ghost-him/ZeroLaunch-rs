use serde::{Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};

/// 按键组合定义 —— 一次全局快捷键的完整形态（主键 + 修饰键）。
///
/// 经 `plugin/hotkey.event`（GlobalHotkey 变体平铺字段）与
/// `host/hotkey.subscribe` 过滤器载荷传输；进程内经 HotkeyManager 注册监听。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Hotkey {
    /// 主键（如 "Space", "A", "F1"）
    #[serde(rename = "key")]
    pub key: String,
    /// Ctrl 修饰键
    #[serde(rename = "ctrl")]
    pub ctrl: bool,
    /// Alt 修饰键
    #[serde(rename = "alt")]
    pub alt: bool,
    /// Shift 修饰键
    #[serde(rename = "shift")]
    pub shift: bool,
    /// Meta/Win 修饰键
    #[serde(rename = "meta")]
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
/// 跨进程作为 `plugin/hotkey.event` 载荷字段 `event` 推送（内部 tag "kind"；
/// 全局快捷键变体把 Hotkey 字段平铺进同一对象，如
/// `{"kind":"globalHotkey","key":"Space","ctrl":true,...}`）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum HotkeyEvent {
    /// 全局快捷键被按下（标签 "globalHotkey"，内容为触发的 Hotkey）。
    #[serde(rename = "globalHotkey")]
    GlobalHotkey(Hotkey),
    /// 双击 Ctrl 被触发（标签 "doubleCtrl"）。
    #[serde(rename = "doubleCtrl")]
    DoubleCtrl,
}

/// 事件过滤器，用于回调注册时指定关注的事件类型。
/// 跨进程作为 `host/hotkey.subscribe` 的 `filter` 字段传输，标签词表与 HotkeyEvent 一致。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum HotkeyEventFilter {
    /// 关注全部按键事件（标签 "all"）。
    #[serde(rename = "all")]
    All,
    /// 仅关注指定快捷键（标签 "globalHotkey"）。
    #[serde(rename = "globalHotkey")]
    GlobalHotkey(Hotkey),
    /// 仅关注双击 Ctrl（标签 "doubleCtrl"）。
    #[serde(rename = "doubleCtrl")]
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

/// 单个快捷键注册信息 —— 一条待监听的全局快捷键。
///
/// 仅进程内流转：由 `HotkeyConfigComponent::settings_to_hotkey_config` 构造，
/// 经 `HostApi::apply_hotkey_config`（inherent 方法，非 PluginHost 契约）应用；
/// 不跨 host/* RPC，不进前端契约。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyRegistration {
    /// 待注册监听的按键组合。
    #[serde(rename = "hotkey")]
    pub hotkey: Hotkey,
}

/// 按键监听配置 —— 本机全局快捷键与双击 Ctrl 的完整配置快照。
///
/// 仅进程内流转：由 `HotkeyConfigComponent::settings_to_hotkey_config` 构造，
/// 经 `HostApi::apply_hotkey_config` 应用；不跨 host/* RPC，不进前端契约。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HotkeyConfig {
    /// 待注册的快捷键列表（可为空，即只配双击 Ctrl）。
    #[serde(rename = "hotkeys")]
    pub hotkeys: Vec<HotkeyRegistration>,
    /// 是否启用双击 Ctrl 监听。
    #[serde(rename = "doubleCtrlEnabled")]
    pub double_ctrl_enabled: bool,
}
