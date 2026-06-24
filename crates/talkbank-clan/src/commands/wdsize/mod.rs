//! WDSIZE, Word size (character length) histogram from `%mor` tier stems.
//!
//! By default WDSIZE uses the `%mor` tier to extract word stems and counts
//! their character lengths. This differs from WDLEN which counts main tier
//! word lengths. When `%mor` is unavailable, falls back to main tier words.
//!
//! Output: character-length histogram per speaker with mean word size.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) for the
//! original WDSIZE command specification.
//!
//! # Differences from CLAN
//!
//! - Uses typed `MorTier` items with `MorWord.lemma` rather than raw string
//!   parsing of `%mor` tier text.
//! - Compound words concatenate all compound lemmas (matching CLAN behavior).
//! - Supports JSON and CSV output in addition to text/XLS.
//! - Optional `--main-tier` flag to count main tier words instead of stems.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::Utterance;

use crate::framework::word_filter::{countable_words, has_countable_words};
use crate::framework::{AnalysisCommand, FileContext};

pub use output::{WdsizeDistribution, WdsizeResult};

/// Length-comparison used by WDSIZE's `+w[>|<|=]N` filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthComparator {
    /// CLAN `+w>N`, include only words with length > N.
    GreaterThan,
    /// CLAN `+w<N`, include only words with length < N.
    LessThan,
    /// CLAN `+w=N`, include only words with length == N.
    Equal,
}

/// Optional per-word length predicate for WDSIZE (CLAN `+w[>|<|=]N`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthFilter {
    /// Comparison applied to each word's character length.
    pub comparator: LengthComparator,
    /// Threshold; the right-hand side of the comparison.
    pub threshold: usize,
}

impl LengthFilter {
    /// Whether the given length passes this filter.
    pub fn includes(self, length: usize) -> bool {
        match self.comparator {
            LengthComparator::GreaterThan => length > self.threshold,
            LengthComparator::LessThan => length < self.threshold,
            LengthComparator::Equal => length == self.threshold,
        }
    }
}

/// Parse `gt:N` / `lt:N` / `eq:N` into a `LengthFilter`. Returns
/// `None` for any other shape.
impl std::str::FromStr for LengthFilter {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (tag, n) = s
            .split_once(':')
            .ok_or_else(|| format!("expected `<gt|lt|eq>:<N>`, got {s:?}"))?;
        let comparator = match tag {
            "gt" => LengthComparator::GreaterThan,
            "lt" => LengthComparator::LessThan,
            "eq" => LengthComparator::Equal,
            other => return Err(format!("unknown length comparator: {other:?}")),
        };
        let threshold = n
            .parse::<usize>()
            .map_err(|_| format!("invalid threshold: {n:?}"))?;
        Ok(LengthFilter {
            comparator,
            threshold,
        })
    }
}

/// Configuration for the WDSIZE command.
#[derive(Debug, Clone, Default)]
pub struct WdsizeConfig {
    /// Use main tier words instead of `%mor` stems.
    pub use_main_tier: bool,
    /// CLAN `+w[>|<|=]N`: include only words whose character
    /// length satisfies the comparison. `None` ⇒ no filter.
    pub length_filter: Option<LengthFilter>,
}

/// Per-speaker accumulator.
#[derive(Debug, Default)]
struct SpeakerAccum {
    distribution: BTreeMap<usize, u64>,
    total_words: u64,
    total_chars: u64,
}

impl SpeakerAccum {
    fn record(&mut self, char_len: usize) {
        *self.distribution.entry(char_len).or_insert(0) += 1;
        self.total_words += 1;
        self.total_chars += char_len as u64;
    }

    fn into_distribution(self, speaker: &str) -> WdsizeDistribution {
        WdsizeDistribution {
            speaker: speaker.to_owned(),
            distribution: self.distribution,
            total_words: self.total_words,
            total_chars: self.total_chars,
        }
    }
}

/// Accumulated state for WDSIZE.
#[derive(Debug, Default)]
pub struct WdsizeState {
    by_speaker: indexmap::IndexMap<String, SpeakerAccum>,
}

/// WDSIZE command implementation.
#[derive(Debug, Clone)]
pub struct WdsizeCommand {
    config: WdsizeConfig,
}

impl WdsizeCommand {
    /// Create a new WDSIZE command with configuration.
    pub fn new(config: WdsizeConfig) -> Self {
        Self { config }
    }
}

impl Default for WdsizeCommand {
    fn default() -> Self {
        Self::new(WdsizeConfig::default())
    }
}

impl AnalysisCommand for WdsizeCommand {
    type Config = WdsizeConfig;
    type State = WdsizeState;
    type Output = WdsizeResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        if !has_countable_words(&utterance.main.content.content) {
            return;
        }

        let speaker = utterance.main.speaker.to_string();
        let accum = state.by_speaker.entry(speaker).or_default();

        // `+w[>|<|=]N` (`length_filter`) gates each character
        // length before it enters the histogram. `None` ⇒ accept.
        let length_filter = self.config.length_filter;
        let record_if_passes = |accum: &mut SpeakerAccum, char_len: usize| {
            if length_filter.is_none_or(|f| f.includes(char_len)) {
                accum.record(char_len);
            }
        };

        if self.config.use_main_tier {
            // Count main tier word character lengths
            for word in countable_words(&utterance.main.content.content) {
                let char_len = word.cleaned_text().chars().count();
                record_if_passes(accum, char_len);
            }
        } else if let Some(mor_tier) = utterance.mor_tier() {
            // Count %mor lemma character lengths (default behavior)
            for mor_item in mor_tier.items().iter() {
                let char_len = mor_item.main.lemma.as_str().chars().count();
                record_if_passes(accum, char_len);

                // Count clitic lemmas separately
                for clitic in &mor_item.post_clitics {
                    let char_len = clitic.lemma.as_str().chars().count();
                    record_if_passes(accum, char_len);
                }
            }
        } else {
            // Fallback to main tier if no %mor
            for word in countable_words(&utterance.main.content.content) {
                let char_len = word.cleaned_text().chars().count();
                record_if_passes(accum, char_len);
            }
        }
    }

    fn finalize(&self, state: Self::State) -> WdsizeResult {
        let speakers: Vec<_> = state
            .by_speaker
            .into_iter()
            .map(|(speaker, accum)| accum.into_distribution(&speaker))
            .collect();

        WdsizeResult { speakers }
    }
}
