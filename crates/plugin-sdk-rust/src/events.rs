//! 宿主推送事件的处理函数注册表。
//!
//! 定时器到期与订阅事件（按键 / 安装 / 焦点）由宿主以 `plugin/*` 通知推送
//! （见 `zerolaunch_plugin_protocol::methods::event`），SDK 读任务解析后按
//! 插件内 ID 查表调用注册的处理函数。插件作者经 `HostProxy` 的
//! `set_timeout` / `set_interval` / `subscribe_*` 注册，不直接使用本模块。
//!
//! 处理函数在 SDK 读任务上同步调用：必须快速返回，耗时操作请自行 spawn。

use dashmap::DashMap;
use std::sync::{Arc, LazyLock};

use zerolaunch_plugin_api::services::focus_monitor::FocusEvent;
use zerolaunch_plugin_api::services::hotkey::types::HotkeyEvent;
use zerolaunch_plugin_api::services::installation_monitor::types::InstallationEvent;
use zerolaunch_plugin_protocol::methods::event;
use zerolaunch_plugin_protocol::{
    FocusEventParams, HotkeyEventParams, InstallationEventParams, TimerFiredParams,
};

/// 定时器到期处理函数：入参为定时器 ID。
pub type TimerHandler = Arc<dyn Fn(u64) + Send + Sync>;
/// 按键事件处理函数。
pub type HotkeyEventHandler = Arc<dyn Fn(HotkeyEvent) + Send + Sync>;
/// 安装监控事件处理函数。
pub type InstallationEventHandler = Arc<dyn Fn(InstallationEvent) + Send + Sync>;
/// 窗口焦点事件处理函数。
pub type FocusEventHandler = Arc<dyn Fn(FocusEvent) + Send + Sync>;

/// 定时器登记项。
/// 仅本模块内使用，由 register_timer 写入、dispatch / unregister_timer 消费。
struct TimerEntry {
    /// 到期时调用的处理函数。
    handler: TimerHandler,
    /// true = 重复定时器（登记保留至显式取消）；false = 一次性（首次触发后移除登记）。
    repeat: bool,
}

/// 定时器登记表（宿主分配的定时器 ID → 登记项）。
static TIMERS: LazyLock<DashMap<u64, TimerEntry>> = LazyLock::new(DashMap::new);
/// 按键事件订阅表（插件内订阅 ID → 处理函数）。
static HOTKEY_SUBS: LazyLock<DashMap<String, HotkeyEventHandler>> = LazyLock::new(DashMap::new);
/// 安装监控订阅表（插件内订阅 ID → 处理函数）。
static INSTALLATION_SUBS: LazyLock<DashMap<String, InstallationEventHandler>> =
    LazyLock::new(DashMap::new);
/// 窗口焦点订阅表（插件内订阅 ID → 处理函数）。
static FOCUS_SUBS: LazyLock<DashMap<String, FocusEventHandler>> = LazyLock::new(DashMap::new);

/// 登记定时器处理函数。
/// 参数：id - 宿主返回的定时器 ID；repeat - 是否重复触发；handler - 到期处理函数。
/// 特性：同 ID 重复登记覆盖旧处理函数。
pub(crate) fn register_timer(id: u64, repeat: bool, handler: TimerHandler) {
    TIMERS.insert(id, TimerEntry { handler, repeat });
}

/// 注销定时器处理函数（取消成功后调用；未登记时为空操作）。
pub(crate) fn unregister_timer(id: u64) {
    TIMERS.remove(&id);
}

/// 清空全部定时器处理函数（cancel_all_timers 成功后调用）。
pub(crate) fn clear_timers() {
    TIMERS.clear();
}

/// 登记按键事件订阅处理函数。
pub(crate) fn register_hotkey(id: String, handler: HotkeyEventHandler) {
    HOTKEY_SUBS.insert(id, handler);
}

/// 注销按键事件订阅处理函数（未登记时为空操作）。
pub(crate) fn unregister_hotkey(id: &str) {
    HOTKEY_SUBS.remove(id);
}

/// 登记安装监控订阅处理函数。
pub(crate) fn register_installation(id: String, handler: InstallationEventHandler) {
    INSTALLATION_SUBS.insert(id, handler);
}

/// 注销安装监控订阅处理函数（未登记时为空操作）。
pub(crate) fn unregister_installation(id: &str) {
    INSTALLATION_SUBS.remove(id);
}

/// 登记窗口焦点订阅处理函数。
pub(crate) fn register_focus(id: String, handler: FocusEventHandler) {
    FOCUS_SUBS.insert(id, handler);
}

/// 注销窗口焦点订阅处理函数（未登记时为空操作）。
pub(crate) fn unregister_focus(id: &str) {
    FOCUS_SUBS.remove(id);
}

/// 调用插件注册的处理函数，隔离 panic：单个坏回调记 error 日志，不得杀死 SDK 读任务。
/// 参数：kind - 事件族（日志用）；id - 定时器 ID / 订阅 ID（日志用）；f - 处理函数调用。
fn guard_call(kind: &str, id: impl std::fmt::Display, f: impl FnOnce()) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err() {
        tracing::error!("插件 {kind} 处理函数 panic，已隔离（id={id}）");
    }
}

