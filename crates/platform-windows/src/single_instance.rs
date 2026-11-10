//! Windows 单实例实现：命名互斥量 + 隐藏窗口 + WM_COPYDATA 通知。
//!
//! 首个实例创建互斥量与隐藏窗口；后续实例发现互斥量已存在时，
//! 向隐藏窗口发送工作目录与命令行参数，然后退出。

use std::ffi::{c_void, CStr, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;

use tracing::{info, warn};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, FindWindowW, GetWindowLongPtrW, RegisterClassExW,
    SendMessageW, SetWindowLongPtrW, CREATESTRUCTW, GWLP_USERDATA, GWL_STYLE, WM_COPYDATA,
    WM_CREATE, WM_DESTROY, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP, WS_VISIBLE,
};

/// WM_COPYDATA 的私有标识，区分本应用的单实例消息。
const SINGLE_INSTANCE_COPYDATA_MAGIC: usize = 0x5A4C;

/// 第二实例通知回调 —— 首个实例收到后续实例的命令行参数与工作目录。
pub type SecondaryInstanceCallback = Arc<dyn Fn(Vec<String>, String) + Send + Sync>;

/// 单实例获取结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleInstanceStatus {
    /// 当前进程是首个实例，已创建接收通知的隐藏窗口。
    Primary,
    /// 已有实例在运行，通知已发送；调用方应退出。
    Secondary,
}

/// 单实例运行时数据 —— 挂在隐藏窗口的 GWLP_USERDATA 上，随窗口销毁释放。
struct SingleInstanceUserData {
    /// 第二实例通知回调。
    callback: SecondaryInstanceCallback,
}

/// 尝试成为应用唯一实例。
/// 参数：app_id - 互斥量、窗口类与窗口名的前缀（通常用应用 identifier）；
///       callback - 首个实例收到后续实例通知时调用，参数为命令行参数与工作目录。
/// 返回：Primary 表示获得唯一实例；Secondary 表示已有实例在运行且已收到通知。
/// 首个实例的互斥量与隐藏窗口随进程存活，不显式释放。
pub fn acquire_single_instance(
    app_id: &str,
    callback: SecondaryInstanceCallback,
) -> SingleInstanceStatus {
    let class_name = wide(&format!("{app_id}-sic"));
    let window_name = wide(&format!("{app_id}-siw"));
    let mutex_name = wide(&format!("{app_id}-sim"));

    let mutex = unsafe {
        CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr())).expect("创建单实例互斥量失败")
    };

    // 互斥量已存在说明已有实例：通知它并返回 Secondary。
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        let hwnd =
            unsafe { FindWindowW(PCWSTR(class_name.as_ptr()), PCWSTR(window_name.as_ptr())) }
                .unwrap_or_default();
        unsafe {
            let _ = CloseHandle(mutex);
        };
        if hwnd.is_invalid() {
            warn!("单实例互斥量已存在，但未找到通知窗口");
        } else {
            unsafe { notify_primary(hwnd) };
        }
        return SingleInstanceStatus::Secondary;
    }

    let userdata = Box::into_raw(Box::new(SingleInstanceUserData { callback }));
    unsafe {
        create_event_target_window(&class_name, &window_name, userdata)
            .expect("创建单实例通知窗口失败");
    }
    info!("单实例校验通过，已创建通知窗口");
    SingleInstanceStatus::Primary
}

/// 向首个实例的隐藏窗口发送当前进程的工作目录与命令行参数。
/// 参数：hwnd - 首个实例的隐藏窗口。
/// 返回：无；发送失败由系统忽略。
unsafe fn notify_primary(hwnd: HWND) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let cwd = cwd.to_string_lossy();
    let args = std::env::args().collect::<Vec<_>>().join("|");
    let data = format!("{cwd}|{args}\0");
    let bytes = data.as_bytes();
    let cds = COPYDATASTRUCT {
        dwData: SINGLE_INSTANCE_COPYDATA_MAGIC,
        cbData: bytes.len() as u32,
        lpData: bytes.as_ptr() as *mut c_void,
    };
    SendMessageW(
        hwnd,
        WM_COPYDATA,
        Some(WPARAM(0)),
        Some(LPARAM(&cds as *const _ as isize)),
    );
}

/// 创建接收 WM_COPYDATA 的隐藏窗口。
/// 参数：class_name / window_name - 宽字符类名与窗口名；userdata - 回调数据指针。
/// 返回：创建成功返回窗口句柄；注册或创建失败返回错误。
unsafe fn create_event_target_window(
    class_name: &[u16],
    window_name: &[u16],
    userdata: *mut SingleInstanceUserData,
) -> windows::core::Result<HWND> {
    let module = GetModuleHandleW(None)?;
    let instance = HINSTANCE(module.0);
    let window_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(single_instance_window_proc),
        hInstance: instance,
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };
    RegisterClassExW(&window_class);

    let hwnd = CreateWindowExW(
        WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
        PCWSTR(class_name.as_ptr()),
        PCWSTR(window_name.as_ptr()),
        WS_OVERLAPPED,
        0,
        0,
        0,
        0,
        None,
        None,
        Some(instance),
        Some(userdata as *const c_void),
    )?;

    // 隐藏窗口需要可见样式才能参与部分消息分发，LAYERED 保证不显示。
    SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_VISIBLE | WS_POPUP).0 as isize);
    Ok(hwnd)
}

/// 隐藏窗口过程：处理创建、WM_COPYDATA 与销毁。
/// 参数由 Win32 传入。
/// 返回：处理结果为 0/1；其余消息交给默认过程。
unsafe extern "system" fn single_instance_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create_struct = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create_struct.lpCreateParams as isize);
            LRESULT(0)
        }
        WM_COPYDATA => {
            let copy_data = &*(lparam.0 as *const COPYDATASTRUCT);
            if copy_data.dwData == SINGLE_INSTANCE_COPYDATA_MAGIC {
                let userdata =
                    &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const SingleInstanceUserData);
                let data = CStr::from_ptr(copy_data.lpData as *const i8).to_string_lossy();
                let mut parts = data.split('|');
                let cwd = parts.next().unwrap_or_default().to_string();
                let args = parts.map(str::to_string).collect();
                (userdata.callback)(args, cwd);
            }
            LRESULT(1)
        }
        WM_DESTROY => {
            let userdata = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SingleInstanceUserData;
            if !userdata.is_null() {
                drop(Box::from_raw(userdata));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// 把字符串编码为以 NUL 结尾的 UTF-16。
/// 参数：value - 待编码字符串。
/// 返回：包含结尾 NUL 的宽字符序列。
fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
