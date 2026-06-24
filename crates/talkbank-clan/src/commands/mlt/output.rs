use std::fmt;

use serde::Serialize;

use crate::framework::{CommandOutput, TurnCount, UtteranceCount, WordCount};

/// Typed output from the MLT command.
///
/// Contains per-speaker turn statistics with strongly-typed numeric fields.
#[derive(Debug, Clone, Serialize)]
pub struct MltResult {
    /// Per-speaker MLT statistics, in encounter order.
    pub speakers: Vec<MltSpeakerResult>,
}

/// MLT statistics for a single speaker.
#[derive(Debug, Clone, Serialize)]
pub struct MltSpeakerResult {
    /// Speaker code (e.g., "CHI", "MOT")
    pub speaker: String,
    /// Number of turns
    pub turns: TurnCount,
    /// Total utterances across all turns
    pub utterances: UtteranceCount,
    /// Total words across all turns
    pub words: WordCount,
    /// Mean words per turn (words / turns)
    pub mlt_words: f64,
    /// Mean utterances per turn (utterances / turns)
    pub mlt_utterances: f64,
    /// Mean words per utterance (words / utterances)
    pub words_per_utterance: f64,
    /// Population standard deviation of words-per-utterance (NaN when utterances <= 1)
    pub sd: f64,
}

impl CommandOutput for MltResult {
    /// Our clean text format.
    fn render_text(&self) -> String {
        let mut out = String::new();
        for (i, s) in self.speakers.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            fmt::write(
                &mut out,
                format_args!(
                    "Speaker: {}\n\
                     \x20 Turns: {}\n\
                     \x20 Total utterances: {}\n\
                     \x20 Total words: {}\n\
                     \x20 MLT (utterances): {:.3}\n\
                     \x20 MLT (words): {:.3}\n",
                    s.speaker, s.turns, s.utterances, s.words, s.mlt_utterances, s.mlt_words
                ),
            )
            .ok();
        }
        out
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// Format (from CLAN snapshot):
    /// ```text
    /// MLT for Speaker: *CHI:
    ///   MLT (xxx, yyy and www are EXCLUDED from the word counts, but are INCLUDED in utterance counts):
    ///     Number of: utterances = 2, turns = 2, words = 3
    /// \tRatio of words over turns = 1.500
    /// \tRatio of utterances over turns = 1.000
    /// \tRatio of words over utterances = 1.500
    /// \tStandard deviation = 0.500
    /// ```
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for (i, s) in self.speakers.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let sd_display = if s.sd.is_nan() {
                "NA".to_owned()
            } else {
                format!("{:.3}", s.sd)
            };
            fmt::write(
                &mut out,
                format_args!(
                    "MLT for Speaker: *{}:\n\
                     \x20 MLT (xxx, yyy and www are EXCLUDED from the word counts, but are INCLUDED in utterance counts):\n\
                     \x20   Number of: utterances = {}, turns = {}, words = {}\n\
                     \tRatio of words over turns = {:.3}\n\
                     \tRatio of utterances over turns = {:.3}\n\
                     \tRatio of words over utterances = {:.3}\n\
                     \tStandard deviation = {}\n",
                    s.speaker,
                    s.utterances,
                    s.turns,
                    s.words,
                    s.mlt_words,
                    s.mlt_utterances,
                    s.words_per_utterance,
                    sd_display,
                ),
            )
            .ok();
        }
        // CLAN emits a trailing blank line after the last per-speaker
        // block; match that so a hex-level diff against legacy mlt
        // output ends cleanly.
        if !self.speakers.is_empty() {
            out.push('\n');
        }
        out
    }
}
