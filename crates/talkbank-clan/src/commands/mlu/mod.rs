//! MLU, Mean Length of Utterance.
//!
//! Calculates mean length of utterance in morphemes from the `%mor` tier.
//! When no `%mor` tier is available and not in `words_only` mode, reports
//! "utterances = 0, morphemes = 0" (matching CLAN behavior, no fallback
//! to word counting).
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409094)
//! for the original MLU command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command              | Rust equivalent                        |
//! |---------------------------|----------------------------------------|
//! | `mlu file.cha`            | `chatter analyze mlu file.cha`         |
//! | `mlu +t*CHI file.cha`     | `chatter analyze mlu file.cha -s CHI`  |
//!
//! # MLU Calculation
//!
//! For each utterance:
//! 1. Count morphemes on the `%mor` tier: 1 per stem + 1 per `-` suffix
//!    (bound morpheme) + 1 per `~` clitic stem + 1 per clitic `-` suffix.
//!    Fusional features (`&`) do NOT count.
//! 2. If no `%mor` tier, skip the utterance (report 0 utterances for the speaker)
//!
//! Per speaker, compute:
//! - Number of utterances
//! - Total morphemes
//! - MLU (mean)
//! - Standard deviation (sample, n-1)
//! - Range (min, max)
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Morpheme counting uses parsed `%mor` tier structure (MorWord features
//!   and post-clitics) rather than text splitting on spaces and delimiters.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use indexmap::IndexMap;
use talkbank_model::{SpeakerCode, Utterance};

use talkbank_model::model::content::word::UntranscribedStatus;

use crate::framework::word_filter::{
    countable_words_in_utterance, has_countable_words, main_tier_has_excluded_untranscribed,
    utterance_is_solo_excluded,
};
use crate::framework::{AnalysisCommand, FileContext, MorphemeCount, population_sd};

pub use output::{MluResult, MluSpeakerResult};

/// Configuration for the MLU command.
#[derive(Debug, Clone, Default)]
pub struct MluConfig {
    /// Use word count from main tier instead of morpheme count from %mor
    pub words_only: bool,
    /// Words that, when an utterance consists *solely* of them, cause
    /// the whole utterance to be excluded from the MLU count.
    /// Maps CLAN's command-specific `+gS` (e.g. `mlu +gum`) which is
    /// distinct from the inherited general `+gX` gem-segment filter.
    /// Comparison is by lower-cased word text after `NormalizedWord`
    /// normalization (same form chatter uses for countable-word
    /// iteration).
    pub solo_word_exclusions: Vec<String>,
    /// CLAN `+o3` (`mlu_isCombineSpeakers`, mlu.cpp:721): pool every selected
    /// speaker's utterances into one `*COMBINED*` MLU result instead of a
    /// per-speaker breakdown. Default `false` keeps the per-speaker layout.
    pub combine_speakers: bool,
    /// CLAN `+sxxx`/`+syyy`: untranscribed statuses re-admitted to the utterance
    /// count. By default MLU excludes any utterance containing `xxx`/`yyy`/`www`
    /// (manual §7.21 pt2); `+sxxx` re-includes the `xxx` utterances
    /// (`Unintelligible`) and `+syyy` the `yyy` utterances (`Phonetic`), with the
    /// marker still kept out of the morpheme count. `www` (`Untranscribed`) is
    /// never re-includable, so it never appears here. Empty = the default.
    pub re_included_untranscribed: Vec<UntranscribedStatus>,
}

/// Per-speaker MLU data accumulated during processing.
#[derive(Debug, Default)]
struct SpeakerMlu {
    /// Morpheme (or word) counts per utterance
    utterance_lengths: Vec<MorphemeCount>,
}

/// Accumulated state for MLU across all files.
#[derive(Debug, Default)]
pub struct MluState {
    /// Per-speaker MLU data, keyed by speaker code
    by_speaker: IndexMap<SpeakerCode, SpeakerMlu>,
}

/// MLU command implementation.
///
/// Counts morphemes per utterance from the %mor tier (or words from
/// the main tier), computing mean, SD, and range per speaker.
#[derive(Debug, Clone, Default)]
pub struct MluCommand {
    config: MluConfig,
    /// `config.solo_word_exclusions` lower-cased once at construction so
    /// the per-utterance hot path in [`utterance_is_solo_excluded`] does
    /// not re-allocate. Empty when the user did not pass
    /// `--exclude-solo-word`.
    solo_words_normalized: Vec<String>,
}

impl MluCommand {
    /// Create an MLU command with the given configuration.
    pub fn new(config: MluConfig) -> Self {
        let solo_words_normalized = config
            .solo_word_exclusions
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        Self {
            config,
            solo_words_normalized,
        }
    }
}

impl AnalysisCommand for MluCommand {
    type Config = MluConfig;
    type State = MluState;
    type Output = MluResult;

    /// Record one utterance length for the current speaker when lexical material exists.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // Skip utterances with no countable lexical content (e.g., "xxx .")
        // These would deflate MLU by adding zero-morpheme utterances to the
        // denominator. CLAN achieves this by string-prefix exclusion; we use
        // the AST's semantic word classification instead.
        if !has_countable_words(&utterance.main.content.content) {
            return;
        }

        // CLAN MLU excludes the WHOLE utterance when its main tier carries a
        // standalone `xxx`/`yyy`/`www` token, by default (manual §7.21 pt2;
        // `mlu_excludeUtter`, mllib.cpp:303-348, invoked on the main tier at
        // mlu.cpp:509). This is distinct from the no-countable-words skip above:
        // `*CHI: it xxx xxx` HAS a countable word (`it`) yet must be dropped
        // entirely. `+sxxx`/`+syyy` re-admit the xxx/yyy utterances via
        // `re_included_untranscribed` (the marker still stays out of the
        // `%mor`-based morpheme count); `www` can never be re-included.
        if main_tier_has_excluded_untranscribed(utterance, &self.config.re_included_untranscribed) {
            return;
        }

        // CLAN MLU excludes any utterance carrying the `[+ mlue]` postcode by
        // default (`isMLUEpostcode` defaults TRUE, mlu.cpp:108; the per-utterance
        // `isPostCodeOnUtt(line, "[+ mlue]")` -> `isSkip = TRUE`, mlu.cpp:503).
        // It is the explicit "exclude this utterance from MLU" researcher tag
        // (CHAT postcodes); chatter stores the payload (`mlue`) on
        // `MainTier::content.postcodes`. (CLAN's `+s"[+ mlue]"` can flip
        // `isMLUEpostcode` off to FORCE inclusion, manual §7.21 pt5; that
        // override is not yet surfaced, the default always excludes.)
        if utterance
            .main
            .content
            .postcodes
            .iter()
            .any(|postcode| postcode.text.as_str() == MLUE_EXCLUDE_POSTCODE)
        {
            return;
        }

        // CLAN's `mlu +gS` (filler-word elision) drops an utterance when
        // every countable word is in the user's solo-word list. Empty
        // list ⇒ no-op (fast path inside the helper).
        if utterance_is_solo_excluded(utterance, &self.solo_words_normalized) {
            return;
        }

        // Arc<str> clone, cheap atomic ref-count increment, no allocation
        let speaker = utterance.main.speaker.clone();

        let count = if self.config.words_only {
            Some(count_words_in_utterance(utterance))
        } else {
            crate::framework::count_traced_morphemes_in_utterance(utterance)
        };

        // Always register the speaker so they appear in output, even when
        // no %mor tier is available (CLAN parity: shows "utterances = 0").
        let speaker_mlu = state
            .by_speaker
            .entry(speaker)
            .or_insert_with(SpeakerMlu::default);

        // When no %mor tier exists and not in words_only mode, CLAN reports
        // "utterances = 0, morphemes = 0"; it does NOT fall back to counting
        // words. We skip adding the utterance length but still register the
        // speaker above.
        if let Some(count) = count {
            speaker_mlu.utterance_lengths.push(count);
        }
    }

    /// Compute per-speaker MLU aggregates (mean, SD, min, max) from collected
    /// lengths, or a single pooled `*COMBINED*` aggregate under CLAN `+o3`.
    fn finalize(&self, state: Self::State) -> MluResult {
        let speakers = if self.config.combine_speakers {
            // CLAN `+o3`: pool every selected speaker's utterance lengths into
            // one combined result. The label is rendered as `*COMBINED*`
            // (output.rs), so the speaker code here is just a placeholder.
            let pooled: Vec<MorphemeCount> = state
                .by_speaker
                .values()
                .flat_map(|m| m.utterance_lengths.iter().copied())
                .collect();
            vec![aggregate_mlu(COMBINED_SPEAKER_LABEL, &pooled)]
        } else {
            state
                .by_speaker
                .iter()
                .map(|(speaker, mlu_data)| {
                    aggregate_mlu(speaker.as_str(), &mlu_data.utterance_lengths)
                })
                .collect()
        };
        MluResult {
            speakers,
            combine_speakers: self.config.combine_speakers,
            re_included_untranscribed: self.config.re_included_untranscribed.clone(),
        }
    }
}

