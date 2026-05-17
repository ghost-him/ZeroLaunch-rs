use super::error::ApiError;
use super::read_api::ReadApi;
use super::types::*;
use axum::extract::Query as AxumQuery;
use axum::extract::{Extension, Path};
use axum::Json;
use std::sync::Arc;

/// 健康检查
pub async fn health_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
) -> Result<Json<ApiHealth>, ApiError> {
    Ok(Json(api.health().await))
}

/// 搜索候选项
pub async fn search_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
    AxumQuery(params): AxumQuery<SearchParams>,
) -> Result<Json<ApiSearchResponse>, ApiError> {
    if params.q.trim().is_empty() {
        return Err(ApiError::invalid_query("query parameter 'q' is required"));
    }
    Ok(Json(api.search(&params.q).await))
}

/// 获取所有缓存的候选项
pub async fn candidates_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
) -> Result<Json<ApiCandidatesResponse>, ApiError> {
    Ok(Json(api.get_candidates().await))
}

/// 获取指定 ID 的候选项
pub async fn candidate_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiCandidate>, ApiError> {
    Ok(Json(api.get_candidate(id).await?))
}

/// 获取候选项数量
pub async fn candidates_count_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
) -> Result<Json<ApiCountResponse>, ApiError> {
    Ok(Json(api.get_candidate_count().await))
}

/// 获取当前会话模式
pub async fn session_mode_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
) -> Result<Json<ApiSessionModeResponse>, ApiError> {
    Ok(Json(api.get_session_mode().await))
}

/// 获取所有可配置组件
pub async fn components_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
) -> Result<Json<ApiComponentsResponse>, ApiError> {
    Ok(Json(api.get_components().await))
}

/// 获取指定组件的配置 Schema
pub async fn component_schema_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
    Path(id): Path<String>,
) -> Result<Json<ApiComponentSchemaResponse>, ApiError> {
    Ok(Json(api.get_component_schema(&id).await?))
}

/// 获取指定组件的当前配置值
pub async fn component_settings_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
    Path(id): Path<String>,
) -> Result<Json<ApiComponentSettingsResponse>, ApiError> {
    Ok(Json(api.get_component_settings(&id).await?))
}

/// 获取所有已注册插件的元数据
pub async fn plugins_handler(
    Extension(api): Extension<Arc<dyn ReadApi>>,
) -> Result<Json<ApiPluginsResponse>, ApiError> {
    Ok(Json(api.get_plugins().await))
}
