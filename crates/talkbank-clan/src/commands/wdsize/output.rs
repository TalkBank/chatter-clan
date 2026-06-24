//! Typed WDSIZE results and rendering logic.

use std::collections::BTreeMap;
use std::fmt::Write;

use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow, WordCount};

/// Per-speaker word size distribution.
#[derive(Debug, Clone, Serialize)]
pub struct WdsizeDistribution {
    /// Speaker identifier.
    pub speaker: String,
    /// Character length -> count mapping.
    pub distribution: BTreeMap<usize, u64>,
    /// Total number of words measured.
    pub total_words: WordCount,
    /// Sum of all character lengths.
    pub total_chars: u64,
}

impl WdsizeDistribution {
    /// Mean word size in characters.
    pub(crate) fn mean(&self) -> f64 {
        if self.total_words == 0 {
            0.0
        } else {
            self.total_chars as f64 / self.total_words as f64
        }
    }
}

/// Result of the WDSIZE command.
#[derive(Debug, Clone, Serialize)]
pub struct WdsizeResult {
    /// Per-speaker distributions.
    pub speakers: Vec<WdsizeDistribution>,
}

impl CommandOutput for WdsizeResult {
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    fn render_clan(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "\nNumber of words of each length in characters");

        let max_len = self
            .speakers
            .iter()
            .flat_map(|d| d.distribution.keys())
            .copied()
            .max()
            .unwrap_or(1);

        let col_width = 5;
        let max_speaker_label = self
            .speakers
            .iter()
            .map(|d| d.speaker.len() + 2)
            .max()
            .unwrap_or(0);
        let label_width = "lengths".len().max(max_speaker_label) + 1;

        let mut header = format!("{:<label_width$}", "lengths");
        for col in 1..=max_len {
            let _ = write!(header, "{:>col_width$}", col);
        }
        let _ = write!(header, "{:>7}", "Mean");
        let _ = writeln!(out, "{header}");

        for dist in self.speakers.iter().rev() {
            let speaker_label = format!("*{}:", dist.speaker);
            let mut row = format!("{:<label_width$}", speaker_label);
            for col in 1..=max_len {
                let count = dist.distribution.get(&col).copied().unwrap_or(0);
                let _ = write!(row, "{:>col_width$}", count);
            }
            let _ = write!(row, "{:>7.3}", dist.mean());
            let _ = writeln!(out, "{row}");
        }
        out
    }
}

impl WdsizeResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("wdsize");
        for data in &self.speakers {
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
            let mut fields = indexmap::IndexMap::new();
            fields.insert("Total words".to_owned(), data.total_words.to_string());
            fields.insert("Mean word size".to_owned(), format!("{:.3}", data.mean()));
            section.fields = fields;
            result.add_section(section);
        }
        result
    }
}
