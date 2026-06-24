//! CLAN analysis-command adapters for the `chatter` CLI.
//!
//! Each match arm here now performs only CLI-facing adaptation: convert parsed
//! clap arguments into a typed
//! `AnalysisCommandName` plus
//! `AnalysisOptions`, then delegate defaults,
//! validation, and execution to the library-owned builder and service
//! boundaries. Keep command construction and shared output policy inside
//! `talkbank-clan`; keep CLI argument mapping and terminal UX here.

use crate::cli::{ClanCommands, FreqposPositionArg};
use talkbank_clan::commands::codes::CodesConfig;
use talkbank_clan::commands::corelex::CorelexConfig;
use talkbank_clan::commands::dss::DssConfig;
use talkbank_clan::commands::freq::{CountSource, FreqSpreadsheetMode, IncludeMultiplicity};
use talkbank_clan::commands::freqpos::PositionClassification;
use talkbank_clan::commands::ipsyn::IpsynConfig;
use talkbank_clan::commands::keymap::KeymapConfig;
use talkbank_clan::commands::maxwd::MaxwdConfig;
use talkbank_clan::commands::rely::RelyConfig;
use talkbank_clan::commands::trnfix::TrnfixConfig;
use talkbank_clan::service_types::{
    AnalysisCommandName, AnalysisOptions, ChainsOptions, CodesOptions, ComboOptions,
    CorelexOptions, DistOptions, DssOptions, EvalOptions, FlucalcOptions, FreqOptions,
    IpsynOptions, KeymapOptions, KidevalOptions, KwalOptions, MaxwdOptions, MltOptions, MluOptions,
    MortableOptions, RelyOptions, ScriptOptions, SugarOptions, TrnfixOptions, UniqOptions,
    VocdOptions, WdsizeOptions,
};

use super::helpers::{
    capitalization_to_filter, load_search_expr_files_or_exit, multiword_args_to_match,
    multiword_display_to_display, option_if_not_default, parenthesis_mode_arg_to_mode,
    prosody_mode_arg_to_mode, replacement_mode_arg_to_choice, run_analysis_and_print,
    run_paired_analysis_and_print, search_multiplicity_to_include, sort_arg_to_freq_sort,
    spreadsheet_arg_to_mode,
};

