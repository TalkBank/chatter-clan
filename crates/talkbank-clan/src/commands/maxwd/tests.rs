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

/// Longest lexical item should surface first with its character count.
#[test]
fn maxwd_finds_longest_words() {
    let command = MaxwdCommand::default();
    let mut state = MaxwdState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["hippopotamus", "is", "big"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 1);

    let chi = &result.speakers[0];
    assert_eq!(chi.max_length, 12);
    assert_eq!(chi.top_words[0].1, "hippopotamus");
    assert_eq!(chi.top_words[0].0, 12);
}

/// Configured output limit should cap number of reported longest words.
#[test]
fn maxwd_respects_limit() {
    let config = MaxwdConfig {
        limit: crate::framework::WordLimit::new(2),
        ..MaxwdConfig::default()
    };
    let command = MaxwdCommand::new(config);
    let mut state = MaxwdState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["a", "bb", "ccc", "dddd", "eeeee"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = &result.speakers[0];
    assert_eq!(chi.top_words.len(), 2);
    assert_eq!(chi.top_words[0].1, "eeeee");
    assert_eq!(chi.top_words[1].1, "dddd");
}

/// `+xN` (`exclude_lengths`) drops words whose character
/// length is in the exclusion set.
#[test]
fn maxwd_exclude_lengths_drops_listed_lengths() {
    let config = MaxwdConfig {
        limit: crate::framework::WordLimit::new(20),
        exclude_lengths: vec![2, 4],
        ..MaxwdConfig::default()
    };
    let command = MaxwdCommand::new(config);
    let mut state = MaxwdState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["a", "bb", "ccc", "dddd", "eeeee"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = &result.speakers[0];
    let words: Vec<&str> = chi.top_words.iter().map(|(_, w)| w.as_str()).collect();
    assert_eq!(words, vec!["eeeee", "ccc", "a"]);
    assert_eq!(chi.max_length, 5);
}

/// `+a` (`unique_length_only`) drops words whose length is
/// shared with another word in the same speaker's lexicon.
#[test]
fn maxwd_unique_length_only_drops_shared_length_words() {
    let config = MaxwdConfig {
        limit: crate::framework::WordLimit::new(20),
        unique_length_only: true,
        ..MaxwdConfig::default()
    };
    let command = MaxwdCommand::new(config);
    let mut state = MaxwdState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["a", "bb", "ccc", "dddd", "eeeee", "fffff"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = &result.speakers[0];
    let lengths: Vec<usize> = chi.top_words.iter().map(|(len, _)| *len).collect();
    assert!(
        !lengths.contains(&5),
        "length-5 words should be dropped, got {lengths:?}"
    );
    assert_eq!(chi.top_words.len(), 4);
    assert_eq!(chi.top_words[0].1, "dddd");
    assert_eq!(chi.top_words[0].0, 4);
    assert_eq!(chi.max_length, 4);
}

/// Default (without +a) keeps every length, including shared ones.
#[test]
fn maxwd_default_keeps_shared_length_words() {
    let command = MaxwdCommand::default();
    let mut state = MaxwdState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["a", "bb", "ccc", "dddd", "eeeee", "fffff"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = &result.speakers[0];
    assert_eq!(chi.top_words.len(), 6);
    assert_eq!(chi.max_length, 5);
}

/// Repeated tokens should increment totals but keep one unique-word entry.
#[test]
fn maxwd_deduplicates_words() {
    let command = MaxwdCommand::default();
    let mut state = MaxwdState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["cookie", "cookie", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = &result.speakers[0];
    assert_eq!(chi.unique_words, 1);
    assert_eq!(chi.total_words, 3);
    assert_eq!(chi.top_words.len(), 1);
}

/// Finalizing untouched state should return no speaker sections.
#[test]
fn maxwd_empty_state() {
    let command = MaxwdCommand::default();
    let state = MaxwdState::default();

    let result = command.finalize(state);
    assert!(result.speakers.is_empty());
}

/// CLAN MAXWD `+k` / `--case-sensitive`: case variants are
/// treated as distinct words, so the deduplicated word table
/// preserves all three.
#[test]
fn maxwd_case_sensitive_splits_case_variants() {
    let command = MaxwdCommand::new(MaxwdConfig {
        case_sensitive: true,
        ..MaxwdConfig::default()
    });
    let mut state = MaxwdState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["Want", "want", "WANT"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = result
        .speakers
        .iter()
        .find(|s| s.speaker == "CHI")
        .expect("CHI speaker");
    assert_eq!(chi.unique_words, 3);
}

/// Default lowercases, collapsing the three case variants.
#[test]
fn maxwd_default_collapses_case_variants() {
    let command = MaxwdCommand::default();
    let mut state = MaxwdState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["Want", "want", "WANT"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = result
        .speakers
        .iter()
        .find(|s| s.speaker == "CHI")
        .expect("CHI speaker");
    assert_eq!(chi.unique_words, 1);
}
