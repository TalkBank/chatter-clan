use crate::commands::kideval::SpeakerKideval;
use crate::database::ComparisonResult;

use super::{col, map_kideval_comparison, speaker_to_score_vector};

#[test]
fn score_vector_roundtrip() {
    let speaker = SpeakerKideval {
        speaker: "CHI".to_owned(),
        utterances: 55,
        total_words: 94,
        ndw: 38,
        mlu_words: 1.71,
        mlu_morphemes: 1.87,
        vocd_score: 20.23,
        dss_score: 3.5,
        ipsyn_score: 33,
        word_errors: 2,
        ..Default::default()
    };
    let vec = speaker_to_score_vector(&speaker);
    assert!((vec[col::TOTAL_UTTS] - 55.0).abs() < f64::EPSILON);
    assert!((vec[col::FREQ_TOKENS] - 94.0).abs() < f64::EPSILON);
    assert!((vec[col::NDW] - 38.0).abs() < f64::EPSILON);
    assert!((vec[col::MLU_WORDS] - 1.71).abs() < 1e-10);
    assert!((vec[col::VOCD] - 20.23).abs() < 1e-10);
    assert!((vec[col::DSS] - 3.5).abs() < 1e-10);
    assert!((vec[col::IPSYN_TOTAL] - 33.0).abs() < f64::EPSILON);
}

#[test]
fn mapping_with_empty_comparison() {
    let speaker = SpeakerKideval::default();
    let comparison = ComparisonResult {
        measures: Vec::new(),
        matched_entries: 0,
    };
    let mapped = map_kideval_comparison(&speaker, &comparison);
    assert!(mapped.is_empty());
}
