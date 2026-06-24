//! Raw typed option payloads for CLAN analysis requests.

use std::path::PathBuf;

use super::AnalysisCommandName;

/// FREQ-specific raw input. See [`AnalysisOptions`].
#[derive(Debug, Clone, Default)]
pub struct FreqOptions {
    /// What FREQ counts: main-tier words (default), structural `%mor` morphemes
    /// (`--mor`), or an arbitrary dependent tier's whitespace tokens (CLAN
    /// `+t%X`, `--tier`). Built at the dispatch boundary, where `--mor` and
    /// `--tier` are validated as mutually exclusive.
    pub count_source: crate::commands::freq::CountSource,
    /// CLAN `+c` / `+c0` / `+c1` capitalization filter. Default
    /// (`Any`) counts every countable word.
    pub capitalization: crate::framework::CapitalizationFilter,
    /// How the per-word entries are ordered: `Alphabetical` (CLAN default),
    /// `Frequency` (CLAN `+o`/`+o0`), or `ReverseConcordance` (CLAN `+o1`).
    pub sort: crate::commands::freq::FreqSort,
    /// CLAN `+d1`: emit alphabetized deduped word list only.
    pub word_list_only: bool,
    /// CLAN `+d4`: emit only per-speaker type/token/TTR summary.
    pub types_tokens_only: bool,
    /// CLAN `+k`: case-sensitive keying.
    pub case_sensitive: bool,
    /// CLAN `+sWORD` / `-sWORD`: per-word include/exclude filter.
    /// Always constructed with
    /// [`crate::framework::WordFilterMode::PerWordEmit`] for FREQ.
    pub word_filter: crate::framework::WordFilter,
    /// CLAN `+d2` / `+d3`: emit an aggregate SpreadsheetML file instead of
    /// stdout text. `None` is the ordinary stdout path.
    pub spreadsheet: Option<crate::commands::freq::FreqSpreadsheetMode>,
    /// CLAN `+bN`: frame size for the Moving-Average TTR. `None` skips MATTR.
    pub frame_size: Option<crate::framework::FrameSize>,
    /// Multi-word `+s` match mode: CLAN `+c3` order and `+c4` scope.
    pub multiword_match: crate::framework::MultiWordMatch,
    /// CLAN `+c2`: count a word once (default) or once per matching `+s` pattern.
    pub include_multiplicity: crate::commands::freq::IncludeMultiplicity,
    /// CLAN `+c7`: display a multi-word match as the pattern (default) or the
    /// actual matched words.
    pub multiword_display: crate::commands::freq::MultiWordDisplay,
    /// CLAN `+d5` (zeroMatch): emit each literal `+s` word even when unmatched,
    /// with count 0. The CLI layer rejects wildcards/duplicates in `+s` and
    /// requires at least one `+s` word before setting this.
    pub include_zero_frequency: bool,
    /// CLAN `+o3` (isCombineSpeakers): pool all speakers into one combined table.
    pub combine_speakers: bool,
    /// CLAN `+r1`/`+r2`/`+r3` (`Parans`): how omitted-material parentheses
    /// (`bein(g)`) render. Default
    /// [`crate::framework::ParenthesisMode::RemoveParens`] = CLAN's `+r1`
    /// default (`bein(g)` -> `being`).
    pub parenthesis_mode: crate::framework::ParenthesisMode,
    /// CLAN `+r7`: whether within-word prosodic `:`/`^`/`~` are kept. Default
    /// [`crate::framework::ProsodyMode::Strip`] (`ca:t` -> `cat`).
    pub prosody_mode: crate::framework::ProsodyMode,
    /// CLAN `+r6`: include retraced material in the counts (default `false`).
    pub include_retracings: bool,
    /// CLAN `+r5`: which word a `[: text]` replacement contributes (default
    /// [`crate::framework::ReplacementChoice::Replacement`]).
    pub replacement_mode: crate::framework::ReplacementChoice,
    /// CLAN `+pS`: extra characters that split a counted word into separate
    /// tokens (default empty, no splitting).
    pub word_delimiters: crate::framework::WordDelimiters,
}

/// MLU-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct MluOptions {
    /// Count words instead of morphemes.
    pub words: bool,
    /// CLAN `+gS`: drop utterances consisting solely of these words.
    pub solo_word_exclusions: Vec<String>,
    /// CLAN `+o3`: pool selected speakers into one `*COMBINED*` MLU result.
    pub combine_speakers: bool,
    /// CLAN `+sxxx`: re-admit `xxx` (unintelligible) utterances to the count.
    pub include_xxx: bool,
    /// CLAN `+syyy`: re-admit `yyy` (phonological) utterances to the count.
    pub include_yyy: bool,
}

