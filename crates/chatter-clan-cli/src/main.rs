//! `chatter-clan`: the standalone CLI for the TalkBank CLAN analysis
//! reimplementation.
//!
//! These commands were previously the `chatter clan <command>` subcommand in
//! the chatter workspace. In this active standalone repository they run as
//! `chatter-clan <command>`. The analysis engine lives in the `talkbank-clan`
//! library; this binary provides argument parsing and dispatch.

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
    about = "Standalone CLI for CLAN-compatible analysis, transform, and converter commands",
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
    let matches = command.get_matches_from(clan_argv());
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(err) => err.exit(),
    };
    commands::clan::run_clan(cli.command);
}

/// Build the argv clap sees: multi-call dispatch (a `freq` symlink behaves as
/// `chatter-clan freq`, as with the original CLAN binaries), then rewrite legacy
/// CLAN `+flag`/`-flag` syntax into long options.
fn clan_argv() -> Vec<String> {
    let mut args: Vec<String> = std::env::args().collect();
    let invoked_as = std::path::Path::new(&args[0])
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_owned);
    if let Some(name) = invoked_as
        && name != "chatter-clan"
        && Cli::command().find_subcommand(&name).is_some()
    {
        args.insert(1, name);
    }
    talkbank_clan::clan_args::rewrite_clan_args(&args)
}
