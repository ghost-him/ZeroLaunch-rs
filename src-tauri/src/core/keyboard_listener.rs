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
use tracing::error;
use tracing::warn;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shortcut {
    key: String,
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShortcutUnit {
    id: String,
    shortcut: Shortcut,
}

impl Default for Shortcut {
    fn default() -> Self {
        Shortcut {
            key: "Space".to_string(),
            ctrl: false,
            alt: true,
            shift: false,
            meta: false,
        }
    }
}

type ShortcutCallback = Box<dyn Fn(&tauri::AppHandle) + Send + Sync>;

struct ShortcutManagerInner {
    shortcuts: Arc<Mutex<HashMap<TauriShortcut, ShortcutCallback>>>,
    id_to_shortcut: Arc<Mutex<HashMap<String, TauriShortcut>>>,
    app_handle: Arc<AppHandle>,
    game_mode: Arc<Mutex<bool>>,
}

impl ShortcutManagerInner {
    pub fn new(app_handle: Arc<AppHandle>) -> Self {
        ShortcutManagerInner {
            shortcuts: Arc::new(Mutex::new(HashMap::new())),
            id_to_shortcut: Arc::new(Mutex::new(HashMap::new())),
            app_handle: app_handle,
            game_mode: Arc::new(Mutex::new(false)),
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
    pub fn register_shortcut<F>(
        &self,
        id: String,
        shortcut: Shortcut,
        callback: F,
    ) -> Result<(), String>
    where
        F: Fn(&tauri::AppHandle) + Send + Sync + 'static,
    {
        let game_mode = self.game_mode.lock();
        if *game_mode {
            return Err("当前为游戏模式，请退出后重试".to_string());
        }

        let tauri_shortcut = self.convert_shortcut(&shortcut)?;
        let mut shortcuts = self.shortcuts.lock();
        let mut id_to_shortcut = self.id_to_shortcut.lock();

        shortcuts.insert(tauri_shortcut.clone(), Box::new(callback));
        id_to_shortcut.insert(id, tauri_shortcut);

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

    // 更新快捷键
    pub fn update_shortcut<F>(
        &self,
        id: String,
        new_shortcut: Shortcut,
        callback: F,
    ) -> Result<(), String>
    where
        F: Fn(&tauri::AppHandle) + Send + Sync + 'static,
    {
        let game_mode = self.game_mode.lock();
        if *game_mode {
            return Err("当前为游戏模式，请退出后重试".to_string());
        }
        let new_tauri_shortcut = self.convert_shortcut(&new_shortcut)?;
        let mut id_to_shortcut = self.id_to_shortcut.lock();

        let old_tauri_shortcut = match id_to_shortcut.get(&id) {
            Some(shortcut) => shortcut.clone(),
            None => return Err(format!("找不到ID为 {} 的快捷键", id)),
        };

        // 先尝试取消注册旧快捷键
        if let Err(e) = self
            .app_handle
            .global_shortcut()
            .unregister(old_tauri_shortcut.clone())
        {
            error!("取消注册快捷键失败: {:?}", e);
            // 继续执行，因为可能是之前没有成功注册
        }

        let mut shortcuts = self.shortcuts.lock();
        shortcuts.remove(&old_tauri_shortcut); // 使用引用
        shortcuts.insert(new_tauri_shortcut.clone(), Box::new(callback));
        id_to_shortcut.insert(id, new_tauri_shortcut);

        // 注册新快捷键
        if let Err(e) = self
            .app_handle
            .global_shortcut()
            .register(new_tauri_shortcut.clone())
        {
            notify("ZeroLaunch-rs", &format!("注册新快捷键失败: {:?}", e));
            return Err(format!("注册新快捷键失败: {:?}", e));
        }

        Ok(())
    }

    /// 删除指定ID的快捷键
    pub fn delete_shortcut(&self, id: &str) -> Result<(), String> {
        let game_mode = self.game_mode.lock();
        if *game_mode {
            return Err("当前为游戏模式，请退出后重试".to_string());
        }
        // 获取快捷键映射的锁
        let mut id_to_shortcut = self.id_to_shortcut.lock();

        // 查找并移除指定ID的快捷键
        let tauri_shortcut = id_to_shortcut
            .remove(id)
            .ok_or_else(|| format!("找不到ID为 {} 的快捷键", id))?;

        // 从全局快捷键系统中取消注册
        if let Err(e) = self
            .app_handle
            .global_shortcut()
            .unregister(tauri_shortcut.clone())
        {
            error!("取消注册快捷键失败: {:?}", e);
            // 即使取消注册失败，我们仍然从内存中移除快捷键
        }

        // 从快捷键回调映射中移除
        let mut shortcuts = self.shortcuts.lock();
        shortcuts.remove(&tauri_shortcut);
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

    /// 打开或关闭游戏模式
    pub fn switch_game_mode(&self, game_mode: bool) {
        let mut guard = self.game_mode.lock();
        *guard = game_mode;

        if game_mode {
            // 如果打开了游戏模式
            let _ = self.unregister_all_shortcut();
        } else {
            // 如果关闭游戏模式
            let _ = self.register_all_shortcuts();
        }
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

    /// 将Tauri的Shortcut转换回自定义的Shortcut结构
    fn convert_from_tauri_shortcut(&self, shortcut: &TauriShortcut) -> Shortcut {
        let code_str = format!("{:?}", shortcut.key);
        let modifiers = shortcut.mods;

        Shortcut {
            key: code_str,
            ctrl: modifiers.contains(Modifiers::CONTROL),
            alt: modifiers.contains(Modifiers::ALT),
            shift: modifiers.contains(Modifiers::SHIFT),
            meta: modifiers.contains(Modifiers::META),
        }
    }

    /// 获取所有已注册的快捷键
    pub fn get_all_shortcuts(&self) -> Vec<ShortcutUnit> {
        let id_to_shortcut = self.id_to_shortcut.lock();
        id_to_shortcut
            .iter()
            .map(|(id, tauri_shortcut)| {
                let shortcut = self.convert_from_tauri_shortcut(tauri_shortcut);
                ShortcutUnit {
                    id: id.clone(),
                    shortcut,
                }
            })
            .collect()
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

    pub fn register_shortcut<F>(
        &self,
        id: String,
        shortcut: Shortcut,
        callback: F,
    ) -> Result<(), String>
    where
        F: Fn(&tauri::AppHandle) + Send + Sync + 'static,
    {
        self.inner.lock().register_shortcut(id, shortcut, callback)
    }

    pub fn update_shortcut<F>(
        &self,
        id: String,
        new_shortcut: Shortcut,
        callback: F,
    ) -> Result<(), String>
    where
        F: Fn(&tauri::AppHandle) + Send + Sync + 'static,
    {
        self.inner
            .lock()
            .update_shortcut(id, new_shortcut, callback)
    }

    pub fn delete_shortcut(&self, id: &str) -> Result<(), String> {
        self.inner.lock().delete_shortcut(id)
    }

    pub fn init_shortcut_listener(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.lock().init_shortcut_listener()
    }

    pub fn register_all_shortcuts(&self) -> Result<(), String> {
        self.inner.lock().register_all_shortcuts()
    }

    pub fn get_all_shortcuts(&self) -> Vec<ShortcutUnit> {
        self.inner.lock().get_all_shortcuts()
    }

    /// 打开或关闭游戏模式
    pub fn switch_game_mode(&self, game_mode: bool) {
        self.inner.lock().switch_game_mode(game_mode);
    }
}

pub fn start_key_listener(app: &mut tauri::App) {
    let state = ServiceLocator::get_state();

    let runtime_config = state.get_runtime_config().unwrap();
    let app_config = runtime_config.get_app_config();
    let shortcut = app_config.get_shortcut();
    let app_handle = app.handle();
    let shortcut_manager = ShortcutManager::new(Arc::new(app_handle.clone()));
    if let Err(e) = shortcut_manager.init_shortcut_listener() {
        warn!("初始化失败:{:?}", e);
        notify("ZeroLaunch-rs", &format!("键盘监听器初始化失败：{:?}", e));
    }
    if let Err(e) =
        shortcut_manager.register_shortcut("show_search_bar".to_string(), shortcut, move |handle| {
            handle_pressed(handle);
        })
    {
        warn!("注册快捷键失败 {:?}", e);
        notify("ZeroLaunch-rs", &format!("注册快捷键失败 {:?}", e));
    }

    state.set_shortcut_manager(Arc::new(shortcut_manager));
}
