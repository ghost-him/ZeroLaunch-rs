use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::core::config::models::{ComponentInfo, ComponentSchema};
use crate::state::app_state::AppState;
use crate::utils::trace_id::generate_trace_id;
use serde::Deserialize;
use std::sync::Arc;
use zerolaunch_plugin_api::config::ConfigActionDef;

/// 获取应用版本号（从 Cargo.toml 编译时注入）。
#[tauri::command]
#[tracing::instrument(fields(trace_id))]
pub fn config_get_version() -> Result<String, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    Ok::<_, BridgeError>(env!("CARGO_PKG_VERSION").to_string()).with_trace_id(&trace_id)
}

/// 获取指定组件的配置动作列表。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn config_get_actions(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<Vec<ConfigActionDef>, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    Ok::<_, BridgeError>(state.get_config_manager().get_config_actions(&component_id))
        .with_trace_id(&trace_id)
}

/// `config_execute_action` 的 IPC 参数。
#[derive(Debug, Deserialize)]
pub struct ConfigActionPayload {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "action")]
    pub action: String,
    #[serde(rename = "params", default)]
    pub params: Option<serde_json::Value>,
}

/// 执行指定组件的配置动作。
#[tauri::command]
#[tracing::instrument(skip(state, payload), fields(trace_id))]
pub async fn config_execute_action(
    state: tauri::State<'_, Arc<AppState>>,
    payload: ConfigActionPayload,
) -> Result<serde_json::Value, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let params = payload.params.unwrap_or(serde_json::Value::Null);
    state
        .get_config_manager()
        .execute_config_action(&payload.component_id, &payload.action, &params)
        .await
        .with_trace_id(&trace_id)
}

/// 获取所有可配置组件的概览信息。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn config_get_all_components(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<ComponentInfo>, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let components = state.get_config_manager().get_all_components();
    Ok::<_, BridgeError>(components).with_trace_id(&trace_id)
}

/// 获取指定组件的配置 Schema。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn config_get_schema(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<ComponentSchema, BridgeError> {
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
pub async fn config_apply_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    settings: serde_json::Value,
) -> Result<(), BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_config_manager()
        .apply_settings(&component_id, settings)
        .await
        .with_trace_id(&trace_id)
}

/// 重置组件配置为默认值
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn config_reset_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<(), BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_config_manager()
        .reset_to_default(&component_id)
        .await
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
