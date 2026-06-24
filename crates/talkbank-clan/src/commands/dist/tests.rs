use super::*;
use talkbank_model::Span;
use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

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

/// Every utterance counts as its own turn (matching CLAN behavior).
#[test]
fn dist_turn_counting() {
    let command = DistCommand::default();
    let mut state = DistState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // CHI → MOT → CHI = 3 utterances = 3 turns
    let u1 = make_utterance("CHI", &["hello"]);
    let u2 = make_utterance("MOT", &["hi"]);
    let u3 = make_utterance("CHI", &["bye"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.process_utterance(&u3, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.total_turns, 3);
}

/// Consecutive same-speaker utterances each count as a turn (CLAN behavior).
#[test]
fn dist_same_speaker_still_increments_turns() {
    let command = DistCommand::default();
    let mut state = DistState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // CHI → CHI → CHI = 3 utterances = 3 turns
    let u1 = make_utterance("CHI", &["hello"]);
    let u2 = make_utterance("CHI", &["there"]);
    let u3 = make_utterance("CHI", &["bye"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.process_utterance(&u3, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.total_turns, 3);
}

/// `+g` (`once_per_turn`) deduplicates repeated words within
/// a single turn, `hello hello bye` counts `hello` once, not
/// twice. `first_turn` / `last_turn` are unchanged by the flag.
#[test]
fn dist_once_per_turn_collapses_repeats_in_one_turn() {
    let command = DistCommand::new(DistConfig {
        once_per_turn: true,
        case_sensitive: false,
    });
    let mut state = DistState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // Turn 1 has "hello" twice + "bye"; Turn 2 has "hello".
    // Default: total_count(hello)=3, total_count(bye)=1.
    // +g:      total_count(hello)=2 (one per turn), bye=1.
    let u1 = make_utterance("CHI", &["hello", "hello", "bye"]);
    let u2 = make_utterance("MOT", &["hello"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);

    let result = command.finalize(state);
    let hello = result.words.iter().find(|w| w.word == "hello").unwrap();
    assert_eq!(hello.total_count, 2);
    assert_eq!(hello.first_turn, 1);
    assert_eq!(hello.last_turn, 2);
    let bye = result.words.iter().find(|w| w.word == "bye").unwrap();
    assert_eq!(bye.total_count, 1);
}

/// Default behaviour (without `+g`) still counts every occurrence.
/// Companion to the once-per-turn test for an obvious diff.
#[test]
fn dist_default_counts_every_occurrence() {
    let command = DistCommand::default();
    let mut state = DistState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utterance("CHI", &["hello", "hello", "bye"]);
    let u2 = make_utterance("MOT", &["hello"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);

    let result = command.finalize(state);
    let hello = result.words.iter().find(|w| w.word == "hello").unwrap();
    assert_eq!(hello.total_count, 3);
}

/// Word entries should retain first and last turn positions across speakers.
#[test]
fn dist_word_first_last_turn() {
    let command = DistCommand::default();
    let mut state = DistState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // Turn 1: CHI says "hello", Turn 2: MOT says "hello"
    let u1 = make_utterance("CHI", &["hello"]);
    let u2 = make_utterance("MOT", &["hello"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);

    let result = command.finalize(state);
    let hello = result.words.iter().find(|w| w.word == "hello").unwrap();
    assert_eq!(hello.first_turn, 1);
    assert_eq!(hello.last_turn, 2);
    assert_eq!(hello.total_count, 2);
}

/// Finalizing untouched state should produce zero turns and no words.
#[test]
fn dist_empty_state() {
    let command = DistCommand::default();
    let state = DistState::default();
    let result = command.finalize(state);
    assert!(result.words.is_empty());
    assert_eq!(result.total_turns, 0);
}

/// CLAN DIST `+k` / `--case-sensitive`: case variants land in
/// separate by-word entries.
#[test]
fn dist_case_sensitive_splits_case_variants() {
    let command = DistCommand::new(DistConfig {
        case_sensitive: true,
        ..DistConfig::default()
    });
    let mut state = DistState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["Want", "want", "WANT"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    // Three distinct keys.
    assert_eq!(result.words.len(), 3);
}

/// Default lowercases the key, collapsing case variants into
/// one entry.
#[test]
fn dist_default_collapses_case_variants() {
    let command = DistCommand::default();
    let mut state = DistState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["Want", "want", "WANT"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.words.len(), 1);
}
