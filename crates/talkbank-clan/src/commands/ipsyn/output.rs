use std::collections::BTreeMap;

use serde::Serialize;

use crate::framework::{
    AnalysisResult, CommandOutput, OutputFormat, ScorePoints, Section, TableRow, UtteranceCount,
};

/// Per-rule match result.
#[derive(Debug, Clone, Serialize)]
pub struct RuleMatch {
    /// Rule name.
    pub rule: String,
    /// Category.
    pub category: String,
    /// Number of distinct utterances matching (max 2 → 1 point each).
    pub matches: u32,
    /// Points awarded (min(matches, 2)).
    pub points: ScorePoints,
}

/// Per-speaker IPSYN result.
#[derive(Debug, Clone, Serialize)]
pub struct SpeakerIpsyn {
    /// Speaker identifier.
    pub speaker: String,
    /// Number of utterances analyzed.
    pub utterances_analyzed: UtteranceCount,
    /// Per-rule match results.
    pub rule_matches: Vec<RuleMatch>,
    /// Total IPSYN score.
    pub total_score: ScorePoints,
    /// Scores by category.
    pub category_scores: BTreeMap<String, ScorePoints>,
}

/// Typed output for the IPSYN command.
#[derive(Debug, Clone, Serialize)]
pub struct IpsynResult {
    /// Per-speaker results.
    pub speakers: Vec<SpeakerIpsyn>,
}

impl IpsynResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("ipsyn");
        for sp in &self.speakers {
            let rows: Vec<TableRow> = sp
                .rule_matches
                .iter()
                .filter(|r| r.points > 0)
                .map(|r| TableRow {
                    values: vec![
                        r.rule.clone(),
                        r.category.clone(),
                        r.matches.to_string(),
                        r.points.to_string(),
                    ],
                })
                .collect();
            let mut section = Section::with_table(
                format!("Speaker: {}", sp.speaker),
                vec![
                    "Rule".to_owned(),
                    "Category".to_owned(),
                    "Matches".to_owned(),
                    "Points".to_owned(),
                ],
                rows,
            );
            section.fields.insert(
                "Utterances analyzed".to_owned(),
                sp.utterances_analyzed.to_string(),
            );
            section
                .fields
                .insert("Total IPSYN".to_owned(), sp.total_score.to_string());
            for (cat, score) in &sp.category_scores {
                section
                    .fields
                    .insert(format!("{cat} score"), score.to_string());
            }
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for IpsynResult {
    /// Render per-speaker rule-match tables and scores.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible per-speaker summary.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for sp in &self.speakers {
            out.push_str(&format!("Speaker: {}\n", sp.speaker));
            out.push_str(&format!(
                "  Utterances: {}\n  Total IPSYN: {}\n",
                sp.utterances_analyzed, sp.total_score
            ));
            for (cat, score) in &sp.category_scores {
                out.push_str(&format!("  {cat}: {score}\n"));
            }
            out.push('\n');
        }
        out
    }
}
