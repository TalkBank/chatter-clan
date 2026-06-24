use super::super::domain_types::{GemLabel, WordPattern};
use super::*;

/// Empty speaker filters should allow any speaker code.
#[test]
fn speaker_filter_empty_matches_all() {
    let filter = SpeakerFilter::default();
    let speaker: SpeakerCode = "CHI".into();
    assert!(filter.matches(&speaker));
}

/// An include list should restrict matches to listed speakers only.
#[test]
fn speaker_filter_include_restricts() {
    let filter = SpeakerFilter {
        include: vec!["CHI".into(), "MOT".into()],
        exclude: vec![],
    };
    assert!(filter.matches(&"CHI".into()));
    assert!(filter.matches(&"MOT".into()));
    assert!(!filter.matches(&"FAT".into()));
}

/// Excluded speakers should fail even when include is empty.
#[test]
fn speaker_filter_exclude_removes() {
    let filter = SpeakerFilter {
        include: vec![],
        exclude: vec!["INV".into()],
    };
    assert!(filter.matches(&"CHI".into()));
    assert!(!filter.matches(&"INV".into()));
}

/// Empty gem filters should match whether or not a gem is active.
#[test]
fn gem_filter_empty_matches_all() {
    let filter = GemFilter::default();
    assert!(filter.matches(&[]));
    assert!(filter.matches(&["Story".to_owned()]));
}

/// Gem include filters require at least one active matching label.
#[test]
fn gem_filter_include_requires_match() {
    let filter = GemFilter {
        include: vec![GemLabel::from("Story")],
        exclude: vec![],
    };
    assert!(!filter.matches(&[]));
    assert!(filter.matches(&["Story".to_owned()]));
    assert!(filter.matches(&["story".to_owned()])); // case-insensitive
    assert!(!filter.matches(&["Narrative".to_owned()]));
}

/// Gem exclude filters should reject matching labels case-insensitively.
#[test]
fn gem_filter_exclude_blocks_match() {
    let filter = GemFilter {
        include: vec![],
        exclude: vec![GemLabel::from("Warmup")],
    };
    assert!(filter.matches(&[]));
    assert!(filter.matches(&["Story".to_owned()]));
    assert!(!filter.matches(&["Warmup".to_owned()]));
    assert!(!filter.matches(&["warmup".to_owned()]));
}

/// Empty word filters should not gate utterances.
#[test]
fn word_filter_empty_matches_all() {
    let filter = WordFilter::default();
    let utterance = make_test_utterance(&["hello", "world"]);
    assert!(filter.matches(&utterance));
}

/// Include patterns should require at least one lexical match.
#[test]
fn word_filter_include_requires_match() {
    let filter = WordFilter {
        include: vec![WordPattern::from("hello")],
        exclude: vec![],
        ..WordFilter::default()
    };
    let matching = make_test_utterance(&["hello", "world"]);
    assert!(filter.matches(&matching));

    let non_matching = make_test_utterance(&["goodbye", "world"]);
    assert!(!filter.matches(&non_matching));
}

/// Word include matching is case-insensitive.
#[test]
fn word_filter_include_case_insensitive() {
    let filter = WordFilter {
        include: vec![WordPattern::from("Hello")],
        exclude: vec![],
        ..WordFilter::default()
    };
    let utterance = make_test_utterance(&["hello", "world"]);
    assert!(filter.matches(&utterance));
}

/// CLAN's `+k` flag flips matching to case-sensitive, a
/// lower-case word should NOT match a capitalised pattern.
#[test]
fn word_filter_case_sensitive_pattern_does_not_match_other_case() {
    let filter = WordFilter {
        include: vec![WordPattern::from("Hello")],
        exclude: vec![],
        case_sensitive: true,
        ..WordFilter::default()
    };
    let lower = make_test_utterance(&["hello", "world"]);
    assert!(!filter.matches(&lower));

    let mixed = make_test_utterance(&["Hello", "world"]);
    assert!(filter.matches(&mixed));
}

/// Word includes use exact match semantics (CLAN parity).
#[test]
fn word_filter_include_exact() {
    let filter = WordFilter {
        include: vec![WordPattern::from("hello")],
        exclude: vec![],
        ..WordFilter::default()
    };
    let utterance = make_test_utterance(&["hello", "world"]);
    assert!(filter.matches(&utterance));

    // Substring should NOT match
    let filter_sub = WordFilter {
        include: vec![WordPattern::from("ell")],
        exclude: vec![],
        ..WordFilter::default()
    };
    assert!(!filter_sub.matches(&utterance));
}

/// Wildcard `*` enables partial matching in word filters.
#[test]
fn word_filter_include_wildcard() {
    let filter = WordFilter {
        include: vec![WordPattern::from("hel*")],
        exclude: vec![],
        ..WordFilter::default()
    };
    let utterance = make_test_utterance(&["hello", "world"]);
    assert!(filter.matches(&utterance));
}

