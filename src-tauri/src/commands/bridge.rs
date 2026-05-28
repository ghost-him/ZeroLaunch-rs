use crate::core::types::BridgeError;
use crate::plugin_system::types::{ConfirmResult, Query, QueryResponse};
use crate::state::app_state::AppState;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

// ============================================================================
// 搜索接口
// ============================================================================

#[derive(Serialize, Debug)]
pub struct BridgeSearchResult {
    #[serde(rename = "id")]
    pub id: u64,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "subtitle")]
    pub subtitle: String,
    #[serde(rename = "icon")]
    pub icon: String,
    #[serde(rename = "score")]
    pub score: f64,
    #[serde(rename = "actions")]
    pub actions: Vec<BridgeResultAction>,
    #[serde(rename = "targetType")]
    pub target_type: String,
    #[serde(rename = "userArgCount")]
    pub user_arg_count: usize,
    #[serde(rename = "hasSystemParams")]
    pub has_system_params: bool,
    #[serde(rename = "triggerKeywords")]
    pub trigger_keywords: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct BridgeResultAction {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "icon")]
    pub icon: String,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
    #[serde(rename = "shortcutKey")]
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
pub struct BridgeQueryResponse {
    #[serde(rename = "mode")]
    pub mode: String,
    #[serde(rename = "results")]
    pub results: Vec<BridgeSearchResult>,
    #[serde(rename = "panelType")]
    pub panel_type: Option<String>,
    #[serde(rename = "panelData")]
    pub panel_data: Option<serde_json::Value>,
    #[serde(rename = "panelActions")]
    pub panel_actions: Option<Vec<BridgeResultAction>>,
    /// 行内参数模式数据（仅 mode="inline_param" 时有值）
    #[serde(rename = "inlineParam")]
    pub inline_param: Option<BridgeInlineParamData>,
}

/// 行内参数模式携带的数据。
#[derive(Serialize, Debug)]
pub struct BridgeInlineParamData {
    #[serde(rename = "candidateId")]
    pub candidate_id: u64,
    #[serde(rename = "triggerKeyword")]
    pub trigger_keyword: String,
    #[serde(rename = "userArgCount")]
    pub user_arg_count: usize,
}

/// 确认执行负载
#[derive(Deserialize, Debug)]
pub struct ConfirmPayload {
    #[serde(rename = "candidateId")]
    pub candidate_id: u64,
    #[serde(rename = "actionId")]
    pub action_id: String,
    #[serde(rename = "queryText")]
    pub query_text: String,
    #[serde(rename = "userArgs")]
    pub user_args: Option<Vec<String>>,
}

/// 确认执行响应。
/// Executed 表示已执行完成；EnterParamPanel 表示应进入参数面板。
#[derive(Serialize, Debug)]
#[serde(tag = "status")]
pub enum BridgeConfirmResponse {
    #[serde(rename = "executed")]
    Executed,
    #[serde(rename = "enterParamPanel")]
    EnterParamPanel {
        #[serde(rename = "candidateId")]
        candidate_id: u64,
        #[serde(rename = "userArgCount")]
        user_arg_count: usize,
    },
}

impl From<ConfirmResult> for BridgeConfirmResponse {
    fn from(result: ConfirmResult) -> Self {
        match result {
            ConfirmResult::Executed => BridgeConfirmResponse::Executed,
            ConfirmResult::EnterParamPanel {
                candidate_id,
                user_arg_count,
            } => BridgeConfirmResponse::EnterParamPanel {
                candidate_id,
                user_arg_count,
            },
        }
    }
}

/// 将 PNG 图标字节数据转换为前端可直接使用的 base64 data URL。
/// 参数：png_data - PNG 格式图标字节数据。
/// 返回：data:image/png;base64,... 格式的字符串，空数据返回空字符串。
fn icon_to_data_url(png_data: &[u8]) -> String {
    if png_data.is_empty() {
        return String::new();
    }
    format!("data:image/png;base64,{}", STANDARD.encode(png_data))
}

