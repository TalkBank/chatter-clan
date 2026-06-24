use indexmap::IndexMap;
use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section};

/// Per-file overlap results.
#[derive(Debug, Clone, Serialize)]
pub(super) struct FileResult {
    pub(super) filename: String,
    pub(super) total_utterances: usize,
    pub(super) overlap_groups: usize,
    pub(super) total_bottoms: usize,
    pub(super) orphaned_tops: usize,
    pub(super) orphaned_bottoms: usize,
    pub(super) timed_groups: usize,
    pub(super) temporally_consistent: usize,
    pub(super) quality: String,
}

/// Corpus-wide summary.
#[derive(Debug, Clone, Serialize)]
pub(super) struct CorpusSummary {
    pub(super) files_total: usize,
    pub(super) files_with_overlaps: usize,
    pub(super) total_groups: usize,
    pub(super) total_bottoms: usize,
    pub(super) total_orphaned_tops: usize,
    pub(super) total_orphaned_bottoms: usize,
    pub(super) timed_groups: usize,
    pub(super) temporally_consistent: usize,
}

/// Typed output for the OVERLAP-AUDIT command.
#[derive(Debug, Clone, Serialize)]
pub struct OverlapAuditResult {
    pub(super) files: Vec<FileResult>,
    pub(super) summary: CorpusSummary,
}

impl OverlapAuditResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("overlap-audit");

        for f in &self.files {
            if f.overlap_groups == 0 && f.orphaned_tops == 0 && f.orphaned_bottoms == 0 {
                continue;
            }
            let mut fields = IndexMap::new();
            fields.insert("Groups".to_owned(), f.overlap_groups.to_string());
            fields.insert("Bottoms".to_owned(), f.total_bottoms.to_string());
            fields.insert("Orphaned tops".to_owned(), f.orphaned_tops.to_string());
            fields.insert(
                "Orphaned bottoms".to_owned(),
                f.orphaned_bottoms.to_string(),
            );
            fields.insert("Quality".to_owned(), f.quality.clone());
            fields.insert("Utterances".to_owned(), f.total_utterances.to_string());
            if f.timed_groups > 0 {
                fields.insert(
                    "Temporal".to_owned(),
                    format!("{}/{} consistent", f.temporally_consistent, f.timed_groups),
                );
            }
            result.add_section(Section::with_fields(f.filename.clone(), fields));
        }

        let s = &self.summary;
        let mut fields = IndexMap::new();
        fields.insert("Files".to_owned(), s.files_total.to_string());
        fields.insert(
            "Files with overlaps".to_owned(),
            s.files_with_overlaps.to_string(),
        );
        fields.insert("Total groups".to_owned(), s.total_groups.to_string());
        fields.insert("Total bottoms".to_owned(), s.total_bottoms.to_string());
        fields.insert(
            "Orphaned tops".to_owned(),
            s.total_orphaned_tops.to_string(),
        );
        fields.insert(
            "Orphaned bottoms".to_owned(),
            s.total_orphaned_bottoms.to_string(),
        );
        if s.timed_groups > 0 {
            let pct = s.temporally_consistent as f64 / s.timed_groups as f64 * 100.0;
            fields.insert(
                "Temporal consistency".to_owned(),
                format!(
                    "{}/{} ({:.0}%)",
                    s.temporally_consistent, s.timed_groups, pct
                ),
            );
        }
        result.add_section(Section::with_fields("Summary".to_owned(), fields));

        result
    }
}

impl CommandOutput for OverlapAuditResult {
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    fn render_clan(&self) -> String {
        self.render_text()
    }
}
