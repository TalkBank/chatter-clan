use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section};

/// Per-file accuracy metrics.
#[derive(Debug, Clone, Serialize)]
pub struct FileMetrics {
    /// Filename.
    pub filename: String,
    /// Words produced by subject.
    pub words_produced: u64,
    /// Words expected from template.
    pub words_ideal: u64,
    /// Correct words (matched).
    pub words_correct: u64,
    /// Omitted words (in template but not produced).
    pub words_omitted: u64,
    /// Added words (produced but not in template).
    pub words_added: u64,
    /// Percentage correct.
    pub pct_correct: f64,
}

/// Typed output for the SCRIPT command.
#[derive(Debug, Clone, Serialize)]
pub struct ScriptResult {
    /// Per-file metrics.
    pub files: Vec<FileMetrics>,
    /// Overall metrics.
    /// Total words produced across all files.
    pub total_produced: u64,
    /// Total words expected from template.
    pub total_ideal: u64,
    /// Total correct words.
    pub total_correct: u64,
    /// Total omitted words.
    pub total_omitted: u64,
    /// Total added words.
    pub total_added: u64,
    /// Overall percentage correct.
    pub overall_pct: f64,
}

impl ScriptResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("script");
        for file in &self.files {
            let mut section = Section::with_fields(
                format!("File: {}", file.filename),
                indexmap::IndexMap::new(),
            );
            section
                .fields
                .insert("Words produced".to_owned(), file.words_produced.to_string());
            section
                .fields
                .insert("Words ideal".to_owned(), file.words_ideal.to_string());
            section
                .fields
                .insert("Words correct".to_owned(), file.words_correct.to_string());
            section
                .fields
                .insert("Words omitted".to_owned(), file.words_omitted.to_string());
            section
                .fields
                .insert("Words added".to_owned(), file.words_added.to_string());
            section
                .fields
                .insert("% correct".to_owned(), format!("{:.1}%", file.pct_correct));
            result.add_section(section);
        }

        let mut summary = Section::with_fields("Summary".to_owned(), indexmap::IndexMap::new());
        summary
            .fields
            .insert("Total produced".to_owned(), self.total_produced.to_string());
        summary
            .fields
            .insert("Total ideal".to_owned(), self.total_ideal.to_string());
        summary
            .fields
            .insert("Total correct".to_owned(), self.total_correct.to_string());
        summary
            .fields
            .insert("Overall %".to_owned(), format!("{:.1}%", self.overall_pct));
        result.add_section(summary);
        result
    }
}

impl CommandOutput for ScriptResult {
    /// Render per-file accuracy metrics and overall summary.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible per-file accuracy report.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for file in &self.files {
            out.push_str(&format!("File: {}\n", file.filename));
            out.push_str(&format!("  Words produced: {}\n", file.words_produced));
            out.push_str(&format!("  Words ideal:    {}\n", file.words_ideal));
            out.push_str(&format!("  Words correct:  {}\n", file.words_correct));
            out.push_str(&format!("  Words omitted:  {}\n", file.words_omitted));
            out.push_str(&format!("  Words added:    {}\n", file.words_added));
            out.push_str(&format!("  % correct:      {:.1}%\n\n", file.pct_correct));
        }
        out.push_str(&format!(
            "Overall: {:.1}% correct ({} of {} ideal words)\n",
            self.overall_pct, self.total_correct, self.total_ideal
        ));
        out
    }
}
