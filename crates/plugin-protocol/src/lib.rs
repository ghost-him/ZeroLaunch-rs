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

pub const PROTOCOL_VERSION: &str = "1.0";
