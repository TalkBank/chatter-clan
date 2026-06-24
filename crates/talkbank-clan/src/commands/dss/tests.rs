use super::*;

use smallvec::smallvec;
use talkbank_model::{MorFeature, MorWord};

fn mor(pos: &str, lemma: &str, features: &[&str]) -> Mor {
    let mut word = MorWord::new(pos, lemma);
    word.features = features.iter().map(MorFeature::new).collect();
    Mor {
        main: word,
        post_clitics: smallvec![],
    }
}

#[test]
fn dss_empty() {
    let cmd = DssCommand::new(DssConfig::default()).unwrap();
    let state = DssState::default();
    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
}

#[test]
fn score_utterance_basic() {
    let rules = DssRuleSet::default();
    let items = vec![
        mor("pro:sub", "I", &[]),
        mor("v", "want", &[]),
        mor("det:art", "the", &[]),
        mor("n", "ball", &[]),
    ];
    let (points, total) = score_utterance(&items, &rules);
    assert!(total > 0);
    assert!(points.contains_key("personal_pronouns"));
    assert!(points.contains_key("main_verbs"));
    assert!(points.contains_key("articles"));
}

#[test]
fn score_utterance_past_tense() {
    let rules = DssRuleSet::default();
    let items = vec![mor("pro:sub", "I", &[]), mor("v", "walk", &["PAST"])];
    let (points, _total) = score_utterance(&items, &rules);
    assert!(points.contains_key("past_tense"));
}

#[test]
fn is_complete_sentence_check() {
    let complete = vec![
        mor("pro:sub", "I", &[]),
        mor("v", "want", &[]),
        mor("det:art", "the", &[]),
        mor("n", "ball", &[]),
    ];
    assert!(is_complete_sentence(&complete));

    let no_verb = vec![mor("det:art", "the", &[]), mor("n", "ball", &[])];
    assert!(!is_complete_sentence(&no_verb));

    let no_subject = vec![mor("v", "run", &[])];
    assert!(!is_complete_sentence(&no_subject));
}

#[test]
fn is_complete_sentence_with_copula() {
    let items = vec![
        mor("n", "dog", &[]),
        mor("cop", "be", &["3S"]),
        mor("adj", "big", &[]),
    ];
    assert!(is_complete_sentence(&items));
}

#[test]
fn is_complete_sentence_with_proper_noun() {
    let items = vec![mor("n:prop", "John", &[]), mor("v", "run", &["PAST"])];
    assert!(is_complete_sentence(&items));
}

#[test]
fn is_complete_sentence_ud_tags() {
    let items = vec![
        mor("pron", "I", &["Prs", "Nom", "S1"]),
        mor("verb", "want", &["Fin", "Ind", "Pres"]),
        mor("noun", "cookie", &["Plur"]),
    ];
    assert!(is_complete_sentence(&items));
}

#[test]
fn is_complete_sentence_ud_propn_subject() {
    let items = vec![
        mor("propn", "Mommy", &[]),
        mor("aux", "will", &[]),
        mor("verb", "get", &["Inf"]),
    ];
    assert!(is_complete_sentence(&items));
}

#[test]
fn is_complete_sentence_ud_noun_subject() {
    let items = vec![
        mor("noun", "dog", &[]),
        mor("aux", "be", &["Fin", "Ind", "Pres", "S3"]),
        mor("adj", "big", &[]),
    ];
    assert!(is_complete_sentence(&items));
}

#[test]
fn score_utterance_ud_format() {
    let rules = DssRuleSet::default();
    let items = vec![
        mor("pron", "I", &["Prs", "Nom", "S1"]),
        mor("verb", "want", &["Fin", "Ind", "Pres"]),
        mor("det", "the", &[]),
        mor("noun", "ball", &[]),
    ];
    let (_points, total) = score_utterance(&items, &rules);
    assert!(total > 0);
}
