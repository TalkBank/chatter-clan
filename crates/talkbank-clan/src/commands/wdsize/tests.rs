use super::*;
use talkbank_model::Span;
use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    Utterance::new(main)
}

fn file_ctx(chat_file: &talkbank_model::ChatFile) -> FileContext<'_> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

#[test]
fn main_tier_word_sizes() {
    let cmd = WdsizeCommand::new(WdsizeConfig {
        use_main_tier: true,
        ..WdsizeConfig::default()
    });
    let mut state = WdsizeState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // "I" = 1, "want" = 4, "cookie" = 6
    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    assert_eq!(result.speakers.len(), 1);
    let sp = &result.speakers[0];
    assert_eq!(sp.total_words, 3);
    assert_eq!(sp.distribution[&1], 1);
    assert_eq!(sp.distribution[&4], 1);
    assert_eq!(sp.distribution[&6], 1);
    assert!((sp.mean() - 3.667).abs() < 0.01);
}

/// `+w>4` (`LengthFilter::GreaterThan`, threshold 4) drops
/// `"I"` (1) and `"want"` (4); only `"cookie"` (6) enters
/// the histogram.
#[test]
fn length_filter_greater_than() {
    let cmd = WdsizeCommand::new(WdsizeConfig {
        use_main_tier: true,
        length_filter: Some(LengthFilter {
            comparator: LengthComparator::GreaterThan,
            threshold: 4,
        }),
    });
    let mut state = WdsizeState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    let sp = &result.speakers[0];
    assert_eq!(sp.total_words, 1);
    assert_eq!(sp.distribution.get(&6).copied(), Some(1));
    assert!(!sp.distribution.contains_key(&1));
    assert!(!sp.distribution.contains_key(&4));
}

/// `+w<5` keeps lengths strictly less than 5: `"I"` (1) and
/// `"want"` (4) pass; `"cookie"` (6) does not.
#[test]
fn length_filter_less_than() {
    let cmd = WdsizeCommand::new(WdsizeConfig {
        use_main_tier: true,
        length_filter: Some(LengthFilter {
            comparator: LengthComparator::LessThan,
            threshold: 5,
        }),
    });
    let mut state = WdsizeState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    let sp = &result.speakers[0];
    assert_eq!(sp.total_words, 2);
    assert_eq!(sp.distribution.get(&1).copied(), Some(1));
    assert_eq!(sp.distribution.get(&4).copied(), Some(1));
    assert!(!sp.distribution.contains_key(&6));
}

/// `+w=4` keeps only length-4 words: `"want"` passes.
#[test]
fn length_filter_equal() {
    let cmd = WdsizeCommand::new(WdsizeConfig {
        use_main_tier: true,
        length_filter: Some(LengthFilter {
            comparator: LengthComparator::Equal,
            threshold: 4,
        }),
    });
    let mut state = WdsizeState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    let sp = &result.speakers[0];
    assert_eq!(sp.total_words, 1);
    assert_eq!(sp.distribution.get(&4).copied(), Some(1));
}

/// `LengthFilter::includes` direct unit-tests for the three
/// comparators. Edge cases: `>0` admits everything positive;
/// `<0` admits nothing; `=0` admits only zero-length input.
#[test]
fn length_filter_includes_predicate() {
    let gt5 = LengthFilter {
        comparator: LengthComparator::GreaterThan,
        threshold: 5,
    };
    assert!(!gt5.includes(5));
    assert!(gt5.includes(6));
    assert!(!gt5.includes(0));

    let lt5 = LengthFilter {
        comparator: LengthComparator::LessThan,
        threshold: 5,
    };
    assert!(lt5.includes(4));
    assert!(!lt5.includes(5));

    let eq3 = LengthFilter {
        comparator: LengthComparator::Equal,
        threshold: 3,
    };
    assert!(eq3.includes(3));
    assert!(!eq3.includes(2));
    assert!(!eq3.includes(4));
}

/// `FromStr` parses the `gt:N` / `lt:N` / `eq:N` shape that
/// the rewriter emits.
#[test]
fn length_filter_from_str_parses_rewriter_output() {
    use std::str::FromStr;
    assert_eq!(
        LengthFilter::from_str("gt:4").unwrap(),
        LengthFilter {
            comparator: LengthComparator::GreaterThan,
            threshold: 4,
        }
    );
    assert_eq!(
        LengthFilter::from_str("lt:5").unwrap(),
        LengthFilter {
            comparator: LengthComparator::LessThan,
            threshold: 5,
        }
    );
    assert_eq!(
        LengthFilter::from_str("eq:3").unwrap(),
        LengthFilter {
            comparator: LengthComparator::Equal,
            threshold: 3,
        }
    );
    assert!(LengthFilter::from_str("garbage").is_err());
    assert!(LengthFilter::from_str("xx:4").is_err());
    assert!(LengthFilter::from_str("gt:notanumber").is_err());
}

#[test]
fn empty_state_produces_empty_result() {
    let cmd = WdsizeCommand::default();
    let state = WdsizeState::default();
    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
}
