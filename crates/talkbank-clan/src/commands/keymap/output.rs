use std::collections::BTreeMap;

use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// A single contingency entry: keyword followed by code.
#[derive(Debug, Clone, Serialize)]
pub struct ContingencyEntry {
    /// The keyword that was found.
    pub keyword: String,
    /// Speaker who produced the keyword.
    pub keyword_speaker: String,
    /// The following code.
    pub following_code: String,
    /// Speaker who produced the following code.
    pub following_speaker: String,
    /// Count of this specific transition.
    pub count: u64,
}

/// Per-speaker keyword occurrence data.
#[derive(Debug, Clone, Serialize)]
pub struct SpeakerKeywordData {
    /// Speaker identifier.
    pub speaker: String,
    /// Keyword.
    pub keyword: String,
    /// Total occurrences of this keyword.
    pub total: u64,
    /// Following code contingencies.
    pub following: Vec<FollowingCode>,
}

/// A following code and its count.
#[derive(Debug, Clone, Serialize)]
pub struct FollowingCode {
    /// Following speaker.
    pub speaker: String,
    /// Following code.
    pub code: String,
    /// Count.
    pub count: u64,
}

/// Typed output for the KEYMAP command.
#[derive(Debug, Clone, Serialize)]
pub struct KeymapResult {
    /// Per-speaker keyword data.
    pub data: Vec<SpeakerKeywordData>,
}

impl KeymapResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("keymap");
        for entry in &self.data {
            let rows: Vec<TableRow> = entry
                .following
                .iter()
                .map(|f| TableRow {
                    values: vec![f.speaker.clone(), f.code.clone(), f.count.to_string()],
                })
                .collect();
            let mut section = Section::with_table(
                format!("Speaker: {}, Keyword: {}", entry.speaker, entry.keyword),
                vec![
                    "Following Speaker".to_owned(),
                    "Following Code".to_owned(),
                    "Count".to_owned(),
                ],
                rows,
            );
            section
                .fields
                .insert("Total occurrences".to_owned(), entry.total.to_string());
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for KeymapResult {
    /// Render per-keyword contingency tables.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible keyword frequency output grouped by speaker.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for entry in &self.data {
            out.push_str(&format!(
                "Speaker {}:\n  Key word \"{}\" found {} times\n",
                entry.speaker, entry.keyword, entry.total
            ));
            // Group following by speaker
            let mut by_speaker: BTreeMap<&str, Vec<&FollowingCode>> = BTreeMap::new();
            for f in &entry.following {
                by_speaker.entry(f.speaker.as_str()).or_default().push(f);
            }
            for (sp, codes) in by_speaker {
                let total: u64 = codes.iter().map(|c| c.count).sum();
                out.push_str(&format!(
                    "    {} instances followed by speaker {}, of these\n",
                    total, sp
                ));
                for c in codes {
                    out.push_str(&format!(
                        "      code \"{}\" maps {} time{}\n",
                        c.code,
                        c.count,
                        if c.count == 1 { "" } else { "s" }
                    ));
                }
            }
        }
        out
    }
}
