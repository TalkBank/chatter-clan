#![warn(missing_docs)]
// Test code is exempt from this crate's `deny`-level panic lints,
// see `docs/panic-audit/talkbank-clan.md`.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]
//! Reimplementation of CLAN analysis commands in Rust.
//!
//! CLAN (Computerized Language Analysis) is MacWhinney's CHAT analysis toolkit.
//! This crate reimplements the self-contained CLAN analysis, transform, and
//! conversion commands in Rust, leveraging the existing workspace
//! `talkbank-parser` / `talkbank-model` / `talkbank-transform` infrastructure.
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) for comprehensive
//! documentation of the original CLAN commands and their semantics.
//!
//! # Public contract
//!
//! This crate is the engine behind `chatter clan ...`. Most end users should use the
//! CLI surface rather than embedding `talkbank-clan` directly.
//!
//! The library API is intended for:
//!
//! - tests and tooling that need typed command output,
//! - integrators that want to run CLAN-style analyses inside a Rust process,
//! - contributors working on command implementations and parity work.
//!
//! Command parity with upstream CLAN is still in progress. For the current
//! audited status, see the CLAN parity status page:
//! <https://github.com/TalkBank/chatter/blob/main/book/src/clan-reference/parity-status.md>
//!
//! # Architecture
//!
//! The crate is split into four layers:
//!
//! - **[`framework`]**: Shared infrastructure replacing CLAN's CUTT framework:
//!   [`framework::AnalysisCommand`] trait, [`framework::FilterConfig`] for speaker/tier/word/gem filtering,
//!   [`framework::UtteranceRange`] and [`framework::DiscoveredChatFiles`] for reusable analysis input models,
//!   [`framework::AnalysisRunner`] for file loading and command dispatch, [`framework::AnalysisResult`]
//!   / [`framework::CommandOutput`] for text and structured output, and [`framework::TransformCommand`]
//!   for file-modifying commands.
//!
//! - **[`commands`]**: Individual analysis command implementations (FREQ, MLU, MLT, etc.),
//!   each implementing the [`framework::AnalysisCommand`] trait.
//!
//! - **[`transforms`]**: File-modifying commands (FLO, LOWCASE, CHSTRING, etc.),
//!   each implementing the [`framework::TransformCommand`] trait.
//!
//! - **[`converters`]**: Format conversion between CHAT and external formats
//!   (SRT, ELAN, Praat TextGrid, SALT, etc.).
//!
//! The most important entry points are:
//!
//! - [`framework::AnalysisRunner`] for read-only analyses,
//! - [`framework::AnalysisCommand`] for implementing analysis commands,
//! - [`framework::TransformCommand`] for file-modifying commands,
//! - [`commands`] / [`transforms`] / [`converters`] for the concrete command families.
//!
//! # Usage
//!
//! ```no_run
//! use std::path::Path;
//! use talkbank_clan::framework::{AnalysisRunner, CommandOutput, OutputFormat};
//! use talkbank_clan::commands::freq::FreqCommand;
//!
//! let runner = AnalysisRunner::new();
//! let command = FreqCommand::default();
//! let result = runner.run(&command, &[Path::new("file.cha").to_path_buf()]);
//! match result {
//!     Ok(output) => print!("{}", output.render(OutputFormat::Text)),
//!     Err(e) => eprintln!("Error: {e}"),
//! }
//! ```
//!
//! For command-line usage, see the `talkbank-cli` crate and the book's CLAN reference.

pub mod clan_args;
pub mod commands;
pub mod converters;
pub mod database;
pub mod framework;
pub mod service;
pub mod service_types;
pub mod transforms;
