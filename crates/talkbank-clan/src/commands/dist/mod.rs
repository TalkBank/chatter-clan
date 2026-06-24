//! DIST, Word distribution analysis across conversational turns.
//!
//! Reimplements CLAN's DIST command, which counts turns and tracks for each
//! word the first and last turn in which it appears. CLAN counts every
//! utterance as its own turn, regardless of whether the speaker changed.
//!
//! DIST is part of the FREQ family of commands and is useful for studying
//! when words first appear and how their usage is distributed across a
//! conversation.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                     | Rust equivalent                                  |
//! |----------------------------------|--------------------------------------------------|
//! | `dist file.cha`                  | `chatter analyze dist file.cha`                  |
//! | `dist +t*CHI file.cha`           | `chatter analyze dist file.cha -s CHI`           |
//!
//! # Output
//!
//! Global word list (sorted alphabetically by display form) with:
//! - Occurrence count across all turns
//! - First turn number (1-based) in which the word occurs
//! - Last turn number (omitted in CLAN output if same as first)
//! - Total number of turns in the transcript
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Turn detection and word extraction operate on parsed AST content
//!   rather than raw text lines.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::Utterance;

use crate::framework::word_filter::countable_words;
use crate::framework::{
    AnalysisCommand, FileContext, NormalizedWord, TurnCount, WordCount, clan_display_form,
};

pub use output::{DistResult, DistWordEntry};

/// Configuration for the DIST command.
#[derive(Debug, Clone, Default)]
pub struct DistConfig {
    /// CLAN `+g`: count each word at most once per utterance/turn.
    /// Mainly affects the `total_count` column; `first_turn` /
    /// `last_turn` are unchanged.
    pub once_per_turn: bool,
    /// CLAN `+k`: case-sensitive word keying. Default (`false`)
    /// lowercases via `NormalizedWord::from_word`; when `true`,
    /// the key preserves original case so `Want`/`want`/`WANT`
    /// land in separate by-word entries.
    pub case_sensitive: bool,
}

/// Per-word distribution data (internal accumulation).
#[derive(Debug, Default)]
struct WordDist {
    /// Total occurrences.
    total_count: WordCount,
    /// First turn (1-based) containing this word.
    first_turn: TurnCount,
    /// Last turn (1-based) containing this word.
    last_turn: TurnCount,
    /// CLAN display form.
    display_form: String,
}

/// Accumulated state for DIST across all files.
#[derive(Debug, Default)]
pub struct DistState {
    /// Per-word distribution data, keyed by normalized word.
    by_word: BTreeMap<NormalizedWord, WordDist>,
    /// Current turn number (incremented per utterance).
    current_turn: TurnCount,
}

/// DIST command implementation.
///
/// Tracks turns (one per utterance) and records per-word first/last turn.
#[derive(Debug, Clone, Default)]
pub struct DistCommand {
    /// User-facing configuration (e.g. CLAN `+g` once-per-turn).
    pub config: DistConfig,
}

impl DistCommand {
    /// Construct with explicit configuration.
    pub fn new(config: DistConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for DistCommand {
    type Config = DistConfig;
    type State = DistState;
    type Output = DistResult;

    /// Each utterance is a new turn. Update per-word first/last-turn metadata.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // CLAN counts every utterance as its own turn, regardless of speaker.
        state.current_turn += 1;

        let turn = state.current_turn;

        // `+g` (`once_per_turn`) collapses repeated occurrences of
        // the same word within one utterance to a single count.
        // `first_turn` / `last_turn` are unaffected because they
        // only ever update on first/most-recent encounter.
        let mut seen_this_turn: std::collections::HashSet<NormalizedWord> =
            std::collections::HashSet::new();
        let case_sensitive = self.config.case_sensitive;
        for word in countable_words(&utterance.main.content.content) {
            let key = NormalizedWord::from_word_cased(word, case_sensitive);
            let display = clan_display_form(word);

            let dist = state.by_word.entry(key.clone()).or_default();
            if !self.config.once_per_turn || seen_this_turn.insert(key) {
                dist.total_count += 1;
            }
            if dist.first_turn == 0 {
                dist.first_turn = turn;
                dist.display_form = display;
            }
            dist.last_turn = turn;
        }
    }

    /// Build sorted word rows and finalize total-turn count.
    fn finalize(&self, state: Self::State) -> DistResult {
        // Sort by display form alphabetically
        let mut entries: Vec<(NormalizedWord, WordDist)> = state.by_word.into_iter().collect();
        entries.sort_by(|a, b| a.1.display_form.cmp(&b.1.display_form));

        let words: Vec<DistWordEntry> = entries
            .into_iter()
            .map(|(key, dist)| {
                let average_distance = if dist.total_count >= 2 {
                    Some((dist.last_turn - dist.first_turn) as f64 / dist.total_count as f64)
                } else {
                    None
                };
                DistWordEntry {
                    word: key.as_str().to_owned(),
                    display_form: dist.display_form,
                    total_count: dist.total_count,
                    first_turn: dist.first_turn,
                    last_turn: dist.last_turn,
                    average_distance,
                }
            })
            .collect();

        DistResult {
            total_turns: state.current_turn,
            words,
        }
    }
}
