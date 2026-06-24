//! FREQPOS, Word frequency by position in utterance.
//!
//! Reimplements CLAN's FREQPOS command, which counts how often each word
//! appears in initial, final, other (middle), or one-word positions within
//! utterances. FREQPOS is part of the FREQ family of commands and is useful
//! for studying positional word preferences -- for example, whether a child
//! tends to place certain words at the beginning or end of utterances.
//!
//! Position classification rules:
//! - **Initial**: first word of a multi-word utterance
//! - **Final**: last word of a multi-word utterance
//! - **Other**: any middle word of a multi-word utterance (3+ words)
//! - **One-word**: the sole word in a single-word utterance
//!
//! # CLAN Equivalence
//!
//! | CLAN command                | Rust equivalent                           |
//! |-----------------------------|-------------------------------------------|
//! | `freqpos file.cha`          | `chatter analyze freqpos file.cha`        |
//! | `freqpos +t*CHI file.cha`   | `chatter analyze freqpos file.cha -s CHI` |
//!
//! # Output
//!
//! Global word list (sorted alphabetically by display form) with positional
//! breakdown (initial/final/other/one-word counts per word), followed by
//! aggregate position totals.
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Position classification operates on parsed AST word lists rather than
//!   raw text token splitting.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use serde::Serialize;
use talkbank_model::Utterance;

use crate::framework::word_filter::countable_words;
use crate::framework::{AnalysisCommand, FileContext, NormalizedWord, clan_display_form};

pub use output::{FreqposEntry, FreqposResult};

/// Position classification mode for FREQPOS (CLAN `+d`).
///
/// Default `FirstLastOther` matches CLAN's default behaviour:
/// position 0 is "initial", position `len-1` is "final", all
/// middle positions are "other". The `FirstSecondOther` mode
/// (CLAN `+d`) reclassifies position 1 as "second" instead, so
/// nothing past position 1 carries a positional label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum PositionClassification {
    /// CLAN default: position 0 → initial, position `len-1` →
    /// final, middle → other.
    #[default]
    FirstLastOther,
    /// CLAN `+d`: position 0 → initial, position 1 → second
    /// (formerly "final"), positions ≥ 2 → other.
    FirstSecondOther,
}

/// Configuration for the FREQPOS command.
#[derive(Debug, Clone, Default)]
pub struct FreqposConfig {
    /// CLAN `+d`: switch the classification mode. Default
    /// `FirstLastOther` matches the legacy CLAN behaviour.
    pub position_classification: PositionClassification,
    /// CLAN `+k`: case-sensitive word keying. Default (`false`)
    /// lowercases each word's `cleaned_text()` via the standard
    /// `NormalizedWord::from_word`; when `true`, the key preserves
    /// original case so `Want`/`want`/`WANT` become three distinct
    /// entries in the position-classification table.
    pub case_sensitive: bool,
}

/// Positional counts for a single word.
#[derive(Debug, Default, Clone)]
struct WordPositionCounts {
    /// Total occurrences
    total: u64,
    /// Occurrences as first word of a multi-word utterance
    initial: u64,
    /// Occurrences in the "second slot" of a multi-word utterance.
    /// Meaning depends on `position_classification`:
    /// `FirstLastOther` ⇒ last position (`i == len - 1`);
    /// `FirstSecondOther` (CLAN `+d`) ⇒ position 1.
    final_pos: u64,
    /// Occurrences in middle positions of a multi-word utterance
    other: u64,
    /// Occurrences as the sole word in a one-word utterance
    one_word: u64,
    /// CLAN display form (preserves `+` in compounds)
    display_form: String,
}

/// Accumulated state for FREQPOS across all files.
#[derive(Debug, Default)]
pub struct FreqposState {
    /// Per-word position counts, keyed by normalized word.
    by_word: BTreeMap<NormalizedWord, WordPositionCounts>,
}

/// FREQPOS command implementation.
///
/// For each utterance, classifies each word by its position
/// (initial/final/other/one-word) and accumulates counts globally.
#[derive(Debug, Clone, Default)]
pub struct FreqposCommand {
    /// User-facing configuration.
    pub config: FreqposConfig,
}

impl FreqposCommand {
    /// Construct with explicit configuration.
    pub fn new(config: FreqposConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for FreqposCommand {
    type Config = FreqposConfig;
    type State = FreqposState;
    type Output = FreqposResult;

    /// Classify each lexical token by utterance position and accumulate counts.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let case_sensitive = self.config.case_sensitive;
        let words: Vec<(NormalizedWord, String)> = countable_words(&utterance.main.content.content)
            .map(|w| {
                (
                    NormalizedWord::from_word_cased(w, case_sensitive),
                    clan_display_form(w),
                )
            })
            .collect();

        let len = words.len();
        if len == 0 {
            return;
        }

        for (i, (key, display)) in words.iter().enumerate() {
            let entry = state.by_word.entry(key.clone()).or_default();
            if entry.display_form.is_empty() {
                entry.display_form.clone_from(display);
            }
            entry.total += 1;

            // Classification depends on `position_classification`.
            // `FirstSecondOther` reinterprets the "final" counter
            // as "second", position 1 increments it, positions
            // past 1 go to "other".
            let mode = self.config.position_classification;
            if len == 1 {
                entry.one_word += 1;
            } else if i == 0 {
                entry.initial += 1;
            } else {
                let is_second_slot = match mode {
                    PositionClassification::FirstLastOther => i == len - 1,
                    PositionClassification::FirstSecondOther => i == 1,
                };
                if is_second_slot {
                    entry.final_pos += 1;
                } else {
                    entry.other += 1;
                }
            }
        }
    }

    /// Build sorted entries and compute global position totals.
    fn finalize(&self, state: Self::State) -> FreqposResult {
        let mut total_initial: u64 = 0;
        let mut total_other: u64 = 0;
        let mut total_final: u64 = 0;
        let mut total_one_word: u64 = 0;

        // Sort by display form alphabetically
        let mut entries_vec: Vec<(NormalizedWord, WordPositionCounts)> =
            state.by_word.into_iter().collect();
        entries_vec.sort_by(|a, b| a.1.display_form.cmp(&b.1.display_form));

        let entries: Vec<FreqposEntry> = entries_vec
            .into_iter()
            .map(|(key, counts)| {
                total_initial += counts.initial;
                total_other += counts.other;
                total_final += counts.final_pos;
                total_one_word += counts.one_word;

                FreqposEntry {
                    word: key.as_str().to_owned(),
                    display_form: counts.display_form,
                    total: counts.total,
                    initial: counts.initial,
                    final_pos: counts.final_pos,
                    other: counts.other,
                    one_word: counts.one_word,
                }
            })
            .collect();

        FreqposResult {
            entries,
            total_initial,
            total_other,
            total_final,
            total_one_word,
            position_classification: self.config.position_classification,
        }
    }
}
