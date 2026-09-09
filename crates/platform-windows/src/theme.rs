use std::sync::{Arc, OnceLock};

use tracing::error;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Registry::{
    RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, KEY_NOTIFY,
    REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, REG_SAM_FLAGS,
};
use windows::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject, INFINITE};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::{Theme, ThemeProvider};

/// Windows 系统主题提供器。
///
/// 读取 Windows 个性化注册表中的应用主题设置；仅由 platform-windows 注入 HostApi 使用。
#[derive(Default)]
pub struct WindowsThemeProvider;

impl WindowsThemeProvider {
    /// 创建主题提供器实例。
    pub fn new() -> Self {
        Self
    }
}

impl ThemeProvider for WindowsThemeProvider {
    /// 读取 AppsUseLightTheme，将 0 转换为深色，其余有效值转换为浅色。
    fn current_system_theme(&self) -> Result<Theme, HostApiError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
            .map_err(|error| HostApiError::ExecutionFailed {
                service: "theme".to_string(),
                reason: error.to_string(),
            })?;
        let value: u32 =
            key.get_value("AppsUseLightTheme")
                .map_err(|error| HostApiError::ExecutionFailed {
                    service: "theme".to_string(),
                    reason: error.to_string(),
                })?;
        Ok(if value == 0 {
            Theme::Dark
        } else {
            Theme::Light
        })
    }
}

/// 启动系统主题变化监控（进程内单例，重复调用仅首次生效）。
///
/// 由宿主启动流程注册回调；回调在监听线程执行（须快速返回）。
/// 监听线程运行至进程结束；启动失败时返回错误信息，由调用方记录。
/// crate 级 cfg（lib.rs）保证本实现仅随 Windows 平台编译。
pub fn start_system_theme_monitor(
    callback: impl Fn(Theme) + Send + Sync + 'static,
) -> Result<(), String> {
    static STATE: OnceLock<Result<(), String>> = OnceLock::new();
    STATE
        .get_or_init(|| WindowsThemeListener::new(callback).start())
        .clone()
}

/// Windows 系统主题监听器：注册表通知（RegNotifyChangeKeyValue）驱动，
/// 探测 AppsUseLightTheme 变化后回调；无需窗口消息或轮询。
pub struct WindowsThemeListener {
    callback: Arc<dyn Fn(Theme) + Send + Sync>,
}

impl WindowsThemeListener {
    /// 创建监听器，变化时调用回调（回调在监听线程中执行，须快速返回）。
    pub fn new(callback: impl Fn(Theme) + Send + Sync + 'static) -> Self {
        Self {
            callback: Arc::new(callback),
        }
    }

    /// 打开注册表键并启动监听线程；失败时清理已打开句柄并返回错误。
    pub fn start(self) -> Result<(), String> {
        const PERSONALIZE_PATH: &str =
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";

        // KEY_NOTIFY 只监听不读取；读取仍走 WindowsThemeProvider（winreg）
        let path_wide: Vec<u16> = PERSONALIZE_PATH
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut key = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                windows::Win32::System::Registry::HKEY_CURRENT_USER,
                PCWSTR::from_raw(path_wide.as_ptr()),
                None,
                REG_SAM_FLAGS(KEY_NOTIFY.0),
                &mut key,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("RegOpenKeyExW 失败: {status:?}"));
        }
        // windows crate 句柄类型非 Send，以 usize 跨线程传输后重建
        let key_raw = key.0 as usize;

        let notify_event = match unsafe { CreateEventW(None, true, false, PCWSTR::null()) } {
            Ok(event) => event,
            Err(error) => {
                let _ = unsafe { RegCloseKey(key) };
                return Err(format!("CreateEventW 失败: {error}"));
            }
        };
        let notify_event_raw = notify_event.0 as usize;

        let callback = self.callback;
        std::thread::spawn(move || {
            let key = HKEY(key_raw as *mut _);
            let notify_event = HANDLE(notify_event_raw as *mut _);
            let provider = WindowsThemeProvider;
            // 记录启动时的系统主题；注册表通知按键订阅，同键下其他值写入
            // （壁纸/强调色等）也会触发，仅主题值实际变化时才回调
            let mut last_theme = provider.current_system_theme().ok();
            loop {
                // 注册通知：异步模式，变化时 notify_event 置位；每次等待后需重新注册
                let status = unsafe {
                    RegNotifyChangeKeyValue(
                        key,
                        false,
                        REG_NOTIFY_CHANGE_LAST_SET | REG_NOTIFY_CHANGE_NAME,
                        Some(notify_event),
                        true,
                    )
                };
                if status != ERROR_SUCCESS {
                    error!(?status, "RegNotifyChangeKeyValue 失败，停止系统主题监听");
                    break;
                }
                let wait = unsafe { WaitForSingleObject(notify_event, INFINITE) };
                if wait != WAIT_OBJECT_0 {
                    error!(?wait, "等待系统主题通知失败，停止系统主题监听");
                    break;
                }
                let _ = unsafe { ResetEvent(notify_event) };
                // 触发信号：读取当前主题，与上次值比较后回调；重置事件等待下次变化
                match provider.current_system_theme() {
                    Ok(theme) => {
                        if last_theme != Some(theme) {
                            last_theme = Some(theme);
                            callback(theme);
                        }
                    }
                    Err(error) => error!(error = %error, "读取系统主题失败"),
                }
            }

            let _ = unsafe { RegCloseKey(key) };
            let _ = unsafe { CloseHandle(notify_event) };
        });

        Ok(())
    }
}
