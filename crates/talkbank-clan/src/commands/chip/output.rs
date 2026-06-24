//! Typed CHIP results and rendering logic.

use std::fmt::Write;

use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// A directed speaker-pair interaction entry.
#[derive(Debug, Clone, Serialize)]
pub struct ChipPairEntry {
    /// Speaker who produced the first utterance.
    pub from: String,
    /// Speaker who produced the second utterance.
    pub to: String,
    /// Number of exact-repetition interactions.
    pub exact_repetitions: u64,
    /// Number of overlap interactions (≥50% shared words).
    pub overlaps: u64,
    /// Number of no-overlap interactions (<50% shared words).
    pub no_overlaps: u64,
}

impl ChipPairEntry {
    /// Total interactions for this pair.
    pub fn total(&self) -> u64 {
        self.exact_repetitions + self.overlaps + self.no_overlaps
    }
}

/// Typed output for the CHIP command.
#[derive(Debug, Clone, Serialize)]
pub struct ChipResult {
    /// Speaker pair entries in encounter order.
    pub pairs: Vec<ChipPairEntry>,
    /// Total interactions across all pairs.
    pub total_interactions: u64,
    /// Total exact repetitions across all pairs.
    pub total_exact: u64,
    /// Total overlaps across all pairs.
    pub total_overlaps: u64,
    /// Echoed utterance lines for CLAN output.
    #[serde(skip)]
    pub echoed_lines: Vec<String>,
}

impl ChipResult {
    /// Convert typed CHIP output into the shared table/field render container.
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("chip");
        if self.pairs.is_empty() {
            return result;
        }

        let rows: Vec<TableRow> = self
            .pairs
            .iter()
            .map(|entry| {
                let total = entry.total();
                let rep_pct = if total > 0 {
                    format!(
                        "{:.1}%",
                        entry.exact_repetitions as f64 / total as f64 * 100.0
                    )
                } else {
                    "0.0%".to_owned()
                };
                let ovl_pct = if total > 0 {
                    format!("{:.1}%", entry.overlaps as f64 / total as f64 * 100.0)
                } else {
                    "0.0%".to_owned()
                };
                TableRow {
                    values: vec![
                        format!("{} → {}", entry.from, entry.to),
                        entry.exact_repetitions.to_string(),
                        entry.overlaps.to_string(),
                        entry.no_overlaps.to_string(),
                        total.to_string(),
                        rep_pct,
                        ovl_pct,
                    ],
                }
            })
            .collect();

        let mut section = Section::with_table(
            "Interaction profile".to_owned(),
            vec![
                "Pair".to_owned(),
                "Exact".to_owned(),
                "Overlap".to_owned(),
                "No overlap".to_owned(),
                "Total".to_owned(),
                "Exact %".to_owned(),
                "Overlap %".to_owned(),
            ],
            rows,
        );
        section.fields.insert(
            "Total interactions".to_owned(),
            self.total_interactions.to_string(),
        );
        section.fields.insert(
            "Total exact repetitions".to_owned(),
            self.total_exact.to_string(),
        );
        section
            .fields
            .insert("Total overlaps".to_owned(), self.total_overlaps.to_string());

        result.add_section(section);
        result
    }
}

/// CLAN CHIP measure labels and their format type (integer or float).
const CHIP_MEASURES: &[(&str, bool)] = &[
    ("Responses", false),
    ("Overlap  ", false),
    ("No_Overlap", false),
    ("%_Overlap", true),
    ("Avg_Dist", true),
    ("Rep_Index", true),
    ("ADD_OPS  ", false),
    ("DEL_OPS  ", false),
    ("EXA_OPS  ", false),
    ("%_ADD_OPS", true),
    ("%_DEL_OPS", true),
    ("%_EXA_OPS", true),
    ("ADD_WORD", false),
    ("DEL_WORD", false),
    ("EXA_WORD", false),
    ("%_ADD_WORDS", true),
    ("%_DEL_WORDS", true),
    ("%_EXA_WORDS", true),
    ("MORPH_ADD", false),
    ("MORPH_DEL", false),
    ("MORPH_EXA", false),
    ("MORPH_SUB", false),
    ("%_MORPH_ADD", true),
    ("%_MORPH_DEL", true),
    ("%_MORPH_EXA", true),
    ("%_MORPH_SUB", true),
    ("AV_WORD_ADD", true),
    ("AV_WORD_DEL", true),
    ("AV_WORD_EXA", true),
    ("IMITAT   ", false),
    ("%_IMITAT", true),
    ("EXACT    ", false),
    ("EXPAN    ", false),
    ("REDUC    ", false),
    ("%_EXACT  ", true),
    ("%_EXPAN  ", true),
    ("%_REDUC  ", true),
];

impl CommandOutput for ChipResult {
    /// Render CHIP output through the shared text table renderer.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible CHIP output.
    ///
    /// Format: echoed utterances, separator, scored counts, 36-row measure matrix
    /// with ADU/CHI/ASR/CSR columns.
    fn render_clan(&self) -> String {
        let mut out = String::new();

        // Echo utterances.
        for line in &self.echoed_lines {
            writeln!(out, "{line}").ok();
        }

        // Separator and file header.
        writeln!(
            out,
            "==========================================================="
        )
        .ok();
        writeln!(out, "File: pipeout").ok();
        writeln!(out).ok();

        // Scored utterance counts (currently always 0).
        writeln!(out, "Total  scored utterances: 0").ok();
        writeln!(out, "Total  scored utterances: 0").ok();
        writeln!(out).ok();

        // Matrix header.
        writeln!(out, "Measure  \tADU\tCHI\tASR\tCSR").ok();
        writeln!(
            out,
            "-----------------------------------------------------------"
        )
        .ok();

        // 36 measure rows (currently all zeros, full computation not yet implemented).
        for &(label, is_float) in CHIP_MEASURES {
            if is_float {
                writeln!(out, "{label}\t0.00\t0.00\t0.00\t0.00").ok();
            } else {
                writeln!(out, "{label}\t0\t0\t0\t0").ok();
            }
        }

        out
    }
}
