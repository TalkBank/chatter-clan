//! Typed LINKER-AUDIT results and rendering logic.

use indexmap::IndexMap;
use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section};

use super::CorpusSummary;
use super::FileStats;

/// Top-level result for the LINKER-AUDIT command.
#[derive(Debug, Clone, Serialize)]
pub struct LinkerAuditResult {
    pub(super) files: Vec<FileStats>,
    pub(super) summary: CorpusSummary,
}

impl LinkerAuditResult {
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("linker-audit");
        let s = &self.summary;

        let mut fields = IndexMap::new();
        fields.insert(
            "+< (lazy overlap)".to_owned(),
            s.total_lazy_overlap.to_string(),
        );
        fields.insert(
            "++ (other completion)".to_owned(),
            s.total_other_completion.to_string(),
        );
        fields.insert(
            "+^ (quick uptake)".to_owned(),
            s.total_quick_uptake.to_string(),
        );
        fields.insert(
            "+\" (quotation follows)".to_owned(),
            s.total_quotation_follows.to_string(),
        );
        fields.insert(
            "+, (self completion)".to_owned(),
            s.total_self_completion.to_string(),
        );
        fields.insert(
            "+≋ (TCU continuation)".to_owned(),
            s.total_tcu_continuation.to_string(),
        );
        fields.insert(
            "+≈ (no-break TCU)".to_owned(),
            s.total_no_break_tcu.to_string(),
        );
        result.add_section(Section::with_fields(
            "Linker Frequencies".to_owned(),
            fields,
        ));

        let mut fields = IndexMap::new();
        fields.insert(
            "+... (trailing off)".to_owned(),
            s.total_trailing_off.to_string(),
        );
        fields.insert(
            "+..? (trailing off question)".to_owned(),
            s.total_trailing_off_question.to_string(),
        );
        fields.insert(
            "+/. (interruption)".to_owned(),
            s.total_interruption.to_string(),
        );
        fields.insert(
            "+/? (interrupted question)".to_owned(),
            s.total_interrupted_question.to_string(),
        );
        fields.insert(
            "+//. (self-interruption)".to_owned(),
            s.total_self_interruption.to_string(),
        );
        fields.insert(
            "+//? (self-interrupted question)".to_owned(),
            s.total_self_interrupted_question.to_string(),
        );
        fields.insert(
            "+!? (broken question)".to_owned(),
            s.total_broken_question.to_string(),
        );
        fields.insert(
            "+\"/. (quotation follows)".to_owned(),
            s.total_quotation_follows_term.to_string(),
        );
        fields.insert(
            "+\". (quotation precedes)".to_owned(),
            s.total_quotation_precedes_term.to_string(),
        );
        fields.insert(
            "+. (break for coding)".to_owned(),
            s.total_break_for_coding.to_string(),
        );
        result.add_section(Section::with_fields(
            "Special Terminator Frequencies".to_owned(),
            fields,
        ));

        let pp_total =
            s.pp_correct + s.pp_same_speaker + s.pp_wrong_terminator + s.pp_first_utterance;
        let mut fields = IndexMap::new();
        fields.insert("Total ++".to_owned(), pp_total.to_string());
        fields.insert(
            "Correct (diff speaker + +...)".to_owned(),
            pct_str(s.pp_correct, pp_total),
        );
        fields.insert(
            "ANOMALY: same speaker (should be +,)".to_owned(),
            pct_str(s.pp_same_speaker, pp_total),
        );
        fields.insert(
            "ANOMALY: wrong terminator".to_owned(),
            pct_str(s.pp_wrong_terminator, pp_total),
        );
        fields.insert(
            "ANOMALY: first utterance".to_owned(),
            pct_str(s.pp_first_utterance, pp_total),
        );
        result.add_section(Section::with_fields(
            "++ (Other Completion) Pairing".to_owned(),
            fields,
        ));

        let sc_total = s.sc_correct + s.sc_wrong_terminator + s.sc_no_prior;
        let mut fields = IndexMap::new();
        fields.insert("Total +,".to_owned(), sc_total.to_string());
        fields.insert(
            "Correct (same speaker + +/.)".to_owned(),
            pct_str(s.sc_correct, sc_total),
        );
        fields.insert(
            "ANOMALY: wrong terminator".to_owned(),
            pct_str(s.sc_wrong_terminator, sc_total),
        );
        fields.insert(
            "ANOMALY: no prior same-speaker".to_owned(),
            pct_str(s.sc_no_prior, sc_total),
        );
        result.add_section(Section::with_fields(
            "+, (Self Completion) Pairing".to_owned(),
            fields,
        ));

