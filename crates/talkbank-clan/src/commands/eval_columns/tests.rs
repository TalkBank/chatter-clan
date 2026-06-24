use crate::commands::eval::SpeakerEval;
use crate::database::ComparisonResult;

use super::{col, map_eval_comparison, speaker_to_score_vector};

#[test]
fn score_vector_places_values_correctly() {
    let speaker = SpeakerEval {
        speaker: "PAR".to_owned(),
        utterances: 42,
        total_words: 120,
        ndw: 55,
        nouns: 18,
        verbs: 15,
        word_errors: 3,
        ..Default::default()
    };
    let vec = speaker_to_score_vector(&speaker);
    assert!((vec[col::TOTAL_UTTS] - 42.0).abs() < f64::EPSILON);
    assert!((vec[col::FREQ_TOKENS] - 120.0).abs() < f64::EPSILON);
    assert!((vec[col::NDW] - 55.0).abs() < f64::EPSILON);
    assert!((vec[col::NOUNS] - 18.0).abs() < f64::EPSILON);
    assert!((vec[col::WORD_ERRORS] - 3.0).abs() < f64::EPSILON);
}

#[test]
fn mapping_with_empty_comparison() {
    let speaker = SpeakerEval::default();
    let comparison = ComparisonResult {
        measures: Vec::new(),
        matched_entries: 0,
    };
    let mapped = map_eval_comparison(&speaker, &comparison);
    assert!(mapped.is_empty());
}
