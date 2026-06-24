use std::collections::BTreeMap;

use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow, WordCount};

/// Per-speaker morphological category counts.
#[derive(Debug, Clone, Serialize)]
pub struct SpeakerMortable {
    /// Speaker identifier.
    pub speaker: String,
    /// Category label → count.
    pub categories: BTreeMap<String, u64>,
    /// Total words counted.
    pub total_words: WordCount,
}

/// Typed output for the MORTABLE command.
#[derive(Debug, Clone, Serialize)]
pub struct MortableResult {
    /// Per-speaker category frequencies.
    pub speakers: Vec<SpeakerMortable>,
    /// Ordered list of category labels.
    pub labels: Vec<String>,
}

impl MortableResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("mortable");
        for speaker in &self.speakers {
            let mut headers = vec!["Total".to_owned()];
            headers.extend(self.labels.iter().cloned());

            let mut values = vec![speaker.total_words.to_string()];
            for label in &self.labels {
                let count = speaker.categories.get(label).copied().unwrap_or(0);
                values.push(count.to_string());
            }

            let section = Section::with_table(
                format!("Speaker: {}", speaker.speaker),
                headers,
                vec![TableRow { values }],
            );
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for MortableResult {
    /// Render per-speaker category frequency table.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible tab-separated frequency table.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        // Header row
        out.push_str("Speaker\tTotal");
        for label in &self.labels {
            out.push_str(&format!("\t{label}"));
        }
        out.push('\n');

        for speaker in &self.speakers {
            out.push_str(&format!("{}\t{}", speaker.speaker, speaker.total_words));
            for label in &self.labels {
                let count = speaker.categories.get(label).copied().unwrap_or(0);
                out.push_str(&format!("\t{count}"));
            }
            out.push('\n');
        }
        out
    }
}
