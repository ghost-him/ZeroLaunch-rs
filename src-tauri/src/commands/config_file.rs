use crate::plugin_system::ConfigActionDef;
use crate::state::app_state::AppState;
use std::sync::Arc;

/// 获取指定组件的配置动作列表
#[tauri::command]
pub fn get_config_actions(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Vec<ConfigActionDef> {
    state.get_session_router().get_config_actions(&component_id)
}

/// 执行指定组件的配置动作
#[tauri::command]
pub fn execute_config_action(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    action: String,
) -> Result<serde_json::Value, String> {
    state
        .get_session_router()
        .execute_config_action(&component_id, &action)
}

/// 获取所有可配置组件的概览信息
#[tauri::command]
pub fn get_all_components(
    state: tauri::State<'_, Arc<AppState>>,
) -> Vec<crate::core::config::ComponentInfo> {
    state.get_config_manager().get_all_components()
}

/// 获取指定组件的配置 Schema
#[tauri::command]
pub fn get_component_schema(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<crate::core::config::ComponentSchema, String> {
    state
        .get_config_manager()
        .get_component_schema(&component_id)
        .ok_or_else(|| format!("Component not found: {}", component_id))
}

/// 获取指定组件的当前配置值
#[tauri::command]
pub fn get_component_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<serde_json::Value, String> {
    state
        .get_config_manager()
        .get_settings(&component_id)
        .ok_or_else(|| format!("Component not found: {}", component_id))
}

/// 应用配置到指定组件
#[tauri::command]
pub fn apply_component_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    settings: serde_json::Value,
) -> Result<(), String> {
    state
        .get_config_manager()
        .apply_settings(&component_id, settings)
        .map_err(|e| e.to_string())
}

/// 重置组件配置为默认值
#[tauri::command]
pub fn reset_component_settings(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
) -> Result<(), String> {
    state
        .get_config_manager()
        .reset_to_default(&component_id)
        .map_err(|e| e.to_string())
}

/// 设置组件启用状态
#[tauri::command]
pub fn set_component_enabled(
    state: tauri::State<'_, Arc<AppState>>,
    component_id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .get_config_manager()
        .set_enabled(&component_id, enabled)
        .map_err(|e| e.to_string())
}
