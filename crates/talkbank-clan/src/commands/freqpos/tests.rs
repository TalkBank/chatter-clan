use super::*;
use crate::framework::CommandOutput;
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

/// `+d` (`FirstSecondOther`) reclassifies position 1 as the
/// "second" slot and pushes positions ≥ 2 to "other". For
/// `[I, want, a, cookie]`:
///   default: I=initial, cookie=final, want+a=other
///   +d:      I=initial, want=second, a+cookie=other
#[test]
fn freqpos_second_mode_reclassifies_position_one() {
    let command = FreqposCommand::new(FreqposConfig {
        position_classification: PositionClassification::FirstSecondOther,
        case_sensitive: false,
    });
    let mut state = FreqposState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "a", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.total_initial, 1); // I
    assert_eq!(result.total_final, 1); // want (position 1; counter reused as "second")
    assert_eq!(result.total_other, 2); // a + cookie
    assert_eq!(result.total_one_word, 0);

    // Render uses "second" label, not "final".
    let clan = result.render_clan();
    assert!(
        clan.contains("Number of words in a second position"),
        "expected 'second' footer label, got:\n{clan}"
    );
    assert!(
        !clan.contains("Number of words in a final position"),
        "default 'final' label should NOT appear in +d mode"
    );
    assert!(clan.contains("second = "));
    assert!(!clan.contains("final = "));
}

/// Default mode (`FirstLastOther`) renders with "final" label.
/// Companion to the +d test for an obvious diff.
#[test]
fn freqpos_default_mode_keeps_final_label() {
    let command = FreqposCommand::default();
    let mut state = FreqposState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "a", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.total_initial, 1); // I
    assert_eq!(result.total_final, 1); // cookie
    assert_eq!(result.total_other, 2); // want + a

    let clan = result.render_clan();
    assert!(clan.contains("Number of words in a final position"));
    assert!(clan.contains("final = "));
}

/// Multi-word utterances should split counts across initial/other/final buckets.
#[test]
fn freqpos_position_tracking() {
    let command = FreqposCommand::default();
    let mut state = FreqposState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // "I want cookie" → I=initial, want=other, cookie=final
    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.entries.len(), 3);
    assert_eq!(result.total_initial, 1);
    assert_eq!(result.total_other, 1);
    assert_eq!(result.total_final, 1);
    assert_eq!(result.total_one_word, 0);
}

/// Single-token utterances should increment only the one-word bucket.
#[test]
fn freqpos_one_word_utterance() {
    let command = FreqposCommand::default();
    let mut state = FreqposState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["hello"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.total_one_word, 1);
    assert_eq!(result.total_initial, 0);
}

/// Finalizing untouched state should produce empty entries and zero totals.
#[test]
fn freqpos_empty_state() {
    let command = FreqposCommand::default();
    let state = FreqposState::default();
    let result = command.finalize(state);
    assert!(result.entries.is_empty());
}

/// CLAN FREQPOS `+k` / `--case-sensitive`: word keying preserves
/// original case. Without `+k`, the by-word entries collapse
/// case variants under one normalized key.
#[test]
fn freqpos_case_sensitive_splits_case_variants() {
    let command = FreqposCommand::new(FreqposConfig {
        case_sensitive: true,
        ..FreqposConfig::default()
    });
    let mut state = FreqposState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["Want", "want", "WANT"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    // Three distinct keys, one for each case variant.
    assert_eq!(result.entries.len(), 3);
}

/// Companion regression: default lowercases the key, collapsing
/// the three case variants into one entry.
#[test]
fn freqpos_default_collapses_case_variants() {
    let command = FreqposCommand::default();
    let mut state = FreqposState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["Want", "want", "WANT"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.entries.len(), 1);
}
