use super::error::ApiError;
use super::types::*;
use crate::plugin_system::types::Query;
use crate::utils::service_locator::ServiceLocator;
use async_trait::async_trait;
use uuid::Uuid;

/// 只读查询契约 — HTTP API 的数据面。
///
/// 每个方法映射到一个 GET 端点。所有方法通过 ServiceLocator 访问
/// 运行时状态，将内部类型转换为 `api::types` 中的 DTO。
#[async_trait]
pub trait ReadApi: Send + Sync {
    /// 健康检查：返回服务状态、版本号、候选数量、会话模式
    async fn health(&self) -> ApiHealth;

    /// 执行搜索查询
    async fn search(&self, query: &str) -> ApiSearchResponse;

    /// 获取所有缓存的候选项
    async fn get_candidates(&self) -> ApiCandidatesResponse;

    /// 获取指定 ID 的候选项。不存在时返回 ApiError::not_found
    async fn get_candidate(&self, id: u64) -> Result<ApiCandidate, ApiError>;

    /// 获取候选项数量
    async fn get_candidate_count(&self) -> ApiCountResponse;

    /// 获取当前会话模式
    async fn get_session_mode(&self) -> ApiSessionModeResponse;

    /// 获取所有可配置组件
    async fn get_components(&self) -> ApiComponentsResponse;

    /// 获取指定组件的配置 Schema。
    /// 组件不存在时返回 ApiError::not_found
    async fn get_component_schema(&self, id: &str) -> Result<ApiComponentSchemaResponse, ApiError>;

    /// 获取指定组件的当前配置值。
    /// 组件不存在时返回 ApiError::not_found
    async fn get_component_settings(
        &self,
        id: &str,
    ) -> Result<ApiComponentSettingsResponse, ApiError>;

    /// 获取所有已注册插件的元数据
    async fn get_plugins(&self) -> ApiPluginsResponse;
}

/// ReadApi 的默认实现。
///
/// 零大小结构体，所有状态通过 ServiceLocator 获取。
pub struct ReadApiImpl;

#[async_trait]
impl ReadApi for ReadApiImpl {
    async fn health(&self) -> ApiHealth {
        let state = ServiceLocator::get_state();
        let count = state.get_session_router().get_cached_candidates_count();
        let session_mode = state
            .get_session_router()
            .current_mode()
            .as_str()
            .to_string();

        ApiHealth {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            candidates_count: count,
            session_mode,
        }
    }

    async fn search(&self, query: &str) -> ApiSearchResponse {
        let state = ServiceLocator::get_state();
        let session_router = state.get_session_router();
        let trace_id = Uuid::new_v4().to_string()[..8].to_string();

        let query_obj = Query {
            id: trace_id,
            raw_query: query.to_string(),
            search_term: query.to_lowercase(),
        };

        let response = session_router.route_query(&query_obj.id, &query_obj).await;
        ApiSearchResponse::from_internal(response, query)
    }

    async fn get_candidates(&self) -> ApiCandidatesResponse {
        let state = ServiceLocator::get_state();
        let session_router = state.get_session_router();
        let (total_count, snapshot) = session_router.get_candidates_snapshot();
        let candidates: Vec<ApiCandidate> = snapshot.iter().map(ApiCandidate::from).collect();

        ApiCandidatesResponse {
            total_count,
            candidates,
        }
    }

    async fn get_candidate(&self, id: u64) -> Result<ApiCandidate, ApiError> {
        let state = ServiceLocator::get_state();
        let session_router = state.get_session_router();
        let candidate = session_router
            .get_cached_candidate_by_id(id)
            .ok_or_else(|| ApiError::not_found(format!("candidate not found: {}", id)))?;
        Ok(ApiCandidate::from(&candidate))
    }

    async fn get_candidate_count(&self) -> ApiCountResponse {
        let state = ServiceLocator::get_state();
        let count = state.get_session_router().get_cached_candidates_count();
        ApiCountResponse { count }
    }

    async fn get_session_mode(&self) -> ApiSessionModeResponse {
        let state = ServiceLocator::get_state();
        let mode = state
            .get_session_router()
            .current_mode()
            .as_str()
            .to_string();
        ApiSessionModeResponse { mode }
    }

    async fn get_components(&self) -> ApiComponentsResponse {
        let state = ServiceLocator::get_state();
        let config_manager = state.get_config_manager();
        let components = config_manager.get_all_components();
        ApiComponentsResponse {
            total_count: components.len(),
            components,
        }
    }

    async fn get_component_schema(&self, id: &str) -> Result<ApiComponentSchemaResponse, ApiError> {
        let state = ServiceLocator::get_state();
        let config_manager = state.get_config_manager();
        let schema = config_manager
            .get_component_schema(id)
            .ok_or_else(|| ApiError::not_found(format!("component not found: {}", id)))?;
        Ok(ApiComponentSchemaResponse { schema })
    }

    async fn get_component_settings(
        &self,
        id: &str,
    ) -> Result<ApiComponentSettingsResponse, ApiError> {
        let state = ServiceLocator::get_state();
        let config_manager = state.get_config_manager();
        let settings = config_manager
            .get_settings(id)
            .ok_or_else(|| ApiError::not_found(format!("component not found: {}", id)))?;
        Ok(ApiComponentSettingsResponse {
            component_id: id.to_string(),
            settings,
        })
    }

    async fn get_plugins(&self) -> ApiPluginsResponse {
        let state = ServiceLocator::get_state();
        let session_router = state.get_session_router();
        let registry = session_router.plugin_service().registry();
        let plugins = registry.get_all_metadata();
        ApiPluginsResponse {
            total_count: plugins.len(),
            plugins,
        }
    }
}
