use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// A single mismatch between two tiers.
#[derive(Debug, Clone, Serialize)]
pub struct TrnfixMismatch {
    /// Word/token from the first tier.
    pub tier1_word: String,
    /// Word/token from the second tier.
    pub tier2_word: String,
    /// Number of occurrences.
    pub count: u64,
}

/// Typed output for the TRNFIX command.
#[derive(Debug, Clone, Serialize)]
pub struct TrnfixResult {
    /// Unique mismatch pairs with counts.
    pub mismatches: Vec<TrnfixMismatch>,
    /// Total items compared.
    pub total_items: u64,
    /// Total mismatched items.
    pub total_errors: u64,
    /// Accuracy percentage (0.0-100.0).
    pub accuracy: f64,
    /// First tier name.
    pub tier1: String,
    /// Second tier name.
    pub tier2: String,
}

impl TrnfixResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("trnfix");
        let rows: Vec<TableRow> = self
            .mismatches
            .iter()
            .map(|m| TableRow {
                values: vec![
                    m.count.to_string(),
                    m.tier1_word.clone(),
                    m.tier2_word.clone(),
                ],
            })
            .collect();
        let mut section = Section::with_table(
            "Mismatches".to_owned(),
            vec![
                "Count".to_owned(),
                format!("%{}", self.tier1),
                format!("%{}", self.tier2),
            ],
            rows,
        );
        let mut fields = indexmap::IndexMap::new();
        fields.insert("Total items".to_owned(), self.total_items.to_string());
        fields.insert("Total errors".to_owned(), self.total_errors.to_string());
        fields.insert("Accuracy".to_owned(), format!("{:.1}%", self.accuracy));
        section.fields = fields;
        result.add_section(section);
        result
    }
}

impl CommandOutput for TrnfixResult {
    /// Render mismatch table with accuracy summary.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible mismatch report with accuracy.
    fn render_clan(&self) -> String {
        if self.total_items == 0 {
            return String::new();
        }
        let mut out = String::new();
        for m in &self.mismatches {
            out.push_str(&format!(
                "{:>5}  {} <> {}\n",
                m.count, m.tier1_word, m.tier2_word
            ));
        }
        out.push_str(&format!(
            "Total items on tier: {}\nTotal errors: {}\nAccuracy: {:.1}%\n",
            self.total_items, self.total_errors, self.accuracy
        ));
        out
    }
}
