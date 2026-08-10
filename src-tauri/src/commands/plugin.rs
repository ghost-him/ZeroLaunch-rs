//! 插件管理 IPC 命令（第三方插件）。
//!
//! 业务协调逻辑：组合 PluginManager（生命周期）与 ConfigManager（配置），
//! 不新增中间层，直接在命令处理器中编排两个管理器的调用。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::core::config::manager::ConfigManager;
use crate::state::app_state::AppState;
use std::sync::Arc;
use tauri::State;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_host::manager::{InstalledPluginInfo, PluginHostManager};
use zerolaunch_plugin_protocol::Manifest;

// ── 私有辅助 ─────────────────────────────────────────────────────

/// 已加载第三方插件的运行时信息列表。
///
/// `enabled_fn` 仅用于计算 enabled 字段，不过滤列表；
/// 供 plugin_list（全量）与 plugin_get_detail（按 id 查找）共用同一判据。
fn host_runtime_infos(cm: &ConfigManager, hm: &PluginHostManager) -> Vec<InstalledPluginInfo> {
    hm.list_plugin_info(|a| {
        a.components.iter().all(|c| cm.is_enabled(c.component_id())) && !a.components.is_empty()
    })
}

// ── Commands ─────────────────────────────────────────────────────

/// List all installed plugins (built-in + third-party) with runtime details.
///
/// 内置条目元数据取自 PluginRegistry 的 PluginMetadata（插件级数据源）：
/// 插件管理页的实体是插件而非组件，ComponentCore 是组件级身份、为配置系统服务，
/// 多组件插件场景下组件级数据无唯一性，故不从这里派生插件元数据。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn plugin_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<InstalledPluginInfo>, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let pm = state.get_plugin_manager();
    let cm = state.get_config_manager();
    let dispatcher = state.get_session_dispatcher();
    let hm = pm.host_manager();

    let mut list = host_runtime_infos(&cm, &hm);
    // 内置插件：以 PluginRegistry 的 PluginMetadata 为插件级数据源。
    // state 固定 "builtin"（与 plugin_get_detail 判据一致），组件 id 即插件 id。
    for meta in dispatcher.plugin_registry().get_all_metadata() {
        if list.iter().any(|i| i.plugin_id == meta.id) {
            continue;
        }
        list.push(InstalledPluginInfo {
            plugin_id: meta.id.clone(),
            name: meta.name.clone(),
            version: meta.version.clone(),
            description: meta.description.clone(),
            author: meta.author.clone(),
            state: "builtin".to_string(),
            enabled: cm.is_enabled(&meta.id),
            // 内置插件优先级为非负小整数，可直接转换
            priority: meta.priority as u32,
            component_ids: vec![meta.id.clone()],
        });
    }
    list.sort_by_key(|p| (p.priority, p.plugin_id.clone()));
    Ok(list)
}

/// 获取第三方插件的完整 manifest。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn plugin_get_manifest(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Manifest, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let plugin_manager = state.get_plugin_manager();
    plugin_manager
        .get_manifest(&plugin_id)
        .ok_or_else(|| BridgeError::not_found(&plugin_id).with_trace_id(&trace_id))
}

/// 插件详情 DTO —— 插件管理页详情弹窗的数据契约（跨 IPC 序列化）。
///
/// 由 plugin_get_detail 命令返回，仅前端 PluginsManagement.vue 消费；
/// manifest 为 None 即内置插件（无 manifest 文件），前端据此隐藏第三方专属字段。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginDetail {
    /// 插件唯一标识（内置为组件 id，第三方为 manifest 的 plugin.id）。
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    /// 显示名称（key-or-literal：内置为 i18n key，第三方为 manifest 字面量，渲染前须 resolveText）。
    #[serde(rename = "name")]
    pub name: String,
    /// 版本号（内置为空串：随应用分发，无独立版本）。
    #[serde(rename = "version")]
    pub version: String,
    /// 描述文本（key-or-literal，渲染前须 resolveText）。
    #[serde(rename = "description")]
    pub description: String,
    /// 作者（内置为空串：无独立作者，UI 按内置标识展示）。
    #[serde(rename = "author")]
    pub author: String,
    /// 触发词列表（插件唤起关键字，来自 PluginMetadata）。
    #[serde(rename = "triggerKeywords")]
    pub trigger_keywords: Vec<String>,
    /// 支持的操作系统列表。
    #[serde(rename = "supportedOs")]
    pub supported_os: Vec<String>,
    /// 排序优先级（与协议层 PluginMetadata.priority 同型，内置恒非负）。
    #[serde(rename = "priority")]
    pub priority: i32,
    /// 运行状态：内置固定 "builtin"，第三方为进程状态字符串（Running/Stopped 等）。
    #[serde(rename = "state")]
    pub state: String,
    /// 是否启用（内置由 ConfigManager 判定，第三方与 plugin_list 同判据）。
    #[serde(rename = "enabled")]
    pub enabled: bool,
    /// 第三方插件的完整 manifest；内置插件为 None（无 manifest 文件）。
    #[serde(rename = "manifest")]
    pub manifest: Option<Manifest>,
}

