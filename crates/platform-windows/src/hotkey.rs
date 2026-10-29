use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut as TauriShortcut, ShortcutState,
};
use tracing::{debug, info, warn};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LCONTROL, VK_RCONTROL};
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::hotkey::{
    CallbackRegistration, Hotkey, HotkeyCallback, HotkeyEvent, HotkeyEventFilter, HotkeyManager,
};

use crate::raw_keyboard::{RawKeyEvent, RawKeyboardListener};

/// 双击 Ctrl 判定窗口：两次 Ctrl 按下需落在此间隔内。
const DOUBLE_CTRL_WINDOW: Duration = Duration::from_millis(400);
/// 判定为「Ctrl」的虚拟键码。
/// 原始输入（RAWKEYBOARD.VKey）上报的是通用 VK_CONTROL（0x11，由扫描码映射而来），
/// 左右区分码只在消息层出现，故两者都接受。
const DOUBLE_CTRL_KEYS: [u16; 3] = [0x11, VK_LCONTROL.0, VK_RCONTROL.0];

/// Windows 平台按键管理器实现。
/// 使用 `tauri-plugin-global-shortcut` 注册全局快捷键；双击 Ctrl 用原始输入（RIDEV_INPUTSINK）
/// 监听，只在功能启用且监听开启时读取全局按键，不安装任何输入钩子。
pub struct WindowsHotkeyManager {
    app_handle: Arc<AppHandle>,
    /// 事件回调，通过 Arc<RwLock> 支持动态更新
    event_callback: Arc<RwLock<Option<HotkeyCallback>>>,
    /// 双击 Ctrl 监听是否启用
    double_ctrl_enabled: AtomicBool,
    /// 是否正在监听
    is_listening: Arc<AtomicBool>,
    /// 双击 Ctrl 的原始输入监听器；未启用时为 None，进程内不读取任何全局输入
    double_ctrl_listener: Mutex<Option<RawKeyboardListener>>,
    /// 已注册的全局快捷键（注册状态由本管理器独占维护，用于失败回滚）
    registered: Mutex<Vec<Hotkey>>,
    /// 上层回调注册表
    callbacks: Arc<DashMap<String, CallbackRegistration>>,
}

/// 将 Hotkey 的主键名映射为 Tauri（keyboard_types::Code）的按键码。
/// 单字符按键按录制端约定归一：字母 → `Key{x}`，数字 → `Digit{x}`；其余按 Code 名称解析。
/// 参数：key - 主键名（如 "A"/"1"/"Space"/"F5"/"ArrowUp"）。
/// 返回：可识别的按键码，未知键名返回 None。
fn parse_key_code(key: &str) -> Option<Code> {
    let mut chars = key.chars();
    let normalized = match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => format!("Key{}", c.to_ascii_uppercase()),
        (Some(c), None) if c.is_ascii_digit() => format!("Digit{c}"),
        _ => key.to_string(),
    };
    normalized.parse::<Code>().ok()
}

/// 校验快捷键能否作为全局快捷键注册：必须至少包含一个修饰键，且主键可识别。
/// 无修饰键的按键会注册为系统级热键，劫持该键在全系统的输入，因此拒绝。
/// 参数：hotkey - 待校验的按键组合。
/// 返回：可用返回 Ok(())，否则返回 ExecutionFailed。
fn validate_global_hotkey(hotkey: &Hotkey) -> Result<(), HostApiError> {
    if !(hotkey.ctrl || hotkey.alt || hotkey.shift || hotkey.meta) {
        return Err(HostApiError::ExecutionFailed {
            service: "hotkey".to_string(),
            reason: "全局快捷键必须包含至少一个修饰键（Ctrl/Alt/Shift/Win）".to_string(),
        });
    }
    convert_hotkey(hotkey).map(|_| ())
}

