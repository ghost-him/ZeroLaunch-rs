use super::registry::PluginRegistry;
use super::types::{PluginContext, PluginError, Query, QueryResponse};
use crate::error;
use std::sync::Arc;

pub struct QueryDispatcher {
    registry: Arc<PluginRegistry>,
}

impl QueryDispatcher {
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self { registry }
    }

    /// 如果当前击中了插件的触发器，则会返回结果，否则，返回None
    pub async fn dispatch_plugin(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Option<QueryResponse> {
        let (trigger, search_term) = self.registry.parse_trigger(&query.raw_query);

        if let Some(trigger) = trigger {
            if let Some(plugin) = self.registry.get_by_trigger(&trigger) {
                let query = Query {
                    id: query.id.clone(),
                    raw_query: query.raw_query.clone(),
                    search_term: search_term.to_string(),
                };
                return Some(plugin.query(ctx, &query).await.unwrap());
            } else {
                error!("当前已成功解析 trigger 但是没找到对应的插件 '{}'", trigger);
            }
        }

        None
    }

    pub async fn execute_action(
        &self,
        ctx: &PluginContext,
        plugin_id: &str,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        let plugin = self
            .registry
            .get(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        plugin.execute_action(ctx, action_id, payload).await
    }
}
