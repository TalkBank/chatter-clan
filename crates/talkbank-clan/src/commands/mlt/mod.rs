//! MLT, Mean Length of Turn.
//!
//! Calculates mean length of turn in utterances and words. A "turn" is a
//! maximal consecutive sequence of utterances by the same speaker; the
//! turn boundary is detected when a different speaker produces the next
//! utterance.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409101)
//! for the original MLT command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command              | Rust equivalent                        |
//! |---------------------------|----------------------------------------|
//! | `mlt file.cha`            | `chatter analyze mlt file.cha`         |
//! | `mlt +t*CHI file.cha`     | `chatter analyze mlt file.cha -s CHI`  |
//!
//! # Output
//!
//! Per speaker:
//! - Number of turns
//! - Total utterances and words
//! - Mean turn length in utterances (MLT-u) and words (MLT-w)
//! - Sample standard deviation of words per turn
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Turn detection operates on parsed speaker codes from the AST rather
//!   than raw text line prefixes.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use indexmap::IndexMap;
use talkbank_model::{SpeakerCode, Utterance};

use crate::framework::word_filter::{
    countable_words, has_countable_words, utterance_is_solo_excluded,
};
use crate::framework::{AnalysisCommand, FileContext, UtteranceCount, WordCount, population_sd};

pub use output::{MltResult, MltSpeakerResult};

/// Configuration for the MLT command.
#[derive(Debug, Clone, Default)]
pub struct MltConfig {
    /// Words that, when an utterance consists *solely* of them, cause
    /// the whole utterance to be excluded from the MLT count.
    /// Maps CLAN's command-specific `+gS` (e.g. `mlt +gum`) which is
    /// distinct from the inherited general `+gX` gem-segment filter.
    /// Comparison is by lower-cased word text after `NormalizedWord`
    /// normalization (same form chatter uses for countable-word
    /// iteration).
    pub solo_word_exclusions: Vec<String>,
}

/// A single completed turn's statistics.
#[derive(Debug, Default)]
struct Turn {
    /// Number of utterances in this turn.
    utterances: UtteranceCount,
    /// Number of countable words in this turn.
    words: WordCount,
}

/// Per-speaker turn data accumulated during processing.
///
/// Uses a `Vec<Turn>` for completed turns and a single `Turn` for the
/// in-progress turn, replacing the previous parallel `Vec<u64>` fields
/// that represented the same data redundantly.
#[derive(Debug, Default)]
struct SpeakerTurns {
    /// Completed turns (closed when speaker changed or file ended).
    completed: Vec<Turn>,
    /// Current (in-progress) turn.
    current: Turn,
}

impl SpeakerTurns {
    /// Close the current turn (if non-empty) and start a new one.
    ///
    /// # Postcondition
    /// After calling, `current` is reset to a default (empty) turn.
    fn close_turn(&mut self) {
        if self.current.utterances > 0 {
            self.completed.push(Turn {
                utterances: self.current.utterances,
                words: self.current.words,
            });
            self.current = Turn::default();
        }
    }
}

/// Accumulated state for MLT across all files.
#[derive(Debug, Default)]
pub struct MltState {
    /// Per-speaker turn data, keyed by speaker code
    by_speaker: IndexMap<SpeakerCode, SpeakerTurns>,
    /// Speaker code of the most recent utterance (for turn boundary detection)
    last_speaker: Option<SpeakerCode>,
    /// Per-speaker per-utterance word counts (for SD computation)
    words_per_utterance: IndexMap<SpeakerCode, Vec<WordCount>>,
}

/// MLT command implementation.
///
/// Tracks turn boundaries by detecting when the speaker changes between
/// consecutive utterances. Each turn accumulates utterance and word counts.
#[derive(Debug, Clone, Default)]
pub struct MltCommand {
    /// `MltConfig::solo_word_exclusions` lower-cased once at construction
    /// so the per-utterance hot path in [`utterance_is_solo_excluded`]
    /// does not re-allocate. The source `MltConfig` is consumed by
    /// [`MltCommand::new`] to derive this; it is not retained.
    solo_words_normalized: Vec<String>,
}

