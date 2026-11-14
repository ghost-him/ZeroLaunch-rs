//! 图标总览页 IPC 命令。
//!
//! 供设置页「图标」自定义页面列出候选缓存中的全部图标，并提供整体重试。
//! 列表只读图标缓存并立即返回；缺失图标交给后台预取（与查询共用去重与限流），
//! 逐项推送 `bridge-icon-updated` 由前端回填。

use crate::commands::bridge::prefetch_icons;
use crate::commands::bridge_error::BridgeError;
use crate::sdk::HostApi;
use crate::state::app_state::AppState;
use crate::utils::trace_id::generate_trace_id;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;
use zerolaunch_plugin_api::common::ImageUtils;
use zerolaunch_plugin_api::host::{CacheLevel, PluginHost};
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{CandidateId, SearchCandidate};

/// 图标总览页单条数据 —— 跨 IPC 序列化。
///
/// 仅由本模块的命令返回，供设置页「图标」页面渲染，不参与搜索链路。
#[derive(Debug, Clone, Serialize)]
pub struct GalleryIconEntry {
    /// 候选项 id（与候选缓存一致）。
    #[serde(rename = "id")]
    pub id: CandidateId,
    /// 候选项展示名称。
    #[serde(rename = "name")]
    pub name: String,
    /// 图标来源：网址、文件路径或扩展名；内嵌图标为空串（前端显示固定文案）。
    #[serde(rename = "source")]
    pub source: String,
    /// 图标 data URL；无缓存图标时为空串，前端先渲染占位图标。
    #[serde(rename = "icon")]
    pub icon: String,
}

/// 图标总览页列表 —— 跨 IPC 序列化。
#[derive(Debug, Clone, Serialize)]
pub struct GalleryIconList {
    /// 候选缓存世代：前端据此过滤后台回填事件。
    #[serde(rename = "generation")]
    pub generation: u64,
    /// 全部图标条目。
    #[serde(rename = "entries")]
    pub entries: Vec<GalleryIconEntry>,
}

/// 展示上限：候选缓存可能很大，限制单次 IPC 载荷与首屏渲染量。
const MAX_GALLERY_ICONS: usize = 500;

/// 从候选缓存中取出带图标来源的候选项（空图标表示不需要图标，不入列表）。
/// 参数：state - 应用状态。
/// 返回：保持缓存顺序、最多 MAX_GALLERY_ICONS 条的候选项。
fn icon_candidates(state: &AppState) -> Vec<SearchCandidate> {
    state
        .get_session_dispatcher()
        .get_cached_candidates()
        .into_iter()
        .filter(|candidate| !candidate.icon.is_empty())
        .take(MAX_GALLERY_ICONS)
        .collect()
}

/// 读取单条候选项的已缓存图标。
/// 参数：host_api - 宿主执行体；candidate - 候选项。
/// 返回：图标 data URL；未缓存时为空串（未命中属于正常状态，不是错误）。
async fn cached_icon(host_api: &Arc<HostApi>, candidate: &SearchCandidate) -> String {
    match host_api
        .get_icon(&candidate.icon, CacheLevel::CacheOnly)
        .await
    {
        Ok(data) => ImageUtils::to_data_url(&data),
        Err(_) => String::new(),
    }
}

/// 图标来源的展示文本：内嵌图标（data URL）不把整段载荷当作来源展示。
/// 参数：icon - 图标请求。
/// 返回：网址 / 路径 / 扩展名；内嵌图标返回空串，由前端显示固定文案。
fn source_text(icon: &IconRequest) -> String {
    match icon {
        IconRequest::Data(_) => String::new(),
        other => other.value().to_string(),
    }
}

/// 组装列表：只读缓存立即返回，缺失图标交给后台预取逐项回填。
/// 参数：state - 应用状态；retry - true 时先清空全部失败记录。
/// 返回：当前候选世代与图标条目，保持候选缓存顺序。
async fn collect_icons(state: &AppState, retry: bool) -> GalleryIconList {
    let candidates = icon_candidates(state);
    let host_api = state.get_host_api();
    if retry {
        host_api.clear_icon_failures();
    }
    let generation = state.get_session_dispatcher().get_candidates_generation();

    let mut entries = Vec::with_capacity(candidates.len());
    let mut missing = Vec::new();
    for candidate in &candidates {
        let icon = cached_icon(&host_api, candidate).await;
        if icon.is_empty() {
            missing.push((candidate.id, candidate.icon.clone()));
        }
        entries.push(GalleryIconEntry {
            id: candidate.id,
            name: candidate.name.clone(),
            source: source_text(&candidate.icon),
            icon,
        });
    }

    if !missing.is_empty() {
        prefetch_icons(
            state.get_main_handle(),
            state.get_core_handle(),
            generation,
            missing,
        );
    }

    GalleryIconList {
        generation,
        entries,
    }
}

/// 列出候选缓存中的全部图标；未缓存条目由后台补齐后经 `bridge-icon-updated` 回填。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn icon_gallery_list(
    state: State<'_, Arc<AppState>>,
) -> Result<GalleryIconList, BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    Ok(collect_icons(&state, false).await)
}

/// 整体重试：清空全部失败记录，重新排队补齐列表中未缓存的图标。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn icon_gallery_retry_all(
    state: State<'_, Arc<AppState>>,
) -> Result<GalleryIconList, BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    Ok(collect_icons(&state, true).await)
}
