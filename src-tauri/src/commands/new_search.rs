// 该文件用于临时向前端传递可用的命令，后续需要被重构！

use crate::plugin_system::types::{Query, QueryResponse};
use crate::state::app_state::AppState;
use crate::utils::service_locator::ServiceLocator;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Serialize, Debug)]
pub struct NewSearchResult {
    pub id: u64,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub score: f64,
    pub actions: Vec<NewSearchAction>,
}

#[derive(Serialize, Debug)]
pub struct NewSearchAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub is_default: bool,
    pub shortcut_key: String,
}

impl From<crate::plugin_system::types::ResultAction> for NewSearchAction {
    fn from(action: crate::plugin_system::types::ResultAction) -> Self {
        NewSearchAction {
            id: action.id,
            label: action.label,
            icon: action.icon.value().to_string(),
            is_default: action.is_default,
            shortcut_key: action.shortcut_key,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct NewSearchResponse {
    pub results: Vec<NewSearchResult>,
    pub mode: String,
}

#[derive(Deserialize, Debug)]
pub struct NewLaunchPayload {
    pub candidate_id: u64,
}

/// 使用新架构执行搜索
#[tauri::command]
pub async fn handle_new_search(
    state: tauri::State<'_, Arc<AppState>>,
    search_text: String,
) -> Result<NewSearchResponse, String> {
    debug!("🔍 [新架构] 处理搜索请求: '{}'", search_text);

    let session_router = state.get_session_router();

    let trace_id = Uuid::new_v4().to_string()[..8].to_string();

    let query = Query {
        id: trace_id.clone(),
        raw_query: search_text.clone(),
        search_term: search_text.to_lowercase(),
    };

    let response = session_router.route_query(&trace_id, &query).await;

    match response {
        QueryResponse::List { results } => {
            let new_results: Vec<NewSearchResult> = results
                .into_iter()
                .map(|item| NewSearchResult {
                    id: item.id,
                    title: item.title,
                    subtitle: item.subtitle,
                    icon: item.icon.value().to_string(),
                    score: item.score,
                    actions: item.actions.into_iter().map(|a| a.into()).collect(),
                })
                .collect();

            info!(
                "🔍 [新架构] 搜索完成: '{}' -> {} 个结果",
                search_text,
                new_results.len()
            );

            Ok(NewSearchResponse {
                results: new_results,
                mode: "search".to_string(),
            })
        }
        QueryResponse::Empty => {
            info!("🔍 [新架构] 搜索完成: '{}' -> 0 个结果", search_text);
            Ok(NewSearchResponse {
                results: Vec::new(),
                mode: "search".to_string(),
            })
        }
        _ => {
            info!("🔍 [新架构] 搜索完成: '{}' -> 未知响应类型", search_text);
            Ok(NewSearchResponse {
                results: Vec::new(),
                mode: "unknown".to_string(),
            })
        }
    }
}

/// 使用新架构启动候选项
#[tauri::command]
pub async fn handle_new_launch(
    state: tauri::State<'_, Arc<AppState>>,
    candidate_id: u64,
    action_id: String,
    query_text: String,
) -> Result<(), String> {
    debug!(
        "🚀 [新架构] 启动候选项: id={}, action={}, query='{}'",
        candidate_id, action_id, query_text
    );

    let session_router = state.get_session_router();

    let trace_id = Uuid::new_v4().to_string()[..8].to_string();

    let payload = serde_json::json!({
        "candidate_id": candidate_id,
        "query_text": query_text
    });

    session_router
        .route_confirm(&trace_id, &action_id, payload)
        .await?;

    info!("🚀 [新架构] 启动成功: id={}", candidate_id);
    Ok(())
}

/// 获取新架构的候选项数量
#[tauri::command]
pub fn get_new_candidates_count() -> usize {
    let state = ServiceLocator::get_state();
    let session_router = state.get_session_router();
    session_router.get_cached_candidates_count()
}

/// 刷新新架构的候选项缓存
#[tauri::command]
pub async fn refresh_new_candidates(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    debug!("🔄 [新架构] 刷新候选项缓存");
    let session_router = state.get_session_router();
    session_router.refresh_candidates();
    let count = session_router.get_cached_candidates_count();
    info!("🔄 [新架构] 刷新完成，共 {} 个候选项", count);
    Ok(count)
}
