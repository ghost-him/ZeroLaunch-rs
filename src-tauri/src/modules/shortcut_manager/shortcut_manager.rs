use crate::notify;
use crate::utils::service_locator::ServiceLocator;
use crate::utils::ui_controller::handle_pressed;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut as TauriShortcut, ShortcutState,
};
use tracing::warn;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shortcut {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl Shortcut {
    pub fn new() -> Self {
        Shortcut {
            key: "".to_string(),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }
}

type ShortcutCallback = Box<dyn Fn(&tauri::AppHandle) + Send + Sync>;

struct ShortcutManagerInner {
    shortcuts: Arc<Mutex<HashMap<TauriShortcut, ShortcutCallback>>>,
    app_handle: Arc<AppHandle>,
}

impl ShortcutManagerInner {
    pub fn new(app_handle: Arc<AppHandle>) -> Self {
        ShortcutManagerInner {
            shortcuts: Arc::new(Mutex::new(HashMap::new())),
            app_handle: app_handle,
        }
    }

    // 将自定义Shortcut转换为Tauri的Shortcut
    fn convert_shortcut(&self, shortcut: &Shortcut) -> Result<TauriShortcut, String> {
        let code = {
            // 如果是单个字符，则使用单个字符的方式处理
            if shortcut.key.len() == 1 {
                let first_char = shortcut.key.chars().next().unwrap();
                match first_char {
                    'a'..='z' | 'A'..='Z' => {
                        // 统一转换为大写字母，然后计算偏移量
                        let uppercase = first_char.to_uppercase().next().unwrap();
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
                // 如果是多个字符，则额外处理
                match shortcut.key.as_str() {
                    "Space" => Some(Code::Space),
                    "Tab" => Some(Code::Tab),
                    "CapsLock" => Some(Code::CapsLock),
                    _ => None,
                }
            }
        };

        // 检查是否找到了有效的键码
        let code = {
            if code.is_none() {
                return Err(format!("无效的按键: {:?}", shortcut.key));
            } else {
                code.unwrap()
            }
        };

        let mut modifiers = None;
        if shortcut.ctrl || shortcut.alt || shortcut.shift || shortcut.meta {
            let mut mods = Modifiers::empty();
            if shortcut.ctrl {
                mods |= Modifiers::CONTROL;
            }
            if shortcut.alt {
                mods |= Modifiers::ALT;
            }
            if shortcut.shift {
                mods |= Modifiers::SHIFT;
            }
            if shortcut.meta {
                mods |= Modifiers::META;
            }
            modifiers = Some(mods);
        }

        Ok(TauriShortcut::new(modifiers, code))
    }

    // 注册快捷键和回调函数
    pub fn register_shortcut<F>(&self, shortcut: Shortcut, callback: F) -> Result<(), String>
    where
        F: Fn(&tauri::AppHandle) + Send + Sync + 'static,
    {
        let tauri_shortcut = self.convert_shortcut(&shortcut)?;
        let mut shortcuts = self.shortcuts.lock();

        shortcuts.insert(tauri_shortcut.clone(), Box::new(callback));

        // 直接注册到全局快捷键系统
        if let Err(e) = self
            .app_handle
            .global_shortcut()
            .register(tauri_shortcut.clone())
        {
            notify("ZeroLaunch-rs", &format!("注册快捷键失败: {:?}", e));
            return Err(format!("注册快捷键失败: {:?}", e));
        }

        Ok(())
    }

    /// 删除指定ID的快捷键
    pub fn delete_all_shortcut(&self) -> Result<(), String> {
        self.unregister_all_shortcut();
        let mut shortcuts = self.shortcuts.lock();
        shortcuts.clear();
        Ok(())
    }

    /// 取消注册所有的快捷键（用于游戏模式）
    pub fn unregister_all_shortcut(&self) -> Result<(), String> {
        if let Err(e) = self.app_handle.global_shortcut().unregister_all() {
            println!("取消注册失败: {:?}", e);
            notify("ZeroLaunch-rs", &format!("取消注册失败: {:?}", e));
        }
        Ok(())
    }

    /// 注册所有快捷键到全局快捷键管理器
    pub fn register_all_shortcuts(&self) -> Result<(), String> {
        let shortcuts = self.shortcuts.lock();
        for shortcut in shortcuts.keys() {
            if let Err(e) = self.app_handle.global_shortcut().register(shortcut.clone()) {
                println!("注册快捷键失败: {:?}", e);
                notify("ZeroLaunch-rs", &format!("注册快捷键失败: {:?}", e));
            }
        }
        Ok(())
    }

    // 初始化快捷键监听器
    pub fn init_shortcut_listener(&self) -> Result<(), Box<dyn std::error::Error>> {
        let shortcuts_clone = Arc::clone(&self.shortcuts);

        self.app_handle.plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    let shortcuts = shortcuts_clone.lock();
                    if let Some(callback) = shortcuts.get(shortcut) {
                        if let ShortcutState::Pressed = event.state() {
                            callback(app);
                        }
                    }
                })
                .build(),
        )?;

        Ok(())
    }
}

pub struct ShortcutManager {
    inner: Mutex<ShortcutManagerInner>,
}

impl ShortcutManager {
    pub fn new(app_handle: Arc<AppHandle>) -> Self {
        ShortcutManager {
            inner: Mutex::new(ShortcutManagerInner::new(app_handle)),
        }
    }

    pub fn register_shortcut<F>(&self, shortcut: Shortcut, callback: F) -> Result<(), String>
    where
        F: Fn(&tauri::AppHandle) + Send + Sync + 'static,
    {
        self.inner.lock().register_shortcut(shortcut, callback)
    }

    pub fn delete_all_shortcut(&self) -> Result<(), String> {
        self.inner.lock().delete_all_shortcut()
    }

    pub fn init_shortcut_listener(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.lock().init_shortcut_listener()
    }

    pub fn register_all_shortcuts(&self) -> Result<(), String> {
        self.inner.lock().register_all_shortcuts()
    }

    pub fn unregister_all_shortcut(&self) -> Result<(), String> {
        self.inner.lock().unregister_all_shortcut()
    }
}

pub fn start_shortcut_manager(app: &mut tauri::App) {
    let state = ServiceLocator::get_state();
    let app_handle = app.handle();
    let shortcut_manager: ShortcutManager = ShortcutManager::new(Arc::new(app_handle.clone()));
    if let Err(e) = shortcut_manager.init_shortcut_listener() {
        warn!("初始化失败:{:?}", e);
        notify("ZeroLaunch-rs", &format!("键盘监听器初始化失败：{:?}", e));
    }
    state.set_shortcut_manager(Arc::new(shortcut_manager));
    update_shortcut_manager();
}

pub fn update_shortcut_manager() {
    let state = ServiceLocator::get_state();
    if state.get_game_mode() {
        return;
    }

    let shortcut_manager = state.get_shortcut_manager().unwrap();

    if let Err(e) = shortcut_manager.delete_all_shortcut() {
        println!("{:?}", e);
        warn!("{:?}", e);
        return;
    }

    let runtime_config = state.get_runtime_config().unwrap();
    let shortcut_config = runtime_config.get_shortcut_config();
    let shortcut: Shortcut = shortcut_config.get_open_search_bar();
    if let Err(e) = shortcut_manager.register_shortcut(shortcut, move |handle| {
        handle_pressed(handle);
    }) {
        warn!("注册快捷键失败 {:?}", e);
        notify("ZeroLaunch-rs", &format!("注册快捷键失败 {:?}", e));
    }
}
