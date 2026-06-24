//! FREQ, Word frequency analysis.
//!
//! Reimplements CLAN's FREQ command, which counts word tokens and types
//! on the main tier and/or `%mor` tier, computing type-token ratio (TTR).
//! FREQ is the most commonly used CLAN command and serves as the foundation
//! for lexical diversity analysis in child language research.
//!
//! Word normalization uses [`NormalizedWord`], which lowercases and strips
//! compound markers (`+`) for grouping, while preserving the original
//! CLAN display form (with `+`) for output.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409093)
//! for the original FREQ command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command              | Rust equivalent                        |
//! |---------------------------|----------------------------------------|
//! | `freq file.cha`           | `chatter analyze freq file.cha`        |
//! | `freq +t*CHI file.cha`    | `chatter analyze freq file.cha -s CHI` |
//!
//! # Output
//!
//! Per-speaker frequency tables with:
//! - Word frequency counts (sorted by count descending, then alphabetically)
//! - Total types (unique words) and tokens (total words)
//! - TTR (type-token ratio = types / tokens)
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

use std::collections::HashMap;

mod counting;
mod output;
mod spreadsheet;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use indexmap::IndexMap;
use talkbank_model::{IDHeader, SpeakerCode, Utterance};

use crate::framework::word_filter::CapitalizationFilter;
use crate::framework::{
    AnalysisCommand, FileContext, FrameSize, MultiWordGroup, MultiWordMatch, NormalizedWord,
    ParenthesisMode, ProsodyMode, ReplacementChoice, TierKind, WordCount, moving_average_ttr,
};

pub use output::{FreqEntry, FreqFileSpeakerRow, FreqResult, FreqSort, FreqSpeakerResult};
pub use spreadsheet::{
    FreqSpreadsheetMode, SpeakerPercent, SpeakerPercentComparison, SpeakerPercentFilter,
};

/// CLAN `+c2`: how many times a word counts when it matches the `+s` search.
///
/// The default counts a word once no matter how many `+s` patterns it matches;
/// `+c2` (`capwd == 3`, freq.cpp:432-438) counts it once per matching pattern.
/// This is a SINGLE-word `+s` concern, distinct from the multi-word match modes
/// (`+c3`/`+c4`), and CLAN rejects `+c2` combined with a multi-word `+s` group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IncludeMultiplicity {
    /// CLAN default: a word counts once if it matches any `+s` pattern.
    #[default]
    Once,
    /// CLAN `+c2`: a word counts once per matching `+s` pattern.
    PerPattern,
}

/// CLAN `+c7`: how a multi-word `+s` match is displayed as a frequency item.
///
/// The default keys every match by the search pattern (e.g. `the *`), so they
/// collapse into one entry; `+c7` (`isMultiWordsActual`, freq.cpp:2444) keys
/// each match by the words that actually matched (e.g. `the hill`, `the top`),
/// so the wildcard slot reveals what occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MultiWordDisplay {
    /// CLAN default: the item is the search pattern (one entry per group).
    #[default]
    Pattern,
    /// CLAN `+c7`: the item is the actual matched words (one entry per distinct
    /// matched word sequence).
    MatchedWords,
}

/// What FREQ counts: main-tier words (default), chatter's structural `%mor`
/// morphemes (`--mor`), or the whitespace-delimited tokens of an arbitrary
/// dependent tier (CLAN `+t%X`).
///
/// CLAN's `+t%X` (`freq.cpp:914-938`) sets `nomain=TRUE` and counts the
/// whitespace-delimited tokens of the named dependent tier's raw line. "What to
/// count" is a SINGLE axis with three mutually-exclusive values, so it is an
/// enum rather than a `use_mor: bool` plus a separate tier field, which would
/// admit the nonsensical "structural-mor AND dependent-tier-gra" state (the
/// no-boolean-blindness / no-invalid-states rule).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CountSource {
    /// Default: count countable words on the main tier.
    #[default]
    MainTier,
    /// chatter `--mor`: count `%mor` morphemes structurally, where each
    /// `MorWord` (its main item and each post-clitic) is a separate frequency
    /// item. A chatter-only convenience flag, NOT a CLAN slot; CLAN's `+t%mor`
    /// is `DependentTierTokens(TierKind::Mor)` instead, which whitespace-splits
    /// the raw line (so a clitic `v|go~aux|be` is ONE token, not two).
    MorStructural,
    /// CLAN `+t%X` (`freq.cpp:914-938`): count the whitespace-delimited tokens of
    /// the named dependent tier's raw line, dropping CLAN-excluded tokens (the
    /// bare `.` terminator, etc.). The faithful partner of the CLAN slot; see
    /// [`crate::framework::dependent_tier_tokens`].
    DependentTierTokens(TierKind),
    /// CLAN `-t%X` (the EXCLUDE form, `freq.cpp` `case 't'` tier-selection): count
    /// the main tier PLUS every present dependent tier EXCEPT the listed kinds,
    /// pooled into one table (banner: "ALL speaker tiers / and those speakers'
    /// ALL dependent tiers EXCEPT ..."). A composition of [`CountSource::MainTier`]
    /// and [`CountSource::DependentTierTokens`] over all present tier kinds. The
    /// "%mor line forms" TTR advisory STAYS ON in this mode (CLAN sets `isMorUsed`
    /// only for the explicit `+t%mor` include), so [`Self::is_mor_based`] is
    /// `false` here even when `%mor` is among the counted tiers.
    MainPlusDependentTiersExcept(Vec<TierKind>),
}

