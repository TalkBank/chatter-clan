use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// A single code entry with its subcode structure.
#[derive(Debug, Clone, Serialize)]
pub struct CodeEntry {
    /// The full code string (e.g., "AC:DI:PP").
    pub code: String,
    /// Number of occurrences.
    pub count: u64,
}

/// Per-speaker code frequency data.
#[derive(Debug, Clone, Serialize)]
pub struct SpeakerCodes {
    /// Speaker identifier.
    pub speaker: String,
    /// Code frequency entries sorted alphabetically.
    pub entries: Vec<CodeEntry>,
    /// Total codes counted.
    pub total: u64,
}

/// Typed output for the CODES command.
#[derive(Debug, Clone, Serialize)]
pub struct CodesResult {
    /// Per-speaker code frequencies.
    pub speakers: Vec<SpeakerCodes>,
    /// Total codes across all speakers.
    pub total: u64,
}

impl CodesResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("codes");
        for speaker in &self.speakers {
            let rows: Vec<TableRow> = speaker
                .entries
                .iter()
                .map(|e| TableRow {
                    values: vec![e.count.to_string(), e.code.clone()],
                })
                .collect();
            let mut section = Section::with_table(
                format!("Speaker: {}", speaker.speaker),
                vec!["Count".to_owned(), "Code".to_owned()],
                rows,
            );
            let mut fields = indexmap::IndexMap::new();
            fields.insert("Total codes".to_owned(), speaker.total.to_string());
            section.fields = fields;
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for CodesResult {
    /// Render code frequencies as a human-readable text table.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render code frequencies in CLAN-compatible format.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for speaker in &self.speakers {
            out.push_str(&format!("Speaker: {}\n", speaker.speaker));
            for entry in &speaker.entries {
                out.push_str(&format!("{:>5}  {}\n", entry.count, entry.code));
            }
            out.push_str(&format!("Total: {}\n\n", speaker.total));
        }
        out
    }
}
