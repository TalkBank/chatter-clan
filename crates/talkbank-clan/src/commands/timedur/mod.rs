//! TIMEDUR, Time Duration from Bullets.
//!
//! Computes time duration statistics from media timestamp bullets
//! (`\x15start_end\x15`) attached to utterances. Utterances without
//! bullet timing are silently skipped.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409240)
//! for the original TIMEDUR command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                 | Rust equivalent                            |
//! |------------------------------|--------------------------------------------|
//! | `timedur file.cha`           | `chatter analyze timedur file.cha`         |
//! | `timedur +t*CHI file.cha`    | `chatter analyze timedur file.cha -s CHI`  |
//!
//! # Output
//!
//! Per speaker:
//! - Number of timed utterances
//! - Total duration
//! - Mean utterance duration
//! - Min/max duration
//!
//! Plus a corpus-wide summary with total timed utterances, total duration,
//! and recording span (earliest start to latest end).
//!
//! # Differences from CLAN
//!
//! - Timestamp extraction uses parsed media bullet structures from the
//!   AST rather than raw `\x15` byte scanning in text.
//! - Duration computation operates on typed timestamp values.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use indexmap::IndexMap;
use talkbank_model::Utterance;

use crate::framework::{AnalysisCommand, FileContext};

pub use output::{TimedurResult, TimedurSpeakerResult, TimedurSummary};

/// Duration in milliseconds.
type DurationMs = u64;

/// Configuration for the TIMEDUR command.
#[derive(Debug, Clone, Default)]
pub struct TimedurConfig {}

/// Per-speaker timing data accumulated during processing.
#[derive(Debug, Default)]
struct SpeakerTiming {
    /// Duration of each timed utterance in milliseconds
    durations_ms: Vec<DurationMs>,
}

/// Accumulated state for TIMEDUR across all files.
#[derive(Debug, Default)]
pub struct TimedurState {
    /// Per-speaker timing data, keyed by speaker code string
    by_speaker: IndexMap<String, SpeakerTiming>,
    /// All speakers seen in encounter order (includes speakers with no bullet timings).
    seen_speakers: Vec<String>,
    /// Total time span across all speakers (earliest start, latest end)
    earliest_start_ms: Option<DurationMs>,
    latest_end_ms: Option<DurationMs>,
}

/// TIMEDUR command implementation.
///
/// Extracts bullet timing from `utterance.main.content.bullet` and
/// computes duration statistics per speaker.
#[derive(Debug, Clone, Default)]
pub struct TimedurCommand;

impl AnalysisCommand for TimedurCommand {
    type Config = TimedurConfig;
    type State = TimedurState;
    type Output = TimedurResult;

    /// Record one utterance duration from bullet timings when present.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // Track all speakers in encounter order, even without bullet timings.
        let speaker_str = utterance.main.speaker.as_str();
        if !state.seen_speakers.iter().any(|s| s == speaker_str) {
            state.seen_speakers.push(speaker_str.to_owned());
        }

        let Some(ref bullet) = utterance.main.content.bullet else {
            return;
        };

        let start = bullet.timing.start_ms;
        let end = bullet.timing.end_ms;
        let duration = end.saturating_sub(start);

        let speaker = utterance.main.speaker.as_str().to_owned();
        let speaker_data = state
            .by_speaker
            .entry(speaker)
            .or_insert_with(SpeakerTiming::default);

        speaker_data.durations_ms.push(duration);

        // Track overall time span
        state.earliest_start_ms = Some(
            state
                .earliest_start_ms
                .map_or(start, |prev| prev.min(start)),
        );
        state.latest_end_ms = Some(state.latest_end_ms.map_or(end, |prev| prev.max(end)));
    }

    /// Compute per-speaker aggregates and optional corpus-wide timing summary.
    fn finalize(&self, state: Self::State) -> TimedurResult {
        let mut speakers = Vec::new();
        for (speaker, data) in &state.by_speaker {
            if data.durations_ms.is_empty() {
                continue;
            }
            let n = data.durations_ms.len() as u64;
            let total_ms: u64 = data.durations_ms.iter().sum();
            let mean_ms = total_ms / n;
            let min_ms = data.durations_ms.iter().copied().min().unwrap_or(0);
            let max_ms = data.durations_ms.iter().copied().max().unwrap_or(0);
            speakers.push(TimedurSpeakerResult {
                speaker: speaker.clone(),
                timed_utterances: n,
                total_ms,
                mean_ms,
                min_ms,
                max_ms,
            });
        }

        let summary =
            if let (Some(start), Some(end)) = (state.earliest_start_ms, state.latest_end_ms) {
                let span_ms = end.saturating_sub(start);
                let total_ms: u64 = state
                    .by_speaker
                    .values()
                    .flat_map(|d| d.durations_ms.iter())
                    .sum();
                let total_utterances: usize = state
                    .by_speaker
                    .values()
                    .map(|d| d.durations_ms.len())
                    .sum();
                Some(TimedurSummary {
                    total_utterances,
                    total_ms,
                    span_ms,
                })
            } else {
                None
            };

        TimedurResult {
            speakers,
            summary,
            seen_speakers: state.seen_speakers,
        }
    }
}
