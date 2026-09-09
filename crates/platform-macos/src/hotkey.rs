use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::hotkey::{
    CallbackRegistration, Hotkey, HotkeyCallback, HotkeyEvent, HotkeyEventFilter, HotkeyManager,
};
/// Registers global macOS shortcuts and dispatches matching events.
///
/// Used by the core hotkey configuration component through the platform abstraction.
pub struct MacosHotkeyManager {
    /// Tauri application handle used to access the global-shortcut plugin.
    app: Arc<AppHandle>,
    /// Callback registrations keyed by their stable service identifier.
    callbacks: Arc<DashMap<String, CallbackRegistration>>,
    /// Whether the optional double-Control listener is active.
    listening: AtomicBool,
}
impl MacosHotkeyManager {
    pub fn new(app: Arc<AppHandle>) -> Self {
        let _ = app.plugin(tauri_plugin_global_shortcut::Builder::new().build());
        Self {
            app,
            callbacks: Arc::new(DashMap::new()),
            listening: AtomicBool::new(false),
        }
    }
    fn shortcut(hotkey: &Hotkey) -> Result<Shortcut, HostApiError> {
        let code = match hotkey.key.as_str() {
            "Space" => Code::Space,
            "Tab" => Code::Tab,
            "CapsLock" => Code::CapsLock,
            key if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() => {
                match key.as_bytes()[0] {
                    b'0' => Code::Digit0,
                    b'1' => Code::Digit1,
                    b'2' => Code::Digit2,
                    b'3' => Code::Digit3,
                    b'4' => Code::Digit4,
                    b'5' => Code::Digit5,
                    b'6' => Code::Digit6,
                    b'7' => Code::Digit7,
                    b'8' => Code::Digit8,
                    b'9' => Code::Digit9,
                    _ => unreachable!(),
                }
            }
            key if key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic() => {
                match key.to_ascii_uppercase().as_str() {
                    "A" => Code::KeyA,
                    "B" => Code::KeyB,
                    "C" => Code::KeyC,
                    "D" => Code::KeyD,
                    "E" => Code::KeyE,
                    "F" => Code::KeyF,
                    "G" => Code::KeyG,
                    "H" => Code::KeyH,
                    "I" => Code::KeyI,
                    "J" => Code::KeyJ,
                    "K" => Code::KeyK,
                    "L" => Code::KeyL,
                    "M" => Code::KeyM,
                    "N" => Code::KeyN,
                    "O" => Code::KeyO,
                    "P" => Code::KeyP,
                    "Q" => Code::KeyQ,
                    "R" => Code::KeyR,
                    "S" => Code::KeyS,
                    "T" => Code::KeyT,
                    "U" => Code::KeyU,
                    "V" => Code::KeyV,
                    "W" => Code::KeyW,
                    "X" => Code::KeyX,
                    "Y" => Code::KeyY,
                    "Z" => Code::KeyZ,
                    _ => unreachable!(),
                }
            }
            _ => {
                return Err(HostApiError::ExecutionFailed {
                    service: "hotkey".into(),
                    reason: format!("unsupported key: {}", hotkey.key),
                })
            }
        };
        let mut modifiers = Modifiers::empty();
        if hotkey.ctrl {
            modifiers |= Modifiers::CONTROL;
        }
        if hotkey.alt {
            modifiers |= Modifiers::ALT;
        }
        if hotkey.shift {
            modifiers |= Modifiers::SHIFT;
        }
        if hotkey.meta {
            modifiers |= Modifiers::META;
        }
        Ok(Shortcut::new(Some(modifiers), code))
    }
    fn dispatch(callbacks: &DashMap<String, CallbackRegistration>, event: HotkeyEvent) {
        for entry in callbacks.iter() {
            let should_dispatch = match (&entry.filter, &event) {
                (HotkeyEventFilter::All, _) => true,
                (HotkeyEventFilter::DoubleCtrl, HotkeyEvent::DoubleCtrl) => true,
                (HotkeyEventFilter::GlobalHotkey(expected), HotkeyEvent::GlobalHotkey(actual)) => {
                    expected == actual
                }
                _ => false,
            };
            if should_dispatch {
                (entry.callback)(event.clone());
            }
        }
    }
}
#[async_trait]
impl HotkeyManager for MacosHotkeyManager {
    async fn register_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError> {
        let shortcut = Self::shortcut(hotkey)?;
        let callbacks = self.callbacks.clone();
        let registered = hotkey.clone();
        self.app
            .global_shortcut()
            .on_shortcut(shortcut, move |_, _, event| {
                if event.state() == ShortcutState::Pressed {
                    MacosHotkeyManager::dispatch(
                        &callbacks,
                        HotkeyEvent::GlobalHotkey(registered.clone()),
                    );
                }
            })
            .map_err(|e| HostApiError::ExecutionFailed {
                service: "hotkey".into(),
                reason: e.to_string(),
            })
    }
    async fn unregister_hotkey(&self, hotkey: &Hotkey) -> Result<(), HostApiError> {
        self.app
            .global_shortcut()
            .unregister(Self::shortcut(hotkey)?)
            .map_err(|e| HostApiError::ExecutionFailed {
                service: "hotkey".into(),
                reason: e.to_string(),
            })
    }
    async fn unregister_all(&self) -> Result<(), HostApiError> {
        self.app
            .global_shortcut()
            .unregister_all()
            .map_err(|e| HostApiError::ExecutionFailed {
                service: "hotkey".into(),
                reason: e.to_string(),
            })
    }
    async fn set_double_ctrl_enabled(&self, enabled: bool) -> Result<(), HostApiError> {
        if enabled {
            return Err(HostApiError::ExecutionFailed {
                service: "hotkey".into(),
                reason: "double Ctrl requires Input Monitoring permission and is not implemented"
                    .into(),
            });
        }
        Ok(())
    }
    async fn start_listening(&self) -> Result<(), HostApiError> {
        self.listening.store(true, Ordering::Relaxed);
        Ok(())
    }
    async fn stop_listening(&self) -> Result<(), HostApiError> {
        self.listening.store(false, Ordering::Relaxed);
        Ok(())
    }
    fn is_listening(&self) -> bool {
        self.listening.load(Ordering::Relaxed)
    }
    fn register_callback(&self, id: &str, filter: HotkeyEventFilter, callback: HotkeyCallback) {
        self.callbacks.insert(
            id.into(),
            CallbackRegistration {
                id: id.into(),
                filter,
                callback,
            },
        );
    }
    fn unregister_callback(&self, id: &str) {
        self.callbacks.remove(id);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_shortcut_key() {
        assert!(MacosHotkeyManager::shortcut(&Hotkey::new("Invalid")).is_err());
    }
}
