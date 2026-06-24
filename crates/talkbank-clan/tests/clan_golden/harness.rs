//! Shared harness helpers for CLAN golden integration tests.

use std::path::{Path, PathBuf};

use talkbank_clan::framework::{FilterConfig, SpeakerFilter, UtteranceRange, WordFilter};
use talkbank_model::SpeakerCode;

pub use crate::common::corpus_file;
use crate::common::run_clan_stdout_from_stdin;
pub use talkbank_clan::framework::OutputFormat;

/// CLAN selection-filter space for parity cases: the `+t` / `+s` / `+z` / `+x`
/// selection flags a golden may apply to the chatter run. Not every variant is
/// exercised by the current committed goldens (e.g. the `+s` word-gate
/// `WordInclude`), so, like the other aspirational parity infra in
/// `golden_parity.rs`, the enum carries a `dead_code` allowance rather than
/// being pruned to today's exact usage.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) enum FilterSpec {
    None,
    SpeakerInclude(&'static [&'static str]),
    /// CLAN `-t*X`: exclude the listed speakers.
    SpeakerExclude(&'static [&'static str]),
    WordInclude(&'static [&'static str]),
    UtteranceRange {
        start: usize,
        end: usize,
    },
    /// CLAN `+x C N U`: include only utterances whose length in `unit`
    /// satisfies the comparison against the threshold.
    UtteranceLength {
        comparison: talkbank_clan::framework::LengthComparison,
        threshold: usize,
        unit: talkbank_clan::framework::CountUnit,
    },
    /// CLAN `+t#ROLE`: include only speakers whose `@ID:` role matches.
    Role(&'static [&'static str]),
    /// CLAN `+t@ID="..."`: include only speakers whose `@ID` line matches the
    /// glob pattern (whole-string, case-insensitive wildcard).
    IdFilter(&'static str),
}

pub(crate) struct RustSnapshotCase {
    command: &'static str,
    file: &'static str,
    rust_args: &'static [&'static str],
    format: OutputFormat,
    rust_snapshot: &'static str,
}

impl RustSnapshotCase {
    pub(crate) fn new(
        command: &'static str,
        file: &'static str,
        rust_args: &'static [&'static str],
        format: OutputFormat,
        rust_snapshot: &'static str,
    ) -> Self {
        Self {
            command,
            file,
            rust_args,
            format,
            rust_snapshot,
        }
    }
}

pub(crate) struct RustSnapshotOutput {
    pub(crate) rust_snapshot: &'static str,
    pub(crate) rust_output: String,
}

pub(crate) fn run_rust_snapshot_case(case: &RustSnapshotCase) -> RustSnapshotOutput {
    let file = corpus_file(case.file);
    let rust_output = run_rust(case.command, &file, case.rust_args, case.format);
    RustSnapshotOutput {
        rust_snapshot: case.rust_snapshot,
        rust_output,
    }
}

/// Map test speaker codes to the domain `SpeakerCode` newtype.
fn speaker_codes(speakers: &[&str]) -> Vec<SpeakerCode> {
    speakers.iter().map(|s| SpeakerCode::from(*s)).collect()
}