/// Placeholder speaker code for the `+o3` pooled result; the CLAN renderer shows
/// it as `*COMBINED*` (gated on `MluResult::combine_speakers`).
const COMBINED_SPEAKER_LABEL: &str = "COMBINED";

/// The CHAT postcode payload (`[+ mlue]`, stored as `mlue`) that marks an
/// utterance for exclusion from MLU. CLAN matches the literal `[+ mlue]`
/// (mlu.cpp:503); chatter compares the parsed postcode text.
const MLUE_EXCLUDE_POSTCODE: &str = "mlue";

/// Build the [`MluConfig::re_included_untranscribed`] set from the two CLAN
/// `+sxxx`/`+syyy` re-include flags. `www` (`Untranscribed`) is never
/// includable, so it never appears. Centralizes the flag-to-status mapping so
/// the service builder and the golden-test harness stay in lock-step (one place
/// owns "xxx -> Unintelligible, yyy -> Phonetic").
pub fn re_included_untranscribed(include_xxx: bool, include_yyy: bool) -> Vec<UntranscribedStatus> {
    [
        (include_xxx, UntranscribedStatus::Unintelligible),
        (include_yyy, UntranscribedStatus::Phonetic),
    ]
    .into_iter()
    .filter_map(|(flag, status)| flag.then_some(status))
    .collect()
}

