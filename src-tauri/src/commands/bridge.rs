use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::plugin_framework::inspector::InspectedQueryEvent;
use crate::plugin_framework::{ConfirmOutcome, ConfirmRequest, SessionDispatcher};
use crate::state::app_state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tracing::{debug, info};
use zerolaunch_plugin_api::common::ImageUtils;
use zerolaunch_plugin_api::plugin::PluginKind;
use zerolaunch_plugin_api::{CandidateId, Query, QueryResponse, ResultAction};
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

impl From<ResultAction> for BridgeResultAction {
    fn from(action: ResultAction) -> Self {
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
    #[serde(rename = "generation")]
    pub generation: u64,
    /// 候选缓存世代：前端确认时回传校验（防止缓存刷新后 id 漂移执行错误候选）。
    #[serde(rename = "candidateGeneration")]
    pub candidate_generation: u64,
    #[serde(rename = "results")]
    pub results: Vec<BridgeSearchResult>,
    #[serde(rename = "panelType", default)]
    pub panel_type: Option<String>,
    #[serde(rename = "panelData", default)]
    pub panel_data: Option<serde_json::Value>,
    #[serde(rename = "panelActions", default)]
    pub panel_actions: Option<Vec<BridgeResultAction>>,
    /// 行内参数模式数据（仅 mode="inline_param" 时有值）
    #[serde(rename = "inlineParam", default)]
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

/// 确认请求载荷 —— `bridge_confirm` 的 IPC 请求契约（Deserialize 侧），
/// 与 Dispatcher 的 `ConfirmRequest` 一一对应（命令层构造后透传，无 JSON 载荷往返）。
///
/// 由前端构造并经 `bridge_confirm` 下发；两种载荷对应两条确认语义：
/// - `candidate`：宿主候选确认（默认搜索执行 / 插件面板默认动作）；
/// - `pluginAction`：插件面板动作（面板按键契约 Custom / GotoPanel 回插件）。
#[derive(Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum ConfirmRequestPayload {
    /// 宿主候选确认：执行候选项（缺参数时引导参数面板）。
    #[serde(rename = "candidate")]
    Candidate {
        /// 目标候选项 ID。
        #[serde(rename = "candidateId")]
        candidate_id: u64,
        /// 动作 ID。
        #[serde(rename = "actionId")]
        action_id: String,
        /// 发起确认时的查询文本。
        #[serde(rename = "queryText")]
        query_text: String,
        /// 用户参数（行内参数/参数面板场景；缺省为未提供）。
        #[serde(rename = "userArgs")]
        user_args: Option<Vec<String>>,
        /// 会话代际：前端最后一次观测到的代际，后端据此拒绝过期确认（必填，见设计 §5.4）。
        #[serde(rename = "generation")]
        generation: u64,
        /// 候选缓存世代：前端最后一次观测到的缓存世代（refresh_candidates 递增），
        /// 后端校验与当前缓存一致，防止刷新后 id 漂移执行错误候选。
        /// 缺省 0 = 旧前端未携带，后端跳过校验（兼容）。
        #[serde(rename = "candidateGeneration", default)]
        candidate_generation: u64,
    },
    /// 插件面板动作：自定义能力调用（面板按键契约 Custom / GotoPanel 回插件）。
    #[serde(rename = "pluginAction")]
    PluginAction {
        /// 声明发起动作的插件（Dispatcher 路由时校验归属，须与活动会话一致）。
        #[serde(rename = "pluginId")]
        plugin_id: String,
        /// 插件动作 ID（插件 `execute_action` 的分支名）。
        #[serde(rename = "action")]
        action: String,
        /// 插件自定义载荷（自由 JSON）。
        #[serde(rename = "args", default)]
        args: serde_json::Value,
        /// 会话代际：前端最后一次观测到的代际，后端据此拒绝过期面板动作（必填，见设计 §5.4）。
        #[serde(rename = "generation")]
        generation: u64,
    },
}

/// 确认执行响应 —— 由 `route_confirm` 返回的 `RoutedConfirm` 映射而来（IPC 序列化契约）。
/// Executed 表示动作已执行完成；EnterParamPanel 表示确认后需要更多用户输入
/// （参数面板，核心程序专属形态——载荷自包含：候选 ID + 参数个数，
/// 前端据此构造输入字段，无需依赖列表项）。
/// 两个变体均携带当前会话代际：投影转换后前端无需等下一次查询即可更新投影。
#[derive(Serialize, Debug)]
#[serde(tag = "status")]
pub enum BridgeConfirmResponse {
    #[serde(rename = "executed")]
    Executed {
        #[serde(rename = "generation")]
        generation: u64,
        /// 确认后最新候选缓存世代（前端更新本地，后续确认回传）。
        #[serde(rename = "candidateGeneration")]
        candidate_generation: u64,
    },
    #[serde(rename = "enterParamPanel")]
    EnterParamPanel {
        #[serde(rename = "candidateId")]
        candidate_id: CandidateId,
        #[serde(rename = "userArgCount")]
        user_arg_count: usize,
        #[serde(rename = "generation")]
        generation: u64,
        /// 确认后最新候选缓存世代（前端更新本地，后续确认回传）。
        #[serde(rename = "candidateGeneration")]
        candidate_generation: u64,
    },
}

/// 查询请求载荷（bridge_query 参数）。
#[derive(Deserialize, Debug)]
pub struct QueryPayload {
    /// 原始查询文本。
    #[serde(rename = "rawQuery")]
    raw_query: String,
    /// 查询是否由用户显式确认（如按 Enter）触发（语义见 bridge_query 文档）。
    #[serde(rename = "confirm")]
    confirm: bool,
    /// 沉浸式面板数据通道：面板内输入查询时显式指定目标插件
    /// （经只读面板入口直调其 query()）；None = 经 UI 入口路由（触发词/默认搜索）。
    #[serde(rename = "panelPluginId")]
    panel_plugin_id: Option<String>,
}

/// 通用查询入口（含沉浸式面板数据通道）。
/// 前端搜索输入变化时调用此命令，后端经 SessionDispatcher 路由到搜索引擎或插件；
/// 图标会被解析为 base64 data URL，前端 IconDisplay 可直接渲染。
///
/// `confirm`：查询是否由用户显式确认（如按 Enter）触发，前端必须显式传入：
/// - `false`：输入/路由触发的预览查询。行内插件手动模式（OnEnter）下返回 ready 提示
///   （不执行面板动作），并承担路由职责——文本回退到插件触发词之外时回落搜索、退出面板。
/// - `true`：用户按 Enter 触发的确认查询。OnEnter 模式下插件据此直接执行动作（如翻译）。
/// 自动模式（OnInput）与普通搜索忽略该标志，行为与旧版一致。
///
/// `payload.panel_plugin_id`：沉浸式面板数据通道（面板内输入查询，显式指定目标插件）。
/// 有值 → 经只读面板入口直调该插件 query()（只读辅助路径，不改写会话）；
/// None → 经 UI 入口路由（触发词/默认搜索）。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn bridge_query(
    state: tauri::State<'_, Arc<AppState>>,
    payload: QueryPayload,
) -> Result<BridgeQueryResponse, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let QueryPayload {
        raw_query,
        confirm,
        panel_plugin_id,
    } = payload;
    debug!("[Bridge] 查询: '{}'", raw_query);

    let session_dispatcher = state.get_session_dispatcher();

    let query = Query {
        id: trace_id.clone(),
        raw_query: raw_query.clone(),
        search_term: raw_query.to_lowercase(),
        confirm,
    };

    let query_start = std::time::Instant::now();
    // 面板查询走只读入口（直调指定插件）；否则走 UI 入口（唯一可改写会话的查询入口）。
    let routed = match panel_plugin_id.as_deref() {
        Some(plugin_id) => {
            session_dispatcher
                .route_query_panel(&trace_id, &query, plugin_id)
                .await
        }
        None => session_dispatcher.route_query_ui(&trace_id, &query).await,
    }
    .with_trace_id(&trace_id)?;

    // 录制查询事件到 Inspector（仅在调试模式开启时）
    // 统一词表：空结果合并为 search（展示形态层面不区分 List/Empty）。
    let (mode, result_count) = match &routed.response {
        QueryResponse::List { results } => ("search", results.len()),
        QueryResponse::Empty => ("search", 0),
        QueryResponse::CustomPanel { .. } => ("plugin_panel", 1),
        QueryResponse::InlineParam { .. } => ("inline_param", 0),
    };
    if state.is_debug_mode() {
        if let Some(inspector) = state.get_inspector() {
            inspector.record(InspectedQueryEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                trace_id: trace_id.clone(),
                raw_query: raw_query.clone(),
                mode: mode.to_string(),
                result_count,
                duration_ms: query_start.elapsed().as_millis() as u64,
                owner_id: routed
                    .plugin_id
                    .clone()
                    .unwrap_or_else(|| "default-search".to_string()),
            });
            let _ = state.get_main_handle().emit("inspector-state-updated", ());
        }
    }

    match routed.response {
        QueryResponse::List { results } => {
            let core_handle = state.get_core_handle();

            // 解析图标：L1 缓存命中率高，几乎零开销；未命中由 L2 文件缓存兜底。
            // IconRequest::Data（插件候选 data URL）经提取链路解码直通。
            let mut bridge_results = Vec::with_capacity(results.len());
            for item in results {
                let icon_data = {
                    let data = core_handle.get_icon_or_default(item.icon.clone()).await;
                    ImageUtils::to_data_url(&data)
                };
                bridge_results.push(BridgeSearchResult {
                    id: item.id,
                    title: item.title,
                    subtitle: item.subtitle,
                    icon: icon_data,
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
                generation: routed.generation,
                candidate_generation: session_dispatcher.get_candidates_generation(),
                results: bridge_results,
                panel_type: None,
                panel_data: None,
                panel_actions: None,
                inline_param: None,
            })
        }
        QueryResponse::Empty => {
            info!("[Bridge] 查询完成: '{}' -> 0 个结果", raw_query);
            // 统一词表：空结果合并入 search（前端行为与原 'empty' 分支相同）。
            Ok(BridgeQueryResponse {
                mode: "search".to_string(),
                generation: routed.generation,
                candidate_generation: session_dispatcher.get_candidates_generation(),
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
            ..
        } => {
            let mode = if keep_search_bar {
                "plugin_panel"
            } else {
                "plugin_immersive"
            };
            // 第三方插件 panel_type 统一为 third-party:<id>（前端 provider 匹配契约）
            let kind = routed
                .plugin_id
                .as_deref()
                .and_then(|id| {
                    session_dispatcher
                        .plugin_registry()
                        .get_metadata(id)
                        .map(|m| m.kind)
                })
                .unwrap_or(PluginKind::Builtin);
            let panel_type =
                SessionDispatcher::normalize_panel_type(&routed.plugin_id, kind, &panel_type);
            info!(
                "[Bridge] 查询完成: '{}' -> 插件面板 '{}' ({})",
                raw_query, panel_type, mode
            );
            Ok(BridgeQueryResponse {
                mode: mode.to_string(),
                generation: routed.generation,
                candidate_generation: session_dispatcher.get_candidates_generation(),
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
                generation: routed.generation,
                candidate_generation: session_dispatcher.get_candidates_generation(),
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
#[tracing::instrument(skip(state, payload), fields(trace_id))]
pub async fn bridge_confirm(
    state: tauri::State<'_, Arc<AppState>>,
    payload: ConfirmRequestPayload,
) -> Result<BridgeConfirmResponse, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let kind = match &payload {
        ConfirmRequestPayload::Candidate { .. } => "candidate",
        ConfirmRequestPayload::PluginAction { .. } => "pluginAction",
    };
    debug!("[Bridge] 确认请求: kind={}", kind);

    let session_dispatcher = state.get_session_dispatcher();

    // 请求全程类型化：载荷与 Dispatcher 的 ConfirmRequest 一一对应，直接透传（无 JSON 往返）。
    // 归属校验（插件动作须属于活动会话插件）与代际校验在 Dispatcher 路由层完成（薄命令层）。
    let req = match payload {
        ConfirmRequestPayload::Candidate {
            candidate_id,
            action_id,
            query_text,
            user_args,
            generation,
            candidate_generation,
        } => ConfirmRequest::Candidate {
            candidate_id: candidate_id as CandidateId,
            action_id,
            query_text,
            user_args: user_args.unwrap_or_default(),
            generation,
            candidate_generation,
        },
        ConfirmRequestPayload::PluginAction {
            plugin_id,
            action,
            args,
            generation,
        } => ConfirmRequest::PluginAction {
            plugin_id,
            action,
            args,
            generation,
        },
    };

    let routed = match session_dispatcher.route_confirm(&trace_id, req).await {
        Ok(routed) => routed,
        Err(e) => {
            // 先隐藏窗口后执行（launcher 瞬时工具语义）：失败时窗口已不可见，
            // 系统通知是唯一反馈通道（沿用现有硬编码中文通知模式）。
            let bridge_err: BridgeError = BridgeError::from(e).with_trace_id(&trace_id);
            let msg = format!("执行失败: {}", bridge_err.message);
            state.get_host_api().notify("ZeroLaunch", &msg).await;
            return Err(bridge_err);
        }
    };

    Ok(match routed.outcome {
        ConfirmOutcome::Executed => BridgeConfirmResponse::Executed {
            generation: routed.generation,
            candidate_generation: session_dispatcher.get_candidates_generation(),
        },
        ConfirmOutcome::EnterParamPanel {
            candidate_id,
            user_arg_count,
        } => BridgeConfirmResponse::EnterParamPanel {
            candidate_id,
            user_arg_count,
            generation: routed.generation,
            candidate_generation: session_dispatcher.get_candidates_generation(),
        },
    })
}

// ============================================================================
// 会话管理接口
// ============================================================================

/// 唤醒搜索栏时调用。
/// 捕获系统参数快照（选中文本、窗口句柄等）。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn bridge_wake(state: tauri::State<'_, Arc<AppState>>) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    debug!("📸 [Bridge] 搜索栏唤醒");
    let session_dispatcher = state.get_session_dispatcher();
    session_dispatcher
        .on_search_bar_wake()
        .await
        .with_trace_id(&trace_id)?;
    Ok(())
}

/// 插件热键唤醒（前端驱动）：搜索栏唤起后由前端匹配插件声明热键并调用。
/// 空查询进入插件面板会话（载荷经 session-state panelContent 推送），随后确保窗口可见。
/// 插件热键不注册 OS 全局热键——OS 级只负责搜索栏呼出（Alt+Space），
/// 本命令是插件唤醒的唯一入口。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn bridge_wake_plugin(
    state: tauri::State<'_, Arc<AppState>>,
    plugin_id: String,
) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    debug!("📸 [Bridge] 插件热键唤醒: {}", plugin_id);
    let session_dispatcher = state.get_session_dispatcher();
    session_dispatcher
        .wake_plugin(&plugin_id)
        .await
        .with_trace_id(&trace_id)?;
    // 窗口已在搜索栏会话中；此处兜底确保可见（前端驱动场景下通常已可见）
    state.get_host_api().show_window().await;
    Ok(())
}

// ============================================================================
// 候选缓存管理
// ============================================================================

/// 强制刷新候选项缓存。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn bridge_refresh_candidates(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<usize, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    debug!("🔄 [Bridge] 刷新候选项缓存");
    let session_dispatcher = state.get_session_dispatcher();
    session_dispatcher.refresh_candidates().await;
    let count = session_dispatcher.get_cached_candidates_count();
    info!("🔄 [Bridge] 刷新完成，共 {} 个候选项", count);
    Ok(count)
}

/// 获取缓存的候选项数量。
#[tauri::command]
pub fn bridge_get_candidates_count(state: tauri::State<'_, Arc<AppState>>) -> usize {
    state.get_session_dispatcher().get_cached_candidates_count()
}

/// 隐藏搜索栏窗口。
/// 前端确认执行、Esc 退出等场景统一通过此命令委托后端隐藏窗口。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn bridge_hide_window(state: tauri::State<'_, Arc<AppState>>) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state.get_host_api().hide_window().await;
    Ok(())
}

/// 查询系统主题（"light"/"dark"），未经宿主显式配置解析。
/// 前端 system 模式跟随的唯一数据源：初始值经本命令获取，运行期变化经
/// system-theme-changed 事件推送（两者语义一致，见 bootstrap 注册表监听）。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn bridge_get_system_theme(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let theme = state
        .get_core_handle()
        .get_system_theme()
        .with_trace_id(&trace_id)?;
    Ok(theme.as_str().to_string())
}