pub(crate) fn build_filter(spec: FilterSpec) -> Option<FilterConfig> {
    match spec {
        FilterSpec::None => None,
        FilterSpec::SpeakerInclude(speakers) => Some(FilterConfig {
            speakers: SpeakerFilter {
                include: speaker_codes(speakers),
                ..SpeakerFilter::default()
            },
            ..FilterConfig::default()
        }),
        FilterSpec::SpeakerExclude(speakers) => Some(FilterConfig {
            speakers: SpeakerFilter {
                exclude: speaker_codes(speakers),
                ..SpeakerFilter::default()
            },
            ..FilterConfig::default()
        }),
        FilterSpec::Role(roles) => Some(FilterConfig {
            roles: talkbank_clan::framework::RoleFilter {
                include: roles.iter().map(|role| role.to_string()).collect(),
            },
            ..FilterConfig::default()
        }),
        FilterSpec::WordInclude(words) => Some(FilterConfig {
            words: WordFilter {
                include: words
                    .iter()
                    .map(|word| talkbank_clan::framework::WordPattern::from(*word))
                    .collect(),
                ..WordFilter::default()
            },
            ..FilterConfig::default()
        }),
        FilterSpec::UtteranceRange { start, end } => Some(FilterConfig {
            utterance_range: Some(UtteranceRange::new(start, end).expect("valid range")),
            ..FilterConfig::default()
        }),
        FilterSpec::UtteranceLength {
            comparison,
            threshold,
            unit,
        } => Some(FilterConfig {
            utterance_length: Some(talkbank_clan::framework::UtteranceLengthFilter {
                comparison,
                threshold: talkbank_clan::framework::LengthThreshold(threshold),
                unit,
                exclude_from_count: Vec::new(),
                restore: talkbank_clan::framework::RestoreMarkers::default(),
            }),
            ..FilterConfig::default()
        }),
        FilterSpec::IdFilter(pattern) => Some(FilterConfig {
            id_filter: Some(
                talkbank_clan::framework::parse_id_filter(pattern).expect("valid id filter"),
            ),
            ..FilterConfig::default()
        }),
    }
}

macro_rules! rust_snapshot_tests {
    ($($name:ident => $case:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let case = $case;
                let output = crate::harness::run_rust_snapshot_case(&case);
                insta::assert_snapshot!(output.rust_snapshot, output.rust_output);
            }
        )+
    };
}

pub(crate) use rust_snapshot_tests;

/// Run a legacy CLAN command by piping file content to standard input.
pub fn run_clan(command: &str, file: &Path, args: &[&str]) -> Option<String> {
    run_clan_stdout_from_stdin(command, file, args).map(|raw| strip_clan_header(&raw))
}

/// Strip the legacy CLAN boilerplate header from command output.
pub fn strip_clan_header(output: &str) -> String {
    let mut lines: Vec<&str> = output.lines().collect();

    if let Some(pos) = lines.iter().rposition(|l| l.trim() == "From pipe input") {
        lines = lines[pos + 1..].to_vec();
    }

    // Drop the CLAN progress line that can follow "From pipe input": a bare
    // number with trailing control characters (e.g. "8\r\t  "), which trims to a
    // SINGLE numeric token. A real frequency entry ("  1 Triangle") also leads
    // with a number but carries a following word, so it has two+ tokens and must
    // be kept; this matters for header-less combined output (`+o3`), whose first
    // line is a count entry, not a "Speaker:" banner.
    if let Some(first) = lines.first() {
        let mut tokens = first.split_whitespace();
        let is_progress_line = matches!(
            (tokens.next(), tokens.next()),
            (Some(tok), None) if tok.parse::<u64>().is_ok()
        );
        if is_progress_line {
            lines = lines[1..].to_vec();
        }
    }

    while lines.first().is_some_and(|l| l.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }

    lines.join(
        "
",
    )
}

/// Run the Rust implementation of a CLAN command and render the result.
pub fn run_rust(
    command_name: &str,
    file: &Path,
    extra_args: &[&str],
    format: OutputFormat,
) -> String {
    run_rust_filtered(command_name, file, extra_args, format, None)
}

/// Parse the `--capitalization <initial|mid>` argv pair from the
/// harness's `extra_args` slice into the domain enum.
fn parse_capitalization_arg(extra_args: &[&str]) -> talkbank_clan::framework::CapitalizationFilter {
    use talkbank_clan::framework::CapitalizationFilter;
    extra_args
        .windows(2)
        .find(|w| w[0] == "--capitalization")
        .map(|w| match w[1] {
            "initial" => CapitalizationFilter::InitialUpper,
            "mid" => CapitalizationFilter::MidUpper,
            _ => CapitalizationFilter::Any,
        })
        .unwrap_or(CapitalizationFilter::Any)
}