/// Exclude patterns block utterances containing any matching word.
#[test]
fn word_filter_exclude_blocks() {
    let filter = WordFilter {
        include: vec![],
        exclude: vec![WordPattern::from("world")],
        ..WordFilter::default()
    };
    let blocked = make_test_utterance(&["hello", "world"]);
    assert!(!filter.matches(&blocked));

    let allowed = make_test_utterance(&["hello", "there"]);
    assert!(filter.matches(&allowed));
}

/// Exclude matches win when both include and exclude patterns match.
#[test]
fn word_filter_include_and_exclude() {
    let filter = WordFilter {
        include: vec![WordPattern::from("hello")],
        exclude: vec![WordPattern::from("world")],
        ..WordFilter::default()
    };
    // Has include match but also has exclude match → blocked
    let blocked = make_test_utterance(&["hello", "world"]);
    assert!(!filter.matches(&blocked));

    // Has include match, no exclude match → pass
    let allowed = make_test_utterance(&["hello", "there"]);
    assert!(filter.matches(&allowed));
}

/// The utterance range filter uses inclusive 1-based bounds.
#[test]
fn utterance_range_filters() {
    let config = FilterConfig {
        utterance_range: Some(UtteranceRange::new(2, 4).expect("valid test range")),
        ..FilterConfig::default()
    };
    let utterance = make_test_utterance(&["hello"]);
    let gems: Vec<String> = vec![];

    assert!(!config.matches(&utterance, &gems, 1));
    assert!(config.matches(&utterance, &gems, 2));
    assert!(config.matches(&utterance, &gems, 3));
    assert!(config.matches(&utterance, &gems, 4));
    assert!(!config.matches(&utterance, &gems, 5));
}

/// Utterance ranges should parse from CLAN-style `start-end` strings.
#[test]
fn utterance_range_parses() {
    let range = "25-125"
        .parse::<UtteranceRange>()
        .expect("range should parse");
    assert_eq!(range.start(), 25);
    assert_eq!(range.end(), 125);
    assert_eq!(range.to_string(), "25-125");
}

/// Invalid utterance ranges should report whether syntax or bounds failed.
#[test]
fn utterance_range_rejects_invalid_input() {
    assert!(matches!(
        "oops".parse::<UtteranceRange>(),
        Err(ParseUtteranceRangeError::InvalidFormat { .. })
    ));
    assert!(matches!(
        "0-5".parse::<UtteranceRange>(),
        Err(ParseUtteranceRangeError::InvalidBounds { .. })
    ));
    assert!(matches!(
        "9-3".parse::<UtteranceRange>(),
        Err(ParseUtteranceRangeError::InvalidBounds { .. })
    ));
}

/// Reads one pattern per non-comment line; skips blanks,
/// `# `-comments, and `;%* `-annotation lines (CLAN's
/// `cutt.cpp::rdexclf` conventions).
#[test]
fn load_word_list_file_strips_comments_and_blanks() {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::with_suffix(".cut").expect("tmp file");
    file.write_all(
        "\u{feff}# leading comment\n\
         want\n\
         \n\
         cookie   \n\
         ;%* annotation marker, skipped\n\
         milk\t\n\
         # another comment\n\
         juice"
            .as_bytes(),
    )
    .expect("write tmp word-list");
    let patterns = super::load_word_list_file(file.path()).expect("load");
    let texts: Vec<&str> = patterns.iter().map(|p| p.as_str()).collect();
    assert_eq!(texts, vec!["want", "cookie", "milk", "juice"]);
}

/// Missing files surface as `LoadWordListError::Io` with the
/// original path attached, the CLI maps this to a CLAN-style
/// stderr message.
#[test]
fn load_word_list_file_missing_path_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bogus = dir.path().join("never.cut");
    let err = super::load_word_list_file(&bogus).expect_err("should fail");
    match err {
        super::LoadWordListError::Io { path, .. } => assert_eq!(path, bogus),
    }
}

/// COMBO's `+s@FILE` shares the file format but returns raw
/// expression lines (parsed downstream by `SearchExpr::parse`),
/// not `WordPattern`s.
#[test]
fn load_search_expr_file_keeps_expression_lines_intact() {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::with_suffix(".cut").expect("tmp file");
    file.write_all(
        "\u{feff}# search expressions\n\
         want+cookie\n\
         \n\
         milk,juice\n\
         ;%* annotated boolean, skipped\n\
         hello"
            .as_bytes(),
    )
    .expect("write tmp search-list");
    let exprs = super::load_search_expr_file(file.path()).expect("load");
    assert_eq!(exprs, vec!["want+cookie", "milk,juice", "hello"]);
}

/// Build a minimal Utterance with the given words for filter testing.
fn make_test_utterance(words: &[&str]) -> talkbank_model::Utterance {
    use talkbank_model::Span;
    use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
    talkbank_model::Utterance::new(main)
}
