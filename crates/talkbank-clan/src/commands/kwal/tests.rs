use super::*;
use crate::framework::{CommandOutput, FileContext};
use talkbank_model::Span;
use talkbank_model::{MainTier, Terminator, Utterance, UtteranceContent, Word};

/// Build a minimal utterance with plain lexical tokens for tests.
fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    Utterance::new(main)
}

/// `+b` (`strict_match`) restricts matches to utterances
/// whose entire tier consists of exactly one keyword.
/// `["want"]` matches; `["I", "want", "cookie"]` does not,
/// even though it contains "want".
#[test]
fn kwal_strict_match_only_solo_word_matches() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("want")],
        strict_match: true,
        case_sensitive: false,
        legal_chat: false,
        context_before: 0,
        context_after: 0,
    });
    let mut state = KwalState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let solo = make_utterance("CHI", &["want"]);
    let mixed = make_utterance("CHI", &["I", "want", "cookie"]);

    command.process_utterance(&solo, &file_ctx, &mut state);
    command.process_utterance(&mixed, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].utterance_text, "*CHI:\twant .");
}

/// Default (no `+b`) still matches the keyword anywhere on the
/// tier. Companion to the strict-match test for an obvious diff.
#[test]
fn kwal_default_matches_anywhere_on_tier() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("want")],
        strict_match: false,
        case_sensitive: false,
        legal_chat: false,
        context_before: 0,
        context_after: 0,
    });
    let mut state = KwalState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let solo = make_utterance("CHI", &["want"]);
    let mixed = make_utterance("CHI", &["I", "want", "cookie"]);

    command.process_utterance(&solo, &file_ctx, &mut state);
    command.process_utterance(&mixed, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.matches.len(), 2);
}

/// Matching keywords should produce one row per matching utterance.
#[test]
fn kwal_finds_keyword() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("cookie")],
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["more", "milk"]);
    let u3 = make_utterance("MOT", &["have", "a", "cookie"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);
    command.process_utterance(&u3, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.matches.len(), 2);
    assert_eq!(result.keyword_counts["cookie"], 2);
}

/// Keyword matching should be case-insensitive.
#[test]
fn kwal_case_insensitive() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("WANT")],
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
}

/// `+k` / `--case-sensitive`: an uppercase keyword should NOT
/// match a lowercase word, and vice versa. Pinned by contrast
/// against `kwal_case_insensitive` above.
#[test]
fn kwal_case_sensitive_uppercase_keyword_misses_lowercase_word() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("WANT")],
        case_sensitive: true,
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    assert_eq!(state.matches.len(), 0);
}

/// CLAN KWAL `+d` (no N) / `--legal-chat`: render emits ONLY
/// the matching utterance lines as legal CHAT, no `---`
/// separators and no `*** File ... Keyword: X` decoration. The
/// default render keeps the decoration.
///
/// Per CLAN manual §7.16.7 (+d, no number): "Normally, kwal
/// outputs the location of the tier where the match occurs.
/// When the +d switch is turned on you can [output] in these
/// formats: ... outputs legal CHAT format."
#[test]
fn kwal_legal_chat_format_drops_location_decoration() {
    let result = KwalResult {
        matches: vec![
            KwalMatch {
                filename: "test".to_owned(),
                speaker: "CHI".to_owned(),
                utterance_text: "*CHI:\tI want a cookie .".to_owned(),
                line_number: 6,
                keyword: "want".to_owned(),
                pre_context: Vec::new(),
                post_context: Vec::new(),
            },
            KwalMatch {
                filename: "test".to_owned(),
                speaker: "MOT".to_owned(),
                utterance_text: "*MOT:\tI Want milk .".to_owned(),
                line_number: 7,
                keyword: "want".to_owned(),
                pre_context: Vec::new(),
                post_context: Vec::new(),
            },
        ],
        keyword_counts: IndexMap::new(),
        legal_chat: true,
    };
    let clan = result.render_clan();
    assert!(clan.contains("*CHI:\tI want a cookie ."));
    assert!(clan.contains("*MOT:\tI Want milk ."));
    assert!(
        !clan.contains("***"),
        "legal-chat format must not emit `*** File ...` decoration: {clan:?}"
    );
    assert!(
        !clan.contains("----------------------------------------"),
        "legal-chat format must not emit `---` separators: {clan:?}"
    );
}

