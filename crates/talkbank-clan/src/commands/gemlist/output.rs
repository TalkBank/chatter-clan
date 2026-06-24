use serde::Serialize;

use crate::framework::{
    AnalysisResult, CommandOutput, OutputFormat, Section, TableRow, UtteranceCount,
};

/// A single gem segment's aggregated data.
#[derive(Debug, Clone, Serialize)]
pub struct GemEntry {
    /// Gem label (the value after `@Bg:`/`@Eg:`).
    pub label: String,
    /// Number of `@Bg` occurrences with this label.
    pub occurrences: u64,
    /// Total utterances within all instances of this gem.
    pub utterance_count: UtteranceCount,
    /// Speaker codes who produced utterances within this gem (sorted).
    pub speakers: Vec<String>,
}

/// Typed output for the GEMLIST command.
#[derive(Debug, Clone, Serialize)]
pub struct GemlistResult {
    /// Gem entries in encounter order.
    pub gems: Vec<GemEntry>,
    /// Total `@Bg` occurrences across all gem labels.
    pub total_occurrences: u64,
    /// Total utterances inside any gem scope.
    pub total_utterances: UtteranceCount,
}

impl GemlistResult {
    /// Convert typed gem aggregates into the shared section/table render model.
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("gemlist");
        if self.gems.is_empty() {
            return result;
        }

        let rows: Vec<TableRow> = self
            .gems
            .iter()
            .map(|g| TableRow {
                values: vec![
                    g.label.clone(),
                    g.occurrences.to_string(),
                    g.utterance_count.to_string(),
                    g.speakers.join(", "),
                ],
            })
            .collect();

        let mut section = Section::with_table(
            "Gem segments".to_owned(),
            vec![
                "Label".to_owned(),
                "Occurrences".to_owned(),
                "Utterances".to_owned(),
                "Speakers".to_owned(),
            ],
            rows,
        );
        section
            .fields
            .insert("Total gems".to_owned(), self.gems.len().to_string());
        section.fields.insert(
            "Total occurrences".to_owned(),
            self.total_occurrences.to_string(),
        );
        section.fields.insert(
            "Total utterances in gems".to_owned(),
            self.total_utterances.to_string(),
        );

        result.add_section(section);
        result
    }
}

impl CommandOutput for GemlistResult {
    /// Render via the shared tabular text formatter.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }
}
