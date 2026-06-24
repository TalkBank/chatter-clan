//! CODES, Frequency table of codes from the `%cod` dependent tier.
//!
//! Reimplements CLAN's CODES command, which tabulates the frequency and
//! distribution of coding annotations found on `%cod:` dependent tiers,
//! organized by speaker. This is useful for analyzing hand-coded behavioral
//! or discourse annotations attached to transcripts.
//!
//! Codes on `%cod:` tiers typically use colon-separated hierarchical structure
//! (e.g., `AC:DI:PP`), but this implementation treats each whitespace-delimited
//! token as a single code string without parsing the internal hierarchy.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409098)
//! for the original CODES command specification.
//!
//! # Differences from CLAN
//!
//! - Codes are extracted from parsed `%cod:` dependent tier content rather
//!   than raw text line scanning.
//! - Each whitespace-delimited token is treated as a single code string;
//!   colon-separated hierarchy is preserved but not parsed into sublevels.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - `BTreeMap` ordering ensures deterministic output across runs.
//!
//! # Output
//!
//! Per-speaker frequency tables listing each code and its count, plus a
//! per-speaker total and a grand total across all speakers.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::{DependentTier, Utterance};

use crate::framework::{AnalysisCommand, CodeDepth, FileContext, cod_item_values};

pub use output::{CodeEntry, CodesResult, SpeakerCodes};

/// Configuration for the CODES command.
#[derive(Debug, Clone)]
pub struct CodesConfig {
    /// Maximum depth of code parsing (0 = all levels).
    pub max_depth: CodeDepth,
}

impl Default for CodesConfig {
    fn default() -> Self {
        Self {
            max_depth: CodeDepth::new(0),
        }
    }
}

/// Accumulated state for CODES across all files.
#[derive(Debug, Default)]
pub struct CodesState {
    /// Speaker → (code → count).
    speakers: BTreeMap<String, BTreeMap<String, u64>>,
}

/// CODES command implementation.
///
/// Extracts coding annotations from `%cod` dependent tiers and accumulates
/// per-speaker frequency counts. Each whitespace-delimited token on the
/// `%cod` line is treated as a separate code.
pub struct CodesCommand {
    _config: CodesConfig,
}

impl CodesCommand {
    /// Create a new CODES command with the given config.
    pub fn new(config: CodesConfig) -> Self {
        Self { _config: config }
    }
}

impl AnalysisCommand for CodesCommand {
    type Config = CodesConfig;
    type State = CodesState;
    type Output = CodesResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let speaker = utterance.main.speaker.to_string();

        for dep in &utterance.dependent_tiers {
            if let DependentTier::Cod(tier) = dep {
                let speaker_codes = state.speakers.entry(speaker.clone()).or_default();
                for code in cod_item_values(tier) {
                    *speaker_codes.entry(code).or_insert(0) += 1;
                }
            }
        }
    }

    fn finalize(&self, state: Self::State) -> CodesResult {
        let mut total = 0u64;
        let speakers: Vec<SpeakerCodes> = state
            .speakers
            .into_iter()
            .map(|(speaker, codes)| {
                let speaker_total: u64 = codes.values().sum();
                total += speaker_total;
                let entries: Vec<CodeEntry> = codes
                    .into_iter()
                    .map(|(code, count)| CodeEntry { code, count })
                    .collect();
                SpeakerCodes {
                    speaker,
                    entries,
                    total: speaker_total,
                }
            })
            .collect();

        CodesResult { speakers, total }
    }
}