/// Build one [`MluSpeakerResult`] from a speaker's (or the pooled) per-utterance
/// lengths: mean, population SD (`/ n`, NaN when `n == 1` -> CLAN "NA"), min/max.
/// `n == 0` (a registered speaker with no `%mor`) yields all-zero counts and no
/// Ratio/SD lines, matching CLAN.
fn aggregate_mlu(speaker: &str, lengths: &[MorphemeCount]) -> MluSpeakerResult {
    let n = lengths.len() as u64;
    if n == 0 {
        return MluSpeakerResult {
            speaker: speaker.to_owned(),
            utterances: 0,
            morphemes: 0,
            mlu: 0.0,
            sd: 0.0,
            min: 0,
            max: 0,
        };
    }

    let total: u64 = lengths.iter().sum();
    let mean = total as f64 / n as f64;
    MluSpeakerResult {
        speaker: speaker.to_owned(),
        utterances: n,
        morphemes: total,
        mlu: mean,
        // Population SD (/ n), NaN at n=1 -> CLAN "NA"; shared with MLT.
        sd: population_sd(lengths),
        min: lengths.iter().copied().min().unwrap_or(0),
        max: lengths.iter().copied().max().unwrap_or(0),
    }
}

/// Count words in an utterance from the main tier (fallback when no %mor).
///
/// Uses the shared [`countable_words_in_utterance`] iterator to avoid
/// duplicating the tree-walking logic.
fn count_words_in_utterance(utterance: &Utterance) -> u64 {
    countable_words_in_utterance(utterance).count() as u64
}
