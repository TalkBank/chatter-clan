use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// Per-line frequency data.
#[derive(Debug, Clone, Serialize)]
pub struct UniqEntry {
    /// The line text.
    pub text: String,
    /// Number of occurrences.
    pub count: u64,
}

/// Typed output for the UNIQ command.
#[derive(Debug, Clone, Serialize)]
pub struct UniqResult {
    /// Unique entries with frequency counts.
    pub entries: Vec<UniqEntry>,
    /// Total lines processed.
    pub total: u64,
    /// Number of unique lines.
    pub unique: u64,
}

impl UniqResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("uniq");
        let rows: Vec<TableRow> = self
            .entries
            .iter()
            .map(|e| TableRow {
                values: vec![e.count.to_string(), e.text.clone()],
            })
            .collect();

        let mut section = Section::with_table(
            "Lines".to_owned(),
            vec!["Count".to_owned(), "Text".to_owned()],
            rows,
        );
        let mut fields = indexmap::IndexMap::new();
        fields.insert("Total lines".to_owned(), self.total.to_string());
        fields.insert("Unique lines".to_owned(), self.unique.to_string());
        section.fields = fields;
        result.add_section(section);
        result
    }
}

impl CommandOutput for UniqResult {
    /// Render frequency table with total/unique counts.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible frequency list with summary line.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            out.push_str(&format!("{:>5}  {}\n", entry.count, entry.text));
        }
        out.push_str(&format!(
            "Unique number: {}    Total number: {}\n",
            self.unique, self.total
        ));
        out
    }
}