/// 分发一条宿主推送的通知。
/// 参数：method - 通知方法名（`plugin/*` 推送）；params - 通知载荷。
/// 返回：true = 本模块认识的推送方法（含载荷解析失败的情况），false = 未知方法。
/// 错误：载荷解析失败时静默丢弃（协议层已保证帧合法，字段不符属版本漂移，仅影响该条事件）。
pub(crate) fn dispatch(method: &str, params: &serde_json::Value) -> bool {
    match method {
        // 定时器到期：一次性定时器触发后移除登记，重复定时器保留至显式取消。
        event::TIMER_FIRED => {
            if let Ok(p) = serde_json::from_value::<TimerFiredParams>(params.clone()) {
                let entry = TIMERS.get(&p.id).map(|e| (e.handler.clone(), e.repeat));
                if let Some((handler, repeat)) = entry {
                    if !repeat {
                        TIMERS.remove(&p.id);
                    }
                    let id = p.id;
                    guard_call("timer", id, move || handler(id));
                }
            }
            true
        }
        // 按键事件：按订阅 ID 路由到处理函数。
        event::HOTKEY_EVENT => {
            if let Ok(p) = serde_json::from_value::<HotkeyEventParams>(params.clone()) {
                let handler = HOTKEY_SUBS.get(&p.id).map(|h| h.clone());
                if let Some(handler) = handler {
                    let sub_id = p.id.clone();
                    guard_call("hotkey", &sub_id, move || handler(p.event));
                }
            }
            true
        }
        // 安装监控事件：按订阅 ID 路由。
        event::INSTALLATION_EVENT => {
            if let Ok(p) = serde_json::from_value::<InstallationEventParams>(params.clone()) {
                let handler = INSTALLATION_SUBS.get(&p.id).map(|h| h.clone());
                if let Some(handler) = handler {
                    let sub_id = p.id.clone();
                    guard_call("installation", &sub_id, move || handler(p.event));
                }
            }
            true
        }
        // 窗口焦点事件：按订阅 ID 路由。
        event::FOCUS_EVENT => {
            if let Ok(p) = serde_json::from_value::<FocusEventParams>(params.clone()) {
                let handler = FOCUS_SUBS.get(&p.id).map(|h| h.clone());
                if let Some(handler) = handler {
                    let sub_id = p.id.clone();
                    guard_call("focus", &sub_id, move || handler(p.event));
                }
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 一次性定时器首次触发即移除登记；重复定时器可多次触发；未知方法返回 false。
    #[test]
    fn dispatch_routes_timers_and_reports_unknown_methods() {
        let oneshot_fires = Arc::new(AtomicU64::new(0));
        let interval_fires = Arc::new(AtomicU64::new(0));

        let counter = oneshot_fires.clone();
        register_timer(
            1,
            false,
            Arc::new(move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        );
        let counter = interval_fires.clone();
        register_timer(
            2,
            true,
            Arc::new(move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        );

        for _ in 0..2 {
            assert!(dispatch(
                event::TIMER_FIRED,
                &serde_json::json!({ "id": 1 })
            ));
            assert!(dispatch(
                event::TIMER_FIRED,
                &serde_json::json!({ "id": 2 })
            ));
        }

        assert_eq!(oneshot_fires.load(Ordering::SeqCst), 1);
        assert_eq!(interval_fires.load(Ordering::SeqCst), 2);
        // 一次性定时器登记已移除，重复定时器仍在。
        assert!(!TIMERS.contains_key(&1));
        assert!(TIMERS.contains_key(&2));
        // 坏回调 panic 被隔离：不传播出 dispatch，读循环继续服务。
        register_timer(2, true, Arc::new(|_| panic!("坏回调")));
        assert!(dispatch(
            event::TIMER_FIRED,
            &serde_json::json!({ "id": 2 })
        ));
        assert_eq!(interval_fires.load(Ordering::SeqCst), 2);
        unregister_timer(2);
        assert!(TIMERS.is_empty());

        assert!(!dispatch("plugin/unknown.method", &serde_json::json!({})));
    }

    /// 订阅事件按插件内 ID 路由；未登记的 ID 不触发任何处理函数。
    #[test]
    fn dispatch_routes_subscriptions_by_id() {
        let hits = Arc::new(AtomicU64::new(0));
        let counter = hits.clone();
        register_installation(
            "sub-a".to_string(),
            Arc::new(move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        );

        assert!(dispatch(
            event::INSTALLATION_EVENT,
            &serde_json::json!({
                "id": "sub-a",
                "event": { "changedPaths": ["C:/x/a.lnk"], "kind": "created" }
            })
        ));
        // 未登记 ID：方法被识别但无处理函数。
        assert!(dispatch(
            event::INSTALLATION_EVENT,
            &serde_json::json!({
                "id": "sub-b",
                "event": { "changedPaths": [], "kind": "other" }
            })
        ));
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        unregister_installation("sub-a");
        assert!(INSTALLATION_SUBS.is_empty());
    }
}
