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

pub mod host_proxy;
pub mod runtime;

pub use host_proxy::HostProxy;
pub use runtime::{host, run};
