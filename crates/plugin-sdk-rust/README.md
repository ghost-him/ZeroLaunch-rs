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
    fn metadata(&self) -> &PluginMetadata { /* ... */ }
    async fn init(&self, ctx: &PluginContext, handle: Arc<PluginHandle>) -> Result<(), PluginError> { Ok(()) }
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
- Handles `plugin/initialize`, `plugin/get_metadata`, `plugin/get_components`,
  `plugin/query`, `plugin/execute_action`, `plugin/get_settings`, `plugin/apply_settings`,
  `plugin/validate_settings`, `plugin/config_actions`, `plugin/execute_config_action`

## HostProxy

For calling host-side APIs from your plugin:

```rust
use zerolaunch_plugin_sdk_rust::HostProxy;
let host = HostProxy::new();
host.log("info", "Hello from plugin!")?;
host.shell_open("notepad.exe")?;
```
