use crate::sdk::host_api::HostApiError;
use crate::sdk::window::WindowManager;
use async_trait::async_trait;
use tracing::warn;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::SwitchToThisWindow;
use windows::Win32::{
    Foundation::{FALSE, HWND, LPARAM, TRUE},
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    },
};
use windows_core::BOOL;

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
    /// 遍历进程快照找到匹配进程，枚举其窗口，调用 SwitchToThisWindow 激活。
    async fn activate_window_by_process(&self, process_name: &str) -> Result<bool, HostApiError> {
        let hwnd = get_window_by_process_name(process_name);
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
