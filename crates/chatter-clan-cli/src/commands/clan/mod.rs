//! Dispatch CLAN analysis, transform, converter, and compatibility commands.

mod analysis;
mod compatibility;
mod converters;
mod helpers;
mod transforms;

use crate::cli::ClanCommands;

/// Dispatch a `chatter clan` subcommand.
pub fn run_clan(command: ClanCommands) {
    let command = match analysis::dispatch(command) {
        Ok(()) => return,
        Err(command) => command,
    };
    let command = match transforms::dispatch(command) {
        Ok(()) => return,
        Err(command) => command,
    };
    let command = match converters::dispatch(command) {
        Ok(()) => return,
        Err(command) => command,
    };
    if let Err(command) = compatibility::dispatch(command) {
        match command {
            ClanCommands::Mor {} => {
                if let Err(e) = talkbank_clan::commands::mor::run_mor() {
                    eprintln!("Error: {e}");
                }
            }
            ClanCommands::Post {} => {
                if let Err(e) = talkbank_clan::commands::post::run_post() {
                    eprintln!("Error: {e}");
                }
            }
            ClanCommands::Megrasp {} => {
                if let Err(e) = talkbank_clan::commands::megrasp::run_megrasp() {
                    eprintln!("Error: {e}");
                }
            }
            ClanCommands::Postlist {} => {
                if let Err(e) = talkbank_clan::commands::postlist::run_postlist() {
                    eprintln!("Error: {e}");
                }
            }
            ClanCommands::Postmodrules {} => {
                if let Err(e) = talkbank_clan::commands::postmodrules::run_postmodrules() {
                    eprintln!("Error: {e}");
                }
            }
            ClanCommands::Posttrain {} => {
                if let Err(e) = talkbank_clan::commands::posttrain::run_posttrain() {
                    eprintln!("Error: {e}");
                }
            }
            // Routing invariant: this branch only runs for clan
            // commands routed by the dispatcher; non-clan variants
            // are filtered upstream.
            #[allow(clippy::unreachable)]
            _ => unreachable!("unhandled clan command family"),
        }
        std::process::exit(1);
    }
}