impl CountSource {
    /// Whether counting is `%mor`-based (CLAN `isMorUsed`, set at `freq.cpp:605`
    /// and `freq.cpp:924`). True for both chatter's structural `--mor` and the
    /// CLAN `+t%mor` slot. CLAN gates the "%mor line forms" TTR advisory on
    /// `!isMorUsed` (`freq.cpp:1536`), so a `%mor`-based count suppresses that
    /// advisory while main-tier and non-`%mor` dependent tiers (e.g. `+t%gra`)
    /// keep it.
    pub fn is_mor_based(&self) -> bool {
        matches!(
            self,
            CountSource::MorStructural | CountSource::DependentTierTokens(TierKind::Mor)
        )
    }
}

/// Configuration for the FREQ command.
#[derive(Debug, Clone)]
pub struct FreqConfig {
    /// What FREQ counts: main-tier words (default), structural `%mor` morphemes
    /// (`--mor`), or an arbitrary dependent tier's whitespace tokens (CLAN
    /// `+t%X`). See [`CountSource`].
    pub count_source: CountSource,
    /// CLAN's `+c` / `+c0` / `+c1`: restrict counting to words
    /// whose surface form matches a capitalization predicate.
    /// `Any` (default) counts every countable word.
    pub capitalization: CapitalizationFilter,
    /// How the per-word entries are ordered: `Alphabetical` (CLAN default),
    /// `Frequency` (CLAN `+o`/`+o0`, descending count, ties alphabetical), or
    /// `ReverseConcordance` (CLAN `+o1`, reversed display form so shared
    /// suffixes cluster).
    pub sort: FreqSort,
    /// CLAN `+d1`: emit only an alphabetized deduped word list,
    /// one word per line, with no banners, counts, or totals.
    /// Intended as fodder for `kwal +s@FILE`.
    pub word_list_only: bool,
    /// CLAN `+d4`: emit only per-speaker type/token/TTR summary,
    /// dropping per-word frequency entries.
    pub types_tokens_only: bool,
    /// Frequency-table case keying. `true` preserves each word's original
    /// case so `Want`/`want`/`WANT` are three distinct entries; `false` folds
    /// to lowercase (via `NormalizedWord`), collapsing them to one.
    ///
    /// CLAN FREQ is in `mmaininit`'s `nomap=TRUE` set (cutt.cpp:7845), so it
    /// PRESERVES case by default and `+k` TOGGLES it to folding
    /// (cutt.cpp:13816). chatter's shared `+k` flag carries "+k present", so
    /// callers set this field to `!(+k present)`: preserve by default, fold
    /// under `+k`. `Default` is therefore `true` to match CLAN's FREQ default.
    /// This is the inverse of the fold-by-default commands
    /// (KWAL/COMBO/FREQPOS/DIST/MAXWD); see the `+k` case-polarity
    /// investigation.
    pub case_sensitive: bool,
    /// CLAN `+sWORD` / `-sWORD`: per-word include/exclude filter.
    /// FREQ applies this at per-word emit (not at the utterance
    /// gate), so utterances with no matching words still appear
    /// (with 0 counts) and non-matching words inside matching
    /// utterances are not counted. Single source of truth: the
    /// framework's `FilterConfig.words` must NOT also carry these
    /// patterns for FREQ. Always constructed with
    /// [`crate::framework::WordFilterMode::PerWordEmit`].
    pub word_filter: crate::framework::WordFilter,
    /// CLAN `+d2` / `+d3`: emit an aggregate SpreadsheetML file (one row per
    /// file x speaker) instead of stdout text. `None` (the default) is the
    /// ordinary stdout path; `Some(_)` additionally accumulates the
    /// per-(file, speaker) data the spreadsheet needs.
    pub spreadsheet: Option<FreqSpreadsheetMode>,
    /// CLAN `+bN`: compute the Moving-Average Type-Token Ratio over a
    /// sliding window of `N` tokens. `None` (the default) skips MATTR entirely;
    /// `Some(_)` additionally accumulates the per-speaker ordered token stream
    /// the windowed average needs.
    pub frame_size: Option<FrameSize>,
    /// How multi-word `+s` groups are matched: CLAN `+c3` order and `+c4` scope.
    /// Default is adjacent in-order, anywhere in the utterance.
    pub multiword_match: MultiWordMatch,
    /// CLAN `+c2`: whether a word counts once (default) or once per matching
    /// `+s` pattern. Applies to single-word `+s` search only.
    pub include_multiplicity: IncludeMultiplicity,
    /// CLAN `+c7`: whether a multi-word `+s` match is displayed as the search
    /// pattern (default) or the actual matched words.
    pub multiword_display: MultiWordDisplay,
    /// CLAN `+d5` (zeroMatch, freq.cpp:894): when `true`, every LITERAL `+s`
    /// search word is shown even when it never matched, with count 0
    /// (freq.cpp:1473-1491 injects via `freq_tree_add_zeros`). The injected
    /// zero word is displayed but excluded from types/tokens/TTR. CLAN rejects
    /// wildcards/duplicates in `+s` under `+d5` (freq.cpp:444) and requires at
    /// least one `+s` word (freq.cpp:449); those guards live at the CLI layer.
    pub include_zero_frequency: bool,
    /// CLAN `+o3` (isCombineSpeakers, freq.cpp:832): pool every speaker's counts
    /// into a SINGLE frequency table with no per-speaker `Speaker:` header,
    /// summed counts, and combined types/tokens/TTR. Default `false` keeps the
    /// per-speaker layout.
    pub combine_speakers: bool,
    /// CLAN `+r1`/`+r2`/`+r3` (`Parans`, `cutt.cpp:9530-9583`): how omitted-
    /// material parentheses (`bein(g)`) render for both the grouping key and the
    /// display. Default [`ParenthesisMode::RemoveParens`] = CLAN's `+r1` default
    /// (`bein(g)` -> `being`).
    pub parenthesis_mode: ParenthesisMode,
    /// CLAN `+r7` (`R7…`, `cutt.cpp:9569-9574`): whether within-word prosodic
    /// symbols `:`/`^`/`~` are kept in the counted word form. Default
    /// [`ProsodyMode::Strip`] = CLAN's default (`ca:t`/`hm:` -> `cat`/`hm`).
    pub prosody_mode: ProsodyMode,
    /// CLAN `+r6` (`R6`, `cutt.cpp:9554`): include retraced material (`[/]`,
    /// `[//]`, `[///]`, `[/-]`) in the counts. Default `false` excludes it
    /// (CLAN's FREQ default). When `true`, a retracing's retraced word is counted
    /// in addition to the correction.
    pub include_retracings: bool,
    /// CLAN `+r5` (`R5`, `cutt.cpp:9549-9553`): which word a `[: text]`
    /// replacement contributes. Default [`ReplacementChoice::Replacement`]
    /// counts the correction (`gots [: got]` -> `got`); `Original` counts the
    /// replaced surface form (`gots`).
    pub replacement_mode: ReplacementChoice,
    /// CLAN `+pS` (`cutt.cpp:9798-9818`): extra characters that split a counted
    /// word into separate tokens (`+p_` breaks `choo_choo` into two `choo`).
    /// Empty by default (no splitting). Applied to the main-tier word form after
    /// the `+r` word-form treatment; a trailing word-form marker (`@o`) stays on
    /// the final segment.
    pub word_delimiters: crate::framework::WordDelimiters,
}