/// Parse the `--sort <alphabetical|frequency|reverse-concordance>` argv pair
/// from the harness's `extra_args` slice into the domain enum.
fn parse_sort_arg(extra_args: &[&str]) -> talkbank_clan::commands::freq::FreqSort {
    use talkbank_clan::commands::freq::FreqSort;
    extra_args
        .windows(2)
        .find(|w| w[0] == "--sort")
        .map(|w| match w[1] {
            "frequency" => FreqSort::Frequency,
            "reverse-concordance" => FreqSort::ReverseConcordance,
            _ => FreqSort::Alphabetical,
        })
        .unwrap_or(FreqSort::Alphabetical)
}

/// Parse the `--parenthesis-mode <remove-parens|keep-parens|remove-material>`
/// argv pair (CLAN `+r1`/`+r2`/`+r3`) into the domain enum; default is
/// `RemoveParens` (CLAN `+r1`, the default).
fn parse_parenthesis_mode(extra_args: &[&str]) -> talkbank_clan::framework::ParenthesisMode {
    use talkbank_clan::framework::ParenthesisMode;
    extra_args
        .windows(2)
        .find(|w| w[0] == "--parenthesis-mode")
        .map(|w| match w[1] {
            "keep-parens" => ParenthesisMode::KeepParens,
            "remove-material" => ParenthesisMode::RemoveMaterial,
            _ => ParenthesisMode::RemoveParens,
        })
        .unwrap_or(ParenthesisMode::RemoveParens)
}

/// Parse `--prosody-mode <strip|keep>` (CLAN `+r7`); default `Strip`.
fn parse_prosody_mode(extra_args: &[&str]) -> talkbank_clan::framework::ProsodyMode {
    use talkbank_clan::framework::ProsodyMode;
    extra_args
        .windows(2)
        .find(|w| w[0] == "--prosody-mode")
        .map(|w| match w[1] {
            "keep" => ProsodyMode::Keep,
            _ => ProsodyMode::Strip,
        })
        .unwrap_or(ProsodyMode::Strip)
}

/// Parse `--replacement-mode <replacement|original>` (CLAN `+r5`); default
/// `Replacement`.
fn parse_replacement_mode(extra_args: &[&str]) -> talkbank_clan::framework::ReplacementChoice {
    use talkbank_clan::framework::ReplacementChoice;
    extra_args
        .windows(2)
        .find(|w| w[0] == "--replacement-mode")
        .map(|w| match w[1] {
            "original" => ReplacementChoice::Original,
            _ => ReplacementChoice::Replacement,
        })
        .unwrap_or(ReplacementChoice::Replacement)
}

/// FREQ `+pS` word delimiters from a `--word-delimiters CHARS` argv pair, if
/// present (default: empty, no splitting).
fn parse_word_delimiters(extra_args: &[&str]) -> talkbank_clan::framework::WordDelimiters {
    extra_args
        .windows(2)
        .find(|w| w[0] == "--word-delimiters")
        .map(|w| talkbank_clan::framework::WordDelimiters::new(w[1].chars()))
        .unwrap_or_default()
}

/// FREQ `+bN` MATTR frame size from a `--mattr N` argv pair, if present.
fn parse_mattr_arg(extra_args: &[&str]) -> Option<talkbank_clan::framework::FrameSize> {
    extra_args
        .windows(2)
        .find(|w| w[0] == "--mattr")
        .and_then(|w| w[1].parse().ok())
}

/// FREQ multi-word match mode from `--multiword-order any` (CLAN `+c3`) and
/// `--multiword-scope sole` (CLAN `+c4`) argv pairs; defaults are sequence /
/// anywhere.
fn parse_multiword_match(extra_args: &[&str]) -> talkbank_clan::framework::MultiWordMatch {
    use talkbank_clan::framework::{MatchOrder, MatchScope, MultiWordMatch};
    let order = match extra_args.windows(2).find(|w| w[0] == "--multiword-order") {
        Some(w) if w[1] == "any" => MatchOrder::AnyOrder,
        _ => MatchOrder::Sequence,
    };
    let scope = match extra_args.windows(2).find(|w| w[0] == "--multiword-scope") {
        Some(w) if w[1] == "sole" => MatchScope::SoleContent,
        _ => MatchScope::Anywhere,
    };
    MultiWordMatch { order, scope }
}

