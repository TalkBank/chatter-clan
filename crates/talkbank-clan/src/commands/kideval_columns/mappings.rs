use crate::commands::kideval::SpeakerKideval;
use crate::database::ComparisonResult;

use super::{KidevalMeasureComparison, col};

/// Mapping entry: which `.cut` column corresponds to which speaker field.
struct ColumnMapping {
    label: &'static str,
    col_index: usize,
    extract: fn(&SpeakerKideval) -> f64,
}

/// All mappings between `SpeakerKideval` fields and `.cut` database columns.
const MAPPINGS: &[ColumnMapping] = &[
    ColumnMapping {
        label: "Utterances",
        col_index: col::TOTAL_UTTS,
        extract: |s| s.utterances as f64,
    },
    ColumnMapping {
        label: "Total words",
        col_index: col::FREQ_TOKENS,
        extract: |s| s.total_words as f64,
    },
    ColumnMapping {
        label: "NDW",
        col_index: col::NDW,
        extract: |s| s.ndw as f64,
    },
    ColumnMapping {
        label: "MLU (words)",
        col_index: col::MLU_WORDS,
        extract: |s| s.mlu_words,
    },
    ColumnMapping {
        label: "MLU (morphemes)",
        col_index: col::MLU_MORPHEMES,
        extract: |s| s.mlu_morphemes,
    },
    ColumnMapping {
        label: "VOCD",
        col_index: col::VOCD,
        extract: |s| s.vocd_score,
    },
    ColumnMapping {
        label: "DSS",
        col_index: col::DSS,
        extract: |s| s.dss_score,
    },
    ColumnMapping {
        label: "IPSyn",
        col_index: col::IPSYN_TOTAL,
        extract: |s| s.ipsyn_score as f64,
    },
    ColumnMapping {
        label: "Word errors",
        col_index: col::WORD_ERRORS,
        extract: |s| s.word_errors as f64,
    },
];

/// Extract the scores from a `SpeakerKideval` that have database column mappings,
/// and produce named comparisons from a [`ComparisonResult`].
///
/// This bridges the gap between the positional database comparison and the
/// typed KidEval output, selecting only the columns we compute and labeling them.
pub fn map_kideval_comparison(
    speaker: &SpeakerKideval,
    comparison: &ComparisonResult,
) -> Vec<KidevalMeasureComparison> {
    MAPPINGS
        .iter()
        .filter_map(|m| {
            let measure = comparison.measures.get(m.col_index)?;
            Some(KidevalMeasureComparison {
                label: m.label,
                score: (m.extract)(speaker),
                db_mean: measure.db_mean,
                db_sd: measure.db_sd,
                z_score: measure.z_score,
                db_n: measure.db_n,
            })
        })
        .collect()
}

/// Build a score vector from a `SpeakerKideval` for raw positional comparison.
///
/// Returns a vector with scores placed at their database column positions.
/// Unused positions are filled with 0.0.
pub fn speaker_to_score_vector(speaker: &SpeakerKideval) -> Vec<f64> {
    let max_col = MAPPINGS.iter().map(|m| m.col_index).max().unwrap_or(0);
    let mut scores = vec![0.0; max_col + 1];
    for m in MAPPINGS {
        scores[m.col_index] = (m.extract)(speaker);
    }
    scores
}
