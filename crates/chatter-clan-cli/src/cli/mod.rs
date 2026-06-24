//! CLI argument definitions for the standalone `chatter-clan` binary.
//!
//! Recovered from the chatter monorepo's `talkbank-cli` (the `chatter clan`
//! subcommand, removed there in the clan extraction): `clan_common` holds the
//! shared CLAN argument groups and output formats, `clan_commands` the flat
//! enum of CLAN subcommands. The handlers in `crate::commands::clan` consume
//! these through the re-exports below, exactly as they did in talkbank-cli.

pub mod clan_commands;
pub mod clan_common;

pub use clan_commands::*;
pub use clan_common::*;
