use super::registry::PluginRegistry;
use crate::sdk::HostApi;
use std::sync::Arc;
use zerolaunch_plugin_api::host::PluginSdkConfig;
use zerolaunch_plugin_api::{Plugin, PluginContext, PluginError, Query, QueryResponse};

pub struct PluginService {
    registry: Arc<PluginRegistry>,
}

impl Default for PluginService {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginService {
    pub fn new() -> Self {
        let registry = Arc::new(PluginRegistry::new());
        Self { registry }
    }

    /// 注册一个插件到服务中。
    pub fn register(&self, plugin: Arc<dyn Plugin>) {
        self.registry.register(plugin);
    }

    /// 注销指定插件（按 plugin_id）。
    pub fn unregister(&self, plugin_id: &str) {
        self.registry.unregister(plugin_id);
    }

    /// 初始化当前已注册的所有插件。
    /// 参数：host_api - 宿主 API 句柄，用于插件访问平台能力。
    /// 返回：成功返回 ()，失败返回 PluginError。
    pub async fn init_all(&self, host_api: Arc<HostApi>) -> Result<(), PluginError> {
        let trace_id = crate::utils::trace_id::generate_trace_id();
        let ctx = PluginContext::new(&trace_id);

        for plugin in self.registry.get_all() {
            let plugin_id = plugin.metadata().id.clone();
            let handle = host_api.register(&plugin_id, PluginSdkConfig::default());
            plugin.init(&ctx, handle).await?;
        }

        Ok(())
    }

    /// 执行一次查询并返回结果。
    /// 如果当前击中了插件的触发器，则返回 (插件ID, 查询结果)，否则返回 None。
    /// 参数：ctx - 当前插件上下文；query - 查询内容。
    /// 返回：命中触发器时返回 (插件ID, 查询结果)，否则返回 None。
    pub async fn query(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Option<(String, QueryResponse)> {
        let (trigger, search_term) = self.registry.parse_trigger(&query.raw_query);

        if let Some(trigger) = trigger {
            if let Some(plugin) = self.registry.get_by_trigger(&trigger) {
                let plugin_id = plugin.metadata().id.clone();
                let query = Query {
                    id: query.id.clone(),
                    raw_query: query.raw_query.clone(),
                    search_term: search_term.to_string(),
                };
                let response = plugin.query(ctx, &query).await.unwrap();
                return Some((plugin_id, response));
            } else {
                tracing::error!("当前已成功解析 trigger 但是没找到对应的插件 '{}'", trigger);
            }
        }

        None
    }

    /// 执行指定插件的动作。
    /// 参数：ctx - 当前插件上下文；plugin_id - 插件 ID；action_id - 动作 ID；payload - 执行所需的数据。
    /// 返回：成功返回 ()，失败返回 PluginError。
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

    /// 获取插件注册中心引用。
    /// 参数：无。
    /// 返回：注册中心的共享引用。
    pub fn registry(&self) -> &Arc<PluginRegistry> {
        &self.registry
    }
}
