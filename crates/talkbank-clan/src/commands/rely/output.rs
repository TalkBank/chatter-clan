use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// Per-code agreement statistics.
#[derive(Debug, Clone, Serialize)]
pub struct CodeAgreement {
    /// Code token.
    pub code: String,
    /// Count in file 1.
    pub count_file1: u64,
    /// Count in file 2.
    pub count_file2: u64,
    /// Number of agreed instances.
    pub agreed: u64,
    /// Agreement percentage.
    pub agreement_pct: f64,
}

/// Typed output for the RELY command.
#[derive(Debug, Clone, Serialize)]
pub struct RelyResult {
    /// Per-code agreement statistics.
    pub codes: Vec<CodeAgreement>,
    /// Overall agreement percentage.
    pub overall_agreement: f64,
    /// Cohen's kappa coefficient.
    pub kappa: f64,
    /// Total codes in file 1.
    pub total_file1: u64,
    /// Total codes in file 2.
    pub total_file2: u64,
}

impl RelyResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("rely");
        let rows: Vec<TableRow> = self
            .codes
            .iter()
            .map(|c| TableRow {
                values: vec![
                    c.code.clone(),
                    c.count_file1.to_string(),
                    c.count_file2.to_string(),
                    c.agreed.to_string(),
                    format!("{:.1}%", c.agreement_pct),
                ],
            })
            .collect();
        let mut section = Section::with_table(
            "Code Agreement".to_owned(),
            vec![
                "Code".to_owned(),
                "File 1".to_owned(),
                "File 2".to_owned(),
                "Agreed".to_owned(),
                "Agreement".to_owned(),
            ],
            rows,
        );
        section.fields.insert(
            "Overall Agreement".to_owned(),
            format!("{:.1}%", self.overall_agreement),
        );
        section
            .fields
            .insert("Cohen's Kappa".to_owned(), format!("{:.4}", self.kappa));
        result.add_section(section);
        result
    }
}

impl CommandOutput for RelyResult {
    /// Render per-code agreement table with overall statistics.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible agreement summary with kappa.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        out.push_str("Code Agreement:\n");
        for c in &self.codes {
            out.push_str(&format!(
                "  {:>10}  file1:{:>4}  file2:{:>4}  agreed:{:>4}  {:.1}%\n",
                c.code, c.count_file1, c.count_file2, c.agreed, c.agreement_pct
            ));
        }
        out.push_str(&format!(
            "Overall Agreement: {:.1}%\n",
            self.overall_agreement
        ));
        out.push_str(&format!("Cohen's Kappa: {:.4}\n", self.kappa));
        out
    }
}
