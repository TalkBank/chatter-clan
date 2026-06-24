//! Typed MAXWD results and rendering logic.

use indexmap::IndexMap;
use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow, WordCount};

/// A single occurrence of a longest word, with its line number.
#[derive(Debug, Clone, Serialize)]
pub struct MaxwdOccurrence {
    /// The display form of the word (preserving `+` in compounds).
    pub display_form: String,
    /// Character length (CLAN-style: excluding `+` and `'`).
    pub char_length: usize,
    /// 1-based line number in the source file.
    pub line_number: usize,
}

/// Per-speaker longest-word results.
#[derive(Debug, Clone, Serialize)]
pub struct MaxwdSpeakerResult {
    /// Speaker code.
    pub speaker: String,
    /// Length of the longest word.
    pub max_length: usize,
    /// Mean word length across all tokens.
    pub mean_length: f64,
    /// Total word tokens counted.
    pub total_words: WordCount,
    /// Number of unique words encountered.
    pub unique_words: usize,
    /// Top words sorted by length descending: `(length, word)`.
    pub top_words: Vec<(usize, String)>,
    /// CLAN display forms (preserving `+` in compounds), keyed by normalized word.
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub display_forms: std::collections::HashMap<String, String>,
    /// Line numbers for words (for CLAN format), keyed by normalized word.
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub line_numbers: std::collections::HashMap<String, usize>,
}

/// Typed output for the MAXWD command.
#[derive(Debug, Clone, Serialize)]
pub struct MaxwdResult {
    /// Per-speaker longest-word results.
    pub speakers: Vec<MaxwdSpeakerResult>,
    /// All occurrences of the globally longest word(s), sorted by line number.
    /// Used by `render_clan()` to match CLAN's output of every tied occurrence.
    pub longest_occurrences: Vec<MaxwdOccurrence>,
}

impl MaxwdResult {
    /// Convert typed MAXWD output into the shared section/table render model.
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("maxwd");
        for data in &self.speakers {
            let mut fields = IndexMap::new();
            fields.insert("Max word length".to_owned(), data.max_length.to_string());
            fields.insert(
                "Mean word length".to_owned(),
                format!("{:.3}", data.mean_length),
            );
            fields.insert("Total words".to_owned(), data.total_words.to_string());
            fields.insert("Unique words".to_owned(), data.unique_words.to_string());

            let rows: Vec<TableRow> = data
                .top_words
                .iter()
                .map(|(len, word)| TableRow {
                    values: vec![len.to_string(), word.clone()],
                })
                .collect();

            let mut section = Section::with_table(
                format!("Speaker: {}", data.speaker),
                vec!["Length".to_owned(), "Word".to_owned()],
                rows,
            );
            section.fields = fields;
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for MaxwdResult {
    /// Render via the shared tabular text formatter.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// CLAN prints EVERY occurrence of words tied for the longest length,
    /// each with its line number, sorted by line number. Words are NOT
    /// deduplicated, if the same word appears on two different lines,
    /// both instances are listed.
    ///
    /// Format (from CLAN snapshot):
    /// ```text
    /// *** File "pipeout": line 22: 9 characters long:
    /// choo+choo's
    /// ```
    fn render_clan(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        for occ in &self.longest_occurrences {
            writeln!(
                out,
                "*** File \"pipeout\": line {}: {} characters long:",
                occ.line_number, occ.char_length
            )
            .ok();
            writeln!(out, "{}", occ.display_form).ok();
        }

        out
    }
}