/// 将 Hotkey 转换为 Tauri 的 Shortcut。
/// 参数：hotkey - 主键名 + 修饰键组合。
/// 返回：成功返回 Shortcut；键名不受支持时返回 ExecutionFailed。
fn convert_hotkey(hotkey: &Hotkey) -> Result<TauriShortcut, HostApiError> {
    let Some(code) = parse_key_code(&hotkey.key) else {
        return Err(HostApiError::ExecutionFailed {
            service: "hotkey".to_string(),
            reason: format!("无效的按键: {}", hotkey.key),
        });
    };

    let mut mods = Modifiers::empty();
    if hotkey.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if hotkey.alt {
        mods |= Modifiers::ALT;
    }
    if hotkey.shift {
        mods |= Modifiers::SHIFT;
    }
    if hotkey.meta {
        mods |= Modifiers::META;
    }
    let modifiers = (!mods.is_empty()).then_some(mods);

    Ok(TauriShortcut::new(modifiers, code))
}

impl WindowsHotkeyManager {
    /// 创建新的 WindowsHotkeyManager 实例。
    /// 构造时会尝试加载 `tauri-plugin-global-shortcut` plugin（如果尚未加载）。
    pub fn new(app_handle: Arc<AppHandle>) -> Self {
        // 尝试加载 plugin；若旧系统已加载则忽略错误
        let _ = app_handle.plugin(tauri_plugin_global_shortcut::Builder::new().build());

        Self {
            app_handle,
            event_callback: Arc::new(RwLock::new(None)),
            double_ctrl_enabled: AtomicBool::new(false),
            is_listening: Arc::new(AtomicBool::new(false)),
            double_ctrl_listener: Mutex::new(None),
            registered: Mutex::new(Vec::new()),
            callbacks: Arc::new(DashMap::new()),
        }
    }

    /// 将 Hotkey 转换为 Tauri 的 Shortcut。
    fn convert_to_tauri_shortcut(&self, hotkey: &Hotkey) -> Result<TauriShortcut, HostApiError> {
        convert_hotkey(hotkey)
    }

    /// 启动双击 Ctrl 的原始输入监听（已启动则忽略；仅在启用且处于监听态时启动）。
    /// 原始输入由系统异步投递，不介入输入管线、不修改前台应用键盘状态，也不需要低级钩子。
    /// 返回：无。启动失败只记录告警，不影响全局快捷键。
    fn start_double_ctrl_listener(&self) {
        if !self.double_ctrl_enabled.load(Ordering::Relaxed)
            || !self.is_listening.load(Ordering::Relaxed)
        {
            return;
        }
        let mut slot = self.double_ctrl_listener.lock();
        if slot.is_some() {
            return;
        }

        let callback_ref = self.event_callback.clone();
        let last_ctrl_press = Cell::new(Instant::now());
        let press_count = Cell::new(0u8);
        let last_key_was_release = Cell::new(false);

        let started = RawKeyboardListener::start(move |event: RawKeyEvent| {
            if !DOUBLE_CTRL_KEYS.contains(&event.vk) {
                // 其他按键按下时复位双击计数（与低级钩子实现一致）
                if event.is_down {
                    press_count.set(0);
                    last_key_was_release.set(false);
                }
                return;
            }
            if !event.is_down {
                last_key_was_release.set(true);
                return;
            }

            let now = Instant::now();
            if now.duration_since(last_ctrl_press.get()) < DOUBLE_CTRL_WINDOW {
                if press_count.get() == 1 && last_key_was_release.get() {
                    press_count.set(2);
                } else {
                    press_count.set(1);
                }
            } else {
                press_count.set(1);
            }
            last_ctrl_press.set(now);
            last_key_was_release.set(false);

            if press_count.get() == 2 {
                press_count.set(0);
                if let Some(callback) = callback_ref.read().as_ref() {
                    callback(HotkeyEvent::DoubleCtrl);
                }
            }
        });

        match started {
            Ok(listener) => {
                info!("双击 Ctrl 监听已启动（原始输入，无输入钩子）");
                *slot = Some(listener);
            }
            Err(e) => warn!("双击 Ctrl 监听启动失败: {e}"),
        }
    }