/// 通用查询入口。
/// 前端搜索输入变化时调用此命令，后端通过 SessionRouter 路由到搜索引擎或插件。
/// 图标会被解析为 base64 data URL，前端 IconDisplay 可直接渲染。
#[tauri::command]
pub async fn bridge_query(
    state: tauri::State<'_, Arc<AppState>>,
    raw_query: String,
) -> Result<BridgeQueryResponse, BridgeError> {
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
            let core_handle = state.get_core_handle();

            // 解析图标：L1 缓存命中率高，几乎零开销；未命中时由 L2 文件缓存兜底
            let mut bridge_results = Vec::with_capacity(results.len());
            for item in results {
                let icon_data = core_handle.get_icon_or_default(item.icon.clone()).await;
                bridge_results.push(BridgeSearchResult {
                    id: item.id,
                    title: item.title,
                    subtitle: item.subtitle,
                    icon: icon_to_data_url(&icon_data),
                    score: item.score,
                    actions: item.actions.into_iter().map(|a| a.into()).collect(),
                    target_type: item.target_type,
                    user_arg_count: item.user_arg_count,
                    has_system_params: item.has_system_params,
                    trigger_keywords: item.trigger_keywords,
                });
            }

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
                inline_param: None,
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
                inline_param: None,
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
                inline_param: None,
            })
        }
        QueryResponse::InlineParam {
            candidate_id,
            trigger_keyword,
            user_arg_count,
        } => {
            info!(
                "[Bridge] 进入行内参数模式: candidate_id={}, trigger='{}'",
                candidate_id, trigger_keyword
            );
            Ok(BridgeQueryResponse {
                mode: "inline_param".to_string(),
                results: Vec::new(),
                panel_type: None,
                panel_data: None,
                panel_actions: None,
                inline_param: Some(BridgeInlineParamData {
                    candidate_id,
                    trigger_keyword,
                    user_arg_count,
                }),
            })
        }
    }
}

/// 通用执行入口。
/// 用户选择一个候选项并触发动作时调用。
/// 后端判断是否执行或需要进入参数面板，返回对应状态。
#[tauri::command]
pub async fn bridge_confirm(
    state: tauri::State<'_, Arc<AppState>>,
    payload: ConfirmPayload,
) -> Result<BridgeConfirmResponse, BridgeError> {
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

    let result = session_router
        .route_confirm(&trace_id, &payload.action_id, json_payload)
        .await
        .map_err(BridgeError::internal)?;

    Ok(BridgeConfirmResponse::from(result))
}

// ============================================================================
// 会话管理接口
// ============================================================================

/// 唤醒搜索栏时调用。
/// 捕获系统参数快照（选中文本、窗口句柄等）。
#[tauri::command]
pub async fn bridge_wake(state: tauri::State<'_, Arc<AppState>>) -> Result<(), BridgeError> {
    debug!("📸 [Bridge] 搜索栏唤醒");
    let session_router = state.get_session_router();
    session_router
        .on_search_bar_wake()
        .await
        .map_err(BridgeError::internal)
}

/// 重置当前会话。
/// 通常发生在窗口隐藏或关闭时。
#[tauri::command]
pub fn bridge_reset(state: tauri::State<'_, Arc<AppState>>) {
    debug!("🔄 [Bridge] 重置会话");
    state.get_session_router().reset_session(true);
}

/// 获取当前会话模式。
#[tauri::command]
pub fn bridge_get_session_mode(state: tauri::State<'_, Arc<AppState>>) -> String {
    state
        .get_session_router()
        .current_mode()
        .as_str()
        .to_string()
}

// ============================================================================
// 候选缓存管理
// ============================================================================

/// 强制刷新候选项缓存。
#[tauri::command]
pub async fn bridge_refresh_candidates(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<usize, BridgeError> {
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

/// 隐藏搜索栏窗口。
/// 前端确认执行、Esc 退出等场景统一通过此命令委托后端隐藏窗口。
#[tauri::command]
pub async fn bridge_hide_window(state: tauri::State<'_, Arc<AppState>>) -> Result<(), BridgeError> {
    state.get_host_api().hide_window().await;
    Ok(())
}
