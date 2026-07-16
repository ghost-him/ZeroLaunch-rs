//! 调试工具 IPC 命令。
//! 所有命令仅在调试模式开启时可用（通过 AppState::is_debug_mode() 检查）。
//! 返回类型遵循 serde-rename 规范，每个字段显式标注 `#[serde(rename)]`。

use crate::commands::bridge_error::BridgeError;
use crate::state::app_state::AppState;
use serde::Serialize;
use std::sync::Arc;

// ---- 响应类型 ----

/// 搜索性能测试结果
#[derive(Debug, Clone, Serialize)]
pub struct SearchTimingResult {
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    #[serde(rename = "resultCount")]
    pub result_count: usize,
    #[serde(rename = "totalCandidates")]
    pub total_candidates: usize,
}

/// 索引性能测试结果
#[derive(Debug, Clone, Serialize)]
pub struct IndexTimingResult {
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    #[serde(rename = "candidateCount")]
    pub candidate_count: usize,
}

/// 搜索匹配详情条目（合并 ScoredCandidate + SearchCandidate 数据）
#[derive(Debug, Clone, Serialize)]
pub struct SearchDetailItem {
    #[serde(rename = "rank")]
    pub rank: usize,
    #[serde(rename = "candidateId")]
    pub candidate_id: u64,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "score")]
    pub score: f64,
    #[serde(rename = "targetType")]
    pub target_type: String,
    #[serde(rename = "keywords")]
    pub keywords: Vec<String>,
}

// ---- 命令 ----

/// 搜索性能测试：返回耗时、结果数、候选总数。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn debug_test_search_time(
    state: tauri::State<'_, Arc<AppState>>,
    query: String,
) -> Result<SearchTimingResult, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    if !state.is_debug_mode() {
        return Err(
            BridgeError::internal("调试模式未开启，请在设置中启用").with_trace_id(&trace_id)
        );
    }

    let session_router = state.get_session_router();
    let start = std::time::Instant::now();
    let scored = session_router.debug_search(&query);
    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(SearchTimingResult {
        duration_ms,
        result_count: scored.len(),
        total_candidates: session_router.get_cached_candidates_count(),
    })
}

/// 索引性能测试：重新采集候选项并返回耗时和候选总数。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn debug_test_index_time(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<IndexTimingResult, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    if !state.is_debug_mode() {
        return Err(
            BridgeError::internal("调试模式未开启，请在设置中启用").with_trace_id(&trace_id)
        );
    }

    let session_router = state.get_session_router();
    let (duration_ms, candidate_count) = session_router.debug_index_with_timing().await;

    Ok(IndexTimingResult {
        duration_ms,
        candidate_count,
    })
}

/// 搜索关键字生成：输入名称，返回所有生成的关键字。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn debug_get_search_keys(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
) -> Result<Vec<String>, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    if !state.is_debug_mode() {
        return Err(
            BridgeError::internal("调试模式未开启，请在设置中启用").with_trace_id(&trace_id)
        );
    }

    let session_router = state.get_session_router();
    let keywords = session_router.debug_generate_keywords(&name).await;
    Ok(keywords)
}

/// 搜索匹配详情：输入查询词，返回逐项分数、关键词、类型等。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn debug_search_detail(
    state: tauri::State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<SearchDetailItem>, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    if !state.is_debug_mode() {
        return Err(
            BridgeError::internal("调试模式未开启，请在设置中启用").with_trace_id(&trace_id)
        );
    }

    let session_router = state.get_session_router();
    let scored = session_router.debug_search(&query);

    let items: Vec<SearchDetailItem> = scored
        .into_iter()
        .enumerate()
        .filter_map(|(i, sc)| {
            let candidate = session_router.get_cached_candidate_by_id(sc.candidate_id)?;
            Some(SearchDetailItem {
                rank: i + 1,
                candidate_id: sc.candidate_id,
                name: candidate.name.clone(),
                score: sc.score,
                target_type: candidate.target.target_type().as_str().to_string(),
                keywords: candidate.keywords.clone(),
            })
        })
        .collect();

    Ok(items)
}

/// 模拟一次查询，返回原始 QueryResponse（不含图标解析）。
/// 若调试模式未开启，返回错误。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn debug_simulate_query(
    state: tauri::State<'_, Arc<AppState>>,
    raw_query: String,
) -> Result<serde_json::Value, BridgeError> {
    let trace_id = format!("sim-{}", crate::utils::trace_id::generate_trace_id());
    tracing::Span::current().record("trace_id", trace_id.as_str());
    if !state.is_debug_mode() {
        return Err(
            BridgeError::internal("调试模式未开启，请在设置中启用").with_trace_id(&trace_id)
        );
    }

    use zerolaunch_plugin_api::plugin::Query;

    let session_router = state.get_session_router();
    let query = Query {
        id: trace_id.clone(),
        raw_query: raw_query.clone(),
        search_term: raw_query.to_lowercase(),
    };
    let response = session_router.route_query(&trace_id, &query).await;
    Ok(serde_json::to_value(&response).unwrap_or_default())
}
