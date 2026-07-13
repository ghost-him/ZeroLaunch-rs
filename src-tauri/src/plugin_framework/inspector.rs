use crate::core::config::ConfigManager;
/// Plugin Inspector — 运行时调试面板。
/// 维护一个 ring buffer 记录最近的查询和执行事件。
/// 录制与否由调用方通过 AppState::is_debug_mode() 控制，Inspector 自身不判断。
/// 前端通过 `inspector_get_state` IPC 命令查询，模拟查询通过 `debug_simulate_query`。
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::VecDeque;

/// 单次查询日志。
#[derive(Debug, Clone, Serialize)]
pub struct InspectedQueryEvent {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "traceId")]
    pub trace_id: String,
    #[serde(rename = "rawQuery")]
    pub raw_query: String,
    #[serde(rename = "mode")]
    pub mode: String,
    #[serde(rename = "resultCount")]
    pub result_count: usize,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

/// 完整的 Inspector 状态，由 inspector_get_state 返回。
#[derive(Debug, Clone, Serialize)]
pub struct InspectorState {
    #[serde(rename = "registeredPlugins")]
    pub registered_plugins: Vec<PluginInspectorInfo>,
    #[serde(rename = "recentQueries")]
    pub recent_queries: Vec<InspectedQueryEvent>,
    #[serde(rename = "totalQueriesLogged")]
    pub total_queries_logged: u64,
}

/// 单个已注册组件的基本信息。
#[derive(Debug, Clone, Serialize)]
pub struct PluginInspectorInfo {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "componentName")]
    pub component_name: String,
    #[serde(rename = "componentType")]
    pub component_type: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

/// Ring-buffer 记录器。容量在构造时指定。
/// 录制与否由调用方控制，Inspector 自身不判断。
pub struct Inspector {
    events: RwLock<VecDeque<InspectedQueryEvent>>,
    total_count: parking_lot::Mutex<u64>,
    /// 缓存的组件清单。
    ///
    /// **假设**：组件清单在启动后不变（当前不支持运行时动态注册/卸载组件）。
    /// 若未来引入运行时注册，调用 `invalidate_cache()` 可强制下次 snapshot 重建清单。
    cached_plugins: parking_lot::Mutex<Option<Vec<PluginInspectorInfo>>>,
    capacity: usize,
}

impl Inspector {
    /// 创建指定容量的 ring-buffer 记录器。
    pub fn new(capacity: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(capacity)),
            total_count: parking_lot::Mutex::new(0),
            cached_plugins: parking_lot::Mutex::new(None),
            capacity,
        }
    }

    /// 记录一条查询事件。始终写入，不判断调试模式（由调用方控制）。
    /// 若 buffer 已满，从头部弹出最早的事件。
    pub fn record(&self, event: InspectedQueryEvent) {
        let mut events = self.events.write();
        if events.len() >= self.capacity {
            events.pop_front();
        }
        events.push_back(event);
        *self.total_count.lock() += 1;
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
