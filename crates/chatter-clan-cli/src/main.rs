//! `chatter-clan`: the standalone CLI for the TalkBank CLAN analysis
//! reimplementation.
//!
//! In the chatter monorepo these commands were the `chatter clan <command>`
//! subcommand; extracted into the dormant `chatter-clan` repo they stand alone
//! as `chatter-clan <command>`. The analysis engine lives in the `talkbank-clan`
//! library; this binary is only the argument-parsing and dispatch driver,
//! recovered from talkbank-cli so the halted project stays resumable.

use clap::{CommandFactory, FromArgMatches, Parser};

mod cli;
mod commands;

/// Top-level `chatter-clan` CLI: a flat set of CLAN analysis, transform, and
/// converter subcommands. (The chatter monorepo nested these under
/// `chatter clan`; standing alone they are the top-level commands.)
#[derive(Parser)]
#[command(
    name = "chatter-clan",
    version,
    about = "Faithful Rust reimplementation of CLAN analysis commands for TalkBank CHAT",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: cli::ClanCommands,
}

fn main() {
    // Apply the same help grouping the `chatter clan` subcommand used (groups the
    // ~45 commands into Analysis / Transform / Converter sections), then parse.
    let command = cli::apply_clan_help_grouping(Cli::command());
    let matches = command.get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(err) => err.exit(),
    };
    commands::clan::run_clan(cli.command);
}
