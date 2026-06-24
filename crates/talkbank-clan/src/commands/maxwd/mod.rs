//! MAXWD, Longest Words.
//!
//! Finds the longest words used by each speaker, reporting a ranked table
//! of unique words sorted by character length descending. Word length is
//! measured in characters after normalization (lowercasing, stripping `+`
//! and `'` for CLAN compatibility).
//!
//! MAXWD does not have a dedicated section in the CLAN manual.
//!
//! # CLAN Equivalence
//!
//! | CLAN command               | Rust equivalent                          |
//! |----------------------------|------------------------------------------|
//! | `maxwd file.cha`           | `chatter analyze maxwd file.cha`         |
//! | `maxwd +t*CHI file.cha`    | `chatter analyze maxwd file.cha -s CHI`  |
//!
//! # Output
//!
//! Per speaker:
//! - Table of longest words sorted by length descending (up to `limit`)
//! - Maximum word length
//! - Mean word length
//! - Total and unique word counts
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Word length measurement uses parsed, normalized word content rather
//!   than raw text character counting.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use indexmap::IndexMap;
use talkbank_model::{SpeakerCode, Utterance};

use crate::framework::word_filter::countable_words;
use crate::framework::{
    AnalysisCommand, FileContext, NormalizedWord, WordCount, WordLimit, clan_display_form,
};

pub use output::{MaxwdOccurrence, MaxwdResult, MaxwdSpeakerResult};

/// Configuration for the MAXWD command.
#[derive(Debug, Clone)]
pub struct MaxwdConfig {
    /// Maximum number of words to show in the output table.
    /// Default: 20
    pub limit: WordLimit,
    /// CLAN `+a`: include only words whose length is unique within
    /// a speaker's lexicon. Words sharing a length with another
    /// word in the same speaker's data are dropped from the
    /// output table, and `max_length` is recomputed over the
    /// surviving entries.
    pub unique_length_only: bool,
    /// CLAN `+xN`: word character lengths to drop from output.
    /// Repeatable on the command line (`+x5 +x7` excludes both
    /// lengths). Applied per-speaker before sorting, after
    /// `unique_length_only`.
    pub exclude_lengths: Vec<usize>,
    /// CLAN `+k`: case-sensitive word keying. Default (`false`)
    /// lowercases via `NormalizedWord::from_word`; when `true`,
    /// the key preserves original case so `Want`/`want`/`WANT`
    /// are treated as three distinct words for the unique-length
    /// and exclude-length filters.
    pub case_sensitive: bool,
}

impl Default for MaxwdConfig {
    /// Default to CLAN-style top-20 longest words.
    fn default() -> Self {
        Self {
            limit: WordLimit::new(20),
            unique_length_only: false,
            exclude_lengths: Vec::new(),
            case_sensitive: false,
        }
    }
}

/// Count characters the way CLAN does: strip `+` and `'` before counting.
fn clan_char_count(word: &str) -> usize {
    word.chars().filter(|c| *c != '+' && *c != '\'').count()
}

/// Per-speaker word tracking for finding longest words.
#[derive(Debug, Default)]
struct SpeakerMaxwd {
    /// All unique words encountered, keyed by normalized text,
    /// storing character length.
    /// Using BTreeMap for deterministic iteration order.
    words: BTreeMap<NormalizedWord, usize>,
    /// CLAN display forms (preserving `+` in compounds)
    display_forms: std::collections::HashMap<NormalizedWord, String>,
    /// Total characters across all word tokens (for mean)
    total_chars: u64,
    /// Total word tokens counted
    total_words: WordCount,
}

/// Accumulated state for MAXWD across all files.
#[derive(Debug, Default)]
pub struct MaxwdState {
    /// Per-speaker word data, keyed by speaker code
    by_speaker: IndexMap<SpeakerCode, SpeakerMaxwd>,
    /// Word → line number mapping for CLAN format (first occurrence)
    word_line_numbers: std::collections::HashMap<NormalizedWord, usize>,
    /// Every word occurrence: (display_form, char_length, line_number).
    /// Not deduplicated, used to find all occurrences at the max length.
    all_occurrences: Vec<(String, usize, usize)>,
}

/// MAXWD command implementation.
///
/// Collects unique words per speaker, then reports the longest ones
/// sorted by character length descending.
#[derive(Debug, Clone, Default)]
pub struct MaxwdCommand {
    config: MaxwdConfig,
}