/// MLT-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct MltOptions {
    /// CLAN `+gS`: drop utterances consisting solely of these words.
    pub solo_word_exclusions: Vec<String>,
}

/// WDSIZE-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct WdsizeOptions {
    /// Read from the main tier instead of the `%mor` tier.
    pub main_tier: bool,
    /// CLAN `+w[>|<|=]N`: include only words whose character
    /// length satisfies the comparison.
    pub length_filter: Option<crate::commands::wdsize::LengthFilter>,
}

/// MAXWD-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct MaxwdOptions {
    /// Result limit (CLAN: `+cN`). `None` ⇒ apply `MaxwdConfig`
    /// default.
    pub limit: Option<crate::framework::WordLimit>,
    /// CLAN `+a`: restrict to words whose length is unique within
    /// a speaker's lexicon.
    pub unique_length_only: bool,
    /// CLAN `+xN` (repeatable): drop words of length N. Each
    /// `+xN` on the CLI appends one entry.
    pub exclude_lengths: Vec<usize>,
    /// CLAN `+k`: case-sensitive word keying.
    pub case_sensitive: bool,
}

/// KWAL-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct KwalOptions {
    /// Keyword search list.
    pub keywords: Vec<crate::framework::KeywordPattern>,
    /// CLAN `+b`: keyword must be the only countable word on
    /// the tier (single-word utterance match).
    pub strict_match: bool,
    /// CLAN `+k`: case-sensitive keyword matching. Default
    /// (`false`) lowercases both sides before comparison.
    pub case_sensitive: bool,
    /// CLAN `+d` (no N): emit matching utterances as legal CHAT
    /// (drop the location decoration).
    pub legal_chat: bool,
    /// CLAN `-wN`: pre-match context lines.
    pub context_before: u32,
    /// CLAN `+wN`: post-match context lines.
    pub context_after: u32,
}

/// COMBO-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct ComboOptions {
    /// Search expressions (parsed downstream by
    /// `commands::combo::SearchExpr::parse`).
    pub search: Vec<String>,
    /// Exclude search expressions (CLAN: `-sS`).
    pub exclude_search: Vec<String>,
    /// CLAN `+g3`: only the first matching expression per utterance.
    pub first_match_only: bool,
    /// CLAN `+g7`: deduplicate repeated matched words.
    pub dedupe_matches: bool,
    /// CLAN `+k`: case-sensitive matching. When `true`, the
    /// `SearchExpr::parse_with_case` step preserves case in the
    /// stored terms, and `process_utterance` populates words via
    /// `cleaned_text()` instead of `NormalizedWord::from_word`.
    pub case_sensitive: bool,
    /// CLAN `-wN`: pre-match context lines.
    pub context_before: u32,
    /// CLAN `+wN`: post-match context lines.
    pub context_after: u32,
}

/// DIST-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct DistOptions {
    /// CLAN `+g`: count each word at most once per turn.
    pub once_per_turn: bool,
    /// CLAN `+k`: case-sensitive word keying.
    pub case_sensitive: bool,
}

/// COOCCUR-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct CooccurOptions {
    /// CLAN `+d`: render output without the leading count column.
    pub no_frequency_counts: bool,
    /// CLAN `+nN`: cluster size (number of adjacent words per
    /// row). `0` falls back to the `CooccurConfig` default of 2.
    pub cluster_size: u8,
}

/// FREQPOS-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct FreqposOptions {
    /// CLAN `+d`: switch position classification from
    /// first/last/other to first/second/other.
    pub position_classification: crate::commands::freqpos::PositionClassification,
    /// CLAN `+k`: case-sensitive word keying.
    pub case_sensitive: bool,
}

/// VOCD-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct VocdOptions {
    /// CLAN `+c` / `+c0` / `+c1` capitalization filter. Default
    /// (`Any`) feeds every countable word to the D-statistic
    /// sampler.
    pub capitalization: crate::framework::CapitalizationFilter,
    /// CLAN `+k`: case-sensitive token keying.
    pub case_sensitive: bool,
}

/// CODES-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct CodesOptions {
    /// Maximum hierarchical code depth. `None` ⇒ default.
    pub max_depth: Option<crate::framework::CodeDepth>,
}

/// CHAINS-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct ChainsOptions {
    /// Tier to walk (defaults to `CodesConfig` default).
    pub tier: Option<crate::framework::TierKind>,
}

