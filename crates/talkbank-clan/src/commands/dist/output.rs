use serde::Serialize;

use crate::framework::{CommandOutput, TurnCount, WordCount};

/// Distribution stats for a single word.
#[derive(Debug, Clone, Serialize)]
pub struct DistWordEntry {
    /// The word (lowercased).
    pub word: String,
    /// CLAN display form (preserves `+` in compounds).
    pub display_form: String,
    /// Total occurrences across all turns.
    pub total_count: WordCount,
    /// First turn number (1-based) in which this word appears.
    pub first_turn: TurnCount,
    /// Last turn number (1-based) in which this word appears.
    pub last_turn: TurnCount,
    /// Average distance = (last_turn - first_turn) / total_count.
    /// Only present when total_count >= 2.
    pub average_distance: Option<f64>,
}

/// Typed output for the DIST command.
#[derive(Debug, Clone, Serialize)]
pub struct DistResult {
    /// Total number of turns (one per utterance).
    pub total_turns: TurnCount,
    /// Word entries sorted alphabetically by display form.
    pub words: Vec<DistWordEntry>,
}

impl CommandOutput for DistResult {
    /// Use CLAN-formatted output as the default text representation.
    fn render_text(&self) -> String {
        self.render_clan()
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// Format:
    /// ```text
    /// There were 4 turns.
    ///
    ///
    ///                  Occurrence   First    Last        Average
    /// Word                  Count   Occurs   Occurs      Distance
    /// -----------------------------------------------------------
    /// choo+choo's              1        4
    /// cookie                   1        1
    /// ```
    fn render_clan(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        writeln!(out, "There were {} turns.", self.total_turns).ok();
        writeln!(out).ok();
        writeln!(out).ok();

        // Header
        writeln!(
            out,
            "                 Occurrence   First    Last        Average "
        )
        .ok();
        writeln!(
            out,
            "Word                  Count   Occurs   Occurs      Distance"
        )
        .ok();
        writeln!(
            out,
            "-----------------------------------------------------------"
        )
        .ok();

        for entry in &self.words {
            // CLAN's dist uses an 11-char-wide Average Distance column
            // (`{:>11.4}`, observed against the legacy binary), so the
            // gap from the previous 5-wide column is 5 leading spaces +
            // 6-char float. The other columns stay 5-wide with 4-space
            // gaps.
            if let Some(avg_dist) = entry.average_distance {
                writeln!(
                    out,
                    "{:<20} {:>5}    {:>5}    {:>5}    {:>11.4}",
                    entry.display_form,
                    entry.total_count,
                    entry.first_turn,
                    entry.last_turn,
                    avg_dist,
                )
                .ok();
            } else {
                writeln!(
                    out,
                    "{:<20} {:>5}    {:>5}",
                    entry.display_form, entry.total_count, entry.first_turn,
                )
                .ok();
            }
        }

        out
    }
}
