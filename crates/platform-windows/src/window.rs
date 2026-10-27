use async_trait::async_trait;
use std::time::Duration;
use tracing::warn;
use windows::core::GUID;
use windows::Win32::Foundation::{CloseHandle, FALSE, HWND, LPARAM, TRUE, WPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::Shell::{Shell_NotifyIconGetRect, NOTIFYICONIDENTIFIER};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, EnumWindows, GetClassNameW, GetForegroundWindow,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW,
    SwitchToThisWindow, WM_APP, WM_LBUTTONDOWN, WM_LBUTTONUP,
};
use windows_core::BOOL;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::window::WindowManager;

/// Windows 平台窗口管理器实现。
/// 通过 Win32 API 实现进程遍历、窗口枚举与窗口激活。
pub struct WindowsWindowManager;

impl Default for WindowsWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsWindowManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WindowManager for WindowsWindowManager {
    /// 根据进程名激活已存在的窗口。
    /// 先尝试可见窗口；未找到时尝试目标进程的 Electron 托盘默认动作。
    async fn activate_window_by_process(&self, process_name: &str) -> Result<bool, HostApiError> {
        let hwnd = get_window_by_process_name(process_name);
        if let Some(hwnd) = hwnd {
            unsafe {
                SwitchToThisWindow(hwnd, true);
            }
            Ok(true)
        } else {
            Ok(activate_electron_tray_by_process(process_name).await)
        }
    }
    /// 根据进程 PID 激活已存在的窗口。
    /// 枚举窗口匹配 GetWindowThreadProcessId 的可见窗口，调用 SwitchToThisWindow 激活。
    async fn activate_window_by_pid(&self, pid: u32) -> Result<bool, HostApiError> {
        let hwnd = get_window_by_pid(pid);
        if let Some(hwnd) = hwnd {
            unsafe {
                SwitchToThisWindow(hwnd, true);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 根据窗口标题的部分内容激活已存在的窗口。
    /// 枚举所有可见窗口，标题匹配后调用 SwitchToThisWindow 激活。
    async fn activate_window_by_title(&self, title: &str) -> Result<bool, HostApiError> {
        let hwnd = get_window_by_title(title);
        if let Some(hwnd) = hwnd {
            unsafe {
                SwitchToThisWindow(hwnd, true);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// 托盘窗口枚举状态 —— 仅限本文件内使用。
///
/// 保存目标进程及其 Electron 托盘消息窗口句柄，供顶层窗口枚举回调写入。
struct ElectronTrayWindowData {
    /// 当前待查找托盘窗口所属进程 ID。
    process_id: u32,
    /// 找到时写入的托盘宿主窗口句柄；None 表示尚未找到。
    hwnd: Option<HWND>,
}

/// 查找指定进程的 Electron 托盘宿主窗口，仅限本文件内的窗口枚举使用。
unsafe extern "system" fn find_electron_tray_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut ElectronTrayWindowData);
    let mut process_id = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    if process_id != data.process_id {
        return TRUE;
    }

    let mut class_name = [0u16; 64];
    let length = GetClassNameW(hwnd, &mut class_name);
    let class_name = String::from_utf16_lossy(&class_name[..length.max(0) as usize]);
    if class_name == "Electron_NotifyIconHostWindow" {
        data.hwnd = Some(hwnd);
        return FALSE;
    }
    TRUE
}

/// 根据进程名触发 QQ 当前 Electron 托盘图标默认动作。
/// 仅支持经验证的 QQ.exe/WM_APP+1 协议；图标记录经实时矩形验证后才发送回调。
async fn activate_electron_tray_by_process(process_name: &str) -> bool {
    let target_name = std::path::Path::new(process_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(process_name);
    if !target_name.eq_ignore_ascii_case("QQ.exe") {
        return false;
    }
    let target_name = target_name.to_lowercase();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(settings) = hkcu.open_subkey_with_flags("Control Panel\\NotifyIconSettings", KEY_READ)
    else {
        return false;
    };

    let mut icon_ids = Vec::new();
    for subkey_name in settings.enum_keys().flatten() {
        let Ok(icon_key) = settings.open_subkey_with_flags(subkey_name, KEY_READ) else {
            continue;
        };
        let Ok(executable_path) = icon_key.get_value::<String, _>("ExecutablePath") else {
            continue;
        };
        let executable_name = std::path::Path::new(&executable_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if executable_name.eq_ignore_ascii_case(&target_name) {
            if let Ok(icon_id) = icon_key.get_value::<u32, _>("UID") {
                icon_ids.push(icon_id);
            }
        }
    }
    icon_ids.sort_unstable();
    icon_ids.dedup();
    if icon_ids.len() != 1 {
        return false;
    }

    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return false;
    };
    let mut process = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut target_window = None;
    if unsafe { Process32FirstW(snapshot, &mut process) }.is_ok() {
        loop {
            let executable_name = String::from_utf16_lossy(&process.szExeFile)
                .trim_end_matches('\0')
                .to_lowercase();
            if executable_name == target_name {
                let mut data = ElectronTrayWindowData {
                    process_id: process.th32ProcessID,
                    hwnd: None,
                };
                unsafe {
                    let _ = EnumWindows(
                        Some(find_electron_tray_window),
                        LPARAM(&mut data as *mut _ as isize),
                    );
                }
                if let Some(hwnd) = data.hwnd {
                    target_window = Some((hwnd, data.process_id));
                    break;
                }
            }
            if unsafe { Process32NextW(snapshot, &mut process) }.is_err() {
                break;
            }
        }
    }
    let _ = unsafe { CloseHandle(snapshot) };
    let Some((hwnd, process_id)) = target_window else {
        return false;
    };

    let process_id = {
        let identifier = NOTIFYICONIDENTIFIER {
            cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
            hWnd: hwnd,
            uID: icon_ids[0],
            guidItem: GUID::zeroed(),
        };
        if unsafe { Shell_NotifyIconGetRect(&identifier) }.is_err() {
            return false;
        }
        if unsafe { AllowSetForegroundWindow(process_id) }.is_err() {
            return false;
        }

        let callback_message = WM_APP + 1;
        let posted = unsafe {
            PostMessageW(
                Some(hwnd),
                callback_message,
                WPARAM(icon_ids[0] as usize),
                LPARAM(WM_LBUTTONDOWN as isize),
            )
            .is_ok()
                && PostMessageW(
                    Some(hwnd),
                    callback_message,
                    WPARAM(icon_ids[0] as usize),
                    LPARAM(WM_LBUTTONUP as isize),
                )
                .is_ok()
        };
        if !posted {
            return false;
        }
        process_id
    };
    tokio::time::sleep(Duration::from_millis(250)).await;
    let foreground = unsafe { GetForegroundWindow() };
    let mut foreground_process = 0;
    unsafe { GetWindowThreadProcessId(foreground, Some(&mut foreground_process)) };
    foreground_process == process_id
}

/// 获取窗口标题
fn get_window_title(hwnd: HWND) -> String {
    unsafe {
        let length = GetWindowTextLengthW(hwnd);
        if length == 0 {
            return String::new();
        }

        let mut buffer = vec![0u16; length as usize + 1];
        let chars_copied = GetWindowTextW(hwnd, &mut buffer);
        if chars_copied == 0 {
            return String::new();
        }

        buffer.truncate(chars_copied as usize);
        String::from_utf16_lossy(&buffer)
    }
}

/// 用于窗口枚举的数据结构
struct WindowEnumData {
    process_id: u32,
    hwnd: Option<HWND>,
}

/// 窗口枚举回调函数，查找属于指定进程的可见窗口
unsafe extern "system" fn find_process_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut WindowEnumData);
    let mut pid: u32 = 0;

    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if pid == data.process_id && IsWindowVisible(hwnd).as_bool() {
        data.hwnd = Some(hwnd);
        return FALSE;
    }

    TRUE
}

/// 根据 exe 进程名获取目标窗口句柄。
fn get_window_by_process_name(process_name: &str) -> Option<HWND> {
    let mut result: Option<HWND> = None;

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(s) => s,
            Err(e) => {
                warn!("创建进程快照失败: {:?}", e);
                return None;
            }
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let proc_name_lower = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_end_matches('\0')
                    .to_lowercase();
                let process_name_lower = process_name.to_lowercase();
                if proc_name_lower == process_name_lower {
                    let process_id = entry.th32ProcessID;

                    let mut window_data = WindowEnumData {
                        process_id,
                        hwnd: None,
                    };

                    let _ = EnumWindows(
                        Some(find_process_window),
                        LPARAM(&mut window_data as *mut _ as isize),
                    );

                    if let Some(hwnd) = window_data.hwnd {
                        result = Some(hwnd);
                        break;
                    }
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }
    result
}

/// 根据进程 PID 查找目标窗口句柄。
/// 枚举全部可见窗口，匹配 GetWindowThreadProcessId 属于该 pid 的窗口。
fn get_window_by_pid(pid: u32) -> Option<HWND> {
    let mut window_data = WindowEnumData {
        process_id: pid,
        hwnd: None,
    };

    unsafe {
        let _ = EnumWindows(
            Some(find_process_window),
            LPARAM(&mut window_data as *mut _ as isize),
        );
    }

    window_data.hwnd
}

/// 根据窗口标题的部分内容查找窗口句柄（不区分大小写）。
fn get_window_by_title(title_part: &str) -> Option<HWND> {
    let windows = get_all_windows();

    let matching_windows: Vec<_> = windows
        .iter()
        .filter(|(_, title)| {
            let title_part_lower = title_part.to_lowercase();
            let title_lower = title.to_lowercase();
            title_lower.contains(&title_part_lower)
        })
        .collect();

    if matching_windows.is_empty() {
        return None;
    }

    let (hwnd, _) = matching_windows[0];
    Some(*hwnd)
}

/// 枚举窗口的回调数据结构
struct EnumWindowsCallbackData {
    windows: Vec<(HWND, String)>,
}

/// 枚举窗口的回调函数
extern "system" fn get_all_windows_enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let data = &mut *(lparam.0 as *mut EnumWindowsCallbackData);

        if !IsWindowVisible(hwnd).as_bool() {
            return TRUE;
        }

        let title = get_window_title(hwnd);
        if !title.is_empty() {
            data.windows.push((hwnd, title));
        }

        TRUE
    }
}

/// 获取所有可见窗口及其标题
fn get_all_windows() -> Vec<(HWND, String)> {
    let mut data = EnumWindowsCallbackData {
        windows: Vec::new(),
    };

    unsafe {
        let _ = EnumWindows(
            Some(get_all_windows_enum_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }

    data.windows
}
