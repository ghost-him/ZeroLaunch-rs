//! ZeroLaunch third-party plugin host.
//!
//! Manages subprocess lifecycle, JSON-RPC transport (LSP-style framed stdio),
//! bidirectional RPC client, and RemotePluginAdapter implementations.

pub mod adapter;
pub mod client;
pub mod host_dispatch;
pub mod manager;
pub mod process;
pub mod transport;
