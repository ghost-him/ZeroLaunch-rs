use crate::plugin_system::types::{Query, QueryResponse};
use crate::state::app_state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

// ============================================================================
// 搜索接口
// ============================================================================

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSearchResult {
    pub id: u64,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub score: f64,
    pub actions: Vec<BridgeResultAction>,
    pub target_type: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeResultAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub is_default: bool,
    pub shortcut_key: String,
}

impl From<crate::plugin_system::types::ResultAction> for BridgeResultAction {
    fn from(action: crate::plugin_system::types::ResultAction) -> Self {
        BridgeResultAction {
            id: action.id,
            label: action.label,
            icon: action.icon.value().to_string(),
            is_default: action.is_default,
            shortcut_key: action.shortcut_key,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeQueryResponse {
    pub mode: String,
    pub results: Vec<BridgeSearchResult>,
    pub panel_type: Option<String>,
    pub panel_data: Option<serde_json::Value>,
    pub panel_actions: Option<Vec<BridgeResultAction>>,
}

/// 确认执行负载
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPayload {
    pub candidate_id: u64,
    pub action_id: String,
    pub query_text: String,
    pub user_args: Option<Vec<String>>,
}

/// 通用查询入口。
/// 前端搜索输入变化时调用此命令，后端通过 SessionRouter 路由到搜索引擎或插件。
#[tauri::command]
pub async fn bridge_query(
    state: tauri::State<'_, Arc<AppState>>,
    raw_query: String,
) -> Result<BridgeQueryResponse, String> {
    debug!("[Bridge] 查询: '{}'", raw_query);

    let session_router = state.get_session_router();
    let trace_id = Uuid::new_v4().to_string()[..8].to_string();

    let query = Query {
        id: trace_id.clone(),
        raw_query: raw_query.clone(),
        search_term: raw_query.to_lowercase(),
    };

    let response = session_router.route_query(&trace_id, &query).await;

    match response {
        QueryResponse::List { results } => {
            let bridge_results: Vec<BridgeSearchResult> = results
                .into_iter()
                .map(|item| BridgeSearchResult {
                    id: item.id,
                    title: item.title,
                    subtitle: item.subtitle,
                    icon: item.icon.value().to_string(),
                    score: item.score,
                    actions: item.actions.into_iter().map(|a| a.into()).collect(),
                    target_type: item.target_type,
                })
                .collect();

            info!(
                "[Bridge] 查询完成: '{}' -> {} 个结果",
                raw_query,
                bridge_results.len()
            );

            Ok(BridgeQueryResponse {
                mode: "search".to_string(),
                results: bridge_results,
                panel_type: None,
                panel_data: None,
                panel_actions: None,
            })
        }
        QueryResponse::Empty => {
            info!("[Bridge] 查询完成: '{}' -> 0 个结果", raw_query);
            Ok(BridgeQueryResponse {
                mode: "empty".to_string(),
                results: Vec::new(),
                panel_type: None,
                panel_data: None,
                panel_actions: None,
            })
        }
        QueryResponse::CustomPanel {
            panel_type,
            data,
            actions,
            keep_search_bar,
        } => {
            let mode = if keep_search_bar {
                "plugin_panel"
            } else {
                "plugin_immersive"
            };
            info!(
                "[Bridge] 查询完成: '{}' -> 插件面板 '{}' ({})",
                raw_query, panel_type, mode
            );
            Ok(BridgeQueryResponse {
                mode: mode.to_string(),
                results: Vec::new(),
                panel_type: Some(panel_type),
                panel_data: Some(data),
                panel_actions: Some(actions.into_iter().map(|a| a.into()).collect()),
            })
        }
    }
}

/// 通用执行入口。
/// 用户选择一个候选项并触发动作时调用。
#[tauri::command]
pub async fn bridge_confirm(
    state: tauri::State<'_, Arc<AppState>>,
    payload: ConfirmPayload,
) -> Result<(), String> {
    debug!(
        "[Bridge] 执行: candidate_id={}, action='{}', query='{}'",
        payload.candidate_id, payload.action_id, payload.query_text
    );

    let session_router = state.get_session_router();
    let trace_id = Uuid::new_v4().to_string()[..8].to_string();

    let json_payload = serde_json::json!({
        "candidate_id": payload.candidate_id,
        "query_text": payload.query_text,
        "user_args": payload.user_args.unwrap_or_default(),
    });

    session_router
        .route_confirm(&trace_id, &payload.action_id, json_payload)
        .await?;

    info!("[Bridge] 执行成功: candidate_id={}", payload.candidate_id);
    Ok(())
}

// ============================================================================
// 会话管理接口
// ============================================================================

/// 唤醒搜索栏时调用。
/// 捕获系统参数快照（选中文本、窗口句柄等）。
#[tauri::command]
pub async fn bridge_wake(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    debug!("📸 [Bridge] 搜索栏唤醒");
    let session_router = state.get_session_router();
    session_router.on_search_bar_wake().await
}

/// 重置当前会话。
/// 通常发生在窗口隐藏或关闭时。
#[tauri::command]
pub fn bridge_reset(state: tauri::State<'_, Arc<AppState>>) {
    debug!("🔄 [Bridge] 重置会话");
    state.get_session_router().reset_session();
}

/// 获取当前会话模式。
#[tauri::command]
pub fn bridge_get_session_mode(state: tauri::State<'_, Arc<AppState>>) -> String {
    let mode = state.get_session_router().current_mode();
    match mode {
        crate::plugin_system::SessionMode::None => "none".to_string(),
        crate::plugin_system::SessionMode::Plugin(_) => "plugin".to_string(),
        crate::plugin_system::SessionMode::Search => "search".to_string(),
    }
}

// ============================================================================
// 候选缓存管理
// ============================================================================

/// 强制刷新候选项缓存。
#[tauri::command]
pub async fn bridge_refresh_candidates(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    debug!("🔄 [Bridge] 刷新候选项缓存");
    let session_router = state.get_session_router();
    session_router.refresh_candidates().await;
    let count = session_router.get_cached_candidates_count();
    info!("🔄 [Bridge] 刷新完成，共 {} 个候选项", count);
    Ok(count)
}

/// 获取缓存的候选项数量。
#[tauri::command]
pub fn bridge_get_candidates_count(state: tauri::State<'_, Arc<AppState>>) -> usize {
    state.get_session_router().get_cached_candidates_count()
}