/// CORELEX-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct CorelexOptions {
    /// Minimum frequency for core classification.
    pub threshold: Option<crate::framework::FrequencyThreshold>,
}

/// DSS-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct DssOptions {
    /// Override the bundled DSS rules file.
    pub rules_path: Option<PathBuf>,
    /// Cap on utterances scored.
    pub max_utterances: Option<crate::framework::UtteranceLimit>,
}

/// EVAL-specific raw input (shared by `eval` and `eval-dialect`).
#[derive(Debug, Clone, Default)]
pub struct EvalOptions {
    /// Optional normative database path.
    pub database_path: Option<PathBuf>,
    /// Optional normative database demographic filter.
    pub database_filter: Option<crate::database::DatabaseFilter>,
}

/// FLUCALC-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct FlucalcOptions {
    /// Use syllable counts instead of word counts.
    pub syllable_mode: bool,
}

/// IPSYN-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct IpsynOptions {
    /// Override the bundled IPSyn rules file.
    pub rules_path: Option<PathBuf>,
    /// Cap on utterances scored.
    pub max_utterances: Option<crate::framework::UtteranceLimit>,
}

/// KEYMAP-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct KeymapOptions {
    /// Keyword search list.
    pub keywords: Vec<crate::framework::KeywordPattern>,
    /// Tier to scan (defaults to `KeymapConfig::default().tier`).
    pub tier: Option<crate::framework::TierKind>,
}

/// KIDEVAL-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct KidevalOptions {
    /// Override the bundled DSS rules file.
    pub dss_rules_path: Option<PathBuf>,
    /// Override the bundled IPSyn rules file.
    pub ipsyn_rules_path: Option<PathBuf>,
    /// Cap on utterances scored by the embedded DSS sub-analysis.
    pub dss_max_utterances: Option<crate::framework::UtteranceLimit>,
    /// Cap on utterances scored by the embedded IPSyn sub-analysis.
    pub ipsyn_max_utterances: Option<crate::framework::UtteranceLimit>,
    /// Optional normative database path.
    pub database_path: Option<PathBuf>,
    /// Optional normative database demographic filter.
    pub database_filter: Option<crate::database::DatabaseFilter>,
}

/// MORTABLE-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct MortableOptions {
    /// Path to the language-script `.cut` file (required).
    pub script_path: Option<PathBuf>,
}

/// RELY-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct RelyOptions {
    /// Path to the comparison file (required).
    pub second_file: Option<PathBuf>,
    /// Tier to align (defaults to `RelyConfig::default().tier`).
    pub tier: Option<crate::framework::TierKind>,
}

/// SCRIPT-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct ScriptOptions {
    /// Path to the template file (required).
    pub template_path: Option<PathBuf>,
}

/// SUGAR-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct SugarOptions {
    /// Minimum utterance count threshold.
    pub min_utterances: Option<crate::framework::UtteranceLimit>,
}

/// TRNFIX-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct TrnfixOptions {
    /// First tier of the swap.
    pub tier1: Option<crate::framework::TierKind>,
    /// Second tier of the swap.
    pub tier2: Option<crate::framework::TierKind>,
}

/// UNIQ-specific raw input.
#[derive(Debug, Clone, Default)]
pub struct UniqOptions {
    /// Sort by descending frequency instead of alphabetical order.
    pub sort_by_frequency: bool,
}

