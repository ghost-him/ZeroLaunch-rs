//! Mock implementations of plugin-api traits for unit testing.
//!
//! Enable with `features = ["mock"]` in dev-dependencies.
//! All stubs return `Ok(Default::default())` or empty collections by default.

#[cfg(feature = "mock")]
mod stubs;

#[cfg(feature = "mock")]
pub use stubs::*;

#[cfg(feature = "mock")]
pub mod helpers;