/// 获取插件详情：元数据（含触发词）+ 第三方完整 manifest + 运行状态。
/// 触发词不在 manifest 中，来自运行时 PluginMetadata，故此处统一从注册中心取，
/// 内置与第三方插件共用同一数据通路。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn plugin_get_detail(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<PluginDetail, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let pm = state.get_plugin_manager();
    let cm = state.get_config_manager();
    let dispatcher = state.get_session_dispatcher();

    // 元数据（含触发词）：内置与第三方插件统一来自 PluginRegistry。
    let metadata = dispatcher
        .plugin_registry()
        .get(&plugin_id)
        .map(|p| p.metadata().clone())
        .ok_or_else(|| BridgeError::not_found(&plugin_id).with_trace_id(&trace_id))?;

    // 运行状态与启用态：第三方来自 host 运行时，内置由 ConfigManager 决定。
    let hm = pm.host_manager();
    let info = host_runtime_infos(&cm, &hm)
        .into_iter()
        .find(|i| i.plugin_id == plugin_id);
    let state_str = info
        .as_ref()
        .map(|i| i.state.clone())
        .unwrap_or_else(|| "builtin".to_string());
    let enabled = info
        .map(|i| i.enabled)
        .unwrap_or_else(|| cm.is_enabled(&plugin_id));

    Ok(PluginDetail {
        plugin_id: metadata.id.clone(),
        name: metadata.name.clone(),
        version: metadata.version.clone(),
        description: metadata.description.clone(),
        author: metadata.author.clone(),
        trigger_keywords: metadata.trigger_keywords.clone(),
        supported_os: metadata.supported_os.clone(),
        priority: metadata.priority,
        state: state_str,
        enabled,
        manifest: pm.get_manifest(&plugin_id),
    })
}

/// Install a plugin from a local .zip file or directory.
/// Emits `plugin-installed` on success.
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn plugin_install_local(
    file_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<InstalledPluginInfo, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let plugin_manager = state.get_plugin_manager();
    let path = std::path::PathBuf::from(&file_path);
    let app_handle = state.get_main_handle();

    plugin_manager
        .install(&path, app_handle)
        .await
        .with_trace_id(&trace_id)
}

/// Reload a third-party plugin.
/// Emits `plugin-installed` on success.
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn plugin_reload(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let plugin_manager = state.get_plugin_manager();
    let app_handle = state.get_main_handle();

    plugin_manager
        .reload(&plugin_id, app_handle)
        .await
        .with_trace_id(&trace_id)
}

/// Uninstall a third-party plugin.
/// Emits `plugin-uninstalled` on success.
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn plugin_uninstall(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let plugin_manager = state.get_plugin_manager();
    let app_handle = state.get_main_handle();

    plugin_manager
        .uninstall(&plugin_id, app_handle)
        .await
        .with_trace_id(&trace_id)
}

/// Enable or disable all components of a plugin (third-party or builtin).
///
/// 第三方插件：遍历其所有 Configurable 逐个调用 CM.set_enabled()。
/// 内置组件：直接按 plugin_id 调用 CM.set_enabled()。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn plugin_set_enabled(
    plugin_id: String,
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let pm = state.get_plugin_manager();
    let cm = state.get_config_manager();
    let dispatcher = state.get_session_dispatcher();
    let hm = pm.host_manager();

    // 先持久化全部组件，全部成功后才同步触发词索引：
    // 任一步持久化失败时命令返回 Err 且路由索引零变更（配置与路由不分叉）。
    if let Some(plugin) = hm.plugins.get(&plugin_id) {
        // 第三方插件：遍历其所有 Configurable 逐个调用 CM.set_enabled()。
        for c in &plugin.components {
            cm.set_enabled(c.component_id(), enabled)
                .with_trace_id(&trace_id)?;
        }
    } else {
        // 内置组件：直接按 plugin_id 调用 CM.set_enabled()。
        cm.set_enabled(&plugin_id, enabled)
            .with_trace_id(&trace_id)?;
    }

    // 全部持久化成功后同步触发词索引（第三方组件 id 可能与 plugin_id 不一致，
    // EnabledChanged 事件按组件 id 处理无法命中，这里按 plugin_id 直调兜底；
    // 内置组件 component_id == plugin_id，事件异步到达后为幂等重入）。
    dispatcher.set_plugin_enabled(&plugin_id, enabled);

    Ok(())
}

/// 获取插件 stderr 日志的最近 N 行。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn plugin_get_logs(
    plugin_id: String,
    tail_lines: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let plugin_manager = state.get_plugin_manager();
    plugin_manager
        .get_logs(&plugin_id, tail_lines.unwrap_or(50))
        .with_trace_id(&trace_id)
}