/// FREQ include multiplicity from a `--search-multiplicity per-pattern` argv
/// pair (CLAN `+c2`); the default counts a word once.
fn parse_include_multiplicity(
    extra_args: &[&str],
) -> talkbank_clan::commands::freq::IncludeMultiplicity {
    use talkbank_clan::commands::freq::IncludeMultiplicity;
    match extra_args
        .windows(2)
        .find(|w| w[0] == "--search-multiplicity")
    {
        Some(w) if w[1] == "per-pattern" => IncludeMultiplicity::PerPattern,
        _ => IncludeMultiplicity::Once,
    }
}

/// FREQ multi-word display from a `--multiword-display matched` argv pair (CLAN
/// `+c7`); the default shows the search pattern.
fn parse_multiword_display(extra_args: &[&str]) -> talkbank_clan::commands::freq::MultiWordDisplay {
    use talkbank_clan::commands::freq::MultiWordDisplay;
    match extra_args
        .windows(2)
        .find(|w| w[0] == "--multiword-display")
    {
        Some(w) if w[1] == "matched" => MultiWordDisplay::MatchedWords,
        _ => MultiWordDisplay::Pattern,
    }
}

/// Build FREQ's per-word `+sWORD` / `-sWORD` filter from the harness's
/// `--include-word` / `--exclude-word` argv pairs, plus `--include-word-file`
/// / `--exclude-word-file` (`+s@F` / `-s@F`), loaded via the same
/// `load_word_list_file` the CLI uses. FREQ consumes word filtering at per-word
/// emit time (`WordFilterMode::PerWordEmit`), NOT the utterance gate
/// (`FilterSpec::WordInclude`), so the golden harness mirrors the CLI's
/// per-word path rather than the framework selection filter.
fn parse_freq_word_filter(
    extra_args: &[&str],
    case_sensitive: bool,
) -> talkbank_clan::framework::WordFilter {
    use talkbank_clan::framework::{WordFilter, WordFilterMode, WordPattern, load_word_list_file};
    let collect = |flag: &str| -> Vec<WordPattern> {
        extra_args
            .windows(2)
            .filter(|w| w[0] == flag)
            .map(|w| WordPattern::from(w[1]))
            .collect()
    };
    // `+s@F` / `-s@F`: load the word-list file (one pattern per line; skips
    // #-comments, `;%*` annotation lines, and blanks), matching the CLI.
    let collect_file = |flag: &str| -> Vec<WordPattern> {
        extra_args
            .windows(2)
            .filter(|w| w[0] == flag)
            .flat_map(|w| {
                load_word_list_file(Path::new(w[1]))
                    .unwrap_or_else(|e| panic!("load word-list file {}: {e}", w[1]))
            })
            .collect()
    };
    let mut include = collect("--include-word");
    include.extend(collect_file("--include-word-file"));
    let mut exclude = collect("--exclude-word");
    exclude.extend(collect_file("--exclude-word-file"));
    WordFilter {
        include,
        exclude,
        case_sensitive,
        mode: WordFilterMode::PerWordEmit,
    }
}