// Err carries the unhandled command back to the caller's try-this-then-try-that
// dispatcher chain. The "large" size is just clap's ClanCommands enum; this is
// control flow, not error handling, and the Err arm runs once per startup.
#[allow(clippy::result_large_err)]
pub(super) fn dispatch(command: ClanCommands) -> Result<(), ClanCommands> {
    match command {
        ClanCommands::Freq {
            path,
            mor,
            tier,
            exclude_tier,
            capitalization,
            sort,
            word_list_only,
            types_tokens_only,
            spreadsheet,
            speaker_percentage,
            reject_clan_gem,
            mattr,
            multiword_order,
            multiword_scope,
            search_multiplicity,
            multiword_display,
            include_zero_frequency,
            combine_speakers,
            parenthesis_mode,
            replacement_mode,
            prosody_mode,
            word_delimiters,
            common,
            ..
        } => {
            // CLAN FREQ has no `+g`/`-g` gem flag (it rejects them; gem-limiting
            // is the GEM program). The rewriter routes `+gX`/`-gX` here so we
            // refuse them rather than squat the slot on `--gem` (which stays a
            // chatter-only convenience reachable directly).
            if !reject_clan_gem.is_empty() {
                super::helpers::exit_with_error(
                    "Error: freq has no +g/-g gem flag (CLAN rejects it; \
                     gem-limiting is the GEM program). Use --gem / --exclude-gem \
                     for chatter's gem filter."
                        .to_owned(),
                );
            }
            // What freq counts. `--mor` (chatter's structural %mor), `--tier`
            // (CLAN `+t%X`, raw whitespace tokens of one dependent tier), and
            // `--exclude-tier` (CLAN `-t%X`, main tier + all dependent tiers
            // except the named ones) are mutually-exclusive ways to pick the
            // source; the combination guard lives here at the dispatch boundary
            // (the flag-combination-guards-at-dispatch rule). CLAN's `+t%mor`
            // rewrites to `--tier mor` (NOT `--mor`), and `-t%X` to
            // `--exclude-tier X`. The `+t`/`-t` *combination* semantics (an
            // include and exclude together) are a separate, not-yet-implemented
            // row, so any mix errors.
            let count_source = match (mor, tier, exclude_tier) {
                (false, None, ex) if ex.is_empty() => CountSource::MainTier,
                (true, None, ex) if ex.is_empty() => CountSource::MorStructural,
                (false, Some(tier), ex) if ex.is_empty() => CountSource::DependentTierTokens(tier),
                (false, None, ex) if !ex.is_empty() => {
                    CountSource::MainPlusDependentTiersExcept(ex)
                }
                _ => super::helpers::exit_with_error(
                    "Error: --mor, --tier, and --exclude-tier each select what freq counts \
                     and cannot be combined (the +t/-t combination semantics are not yet \
                     implemented)."
                        .to_owned(),
                ),
            };
            let mut common = common;
            // CLAN FREQ preserves case by default; `+k` folds. Resolve the
            // effective case ONCE and write it back to `common` so BOTH the
            // per-word `+s` filter (take_per_word_filter) and the utterance-gate
            // filter (build_filter, via run_analysis_and_print) see it. The
            // polarity lives in AnalysisCommandName::effective_case_sensitive.
            common.case_sensitive =
                AnalysisCommandName::Freq.effective_case_sensitive(common.case_sensitive);
            let case_sensitive = common.case_sensitive;
            // FREQ's `+sWORD` is per-word, not utterance-gate; the
            // helper extracts the patterns and clears them from
            // `common` so the framework's utterance gate is a no-op
            // on word filtering for FREQ.
            let word_filter = super::helpers::take_per_word_filter(&mut common)
                .unwrap_or_else(|err| super::helpers::exit_with_error(format!("Error: {err}")));

            // CLAN `+dCN` (the percent-of-speakers filter) cannot be combined
            // with `+d5` (zeroMatch): freq.cpp:867-870 / 890-893 error either
            // order, AT FLAG-PARSE TIME (before the `+s`-content validation), so
            // this combination guard runs BEFORE the `+c2`/`+d5` content guards
            // below to match CLAN's precedence. `+dCN` is also a distinct
            // spreadsheet destination from `+d2`/`+d3`/`+d20` (CLAN's single
            // `onlydata`), so it cannot be combined with `--spreadsheet`.
            if let Some(filter) = speaker_percentage {
                if include_zero_frequency {
                    super::helpers::exit_with_error(format!(
                        "Error: +d{}{} option can't be used with +d5 option.",
                        filter.comparison.as_clan_str(),
                        filter.percent.value(),
                    ));
                }
                if spreadsheet.is_some() {
                    super::helpers::exit_with_error(
                        "Error: --speaker-percentage (+dCN) cannot be combined with \
                         --spreadsheet (+d2/+d3/+d20)."
                            .to_owned(),
                    );
                }
            }

            // CLAN `+c2` (per-pattern counting) is single-word only and requires
            // wildcard patterns; CLAN rejects it without a wildcard +s and when
            // combined with a multi-word +s group (freq.cpp:432-438, 455-459).
            let include_multiplicity = search_multiplicity_to_include(search_multiplicity);
            if include_multiplicity == IncludeMultiplicity::PerPattern {
                let includes = &word_filter.include;
                if includes
                    .iter()
                    .any(|p| p.as_str().split_whitespace().count() > 1)
                {
                    super::helpers::exit_with_error(
                        "Error: +c2 cannot be used with a multi-word +s search.".to_owned(),
                    );
                }
                if !includes.iter().any(|p| p.as_str().contains('*')) {
                    super::helpers::exit_with_error(
                        "Error: +c2 requires a +s search pattern with a wildcard (*).".to_owned(),
                    );
                }
            }

            // CLAN `+d5` (zeroMatch): show each literal +s word even when
            // unmatched, at count 0. CLAN requires at least one +s word
            // (freq.cpp:449) and rejects wildcards (* % _) or duplicate +s words
            // (freq.cpp:444, isFoundWildCard(TRUE)). Enforced here, mirroring the
            // +c2 guards, because these depend on the extracted per-word filter.
            if include_zero_frequency {
                let includes = &word_filter.include;
                if includes.is_empty() {
                    super::helpers::exit_with_error(
                        "Error: +d5 requires at least one +s search word.".to_owned(),
                    );
                }
                if includes
                    .iter()
                    .any(|p| p.as_str().contains(['*', '%', '_']))
                {
                    super::helpers::exit_with_error(
                        "Error: +d5 cannot be used with a wildcard (* % _) +s search word."
                            .to_owned(),
                    );
                }
                let mut seen = std::collections::HashSet::new();
                if includes.iter().any(|p| !seen.insert(p.as_str())) {
                    super::helpers::exit_with_error(
                        "Error: +d5 cannot be used with duplicate +s search words.".to_owned(),
                    );
                }
            }

            // CLAN `+pS` (the `--word-delimiters` value): the characters become
            // extra word delimiters. An empty value (CLAN's bare `+p`) is an
            // error (cutt.cpp:9802); whitespace-only collapses to empty too.
            let word_delimiters = match word_delimiters {
                None => talkbank_clan::framework::WordDelimiters::default(),
                Some(chars) => {
                    let delimiters = talkbank_clan::framework::WordDelimiters::new(chars.chars());
                    if delimiters.is_empty() {
                        super::helpers::exit_with_error(
                            "Error: please specify word delimiter characters with +p option."
                                .to_owned(),
                        );
                    }
                    delimiters
                }
            };

            run_analysis_and_print(
                AnalysisOptions::Freq(FreqOptions {
                    count_source,
                    capitalization: capitalization_to_filter(capitalization),
                    sort: sort_arg_to_freq_sort(sort),
                    word_list_only,
                    types_tokens_only,
                    case_sensitive,
                    word_filter,
                    // `--speaker-percentage` (CLAN `+dCN`) drives the
                    // percent-of-speakers spreadsheet; otherwise `--spreadsheet`
                    // (`+d2`/`+d3`/`+d20`). The two are guarded mutually-exclusive
                    // above, so this `or_else` never silently drops one.
                    spreadsheet: speaker_percentage
                        .map(FreqSpreadsheetMode::PercentOfSpeakers)
                        .or_else(|| spreadsheet_arg_to_mode(spreadsheet)),
                    frame_size: mattr,
                    multiword_match: multiword_args_to_match(multiword_order, multiword_scope),
                    include_multiplicity,
                    multiword_display: multiword_display_to_display(multiword_display),
                    include_zero_frequency,
                    combine_speakers,
                    parenthesis_mode: parenthesis_mode_arg_to_mode(parenthesis_mode),
                    prosody_mode: prosody_mode_arg_to_mode(prosody_mode),
                    // CLAN `+r6` -> `--include-retracings`, a shared common arg.
                    include_retracings: common.include_retracings,
                    replacement_mode: replacement_mode_arg_to_choice(replacement_mode),
                    word_delimiters,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Mlu {
            path,
            words,
            mut exclude_solo_word,
            exclude_solo_word_file,
            combine_speakers,
            include_xxx,
            include_yyy,
            common,
            ..
        } => {
            exclude_solo_word.extend(load_search_expr_files_or_exit(&exclude_solo_word_file));
            run_analysis_and_print(
                AnalysisOptions::Mlu(MluOptions {
                    words,
                    solo_word_exclusions: exclude_solo_word,
                    combine_speakers,
                    include_xxx,
                    include_yyy,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Mlt {
            path,
            mut exclude_solo_word,
            exclude_solo_word_file,
            common,
            ..
        } => {
            exclude_solo_word.extend(load_search_expr_files_or_exit(&exclude_solo_word_file));
            run_analysis_and_print(
                AnalysisOptions::Mlt(MltOptions {
                    solo_word_exclusions: exclude_solo_word,
                }),
                &path,
                &common,
            )
        }
        ClanCommands::Wdlen { path, common, .. } => {
            run_analysis_and_print(AnalysisOptions::Wdlen, &path, &common);
        }
        ClanCommands::Wdsize {
            path,
            main_tier,
            length_filter,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisOptions::Wdsize(WdsizeOptions {
                    main_tier,
                    length_filter,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Maxwd {
            path,
            limit,
            unique_length_only,
            exclude_length,
            common,
            ..
        } => {
            let case_sensitive = common.case_sensitive;
            run_analysis_and_print(
                AnalysisOptions::Maxwd(MaxwdOptions {
                    limit: option_if_not_default(limit, MaxwdConfig::default().limit),
                    unique_length_only,
                    exclude_lengths: exclude_length,
                    case_sensitive,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Freqpos {
            path,
            position_classification,
            common,
            ..
        } => {
            let pc = match position_classification {
                FreqposPositionArg::Last => PositionClassification::FirstLastOther,
                FreqposPositionArg::Second => PositionClassification::FirstSecondOther,
            };
            let case_sensitive = common.case_sensitive;
            run_analysis_and_print(
                AnalysisOptions::Freqpos(talkbank_clan::service_types::FreqposOptions {
                    position_classification: pc,
                    case_sensitive,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Timedur { path, common, .. } => {
            run_analysis_and_print(AnalysisOptions::Timedur, &path, &common);
        }
        ClanCommands::Kwal {
            path,
            keyword,
            strict_match,
            legal_chat,
            context_before,
            context_after,
            common,
            ..
        } => {
            let case_sensitive = common.case_sensitive;
            run_analysis_and_print(
                AnalysisOptions::Kwal(KwalOptions {
                    keywords: keyword
                        .into_iter()
                        .map(talkbank_clan::framework::KeywordPattern::from)
                        .collect(),
                    strict_match,
                    case_sensitive,
                    legal_chat,
                    context_before,
                    context_after,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Gemlist { path, common, .. } => {
            run_analysis_and_print(AnalysisOptions::Gemlist, &path, &common);
        }
        ClanCommands::Combo {
            path,
            mut search,
            mut exclude_search,
            search_file,
            exclude_search_file,
            first_match_only,
            dedupe_matches,
            context_before,
            context_after,
            common,
            ..
        } => {
            search.extend(load_search_expr_files_or_exit(&search_file));
            exclude_search.extend(load_search_expr_files_or_exit(&exclude_search_file));
            let case_sensitive = common.case_sensitive;
            run_analysis_and_print(
                AnalysisOptions::Combo(ComboOptions {
                    search,
                    exclude_search,
                    first_match_only,
                    dedupe_matches,
                    case_sensitive,
                    context_before,
                    context_after,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Cooccur {
            path,
            no_frequency_counts,
            cluster_size,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisOptions::Cooccur(talkbank_clan::service_types::CooccurOptions {
                    no_frequency_counts,
                    cluster_size,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Dist {
            path,
            once_per_turn,
            common,
            ..
        } => {
            let case_sensitive = common.case_sensitive;
            run_analysis_and_print(
                AnalysisOptions::Dist(DistOptions {
                    once_per_turn,
                    case_sensitive,
                }),
                &path,
                &common,
            )
        }
        ClanCommands::Chip { path, common, .. } => {
            run_analysis_and_print(AnalysisOptions::Chip, &path, &common);
        }
        ClanCommands::Phonfreq { path, common, .. } => {
            run_analysis_and_print(AnalysisOptions::Phonfreq, &path, &common);
        }
        ClanCommands::Modrep { path, common, .. } => {
            run_analysis_and_print(AnalysisOptions::Modrep, &path, &common);
        }
        ClanCommands::Vocd {
            path,
            capitalization,
            common,
            ..
        } => {
            let mut common = common;
            // CLAN VOCD preserves case by default; `+k` folds. Resolve once and
            // write back to `common` so the utterance-gate `+s` filter
            // (build_filter, via run_analysis_and_print) sees it too.
            common.case_sensitive =
                AnalysisCommandName::Vocd.effective_case_sensitive(common.case_sensitive);
            let case_sensitive = common.case_sensitive;
            run_analysis_and_print(
                AnalysisOptions::Vocd(VocdOptions {
                    capitalization: capitalization_to_filter(capitalization),
                    case_sensitive,
                }),
                &path,
                &common,
            )
        }
        ClanCommands::Uniq {
            path, sort, common, ..
        } => {
            run_analysis_and_print(
                AnalysisOptions::Uniq(UniqOptions {
                    sort_by_frequency: sort,
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Codes {
            path,
            max_depth,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisOptions::Codes(CodesOptions {
                    max_depth: option_if_not_default(max_depth, CodesConfig::default().max_depth),
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Trnfix {
            path,
            tier1,
            tier2,
            common,
            ..
        } => {
            let default = TrnfixConfig::default();
            run_analysis_and_print(
                AnalysisOptions::Trnfix(TrnfixOptions {
                    tier1: option_if_not_default(tier1, default.tier1),
                    tier2: option_if_not_default(tier2, default.tier2),
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Sugar {
            path,
            min_utterances,
            common,
            ..
        } => {
            if common.speaker.is_empty() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify at least one speaker tier code with \"+t\" option on command line.",
                );
            }
            run_analysis_and_print(
                AnalysisOptions::Sugar(SugarOptions { min_utterances }),
                &path,
                &common,
            );
        }
        ClanCommands::Mortable {
            path,
            script,
            common,
            ..
        } => {
            let script = script.unwrap_or_else(|| {
                super::helpers::exit_with_clan_refusal(
                    "Please specify language script file name with \"+l\" option.\n\
                     For example, \"mortable +leng\" or \"mortable +leng.cut\".",
                )
            });
            run_analysis_and_print(
                AnalysisOptions::Mortable(MortableOptions {
                    script_path: Some(script),
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Chains {
            path, tier, common, ..
        } => {
            let tier = tier.unwrap_or_else(|| {
                super::helpers::exit_with_clan_refusal(
                    "Please specify a code tier with \"+t\" option.",
                )
            });
            run_analysis_and_print(
                AnalysisOptions::Chains(ChainsOptions { tier: Some(tier) }),
                &path,
                &common,
            );
        }
        ClanCommands::Complexity { path, common, .. } => {
            run_analysis_and_print(AnalysisOptions::Complexity, &path, &common);
        }
        ClanCommands::Corelex {
            path,
            threshold,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisOptions::Corelex(CorelexOptions {
                    threshold: option_if_not_default(
                        threshold,
                        CorelexConfig::default().min_frequency,
                    ),
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Keymap {
            path,
            keyword,
            tier,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisOptions::Keymap(KeymapOptions {
                    keywords: keyword
                        .into_iter()
                        .map(talkbank_clan::framework::KeywordPattern::from)
                        .collect(),
                    tier: option_if_not_default(tier, KeymapConfig::default().tier),
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Script {
            path,
            template,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisOptions::Script(ScriptOptions {
                    template_path: Some(template),
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Rely {
            file1,
            file2,
            tier,
            format,
        } => {
            run_paired_analysis_and_print(
                AnalysisOptions::Rely(RelyOptions {
                    second_file: Some(file2),
                    tier: option_if_not_default(tier, RelyConfig::default().tier),
                }),
                &file1,
                format,
            );
        }
        ClanCommands::Flucalc { path, common, .. } => {
            run_analysis_and_print(
                AnalysisOptions::Flucalc(FlucalcOptions::default()),
                &path,
                &common,
            );
        }
        ClanCommands::Dss {
            path,
            rules,
            max_utterances,
            common,
            ..
        } => {
            if common.speaker.is_empty() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify at least one speaker tier name with \"+t\" option.",
                );
            }
            run_analysis_and_print(
                AnalysisOptions::Dss(DssOptions {
                    rules_path: rules,
                    max_utterances: option_if_not_default(
                        max_utterances,
                        DssConfig::default().max_utterances,
                    ),
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Ipsyn {
            path,
            rules,
            max_utterances,
            common,
            ..
        } => {
            if rules.is_none() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify ipsyn rules file name with \"+l\" option.\n\
                     For example, \"ipsyn +leng\" or \"ipsyn +leng.cut\".",
                );
            }
            run_analysis_and_print(
                AnalysisOptions::Ipsyn(IpsynOptions {
                    rules_path: rules,
                    max_utterances: option_if_not_default(
                        max_utterances,
                        IpsynConfig::default().max_utterances,
                    ),
                }),
                &path,
                &common,
            );
        }
        ClanCommands::Eval { path, common, .. } => {
            if common.speaker.is_empty() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify at least one speaker tier code with \"+t\" option on command line.",
                );
            }
            run_analysis_and_print(
                AnalysisOptions::Eval(EvalOptions::default()),
                &path,
                &common,
            );
        }
        ClanCommands::Kideval {
            path,
            dss_rules,
            ipsyn_rules,
            common,
            ..
        } => {
            // CLAN's kideval refuses without `+l<script>` (the
            // language file that bundles DSS + IPSYN + EVAL rules
            // for one language). chatter has separate --dss-rules
            // and --ipsyn-rules flags; require at least one so the
            // refusal triggers when neither is given. Match CLAN's
            // exact two-line wording.
            if dss_rules.is_none() && ipsyn_rules.is_none() {
                // CLAN's kideval refusal includes a leading blank
                // line; preserve that quirk for byte-level parity.
                super::helpers::exit_with_clan_refusal(
                    "\nPlease specify language script file name with \"+l\" option.\n\
                     For example, \"kideval +leng\" or \"kideval +leng.cut\".",
                );
            }
            run_analysis_and_print(
                AnalysisOptions::Kideval(KidevalOptions {
                    dss_rules_path: dss_rules,
                    ipsyn_rules_path: ipsyn_rules,
                    ..KidevalOptions::default()
                }),
                &path,
                &common,
            );
        }
        ClanCommands::EvalD { path, common, .. } => {
            if common.speaker.is_empty() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify at least one speaker tier code with \"+t\" option on command line.",
                );
            }
            run_analysis_and_print(
                AnalysisOptions::EvalDialect(EvalOptions::default()),
                &path,
                &common,
            );
        }
        other => return Err(other),
    }
    Ok(())
}
