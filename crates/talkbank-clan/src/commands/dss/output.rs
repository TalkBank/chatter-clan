//! Typed DSS results and rendering logic.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::framework::{
    AnalysisResult, AnalysisScore, CommandOutput, OutputFormat, ScorePoints, Section,
    UtteranceCount,
};

/// Per-utterance DSS score.
#[derive(Debug, Clone, Serialize)]
pub struct UtteranceScore {
    /// Utterance index (1-based).
    pub index: usize,
    /// Utterance text (abbreviated).
    pub text: String,
    /// Points per category.
    pub category_points: BTreeMap<String, ScorePoints>,
    /// Total points for this utterance.
    pub total: ScorePoints,
    /// Whether this utterance is a complete sentence (awards 1 extra point).
    pub sentence_point: bool,
}

/// Per-speaker DSS result.
#[derive(Debug, Clone, Serialize)]
pub struct SpeakerDss {
    /// Speaker identifier.
    pub speaker: String,
    /// Number of utterances scored.
    pub utterances_scored: UtteranceCount,
    /// Individual utterance scores.
    pub scores: Vec<UtteranceScore>,
    /// Grand total (sum of all utterance totals + sentence points).
    pub grand_total: u32,
    /// DSS score (grand total / number of utterances scored).
    pub dss_score: AnalysisScore,
}

/// Typed output for the DSS command.
#[derive(Debug, Clone, Serialize)]
pub struct DssResult {
    /// Per-speaker results.
    pub speakers: Vec<SpeakerDss>,
}

impl DssResult {
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("dss");
        for sp in &self.speakers {
            let mut section = Section::with_fields(
                format!("Speaker: {}", sp.speaker),
                indexmap::IndexMap::new(),
            );
            section.fields.insert(
                "Utterances scored".to_owned(),
                sp.utterances_scored.to_string(),
            );
            section
                .fields
                .insert("Grand total".to_owned(), sp.grand_total.to_string());
            section
                .fields
                .insert("DSS score".to_owned(), format!("{:.2}", sp.dss_score));
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for DssResult {
    /// Render DSS scores as a human-readable text summary.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render DSS scores in CLAN-compatible format.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for sp in &self.speakers {
            out.push_str(&format!("Speaker: {}\n", sp.speaker));
            out.push_str(&format!("  Utterances scored: {}\n", sp.utterances_scored));
            out.push_str(&format!("  Grand total: {}\n", sp.grand_total));
            out.push_str(&format!("  DSS score: {:.2}\n\n", sp.dss_score));
        }
        out
    }
}
