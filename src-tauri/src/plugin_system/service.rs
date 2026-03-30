use super::dispatcher::QueryDispatcher;
use super::registry::PluginRegistry;
use super::types::{Plugin, PluginAPI, PluginContext, PluginError, Query, QueryResponse};
use rand::distr::{Alphanumeric, SampleString};
use std::sync::Arc;

pub struct PluginService {
    registry: Arc<PluginRegistry>,
    dispatcher: Arc<QueryDispatcher>,
    api: Arc<dyn PluginAPI>,
}

impl PluginService {
    pub fn new(api: Arc<dyn PluginAPI>) -> Self {
        let registry = Arc::new(PluginRegistry::new());
        let dispatcher = Arc::new(QueryDispatcher::new(registry.clone()));

        Self {
            registry,
            dispatcher,
            api,
        }
    }

    /// 注册一个插件到服务中。
    /// 参数：plugin - 要注册的插件实例。
    /// 返回：无。
    pub fn register(&self, plugin: Arc<dyn Plugin>) {
        self.registry.register(plugin);
    }

    /// 初始化当前已注册的所有插件。
    /// 参数：无。
    /// 返回：成功返回 ()，失败返回 PluginError。
    pub async fn init_all(&self) -> Result<(), PluginError> {
        let mut rng = rand::rng();
        let trace_id = Alphanumeric.sample_string(&mut rng, 8);
        let ctx = PluginContext::new(&trace_id);

        for plugin in self.registry.get_all() {
            plugin.init(&ctx, self.api.clone()).await?;
        }

        Ok(())
    }

    /// 执行一次查询并返回结果。
    /// 参数：ctx - 当前插件上下文；query - 查询内容。
    /// 返回：查询结果列表。
    pub async fn query(&self, ctx: &PluginContext, query: &Query) -> Option<QueryResponse> {
        self.dispatcher.dispatch_plugin(ctx, query).await
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
        self.dispatcher
            .execute_action(ctx, plugin_id, action_id, payload)
            .await
    }

    /// 获取插件注册中心引用。
    /// 参数：无。
    /// 返回：注册中心的共享引用。
    pub fn registry(&self) -> &Arc<PluginRegistry> {
        &self.registry
    }

    /// 获取查询分发器引用。
    /// 参数：无。
    /// 返回：查询分发器的共享引用。
    pub fn dispatcher(&self) -> &Arc<QueryDispatcher> {
        &self.dispatcher
    }
}