/// Default render (legal_chat=false) keeps the location
/// decoration, regression companion to the +d test above.
#[test]
fn kwal_default_render_keeps_location_decoration() {
    let result = KwalResult {
        matches: vec![KwalMatch {
            filename: "test".to_owned(),
            speaker: "CHI".to_owned(),
            utterance_text: "*CHI:\tI want a cookie .".to_owned(),
            line_number: 6,
            keyword: "want".to_owned(),
            pre_context: Vec::new(),
            post_context: Vec::new(),
        }],
        keyword_counts: IndexMap::new(),
        legal_chat: false,
    };
    let clan = result.render_clan();
    assert!(clan.contains("*** File \"pipeout\": line 6. Keyword: want"));
    assert!(clan.contains("----------------------------------------"));
    assert!(clan.contains("*CHI:\tI want a cookie ."));
}

/// CLAN KWAL `+wN` (`--context-after N`) emits the N
/// utterances immediately following each match as post-context.
/// Each `KwalMatch.post_context` is filled lazily as later
/// utterances stream by `process_utterance`.
#[test]
fn kwal_context_after_captures_post_match_lines() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("cookie")],
        context_after: 2,
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    command.process_utterance(&make_utterance("CHI", &["hello"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["milk"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["cookie"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["thanks"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["bye"]), &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
    let m = &state.matches[0];
    assert_eq!(m.post_context.len(), 2);
    assert!(m.post_context[0].contains("thanks"));
    assert!(m.post_context[1].contains("bye"));
}

/// CLAN KWAL `-wN` (`--context-before N`) emits the N
/// utterances immediately preceding each match as pre-context.
/// The `KwalState`'s sliding-window ring buffer holds the most
/// recent N utterances; on a match they're snapshotted into
/// `KwalMatch.pre_context`.
#[test]
fn kwal_context_before_captures_pre_match_lines() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("cookie")],
        context_before: 2,
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    command.process_utterance(&make_utterance("CHI", &["hello"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["world"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["milk"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["cookie"]), &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
    let m = &state.matches[0];
    assert_eq!(m.pre_context.len(), 2);
    assert!(m.pre_context[0].contains("world"));
    assert!(m.pre_context[1].contains("milk"));
}

/// Default (no `+wN`/`-wN`) carries no pre- or post-context.
/// Regression companion to the two tests above.
#[test]
fn kwal_default_no_context_window() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("cookie")],
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    command.process_utterance(&make_utterance("CHI", &["hello"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["cookie"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["bye"]), &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
    assert!(state.matches[0].pre_context.is_empty());
    assert!(state.matches[0].post_context.is_empty());
}

/// `+k` companion: case must match exactly, uppercase keyword
/// matches uppercase word; default lowercase keyword still
/// matches lowercase word.
#[test]
fn kwal_case_sensitive_matches_when_case_aligned() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("Want")],
        case_sensitive: true,
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["I", "Want", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
}

/// Exact keyword should NOT match partial words (CLAN parity).
#[test]
fn kwal_exact_match_no_substring() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("cook")],
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    assert_eq!(state.matches.len(), 0);
}

/// Wildcard `*` should enable partial matching (CLAN parity).
#[test]
fn kwal_wildcard_match() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("cook*")],
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
}

/// The `word_pattern_matches` function should handle wildcards correctly.
#[test]
fn keyword_matches_patterns() {
    use crate::framework::word_pattern_matches;

    assert!(word_pattern_matches("cookie", "cookie"));
    assert!(!word_pattern_matches("cookies", "cookie"));
    assert!(word_pattern_matches("cookie", "cook*"));
    assert!(word_pattern_matches("cookies", "cook*"));
    assert!(!word_pattern_matches("book", "cook*"));
    assert!(word_pattern_matches("going", "*ing"));
    assert!(!word_pattern_matches("gong", "*ing"));
    assert!(word_pattern_matches("cookie", "*oki*"));
    assert!(!word_pattern_matches("cook", "*oki*"));
    assert!(word_pattern_matches("anything", "*"));
}

/// Non-matching keywords should leave output collections empty.
#[test]
fn kwal_no_matches() {
    let command = KwalCommand::new(KwalConfig {
        keywords: vec![crate::framework::KeywordPattern::from("zebra")],
        ..KwalConfig::default()
    });
    let mut state = KwalState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert!(result.matches.is_empty());
    assert!(result.keyword_counts.is_empty());
}

/// Empty keyword configuration should short-circuit to no matches.
#[test]
fn kwal_empty_keywords() {
    let command = KwalCommand::new(KwalConfig::default());
    let mut state = KwalState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["hello"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert!(result.matches.is_empty());
}
