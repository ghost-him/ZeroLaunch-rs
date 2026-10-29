//! ZeroLaunch third-party plugin host.
//!
//! Manages subprocess lifecycle, JSON-RPC transport (LSP-style framed stdio),
//! bidirectional RPC client, and `RemoteComponent` implementations.

pub mod adapter;
pub mod client;
pub mod host_dispatch;
pub mod icon;
pub mod info;
pub mod installed;
pub mod manager;
pub mod process;
pub mod transport;

pub use info::PluginInfo;
pub use installed::InstalledPlugin;