/// Run the Rust implementation of a CLAN command with an optional filter.
pub fn run_rust_filtered(
    command_name: &str,
    file: &Path,
    extra_args: &[&str],
    format: OutputFormat,
    filter: Option<FilterConfig>,
) -> String {
    use talkbank_clan::framework::{AnalysisRunner, CommandOutput};

    let files = vec![file.to_path_buf()];
    let runner = AnalysisRunner::with_filter(filter.unwrap_or_default());

    macro_rules! run_and_render {
        ($command:expr) => {
            match runner.run(&$command, &files) {
                Ok(r) => r.render(format),
                Err(e) => format!("Error: {e}"),
            }
        };
    }

    match command_name {
        "freq" => {
            use talkbank_clan::commands::freq::{CountSource, FreqCommand, FreqConfig};

            // Mirror the CLI dispatch: `--tier` (CLAN `+t%X`), `--exclude-tier`
            // (CLAN `-t%X`), and `--mor` are mutually-exclusive count sources; the
            // harness never passes more than one.
            let excluded_tiers: Vec<talkbank_clan::framework::TierKind> = extra_args
                .windows(2)
                .filter(|w| w[0] == "--exclude-tier")
                .map(|w| talkbank_clan::framework::TierKind::from(w[1]))
                .collect();
            let count_source = if !excluded_tiers.is_empty() {
                CountSource::MainPlusDependentTiersExcept(excluded_tiers)
            } else if let Some(w) = extra_args.windows(2).find(|w| w[0] == "--tier") {
                CountSource::DependentTierTokens(talkbank_clan::framework::TierKind::from(w[1]))
            } else if extra_args.contains(&"--mor") {
                CountSource::MorStructural
            } else {
                CountSource::MainTier
            };
            let capitalization = parse_capitalization_arg(extra_args);
            // Effective preserve-case state via the shared per-command polarity
            // (CLAN FREQ preserves by default; `+k` folds). Used by both the
            // keying and the per-word `+s` filter.
            let case_sensitive = talkbank_clan::service_types::AnalysisCommandName::Freq
                .effective_case_sensitive(extra_args.contains(&"--case-sensitive"));
            run_and_render!(FreqCommand::new(FreqConfig {
                count_source,
                capitalization,
                sort: parse_sort_arg(extra_args),
                word_list_only: extra_args.contains(&"--word-list-only"),
                types_tokens_only: extra_args.contains(&"--types-tokens-only"),
                case_sensitive,
                word_filter: parse_freq_word_filter(extra_args, case_sensitive),
                // Spreadsheet goldens drive the +d2/+d3 path directly, not via
                // this stdout-rendering arm.
                spreadsheet: None,
                frame_size: parse_mattr_arg(extra_args),
                multiword_match: parse_multiword_match(extra_args),
                include_multiplicity: parse_include_multiplicity(extra_args),
                multiword_display: parse_multiword_display(extra_args),
                include_zero_frequency: extra_args.contains(&"--include-zero-frequency"),
                combine_speakers: extra_args.contains(&"--combine-speakers"),
                parenthesis_mode: parse_parenthesis_mode(extra_args),
                prosody_mode: parse_prosody_mode(extra_args),
                include_retracings: extra_args.contains(&"--include-retracings"),
                replacement_mode: parse_replacement_mode(extra_args),
                word_delimiters: parse_word_delimiters(extra_args),
            }))
        }
        "mlu" => {
            use talkbank_clan::commands::mlu::{MluCommand, MluConfig, re_included_untranscribed};
            let words_only = extra_args.contains(&"--words");
            run_and_render!(MluCommand::new(MluConfig {
                words_only,
                combine_speakers: extra_args.contains(&"--combine-speakers"),
                re_included_untranscribed: re_included_untranscribed(
                    extra_args.contains(&"--include-xxx"),
                    extra_args.contains(&"--include-yyy"),
                ),
                ..MluConfig::default()
            }))
        }
        "mlt" => {
            use talkbank_clan::commands::mlt::MltCommand;
            run_and_render!(MltCommand::default())
        }
        "wdlen" => {
            use talkbank_clan::commands::wdlen::WdlenCommand;
            run_and_render!(WdlenCommand)
        }
        "freqpos" => {
            use talkbank_clan::commands::freqpos::FreqposCommand;
            run_and_render!(FreqposCommand::default())
        }
        "cooccur" => {
            use talkbank_clan::commands::cooccur::CooccurCommand;
            run_and_render!(CooccurCommand::default())
        }
        "dist" => {
            use talkbank_clan::commands::dist::DistCommand;
            run_and_render!(DistCommand::default())
        }
        "maxwd" => {
            use talkbank_clan::commands::maxwd::{MaxwdCommand, MaxwdConfig};
            let limit = extra_args
                .windows(2)
                .find(|w| w[0] == "--limit")
                .and_then(|w| w[1].parse().ok())
                .unwrap_or(20);
            run_and_render!(MaxwdCommand::new(MaxwdConfig {
                limit: talkbank_clan::framework::WordLimit::new(limit),
                ..MaxwdConfig::default()
            }))
        }
        "kwal" => {
            use talkbank_clan::commands::kwal::{KwalCommand, KwalConfig};
            let keywords: Vec<String> = extra_args
                .windows(2)
                .filter(|w| w[0] == "--keyword")
                .map(|w| w[1].to_owned())
                .collect();
            run_and_render!(KwalCommand::new(KwalConfig {
                keywords: keywords
                    .into_iter()
                    .map(talkbank_clan::framework::KeywordPattern::from)
                    .collect(),
                ..KwalConfig::default()
            }))
        }
        "chip" => {
            use talkbank_clan::commands::chip::ChipCommand;
            run_and_render!(ChipCommand)
        }
        "gemlist" => {
            use talkbank_clan::commands::gemlist::GemlistCommand;
            run_and_render!(GemlistCommand)
        }
        "modrep" => {
            use talkbank_clan::commands::modrep::ModrepCommand;
            run_and_render!(ModrepCommand)
        }
        "phonfreq" => {
            use talkbank_clan::commands::phonfreq::PhonfreqCommand;
            run_and_render!(PhonfreqCommand)
        }
        "vocd" => {
            use talkbank_clan::commands::vocd::{VocdCommand, VocdConfig};
            let capitalization = parse_capitalization_arg(extra_args);
            let config = VocdConfig {
                capitalization,
                // Effective preserve-case state via the shared per-command
                // polarity (CLAN VOCD preserves by default; `+k` folds).
                case_sensitive: talkbank_clan::service_types::AnalysisCommandName::Vocd
                    .effective_case_sensitive(extra_args.contains(&"--case-sensitive")),
                ..VocdConfig::default()
            };
            run_and_render!(VocdCommand::new(config))
        }
        "combo" => {
            use talkbank_clan::commands::combo::{ComboCommand, ComboConfig, SearchExpr};
            let search: Vec<SearchExpr> = extra_args
                .windows(2)
                .filter(|w| w[0] == "--search")
                .map(|w| SearchExpr::parse(w[1]))
                .collect();
            run_and_render!(ComboCommand::new(ComboConfig {
                search,
                exclude: vec![],
                ..ComboConfig::default()
            }))
        }
        "codes" => {
            use talkbank_clan::commands::codes::{CodesCommand, CodesConfig};
            run_and_render!(CodesCommand::new(CodesConfig {
                max_depth: talkbank_clan::framework::CodeDepth::new(0)
            }))
        }
        "chains" => {
            use talkbank_clan::commands::chains::{ChainsCommand, ChainsConfig};
            run_and_render!(ChainsCommand::new(ChainsConfig::default()))
        }
        "sugar" => {
            use talkbank_clan::commands::sugar::{SugarCommand, SugarConfig};
            run_and_render!(SugarCommand::new(SugarConfig {
                min_utterances: talkbank_clan::framework::UtteranceLimit::new(0)
            }))
        }
        "timedur" => {
            use talkbank_clan::commands::timedur::TimedurCommand;
            run_and_render!(TimedurCommand)
        }
        "trnfix" => {
            use talkbank_clan::commands::trnfix::{TrnfixCommand, TrnfixConfig};
            let tier1 = extra_args
                .windows(2)
                .find(|w| w[0] == "--tier1")
                .map(|w| w[1].to_string())
                .unwrap_or_else(|| "pho".to_string());
            let tier2 = extra_args
                .windows(2)
                .find(|w| w[0] == "--tier2")
                .map(|w| w[1].to_string())
                .unwrap_or_else(|| "mod".to_string());
            run_and_render!(TrnfixCommand::new(TrnfixConfig {
                tier1: talkbank_clan::framework::TierKind::from(tier1.as_str()),
                tier2: talkbank_clan::framework::TierKind::from(tier2.as_str()),
            }))
        }
        "uniq" => {
            use talkbank_clan::commands::uniq::{UniqCommand, UniqConfig};
            run_and_render!(UniqCommand::new(UniqConfig {
                sort_by_frequency: false,
            }))
        }
        "dss" => {
            use talkbank_clan::commands::dss::{DssCommand, DssConfig};
            let cmd = DssCommand::new(DssConfig::default()).expect("DSS init failed");
            run_and_render!(cmd)
        }
        "eval" => {
            use talkbank_clan::commands::eval::{EvalCommand, EvalConfig};
            run_and_render!(EvalCommand::new(EvalConfig::default()))
        }
        "flucalc" => {
            use talkbank_clan::commands::flucalc::{FlucalcCommand, FlucalcConfig};
            run_and_render!(FlucalcCommand::new(FlucalcConfig {
                syllable_mode: false,
            }))
        }
        "ipsyn" => {
            use talkbank_clan::commands::ipsyn::{IpsynCommand, IpsynConfig};
            let cmd = IpsynCommand::new(IpsynConfig::default()).expect("IPSYN init failed");
            run_and_render!(cmd)
        }
        "kideval" => {
            use talkbank_clan::commands::kideval::{KidevalCommand, KidevalConfig};
            let cmd = KidevalCommand::new(KidevalConfig::default()).expect("KIDEVAL init failed");
            run_and_render!(cmd)
        }
        "keymap" => {
            use talkbank_clan::commands::keymap::{KeymapCommand, KeymapConfig};
            let keywords: Vec<String> = extra_args
                .windows(2)
                .filter(|w| w[0] == "--keyword")
                .map(|w| w[1].to_owned())
                .collect();
            let tier = extra_args
                .windows(2)
                .find(|w| w[0] == "--tier")
                .map(|w| w[1].to_string())
                .unwrap_or_else(|| "cod".to_string());
            run_and_render!(KeymapCommand::new(KeymapConfig {
                keywords: keywords
                    .into_iter()
                    .map(talkbank_clan::framework::KeywordPattern::from)
                    .collect(),
                tier: talkbank_clan::framework::TierKind::from(tier.as_str()),
            }))
        }
        "complexity" => {
            use talkbank_clan::commands::complexity::ComplexityCommand;
            run_and_render!(ComplexityCommand)
        }
        "corelex" => {
            use talkbank_clan::commands::corelex::{CorelexCommand, CorelexConfig};
            let threshold = extra_args
                .windows(2)
                .find(|w| w[0] == "--threshold")
                .and_then(|w| w[1].parse().ok())
                .unwrap_or(2);
            run_and_render!(CorelexCommand::new(CorelexConfig {
                min_frequency: talkbank_clan::framework::FrequencyThreshold::new(threshold),
            }))
        }
        "wdsize" => {
            use talkbank_clan::commands::wdsize::{WdsizeCommand, WdsizeConfig};
            let use_main_tier = extra_args.contains(&"--main-tier");
            run_and_render!(WdsizeCommand::new(WdsizeConfig {
                use_main_tier,
                ..WdsizeConfig::default()
            }))
        }
        other => panic!("Unknown command: {other}"),
    }
}

/// Run FREQ in a spreadsheet mode (`+d2`/`+d3`) across `files` in-process and
/// return the SpreadsheetML the CLI would write (`to_spreadsheet().write_xml()`).
/// Main-tier run (`mor_based = false`), so the TTR-caveat rows are emitted.
pub(crate) fn run_rust_spreadsheet(
    files: &[PathBuf],
    mode: talkbank_clan::commands::freq::FreqSpreadsheetMode,
    filter: Option<FilterConfig>,
) -> String {
    use talkbank_clan::commands::freq::{FreqCommand, FreqConfig};
    use talkbank_clan::framework::AnalysisRunner;
    let runner = AnalysisRunner::with_filter(filter.unwrap_or_default());
    let config = FreqConfig {
        spreadsheet: Some(mode),
        ..FreqConfig::default()
    };
    let result = runner
        .run(&FreqCommand::new(config), files)
        .expect("freq spreadsheet run");
    result
        .to_spreadsheet(mode, false)
        .write_xml()
        .expect("write spreadsheet xml")
}
