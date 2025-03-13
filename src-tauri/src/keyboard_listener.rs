use crate::utils::{notify::notify, ui_controller::handle_pressed};
use parking_lot::Mutex;
use rdev::{listen, Event, EventType, Key};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing::error;
// 初始化全局快捷键监听器
fn init_key_listener(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let alt_space_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, shortcut, event| {
                if shortcut == &alt_space_shortcut {
                    match event.state() {
                        ShortcutState::Pressed => {}
                        ShortcutState::Released => {}
                    }
                }
            })
            .build(),
    )?;

    Ok(())
}

// 注册快捷键
fn register_shortcut(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let alt_space_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
    app.global_shortcut().register(alt_space_shortcut)?;

    Ok(())
}

// 启动键盘监听器
fn start_keyboard_listener(app: &tauri::AppHandle) {
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let pressed_keys = Arc::new(Mutex::new(HashSet::new()));
            let pressed_keys_clone: Arc<Mutex<HashSet<Key>>> = Arc::clone(&pressed_keys);
            let callback = move |event: Event| {
                let mut keys = pressed_keys_clone.lock();

                match event.event_type {
                    EventType::KeyPress(key) => {
                        keys.insert(key);

                        if keys.contains(&Key::Alt) && keys.contains(&Key::Space) {
                            handle_pressed(&app_handle);
                            keys.clear();
                        }
                    }
                    EventType::KeyRelease(key) => {
                        keys.remove(&key);
                    }
                    _ => {}
                }
            };

            if let Err(error) = listen(callback) {
                error!("监听器启动失败: {:?}", error);
                notify(
                    "ZeroLaunch-rs",
                    &format!("初始化监听器失败，需要重启程序，错误原因：{:?}", error),
                );
                std::process::exit(1);
            }
        });
    });
}

// 主函数，用于启动键盘监听
pub fn start_key_listener(app: &mut tauri::App) {
    // 初始化监听器
    if let Err(e) = init_key_listener(app) {
        notify(
            "ZeroLaunch-rs",
            &format!("初始化监听器失败，需要重启程序，错误原因：{:?}", e),
        );
        error!("初始化快捷键监听器失败: {:?}", e);
        std::process::exit(1);
    }

    // 注册快捷键
    if let Err(e) = register_shortcut(app.app_handle()) {
        notify(
            "ZeroLaunch-rs",
            &format!(
                "按键 Alt + Space 绑定失败，请尝试重新注册，错误原因：{:?}",
                e
            ),
        );
        error!("按键 Alt + Space 绑定失败: {:?}", e);
        return;
    }

    // 启动键盘监听器
    start_keyboard_listener(app.handle());
}

// 重新注册快捷键的函数
pub fn retry_register_shortcut(app: &tauri::AppHandle) -> bool {
    let alt_space_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
    // 尝试取消注册，忽略可能的错误（可能本来就没注册成功）
    let _ = app.global_shortcut().unregister(alt_space_shortcut);
    // 重新注册
    if let Err(error) = register_shortcut(app) {
        notify(
            "ZeroLaunch-rs",
            &format!(
                "按键 Alt + Space 绑定失败，请尝试重新注册，错误原因：{:?}",
                error
            ),
        );
    } else {
        start_keyboard_listener(app);
        // 显示成功通知
        notify("ZeroLaunch-rs", "按键 Alt + Space 重新绑定成功");
        return true;
    }
    return false;
}
