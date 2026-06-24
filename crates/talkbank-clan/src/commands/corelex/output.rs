// `writeln!` to `String` is infallible. See
// `talkbank-clan/src/converters/praat2chat.rs` for the convention.
#![allow(clippy::unwrap_used)]

use std::fmt::Write;

use serde::Serialize;

use crate::framework::{AnalysisScore, CommandOutput, SpeakerCount, TypeCount};

/// A word entry in the core vocabulary report.
#[derive(Debug, Clone, Serialize)]
pub struct CorelexEntry {
    /// The word.
    pub word: String,
    /// Total frequency across all speakers.
    pub frequency: u64,
    /// Number of speakers who used this word.
    pub speaker_count: SpeakerCount,
}

/// Result of the CORELEX command.
#[derive(Debug, Clone, Serialize)]
pub struct CorelexResult {
    /// Words meeting the core vocabulary threshold.
    pub core: Vec<CorelexEntry>,
    /// Words below the core vocabulary threshold.
    pub non_core: Vec<CorelexEntry>,
    /// Total unique words.
    pub total_types: TypeCount,
    /// Number of core words.
    pub core_count: TypeCount,
    /// Number of non-core words.
    pub non_core_count: TypeCount,
    /// Core vocabulary percentage.
    pub core_percentage: AnalysisScore,
    /// Minimum frequency threshold used.
    pub threshold: u64,
}

impl CommandOutput for CorelexResult {
    fn render_text(&self) -> String {
        let mut out = String::new();
        writeln!(out, "Core Vocabulary (frequency >= {}):", self.threshold).unwrap();
        writeln!(
            out,
            "  {} core / {} total = {:.1}%",
            self.core_count, self.total_types, self.core_percentage
        )
        .unwrap();
        writeln!(out).unwrap();

        writeln!(out, "Core words:").unwrap();
        for entry in &self.core {
            writeln!(out, "  {:>4}  {}", entry.frequency, entry.word).unwrap();
        }

        if !self.non_core.is_empty() {
            writeln!(out).unwrap();
            writeln!(out, "Non-core words:").unwrap();
            for entry in &self.non_core {
                writeln!(out, "  {:>4}  {}", entry.frequency, entry.word).unwrap();
            }
        }

        out
    }

    fn render_clan(&self) -> String {
        self.render_text()
    }
}
