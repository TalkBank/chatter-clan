use clap::Command;
use talkbank_clan::service_types::AnalysisCommandName;

/// Command category for grouping in help output.
///
/// Each CLAN subcommand belongs to exactly one category. The mapping is
/// maintained alongside the CLAN command enum so help output stays in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanCommandCategory {
    /// Compute statistics, counts, and metrics from CHAT data.
    Analysis,
    /// Modify CHAT files: add/remove tiers, fix formatting, etc.
    Transform,
    /// Convert between CHAT and other formats (SRT, EAF, Praat, etc.).
    Converter,
    /// Alternate names for existing commands (CLAN compatibility).
    CompatibilityAlias,
    /// Commands that require CLAN directly (morphological analysis, etc.).
    NotAvailable,
}

impl ClanCommandCategory {
    /// Heading text displayed in grouped help output.
    pub const fn heading(self) -> &'static str {
        match self {
            Self::Analysis => "Analysis Commands",
            Self::Transform => "Transform Commands",
            Self::Converter => "Format Converters",
            Self::CompatibilityAlias => "Compatibility Aliases",
            Self::NotAvailable => "Not Available (use CLAN directly)",
        }
    }
}

/// Map from subcommand name to its category.
///
/// Returns `None` for the built-in `help` subcommand that clap adds.
fn command_category(name: &str) -> Option<ClanCommandCategory> {
    use ClanCommandCategory::*;
    if name.parse::<AnalysisCommandName>().is_ok() {
        return Some(Analysis);
    }
    Some(match name {
        // -- Transform commands --
        "flo" | "lowcase" | "chstring" | "dates" | "delim" | "fixbullets" | "retrace"
        | "repeat" | "combtier" | "compound" | "tierorder" | "lines" | "dataclean" | "quotes"
        | "ort" | "postmortem" | "makemod" | "trim" | "roles" => Transform,
        // -- Converter commands --
        "chat2text" | "srt2chat" | "chat2srt" | "chat2vtt" | "text2chat" | "lipp2chat"
        | "elan2chat" | "praat2chat" | "chat2praat" | "lena2chat" | "play2chat" | "lab2chat"
        | "rtf2chat" | "salt2chat" | "gem" | "chat2elan" => Converter,
        // -- Compatibility aliases --
        "check" | "fixit" | "indent" | "longtier" | "gemfreq" => CompatibilityAlias,
        // -- Not available --
        "mor" | "post" | "megrasp" | "postlist" | "postmodrules" | "posttrain" => NotAvailable,
        // clap's built-in help subcommand
        "help" => return None,
        _ => return None,
    })
}

/// All categories in display order.
const CATEGORY_ORDER: &[ClanCommandCategory] = &[
    ClanCommandCategory::Analysis,
    ClanCommandCategory::Transform,
    ClanCommandCategory::Converter,
    ClanCommandCategory::CompatibilityAlias,
    ClanCommandCategory::NotAvailable,
];

/// Apply category grouping to the top-level `chatter-clan` help output.
///
/// Clap 4 does not support grouping subcommands under different headings via
/// derive attributes (`help_heading` on subcommand variants controls argument
/// headings, not subcommand listing headings). This function works around that
/// limitation by replacing the root command's `override_help` with a
/// custom-rendered grouped listing.
///
/// In the chatter monorepo the CLAN commands were nested under a `clan`
/// subcommand, so this grouped that child; standing alone in `chatter-clan`
/// they are the root's own top-level subcommands, so it groups the root.
///
/// Call this on the root `Command` returned by `Cli::command()` before parsing.
pub fn apply_clan_help_grouping(root: Command) -> Command {
    // Build the grouped help text from the actual subcommands registered by clap
    // derive, so names and descriptions stay in sync automatically.
    let grouped_help = build_grouped_help(&root);
    root.override_help(grouped_help)
}

/// Build a help string with subcommands organized under category headings.
fn build_grouped_help(cmd: &Command) -> String {
    use std::fmt::Write;

    let mut out = String::new();

    if let Some(long_about) = cmd.get_long_about() {
        let _ = writeln!(out, "{long_about}");
    } else if let Some(about) = cmd.get_about() {
        let _ = writeln!(out, "{about}");
    }

    let _ = writeln!(out, "\nUsage: chatter-clan [OPTIONS] <COMMAND>");

    let subcmds: std::collections::BTreeMap<&str, &Command> = cmd
        .get_subcommands()
        .map(|sc| (sc.get_name(), sc))
        .collect();

    let longest = subcmds.keys().map(|name| name.len()).max().unwrap_or(0);

    for &category in CATEGORY_ORDER {
        let heading = category.heading();
        let commands_in_category: Vec<&&Command> = subcmds
            .values()
            .filter(|sc| command_category(sc.get_name()) == Some(category))
            .collect();

        if commands_in_category.is_empty() {
            continue;
        }

        let _ = writeln!(out, "\n{heading}:");
        for sc in commands_in_category {
            let name = sc.get_name();
            let about = sc.get_about().map(|a| a.to_string()).unwrap_or_default();
            let _ = writeln!(out, "  {name:<longest$}  {about}");
        }
    }

    let _ = writeln!(
        out,
        "\nOptions:\n  -h, --help  Print help (see more with '--help')"
    );

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ClanCommandCategory::{CompatibilityAlias, Converter, NotAvailable, Transform};

    #[test]
    fn analysis_category_tracks_library_inventory() {
        for &command in AnalysisCommandName::ALL {
            assert_eq!(
                command_category(command.as_str()),
                Some(ClanCommandCategory::Analysis)
            );
        }
    }

    #[test]
    fn non_analysis_categories_remain_explicit() {
        assert_eq!(command_category("flo"), Some(Transform));
        assert_eq!(command_category("chat2text"), Some(Converter));
        assert_eq!(command_category("check"), Some(CompatibilityAlias));
        assert_eq!(command_category("mor"), Some(NotAvailable));
        assert_eq!(command_category("help"), None);
        assert_eq!(command_category("not-a-command"), None);
    }
}
