use super::*;
use crate::framework::FileContext;
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

/// Build a stable `FileContext` fixture reused by command tests.
fn file_ctx(chat_file: &talkbank_model::ChatFile) -> FileContext<'_> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

/// AND expressions should match only when all terms are present.
#[test]
fn combo_and_both_present() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("want+cookie")],
        exclude: vec![],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
}

/// CLAN COMBO `+k` / `--case-sensitive`: search expressions and
/// word stream both stop lowercasing. A lowercase keyword no
/// longer matches an uppercase word, and vice versa.
#[test]
fn combo_case_sensitive_uppercase_keyword_misses_lowercase_word() {
    // Parse the search expression in case-sensitive mode so
    // "Want" stays "Want" instead of being lowercased.
    let expr = SearchExpr::parse_with_case("Want", true);
    let command = ComboCommand::new(ComboConfig {
        search: vec![expr],
        exclude: vec![],
        case_sensitive: true,
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // utterance has lowercase "want", should NOT match the
    // case-sensitive "Want" expression.
    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 0);
}

/// Companion regression: case-sensitive search expression
/// matches when the casing aligns.
#[test]
fn combo_case_sensitive_matches_when_case_aligned() {
    let expr = SearchExpr::parse_with_case("Want", true);
    let command = ComboCommand::new(ComboConfig {
        search: vec![expr],
        exclude: vec![],
        case_sensitive: true,
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "Want", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
}

/// AND expressions should fail when any required term is missing.
#[test]
fn combo_and_missing_one() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("want+cookie")],
        exclude: vec![],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // Has "want" but not "cookie"
    let u = make_utterance("CHI", &["I", "want", "milk"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 0);
}

/// OR expressions should match when any candidate term appears.
#[test]
fn combo_or_either_present() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("cookie,milk")],
        exclude: vec![],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["I", "want", "milk"]);
    let u3 = make_utterance("CHI", &["I", "want", "juice"]);

    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.process_utterance(&u3, &ctx, &mut state);

    assert_eq!(state.matches.len(), 2); // cookie and milk match, juice doesn't
}

/// Multiple `-s` expressions combine with top-level OR semantics.
#[test]
fn combo_multiple_expressions_or() {
    // Multiple -s flags: "want+cookie" OR "need+milk"
    let command = ComboCommand::new(ComboConfig {
        search: vec![
            SearchExpr::parse("want+cookie"),
            SearchExpr::parse("need+milk"),
        ],
        exclude: vec![],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["I", "need", "milk"]);
    let u3 = make_utterance("CHI", &["I", "want", "milk"]); // neither AND matches fully

    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.process_utterance(&u3, &ctx, &mut state);

    assert_eq!(state.matches.len(), 2);
}

/// Exclude expressions drop utterances even when an include
/// expression would match. CLAN's `-sS` semantic for COMBO.
#[test]
fn combo_exclude_drops_matching_utterance() {
    // include: utterance contains "want"
    // exclude: utterance contains "cookie"
    // → "want milk" matches; "want cookie" is dropped
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("want")],
        exclude: vec![SearchExpr::parse("cookie")],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["I", "want", "milk"]);
    let u3 = make_utterance("CHI", &["I", "have", "cookie"]);

    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.process_utterance(&u3, &ctx, &mut state);

    // Only u2 makes it through (matches include, doesn't match exclude).
    // u1 matches include but is dropped by exclude.
    // u3 doesn't match include.
    assert_eq!(state.matches.len(), 1);
    assert!(state.matches[0].utterance_text.contains("milk"));
}

