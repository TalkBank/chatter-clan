//! COOCCUR, Word co-occurrence (bigram) counting.
//!
//! Reimplements CLAN's COOCCUR command, which counts adjacent word pairs
//! (bigrams) across utterances. For each utterance, every pair of consecutive
//! countable words is recorded as a directed bigram. Pairs are directional:
//! ("put", "the") and ("the", "put") are counted separately.
//!
//! COOCCUR is part of the FREQ family of commands and is useful for studying
//! word collocations and sequential patterns in speech.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                         | Rust equivalent                                       |
//! |--------------------------------------|-------------------------------------------------------|
//! | `cooccur file.cha`                   | `chatter analyze cooccur file.cha`                    |
//! | `cooccur +t*CHI file.cha`            | `chatter analyze cooccur file.cha -s CHI`             |
//!
//! # Output
//!
//! - Table of adjacent word pairs with co-occurrence counts
//! - Default sort: by frequency descending, then alphabetically
//! - CLAN output: sorted alphabetically by pair display form
//! - Summary: unique pair count, total pair instances, total utterances
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Bigram extraction operates on parsed AST content rather than raw text.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use smallvec::SmallVec;
use talkbank_model::Utterance;

use crate::framework::word_filter::countable_words;
use crate::framework::{
    AnalysisCommand, FileContext, NormalizedWord, UtteranceCount, clan_display_form,
};

pub use output::{CooccurCluster, CooccurResult};

/// Inline storage for the common bigram case. Avoids a heap
/// allocation per `WordCluster`/`ClusterData` when `cluster_size`
/// is 2 (the default and overwhelmingly common case).
type ClusterInline<T> = SmallVec<[T; 2]>;

/// Configuration for the COOCCUR command.
#[derive(Debug, Clone)]
pub struct CooccurConfig {
    /// CLAN `+d`: render output without the leading frequency-
    /// count column. Each row becomes `<words…>` instead of
    /// `count <words…>`.
    pub no_frequency_counts: bool,
    /// CLAN `+nN`: cluster size (number of adjacent words counted
    /// per row). Default `2` = bigrams; `3` = trigrams; etc. Values
    /// below `2` collapse to `2` at use-site so `.windows(N)` never
    /// panics on `0` and the bigram default is the floor.
    pub cluster_size: u8,
}

impl Default for CooccurConfig {
    fn default() -> Self {
        Self {
            no_frequency_counts: false,
            cluster_size: 2,
        }
    }
}

/// An ordered word cluster (N-gram) used as a map key for adjacent
/// word groups. Order is preserved, ("put", "the") and ("the", "put")
/// are distinct keys. `N` matches `CooccurConfig::cluster_size`
/// (default 2 = bigrams).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct WordCluster(ClusterInline<NormalizedWord>);

impl WordCluster {
    /// Test-only helper for constructing cluster keys from string literals.
    #[cfg(test)]
    fn new(words: &[&str]) -> Self {
        WordCluster(
            words
                .iter()
                .map(|w| NormalizedWord(w.to_string()))
                .collect(),
        )
    }
}

/// Per-cluster accumulated data: count and display forms.
#[derive(Debug, Clone)]
struct ClusterData {
    count: u64,
    displays: ClusterInline<String>,
}

/// Accumulated state for COOCCUR across all files.
#[derive(Debug, Default)]
pub struct CooccurState {
    /// Co-occurrence data for each adjacent cluster (merged counts + display forms).
    clusters: BTreeMap<WordCluster, ClusterData>,
    /// Total utterances examined.
    pub total_utterances: UtteranceCount,
}

/// COOCCUR command implementation.
///
/// For each utterance, extracts countable words and counts adjacent pairs
/// (bigrams), matching CLAN's behavior.
#[derive(Debug, Clone, Default)]
pub struct CooccurCommand {
    /// User-facing configuration.
    pub config: CooccurConfig,
}

impl CooccurCommand {
    /// Construct with explicit configuration.
    pub fn new(config: CooccurConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for CooccurCommand {
    type Config = CooccurConfig;
    type State = CooccurState;
    type Output = CooccurResult;

    /// Count adjacent N-grams from the current utterance. `N`
    /// comes from `CooccurConfig::cluster_size` (default 2 =
    /// bigrams; clamped to a floor of 2 to keep `.windows(N)`
    /// well-defined).
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        state.total_utterances += 1;

        let words: Vec<(NormalizedWord, String)> = countable_words(&utterance.main.content.content)
            .map(|w| (NormalizedWord::from_word(w), clan_display_form(w)))
            .collect();

        let cluster_size = (self.config.cluster_size as usize).max(2);
        if words.len() < cluster_size {
            return;
        }
        for window in words.windows(cluster_size) {
            let key = WordCluster(window.iter().map(|(k, _)| k.clone()).collect());
            state
                .clusters
                .entry(key)
                .and_modify(|data| data.count += 1)
                .or_insert_with(|| ClusterData {
                    count: 1,
                    displays: window.iter().map(|(_, d)| d.clone()).collect(),
                });
        }
    }

    /// Materialize sorted output rows and aggregate totals from map state.
    fn finalize(&self, state: Self::State) -> CooccurResult {
        if state.clusters.is_empty() {
            return CooccurResult {
                clusters: Vec::new(),
                unique_clusters: 0,
                total_cluster_instances: 0,
                total_utterances: state.total_utterances,
                no_frequency_counts: self.config.no_frequency_counts,
            };
        }

        let unique_clusters = state.clusters.len();
        let total_cluster_instances: u64 = state.clusters.values().map(|d| d.count).sum();

        // Sort clusters by frequency (descending), then alphabetically.
        let mut sorted: Vec<(WordCluster, ClusterData)> = state.clusters.into_iter().collect();
        sorted.sort_by(|a, b| b.1.count.cmp(&a.1.count).then_with(|| a.0.cmp(&b.0)));

        let clusters: Vec<CooccurCluster> = sorted
            .into_iter()
            .map(|(cluster, data)| CooccurCluster {
                words: cluster
                    .0
                    .into_iter()
                    .map(|nw| nw.as_str().to_owned())
                    .collect(),
                displays: data.displays.into_iter().collect(),
                count: data.count,
            })
            .collect();

        CooccurResult {
            clusters,
            unique_clusters,
            total_cluster_instances,
            total_utterances: state.total_utterances,
            no_frequency_counts: self.config.no_frequency_counts,
        }
    }
}
