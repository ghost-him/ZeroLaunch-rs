//! ZeroLaunch Rust plugin SDK.
//!
//! Provides a `run()` function that wraps a user's `Plugin` trait implementation
//! in a JSON-RPC 2.0 stdio loop, handling the LSP-style frame protocol and
//! dispatching incoming requests.
//!
//! # Usage
//!
//! ```ignore
//! fn main() {
//!     zerolaunch_plugin_sdk_rust::run(MyPlugin::new())
//! }
//! ```

pub mod events;
pub mod host_proxy;
pub mod logging;
pub mod runtime;

pub mod trace;

use std::sync::OnceLock;

pub use host_proxy::HostProxy;
pub use runtime::{host, run, PluginApp};
pub use trace::{instrument, span_for, with_trace};

/// 当前插件 id：`plugin/initialize` 握手时由宿主下发并写入，
/// 供 `t_key()` 自动补命名空间前缀。
static PLUGIN_ID: OnceLock<String> = OnceLock::new();

/// 记录宿主下发的插件 id（SDK 运行时握手时调用，仅一次）。
pub(crate) fn set_plugin_id(plugin_id: &str) {
    let _ = PLUGIN_ID.set(plugin_id.to_string());
}

/// 生成带当前插件命名空间前缀的翻译键（`plugin.<当前插件id>.<key>`）。
///
/// 插件 id 由 SDK 在 `plugin/initialize` 握手时自动注入，无需手动传入；
/// 配合插件目录 `i18n/<lang>.json` 语言包使用：宿主加载插件时读取该目录、
/// 统一加前缀合并进翻译目录，前端对 key-or-literal 文本（schema 标签、
/// 结果项动作 label 等）自动翻译；未提供语言包时原样显示 key 派生文本。
///
/// # Panics
///
/// 在插件运行时初始化（`run()` 之后的握手）之前调用将 panic，
/// 因为此时尚不知道插件 id。
/// 从环境变量 `ZEROLAUNCH_PLUGIN_ID` 预置插件 id，使 `t_key()` 可在组件构造
/// （`main()` 中、`run()` 之前）使用。宿主 spawn 插件时注入该变量；
/// 非宿主环境（本地调试等）可不调用，此时 `t_key()` 在握手前调用仍 panic。
pub fn init() {
    if let Ok(id) = std::env::var("ZEROLAUNCH_PLUGIN_ID") {
        set_plugin_id(&id);
    }
}

pub fn t_key(key: &str) -> String {
    let plugin_id = PLUGIN_ID
        .get()
        .expect("t_key 必须在插件运行时初始化后调用（run() 之后）");
    format!("plugin.{plugin_id}.{key}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// t_key 自动补当前插件 id 前缀（id 在 plugin/initialize 握手时注入）。
    #[test]
    fn t_key_prefixes_current_plugin_id() {
        set_plugin_id("com.example.hello-world");
        assert_eq!(t_key("greeting"), "plugin.com.example.hello-world.greeting");
        assert_eq!(t_key("sayHello"), "plugin.com.example.hello-world.sayHello");
    }
}