impl Default for FreqConfig {
    /// CLAN FREQ's default is case-sensitive keying (`nomap=TRUE`,
    /// cutt.cpp:7845), so `case_sensitive` defaults to `true`, unlike the
    /// fold-by-default commands. All other fields take their natural default.
    fn default() -> Self {
        Self {
            count_source: CountSource::MainTier,
            capitalization: CapitalizationFilter::default(),
            sort: FreqSort::Alphabetical,
            word_list_only: false,
            types_tokens_only: false,
            case_sensitive: true,
            word_filter: crate::framework::WordFilter::default(),
            spreadsheet: None,
            frame_size: None,
            multiword_match: MultiWordMatch::default(),
            include_multiplicity: IncludeMultiplicity::Once,
            multiword_display: MultiWordDisplay::Pattern,
            include_zero_frequency: false,
            combine_speakers: false,
            parenthesis_mode: ParenthesisMode::default(),
            prosody_mode: ProsodyMode::default(),
            include_retracings: false,
            replacement_mode: ReplacementChoice::default(),
            word_delimiters: crate::framework::WordDelimiters::default(),
        }
    }
}

/// Per-speaker frequency data accumulated during processing.
#[derive(Debug, Default)]
struct SpeakerFreq {
    /// Normalized word → count mapping
    counts: HashMap<NormalizedWord, WordCount>,
    /// Normalized word → CLAN display form (preserves `+` in compounds)
    display_forms: HashMap<NormalizedWord, String>,
    /// Total tokens (sum of all counts)
    total_tokens: WordCount,
    /// The token stream in document order, populated ONLY when a frame size
    /// (`+bN` MATTR) is configured. MATTR depends on token *order* (sliding
    /// windows), which the unordered `counts` map cannot reconstruct; the
    /// non-MATTR path leaves this empty and pays no per-token allocation.
    ordered_tokens: Vec<NormalizedWord>,
}

