//! Typed FREQPOS output and rendering.

use std::fmt::Write;

use serde::Serialize;

use super::PositionClassification;
use crate::framework::CommandOutput;

/// A single word position entry in the output.
#[derive(Debug, Clone, Serialize)]
pub struct FreqposEntry {
    /// The word (normalized).
    pub word: String,
    /// CLAN display form.
    pub display_form: String,
    /// Total occurrences.
    pub total: u64,
    /// Occurrences in initial position.
    pub initial: u64,
    /// Occurrences in the "second slot", meaning depends on the
    /// `position_classification` mode this result was produced
    /// under. `FirstLastOther` (default): the LAST position of a
    /// multi-word utterance (`i == len - 1`). `FirstSecondOther`
    /// (CLAN `+d`): position 1 specifically (`i == 1`). Field name
    /// is `final_pos` for JSON-schema stability across modes;
    /// renderers consult the result's `position_classification`
    /// to label the column "final" vs "second" accordingly.
    pub final_pos: u64,
    /// Occurrences in other (middle) positions.
    pub other: u64,
    /// Occurrences as one-word utterance.
    pub one_word: u64,
}

/// Typed output for the FREQPOS command.
#[derive(Debug, Clone, Serialize)]
pub struct FreqposResult {
    /// Word entries sorted alphabetically by display form.
    pub entries: Vec<FreqposEntry>,
    /// Total words in initial position across all entries.
    pub total_initial: u64,
    /// Total words in other (middle) position.
    pub total_other: u64,
    /// Total words in final position. Under `FirstSecondOther` mode
    /// this counter holds the position-1 ("second") count instead.
    pub total_final: u64,
    /// Total one-word utterances.
    pub total_one_word: u64,
    /// Classification mode used to produce these counts; render
    /// uses this to label `total_final` as "final" or "second".
    pub position_classification: PositionClassification,
}

impl CommandOutput for FreqposResult {
    /// Use CLAN-aligned text as the default textual representation.
    fn render_text(&self) -> String {
        self.render_clan()
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// Format:
    /// ```text
    ///   1  cookie               initial =  0, final =  1, other =  0, one word =  0
    ///
    /// Number of words in an initial position =  3
    /// Number of words in an other position   =  6
    /// Number of words in a final position    =  3
    /// Number of one word utterences          =  1
    /// ```
    fn render_clan(&self) -> String {
        let mut out = String::new();

        // Find the max display form length for alignment.
        // CLAN's freqpos uses a 20-character word-display column.
        let max_display_len = self
            .entries
            .iter()
            .map(|e| e.display_form.len())
            .max()
            .unwrap_or(0)
            .max(20);

        // CLAN labels the position-1 column "final" by default; with
        // `+d` (`FirstSecondOther`), the same column reports a
        // different population (position 1 instead of position
        // `len-1`) and the label becomes "second".
        let second_label = match self.position_classification {
            PositionClassification::FirstLastOther => "final",
            PositionClassification::FirstSecondOther => "second",
        };
        let second_footer_label = match self.position_classification {
            PositionClassification::FirstLastOther => "Number of words in a final position    =",
            PositionClassification::FirstSecondOther => "Number of words in a second position   =",
        };

        for entry in &self.entries {
            writeln!(
                out,
                "{:>3}  {:<width$} initial = {:>2}, {} = {:>2}, other = {:>2}, one word = {:>2}",
                entry.total,
                entry.display_form,
                entry.initial,
                second_label,
                entry.final_pos,
                entry.other,
                entry.one_word,
                width = max_display_len,
            )
            .ok();
        }

        // Position summary footer
        writeln!(out).ok();
        writeln!(
            out,
            "Number of words in an initial position = {:>2}",
            self.total_initial
        )
        .ok();
        writeln!(
            out,
            "Number of words in an other position   = {:>2}",
            self.total_other
        )
        .ok();
        writeln!(out, "{} {:>2}", second_footer_label, self.total_final).ok();
        writeln!(
            out,
            "Number of one word utterences          = {:>2}",
            self.total_one_word
        )
        .ok();

        out
    }
}
