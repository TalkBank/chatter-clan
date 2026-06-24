//! CORELEX, Core vocabulary analysis.
//!
//! Identifies "core" vocabulary items that appear above a frequency
//! threshold. Core vocabulary analysis is used in clinical assessment
//! to evaluate whether a child's lexicon includes expected high-frequency
//! words.
//!
//! # CLAN Equivalence
//!
//! | CLAN command | Rust equivalent |
//! |---|---|
//! | `corelex file.cha` | `chatter clan corelex file.cha` |
//! | `corelex +t*CHI file.cha` | `chatter clan corelex --speaker CHI file.cha` |
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching.
//! - Output supports text, JSON, and CSV formats.
//! - Core/non-core classification uses shared `NormalizedWord` for consistency.

mod output;

use std::collections::BTreeMap;

use indexmap::IndexMap;
use talkbank_model::{SpeakerCode, Utterance};

use crate::framework::word_filter::countable_words;
use crate::framework::{
    AnalysisCommand, FileContext, FrequencyThreshold, NormalizedWord, TypeCount,
};

pub use output::{CorelexEntry, CorelexResult};

/// Configuration for the CORELEX command.
#[derive(Debug, Clone)]
pub struct CorelexConfig {
    /// Minimum frequency for a word to be considered "core" (default: 3).
    pub min_frequency: FrequencyThreshold,
}

impl Default for CorelexConfig {
    fn default() -> Self {
        Self {
            min_frequency: FrequencyThreshold::new(3),
        }
    }
}

/// Accumulated state for CORELEX.
#[derive(Debug, Default)]
pub struct CorelexState {
    /// Per-speaker word frequencies.
    by_speaker: IndexMap<SpeakerCode, BTreeMap<NormalizedWord, u64>>,
}

/// The CORELEX command.
#[derive(Debug, Clone, Default)]
pub struct CorelexCommand {
    config: CorelexConfig,
}

impl CorelexCommand {
    /// Create a CORELEX command with the given configuration.
    pub fn new(config: CorelexConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for CorelexCommand {
    type Config = CorelexConfig;
    type State = CorelexState;
    type Output = CorelexResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _ctx: &FileContext<'_>,
        state: &mut CorelexState,
    ) {
        let speaker = utterance.main.speaker.clone();
        let speaker_freq = state.by_speaker.entry(speaker).or_default();

        for word in countable_words(&utterance.main.content.content) {
            let normalized = NormalizedWord::from_word(word);
            *speaker_freq.entry(normalized).or_insert(0) += 1;
        }
    }

    fn finalize(&self, state: CorelexState) -> CorelexResult {
        let mut total_freq: BTreeMap<NormalizedWord, u64> = BTreeMap::new();
        let mut speaker_counts: BTreeMap<NormalizedWord, usize> = BTreeMap::new();

        for speaker_freq in state.by_speaker.values() {
            for (word, count) in speaker_freq {
                *total_freq.entry(word.clone()).or_insert(0) += count;
                *speaker_counts.entry(word.clone()).or_insert(0) += 1;
            }
        }

        let mut core = Vec::new();
        let mut non_core = Vec::new();

        for (word, freq) in &total_freq {
            let entry = CorelexEntry {
                word: word.as_str().to_owned(),
                frequency: *freq,
                speaker_count: speaker_counts.get(word).copied().unwrap_or(0),
            };
            if *freq >= self.config.min_frequency.get() {
                core.push(entry);
            } else {
                non_core.push(entry);
            }
        }

        core.sort_by(|a, b| b.frequency.cmp(&a.frequency).then(a.word.cmp(&b.word)));
        non_core.sort_by(|a, b| b.frequency.cmp(&a.frequency).then(a.word.cmp(&b.word)));

        let total_types = total_freq.len() as TypeCount;
        let core_count = core.len() as TypeCount;
        let non_core_count = non_core.len() as TypeCount;
        let core_percentage = if total_types > 0 {
            core_count as f64 / total_types as f64 * 100.0
        } else {
            0.0
        };

        CorelexResult {
            core,
            non_core,
            total_types,
            core_count,
            non_core_count,
            core_percentage,
            threshold: self.config.min_frequency.get(),
        }
    }
}
