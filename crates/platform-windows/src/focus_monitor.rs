use dashmap::DashMap;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::sync::Arc;
use tracing::{info, warn};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetAncestor, GetClientRect, GetCursorPos, GetForegroundWindow, GetMessageW,
    TranslateMessage, EVENT_SYSTEM_FOREGROUND, GA_ROOT, MSG, WINEVENT_OUTOFCONTEXT,
};
use zerolaunch_plugin_api::services::focus_monitor::{FocusCallback, FocusEvent, FocusMonitor};

thread_local! {
    /// 当前钩子线程的前台窗口状态；仅由安装钩子的线程读写。
    static HOOK_STATE: RefCell<Option<FocusHookState>> = const { RefCell::new(None) };
}

/// 前台窗口钩子的运行时状态。
///
/// 仅存在于 `focus-monitor` 专用线程：目标窗口句柄、回调表与上一次前台窗口。
struct FocusHookState {
    /// 需要观察的顶层窗口句柄（HWND 数值）；等于目标窗口失焦时才推送事件。
    target: isize,
    /// 已注册的失焦回调，按 id 存储。
    callbacks: Arc<DashMap<String, FocusCallback>>,
    /// 上一次前台顶层窗口句柄；0 表示尚未记录。
    last_foreground: Cell<isize>,
}

/// Windows 平台聚焦监控器实现。
/// 构造即开始监控，不可停止 —— FocusMonitor 是系统强制启用的 push-based 服务。
/// 通过 `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` 观察前台窗口切换，
/// 目标窗口失焦且鼠标不在窗口内时依次调用所有已注册回调。
///
/// 仅由 `platform_services` 构造并经 `FocusMonitor` 端口使用。
pub struct WindowsFocusMonitor {
    /// 已注册的失焦回调，按 id 存储。
    callbacks: Arc<DashMap<String, FocusCallback>>,
}

impl WindowsFocusMonitor {
    /// 创建 WindowsFocusMonitor 实例并立即开始监控。
    /// 参数：target_window - 需要观察的顶层窗口句柄（HWND 数值）；0 表示不监控。
    /// 返回：监控器实例。监控线程随进程常驻，进程退出时由系统回收。
    pub fn new(target_window: isize) -> Self {
        let callbacks = Arc::new(DashMap::<String, FocusCallback>::new());
        if target_window == 0 {
            warn!("未提供主窗口句柄，焦点监控未启动");
            return Self { callbacks };
        }

        let callbacks_for_thread = callbacks.clone();
        std::thread::Builder::new()
            .name("focus-monitor".to_string())
            .spawn(move || unsafe { run_focus_hook(target_window, callbacks_for_thread) })
            .expect("启动焦点监控线程失败");

        Self { callbacks }
    }
}

/// 焦点监控线程入口：安装前台窗口钩子并运行消息循环。
/// 参数：target_window - 目标顶层窗口句柄；callbacks - 回调表。
/// 返回：无。钩子安装失败时记录告警并退出线程。
unsafe fn run_focus_hook(target_window: isize, callbacks: Arc<DashMap<String, FocusCallback>>) {
    let target = GetAncestor(HWND(target_window as *mut c_void), GA_ROOT);
    HOOK_STATE.with(|slot| {
        *slot.borrow_mut() = Some(FocusHookState {
            target: target.0 as isize,
            callbacks,
            last_foreground: Cell::new(GetForegroundWindow().0 as isize),
        });
    });

    let hook = SetWinEventHook(
        EVENT_SYSTEM_FOREGROUND,
        EVENT_SYSTEM_FOREGROUND,
        None,
        Some(on_foreground_changed),
        0,
        0,
        WINEVENT_OUTOFCONTEXT,
    );
    if hook.is_invalid() {
        warn!("安装前台窗口事件钩子失败");
        return;
    }
    info!("焦点监控已启动");

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    let _ = UnhookWinEvent(hook);
}

/// 前台窗口切换回调：目标窗口失去前台时推送 `FocusEvent::Lost`。
/// 参数由 Win32 钩子传入；仅使用新前台窗口句柄。
/// 返回：无。
unsafe extern "system" fn on_foreground_changed(
    _hook: HWINEVENTHOOK,
    _event: u32,
    window: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    let (callbacks, lost) = HOOK_STATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(state) = slot.as_mut() else {
            return (None, false);
        };

        let previous = state.last_foreground.replace(window.0 as isize);
        if previous == 0 || previous == window.0 as isize {
            return (None, false);
        }

        let previous_root = GetAncestor(HWND(previous as *mut c_void), GA_ROOT);
        let lost = state.target != 0
            && previous_root.0 as isize == state.target
            && !cursor_inside(state.target);
        (Some(state.callbacks.clone()), lost)
    });

    if let (Some(callbacks), true) = (callbacks, lost) {
        for entry in callbacks.iter() {
            (entry.value())(FocusEvent::Lost);
        }
    }
}

/// 判断鼠标是否位于目标窗口客户区内。
/// 参数：target - 目标窗口句柄数值。
/// 返回：位于客户区内返回 true；句柄无效或系统调用失败返回 false（按失焦处理）。
unsafe fn cursor_inside(target: isize) -> bool {
    let hwnd = HWND(target as *mut c_void);
    let mut rect = RECT::default();
    if GetClientRect(hwnd, &mut rect).is_err() {
        return false;
    }

    let mut origin = POINT {
        x: rect.left,
        y: rect.top,
    };
    if ClientToScreen(hwnd, &mut origin).0 == 0 {
        return false;
    }

    let mut cursor = POINT::default();
    if GetCursorPos(&mut cursor).is_err() {
        return false;
    }

    cursor.x >= origin.x
        && cursor.x <= origin.x + rect.right
        && cursor.y >= origin.y
        && cursor.y <= origin.y + rect.bottom
}

impl FocusMonitor for WindowsFocusMonitor {
    fn register_callback(&self, id: &str, callback: FocusCallback) {
        self.callbacks.insert(id.to_string(), callback);
    }

    fn unregister_callback(&self, id: &str) {
        self.callbacks.remove(id);
    }
}
