//! CHIP, Child/Parent Interaction Profile.
//!
//! Reimplements CLAN's CHIP command, which analyzes interaction patterns between
//! a child speaker and their conversational partners. It categorizes successive
//! utterance pairs to measure imitation, repetition, and overlap. CHIP is
//! commonly used in child language research to quantify how much a child
//! imitates or echoes their interlocutor.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                       | Rust equivalent                                     |
//! |------------------------------------|-----------------------------------------------------|
//! | `chip +t*CHI file.cha`             | `chatter analyze chip file.cha -s CHI`              |
//! | `chip file.cha`                    | `chatter analyze chip file.cha`                     |
//!
//! # Interaction Categories
//!
//! For each adjacent utterance pair (speaker A followed by speaker B):
//! - **Exact repetition**: B's utterance words are identical to A's (order-independent)
//! - **Overlap**: B's utterance shares >=50% of words with A's (using the smaller
//!   unique-word set as denominator)
//! - **No overlap**: B's utterance shares <50% of words with A's
//!
//! Only cross-speaker adjacency is considered; consecutive utterances by the
//! same speaker do not produce interaction records. Adjacency state is reset
//! at file boundaries.
//!
//! # Output
//!
//! Per directed speaker pair (e.g., MOT->CHI is distinct from CHI->MOT):
//! - Counts of exact repetitions, overlaps, and non-overlaps
//! - Percentages of each category relative to the pair total
//! - Grand totals across all pairs
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Overlap comparison operates on parsed word content, not raw text.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use indexmap::IndexMap;
use talkbank_model::{Utterance, WriteChat};

use crate::framework::{AnalysisCommand, FileContext, countable_words};

pub use output::{ChipPairEntry, ChipResult};

/// Shared-word ratio threshold for classifying consecutive utterances as
/// overlapping. Two utterances with ratio ≥ this value are classified as
/// `Interaction::Overlap`. The CLAN CHIP command uses 50%.
const OVERLAP_THRESHOLD: f64 = 0.5;

/// Configuration for the CHIP command.
#[derive(Debug, Clone, Default)]
pub struct ChipConfig {}

/// Interaction category for an adjacent utterance pair (internal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Interaction {
    /// B's words are identical to A's.
    ExactRepetition,
    /// B shares ≥50% of words with A.
    Overlap,
    /// B shares <50% of words with A.
    NoOverlap,
}

/// Key for a directed speaker pair (from → to).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpeakerPair {
    from: String,
    to: String,
}

/// Accumulated interaction counts for a speaker pair (internal).
#[derive(Debug, Default)]
struct PairInteractions {
    exact_repetitions: u64,
    overlaps: u64,
    no_overlaps: u64,
}

impl PairInteractions {
    /// Total classified interactions accumulated for this directed pair.
    fn total(&self) -> u64 {
        self.exact_repetitions + self.overlaps + self.no_overlaps
    }
}

/// Accumulated state for CHIP across all files.
#[derive(Debug, Default)]
pub struct ChipState {
    /// Per-speaker-pair interaction counts.
    by_pair: IndexMap<SpeakerPair, PairInteractions>,
    /// Previous utterance's speaker and word set (for pair detection).
    pub prev_speaker: Option<String>,
    /// Previous utterance's words (lowercased, sorted for comparison).
    pub prev_words: Vec<String>,
    /// Echoed utterance lines for CLAN output.
    echoed_lines: Vec<String>,
}

/// CHIP command implementation.
///
/// Compares each utterance with the immediately preceding one. When speakers
/// differ, classifies the interaction as exact repetition, overlap, or
/// no overlap based on word-level comparison.
#[derive(Debug, Clone, Default)]
pub struct ChipCommand;

impl AnalysisCommand for ChipCommand {
    type Config = ChipConfig;
    type State = ChipState;
    type Output = ChipResult;

    /// Compare each utterance against the immediately previous utterance.
    ///
    /// Interactions are recorded only when speakers differ and both utterances
    /// contain at least one countable word.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let speaker = utterance.main.speaker.as_str().to_owned();
        let words: Vec<String> = countable_words(&utterance.main.content.content)
            .map(|w| w.cleaned_text().to_lowercase())
            .collect();

