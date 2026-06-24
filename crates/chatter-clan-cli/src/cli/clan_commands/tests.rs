use clap::Parser;

use super::ClanCommands;
use talkbank_clan::commands::corelex::CorelexConfig;
use talkbank_clan::commands::rely::RelyConfig;

fn run_with_large_stack(test: impl FnOnce() + Send + 'static) {
    let join_result = std::thread::Builder::new()
        .name("clan-args-test".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(test)
        .expect("spawn clan args test thread")
        .join();
    match join_result {
        Ok(()) => {}
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[derive(Parser)]
struct TestCli {
    #[command(subcommand)]
    command: ClanCommands,
}

#[test]
fn corelex_default_matches_library_config() {
    run_with_large_stack(|| {
        let parsed = TestCli::parse_from(["test-cli", "corelex", "sample.cha"]);

        let ClanCommands::Corelex { threshold, .. } = parsed.command else {
            panic!("expected corelex command");
        };

        assert_eq!(threshold, CorelexConfig::default().min_frequency);
    });
}

#[test]
fn rely_default_tier_matches_library_config() {
    run_with_large_stack(|| {
        let parsed = TestCli::parse_from(["test-cli", "rely", "left.cha", "right.cha"]);

        let ClanCommands::Rely { tier, .. } = parsed.command else {
            panic!("expected rely command");
        };

        assert_eq!(tier, RelyConfig::default().tier);
    });
}

#[test]
fn gemfreq_uses_common_gem_filter() {
    run_with_large_stack(|| {
        let parsed = TestCli::parse_from(["test-cli", "gemfreq", "--gem", "episode", "sample.cha"]);

        let ClanCommands::Gemfreq { common, .. } = parsed.command else {
            panic!("expected gemfreq command");
        };

        assert_eq!(common.gem, vec!["episode"]);
    });
}

#[test]
fn gemfreq_requires_gem_filter() {
    run_with_large_stack(|| {
        let error = match TestCli::try_parse_from(["test-cli", "gemfreq", "sample.cha"]) {
            Ok(_) => panic!("gemfreq should require --gem"),
            Err(error) => error,
        };
        let rendered = error.to_string();

        assert!(
            rendered.contains("--gem"),
            "expected missing --gem error, got `{rendered}`"
        );
    });
}

#[test]
fn check_list_errors_allows_omitting_path() {
    run_with_large_stack(|| {
        let parsed = TestCli::parse_from(["test-cli", "check", "--list-errors"]);

        let ClanCommands::Check {
            paths, list_errors, ..
        } = parsed.command
        else {
            panic!("expected check command");
        };

        assert!(list_errors);
        assert!(paths.is_empty());
    });
}

// ----------------------------------------------------------------
// Inapplicable `+wN` / `-wN` context-window flags on aggregate commands.
//
// `+wN` (post-context) / `-wN` (pre-context) is a KWAL/COMBO keyword-context
// window. It is INAPPLICABLE to the aggregate commands (MLU, MLT, WDLEN, MAXWD,
// FREQPOS, FREQ), which produce means/totals/histograms with no per-match
// emission to surround. CLAN's binary, given `+w` on these, prints empty
// output (a side-effect of the shared context machinery, not a no-op); chatter
// must REJECT the flag instead (talkbank-clan/CLAUDE.md: "Inapplicable flags
// must ERROR"). The rewriter still converts `+w3` / `-w2` to `--context-after`
// / `--context-before`; with no clap consumer on these commands, clap rejects
// it, which is the intended error.
// ----------------------------------------------------------------

/// Parameterized helper: asserts the aggregate `<cmd>` REJECTS both
/// `--context-after N` and `--context-before N` (the long-form output of the
/// legacy `+wN` / `-wN` rewriter), because the flag is inapplicable.
fn assert_aggregate_rejects_inherited_context(cmd: &'static str) {
    for flag in ["--context-after", "--context-before"] {
        assert!(
            TestCli::try_parse_from(["test-cli", cmd, flag, "3", "sample.cha"]).is_err(),
            "{cmd} must REJECT {flag}: +w/-w (keyword context) is inapplicable to \
             aggregate commands"
        );
    }
}

#[test]
fn mlu_rejects_inherited_context_after() {
    run_with_large_stack(|| {
        assert!(
            TestCli::try_parse_from(["test-cli", "mlu", "--context-after", "3", "sample.cha"])
                .is_err(),
            "mlu must reject --context-after: +w is inapplicable to aggregate commands"
        );
    });
}

#[test]
fn mlu_rejects_context_both_directions() {
    run_with_large_stack(|| assert_aggregate_rejects_inherited_context("mlu"));
}

#[test]
fn mlt_rejects_context_both_directions() {
    run_with_large_stack(|| assert_aggregate_rejects_inherited_context("mlt"));
}

#[test]
fn wdlen_rejects_context_both_directions() {
    run_with_large_stack(|| assert_aggregate_rejects_inherited_context("wdlen"));
}

#[test]
fn maxwd_rejects_context_both_directions() {
    run_with_large_stack(|| assert_aggregate_rejects_inherited_context("maxwd"));
}

#[test]
fn freqpos_rejects_context_both_directions() {
    run_with_large_stack(|| assert_aggregate_rejects_inherited_context("freqpos"));
}

#[test]
fn freq_rejects_context_both_directions() {
    run_with_large_stack(|| assert_aggregate_rejects_inherited_context("freq"));
}
