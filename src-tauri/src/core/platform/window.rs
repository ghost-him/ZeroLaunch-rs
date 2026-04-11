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

/// 获取窗口标题
pub fn get_window_title(hwnd: HWND) -> String {
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

/// 定义用于窗口枚举的数据结构
struct WindowEnumData {
    process_id: u32,
    hwnd: Option<HWND>,
}

/// 窗口枚举回调函数
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

/// 根据 exe程序，获得目标的窗口
pub fn get_window_by_process_name(process_name: &str) -> Option<HWND> {
    let mut result: Option<HWND> = None;

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(s) => s,
            Err(_) => return None,
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

/// 将目标hwnd的窗口激活
pub fn activate_with_hwnd(hwnd: HWND) {
    unsafe {
        SwitchToThisWindow(hwnd, true);
    }
}

/// 根据窗口标题的部分内容查找并激活窗口
pub fn get_window_by_title(title_part: &str) -> Option<HWND> {
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

    let (hwnd, _title) = matching_windows[0];
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
pub fn get_all_windows() -> Vec<(HWND, String)> {
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
