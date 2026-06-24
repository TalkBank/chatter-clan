//! CLI-argument to domain-type conversions for the `chatter clan` adapters.
//!
//! Each `*_to_*` / `*_arg_to_*` function maps one parsed clap argument
//! (`crate::cli::*Arg`) onto the corresponding `talkbank_clan` domain enum, plus
//! the small `option_if_not_default` config-diff helper. Extracted verbatim from
//! `analysis.rs`; the dispatch match calls these by name through the
//! `pub(super) use arg_conversions::*;` re-export in `helpers/mod.rs`.

use crate::cli::{
    CapitalizationArg, MultiWordDisplayArg, MultiWordOrderArg, MultiWordScopeArg,
    ParenthesisModeArg, ProsodyModeArg, ReplacementModeArg, SearchMultiplicityArg, SortArg,
    SpreadsheetArg,
};
use talkbank_clan::commands::freq::{
    FreqSort, FreqSpreadsheetMode, IncludeMultiplicity, MultiWordDisplay,
};
use talkbank_clan::framework::CapitalizationFilter;
use talkbank_clan::framework::{
    MatchOrder, MatchScope, MultiWordMatch, ParenthesisMode, ProsodyMode, ReplacementChoice,
};

pub(crate) fn option_if_not_default<T: PartialEq>(value: T, default: T) -> Option<T> {
    if value == default { None } else { Some(value) }
}

/// Convert the CLI `--capitalization` argument into the domain
/// `CapitalizationFilter` consumed by FREQ and VOCD. `None`
/// (flag absent) maps to `Any`, no filter.
pub(crate) fn capitalization_to_filter(arg: Option<CapitalizationArg>) -> CapitalizationFilter {
    match arg {
        None => CapitalizationFilter::Any,
        Some(CapitalizationArg::Initial) => CapitalizationFilter::InitialUpper,
        Some(CapitalizationArg::Mid) => CapitalizationFilter::MidUpper,
    }
}

/// Convert the CLI `--sort` argument into the domain `FreqSort` consumed by
/// FREQ. `None` (flag absent) maps to `Alphabetical`, CLAN's default order.
pub(crate) fn sort_arg_to_freq_sort(arg: Option<SortArg>) -> FreqSort {
    match arg {
        None | Some(SortArg::Alphabetical) => FreqSort::Alphabetical,
        Some(SortArg::Frequency) => FreqSort::Frequency,
        Some(SortArg::ReverseConcordance) => FreqSort::ReverseConcordance,
    }
}

/// Convert the CLI `--parenthesis-mode` argument into the domain
/// `ParenthesisMode` (CLAN `+r1`/`+r2`/`+r3`). The clap field defaults to
/// `RemoveParens`, so a no-flag invocation already maps to CLAN's `+r1` default.
pub(crate) fn parenthesis_mode_arg_to_mode(arg: ParenthesisModeArg) -> ParenthesisMode {
    match arg {
        ParenthesisModeArg::RemoveParens => ParenthesisMode::RemoveParens,
        ParenthesisModeArg::KeepParens => ParenthesisMode::KeepParens,
        ParenthesisModeArg::RemoveMaterial => ParenthesisMode::RemoveMaterial,
    }
}

/// Convert the CLI `--prosody-mode` argument into the domain `ProsodyMode`
/// (CLAN `+r7`). The clap field defaults to `Strip`, CLAN's default.
pub(crate) fn prosody_mode_arg_to_mode(arg: ProsodyModeArg) -> ProsodyMode {
    match arg {
        ProsodyModeArg::Strip => ProsodyMode::Strip,
        ProsodyModeArg::Keep => ProsodyMode::Keep,
    }
}

/// Convert the CLI `--replacement-mode` argument into the domain
/// `ReplacementChoice` (CLAN `+r5`). The clap field defaults to `Replacement`,
/// so a no-flag invocation maps to CLAN's default (count the replacement).
pub(crate) fn replacement_mode_arg_to_choice(arg: ReplacementModeArg) -> ReplacementChoice {
    match arg {
        ReplacementModeArg::Replacement => ReplacementChoice::Replacement,
        ReplacementModeArg::Original => ReplacementChoice::Original,
    }
}

/// Map the CLI `--spreadsheet` value to the FREQ spreadsheet mode (CLAN
/// `+d2` -> per-word, `+d3` -> types/tokens/TTR only, `+d20` -> one row per
/// (file, speaker, word)). `None` keeps the ordinary stdout path.
pub(crate) fn spreadsheet_arg_to_mode(arg: Option<SpreadsheetArg>) -> Option<FreqSpreadsheetMode> {
    arg.map(|arg| match arg {
        SpreadsheetArg::PerWord => FreqSpreadsheetMode::PerWord,
        SpreadsheetArg::Summary => FreqSpreadsheetMode::TypesTokens,
        SpreadsheetArg::PerSpeakerWord => FreqSpreadsheetMode::PerSpeakerWord,
    })
}

/// Map the CLI `--search-multiplicity` value to the FREQ include multiplicity
/// (CLAN `+c2` -> per-pattern; the default counts a word once).
pub(crate) fn search_multiplicity_to_include(arg: SearchMultiplicityArg) -> IncludeMultiplicity {
    match arg {
        SearchMultiplicityArg::Once => IncludeMultiplicity::Once,
        SearchMultiplicityArg::PerPattern => IncludeMultiplicity::PerPattern,
    }
}

/// Map the CLI `--multiword-display` value to the FREQ multi-word display mode
/// (CLAN `+c7` -> matched words; the default shows the search pattern).
pub(crate) fn multiword_display_to_display(arg: MultiWordDisplayArg) -> MultiWordDisplay {
    match arg {
        MultiWordDisplayArg::Pattern => MultiWordDisplay::Pattern,
        MultiWordDisplayArg::Matched => MultiWordDisplay::MatchedWords,
    }
}

/// Build the framework multi-word match mode from the CLI `--multiword-order`
/// (CLAN `+c3`) and `--multiword-scope` (CLAN `+c4`) values.
pub(crate) fn multiword_args_to_match(
    order: MultiWordOrderArg,
    scope: MultiWordScopeArg,
) -> MultiWordMatch {
    MultiWordMatch {
        order: match order {
            MultiWordOrderArg::Sequence => MatchOrder::Sequence,
            MultiWordOrderArg::Any => MatchOrder::AnyOrder,
        },
        scope: match scope {
            MultiWordScopeArg::Anywhere => MatchScope::Anywhere,
            MultiWordScopeArg::Sole => MatchScope::SoleContent,
        },
    }
}
