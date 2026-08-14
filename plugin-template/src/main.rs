use std::sync::Arc;

use async_trait::async_trait;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    Plugin, PluginContext, PluginError, PluginHandle, PluginMetadata, Query, QueryResponse,
    ListItem, ResultAction,
};
use zerolaunch_plugin_sdk_rust::{run, t_key};

/// Hello World 示例插件 — 演示第三方插件的最小骨架与 i18n 用法。
///
/// 仅作为插件模板使用；文本经 `t_key()` 生成命名空间翻译键
/// （`plugin.com.example.hello-world.<key>`），宿主加载插件目录
/// `i18n/<lang>.json` 语言包后，前端对 key-or-literal 文本自动翻译。
struct HelloWorldPlugin {
    /// 组件 ID、名称、类型等基础元数据（`Configurable` trait 默认实现委托于此）。
    core: ComponentCore,
    /// 插件静态元数据：id、触发关键词、优先级等。
    metadata: PluginMetadata,
}

impl HelloWorldPlugin {
    fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "com.example.hello-world".to_string(),
                "Hello World".to_string(),
                "A simple hello-world plugin".to_string(),
                ComponentType::Plugin,
                100,
            ),
            metadata: PluginMetadata {
                id: "com.example.hello-world".to_string(),
                name: "Hello World".to_string(),
                version: "0.1.0".to_string(),
                description: "A simple hello-world plugin".to_string(),
                author: "You".to_string(),
                trigger_keywords: vec!["hello".to_string(), "hw".to_string()],
                supported_os: vec!["windows".to_string()],
                priority: 100,
                // None = 行内插件，仅关键词唤醒；
                // Some("Ctrl+E") = 完全插件模式，宿主注册全局热键唤醒（如 everything 回归）
                hotkey: None,
            },
        }
    }
}

#[async_trait]
impl Configurable for HelloWorldPlugin {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    /// 本示例无设置项；有设置项时在此声明 schema（label 可用 `t_key!`/`t_key` 形式）。
    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![]
    }
}

#[async_trait]
impl Plugin for HelloWorldPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(
        &self,
        _ctx: &PluginContext,
        _handle: Option<Arc<PluginHandle>>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    async fn query(&self, ctx: &PluginContext, query: &Query) -> Result<QueryResponse, PluginError> {
        // ctx.locale 携带宿主当前界面语言（如 "zh-Hans"），可按语言生成本地化文本；
        // 主动查询可用 host().get_locale().await（经 HostProxy）。
        tracing::debug!(locale = %ctx.locale, "hello query");
        Ok(QueryResponse::List {
            results: vec![ListItem {
                id: 1,
                title: format!("Hello: {}", query.raw_query),
                // key-or-literal：前端命中翻译目录则显示译文，否则回退 key 原文。
                // t_key() 自动带当前插件 id 前缀（plugin.com.example.hello-world.<key>）
                subtitle: t_key("greeting"),
                icon: zerolaunch_plugin_api::services::icon_request::IconRequest::Path(String::new()),
                score: 1.0,
                actions: vec![ResultAction {
                    id: "hello".to_string(),
                    label: t_key("sayHello"),
                    icon: zerolaunch_plugin_api::services::icon_request::IconRequest::Path(String::new()),
                    is_default: true,
                    shortcut_key: String::new(),
                }],
                target_type: "BuiltinCommand".to_string(),
                user_arg_count: 0,
                has_system_params: false,
                trigger_keywords: vec![],
            }],
        })
    }

    async fn execute_action(&self, _ctx: &PluginContext, _action_id: &str, _payload: serde_json::Value) -> Result<(), PluginError> {
        tracing::info!("Hello World action executed!");
        Ok(())
    }
}

fn main() {
    run(HelloWorldPlugin::new())
}
