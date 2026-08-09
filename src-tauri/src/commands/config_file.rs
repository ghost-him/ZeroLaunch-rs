use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::core::config::models::{ComponentInfoSnapshot, ComponentSchemaSnapshot};
use crate::state::app_state::AppState;
use crate::utils::trace_id::generate_trace_id;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zerolaunch_plugin_api::config::{ComponentType, ConfigActionDef, SettingsContribution};

/// IPC `config_get_all_components` 返回的组件概览 DTO。
#[derive(Debug, Serialize)]
pub struct ComponentInfoDto {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "componentName")]
    pub component_name: String,
    #[serde(rename = "componentDescription")]
    pub component_description: String,
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,
    #[serde(rename = "priority")]
    pub priority: u32,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "defaultEnabled")]
    pub default_enabled: bool,
}

impl From<ComponentInfoSnapshot> for ComponentInfoDto {
    /// 将 core 领域快照转换为 camelCase IPC DTO。
    fn from(value: ComponentInfoSnapshot) -> Self {
        Self {
            component_id: value.component_id,
            component_name: value.component_name,
            component_description: value.component_description,
            component_type: value.component_type,
            priority: value.priority,
            enabled: value.enabled,
            default_enabled: value.default_enabled,
        }
    }
}

/// IPC `config_get_schema` 返回的组件 schema DTO。
#[derive(Debug, Serialize)]
pub struct ComponentSchemaDto {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "componentName")]
    pub component_name: String,
    #[serde(rename = "componentDescription")]
    pub component_description: String,
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,
    #[serde(rename = "contribution")]
    pub contribution: SettingsContribution,
}

impl From<ComponentSchemaSnapshot> for ComponentSchemaDto {
    /// 将 core schema 快照转换为 IPC DTO。
    fn from(value: ComponentSchemaSnapshot) -> Self {
        Self {
            component_id: value.component_id,
            component_name: value.component_name,
            component_description: value.component_description,
            component_type: value.component_type,
            contribution: value.contribution,
        }
    }
}

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
) -> Result<Vec<ComponentInfoDto>, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let components = state
        .get_config_manager()
        .get_all_components()
        .into_iter()
        .map(ComponentInfoDto::from)
        .collect();
    Ok::<_, BridgeError>(components).with_trace_id(&trace_id)
}

/// 获取指定组件的配置 Schema。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn config_get_schema(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<ComponentSchemaDto, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_config_manager()
        .get_component_schema(&component_id)
        .map(ComponentSchemaDto::from)
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
