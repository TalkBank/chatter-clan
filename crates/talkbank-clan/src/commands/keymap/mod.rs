//! KEYMAP, Contingency tables for coded data.
//!
//! Builds contingency (co-occurrence) matrices showing how often one
//! behavioral code follows another across consecutive utterances. Given
//! a set of keyword codes, KEYMAP tracks each keyword occurrence on a
//! specified coding tier (default `%cod`) and records what codes appear
//! in the immediately following utterance, broken down by speaker.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409207)
//! for the original KEYMAP command specification.
//!
//! # Differences from CLAN
//!
//! - Code extraction uses parsed dependent tier content rather than raw text.
//! - Keyword matching is case-insensitive by default.
//! - Output supports text, JSON, and CSV formats.
//! - Deterministic ordering via `BTreeMap`.
//!
//! # Output
//!
//! Per speaker per keyword:
//! - Total keyword occurrences
//! - Following codes with speaker attribution and frequency counts

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::Utterance;

use crate::framework::{
    AnalysisCommand, FileContext, cod_item_values, dependent_tier_content_text,
};

pub use output::{ContingencyEntry, FollowingCode, KeymapResult, SpeakerKeywordData};

/// Configuration for the KEYMAP command.
#[derive(Debug, Clone)]
pub struct KeymapConfig {
    /// Primary codes to track (keywords).
    pub keywords: Vec<crate::framework::KeywordPattern>,
    /// Tier kind to read codes from (default: %cod).
    pub tier: crate::framework::TierKind,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            keywords: Vec::new(),
            tier: crate::framework::TierKind::Cod,
        }
    }
}

/// A code occurrence with its speaker context.
#[derive(Debug)]
struct CodeOccurrence {
    speaker: String,
    code: String,
}

/// Accumulated state for KEYMAP across all files.
#[derive(Debug, Default)]
pub struct KeymapState {
    /// Recent code occurrences (sliding window for following-code tracking).
    recent: Vec<CodeOccurrence>,
    /// Speaker → keyword → (following_speaker:following_code → count).
    contingencies: BTreeMap<String, BTreeMap<String, BTreeMap<String, u64>>>,
    /// Speaker → keyword → total count.
    keyword_counts: BTreeMap<String, BTreeMap<String, u64>>,
}

/// KEYMAP command implementation.
///
/// For each utterance, extracts codes from the configured tier and checks
/// whether the previous utterance contained a keyword code. If so, records
/// the (keyword, following-code) pair with speaker attribution. The most
/// recent utterance's codes are kept in a sliding window for next-utterance
/// matching.
pub struct KeymapCommand {
    config: KeymapConfig,
}

impl KeymapCommand {
    /// Create a new KEYMAP command with the given config.
    pub fn new(config: KeymapConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for KeymapCommand {
    type Config = KeymapConfig;
    type State = KeymapState;
    type Output = KeymapResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let speaker = utterance.main.speaker.to_string();

        // Extract codes from the specified tier
        let mut codes: Vec<String> = Vec::new();
        for dep in &utterance.dependent_tiers {
            if self.config.tier == dep.kind() {
                if let talkbank_model::DependentTier::Cod(tier) = dep {
                    codes.extend(cod_item_values(tier));
                } else {
                    codes.extend(
                        dependent_tier_content_text(dep)
                            .split_whitespace()
                            .filter(|token| !token.is_empty() && *token != ".")
                            .map(str::to_owned),
                    );
                }
            }
        }

        // For each code in this utterance
        for code in &codes {
            // Check if any recent occurrence was a keyword, if so, this is a following code
            for recent in &state.recent {
                if self
                    .config
                    .keywords
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(&recent.code))
                {
                    let key = format!("{}:{}", speaker, code);
                    *state
                        .contingencies
                        .entry(recent.speaker.clone())
                        .or_default()
                        .entry(recent.code.clone())
                        .or_default()
                        .entry(key)
                        .or_insert(0) += 1;
                }
            }

            // Track if this code is a keyword
            if self
                .config
                .keywords
                .iter()
                .any(|k| k.eq_ignore_ascii_case(code))
            {
                *state
                    .keyword_counts
                    .entry(speaker.clone())
                    .or_default()
                    .entry(code.clone())
                    .or_insert(0) += 1;
            }
        }

        // Update recent codes (keep only codes from this utterance for next-utterance matching)
        state.recent.clear();
        for code in codes {
            state.recent.push(CodeOccurrence {
                speaker: speaker.clone(),
                code,
            });
        }
    }

    fn finalize(&self, state: Self::State) -> KeymapResult {
        let mut data = Vec::new();

        for (speaker, keywords) in &state.keyword_counts {
            for (keyword, total) in keywords {
                let mut following = Vec::new();

                if let Some(contingency) = state
                    .contingencies
                    .get(speaker)
                    .and_then(|kw| kw.get(keyword))
                {
                    for (key, count) in contingency {
                        let parts: Vec<&str> = key.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            following.push(FollowingCode {
                                speaker: parts[0].to_owned(),
                                code: parts[1].to_owned(),
                                count: *count,
                            });
                        }
                    }
                }

                data.push(SpeakerKeywordData {
                    speaker: speaker.clone(),
                    keyword: keyword.clone(),
                    total: *total,
                    following,
                });
            }
        }

        KeymapResult { data }
    }
}
