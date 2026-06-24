//! Typed KWAL results and rendering logic.

use indexmap::IndexMap;
use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// A single match found during KWAL processing.
#[derive(Debug, Clone, Serialize)]
pub struct KwalMatch {
    /// Speaker code.
    pub speaker: String,
    /// Full utterance text (CHAT format).
    pub utterance_text: String,
    /// Source filename.
    pub filename: String,
    /// Matched keyword that triggered this result.
    pub keyword: String,
    /// 1-based line number of this utterance in the source file.
    pub line_number: usize,
    /// CLAN `-wN` pre-context: up to `context_before` preceding
    /// utterance texts, oldest-first. Default empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_context: Vec<String>,
    /// CLAN `+wN` post-context: up to `context_after` following
    /// utterance texts, in stream order. Default empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_context: Vec<String>,
}

/// Typed output for the KWAL command.
#[derive(Debug, Clone, Serialize)]
pub struct KwalResult {
    /// All matching utterances in order encountered.
    pub matches: Vec<KwalMatch>,
    /// Per-keyword match counts.
    pub keyword_counts: IndexMap<String, u64>,
    /// CLAN `+d` (no N): emit the matching utterances as a legal
    /// CHAT fragment, drop the `---` separator and the `*** File
    /// ... Keyword: X` location annotation. Default `false`
    /// preserves the location-annotated layout.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub legal_chat: bool,
}

impl KwalResult {
    /// Convert typed KWAL matches into the shared section/table render model.
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("kwal");

        if !self.matches.is_empty() {
            let rows: Vec<TableRow> = self
                .matches
                .iter()
                .map(|m| TableRow {
                    values: vec![
                        m.filename.clone(),
                        m.speaker.clone(),
                        m.utterance_text.clone(),
                    ],
                })
                .collect();

            let mut matches_section = Section::with_table(
                "Matches".to_owned(),
                vec![
                    "File".to_owned(),
                    "Speaker".to_owned(),
                    "Utterance".to_owned(),
                ],
                rows,
            );
            matches_section
                .fields
                .insert("Total matches".to_owned(), self.matches.len().to_string());
            result.add_section(matches_section);
        }

        if !self.keyword_counts.is_empty() {
            let mut fields = IndexMap::new();
            for (keyword, count) in &self.keyword_counts {
                fields.insert(format!("\"{keyword}\""), count.to_string());
            }
            result.add_section(Section::with_fields("Keyword counts".to_owned(), fields));
        }

        result
    }
}

impl CommandOutput for KwalResult {
    /// Render via the shared tabular text formatter.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// Format (from CLAN snapshot):
    /// ```text
    /// ----------------------------------------
    /// *** File "pipeout": line 10. Keyword: cookie
    /// *CHI:\tmore cookie . [+ IMP]
    /// ```
    fn render_clan(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        for m in &self.matches {
            // `+d` / `legal_chat`: drop the `---` separator and the
            // `*** File ... Keyword: X` location annotation; emit
            // just the matching utterance (plus any context lines)
            // as legal CHAT.
            if !self.legal_chat {
                writeln!(out, "----------------------------------------").ok();
                // CLAN uses "pipeout" as filename when reading from
                // stdin pipe, and 0-based line numbers (doesn't count
                // the @UTF8 BOM line).
                writeln!(
                    out,
                    "*** File \"pipeout\": line {}. Keyword: {} ",
                    m.line_number, m.keyword
                )
                .ok();
            }
            for line in &m.pre_context {
                writeln!(out, "{line}").ok();
            }
            writeln!(out, "{}", m.utterance_text).ok();
            for line in &m.post_context {
                writeln!(out, "{line}").ok();
            }
        }

        out
    }
}
