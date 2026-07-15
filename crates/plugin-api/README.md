# zerolaunch-plugin-api

ZeroLaunch 插件 SDK — 第三方插件开发的唯一依赖。

只需依赖此 crate，即可编写一个完整的 ZeroLaunch 插件，全程不需要 Tauri、Windows API 或启动器源码。

## 快速开始

### Cargo.toml

```toml
[dependencies]
zerolaunch-plugin-api = { path = "../ZeroLaunch-rs/crates/plugin-api" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
zerolaunch-plugin-api = { path = "../ZeroLaunch-rs/crates/plugin-api", features = ["mock"] }
tokio = { version = "1", features = ["macros", "rt"] }
```

### 插件骨架

```rust
use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::{
    Configurable, ComponentType, ConfigError,
    Plugin, PluginContext, PluginError, PluginMetadata, PluginHandle,
    Query, QueryResponse, ListItem, IconRequest,
};

pub struct EchoPlugin { metadata: PluginMetadata }

impl EchoPlugin {
    pub fn new() -> Self {
        Self { metadata: PluginMetadata {
            id: "echo".into(), name: "Echo".into(), version: "0.1.0".into(),
            description: "回显输入".into(), author: "me".into(),
            trigger_keywords: vec!["echo".into()],
            supported_os: vec!["windows".into()], priority: 50,
        }}
    }
}

#[async_trait]
impl Configurable for EchoPlugin {
    fn component_id(&self) -> &str { "echo" }
    fn component_name(&self) -> &str { "Echo" }
    fn component_type(&self) -> ComponentType { ComponentType::Plugin }
}

#[async_trait]
impl Plugin for EchoPlugin {
    fn metadata(&self) -> &PluginMetadata { &self.metadata }

    async fn init(&self, _ctx: &PluginContext, _handle: Arc<PluginHandle>)
        -> Result<(), PluginError> { Ok(()) }

    async fn query(&self, _ctx: &PluginContext, query: &Query)
        -> Result<QueryResponse, PluginError>
    {
        Ok(QueryResponse::List { results: vec![ListItem {
            id: 1, title: query.search_term.clone(), subtitle: "echo".into(),
            icon: IconRequest::Path(String::new()), score: 100.0,
            actions: vec![], target_type: "Command".into(),
            user_arg_count: 0, has_system_params: false, trigger_keywords: vec![],
        }]})
    }

    async fn execute_action(&self, _ctx: &PluginContext, _action_id: &str,
        _payload: serde_json::Value) -> Result<(), PluginError> { Ok(()) }
}
```

### 单元测试（使用 mock feature）

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zerolaunch_plugin_api::mock::helpers::mock_plugin_handle;

    #[tokio::test]
    async fn echo_returns_input() {
        let plugin = EchoPlugin::new();
        let handle = mock_plugin_handle();
        let ctx = PluginContext::new("test");

        plugin.init(&ctx, handle).await.unwrap();

        let q = Query {
            id: "q1".into(), raw_query: "echo hello".into(),
            search_term: "hello".into(),
        };
        let resp = plugin.query(&ctx, &q).await.unwrap();
        match resp {
            QueryResponse::List { results } => assert_eq!(results[0].title, "hello"),
            _ => panic!("expected List"),
        }
    }
}
```

## 关键类型

| 类型 | 说明 |
|------|------|
| `Plugin` trait | 插件核心契约：`metadata()` + `init()` + `query()` + `execute_action()` |
| `PluginHandle` | 平台能力句柄，通过 `init()` 注入，提供 `get_icon()`、`shell_open()` 等服务 |
| `Configurable` trait | 配置管理契约，提供 `setting_schema()` + `apply_settings()` |
| `PluginMetadata` | 静态元数据：id、触发关键词、优先级等 |
| `Query` / `QueryResponse` | 查询输入/输出类型 |
| `PluginError` | 插件层统一错误类型 |

> **注意：** `HostApi` 与 `HostApiBuilder` 是宿主（zl 主程序）内部类型，负责管理插件注册、存储重配置等全局操作，**插件作者不需要也不会接触到它们**。插件只需通过 `Plugin::init()` 获取 `Arc<PluginHandle>`，所有平台能力调用都通过句柄完成。

## 集成到主程序

1. 在 `src-tauri/Cargo.toml` 添加依赖
2. 在 `lib.rs::init_plugin_system()` 中注册：
   ```rust
   session_router.plugin_service().register(Arc::new(EchoPlugin::new()));
   ```
3. `cargo run` 启动，输入 `echo hello` 测试

## License

MIT