impl SpeakerFreq {
    /// Record `count` occurrences of `key`: bump its frequency and the running
    /// token total, and (only under `+bN` MATTR, `collect_order`) append it to
    /// the ordered stream that many times. The single home for the
    /// order-sensitive token bump shared by the `%mor`, per-word, and multi-word
    /// paths. Callers that need a CLAN display form set `display_forms`
    /// separately, since the `%mor` path keys by the raw CHAT form and has none.
    fn record(&mut self, key: NormalizedWord, count: WordCount, collect_order: bool) {
        if collect_order {
            for _ in 0..count {
                self.ordered_tokens.push(key.clone());
            }
        }
        *self.counts.entry(key).or_insert(0) += count;
        self.total_tokens += count;
    }

    /// Record a frequency item identified by a flat `display` string that is
    /// both the surface to key on and the stored display form: derive the
    /// `NormalizedWord` key, set the display form on first sight, and bump the
    /// count. Used where the counted token is already a plain string rather than
    /// an AST word, the `+pS` word-delimiter segments and the multi-word `+s`
    /// matches (the default per-word path keys via `parans_normalized_key`
    /// instead, so it does not go through here).
    fn record_with_display(
        &mut self,
        display: &str,
        case_sensitive: bool,
        count: WordCount,
        collect_order: bool,
    ) {
        let key = NormalizedWord::from_text_cased(display, case_sensitive);
        self.display_forms
            .entry(key.clone())
            .or_insert_with(|| display.to_owned());
        self.record(key, count, collect_order);
    }
}

/// A CHAT file's display stem (basename without extension): the spreadsheet's
/// `File` column and half of the per-(file, speaker) key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FileStem(String);

/// Per-(file, speaker) accumulation for the `+d2`/`+d3` spreadsheet. Reuses the
/// cross-file [`SpeakerFreq`] counting, plus the speaker's utterance count
/// (CLAN's `*SPEAKER:` pseudo-word column) and the speaker's `@ID` for the file.
#[derive(Debug, Default)]
struct FileSpeakerFreq {
    /// Word counts for this (file, speaker), counted identically to the
    /// cross-file path.
    freq: SpeakerFreq,
    /// Number of this speaker's utterances in this file.
    utterance_count: u64,
    /// The speaker's `@ID` header for this file, captured in `end_file`.
    id: Option<IDHeader>,
}

