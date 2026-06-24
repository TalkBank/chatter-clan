use crate::commands::eval::SpeakerEval;
use crate::database::ComparisonResult;

use super::{EvalMeasureComparison, col};

/// Mapping entry: which `.cut` column corresponds to which speaker field.
struct ColumnMapping {
    label: &'static str,
    col_index: usize,
    extract: fn(&SpeakerEval) -> f64,
}

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
        col_index: col::MLU_WORDS_SUM,
        extract: |s| s.mlu_words,
    },
    ColumnMapping {
        label: "MLU (morphemes)",
        col_index: col::MLU_MORF_SUM,
        extract: |s| s.mlu_morphemes,
    },
    ColumnMapping {
        label: "Total morphemes",
        col_index: col::MOR_TOTAL,
        extract: |s| s.total_morphemes as f64,
    },
    ColumnMapping {
        label: "Nouns",
        col_index: col::NOUNS,
        extract: |s| s.nouns as f64,
    },
    ColumnMapping {
        label: "Verbs",
        col_index: col::VERBS,
        extract: |s| s.verbs as f64,
    },
    ColumnMapping {
        label: "Auxiliaries",
        col_index: col::AUX,
        extract: |s| s.auxiliaries as f64,
    },
    ColumnMapping {
        label: "Modals",
        col_index: col::MODALS,
        extract: |s| s.modals as f64,
    },
    ColumnMapping {
        label: "Prepositions",
        col_index: col::PREP,
        extract: |s| s.prepositions as f64,
    },
    ColumnMapping {
        label: "Adjectives",
        col_index: col::ADJ,
        extract: |s| s.adjectives as f64,
    },
    ColumnMapping {
        label: "Adverbs",
        col_index: col::ADV,
        extract: |s| s.adverbs as f64,
    },
    ColumnMapping {
        label: "Conjunctions",
        col_index: col::CONJ,
        extract: |s| s.conjunctions as f64,
    },
    ColumnMapping {
        label: "Pronouns",
        col_index: col::PRON,
        extract: |s| s.pronouns as f64,
    },
    ColumnMapping {
        label: "Determiners",
        col_index: col::DET,
        extract: |s| s.determiners as f64,
    },
    ColumnMapping {
        label: "Plurals",
        col_index: col::PLURALS,
        extract: |s| s.plurals as f64,
    },
    ColumnMapping {
        label: "Past tense",
        col_index: col::PAST,
        extract: |s| s.past_tense as f64,
    },
    ColumnMapping {
        label: "Present participle",
        col_index: col::PRESENT_PARTICIPLE,
        extract: |s| s.present_participle as f64,
    },
    ColumnMapping {
        label: "Past participle",
        col_index: col::PAST_PARTICIPLE,
        extract: |s| s.past_participle as f64,
    },
    ColumnMapping {
        label: "Word errors",
        col_index: col::WORD_ERRORS,
        extract: |s| s.word_errors as f64,
    },
];

/// Extract the scores from a `SpeakerEval` that have database column mappings,
/// and produce named comparisons from a [`ComparisonResult`].
pub fn map_eval_comparison(
    speaker: &SpeakerEval,
    comparison: &ComparisonResult,
) -> Vec<EvalMeasureComparison> {
    MAPPINGS
        .iter()
        .filter_map(|m| {
            let measure = comparison.measures.get(m.col_index)?;
            Some(EvalMeasureComparison {
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

/// Build a score vector from a `SpeakerEval` for raw positional comparison.
pub fn speaker_to_score_vector(speaker: &SpeakerEval) -> Vec<f64> {
    let max_col = MAPPINGS.iter().map(|m| m.col_index).max().unwrap_or(0);
    let mut scores = vec![0.0; max_col + 1];
    for m in MAPPINGS {
        scores[m.col_index] = (m.extract)(speaker);
    }
    scores
}
