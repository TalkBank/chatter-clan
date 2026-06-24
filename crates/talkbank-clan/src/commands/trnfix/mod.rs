//! TRNFIX, Compare two dependent tiers and flag misalignments.
//!
//! Compares two dependent tiers (default: `%mor` and `%trn`) word-by-word
//! across all utterances, reporting unique mismatch pairs with frequency
//! counts and an overall accuracy percentage. Useful for verifying tier
//! consistency after automatic annotation or manual correction.
//!
//! When tiers have different lengths for a given utterance, missing
//! positions are represented as the null symbol `\u{2205}` (empty set).
//!
//! TRNFIX does not have a dedicated section in the CLAN manual.
//!
//! # Differences from CLAN
//!
//! - Tier content is compared from parsed AST data rather than raw text.
//! - Length mismatches are handled with explicit `∅` null symbols.
//! - Configurable tier names (CLAN uses fixed `%mor`/`%trn` comparison).
//! - Output supports text, JSON, and CSV formats.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::{DependentTier, Utterance};

use crate::framework::{
    AnalysisCommand, FileContext, dependent_tier_content_text, gra_relation_texts, mor_item_texts,
};

pub use output::{TrnfixMismatch, TrnfixResult};

/// Configuration for the TRNFIX command.
#[derive(Debug, Clone)]
pub struct TrnfixConfig {
    /// First tier to compare (default: %mor).
    pub tier1: crate::framework::TierKind,
    /// Second tier to compare (default: %trn, which aliases to %mor).
    pub tier2: crate::framework::TierKind,
}

impl Default for TrnfixConfig {
    fn default() -> Self {
        Self {
            tier1: crate::framework::TierKind::Mor,
            tier2: crate::framework::TierKind::Mor,
        }
    }
}

/// Accumulated state for TRNFIX across all files.
#[derive(Debug, Default)]
pub struct TrnfixState {
    /// Mismatch pair → count.
    mismatches: BTreeMap<(String, String), u64>,
    /// Total items compared.
    total_items: u64,
    /// Total errors found.
    total_errors: u64,
}

/// TRNFIX command implementation.
///
/// For each utterance, extracts text from both configured tiers and
/// compares tokens positionally. Mismatched pairs are accumulated in
/// a frequency map; matched positions contribute to the accuracy
/// percentage.
pub struct TrnfixCommand {
    config: TrnfixConfig,
}

impl TrnfixCommand {
    /// Create a new TRNFIX command with the given config.
    pub fn new(config: TrnfixConfig) -> Self {
        Self { config }
    }
}

/// Extract tier content as token sequence, given a tier label to match.
///
/// `%trn` aliases `%mor` and `%grt` aliases `%gra`.
fn extract_tier_tokens(utterance: &Utterance, label: &str) -> Option<Vec<String>> {
    for dep in &utterance.dependent_tiers {
        let matches = match dep {
            DependentTier::Mor(_) if matches!(label, "mor" | "trn") => true,
            DependentTier::Gra(_) if matches!(label, "gra" | "grt") => true,
            DependentTier::Pho(_) if label == "pho" => true,
            DependentTier::Mod(_) if label == "mod" => true,
            DependentTier::UserDefined(u) if u.label.as_str().eq_ignore_ascii_case(label) => true,
            _ => false,
        };
        if matches {
            return Some(match dep {
                DependentTier::Mor(tier) => mor_item_texts(tier),
                DependentTier::Gra(tier) => gra_relation_texts(tier),
                _ => dependent_tier_content_text(dep)
                    .split_whitespace()
                    .map(ToOwned::to_owned)
                    .collect(),
            });
        }
    }
    None
}

impl AnalysisCommand for TrnfixCommand {
    type Config = TrnfixConfig;
    type State = TrnfixState;
    type Output = TrnfixResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let words1 = match extract_tier_tokens(utterance, self.config.tier1.as_str()) {
            Some(t) => t,
            None => return,
        };
        let words2 = match extract_tier_tokens(utterance, self.config.tier2.as_str()) {
            Some(t) => t,
            None => return,
        };
        let max_len = words1.len().max(words2.len());

        for i in 0..max_len {
            let w1 = words1.get(i).map(String::as_str).unwrap_or("∅");
            let w2 = words2.get(i).map(String::as_str).unwrap_or("∅");
            state.total_items += 1;

            if w1 != w2 {
                state.total_errors += 1;
                *state
                    .mismatches
                    .entry((w1.to_owned(), w2.to_owned()))
                    .or_insert(0) += 1;
            }
        }
    }

    fn finalize(&self, state: Self::State) -> TrnfixResult {
        let accuracy = if state.total_items > 0 {
            100.0 - (state.total_errors as f64 * 100.0 / state.total_items as f64)
        } else {
            100.0
        };

        let mismatches: Vec<TrnfixMismatch> = state
            .mismatches
            .into_iter()
            .map(|((t1, t2), count)| TrnfixMismatch {
                tier1_word: t1,
                tier2_word: t2,
                count,
            })
            .collect();

        TrnfixResult {
            mismatches,
            total_items: state.total_items,
            total_errors: state.total_errors,
            accuracy,
            tier1: self.config.tier1.to_string(),
            tier2: self.config.tier2.to_string(),
        }
    }
}
