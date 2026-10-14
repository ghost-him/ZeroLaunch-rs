# zerolaunch-plugin-sdk-rust

Rust SDK for writing ZeroLaunch third-party plugins.

## Usage

```rust
use async_trait::async_trait;
use zerolaunch_plugin_sdk_rust::run;
use zerolaunch_plugin_api::*;

struct MyPlugin;

#[async_trait]
impl Configurable for MyPlugin { /* ... */ }

#[async_trait]
impl Plugin for MyPlugin {
    async fn init(&self, ctx: &PluginContext, handle: Option<Arc<PluginHandle>>) -> Result<(), PluginError> { Ok(()) }
    async fn query(&self, ctx: &PluginContext, query: &Query) -> Result<QueryResponse, PluginError> { /* ... */ }
    async fn execute_action(&self, ctx: &PluginContext, action_id: &str, payload: serde_json::Value) -> Result<(), PluginError> { /* ... */ }
}

fn main() {
    run(MyPlugin)
}
```

## What the SDK does

- Reads LSP-framed JSON-RPC messages from **stdin**
- Dispatches `plugin/*` methods to your `Plugin` trait implementation
- Writes JSON-RPC responses to **stdout**
- Handles `plugin/initialize`, `plugin/get_components`,
  `plugin/query`, `plugin/execute_action`, `plugin/get_settings`, `plugin/apply_settings`,
  `plugin/validate_settings`, `plugin/config_actions`, `plugin/execute_config_action`

## HostProxy

插件进程内通过 `host()` 取到宿主代理（`run()` 之后可用），所有 `host/*` 能力经它调用：

```rust
use zerolaunch_plugin_sdk_rust::host;

let host = host();
host.log("info", "Hello from plugin!").await?;
host.shell_open("notepad.exe").await?;
```

能力分组（方法名 → `host/*` 协议方法）：

| 分组 | 方法 |
|---|---|
| 日志 / 通知 | `log`、`notify` |
| Shell | `shell_open`、`shell_open_folder`、`shell_execute_elevation`、`shell_execute_command` |
| 窗口 | `activate_window_by_process`、`activate_window_by_pid`、`activate_window_by_title` |
| 图标 | `get_icon`（失败回退默认图标）、`extract_icon`（失败返回错误）、`refresh_icon`、`override_icon_cache` |
| 剪贴板 / 应用 | `set_clipboard_text`、`enumerate_apps`、`launch_app`、`app_icon_path` |
| 路径 / 主题 / 语言 | `resolve_path`、`resolve_lnk_target`、`localized_names_from_dir`、`get_theme`、`get_system_theme`、`get_locale` |
| 参数模板 | `resolve_parameters`、`count_user_parameters`、`has_system_parameters` |
| 能力集 / SDK 配置 | `capabilities`、`update_sdk_config` |
| 插件资源 | `resource_upload`、`resource_put`、`resource_get`、`resource_delete`、`resource_list` |
| 插件本地缓存 | `cache_put`、`cache_get`、`cache_delete`、`cache_cleanup` |
| 模型 | `model_list`、`model_chat`、`model_embedding`、`model_similarity` |
| 定时器 | `set_timeout`、`set_interval`、`cancel_timer`、`cancel_all_timers` |
| 事件订阅 | `subscribe_hotkey` / `unsubscribe_hotkey`、`subscribe_installation` / `unsubscribe_installation`、`subscribe_focus` / `unsubscribe_focus` |

### 定时器与事件订阅（宿主 → 插件推送）

定时器到期与订阅事件由宿主以 `plugin/*` 通知推送给插件进程，SDK 读任务按 ID 路由到注册的处理函数
（同一套机制，无需手搓帧）：

```rust
use std::time::Duration;
use zerolaunch_plugin_api::services::hotkey::types::HotkeyEventFilter;
use zerolaunch_plugin_sdk_rust::host;

let host = host();

// 一次性 / 重复定时器：到期回调收到定时器 ID。
let id = host.set_timeout(Duration::from_secs(5), |id| {
    tracing::info!("timer {id} fired");
}).await?;
host.cancel_timer(id).await?;
host.cancel_all_timers().await?;   // 只影响本插件的定时器

// 订阅事件：按插件内 ID 路由（同一 ID 用于退订）。
host.subscribe_installation("my-sub", |event| {
    tracing::info!("install event: {event:?}");
}).await?;
host.unsubscribe_installation("my-sub").await?;

// 按键事件（All / GlobalHotkey(hotkey) / DoubleCtrl）、窗口焦点事件同理。
host.subscribe_hotkey("keys", HotkeyEventFilter::All, |event| {
    tracing::info!("hotkey: {event:?}");
}).await?;
```

处理函数在 SDK 读任务上同步调用：必须快速返回，耗时操作请自行 `tokio::spawn`。