/// Raw analysis options supplied by outer adapters before defaults
/// are applied. Variant carries the per-command `*Options` for
/// commands that take input, or is unit for commands that don't.
///
/// The variant doubles as the command discriminator: the builder
/// no longer needs a separate [`AnalysisCommandName`] parameter
/// because [`Self::command_name`] derives it from the variant.
///
/// Note: `Eval` and `EvalDialect` share the same `EvalOptions`
/// shape but are distinct variants so the dispatcher can pick the
/// right `EvalVariant` downstream.
#[derive(Debug, Clone)]
pub enum AnalysisOptions {
    /// FREQ.
    Freq(FreqOptions),
    /// MLU.
    Mlu(MluOptions),
    /// MLT.
    Mlt(MltOptions),
    /// WDLEN, no input options.
    Wdlen,
    /// WDSIZE.
    Wdsize(WdsizeOptions),
    /// MAXWD.
    Maxwd(MaxwdOptions),
    /// FREQPOS.
    Freqpos(FreqposOptions),
    /// TIMEDUR, no input options.
    Timedur,
    /// KWAL.
    Kwal(KwalOptions),
    /// GEMLIST, no input options.
    Gemlist,
    /// COMBO.
    Combo(ComboOptions),
    /// COOCCUR.
    Cooccur(CooccurOptions),
    /// DIST.
    Dist(DistOptions),
    /// CHIP, no input options.
    Chip,
    /// PHONFREQ, no input options.
    Phonfreq,
    /// MODREP, no input options.
    Modrep,
    /// VOCD.
    Vocd(VocdOptions),
    /// CODES.
    Codes(CodesOptions),
    /// CHAINS.
    Chains(ChainsOptions),
    /// COMPLEXITY, no input options.
    Complexity,
    /// CORELEX.
    Corelex(CorelexOptions),
    /// DSS.
    Dss(DssOptions),
    /// EVAL.
    Eval(EvalOptions),
    /// EVAL-DIALECT (shares `EvalOptions` shape with `Eval`).
    EvalDialect(EvalOptions),
    /// FLUCALC.
    Flucalc(FlucalcOptions),
    /// IPSYN.
    Ipsyn(IpsynOptions),
    /// KEYMAP.
    Keymap(KeymapOptions),
    /// KIDEVAL.
    Kideval(KidevalOptions),
    /// MORTABLE.
    Mortable(MortableOptions),
    /// RELY.
    Rely(RelyOptions),
    /// SCRIPT.
    Script(ScriptOptions),
    /// SUGAR.
    Sugar(SugarOptions),
    /// TRNFIX.
    Trnfix(TrnfixOptions),
    /// UNIQ.
    Uniq(UniqOptions),
}

impl AnalysisOptions {
    /// Derive the command-identity tag from the variant. Used by
    /// callers (banner rendering, scope determination) that need a
    /// stable name string independent of the option payload.
    pub fn command_name(&self) -> AnalysisCommandName {
        match self {
            AnalysisOptions::Freq(_) => AnalysisCommandName::Freq,
            AnalysisOptions::Mlu(_) => AnalysisCommandName::Mlu,
            AnalysisOptions::Mlt(_) => AnalysisCommandName::Mlt,
            AnalysisOptions::Wdlen => AnalysisCommandName::Wdlen,
            AnalysisOptions::Wdsize(_) => AnalysisCommandName::Wdsize,
            AnalysisOptions::Maxwd(_) => AnalysisCommandName::Maxwd,
            AnalysisOptions::Freqpos(_) => AnalysisCommandName::Freqpos,
            AnalysisOptions::Timedur => AnalysisCommandName::Timedur,
            AnalysisOptions::Kwal(_) => AnalysisCommandName::Kwal,
            AnalysisOptions::Gemlist => AnalysisCommandName::Gemlist,
            AnalysisOptions::Combo(_) => AnalysisCommandName::Combo,
            AnalysisOptions::Cooccur(_) => AnalysisCommandName::Cooccur,
            AnalysisOptions::Dist(_) => AnalysisCommandName::Dist,
            AnalysisOptions::Chip => AnalysisCommandName::Chip,
            AnalysisOptions::Phonfreq => AnalysisCommandName::Phonfreq,
            AnalysisOptions::Modrep => AnalysisCommandName::Modrep,
            AnalysisOptions::Vocd(_) => AnalysisCommandName::Vocd,
            AnalysisOptions::Codes(_) => AnalysisCommandName::Codes,
            AnalysisOptions::Chains(_) => AnalysisCommandName::Chains,
            AnalysisOptions::Complexity => AnalysisCommandName::Complexity,
            AnalysisOptions::Corelex(_) => AnalysisCommandName::Corelex,
            AnalysisOptions::Dss(_) => AnalysisCommandName::Dss,
            AnalysisOptions::Eval(_) => AnalysisCommandName::Eval,
            AnalysisOptions::EvalDialect(_) => AnalysisCommandName::EvalDialect,
            AnalysisOptions::Flucalc(_) => AnalysisCommandName::Flucalc,
            AnalysisOptions::Ipsyn(_) => AnalysisCommandName::Ipsyn,
            AnalysisOptions::Keymap(_) => AnalysisCommandName::Keymap,
            AnalysisOptions::Kideval(_) => AnalysisCommandName::Kideval,
            AnalysisOptions::Mortable(_) => AnalysisCommandName::Mortable,
            AnalysisOptions::Rely(_) => AnalysisCommandName::Rely,
            AnalysisOptions::Script(_) => AnalysisCommandName::Script,
            AnalysisOptions::Sugar(_) => AnalysisCommandName::Sugar,
            AnalysisOptions::Trnfix(_) => AnalysisCommandName::Trnfix,
            AnalysisOptions::Uniq(_) => AnalysisCommandName::Uniq,
        }
    }
}