/// Accumulated state for FREQ across all files.
#[derive(Debug, Default)]
pub struct FreqState {
    /// Per-speaker frequency data, keyed by speaker code, combined across all
    /// files (feeds the text/JSON/CSV/CLAN stdout formats).
    by_speaker: IndexMap<SpeakerCode, SpeakerFreq>,
    /// Per-(file, speaker) frequency data for the `+d2`/`+d3` spreadsheet, in
    /// file-then-speaker encounter order. Empty unless a spreadsheet mode is
    /// active (the accumulation is gated in `process_utterance`).
    by_file_speaker: IndexMap<(FileStem, SpeakerCode), FileSpeakerFreq>,
}

/// A literal `+s` search word to display at count 0 when it never matched, for
/// CLAN `+d5` (zeroMatch). `display` is the verbatim string CLAN injects
/// (`twd->word`, freq.cpp:1480); `key` is its normalized form, tested against
/// each speaker's real counts to decide whether to inject (CLAN's
/// `freq_tree_add_zeros` adds only when absent, freq.cpp:1259).
#[derive(Debug, Clone)]
struct ZeroFrequencyWord {
    display: String,
    key: NormalizedWord,
}

/// FREQ command implementation.
///
/// Counts word frequencies on the main tier, producing per-speaker
/// frequency tables with TTR.
#[derive(Debug, Clone, Default)]
pub struct FreqCommand {
    config: FreqConfig,
    /// The multi-word `+s` groups (CLAN `+s"a b"`), parsed ONCE from the include
    /// patterns. Empty for the common single-word-only `+s` case, so the
    /// per-utterance group pass and its token collection are skipped entirely
    /// there; in the multi-word case the groups are not re-parsed per utterance.
    multiword_groups: Vec<MultiWordGroup>,
    /// Literal `+s` words shown at count 0 when a speaker never matched them
    /// (CLAN `+d5`, zeroMatch). Empty unless `+d5` is set; built once in `new()`
    /// from the include patterns, deduped by normalized key (CLAN rejects
    /// duplicates under `+d5`, so the dedup is defensive).
    zero_frequency_words: Vec<ZeroFrequencyWord>,
}

impl FreqCommand {
    /// Create a FREQ command with the given configuration, parsing any
    /// multi-word `+s` groups from the include patterns up front.
    pub fn new(config: FreqConfig) -> Self {
        let multiword_groups = config
            .word_filter
            .include
            .iter()
            .filter_map(|p| MultiWordGroup::parse(p.as_str(), config.word_filter.case_sensitive))
            .collect();
        // CLAN `+d5` zero-injection candidates: only built when `+d5` is set, so
        // the common path pays nothing.
        let zero_frequency_words = if config.include_zero_frequency {
            build_zero_frequency_words(&config.word_filter)
        } else {
            Vec::new()
        };
        Self {
            config,
            multiword_groups,
            zero_frequency_words,
        }
    }
}

/// Merge every speaker's counts into a single [`SpeakerFreq`] for CLAN `+o3`
/// (isCombineSpeakers): sum each word's count across speakers, take the first
/// display form seen for each key, sum the token totals, and concatenate the
/// ordered token streams (in speaker-encounter order, for any combined `+bN`
/// MATTR). The pooled result renders as one headerless table.
fn merge_speakers(by_speaker: &IndexMap<SpeakerCode, SpeakerFreq>) -> SpeakerFreq {
    let mut combined = SpeakerFreq::default();
    for freq in by_speaker.values() {
        for (key, count) in &freq.counts {
            *combined.counts.entry(key.clone()).or_insert(0) += *count;
        }
        for (key, display) in &freq.display_forms {
            combined
                .display_forms
                .entry(key.clone())
                .or_insert_with(|| display.clone());
        }
        combined.total_tokens += freq.total_tokens;
        combined
            .ordered_tokens
            .extend(freq.ordered_tokens.iter().cloned());
    }
    combined
}

/// Build the CLAN `+d5` zero-injection list from the `+s` include patterns: each
/// literal search word (single token verbatim, or a multi-word group under its
/// joined display) paired with its normalized key. Deduped by key so a repeated
/// pattern never double-injects (CLAN rejects duplicates under `+d5`).
fn build_zero_frequency_words(
    word_filter: &crate::framework::WordFilter,
) -> Vec<ZeroFrequencyWord> {
    // Single source of truth for case treatment: the filter's own flag, matching
    // the sibling `multiword_groups` construction in `new()`.
    let case_sensitive = word_filter.case_sensitive;
    let mut out: Vec<ZeroFrequencyWord> = Vec::new();
    for pattern in &word_filter.include {
        let display = match MultiWordGroup::parse(pattern.as_str(), case_sensitive) {
            Some(group) => group.display().to_owned(),
            None => pattern.as_str().to_owned(),
        };
        let key = NormalizedWord::from_text_cased(&display, case_sensitive);
        if !out.iter().any(|z| z.key == key) {
            out.push(ZeroFrequencyWord { display, key });
        }
    }
    out
}

