use crate::core::config::ConfigManager;
/// Plugin Inspector — 运行时调试面板。
/// 维护一个 ring buffer 记录最近的查询和执行事件。
/// 录制开关由 `is_debug_mode` 配置项控制（`recording_enabled` AtomicBool）。
/// 前端通过 `inspector_get_state` / `inspector_simulate_query` IPC 命令查询。
/// 当录制开启时，每次 `record()` 调用将新事件写入 ring buffer 并返回 true。
/// 调用方收到 true 后可决定是否向前端广播通知。
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

/// 单次查询日志。
#[derive(Debug, Clone, Serialize)]
pub struct InspectedQueryEvent {
    pub timestamp: String,
    pub trace_id: String,
    pub raw_query: String,
    pub mode: String,
    pub result_count: usize,
    pub duration_ms: u64,
}

/// 完整的 Inspector 状态，由 inspector_get_state 返回。
#[derive(Debug, Clone, Serialize)]
pub struct InspectorState {
    pub registered_plugins: Vec<PluginInspectorInfo>,
    pub recent_queries: Vec<InspectedQueryEvent>,
    pub total_queries_logged: u64,
}

/// 单个已注册组件的基本信息。
#[derive(Debug, Clone, Serialize)]
pub struct PluginInspectorInfo {
    pub component_id: String,
    pub component_name: String,
    pub component_type: String,
    pub enabled: bool,
}

/// Ring-buffer 记录器。容量在构造时指定。
/// 录制开关由 `recording_enabled` AtomicBool 控制，运行时通过 `set_recording()` 切换。
pub struct Inspector {
    events: RwLock<VecDeque<InspectedQueryEvent>>,
    total_count: parking_lot::Mutex<u64>,
    /// 缓存的组件清单。
    ///
    /// **假设**：组件清单在启动后不变（当前不支持运行时动态注册/卸载组件）。
    /// 若未来引入运行时注册，调用 `invalidate_cache()` 可强制下次 snapshot 重建清单。
    cached_plugins: parking_lot::Mutex<Option<Vec<PluginInspectorInfo>>>,
    capacity: usize,
    /// 录制开关，由外部通过 `set_recording()` 控制。
    /// `false` 时 `record()` 为空操作，IPC 命令返回不可用状态。
    recording_enabled: AtomicBool,
}

impl Inspector {
    /// 创建指定容量的 ring-buffer 记录器，录制默认关闭。
    /// 录制开关由外部通过 `set_recording(bool)` 根据 `is_debug_mode` 配置控制。
    pub fn new(capacity: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(capacity)),
            total_count: parking_lot::Mutex::new(0),
            cached_plugins: parking_lot::Mutex::new(None),
            capacity,
            recording_enabled: AtomicBool::new(false),
        }
    }

    /// 开启或关闭录制。`false` 时 `record()` 为空操作。
    pub fn set_recording(&self, enabled: bool) {
        self.recording_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_recording(&self) -> bool {
        self.recording_enabled.load(Ordering::Relaxed)
    }
    /// 记录一条查询事件。若录制未开启，直接返回 false。
    /// 若 buffer 已满，从头部弹出最早的事件。
    /// 返回 true 表示有新的记录写入，调用方可根据返回值决定是否通知前端。
    pub fn record(&self, event: InspectedQueryEvent) -> bool {
        if !self.recording_enabled.load(Ordering::Relaxed) {
            return false;
        }
        let mut events = self.events.write();
        if events.len() >= self.capacity {
            events.pop_front();
        }
        events.push_back(event);
        *self.total_count.lock() += 1;
        true
    }

    /// 清除缓存的组件清单，强制下次 `snapshot()` 从 ConfigManager 重建。
    /// 仅在运行时动态注册/卸载组件后需要调用。
    pub fn invalidate_cache(&self) {
        *self.cached_plugins.lock() = None;
    }

    /// 生成当前快照，包含所有已注册插件和最近事件日志。
    /// 组件清单在首次调用时初始化并缓存，后续仅刷新 enabled 状态。
    pub fn snapshot(&self, config_manager: &ConfigManager) -> InspectorState {
        let mut cached = self.cached_plugins.lock();
        let registered_plugins = match cached.as_mut() {
            Some(plugins) => {
                // 仅刷新 enabled 状态（可能因用户操作改变）
                for p in plugins.iter_mut() {
                    p.enabled = config_manager.is_enabled(&p.component_id);
                }
                plugins.clone()
            }
            None => {
                let components = config_manager.get_all_components();
                let plugins: Vec<PluginInspectorInfo> = components
                    .into_iter()
                    .map(|info| PluginInspectorInfo {
                        component_id: info.component_id,
                        component_name: info.component_name,
                        component_type: format!("{:?}", info.component_type),
                        enabled: info.enabled,
                    })
                    .collect();
                *cached = Some(plugins.clone());
                plugins
            }
        };

        let recent_queries = self.events.read().iter().cloned().collect();
        let total_queries_logged = *self.total_count.lock();

        InspectorState {
            registered_plugins,
            recent_queries,
            total_queries_logged,
        }
    }
}
