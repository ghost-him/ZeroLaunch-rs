use async_trait::async_trait;
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable};
use zerolaunch_plugin_api::{
    Plugin, PluginContext, PluginError, PluginHandle, PluginMetadata, Query, QueryResponse,
    ListItem, ResultAction,
};
use zerolaunch_plugin_sdk_rust::run;

struct HelloWorldPlugin {
    metadata: PluginMetadata,
}

impl HelloWorldPlugin {
    fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "com.example.hello-world".to_string(),
                name: "Hello World".to_string(),
                version: "0.1.0".to_string(),
                description: "A simple hello-world plugin".to_string(),
                author: "You".to_string(),
                trigger_keywords: vec!["hello".to_string(), "hw".to_string()],
                supported_os: vec!["windows".to_string()],
                priority: 100,
            },
        }
    }
}

impl Configurable for HelloWorldPlugin {
    fn component_id(&self) -> &str { "com.example.hello-world" }
    fn component_name(&self) -> &str { "Hello World" }
    fn component_type(&self) -> ComponentType { ComponentType::Plugin }
}

#[async_trait]
impl Plugin for HelloWorldPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&self, _ctx: &PluginContext, _handle: Arc<PluginHandle>) -> Result<(), PluginError> {
        Ok(())
    }

    async fn query(&self, _ctx: &PluginContext, query: &Query) -> Result<QueryResponse, PluginError> {
        Ok(QueryResponse::List {
            results: vec![ListItem {
                id: 1,
                title: format!("Hello: {}", query.raw_query),
                subtitle: "来自第三方插件的问候".to_string(),
                icon: zerolaunch_plugin_api::services::icon_request::IconRequest::Path(String::new()),
                score: 1.0,
                actions: vec![ResultAction {
                    id: "hello".to_string(),
                    label: "打招呼".to_string(),
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
        eprintln!("Hello World action executed!");
        Ok(())
    }
}

fn main() {
    run(HelloWorldPlugin::new())
}
