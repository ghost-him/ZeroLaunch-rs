//! ZeroLaunch plugin JSON-RPC protocol definitions.
//!
//! The transport layer uses LSP-style framed stdio JSON-RPC 2.0.
//! This crate defines all message bodies, method name constants,
//! manifest schema, and error codes.

pub mod codec;
pub mod error;
pub mod jsonrpc;
pub mod manifest;
pub mod messages;
pub mod methods;

pub use error::*;
pub use jsonrpc::*;
pub use manifest::*;
pub use messages::*;
pub use methods::*;

/// 协议版本：宿主与插件在 `plugin/initialize` 握手时各自声明，宿主只比较 major
/// （`protocol_version_compatible`）——major 不同即拒绝加载。
/// 载荷级 breaking change 必须提升 major；仅新增可选字段/方法不提升。
pub const PROTOCOL_VERSION: &str = "2.0";