        let qf_total = s.qf_correct + s.qf_chained + s.qf_wrong_terminator + s.qf_no_prior;
        let mut fields = IndexMap::new();
        fields.insert("Total +\"".to_owned(), qf_total.to_string());
        fields.insert(
            "Correct (same speaker + +\"/.)".to_owned(),
            pct_str(s.qf_correct, qf_total),
        );
        fields.insert(
            "Chained (same speaker + +\")".to_owned(),
            pct_str(s.qf_chained, qf_total),
        );
        fields.insert(
            "ANOMALY: wrong terminator".to_owned(),
            pct_str(s.qf_wrong_terminator, qf_total),
        );
        fields.insert(
            "ANOMALY: no prior same-speaker".to_owned(),
            pct_str(s.qf_no_prior, qf_total),
        );
        result.add_section(Section::with_fields(
            "+\" (Quotation) Pairing".to_owned(),
            fields,
        ));

        let mut fields = IndexMap::new();
        fields.insert("Total +< blocks".to_owned(), s.lo_blocks_total.to_string());
        fields.insert("Isolated (size 1)".to_owned(), s.lo_isolated.to_string());
        fields.insert("Pairs (size 2)".to_owned(), s.lo_pairs.to_string());
        fields.insert("Large (size 3+)".to_owned(), s.lo_large_blocks.to_string());
        fields.insert(
            "Same-speaker start (suspicious)".to_owned(),
            s.lo_same_speaker_start.to_string(),
        );
        fields.insert(
            "Combined with other linker".to_owned(),
            s.lo_combined_with_other.to_string(),
        );
        result.add_section(Section::with_fields(
            "+< (Lazy Overlap) Blocks".to_owned(),
            fields,
        ));

        let mut fields = IndexMap::new();
        fields.insert("Same speaker".to_owned(), s.qu_same_speaker.to_string());
        fields.insert(
            "Different speaker".to_owned(),
            s.qu_diff_speaker.to_string(),
        );
        result.add_section(Section::with_fields(
            "+^ (Quick Uptake) Speaker".to_owned(),
            fields,
        ));

        if s.tcu_tech_same + s.tcu_tech_diff > 0 || s.tcu_nb_same + s.tcu_nb_diff > 0 {
            let mut fields = IndexMap::new();
            fields.insert("+≋ same speaker".to_owned(), s.tcu_tech_same.to_string());
            fields.insert("+≋ diff speaker".to_owned(), s.tcu_tech_diff.to_string());
            fields.insert("+≈ same speaker".to_owned(), s.tcu_nb_same.to_string());
            fields.insert("+≈ diff speaker".to_owned(), s.tcu_nb_diff.to_string());
            result.add_section(Section::with_fields("CA TCU Linkers".to_owned(), fields));
        }

        let mut fields = IndexMap::new();
        fields.insert("+... total".to_owned(), s.trailing_off_total.to_string());
        fields.insert(
            "+... followed by ++/+,".to_owned(),
            s.trailing_off_followed.to_string(),
        );
        fields.insert(
            "+... orphaned".to_owned(),
            (s.trailing_off_total - s.trailing_off_followed).to_string(),
        );
        fields.insert("+/. total".to_owned(), s.interruption_total.to_string());
        fields.insert(
            "+/. followed by +,".to_owned(),
            s.interruption_followed.to_string(),
        );
        fields.insert(
            "+/. orphaned".to_owned(),
            (s.interruption_total - s.interruption_followed).to_string(),
        );
        result.add_section(Section::with_fields(
            "Orphaned Special Terminators".to_owned(),
            fields,
        ));

        let mut fields = IndexMap::new();
        fields.insert("Files analyzed".to_owned(), s.files_total.to_string());
        fields.insert(
            "Files with linkers/special terminators".to_owned(),
            s.files_with_linkers.to_string(),
        );
        fields.insert(
            "Files with anomalies".to_owned(),
            s.files_with_anomalies.to_string(),
        );
        result.add_section(Section::with_fields("Summary".to_owned(), fields));

        result
    }
}

impl CommandOutput for LinkerAuditResult {
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    fn render_clan(&self) -> String {
        self.render_text()
    }
}

fn pct_str(count: usize, total: usize) -> String {
    if total == 0 {
        format!("{count}")
    } else {
        format!("{count} ({:.1}%)", count as f64 / total as f64 * 100.0)
    }
}
