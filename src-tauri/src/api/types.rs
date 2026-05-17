use crate::core::config::models::{ComponentInfo, ComponentSchema};
use crate::plugin_system::types::{
    ListItem, PluginMetadata, QueryResponse, ResultAction, SearchCandidate,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// 搜索
// ============================================================================

/// HTTP API 搜索结果项（不含 base64 图标，仅返回路径字符串）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSearchResult {
    pub id: u64,
    pub title: String,
    pub subtitle: String,
    pub icon_path: String,
    pub score: f64,
    pub actions: Vec<ApiResultAction>,
    pub target_type: String,
}

impl From<ListItem> for ApiSearchResult {
    fn from(item: ListItem) -> Self {
        ApiSearchResult {
            id: item.id,
            title: item.title,
            subtitle: item.subtitle,
            icon_path: item.icon.value().to_string(),
            score: item.score,
            actions: item.actions.into_iter().map(|a| a.into()).collect(),
            target_type: item.target_type,
        }
    }
}

/// HTTP API 动作项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResultAction {
    pub id: String,
    pub label: String,
    pub icon_path: String,
    pub is_default: bool,
    pub shortcut_key: String,
}

impl From<ResultAction> for ApiResultAction {
    fn from(action: ResultAction) -> Self {
        ApiResultAction {
            id: action.id,
            label: action.label,
            icon_path: action.icon.value().to_string(),
            is_default: action.is_default,
            shortcut_key: action.shortcut_key,
        }
    }
}

/// 搜索查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSearchResponse {
    pub query: String,
    pub mode: String,
    pub result_count: usize,
    pub results: Vec<ApiSearchResult>,
    /// 仅当 mode 为 plugin_panel 或 plugin_immersive 时有值
    pub panel_data: Option<serde_json::Value>,
}

impl ApiSearchResponse {
    pub fn from_internal(response: QueryResponse, query: &str) -> Self {
        match response {
            QueryResponse::List { results } => ApiSearchResponse {
                query: query.to_string(),
                mode: "search".to_string(),
                result_count: results.len(),
                results: results.into_iter().map(|item| item.into()).collect(),
                panel_data: None,
            },
            QueryResponse::Empty => ApiSearchResponse {
                query: query.to_string(),
                mode: "empty".to_string(),
                result_count: 0,
                results: Vec::new(),
                panel_data: None,
            },
            QueryResponse::CustomPanel {
                panel_type,
                data,
                actions: _,
                keep_search_bar,
            } => {
                let mode = if keep_search_bar {
                    "plugin_panel"
                } else {
                    "plugin_immersive"
                };
                let mut panel = serde_json::Map::new();
                panel.insert(
                    "panelType".to_string(),
                    serde_json::Value::String(panel_type),
                );
                panel.insert("data".to_string(), data);
                ApiSearchResponse {
                    query: query.to_string(),
                    mode: mode.to_string(),
                    result_count: 0,
                    results: Vec::new(),
                    panel_data: Some(serde_json::Value::Object(panel)),
                }
            }
        }
    }
}

// ============================================================================
// 候选列表
// ============================================================================

/// 单个候选项的简化视图
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCandidate {
    pub id: u64,
    pub name: String,
    pub target_type: String,
    pub keywords: Vec<String>,
    pub bias: f64,
}

impl From<&SearchCandidate> for ApiCandidate {
    fn from(c: &SearchCandidate) -> Self {
        ApiCandidate {
            id: c.id,
            name: c.name.clone(),
            target_type: c.target.target_type().as_str().to_string(),
            keywords: c.keywords.clone(),
            bias: c.bias,
        }
    }
}

/// 候选列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCandidatesResponse {
    pub total_count: usize,
    pub candidates: Vec<ApiCandidate>,
}

// ============================================================================
// 通用包装
// ============================================================================

/// 计数响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCountResponse {
    pub count: usize,
}

/// 会话模式响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSessionModeResponse {
    pub mode: String,
}

/// 组件列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiComponentsResponse {
    pub total_count: usize,
    pub components: Vec<ComponentInfo>,
}

/// 组件 Schema 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiComponentSchemaResponse {
    pub schema: ComponentSchema,
}

/// 组件配置响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiComponentSettingsResponse {
    pub component_id: String,
    pub settings: serde_json::Value,
}

/// 插件列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiPluginsResponse {
    pub total_count: usize,
    pub plugins: Vec<PluginMetadata>,
}

/// 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHealth {
    pub status: String,
    pub version: String,
    pub candidates_count: usize,
    pub session_mode: String,
}

/// 搜索查询参数
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
}
