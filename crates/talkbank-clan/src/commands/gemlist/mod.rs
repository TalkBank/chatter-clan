// `unreachable!()` arms used here are guarded by `matches!` checks
// in the surrounding `match` patterns, the let-else cannot fail.
#![allow(clippy::unwrap_used, clippy::unreachable)]

//! GEMLIST, List Gem Segments.
//!
//! Lists all gem segments (`@Bg`/`@Eg` bracketed regions) found in CHAT files,
//! reporting the label, utterance count, and participating speakers for each gem.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409206)
//! for the original GEM command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                    | Rust equivalent                                |
//! |---------------------------------|------------------------------------------------|
//! | `gem file.cha`                  | `chatter analyze gemlist file.cha`             |
//! | `gem +t*CHI file.cha`           | `chatter analyze gemlist file.cha -s CHI`      |
//!
//! # Output
//!
//! Per gem label:
//! - Number of utterances within the gem scope
//! - Number of occurrences (how many `@Bg`/`@Eg` pairs with this label)
//! - Speakers who produced utterances within the gem
//! - Source files containing this gem
//!
//! # Implementation Note
//!
//! Gem boundaries (`@Bg`/`@Eg`) are interleaved headers in `ChatFile.lines`.
//! Since the parser does not populate `Utterance.preceding_headers`, this
//! command scans the full line array in `end_file()` rather than relying
//! on per-utterance callbacks.
//!
//! # Differences from CLAN
//!
//! - Gem boundary detection operates on parsed `Header` variants from the
//!   AST rather than raw text line matching.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use indexmap::IndexMap;
use talkbank_model::{Header, Line, Utterance};

use crate::framework::{AnalysisCommand, FileContext};

pub use output::{GemEntry, GemlistResult};

/// Configuration for the GEMLIST command.
#[derive(Debug, Clone, Default)]
pub struct GemlistConfig {}

/// Accumulated data for a single gem label (internal).
#[derive(Debug, Default)]
struct GemInfo {
    /// Total utterances within all instances of this gem.
    utterance_count: u64,
    /// Number of distinct @Bg occurrences for this label.
    occurrence_count: u64,
    /// Speaker codes who produced utterances within this gem.
    speakers: HashSet<String>,
    /// Files containing this gem.
    files: HashSet<String>,
}

/// Accumulated state for GEMLIST across all files.
#[derive(Debug, Default)]
pub struct GemlistState {
    /// Per-label accumulated gem data.
    by_label: IndexMap<String, GemInfo>,
}

/// GEMLIST command implementation.
///
/// Scans `ChatFile.lines` in `end_file()` to find `@Bg`/`@Eg` boundaries
/// and count utterances within each gem scope. This is necessary because
/// the parser stores gem headers as separate `Line::Header` entries rather
/// than attaching them to `Utterance.preceding_headers`.
#[derive(Debug, Clone, Default)]
pub struct GemlistCommand;

impl AnalysisCommand for GemlistCommand {
    type Config = GemlistConfig;
    type State = GemlistState;
    type Output = GemlistResult;

    /// No-op: gem scope tracking runs in `end_file()` over raw line sequence.
    fn process_utterance(
        &self,
        _utterance: &Utterance,
        _file_context: &FileContext<'_>,
        _state: &mut Self::State,
    ) {
        // Gem tracking is done in end_file() by scanning ChatFile.lines directly,
        // because the parser does not populate Utterance.preceding_headers.
    }

    /// Scan interleaved header/utterance lines to accumulate gem boundary stats.
    fn end_file(&self, file_context: &FileContext<'_>, state: &mut Self::State) {
        // Track currently active gem labels (stack for nested gems)
        let mut active_gems: Vec<String> = Vec::new();
        // Track which @Bg labels we've already counted in this file
        let mut seen_begins: HashSet<String> = HashSet::new();

        for line in &file_context.chat_file.lines {
            match line {
                Line::Header { header, .. }
                    if matches!(header.as_ref(), Header::BeginGem { label: Some(_) }) =>
                {
                    let Header::BeginGem { label: Some(label) } = header.as_ref() else {
                        unreachable!()
                    };
                    let label_str = label.as_str().to_owned();
                    active_gems.push(label_str.clone());

                    // Count this as a new occurrence of this gem label
                    let key = format!("{}:{}", file_context.filename, &label_str);
                    if seen_begins.insert(key) {
                        state
                            .by_label
                            .entry(label_str)
                            .or_default()
                            .occurrence_count += 1;
                    }
                }
                Line::Header { header, .. }
                    if matches!(header.as_ref(), Header::EndGem { label: Some(_) }) =>
                {
                    let Header::EndGem { label: Some(label) } = header.as_ref() else {
                        unreachable!()
                    };
                    // Remove the most recent matching @Bg (LIFO)
                    if let Some(pos) = active_gems
                        .iter()
                        .rposition(|g| g.eq_ignore_ascii_case(label.as_str()))
                    {
                        active_gems.remove(pos);
                    }
                }
                Line::Utterance(utterance) if !active_gems.is_empty() => {
                    let speaker = utterance.main.speaker.as_str().to_owned();
                    let filename = file_context.filename.to_owned();

                    for gem_label in &active_gems {
                        let info = state.by_label.entry(gem_label.clone()).or_default();
                        info.utterance_count += 1;
                        info.speakers.insert(speaker.clone());
                        info.files.insert(filename.clone());
                    }
                }
                _ => {}
            }
        }
    }

    /// Materialize per-label gem rows with corpus-wide totals.
    fn finalize(&self, state: Self::State) -> GemlistResult {
        if state.by_label.is_empty() {
            return GemlistResult {
                gems: Vec::new(),
                total_occurrences: 0,
                total_utterances: 0,
            };
        }

        let total_utterances: u64 = state.by_label.values().map(|i| i.utterance_count).sum();
        let total_occurrences: u64 = state.by_label.values().map(|i| i.occurrence_count).sum();

        let gems: Vec<GemEntry> = state
            .by_label
            .into_iter()
            .map(|(label, info)| {
                let mut speakers: Vec<String> = info.speakers.into_iter().collect();
                speakers.sort();
                GemEntry {
                    label,
                    occurrences: info.occurrence_count,
                    utterance_count: info.utterance_count,
                    speakers,
                }
            })
            .collect();

        GemlistResult {
            gems,
            total_occurrences,
            total_utterances,
        }
    }
}
