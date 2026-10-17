//! 全局键盘事件监听（Raw Input 输入汇聚）。
//!
//! 用 `RegisterRawInputDevices(RIDEV_INPUTSINK)` 把键盘原始输入**异步**投递到隐藏窗口：
//! 事件由系统输入线程投递，不拦截、不阻塞输入管线，也不会像 WH_KEYBOARD_LL 那样
//! 让系统等待回调返回（更不会因 `AttachThreadInput` 改动前台应用的键盘状态）。
//! 注册可随时注销，因此功能关闭后本进程不再读取任何全局输入。

use std::sync::mpsc::channel;
use std::thread::{self, JoinHandle};

use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
    RIDEV_INPUTSINK, RIDEV_REMOVE, RID_INPUT, RIM_TYPEKEYBOARD,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, PostMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW,
    TranslateMessage, GWLP_USERDATA, MSG, WM_CLOSE, WM_DESTROY, WM_INPUT, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

/// 通用桌面设备（键盘/鼠标）的 HID 用途页与用途 ID。
const HID_USAGE_PAGE_GENERIC: u16 = 0x01;
/// 键盘设备用途 ID。
const HID_USAGE_GENERIC_KEYBOARD: u16 = 0x06;
/// `RAWKEYBOARD::Flags` 的松开标志（winuser.h 的 RI_KEY_BREAK）。
const RI_KEY_BREAK: u16 = 0x01;

/// 一次全局键盘事件。
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawKeyEvent {
    /// 虚拟键码（如 0xA2 = 左 Ctrl）。
    pub vk: u16,
    /// true 表示按下，false 表示松开。
    pub is_down: bool,
}

/// 原始输入键盘监听器。
/// 内部持有隐藏窗口与注册在其上的 RIDEV_INPUTSINK 注册；`stop` 后监听线程退出、注册注销。
pub(crate) struct RawKeyboardListener {
    /// 隐藏窗口句柄。以 isize 保存使监听器可跨线程移动（HWND 为裸指针，不实现 Send）。
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl RawKeyboardListener {
    /// 启动全局键盘监听。
    /// 参数：callback - 每个按键事件在监听线程内同步调用，需保持轻量（阻塞会拖住后续事件投递）。
    /// 返回：成功返回监听器；创建隐藏窗口或注册原始输入失败返回错误描述。
    pub(crate) fn start<F>(callback: F) -> Result<Self, String>
    where
        F: Fn(RawKeyEvent) + Send + 'static,
    {
        let (ready_tx, ready_rx) = channel::<Result<isize, String>>();
        let thread = thread::Builder::new()
            .name("zerolaunch-raw-keyboard".to_string())
            .spawn(move || {
                // 回调随线程栈存活；窗口的 GWLP_USERDATA 存其地址，窗口销毁后不再被调用
                let callback: Box<dyn Fn(RawKeyEvent) + Send> = Box::new(callback);
                let hmodule = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
                let hinstance = HINSTANCE(hmodule.0);
                let class_name = w!("ZeroLaunchRawKeyboardSink");
                let wnd_class = WNDCLASSW {
                    lpfnWndProc: Some(wnd_proc),
                    hInstance: hinstance,
                    lpszClassName: class_name,
                    ..Default::default()
                };
                // 类名固定、进程内只注册一次：重复启动时返回 0（类已存在），可忽略
                unsafe { RegisterClassW(&wnd_class) };

                let hwnd = unsafe {
                    CreateWindowExW(
                        WS_EX_TOOLWINDOW,
                        class_name,
                        w!("ZeroLaunchRawKeyboardSink"),
                        WS_POPUP,
                        0,
                        0,
                        0,
                        0,
                        None,
                        None,
                        Some(hinstance),
                        None,
                    )
                };
                let hwnd = match hwnd {
                    Ok(hwnd) => hwnd,
                    Err(e) => {
                        let _ = ready_tx.send(Err(format!("创建原始输入隐藏窗口失败: {e}")));
                        return;
                    }
                };
                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, &callback as *const _ as isize) };

                let registration = RAWINPUTDEVICE {
                    usUsagePage: HID_USAGE_PAGE_GENERIC,
                    usUsage: HID_USAGE_GENERIC_KEYBOARD,
                    dwFlags: RIDEV_INPUTSINK,
                    hwndTarget: hwnd,
                };
                if let Err(e) = unsafe {
                    RegisterRawInputDevices(
                        &[registration],
                        std::mem::size_of::<RAWINPUTDEVICE>() as u32,
                    )
                } {
                    let _ = ready_tx.send(Err(format!("注册原始输入键盘设备失败: {e}")));
                    let _ = unsafe { DestroyWindow(hwnd) };
                    return;
                }
                let _ = ready_tx.send(Ok(hwnd.0 as isize));

                let mut msg = MSG::default();
                unsafe {
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    // 窗口已在 WM_CLOSE 中销毁；这里注销注册做兜底
                    let unregister = RAWINPUTDEVICE {
                        usUsagePage: HID_USAGE_PAGE_GENERIC,
                        usUsage: HID_USAGE_GENERIC_KEYBOARD,
                        dwFlags: RIDEV_REMOVE,
                        hwndTarget: HWND::default(),
                    };
                    let _ = RegisterRawInputDevices(
                        &[unregister],
                        std::mem::size_of::<RAWINPUTDEVICE>() as u32,
                    );
                }
            })
            .map_err(|e| format!("创建原始输入监听线程失败: {e}"))?;

