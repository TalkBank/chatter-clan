//! OVERLAP-AUDIT, CA overlap marker analysis.
//!
//! Analyzes CA overlap markers (⌈⌉⌊⌋) across utterances: counts markers,
//! matches top↔bottom pairs (1:N), reports orphans, checks temporal
//! consistency for files with timing.
//!
//! Uses [`analyze_file_overlaps`] from `talkbank-model` for cross-utterance
//! matching with proper 1:N support and index-aware pairing.
//!
//! # Output
//!
//! Per file:
//! - Number of overlap groups (matched top↔bottom)
//! - Orphaned tops and bottoms
//! - Pairing quality classification
//! - Temporal consistency (for timed files)
//!
//! Plus a corpus-wide summary.

mod output;

use talkbank_model::Utterance;
use talkbank_model::alignment::helpers::overlap_groups::{
    FileOverlapAnalysis, analyze_file_overlaps,
};

use crate::framework::{AnalysisCommand, FileContext};

pub use output::OverlapAuditResult;
use output::{CorpusSummary, FileResult};

/// Configuration for the OVERLAP-AUDIT command.
#[derive(Debug, Clone, Default)]
pub struct OverlapAuditConfig {}

/// Accumulated state for OVERLAP-AUDIT across files.
#[derive(Debug, Default)]
pub struct OverlapAuditState {
    files: Vec<FileResult>,
}

/// OVERLAP-AUDIT command implementation.
#[derive(Debug, Clone, Default)]
pub struct OverlapAuditCommand;

/// Classify pairing quality.
fn classify_quality(analysis: &FileOverlapAnalysis) -> String {
    if !analysis.has_overlaps() {
        return "none".to_owned();
    }
    if analysis.orphaned_tops.is_empty() && analysis.orphaned_bottoms.is_empty() {
        return "fully_paired".to_owned();
    }
    let total =
        analysis.groups.len() + analysis.orphaned_tops.len() + analysis.orphaned_bottoms.len();
    let orphan_fraction =
        (analysis.orphaned_tops.len() + analysis.orphaned_bottoms.len()) as f64 / total as f64;
    const ORPHAN_CLASSIFICATION_THRESHOLD: f64 = 0.8;
    if orphan_fraction > ORPHAN_CLASSIFICATION_THRESHOLD {
        "open_only".to_owned()
    } else {
        "mixed".to_owned()
    }
}

/// Check temporal consistency for a group: does the bottom utterance's
/// timing overlap with the top utterance's timing?
fn is_temporally_consistent(
    top_bullet: Option<(u64, u64)>,
    bottom_bullet: Option<(u64, u64)>,
) -> Option<bool> {
    let (top_start, top_end) = top_bullet?;
    let (bottom_start, _bottom_end) = bottom_bullet?;
    let tolerance_ms: u64 = 2000;
    Some(bottom_start <= top_end + tolerance_ms && bottom_start + tolerance_ms >= top_start)
}

impl AnalysisCommand for OverlapAuditCommand {
    type Config = OverlapAuditConfig;
    type State = OverlapAuditState;
    type Output = OverlapAuditResult;

    fn process_utterance(
        &self,
        _utterance: &Utterance,
        _file_context: &FileContext<'_>,
        _state: &mut Self::State,
    ) {
    }

    fn end_file(&self, file_context: &FileContext<'_>, state: &mut Self::State) {
        let analysis = analyze_file_overlaps(&file_context.chat_file.lines);

        let total_utterances = file_context
            .chat_file
            .lines
            .iter()
            .filter(|l| matches!(l, talkbank_model::Line::Utterance(_)))
            .count();

        let mut timed_groups = 0;
        let mut temporally_consistent = 0;
        for group in &analysis.groups {
            for bottom in &group.bottoms {
                if let Some(consistent) = is_temporally_consistent(group.top.bullet, bottom.bullet)
                {
                    timed_groups += 1;
                    if consistent {
                        temporally_consistent += 1;
                    }
                }
            }
        }

        let quality = classify_quality(&analysis);

        state.files.push(FileResult {
            filename: file_context.filename.to_owned(),
            total_utterances,
            overlap_groups: analysis.groups.len(),
            total_bottoms: analysis.total_bottoms(),
            orphaned_tops: analysis.orphaned_tops.len(),
            orphaned_bottoms: analysis.orphaned_bottoms.len(),
            timed_groups,
            temporally_consistent,
            quality,
        });
    }

    fn finalize(&self, state: Self::State) -> OverlapAuditResult {
        let files_with_overlaps = state
            .files
            .iter()
            .filter(|f| f.overlap_groups > 0 || f.orphaned_tops > 0 || f.orphaned_bottoms > 0)
            .count();

        let summary = CorpusSummary {
            files_total: state.files.len(),
            files_with_overlaps,
            total_groups: state.files.iter().map(|f| f.overlap_groups).sum(),
            total_bottoms: state.files.iter().map(|f| f.total_bottoms).sum(),
            total_orphaned_tops: state.files.iter().map(|f| f.orphaned_tops).sum(),
            total_orphaned_bottoms: state.files.iter().map(|f| f.orphaned_bottoms).sum(),
            timed_groups: state.files.iter().map(|f| f.timed_groups).sum(),
            temporally_consistent: state.files.iter().map(|f| f.temporally_consistent).sum(),
        };

        OverlapAuditResult {
            files: state.files,
            summary,
        }
    }
}
