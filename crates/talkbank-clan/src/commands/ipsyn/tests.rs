use super::*;

use smallvec::smallvec;
use talkbank_model::{MorFeature, MorWord};

fn mor_item(pos: &str, lemma: &str, features: &[&str]) -> Mor {
    let mut word = MorWord::new(pos, lemma);
    word.features = features.iter().map(MorFeature::new).collect();
    Mor {
        main: word,
        post_clitics: smallvec![],
    }
}

#[test]
fn ipsyn_empty() {
    let cmd = IpsynCommand::new(IpsynConfig::default()).unwrap();
    let state = IpsynState::default();
    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
}

#[test]
fn rule_match_basic() {
    let rule = IpsynRule {
        name: "S1".to_owned(),
        category: 'S',
        include_patterns: vec!["pro:sub|".to_owned(), "v|".to_owned()],
        exclude_patterns: vec![],
        description: "Subject-Verb".to_owned(),
    };
    let items = vec![
        mor_item("pro:sub", "I", &[]),
        mor_item("v", "want", &[]),
        mor_item("det:art", "a", &[]),
        mor_item("n", "ball", &[]),
    ];
    assert!(rule_matches(&items, &rule));

    let no_verb = vec![mor_item("det:art", "the", &[]), mor_item("n", "ball", &[])];
    assert!(!rule_matches(&no_verb, &rule));
}

#[test]
fn rule_exclude_works() {
    let rule = IpsynRule {
        name: "Test".to_owned(),
        category: 'T',
        include_patterns: vec!["v|".to_owned()],
        exclude_patterns: vec!["neg|".to_owned()],
        description: "Verb without negation".to_owned(),
    };
    let items = vec![mor_item("pro:sub", "I", &[]), mor_item("v", "want", &[])];
    assert!(rule_matches(&items, &rule));

    let with_neg = vec![
        mor_item("pro:sub", "I", &[]),
        mor_item("neg", "not", &[]),
        mor_item("v", "want", &[]),
    ];
    assert!(!rule_matches(&with_neg, &rule));
}

#[test]
fn rule_match_feature_pattern() {
    let rule = IpsynRule {
        name: "N4".to_owned(),
        category: 'N',
        include_patterns: vec!["n|".to_owned(), "POSS".to_owned()],
        exclude_patterns: vec![],
        description: "Possessive noun".to_owned(),
    };
    let items = vec![mor_item("n", "dog", &["POSS"])];
    assert!(rule_matches(&items, &rule));

    let no_poss = vec![mor_item("n", "dog", &[])];
    assert!(!rule_matches(&no_poss, &rule));
}

#[test]
fn rule_match_past_tense() {
    let rule = IpsynRule {
        name: "V2".to_owned(),
        category: 'V',
        include_patterns: vec!["v|".to_owned(), "PAST".to_owned()],
        exclude_patterns: vec![],
        description: "Past tense verb".to_owned(),
    };
    let items = vec![mor_item("v", "walk", &["PAST"])];
    assert!(rule_matches(&items, &rule));

    let present = vec![mor_item("v", "walk", &["3S"])];
    assert!(!rule_matches(&present, &rule));
}