impl MltCommand {
    /// Construct an MLT command with the given configuration.
    pub fn new(config: MltConfig) -> Self {
        let solo_words_normalized = config
            .solo_word_exclusions
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        Self {
            solo_words_normalized,
        }
    }
}

impl AnalysisCommand for MltCommand {
    type Config = MltConfig;
    type State = MltState;
    type Output = MltResult;

    /// Update turn state for one lexical utterance and detect speaker boundaries.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // Skip utterances with no countable lexical content
        if !has_countable_words(&utterance.main.content.content) {
            return;
        }

        // CLAN's `mlt +gS` (filler-word elision) drops an utterance when
        // every countable word is in the user's solo-word list. Empty
        // list ⇒ no-op (fast path inside the helper).
        if utterance_is_solo_excluded(utterance, &self.solo_words_normalized) {
            return;
        }

        // Arc<str> clone, cheap atomic ref-count increment, no allocation
        let speaker = utterance.main.speaker.clone();

        // Detect turn boundary: if the speaker changed, close all open turns
        if state.last_speaker.as_ref() != Some(&speaker) {
            // Close the previous speaker's turn
            if let Some(ref prev) = state.last_speaker
                && let Some(prev_turns) = state.by_speaker.get_mut(prev)
            {
                prev_turns.close_turn();
            }
            state.last_speaker = Some(speaker.clone());
        }

        let speaker_turns = state
            .by_speaker
            .entry(speaker.clone())
            .or_insert_with(SpeakerTurns::default);

        // Count words using the shared countable_words() iterator
        let word_count = countable_words(&utterance.main.content.content).count() as u64;

        speaker_turns.current.utterances += 1;
        speaker_turns.current.words += word_count;

        // Track per-utterance word count for SD computation
        state
            .words_per_utterance
            .entry(speaker.clone())
            .or_default()
            .push(word_count);
    }

    /// Close any open turns at file boundary so stats do not leak across files.
    fn end_file(&self, _file_context: &FileContext<'_>, state: &mut Self::State) {
        // Close all open turns at file boundary
        for turns in state.by_speaker.values_mut() {
            turns.close_turn();
        }
        state.last_speaker = None;
    }

    /// Compute per-speaker MLT metrics from completed turn sequences.
    fn finalize(&self, state: Self::State) -> MltResult {
        let mut speakers = Vec::new();

        for (speaker, turns) in &state.by_speaker {
            let num_turns = turns.completed.len() as u64;
            if num_turns == 0 {
                continue;
            }

            let total_utterances: u64 = turns.completed.iter().map(|t| t.utterances).sum();
            let total_words: u64 = turns.completed.iter().map(|t| t.words).sum();

            let mlt_utterances = total_utterances as f64 / num_turns as f64;
            let mlt_words = total_words as f64 / num_turns as f64;
            let words_per_utterance = if total_utterances > 0 {
                total_words as f64 / total_utterances as f64
            } else {
                0.0
            };

            // Population standard deviation of words-per-UTTERANCE (not per-turn).
            // CLAN computes SD over individual utterance word counts, using
            // population SD (/ n) and printing "NA" for n <= 1. Shared with MLU
            // via the framework `population_sd` helper (its internal mean,
            // sum / n, equals `words_per_utterance` above).
            let sd = population_sd(
                state
                    .words_per_utterance
                    .get(speaker)
                    .map_or(&[][..], Vec::as_slice),
            );

            speakers.push(MltSpeakerResult {
                speaker: speaker.as_str().to_owned(),
                turns: num_turns,
                utterances: total_utterances,
                words: total_words,
                mlt_words,
                mlt_utterances,
                words_per_utterance,
                sd,
            });
        }

        MltResult { speakers }
    }
}