/// CLAN `+g3` (`first_match_only`) short-circuits per utterance:
/// when multiple expressions could match, only the first hit is
/// recorded in `expr_hits`.
#[test]
fn combo_first_match_only_records_only_first_expr() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![
            SearchExpr::parse("cookie"),
            SearchExpr::parse("milk"),
            SearchExpr::parse("want"),
        ],
        exclude: vec![],
        first_match_only: true,
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // Utterance has all three keywords; without +g3 we'd record
    // three matched expressions, with +g3 only the first one.
    let u = make_utterance("CHI", &["I", "want", "cookie", "and", "milk"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
    let m = &state.matches[0];
    assert_eq!(m.expr_hits.len(), 1);
    assert_eq!(m.expr_hits[0].index, 1);
    assert_eq!(m.expr_hits[0].matched_words, vec!["cookie"]);
}

/// CLAN `+g7` (`dedupe_matches`) drops repeated word forms
/// from `matched_words` while preserving first-encounter order.
/// OR expressions over an utterance with repeated keywords are
/// the natural exercise.
#[test]
fn combo_dedupe_matches_removes_repeated_words() {
    // OR expression "cookie,milk" against utterance
    // "cookie cookie milk cookie" produces matched_words
    // ["cookie", "cookie", "milk", "cookie"] without +g7; with
    // +g7 it collapses to ["cookie", "milk"] (first-encounter
    // order).
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("cookie,milk")],
        exclude: vec![],
        dedupe_matches: true,
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["cookie", "cookie", "milk", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
    let m = &state.matches[0];
    assert_eq!(m.expr_hits.len(), 1);
    assert_eq!(m.expr_hits[0].matched_words, vec!["cookie", "milk"]);
}

/// Without `dedupe_matches` the same utterance preserves every
/// occurrence, including duplicates.
#[test]
fn combo_without_dedupe_matches_keeps_duplicates() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("cookie,milk")],
        exclude: vec![],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["cookie", "cookie", "milk", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
    assert_eq!(
        state.matches[0].expr_hits[0].matched_words,
        vec!["cookie", "cookie", "milk", "cookie"]
    );
}

/// Without `first_match_only` the same utterance records all
/// three matching expressions. Companion to the +g3 test above,
/// they share the same input to make the regression obvious.
#[test]
fn combo_without_first_match_only_records_every_expr() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![
            SearchExpr::parse("cookie"),
            SearchExpr::parse("milk"),
            SearchExpr::parse("want"),
        ],
        exclude: vec![],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "cookie", "and", "milk"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
    assert_eq!(state.matches[0].expr_hits.len(), 3);
}

/// Empty exclude config should be a no-op (every include match
/// passes through, matching pre-2026-05-22 behaviour).
#[test]
fn combo_empty_exclude_is_noop() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("want")],
        exclude: vec![],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
}

/// Empty search config should produce no matches.
#[test]
fn combo_empty_search() {
    let command = ComboCommand::new(ComboConfig::default());
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["hello"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert!(result.matches.is_empty());
}

/// CLAN COMBO `+wN` / `--context-after`: emit N utterances
/// immediately following each match as post-context. Same
/// shape as KWAL's context-window machinery, feeds via the
/// `awaiting_after` Vec as later utterances stream by.
#[test]
fn combo_context_after_captures_post_match_lines() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("cookie")],
        context_after: 2,
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

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

/// CLAN COMBO `-wN` / `--context-before`: emit N utterances
/// immediately preceding each match as pre-context. The
/// `ComboState`'s sliding-window ring buffer captures them.
#[test]
fn combo_context_before_captures_pre_match_lines() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("cookie")],
        context_before: 2,
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

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

/// Default (no `+wN`/`-wN`) carries no context, regression
/// companion to the two tests above.
#[test]
fn combo_default_no_context_window() {
    let command = ComboCommand::new(ComboConfig {
        search: vec![SearchExpr::parse("cookie")],
        ..ComboConfig::default()
    });
    let mut state = ComboState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    command.process_utterance(&make_utterance("CHI", &["hello"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["cookie"]), &ctx, &mut state);
    command.process_utterance(&make_utterance("CHI", &["bye"]), &ctx, &mut state);

    assert_eq!(state.matches.len(), 1);
    assert!(state.matches[0].pre_context.is_empty());
    assert!(state.matches[0].post_context.is_empty());
}

/// Parsing should map `+` to AND, `,` to OR, and bare terms to single AND.
#[test]
fn search_expr_parse() {
    match SearchExpr::parse("want+cookie") {
        SearchExpr::And(terms) => assert_eq!(terms, vec!["want", "cookie"]),
        _ => panic!("expected And"),
    }
    match SearchExpr::parse("want,cookie") {
        SearchExpr::Or(terms) => assert_eq!(terms, vec!["want", "cookie"]),
        _ => panic!("expected Or"),
    }
    match SearchExpr::parse("want") {
        SearchExpr::And(terms) => assert_eq!(terms, vec!["want"]),
        _ => panic!("expected And with single term"),
    }
}