impl AnalysisCommand for FreqCommand {
    type Config = FreqConfig;
    type State = FreqState;
    type Output = FreqResult;

    /// Accumulate per-speaker token counts from main-tier words or `%mor` items.
    ///
    /// The cross-file `by_speaker` table always receives the counts; when a
    /// spreadsheet mode (`+d2`/`+d3`) is active, the same utterance is
    /// additionally counted into the per-(file, speaker) table and that
    /// speaker's utterance count is bumped.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // Arc<str> clone, cheap atomic ref-count increment, no allocation
        let speaker = utterance.main.speaker.clone();
        let cross_file = state.by_speaker.entry(speaker.clone()).or_default();
        self.count_utterance(utterance, cross_file);

        if self.config.spreadsheet.is_some() {
            let key = (FileStem(file_context.filename.to_owned()), speaker);
            let entry = state.by_file_speaker.entry(key).or_default();
            self.count_utterance(utterance, &mut entry.freq);
            entry.utterance_count += 1;
        }
    }

    /// After a file is processed, capture each analyzed speaker's `@ID` header
    /// for that file (for the spreadsheet's `@ID` columns). No-op outside
    /// spreadsheet mode.
    fn end_file(&self, file_context: &FileContext<'_>, state: &mut Self::State) {
        if self.config.spreadsheet.is_none() {
            return;
        }
        let stem = FileStem(file_context.filename.to_owned());
        // First `@ID:` per speaker code wins, matching the runner's id map.
        let mut id_by_speaker: HashMap<SpeakerCode, &IDHeader> = HashMap::new();
        for id in file_context.chat_file.id_headers() {
            id_by_speaker.entry(id.speaker.clone()).or_insert(id);
        }
        for ((file_stem, speaker), entry) in state.by_file_speaker.iter_mut() {
            if *file_stem == stem
                && entry.id.is_none()
                && let Some(id) = id_by_speaker.get(speaker)
            {
                entry.id = Some((*id).clone());
            }
        }
    }

    /// Convert accumulated counts into sorted per-speaker frequency tables.
    fn finalize(&self, state: Self::State) -> FreqResult {
        let mut speakers = Vec::new();

        // CLAN `+o3` (isCombineSpeakers): pool every speaker into one table with
        // no per-speaker header (freq.cpp:832). The merged accumulator must
        // outlive the loop, so build it here and borrow it; the non-combine path
        // borrows `state.by_speaker` directly. The combined label is empty
        // because `render_clan` suppresses the `Speaker:` header in that mode, so
        // the label is never displayed.
        let combined;
        let by_speaker: Vec<(&str, &SpeakerFreq)> = if self.config.combine_speakers {
            combined = merge_speakers(&state.by_speaker);
            vec![("", &combined)]
        } else {
            state
                .by_speaker
                .iter()
                .map(|(speaker, freq)| (speaker.as_str(), freq))
                .collect()
        };

        for (speaker, freq) in by_speaker {
            // This accumulator order feeds chatter's own text/JSON formats;
            // `render_clan` re-derives the CLAN display order from `sort`.
            // `ReverseConcordance` (`+o1`) groups shared suffixes via a
            // Schwartzian transform (reversed key built once per word);
            // the other modes use count-descending with an alphabetical
            // tiebreak.
            let raw_entries: Vec<(&NormalizedWord, &u64)> = match self.config.sort {
                FreqSort::ReverseConcordance => {
                    let mut keyed: Vec<(String, &NormalizedWord, &u64)> = freq
                        .counts
                        .iter()
                        .map(|(w, c)| (w.as_str().chars().rev().collect::<String>(), w, c))
                        .collect();
                    keyed.sort_by(|a, b| a.0.cmp(&b.0));
                    keyed.into_iter().map(|(_, w, c)| (w, c)).collect()
                }
                FreqSort::Frequency | FreqSort::Alphabetical => {
                    let mut entries: Vec<(&NormalizedWord, &u64)> = freq.counts.iter().collect();
                    entries.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
                    entries
                }
            };

            let total_types = freq.counts.len() as u64;
            let total_tokens = freq.total_tokens;
            let ttr = type_token_ratio(total_types, total_tokens);
            // CLAN `+bN`: the Moving-Average TTR over this speaker's ordered
            // token stream. `None` when `+bN` is off, or when the speaker has
            // fewer than `N` tokens (CLAN renders `-` then).
            let mattr = self
                .config
                .frame_size
                .and_then(|frame| moving_average_ttr(&freq.ordered_tokens, frame));

            let mut entries: Vec<FreqEntry> = raw_entries
                .iter()
                .map(|(word, count)| {
                    let display = freq.display_forms.get(*word).cloned();
                    FreqEntry {
                        word: word.as_str().to_owned(),
                        display_form: display,
                        count: **count,
                    }
                })
                .collect();

            // CLAN `+d5` (zeroMatch): append each literal `+s` word this speaker
            // never matched, at count 0 (freq.cpp:1473-1491). Injected into the
            // DISPLAY only, AFTER `total_types`/`total_tokens` were taken from the
            // real matches, so the zero word shows but is excluded from
            // types/tokens/TTR (verified against CLAN: `0 zzz` with Types/Tokens
            // still 1). `render_clan` re-sorts, placing it in alphabetical order.
            for zero in &self.zero_frequency_words {
                if !freq.counts.contains_key(&zero.key) {
                    entries.push(FreqEntry {
                        word: zero.key.as_str().to_owned(),
                        display_form: Some(zero.display.clone()),
                        count: 0,
                    });
                }
            }

            speakers.push(FreqSpeakerResult {
                speaker: speaker.to_owned(),
                entries,
                total_types,
                total_tokens,
                ttr,
                mattr,
            });
        }

        let file_speaker_rows =
            build_file_speaker_rows(state.by_file_speaker, self.config.frame_size);

        FreqResult {
            speakers,
            word_list_only: self.config.word_list_only,
            types_tokens_only: self.config.types_tokens_only,
            sort: self.config.sort,
            file_speaker_rows,
            mattr_enabled: self.config.frame_size.is_some(),
            combine_speakers: self.config.combine_speakers,
            mor_based: self.config.count_source.is_mor_based(),
        }
    }
}

