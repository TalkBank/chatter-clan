//! SUGAR, Sampling Utterances and Grammatical Analysis Revised.
//!
//! Computes language sample analysis metrics from `%mor` and `%gra`
//! tiers, providing a quick clinical assessment of grammatical
//! complexity:
//!
//! - **MLU-S**: Mean Length of Utterance in morphemes
//! - **TNW**: Total Number of Words (tokens with POS tags)
//! - **WPS**: Words Per Sentence (utterances containing verbs)
//! - **CPS**: Clauses Per Sentence (from `%gra` subordination relations)
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409287)
//! for the original SUGAR command specification.
//!
//! # Differences from CLAN
//!
//! - Verb detection uses mapped POS tags from the parsed `%mor` tier.
//!   CLAN may use a slightly different POS tag set for verb identification.
//! - Clause counting uses `%gra` subordination relations only. CLAN's
//!   clause detection may use additional heuristics.
//! - Minimum utterance threshold is configurable (CLAN uses a fixed value).
//! - Output supports text, JSON, and CSV formats.
//!
//! # Algorithm
//!
//! 1. For each utterance, count morphemes and words from `%mor`.
//! 2. Detect verb-containing utterances (POS tags: `v`, `cop`, `aux`,
//!    `mod`, `part`).
//! 3. For verb utterances with `%gra`, count subordinate clauses via
//!    grammatical relations (`COMP`, `CSUBJ`, `CMOD`, etc.).
//! 4. Compute per-speaker ratios at finalization.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::{DependentTier, Utterance};

use crate::framework::{
    AnalysisCommand, FileContext, UtteranceLimit, mor_item_has_verb, mor_item_morpheme_count,
};

pub use output::{SpeakerSugar, SugarResult};

/// Configuration for the SUGAR command.
#[derive(Debug, Clone)]
pub struct SugarConfig {
    /// Minimum number of utterances required (default: 50).
    pub min_utterances: UtteranceLimit,
}

impl Default for SugarConfig {
    fn default() -> Self {
        Self {
            min_utterances: UtteranceLimit::new(50),
        }
    }
}

/// Per-speaker accumulated state.
#[derive(Debug, Default)]
struct SpeakerState {
    /// Total morphemes across all utterances.
    morpheme_count: u64,
    /// Total words (tokens with POS tags).
    word_count: u64,
    /// Total complete utterances (sentences ending with . ? !).
    utterance_count: u64,
    /// Utterances containing at least one verb.
    verb_utterance_count: u64,
    /// Words in utterances containing verbs.
    verb_utterance_words: u64,
    /// Clause count from %gra analysis.
    clause_count: u64,
}

/// Accumulated state for SUGAR across all files.
#[derive(Debug, Default)]
pub struct SugarState {
    speakers: BTreeMap<String, SpeakerState>,
}

/// SUGAR command implementation.
///
/// Processes `%mor` and `%gra` tiers per utterance, accumulating
/// morpheme counts, word counts, verb-utterance tracking, and clause
/// counts for per-speaker metric computation at finalization.
pub struct SugarCommand {
    _config: SugarConfig,
}

impl SugarCommand {
    /// Create a new SUGAR command with the given config.
    pub fn new(config: SugarConfig) -> Self {
        Self { _config: config }
    }
}

/// Verb POS tags recognized in the CHAT `%mor` tier.
///
/// Includes both legacy CLAN tags (`v`, `cop`, `mod`) and modern UD tags (`verb`).
/// UD maps copula to `aux` (already included) and modals to `aux`/`verb`.
const VERB_POS: &[&str] = &["v", "verb", "cop", "aux", "mod", "part"];

/// Check if a `%mor` POS tag indicates a verb (including subtypes like `v:aux`).
fn is_verb_pos(pos: &str) -> bool {
    VERB_POS
        .iter()
        .any(|&v| pos == v || pos.starts_with(&format!("{v}:")))
}

/// Count subordinate clause relations from typed `%gra` entries.
///
/// Recognized subordinating relations: `CSUBJ`, `CPRED`, `CPOBJ`,
/// `COBJ`, `CJCT`, `XJCT`, `CMOD`, `XMOD`, `COMP`. Each occurrence
/// adds one clause to the count.
fn count_clauses_from_gra(tier: &talkbank_model::GraTier) -> u64 {
    let mut clauses = 0u64;
    for relation in tier.relations() {
        let relation = relation.relation.to_string().to_uppercase();
        match relation.as_str() {
            "CSUBJ" | "CPRED" | "CPOBJ" | "COBJ" | "CJCT" | "XJCT" | "CMOD" | "XMOD" | "COMP" => {
                clauses += 1;
            }
            _ => {}
        }
    }
    clauses
}

impl AnalysisCommand for SugarCommand {
    type Config = SugarConfig;
    type State = SugarState;
    type Output = SugarResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let speaker = utterance.main.speaker.to_string();
        let speaker_state = state.speakers.entry(speaker).or_default();

        // Count as a complete utterance
        speaker_state.utterance_count += 1;

        // Process %mor tier
        let mut mor_tier = None;
        let mut gra_tier = None;
        let mut has_verb = false;
        let mut word_count = 0u64;
        let mut morph_count = 0u64;

        for dep in &utterance.dependent_tiers {
            match dep {
                DependentTier::Mor(tier) => {
                    mor_tier = Some(tier);
                }
                DependentTier::Gra(tier) => {
                    gra_tier = Some(tier);
                }
                _ => {}
            }
        }

        if let Some(tier) = mor_tier {
            for item in tier.items() {
                word_count += 1;
                morph_count += mor_item_morpheme_count(item);
                if mor_item_has_verb(item, is_verb_pos) {
                    has_verb = true;
                }
            }
        }

        speaker_state.word_count += word_count;
        speaker_state.morpheme_count += morph_count;

        if has_verb {
            speaker_state.verb_utterance_count += 1;
            speaker_state.verb_utterance_words += word_count;

            // Count clauses from %gra if available
            if let Some(gra) = gra_tier {
                // Base clause count: 1 (for the main clause) + subordinate clauses
                speaker_state.clause_count += 1 + count_clauses_from_gra(gra);
            } else {
                // No %gra, assume 1 clause per verb utterance
                speaker_state.clause_count += 1;
            }
        }
    }

    fn finalize(&self, state: Self::State) -> SugarResult {
        let speakers: Vec<SpeakerSugar> = state
            .speakers
            .into_iter()
            .map(|(speaker, s)| {
                let mlu_s = if s.utterance_count > 0 {
                    Some(s.morpheme_count as f64 / s.utterance_count as f64)
                } else {
                    None
                };
                let wps = if s.verb_utterance_count > 0 {
                    Some(s.verb_utterance_words as f64 / s.verb_utterance_count as f64)
                } else {
                    None
                };
                let cps = if s.verb_utterance_count > 0 {
                    Some(s.clause_count as f64 / s.verb_utterance_count as f64)
                } else {
                    None
                };

                SpeakerSugar {
                    speaker,
                    mlu_s,
                    tnw: s.word_count,
                    wps,
                    cps,
                    utterance_count: s.utterance_count,
                    morpheme_count: s.morpheme_count,
                }
            })
            .collect();

        SugarResult { speakers }
    }
}
