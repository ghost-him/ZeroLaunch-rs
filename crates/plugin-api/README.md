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
    Plugin, PluginContext, PluginError, PluginMetadata, PluginHandle, PluginMode,
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
            hotkey: None,
            // panel 形态插件图标由宿主从 manifest [icon] 段读取，此处无需填写
            icon: None,
            // 插件形态：行内插件填 Inline，完全插件模式（trigger 类型）填 Panel
            mode: PluginMode::Inline,
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
            id: "q1".into(),
            raw_query: "echo hello".into(),
            search_term: "hello".into(),
            confirm: false,
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

## 国际化（i18n）

宿主与前端共享一套翻译系统，插件可提供**自己的语言包**：

### 语言包目录

插件目录下提供 `i18n/<lang>.json`（`lang` ∈ `zh-Hans` / `zh-Hant` / `en`），文件内是**不带前缀**的嵌套 JSON，值必须为字符串：

```json
// <plugin-dir>/i18n/zh-Hans.json
{
  "greeting": "来自第三方插件的问候",
  "settings": { "enabled": "启用" }
}
```

宿主在插件加载时读取并校验（单文件 ≤ 64 KiB，仅允许对象与字符串），统一以
`plugin.<pluginId>.<key>` 命名空间合并进翻译目录；前端经 `i18n_get_plugin_translations`
拉取后自动翻译 key-or-literal 文本（设置项 schema 标签、结果项动作 label 等）。

### 生成翻译键

Rust SDK 提供 `t_key(key)` 帮助函数——插件 id 在 `plugin/initialize` 握手时
自动注入，**无需手动传入**：

```rust
use zerolaunch_plugin_sdk_rust::t_key;

ResultAction {
    id: "hello".into(),
    label: t_key("sayHello"),
    // → "plugin.<当前插件id>.sayHello"（如 plugin.com.example.hello-world.sayHello），
    //   前端命中语言包则显示译文
    ..
}
```

未提供语言包（或缺少某语言）时，前端回退显示 key 原文——插件始终可用，翻译是增量能力。

### 插件进程获取当前语言

- **查询/动作场景**：`PluginContext` 携带 `locale` 字段（宿主注入，如 `"zh-Hans"`），
  可直接按语言生成本地化面板/结果文本。
- **任意时刻主动查询**：`HostProxy::get_locale().await`（`host/i18n.get_locale` RPC）。

```rust
async fn query(&self, ctx: &PluginContext, query: &Query) -> Result<QueryResponse, PluginError> {
    let greeting = if ctx.locale.starts_with("zh") { "你好" } else { "Hello" };
    // 或 let lang = host().get_locale().await?;
    ..
}
```

### 设置项 schema 标签

`SettingDefinition` 的 `label` / `description` / `group` 支持 key-or-literal：
写成 `t_key(key)` 形式即可随语言切换；写死字面量则原样显示（兼容旧插件）。


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
