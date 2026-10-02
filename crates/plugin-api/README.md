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

插件级元数据（id、名称、版本、描述、作者、触发关键词、支持系统、优先级、形态、热键、图标）由宿主从 `manifest.toml` 的 `[plugin]` / `[icon]` 段读取，**插件代码不声明**，`Plugin` trait 上也不提供元数据方法。插件代码只提供组件级身份：一个 `ComponentCore`（组件 id、名称、描述、类型、优先级）。

```rust
use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    Plugin, PluginContext, PluginError, PluginHandle,
    Query, QueryResponse, ListItem,
};

pub struct EchoPlugin { core: ComponentCore }

impl EchoPlugin {
    pub fn new() -> Self {
        Self { core: ComponentCore::new(
            "echo".into(), "Echo".into(), "回显输入".into(),
            ComponentType::Plugin, 50,
        )}
    }
}

#[async_trait]
impl Configurable for EchoPlugin {
    fn core(&self) -> &ComponentCore { &self.core }
    fn setting_schema(&self) -> Vec<SettingDefinition> { vec![] }
}

#[async_trait]
impl Plugin for EchoPlugin {
    async fn init(&self, _ctx: &PluginContext, _handle: Option<Arc<PluginHandle>>)
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

        plugin.init(&ctx, Some(handle)).await.unwrap();

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
| `Plugin` trait | 插件核心契约：`init()` + `query()` + `match_query()` + `execute_action()`；插件级元数据不在本 trait 上，由宿主侧持有 |
| `PluginHandle` | 平台能力句柄，通过 `init()` 注入（`Option<Arc<PluginHandle>>`），提供 `get_icon()`、`shell_open()` 等服务 |
| `Configurable` trait | 组件契约：`core()`（组件身份）+ `setting_schema()`，配置读写与校验有默认实现 |
| `ComponentCore` | 组件级身份信息：组件 id、名称、描述、类型、优先级 |
| `PluginMetadata` | 插件级元数据：宿主从 `manifest.toml` 读取后构造，插件代码不声明 |
| `Query` / `QueryResponse` | 查询输入/输出类型 |
| `PluginError` | 插件层统一错误类型 |

## 查询匹配（match_query）

行内插件（`mode = "inline"`）的查询接管判定**只有一条路径**：宿主在路由阶段调用插件的
`Plugin::match_query(&self, raw_query: &str, declared_trigger_keywords: &[String]) -> bool`。

- **默认实现即框架的关键词判定**：触发表里任一项等于输入首词、且其后还有内容时返回 `true`
  （判定函数与宿主共用同一份实现）。因此**不覆盖该方法的插件行为与旧版关键词路由完全一致**，
  老插件一行代码都不用改。
- 需要自定义判定的插件**覆盖该方法**即可（如路径/网址检测器）：自己决定何时返回 `true`，
  此时框架的关键词规则不再参与。
- 宿主只做三件事：并发询问 → 按优先级裁决 → 推导查询词。查询词的规则是：赢家若同时满足框架
  关键词规则，取「触发词之后的剩余」（模型 `keywords`，前端可本地镜像）；否则取原始输入
  （模型 `custom`，前端粘性）。

### 实现契约

`match_query` 每次按键都会执行，因此：

- 必须快速、**不涉及 IO 或网络**；存在性/可达性等需要 IO 的判定放到 `query()`
  （async 且可自行超时）。
- 远端插件经 `plugin/match_query` RPC 调用（请求携带 `rawQuery` 与该插件声明的触发词）；
  旧 SDK 未实现该方法时宿主按 `METHOD_NOT_FOUND` 用同一份关键词判定兜底——已发布插件不受影响；
  其他错误/超时按不命中处理并告警。

### 路由裁决

只有处于启用状态的行内插件参与路由（`mode = "panel"` 的插件仅经热键/候选项唤醒），
且**输入含空格时才会发起判定**（关键词规则与检测器的提交规则都要求空格）。
宿主**并发**询问全部候选插件（内置进程内、远端 RPC），整体受截止时间兜底；命中者按
**`priority` 数值小者优先、同优先级按 `plugin_id` 字典序**选出唯一赢家——因此同名触发词
可以并存，不再被拒绝注册。

### 协议与兼容

- `plugin/match_query` 是新增的可选方法：未实现的插件宿主容 `METHOD_NOT_FOUND` 并回退到
  本地关键词判定，协议 major 不变（仅新增可选方法不提升 major）。

内置的 `path-detect` / `url-detect`（`src-tauri/src/builtin_plugin/detector/`，共享面板类型
`smart-target`）即自定义匹配的参考实现。

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
