use crate::sdk::host_api::HostApiError;
use crate::sdk::hotkey::types::{
    CallbackRegistration, Hotkey, HotkeyCallback, HotkeyEvent, HotkeyEventFilter,
};
use crate::sdk::hotkey::HotkeyManager;
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use rdev::{listen, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut as TauriShortcut, ShortcutState,
};
use tracing::{debug, error, info, warn};

/// Windows 平台按键管理器实现。
/// 使用 `tauri-plugin-global-shortcut` 注册全局快捷键，使用 `rdev` 监听双击 Ctrl。
pub struct WindowsHotkeyManager {
    app_handle: Arc<AppHandle>,
    /// 事件回调，通过 Arc<RwLock> 支持动态更新
    event_callback: Arc<RwLock<Option<HotkeyCallback>>>,
    /// 双击 Ctrl 监听是否启用
    double_ctrl_enabled: Arc<AtomicBool>,
    /// 是否正在监听
    is_listening: Arc<AtomicBool>,
    /// 双击 Ctrl 监听线程是否已启动
    double_ctrl_thread_started: Arc<AtomicBool>,
    /// 上层回调注册表
    callbacks: Arc<DashMap<String, CallbackRegistration>>,
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
            double_ctrl_enabled: Arc::new(AtomicBool::new(false)),
            is_listening: Arc::new(AtomicBool::new(false)),
            double_ctrl_thread_started: Arc::new(AtomicBool::new(false)),
            callbacks: Arc::new(DashMap::new()),
        }
    }

    /// 将 Hotkey 转换为 Tauri 的 Shortcut。
    fn convert_to_tauri_shortcut(&self, hotkey: &Hotkey) -> Result<TauriShortcut, HostApiError> {
        let code = if hotkey.key.len() == 1 {
            let first_char =
                hotkey
                    .key
                    .chars()
                    .next()
                    .ok_or_else(|| HostApiError::ExecutionFailed {
                        service: "hotkey".to_string(),
                        reason: "快捷键字符串为空".to_string(),
                    })?;
            match first_char {
                'a'..='z' | 'A'..='Z' => {
                    let uppercase = first_char.to_uppercase().next().ok_or_else(|| {
                        HostApiError::ExecutionFailed {
                            service: "hotkey".to_string(),
                            reason: "字符转换大写失败".to_string(),
                        }
                    })?;
                    let offset = uppercase as u8 - b'A';
                    match offset {
                        0 => Some(Code::KeyA),
                        1 => Some(Code::KeyB),
                        2 => Some(Code::KeyC),
                        3 => Some(Code::KeyD),
                        4 => Some(Code::KeyE),
                        5 => Some(Code::KeyF),
                        6 => Some(Code::KeyG),
                        7 => Some(Code::KeyH),
                        8 => Some(Code::KeyI),
                        9 => Some(Code::KeyJ),
                        10 => Some(Code::KeyK),
                        11 => Some(Code::KeyL),
                        12 => Some(Code::KeyM),
                        13 => Some(Code::KeyN),
                        14 => Some(Code::KeyO),
                        15 => Some(Code::KeyP),
                        16 => Some(Code::KeyQ),
                        17 => Some(Code::KeyR),
                        18 => Some(Code::KeyS),
                        19 => Some(Code::KeyT),
                        20 => Some(Code::KeyU),
                        21 => Some(Code::KeyV),
                        22 => Some(Code::KeyW),
                        23 => Some(Code::KeyX),
                        24 => Some(Code::KeyY),
                        25 => Some(Code::KeyZ),
                        _ => None,
                    }
                }
                '0'..='9' => {
                    let offset = first_char as u8 - b'0';
                    match offset {
                        0 => Some(Code::Digit0),
                        1 => Some(Code::Digit1),
                        2 => Some(Code::Digit2),
                        3 => Some(Code::Digit3),
                        4 => Some(Code::Digit4),
                        5 => Some(Code::Digit5),
                        6 => Some(Code::Digit6),
                        7 => Some(Code::Digit7),
                        8 => Some(Code::Digit8),
                        9 => Some(Code::Digit9),
                        _ => None,
                    }
                }
                _ => None,
            }
        } else {
            match hotkey.key.as_str() {
                "Space" => Some(Code::Space),
                "Tab" => Some(Code::Tab),
                "CapsLock" => Some(Code::CapsLock),
                _ => None,
            }
        };

        let Some(code) = code else {
            return Err(HostApiError::ExecutionFailed {
                service: "hotkey".to_string(),
                reason: format!("无效的按键: {}", hotkey.key),
            });
        };

        let mut modifiers = None;
        if hotkey.ctrl || hotkey.alt || hotkey.shift || hotkey.meta {
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
            modifiers = Some(mods);
        }

        Ok(TauriShortcut::new(modifiers, code))
    }

    /// 启动双击 Ctrl 监听线程。
    fn start_double_ctrl_listener(&self) {
        if self
            .double_ctrl_thread_started
            .swap(true, Ordering::Relaxed)
        {
            return;
        }

        let callback_ref = self.event_callback.clone();
        let enabled_ref = self.double_ctrl_enabled.clone();

        thread::spawn(move || {
            info!("Starting Double Ctrl listener");
            let mut last_ctrl_press = Instant::now();
            let mut press_count = 0;
            let mut last_key_was_release = false;

            if let Err(error) = listen(move |event| {
                if !enabled_ref.load(Ordering::Relaxed) {
                    press_count = 0;
                    last_key_was_release = false;
                    return;
                }

                match event.event_type {
                    EventType::KeyPress(Key::ControlLeft)
                    | EventType::KeyPress(Key::ControlRight) => {
                        let now = Instant::now();
                        if now.duration_since(last_ctrl_press) < Duration::from_millis(400) {
                            if press_count == 1 && last_key_was_release {
                                press_count = 2;
                            } else {
                                press_count = 1;
                            }
                        } else {
                            press_count = 1;
                        }
                        last_ctrl_press = now;
                        last_key_was_release = false;

                        if press_count == 2 {
                            press_count = 0;
                            if let Some(callback) = callback_ref.read().as_ref() {
                                callback(HotkeyEvent::DoubleCtrl);
                            }
                        }
                    }
                    EventType::KeyRelease(Key::ControlLeft)
                    | EventType::KeyRelease(Key::ControlRight) => {
                        last_key_was_release = true;
                    }
                    EventType::KeyPress(_) => {
                        press_count = 0;
                        last_key_was_release = false;
                    }
                    _ => {}
                }
            }) {
                error!("Double Ctrl listener error: {:?}", error);
            }
        });
    }
}

#[async_trait]
impl HotkeyManager for WindowsHotkeyManager {
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

        Ok(())
    }

    async fn unregister_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError> {
        let tauri_shortcut = self.convert_to_tauri_shortcut(hotkey)?;
        let _ = self.app_handle.global_shortcut().unregister(tauri_shortcut);
        Ok(())
    }

    async fn unregister_all(&self) -> Result<(), HostApiError> {
        let _ = self.app_handle.global_shortcut().unregister_all();
        Ok(())
    }

    async fn set_double_ctrl_enabled(&self, enabled: bool) -> Result<(), HostApiError> {
        self.double_ctrl_enabled.store(enabled, Ordering::Relaxed);
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
        if !was_listening {
            info!("首次启动按键监听");
            self.start_double_ctrl_listener();
        } else {
            info!("更新按键分发器");
        }
        Ok(())
    }

    async fn stop_listening(&self) -> Result<(), HostApiError> {
        *self.event_callback.write() = None;
        self.is_listening.store(false, Ordering::Relaxed);
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
