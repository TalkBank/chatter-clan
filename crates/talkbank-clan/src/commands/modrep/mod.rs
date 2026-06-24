//! MODREP, Model/replica comparison from `%mod` and `%pho` tiers.
//!
//! Compares the model (target) pronunciation on the `%mod` tier with the
//! actual (replica) pronunciation on the `%pho` tier, tracking word-by-word
//! mappings between model forms and replica forms. This is used in
//! phonological analysis to assess how closely a speaker's productions
//! match the adult target forms.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409226)
//! for the original MODREP command specification.
//!
//! # Algorithm
//!
//! 1. For each utterance with both `%mod` and `%pho` tiers:
//!    - Extract word lists from both tiers (flattening groups).
//!    - Pair words positionally (model word N <-> replica word N).
//!    - Record each (model, replica) pair in a frequency map per speaker.
//! 2. Report per-speaker tables of model words with their replica variants
//!    and frequency counts.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                                | Rust equivalent                           |
//! |---------------------------------------------|-------------------------------------------|
//! | `modrep +b%mod +c%pho file.cha`             | `chatter analyze modrep file.cha`         |
//! | `modrep +b%mod +c%pho +t*CHI file.cha`      | `chatter analyze modrep file.cha -s CHI`  |
//!
//! # Output
//!
//! Per-speaker listing of model words, each with its set of replica variants
//! and their frequency counts, sorted alphabetically by model word.
//!
//! # Differences from CLAN
//!
//! - Model and replica extraction uses parsed `%mod` and `%pho` tier
//!   structures from the AST rather than raw text line parsing.
//! - Word pairing operates on typed `PhoWord` content rather than
//!   string splitting.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use indexmap::IndexMap;
use talkbank_model::{PhoItem, PhoWord, SpeakerCode, Utterance};

use crate::framework::{AnalysisCommand, FileContext};

pub use output::{ModelEntry, ModrepResult, ModrepSpeakerResult, ReplicaEntry};

/// Configuration for the MODREP command.
#[derive(Debug, Clone, Default)]
pub struct ModrepConfig {}

/// Accumulated replica variants for a single model word.
#[derive(Debug, Default)]
struct ModelWordData {
    /// Total occurrences of this model word.
    total: u64,
    /// Replica word → count (BTreeMap for alphabetical ordering).
    replicas: BTreeMap<String, u64>,
}

/// Per-speaker accumulated data.
#[derive(Debug, Default)]
struct SpeakerData {
    /// Model word → replica data (BTreeMap for alphabetical model word ordering).
    models: BTreeMap<String, ModelWordData>,
}

/// Accumulated state for MODREP across all files.
#[derive(Debug, Default)]
pub struct ModrepState {
    /// Per-speaker model/replica data, keyed by speaker code.
    by_speaker: IndexMap<SpeakerCode, SpeakerData>,
}

/// MODREP command: compare `%mod` and `%pho` tiers word-by-word.
///
/// Requires both tiers to be present on an utterance; utterances missing
/// either tier are silently skipped. When tiers have unequal lengths,
/// pairing stops at the shorter tier (`zip` truncation).
#[derive(Default)]
pub struct ModrepCommand;

impl ModrepCommand {
    /// Create a new `ModrepCommand` with the given configuration.
    pub fn new(_config: ModrepConfig) -> Self {
        Self
    }
}

impl AnalysisCommand for ModrepCommand {
    type Config = ModrepConfig;
    type State = ModrepState;
    type Output = ModrepResult;

    /// Compare aligned `%mod` and `%pho` token streams for one utterance.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // Need both %mod and %pho tiers
        let mod_tier = match utterance.mod_tier() {
            Some(t) if t.is_mod() => t,
            _ => return,
        };
        let pho_tier = match utterance.pho_tier() {
            Some(t) if t.is_pho() => t,
            _ => return,
        };

        // Flatten both tiers into word lists
        let mod_words = flatten_pho_items(&mod_tier.items);
        let pho_words = flatten_pho_items(&pho_tier.items);

        let speaker_data = state
            .by_speaker
            .entry(utterance.main.speaker.clone())
            .or_default();

        // Pair words positionally (zip truncates to shorter tier)
        for (model_word, replica_word) in mod_words.iter().zip(pho_words.iter()) {
            let model_str = model_word.as_str().to_lowercase();
            let replica_str = replica_word.as_str().to_lowercase();

            let entry = speaker_data.models.entry(model_str).or_default();
            entry.total += 1;
            *entry.replicas.entry(replica_str).or_insert(0) += 1;
        }
    }

    /// Materialize sorted per-speaker model/replica frequency tables.
    fn finalize(&self, state: Self::State) -> Self::Output {
        let speakers = state
            .by_speaker
            .into_iter()
            .map(|(speaker_code, speaker_data)| {
                let entries = speaker_data
                    .models
                    .into_iter()
                    .map(|(model, data)| {
                        let replicas = data
                            .replicas
                            .into_iter()
                            .map(|(word, count)| ReplicaEntry { word, count })
                            .collect();
                        ModelEntry {
                            model,
                            total: data.total,
                            replicas,
                        }
                    })
                    .collect();
                ModrepSpeakerResult {
                    speaker: speaker_code.to_string(),
                    entries,
                }
            })
            .collect();

        ModrepResult { speakers }
    }
}

/// Flatten PhoItems into a simple list of PhoWords.
///
/// Groups are expanded into their constituent words.
///
/// # Postcondition
///
/// The returned list preserves original tier order after expanding groups.
fn flatten_pho_items(items: &[PhoItem]) -> Vec<&PhoWord> {
    let mut words = Vec::new();
    for item in items {
        match item {
            PhoItem::Word(word) => words.push(word),
            PhoItem::Group(group) => {
                for word in group.iter() {
                    words.push(word);
                }
            }
        }
    }
    words
}
