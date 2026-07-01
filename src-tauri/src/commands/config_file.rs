use crate::core::types::BridgeError;
use crate::plugin_framework::ConfigActionDef;
use crate::state::app_state::AppState;
use std::sync::Arc;

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
pub fn config_execute_action(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    action: String,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, BridgeError> {
    let params = params.unwrap_or(serde_json::Value::Null);
    state
        .get_config_manager()
        .execute_config_action(&component_id, &action, &params)
        .map_err(BridgeError::internal)
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
pub fn config_get_schema(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<crate::core::config::ComponentSchema, BridgeError> {
    state
        .get_config_manager()
        .get_component_schema(&component_id)
        .ok_or_else(|| BridgeError::not_found(&component_id))
}

/// 获取指定组件的当前配置值
#[tauri::command]
pub fn config_get_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<serde_json::Value, BridgeError> {
    state
        .get_config_manager()
        .get_settings(&component_id)
        .ok_or_else(|| BridgeError::not_found(&component_id))
}

/// 应用配置到指定组件
#[tauri::command]
pub fn config_apply_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    settings: serde_json::Value,
) -> Result<(), BridgeError> {
    state
        .get_config_manager()
        .apply_settings(&component_id, settings)
        .map_err(BridgeError::from)
}

/// 重置组件配置为默认值
#[tauri::command]
pub fn config_reset_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<(), BridgeError> {
    state
        .get_config_manager()
        .reset_to_default(&component_id)
        .map_err(BridgeError::from)
}

/// 设置组件启用状态
#[tauri::command]
pub fn config_set_enabled(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    enabled: bool,
) -> Result<(), BridgeError> {
    state
        .get_config_manager()
        .set_enabled(&component_id, enabled)
        .map_err(BridgeError::from)
}