        let hwnd = ready_rx
            .recv()
            .map_err(|e| format!("原始输入监听线程未就绪: {e}"))??;
        Ok(Self {
            hwnd,
            thread: Some(thread),
        })
    }

    /// 停止监听：请求监听线程退出消息循环并等待其收尾（销毁窗口、注销注册）。
    /// 返回：无。
    pub(crate) fn stop(mut self) {
        let hwnd = HWND(self.hwnd as *mut std::ffi::c_void);
        let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// 隐藏窗口的消息处理：解析 WM_INPUT 键盘事件，其余交给默认处理。
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_INPUT => {
            deliver_key_event(hwnd, lparam);
            // WM_INPUT 必须交回默认处理，系统才能完成原始输入缓冲清理
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// 取出 WM_INPUT 载荷里的键盘事件并交给用户回调。
/// 参数：hwnd - 承载注册的隐藏窗口；lparam - WM_INPUT 的 lParam（HRAWINPUT）。
/// 返回：无。非键盘设备的消息直接忽略。
unsafe fn deliver_key_event(hwnd: HWND, lparam: LPARAM) {
    let handle = HRAWINPUT(lparam.0 as *mut std::ffi::c_void);
    let header_size = std::mem::size_of::<RAWINPUTHEADER>() as u32;
    let mut size = 0u32;
    if GetRawInputData(handle, RID_INPUT, None, &mut size, header_size) == u32::MAX || size == 0 {
        return;
    }
    // RAWINPUT 含指针成员，用 u64 缓冲保证指针对齐
    let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
    let read = GetRawInputData(
        handle,
        RID_INPUT,
        Some(buffer.as_mut_ptr() as *mut std::ffi::c_void),
        &mut size,
        header_size,
    );
    if read == u32::MAX || read == 0 {
        return;
    }
    let raw = &*(buffer.as_ptr() as *const RAWINPUT);
    if raw.header.dwType != RIM_TYPEKEYBOARD.0 {
        return;
    }
    let keyboard = raw.data.keyboard;
    let event = RawKeyEvent {
        vk: keyboard.VKey,
        is_down: keyboard.Flags & RI_KEY_BREAK == 0,
    };
    let callback = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Box<dyn Fn(RawKeyEvent) + Send>;
    if !callback.is_null() {
        (*callback)(event);
    }
}