        // Echo utterance lines for CLAN output (main tier + %mor only, not %gra).
        state.echoed_lines.push(utterance.main.to_chat_string());
        if let Some(mor_tier) = utterance.mor_tier() {
            state.echoed_lines.push(mor_tier.to_chat_string());
        }

        // Compare with previous utterance if speakers differ
        if let Some(ref prev_speaker) = state.prev_speaker
            && *prev_speaker != speaker
            && !state.prev_words.is_empty()
            && !words.is_empty()
        {
            let interaction = classify_interaction(&state.prev_words, &words);
            let pair = SpeakerPair {
                from: prev_speaker.clone(),
                to: speaker.clone(),
            };
            let counts = state.by_pair.entry(pair).or_default();
            match interaction {
                Interaction::ExactRepetition => counts.exact_repetitions += 1,
                Interaction::Overlap => counts.overlaps += 1,
                Interaction::NoOverlap => counts.no_overlaps += 1,
            }
        }

        state.prev_speaker = Some(speaker);
        state.prev_words = words;
    }

    /// Reset adjacency state so interactions never cross file boundaries.
    fn end_file(&self, _file_context: &FileContext<'_>, state: &mut Self::State) {
        // Reset cross-utterance state at file boundaries
        state.prev_speaker = None;
        state.prev_words.clear();
    }

    /// Materialize totals and preserve encounter order for pair rows.
    fn finalize(&self, state: Self::State) -> ChipResult {
        let echoed_lines = state.echoed_lines;
        if state.by_pair.is_empty() {
            return ChipResult {
                pairs: Vec::new(),
                total_interactions: 0,
                total_exact: 0,
                total_overlaps: 0,
                echoed_lines,
            };
        }

        let total_interactions: u64 = state.by_pair.values().map(PairInteractions::total).sum();
        let total_exact: u64 = state.by_pair.values().map(|p| p.exact_repetitions).sum();
        let total_overlaps: u64 = state.by_pair.values().map(|p| p.overlaps).sum();

        let pairs: Vec<ChipPairEntry> = state
            .by_pair
            .into_iter()
            .map(|(pair, counts)| ChipPairEntry {
                from: pair.from,
                to: pair.to,
                exact_repetitions: counts.exact_repetitions,
                overlaps: counts.overlaps,
                no_overlaps: counts.no_overlaps,
            })
            .collect();

        ChipResult {
            pairs,
            total_interactions,
            total_exact,
            total_overlaps,
            echoed_lines,
        }
    }
}

/// Classify the interaction between two utterances based on word overlap.
///
/// - Exact repetition: sorted word lists are identical
/// - Overlap: ≥50% of the shorter utterance's unique words appear in the longer
/// - No overlap: <50% overlap
///
/// # Precondition
/// Both word lists must be non-empty.
fn classify_interaction(prev_words: &[String], curr_words: &[String]) -> Interaction {
    // Compare sorted word lists for exact repetition
    let mut prev_sorted = prev_words.to_vec();
    let mut curr_sorted = curr_words.to_vec();
    prev_sorted.sort();
    curr_sorted.sort();

    if prev_sorted == curr_sorted {
        return Interaction::ExactRepetition;
    }

    // Compute word overlap ratio
    let prev_set: HashSet<&str> = prev_words.iter().map(|s| s.as_str()).collect();
    let curr_set: HashSet<&str> = curr_words.iter().map(|s| s.as_str()).collect();
    let intersection_count = prev_set.intersection(&curr_set).count();

    // Use the smaller set as denominator for overlap ratio
    let min_size = prev_set.len().min(curr_set.len());
    let ratio = if min_size > 0 {
        intersection_count as f64 / min_size as f64
    } else {
        0.0
    };

    if ratio >= OVERLAP_THRESHOLD {
        Interaction::Overlap
    } else {
        Interaction::NoOverlap
    }
}
