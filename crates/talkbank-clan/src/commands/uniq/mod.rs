//! UNIQ, Report repeated lines with frequency counts.
//!
//! Identifies and counts duplicate lines (both @header and *speaker
//! utterance lines, lowercased) across all input files. Matches CLAN
//! behavior of including all line types in the frequency table.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409094)
//! for related CLAN command specifications.
//!
//! # CLAN Equivalence
//!
//! | CLAN command               | Rust equivalent                         |
//! |----------------------------|-----------------------------------------|
//! | `uniq file.cha`            | `chatter clan uniq file.cha`            |
//! | `uniq -o file.cha`         | `chatter clan uniq file.cha --sort`     |
//!
//! # Output
//!
//! - Table of unique line texts with frequency counts (headers + utterances)
//! - Total lines processed and number of unique lines
//! - Optional frequency-descending sort (CLAN `-o` flag)
//!
//! # Differences from CLAN
//!
//! - Line identity is based on normalized rendered CHAT lines from the AST,
//!   rather than raw source text line reading.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::{Utterance, WriteChat};

use crate::framework::{AnalysisCommand, FileContext};

pub use output::{UniqEntry, UniqResult};

/// Configuration for the UNIQ command.
#[derive(Debug, Clone, Default)]
pub struct UniqConfig {
    /// Sort output by descending frequency (CLAN `-o` flag).
    pub sort_by_frequency: bool,
}

/// Accumulated state for UNIQ across all files.
///
/// Counts normalized rendered CHAT lines (all lowercased), matching the
/// command's semantic intent: repeated line texts after AST normalization.
#[derive(Debug, Default)]
pub struct UniqState {
    /// Lowercased line text → count (BTreeMap for sorted alphabetical output).
    counts: BTreeMap<String, u64>,
    /// Total lines processed (headers + utterances).
    total: u64,
}

/// UNIQ command implementation.
///
/// Accumulates lowercased rendered line text in a frequency map. The command
/// works over normalized file lines in `end_file()` because its semantic unit
/// is the serialized CHAT line, not a structural field subset.
pub struct UniqCommand {
    config: UniqConfig,
}

impl UniqCommand {
    /// Create a new UNIQ command with the given config.
    pub fn new(config: UniqConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for UniqCommand {
    type Config = UniqConfig;
    type State = UniqState;
    type Output = UniqResult;

    fn process_utterance(
        &self,
        _utterance: &Utterance,
        _file_context: &FileContext<'_>,
        _state: &mut Self::State,
    ) {
    }

    fn end_file(&self, file_context: &FileContext<'_>, state: &mut Self::State) {
        for line in file_context.chat_file.lines.iter() {
            let rendered = line.to_chat_string();
            for part in rendered.lines() {
                let lowered = part.trim().to_lowercase();
                if !lowered.is_empty() {
                    state.total += 1;
                    *state.counts.entry(lowered).or_insert(0) += 1;
                }
            }
        }
    }

    fn finalize(&self, state: Self::State) -> UniqResult {
        let unique = state.counts.len() as u64;
        let mut entries: Vec<UniqEntry> = state
            .counts
            .into_iter()
            .map(|(text, count)| UniqEntry { text, count })
            .collect();

        if self.config.sort_by_frequency {
            entries.sort_by(|a, b| b.count.cmp(&a.count).then(a.text.cmp(&b.text)));
        }

        UniqResult {
            entries,
            total: state.total,
            unique,
        }
    }
}
