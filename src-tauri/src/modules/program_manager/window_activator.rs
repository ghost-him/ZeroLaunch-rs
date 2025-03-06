use super::unit::LaunchMethod;
use super::unit::Program;
use parking_lot::RwLock;
use std::path::Path;
use std::sync::Arc;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, TRUE},
    UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    },
};
#[derive(Debug)]
pub struct WindowActivatorInner {}

impl WindowActivatorInner {
    pub fn new() -> Self {
        Self {}
    }

    pub fn activate_target_program(&self, target: Arc<Program>) -> bool {
        match target.launch_method.clone() {
            LaunchMethod::Path(path) => {
                if path.ends_with(".url") {
                    let program_name = target.show_name.clone();
                    return self.activate_with_title(&program_name);
                } else {
                    let mut exe_path = String::new();
                    if path.ends_with(".exe") {
                        exe_path = path.clone();
                    } else {
                        if let Ok(exe_path_str) = lnk::ShellLink::open(path) {
                            exe_path = exe_path_str
                                .link_info()
                                .as_ref()
                                .unwrap()
                                .local_base_path()
                                .as_ref()
                                .unwrap()
                                .clone();
                        }
                    }
                    if exe_path.is_empty() {
                        return false;
                    }
                    println!("exe_path: {}", exe_path);
                    return self.activate_with_exe(&exe_path);
                }
            }
            LaunchMethod::PackageFamilyName(_family_name) => {}
            _ => {}
        }
        false
    }

    // 直接使用标题来激活窗口
    fn activate_with_title(&self, program_name: &str) -> bool {
        if let Some(hwnd) = self.get_window_by_title(program_name) {
            return self.activate_with_hwnd(hwnd);
        }
        false
    }

    // 激活.exe程序的窗口，传入绝对路径
    fn activate_with_exe(&self, str: &str) -> bool {
        let abs_path = Path::new(str);
        let program_name = abs_path.file_name().unwrap().to_str().unwrap().to_string();
        let program_stem = abs_path.file_stem().unwrap().to_str().unwrap().to_string();
        println!("file_name: {} file_stem: {}", program_name, program_stem);
        let hwnd: Option<HWND> = {
            let mut result = self.get_window_by_process_name(&program_name);
            if result.is_none() {
                result = self.get_window_by_title(&program_stem);
            }
            result
        };

        if hwnd.is_none() {
            return false;
        }

        self.activate_with_hwnd(hwnd.unwrap())
    }

    // 根据 exe程序，获得目标的窗口
    fn get_window_by_process_name(&self, process_name: &str) -> Option<HWND> {
        let mut result: Option<HWND> = None;

        unsafe {
            // 创建进程快照
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();

            let mut entry = PROCESSENTRY32W::default();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let proc_name_lower = String::from_utf16_lossy(&entry.szExeFile)
                        .trim_end_matches('\0')
                        .to_lowercase();
                    let process_name_lower = process_name.to_lowercase();
                    if proc_name_lower.contains(&process_name_lower) {
                        // 找到匹配的进程，获取其主窗口
                        let process_id = entry.th32ProcessID;
                        let mut hwnd = FindWindowExW(None, None, None, None);

                        while hwnd.as_ref().unwrap().0 != std::ptr::null_mut() {
                            let mut pid: u32 = 0;
                            GetWindowThreadProcessId(
                                hwnd.as_ref().unwrap().clone(),
                                Some(&mut pid),
                            );

                            if pid == process_id
                                && IsWindowVisible(hwnd.as_ref().unwrap().clone()).as_bool()
                            {
                                result = Some(hwnd.as_ref().unwrap().clone());
                                break;
                            }

                            hwnd = FindWindowExW(
                                None,
                                Some(hwnd.as_ref().unwrap().clone()),
                                None,
                                None,
                            );
                        }

                        if result.is_some() {
                            break;
                        }
                    }

                    if !Process32NextW(snapshot, &mut entry).is_ok() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        result
    }

    // 将目标hwnd的窗口激活
    fn activate_with_hwnd(&self, hwnd: HWND) -> bool {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);

            SetForegroundWindow(hwnd).as_bool()
        }
    }

    // 根据窗口标题的部分内容查找并激活窗口
    fn get_window_by_title(&self, title_part: &str) -> Option<HWND> {
        let windows = get_all_windows();

        // 查找包含指定标题部分的窗口
        let matching_windows: Vec<_> = windows
            .iter()
            .filter(|(_, title)| {
                let title_part_lower = title_part.to_lowercase();
                let title_lower = title.to_lowercase();
                title_lower.contains(&title_part_lower)
            })
            .collect();

        if matching_windows.is_empty() {
            println!("没有找到包含 '{}' 的窗口", title_part);
            return None;
        }

        // 如果找到多个匹配窗口，选择第一个
        let (hwnd, _title) = matching_windows[0];
        Some(hwnd.clone())
    }
}

/// 这个类用于将已经打开的程序唤醒
#[derive(Debug)]
pub struct WindowActivator {
    inner: RwLock<WindowActivatorInner>,
}

impl WindowActivator {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(WindowActivatorInner::new()),
        }
    }
    pub fn activate_target_program(&self, target: Arc<Program>) -> bool {
        let inner = self.inner.read();
        inner.activate_target_program(target)
    }
}

// 获取窗口标题
fn get_window_title(hwnd: HWND) -> String {
    unsafe {
        // 获取窗口标题长度
        let length = GetWindowTextLengthW(hwnd);
        if length == 0 {
            return String::new();
        }

        // 分配缓冲区并获取窗口标题
        let mut buffer = vec![0u16; length as usize + 1];
        let chars_copied = GetWindowTextW(hwnd, &mut buffer);
        if chars_copied == 0 {
            return String::new();
        }

        // 转换为Rust字符串
        buffer.truncate(chars_copied as usize);
        String::from_utf16_lossy(&buffer)
    }
}

// 枚举窗口的回调数据结构
struct EnumWindowsCallbackData {
    windows: Vec<(HWND, String)>,
}

// 枚举窗口的回调函数
extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let data = &mut *(lparam.0 as *mut EnumWindowsCallbackData);

        // 检查窗口是否可见
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

// 获取所有可见窗口及其标题
fn get_all_windows() -> Vec<(HWND, String)> {
    let mut data = EnumWindowsCallbackData {
        windows: Vec::new(),
    };

    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut data as *mut _ as isize),
        )
        .expect("EnumWindows failed");
    }

    data.windows
}