impl MaxwdCommand {
    /// Create a MAXWD command with the given configuration.
    pub fn new(config: MaxwdConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for MaxwdCommand {
    type Config = MaxwdConfig;
    type State = MaxwdState;
    type Output = MaxwdResult;

    /// Accumulate per-speaker lexical inventory, lengths, and first-seen line numbers.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // Arc<str> clone, cheap atomic ref-count increment, no allocation
        let speaker = utterance.main.speaker.clone();
        let speaker_data = state
            .by_speaker
            .entry(speaker)
            .or_insert_with(SpeakerMaxwd::default);

        // Compute line number: O(log n) via LineMap when available, else 0
        let line_number = file_context
            .line_map
            .map(|lm| lm.line_of(utterance.main.span.start))
            .unwrap_or(0);

        let case_sensitive = self.config.case_sensitive;
        for word in countable_words(&utterance.main.content.content) {
            let text = NormalizedWord::from_word_cased(word, case_sensitive);
            let len = text.as_str().chars().count();
            let display = clan_display_form(word);
            let clan_len = clan_char_count(&display);

            // Track unique word → length (keep the word for display)
            speaker_data.words.entry(text.clone()).or_insert(len);
            speaker_data
                .display_forms
                .entry(text.clone())
                .or_insert_with(|| display.clone());
            state.word_line_numbers.entry(text).or_insert(line_number);

            // Track every occurrence for CLAN output (not deduplicated)
            state.all_occurrences.push((display, clan_len, line_number));

            speaker_data.total_chars += len as u64;
            speaker_data.total_words += 1;
        }
    }

    /// Build per-speaker longest-word tables and summary metrics.
    fn finalize(&self, state: Self::State) -> MaxwdResult {
        let mut speakers = Vec::new();
        for (speaker, data) in state.by_speaker {
            if data.total_words == 0 {
                continue;
            }

            let mut entries: Vec<(NormalizedWord, usize)> = data.words.into_iter().collect();

            // `+a` (`unique_length_only`) drops words whose length
            // is shared with another word in the same speaker's
            // lexicon. Done before sorting so the length-count
            // bucket can be built in one pass.
            if self.config.unique_length_only {
                let mut length_count: std::collections::HashMap<usize, usize> =
                    std::collections::HashMap::new();
                for (_, len) in &entries {
                    *length_count.entry(*len).or_insert(0) += 1;
                }
                entries.retain(|(_, len)| length_count.get(len).copied() == Some(1));
            }

            // `+xN` (`exclude_lengths`) drops words whose length
            // matches any entry in the exclusion list.
            if !self.config.exclude_lengths.is_empty() {
                entries.retain(|(_, len)| !self.config.exclude_lengths.contains(len));
            }

            entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

            let max_length = entries.first().map(|(_, len)| *len).unwrap_or(0);
            let unique_words = entries.len();
            let mean_length = data.total_chars as f64 / data.total_words as f64;

            let top_words: Vec<(usize, String)> = entries
                .into_iter()
                .take(self.config.limit.get())
                .map(|(word, len)| (len, word.as_str().to_owned()))
                .collect();

            // Build display_forms and line_numbers maps keyed by normalized word string
            let display_forms: std::collections::HashMap<String, String> = data
                .display_forms
                .into_iter()
                .map(|(k, v)| (k.as_str().to_owned(), v))
                .collect();
            let line_numbers: std::collections::HashMap<String, usize> = state
                .word_line_numbers
                .iter()
                .map(|(k, v)| (k.as_str().to_owned(), *v))
                .collect();

            speakers.push(MaxwdSpeakerResult {
                speaker: speaker.as_str().to_owned(),
                max_length,
                mean_length,
                total_words: data.total_words,
                unique_words,
                top_words,
                display_forms,
                line_numbers,
            });
        }
        // Find the global max CLAN char length across all occurrences
        let global_max = state
            .all_occurrences
            .iter()
            .map(|(_, len, _)| *len)
            .max()
            .unwrap_or(0);

        // Collect all occurrences at the max length, sorted by line number
        let mut longest_occurrences: Vec<MaxwdOccurrence> = state
            .all_occurrences
            .into_iter()
            .filter(|(_, len, _)| *len == global_max && global_max > 0)
            .map(|(display_form, char_length, line_number)| MaxwdOccurrence {
                display_form,
                char_length,
                line_number,
            })
            .collect();
        longest_occurrences.sort_by_key(|o| o.line_number);

        MaxwdResult {
            speakers,
            longest_occurrences,
        }
    }
}