    /// 停止双击 Ctrl 的原始输入监听（注销原始输入注册并等待监听线程退出）。
    /// 返回：无。
    fn stop_double_ctrl_listener(&self) {
        if let Some(listener) = self.double_ctrl_listener.lock().take() {
            listener.stop();
            info!("双击 Ctrl 监听已停止");
        }
    }
}

#[async_trait]
impl HotkeyManager for WindowsHotkeyManager {
    /// 校验快捷键：键名可转换且至少包含一个修饰键。
    /// 无修饰键的按键会注册为系统级热键，劫持该键在全系统的输入，因此拒绝。
    async fn validate_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError> {
        validate_global_hotkey(hotkey)
    }

    async fn register_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError> {
        let tauri_shortcut = self.convert_to_tauri_shortcut(hotkey)?;
        let callback_ref = self.event_callback.clone();
        let hotkey_clone = hotkey.clone();

        let hotkey_debug = hotkey.clone();
        self.app_handle
            .global_shortcut()
            .on_shortcut(tauri_shortcut, move |_app, _shortcut, event| {
                if let ShortcutState::Pressed = event.state() {
                    debug!("全局快捷键按下: {:?}", hotkey_debug);
                    if let Some(callback) = callback_ref.read().as_ref() {
                        debug!("分发快捷键事件: {:?}", hotkey_debug);
                        callback(HotkeyEvent::GlobalHotkey(hotkey_clone.clone()));
                    } else {
                        warn!("快捷键按下但无事件分发器: {:?}", hotkey_debug);
                    }
                }
            })
            .map_err(|e| HostApiError::ExecutionFailed {
                service: "hotkey".to_string(),
                reason: format!("注册快捷键失败: {:?}", e),
            })?;

        self.registered.lock().push(hotkey.clone());
        Ok(())
    }

    async fn unregister_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError> {
        let tauri_shortcut = self.convert_to_tauri_shortcut(hotkey)?;
        let _ = self.app_handle.global_shortcut().unregister(tauri_shortcut);
        self.registered
            .lock()
            .retain(|registered| registered != hotkey);
        Ok(())
    }

    async fn unregister_all(&self) -> Result<(), HostApiError> {
        let _ = self.app_handle.global_shortcut().unregister_all();
        self.registered.lock().clear();
        Ok(())
    }

    /// 替换全部注册；任一注册失败时恢复替换前的注册集合，避免旧快捷键失效。
    async fn replace_all_hotkeys(&self, hotkeys: &[Hotkey]) -> Result<(), HostApiError> {
        let previous = self.registered.lock().clone();
        let _ = self.app_handle.global_shortcut().unregister_all();
        self.registered.lock().clear();

        for hotkey in hotkeys {
            if let Err(e) = self.register_hotkey(hotkey).await {
                let _ = self.app_handle.global_shortcut().unregister_all();
                self.registered.lock().clear();
                for old in &previous {
                    if let Err(restore_err) = self.register_hotkey(old).await {
                        warn!("恢复旧快捷键失败: {restore_err}");
                    }
                }
                return Err(e);
            }
        }
        Ok(())
    }

    async fn set_double_ctrl_enabled(&self, enabled: bool) -> Result<(), HostApiError> {
        self.double_ctrl_enabled.store(enabled, Ordering::Relaxed);
        if enabled {
            self.start_double_ctrl_listener();
        } else {
            // 关闭时立即注销原始输入：游戏模式下不再读取任何全局按键
            self.stop_double_ctrl_listener();
        }
        Ok(())
    }

