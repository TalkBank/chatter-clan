//! High-level library types for executing CLAN analyses.
//!
//! `talkbank-clan` already exposes low-level command traits and runner plumbing
//! in [`crate::framework`], but editor integrations should not need to import
//! every individual command type just to execute a named analysis. This module
//! keeps the higher-level analysis execution boundary inside the library so CLI
//! and LSP wrappers can stay focused on adapting outer request shapes.

mod builder;
mod command_name;
mod options;
mod request;
#[cfg(test)]
mod tests;

pub use builder::*;
pub use command_name::*;
pub use options::*;
pub use request::*;
