//! 翻译相关 IPC 命令。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::state::app_state::AppState;
use crate::utils::trace_id::generate_trace_id;
use serde_json::Value;
use std::sync::Arc;

/// 获取指定语言下所有已加载第三方插件的翻译目录。
/// 返回嵌套结构 `{"plugin": {"<pluginId>": {…}}}`，前端以 vue-i18n 命名空间方式合并。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn i18n_get_plugin_translations(
    state: tauri::State<'_, Arc<AppState>>,
    lang: String,
) -> Result<Value, BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    Ok::<_, BridgeError>(state.get_i18n_manager().plugin_catalog_for(&lang))
        .with_trace_id(&trace_id)
}
