use crate::bridge_error::{BridgeError, WithTraceId};
use crate::state::app_state::AppState;
use std::sync::Arc;
use zerolaunch_plugin_api::config::ConfigActionDef;

/// 获取应用版本号（从 Cargo.toml 编译时注入）
#[tauri::command]
pub fn config_get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 获取指定组件的配置动作列表
#[tauri::command]
pub fn config_get_actions(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Vec<ConfigActionDef> {
    state.get_config_manager().get_config_actions(&component_id)
}

/// 执行指定组件的配置动作
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn config_execute_action(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    action: String,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let params = params.unwrap_or(serde_json::Value::Null);
    state
        .get_config_manager()
        .execute_config_action(&component_id, &action, &params)
        .await
        .with_trace_id(&trace_id)
}

/// 获取所有可配置组件的概览信息
#[tauri::command]
pub fn config_get_all_components(
    state: tauri::State<'_, Arc<AppState>>,
) -> Vec<crate::core::config::ComponentInfo> {
    state.get_config_manager().get_all_components()
}

/// 获取指定组件的配置 Schema
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn config_get_schema(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<crate::core::config::ComponentSchema, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_config_manager()
        .get_component_schema(&component_id)
        .ok_or_else(|| BridgeError::not_found(&component_id).with_trace_id(&trace_id))
}

/// 获取指定组件的当前配置值
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn config_get_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<serde_json::Value, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_config_manager()
        .get_settings(&component_id)
        .ok_or_else(|| BridgeError::not_found(&component_id).with_trace_id(&trace_id))
}

/// 应用配置到指定组件
#[tauri::command]
#[tracing::instrument(skip(state, settings), fields(trace_id))]
pub fn config_apply_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    settings: serde_json::Value,
) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_config_manager()
        .apply_settings(&component_id, settings)
        .with_trace_id(&trace_id)
}

/// 重置组件配置为默认值
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn config_reset_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_config_manager()
        .reset_to_default(&component_id)
        .with_trace_id(&trace_id)
}

/// 设置组件启用状态
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn config_set_enabled(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    enabled: bool,
) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_config_manager()
        .set_enabled(&component_id, enabled)
        .with_trace_id(&trace_id)
}