    async fn start_listening(&self) -> Result<(), HostApiError> {
        let was_listening = self.is_listening.swap(true, Ordering::Relaxed);
        let callbacks = self.callbacks.clone();
        let callback_count = callbacks.len();
        info!("启动按键监听，已注册 {} 个回调", callback_count);
        let dispatcher: HotkeyCallback = Arc::new(move |event| {
            debug!("按键事件分发: {:?}, 回调总数: {}", event, callback_count);
            let mut matched = 0;
            for entry in callbacks.iter() {
                let registration = entry.value();
                if matches_filter(&registration.filter, &event) {
                    debug!("  匹配回调: {}", registration.id);
                    (registration.callback)(event.clone());
                    matched += 1;
                }
            }
            if matched == 0 {
                debug!("  无匹配回调");
            }
        });
        *self.event_callback.write() = Some(dispatcher);
        // 分发器就绪后再启动双击 Ctrl 监听，避免启动瞬间的事件无人接收
        self.start_double_ctrl_listener();
        if was_listening {
            info!("更新按键分发器");
        } else {
            info!("首次启动按键监听");
        }
        Ok(())
    }

    async fn stop_listening(&self) -> Result<(), HostApiError> {
        *self.event_callback.write() = None;
        self.is_listening.store(false, Ordering::Relaxed);
        self.stop_double_ctrl_listener();
        Ok(())
    }

    fn is_listening(&self) -> bool {
        self.is_listening.load(Ordering::Relaxed)
    }

    fn register_callback(&self, id: &str, filter: HotkeyEventFilter, callback: HotkeyCallback) {
        self.callbacks.insert(
            id.to_string(),
            CallbackRegistration {
                id: id.to_string(),
                filter,
                callback,
            },
        );
    }

    fn unregister_callback(&self, id: &str) {
        self.callbacks.remove(id);
    }
}

fn matches_filter(filter: &HotkeyEventFilter, event: &HotkeyEvent) -> bool {
    match (filter, event) {
        (HotkeyEventFilter::All, _) => true,
        (
            HotkeyEventFilter::GlobalHotkey(filter_hotkey),
            HotkeyEvent::GlobalHotkey(event_hotkey),
        ) => filter_hotkey == event_hotkey,
        (HotkeyEventFilter::DoubleCtrl, HotkeyEvent::DoubleCtrl) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 录制端可产出的键名都能映射为按键码。
    #[test]
    fn parse_key_code_covers_recorded_keys() {
        assert_eq!(parse_key_code("A"), Some(Code::KeyA));
        assert_eq!(parse_key_code("z"), Some(Code::KeyZ));
        assert_eq!(parse_key_code("7"), Some(Code::Digit7));
        for named in [
            "Space",
            "Tab",
            "CapsLock",
            "Enter",
            "ArrowUp",
            "ArrowDown",
            "ArrowLeft",
            "ArrowRight",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "Insert",
            "Delete",
            "F5",
            "F12",
        ] {
            assert!(parse_key_code(named).is_some(), "键名 {named} 应可转换");
        }
        assert_eq!(parse_key_code("Foo"), None);
    }

    /// 组合键与单键都能转换为 Shortcut（是否允许注册由 validate_hotkey 决定）。
    #[test]
    fn convert_hotkey_builds_shortcut() {
        let shortcut = convert_hotkey(&Hotkey::new("Space").with_alt()).unwrap();
        assert_eq!(shortcut.key, Code::Space);
        assert_eq!(shortcut.mods, Modifiers::ALT);

        let single = convert_hotkey(&Hotkey::new("K")).unwrap();
        assert_eq!(single.key, Code::KeyK);
        assert_eq!(single.mods, Modifiers::empty());
    }

    /// 全局快捷键必须有修饰键，且键名必须可识别。
    #[test]
    fn validate_global_hotkey_rejects_bare_key_and_unknown_key() {
        assert!(validate_global_hotkey(&Hotkey::new("Space").with_alt()).is_ok());
        assert!(validate_global_hotkey(&Hotkey::new("K")).is_err());
        assert!(validate_global_hotkey(&Hotkey::new("Enter")).is_err());
        assert!(validate_global_hotkey(&Hotkey::new("Foo").with_ctrl()).is_err());
        assert!(validate_global_hotkey(&Hotkey::new("K").with_ctrl()).is_ok());
        assert!(validate_global_hotkey(&Hotkey::new("F5").with_alt()).is_ok());
    }
}
