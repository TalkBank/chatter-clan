//! Typed WDLEN results and rendering logic.

use std::collections::BTreeMap;
use std::fmt::Write;

use indexmap::IndexMap;
use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// A single speaker's distribution for one section.
#[derive(Debug, Clone, Serialize)]
pub struct WdlenDistribution {
    /// Speaker code (e.g. "MOT", "CHI").
    pub speaker: String,
    /// Value -> count mapping (sorted by value for deterministic output).
    pub distribution: BTreeMap<usize, u64>,
    /// Total number of items (for mean denominator).
    pub total_items: u64,
    /// Sum of all values (for mean numerator).
    pub total_value: u64,
}

impl WdlenDistribution {
    pub(crate) fn mean(&self) -> f64 {
        if self.total_items == 0 {
            0.0
        } else {
            self.total_value as f64 / self.total_items as f64
        }
    }
}

/// Typed output for the WDLEN command -- 6 distribution sections.
#[derive(Debug, Clone, Serialize)]
pub struct WdlenResult {
    /// Section 1: word lengths in characters.
    pub word_lengths: Vec<WdlenDistribution>,
    /// Section 2: utterance lengths in words.
    pub utt_word_lengths: Vec<WdlenDistribution>,
    /// Section 3: turn lengths in utterances.
    pub turn_utt_lengths: Vec<WdlenDistribution>,
    /// Section 4: turn lengths in words.
    pub turn_word_lengths: Vec<WdlenDistribution>,
    /// Section 5: word lengths in morphemes.
    pub morph_lengths: Vec<WdlenDistribution>,
    /// Section 6: utterance lengths in morphemes.
    pub utt_morph_lengths: Vec<WdlenDistribution>,
}

/// Render one distribution section in CLAN table format.
///
/// CLAN uses fixed 5-char right-justified columns for all values.
/// The label field width is `max("lengths".len(), max("*SPK:".len())) + 1`.
fn render_section(out: &mut String, title: &str, distributions: &[WdlenDistribution]) {
    let _ = writeln!(out, "{title}");

    // Find the max length value across all speakers in this section.
    let max_len = distributions
        .iter()
        .flat_map(|d| d.distribution.keys())
        .copied()
        .max()
        .unwrap_or(1);

    // CLAN uses fixed 5-char columns.
    let col_width = 5;

    // Label field: max of "lengths" and longest "*SPK:" plus 1 for padding.
    let max_speaker_label = distributions
        .iter()
        .map(|d| d.speaker.len() + 2) // "*" + speaker + ":"
        .max()
        .unwrap_or(0);
    let label_width = "lengths".len().max(max_speaker_label) + 1;

    let mut header = format!("{:<label_width$}", "lengths");
    for col in 1..=max_len {
        let _ = write!(header, "{:>col_width$}", col);
    }
    let _ = write!(header, "{:>7}", "Mean");
    let _ = writeln!(out, "{header}");

    // Per-speaker rows (CLAN outputs in reverse encounter order).
    for dist in distributions {
        let speaker_label = format!("*{}:", dist.speaker);
        let mut row = format!("{:<label_width$}", speaker_label);
        for col in 1..=max_len {
            let count = dist.distribution.get(&col).copied().unwrap_or(0);
            let _ = write!(row, "{:>col_width$}", count);
        }
        let _ = write!(row, "{:>7.3}", dist.mean());
        let _ = writeln!(out, "{row}");
    }
}

impl WdlenResult {
    /// Convert to the shared section/table model for text rendering.
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("wdlen");
        for data in &self.word_lengths {
            let mut fields = IndexMap::new();
            fields.insert("Total words".to_owned(), data.total_items.to_string());
            fields.insert("Mean word length".to_owned(), format!("{:.3}", data.mean()));

            let rows: Vec<TableRow> = data
                .distribution
                .iter()
                .map(|(length, count)| TableRow {
                    values: vec![length.to_string(), count.to_string()],
                })
                .collect();

            let mut section = Section::with_table(
                format!("Speaker: {}", data.speaker),
                vec!["Length".to_owned(), "Count".to_owned()],
                rows,
            );
            section.fields = fields;
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for WdlenResult {
    /// Render via the shared tabular text formatter (simplified view).
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render the full 6-section CLAN format.
    ///
    /// CLAN outputs speakers in reverse encounter order (its linked-list
    /// iteration pattern). We reverse each section's distributions to match.
    fn render_clan(&self) -> String {
        let sections: &[(&str, &[WdlenDistribution])] = &[
            (
                "Number of words of each length in characters",
                &self.word_lengths,
            ),
            (
                "Number of utterances of each of these lengths in words",
                &self.utt_word_lengths,
            ),
            (
                "Number of single turns of each of these lengths in utterances",
                &self.turn_utt_lengths,
            ),
            (
                "Number of single turns of each of these lengths in words",
                &self.turn_word_lengths,
            ),
            (
                "Number of words of each of these morpheme lengths",
                &self.morph_lengths,
            ),
            (
                "Number of utterances of each of these lengths in morphemes",
                &self.utt_morph_lengths,
            ),
        ];

        let mut out = String::new();
        for (i, (title, dists)) in sections.iter().enumerate() {
            if i > 0 {
                let _ = writeln!(out, "-------");
            }
            let _ = writeln!(out);
            // Reverse to match CLAN's reverse-encounter-order iteration.
            let reversed: Vec<_> = dists.iter().rev().cloned().collect();
            render_section(&mut out, title, &reversed);
        }

        // CLAN appends XML closing tags at the end. The final tag
        // is followed by a newline in CLAN's output (`</Workbook>\n`).
        let _ = writeln!(out, "  </Table>");
        let _ = writeln!(out, " </Worksheet>");
        let _ = writeln!(out, "</Workbook>");

        out
    }
}