/// Type-token ratio (distinct types / total tokens), 0.0 when there are no
/// tokens. Shared by the cross-file and per-(file, speaker) result builders.
fn type_token_ratio(total_types: u64, total_tokens: u64) -> f64 {
    if total_tokens > 0 {
        total_types as f64 / total_tokens as f64
    } else {
        0.0
    }
}

/// Convert the per-(file, speaker) accumulation into the spreadsheet's typed
/// rows, one per (file x speaker) in encounter order. Empty when no spreadsheet
/// mode was active (the map is never populated).
fn build_file_speaker_rows(
    by_file_speaker: IndexMap<(FileStem, SpeakerCode), FileSpeakerFreq>,
    frame_size: Option<FrameSize>,
) -> Vec<FreqFileSpeakerRow> {
    by_file_speaker
        .into_iter()
        .map(|((stem, speaker), entry)| {
            let total_types = entry.freq.counts.len() as u64;
            let total_tokens = entry.freq.total_tokens;
            let ttr = type_token_ratio(total_types, total_tokens);
            // CLAN `+bN`: this (file, speaker)'s MATTR, appended as the trailing
            // spreadsheet column. `None` when `+bN` is off or `T < N`.
            let mattr =
                frame_size.and_then(|frame| moving_average_ttr(&entry.freq.ordered_tokens, frame));
            // Key the word columns by CLAN display form (the column header);
            // sum on the rare display-form collision so no count is lost.
            let mut word_counts: BTreeMap<String, WordCount> = BTreeMap::new();
            for (key, count) in &entry.freq.counts {
                let display = entry
                    .freq
                    .display_forms
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| key.as_str().to_owned());
                *word_counts.entry(display).or_insert(0) += *count;
            }
            FreqFileSpeakerRow {
                filename: stem.0,
                speaker: speaker.as_str().to_owned(),
                id: entry.id,
                word_counts,
                utterance_count: entry.utterance_count,
                total_types,
                total_tokens,
                ttr,
                mattr,
            }
        })
        .collect()
}
