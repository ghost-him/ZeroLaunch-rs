use crate::config::configurable::Configurable;
use crate::host::plugin_handle::PluginHandle;
use crate::plugin::types::{
    PanelInteraction, PluginContext, PluginError, PluginMetadata, Query, QueryResponse,
};
use async_trait::async_trait;
use std::sync::Arc;

/// 所有插件对象都需实现的核心契约。
/// 服务于插件生命周期管理、查询处理与动作执行。
/// 配置管理能力由 Configurable trait 提供。
#[async_trait]
pub trait Plugin: Configurable {
    fn metadata(&self) -> &PluginMetadata;

    async fn init(&self, ctx: &PluginContext, handle: Arc<PluginHandle>)
        -> Result<(), PluginError>;

    async fn query(&self, ctx: &PluginContext, query: &Query)
        -> Result<QueryResponse, PluginError>;

    async fn execute_action(
        &self,
        ctx: &PluginContext,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), PluginError>;

    /// 返回插件当前生效的交互策略（防抖延迟、提交行为等）。
    /// 这是一次快速、同步的调用，不涉及 IO 或网络。
    /// 默认返回无防抖、Execute 提交的交互策略。
    fn interaction_policy(&self) -> PanelInteraction {
        PanelInteraction::default()
    }
}
