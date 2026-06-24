use super::*;
use talkbank_model::Span;
use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

use crate::framework::{CommandOutput, FileContext};

/// Build a minimal utterance with plain lexical tokens for tests.
fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    Utterance::new(main)
}

fn file_ctx<'a>(chat_file: &'a talkbank_model::ChatFile) -> FileContext<'a> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

#[test]
fn basic_word_length_distribution() {
    let command = WdlenCommand;
    let mut state = WdlenState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // "I" = 1, "want" = 4, "cookie" = 6
    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.word_lengths.len(), 1);

    let chi = &result.word_lengths[0];
    assert_eq!(chi.total_items, 3);
    assert_eq!(format!("{:.3}", chi.mean()), "3.667");
    assert_eq!(chi.distribution[&1], 1);
    assert_eq!(chi.distribution[&4], 1);
    assert_eq!(chi.distribution[&6], 1);
}

#[test]
fn utterance_word_counts() {
    let command = WdlenCommand;
    let mut state = WdlenState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utterance("CHI", &["I", "want"]);
    let u2 = make_utterance("CHI", &["more", "cookie", "please"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    let result = command.finalize(state);
    let chi_utt = &result.utt_word_lengths[0];
    // Utterance 1 has 2 words, utterance 2 has 3 words
    assert_eq!(chi_utt.distribution[&2], 1);
    assert_eq!(chi_utt.distribution[&3], 1);
    assert_eq!(chi_utt.total_items, 2);
}

#[test]
fn turn_detection_across_speakers() {
    let command = WdlenCommand;
    let mut state = WdlenState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // MOT turn: 2 utterances
    let u1 = make_utterance("MOT", &["look", "here"]);
    let u2 = make_utterance("MOT", &["see"]);
    // CHI turn: 1 utterance
    let u3 = make_utterance("CHI", &["yes"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.process_utterance(&u3, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    let result = command.finalize(state);
    // MOT: 1 turn with 2 utterances
    let mot_turn_utts = &result.turn_utt_lengths[0];
    assert_eq!(mot_turn_utts.speaker, "MOT");
    assert_eq!(mot_turn_utts.distribution[&2], 1);

    // CHI: 1 turn with 1 utterance
    let chi_turn_utts = &result.turn_utt_lengths[1];
    assert_eq!(chi_turn_utts.speaker, "CHI");
    assert_eq!(chi_turn_utts.distribution[&1], 1);
}

#[test]
fn empty_state_produces_empty_result() {
    let command = WdlenCommand;
    let state = WdlenState::default();
    let result = command.finalize(state);
    assert!(result.word_lengths.is_empty());
}

#[test]
fn clan_render_format() {
    let command = WdlenCommand;
    let mut state = WdlenState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utterance("CHI", &["I", "want"]);
    let u2 = make_utterance("MOT", &["ok"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    let result = command.finalize(state);
    let clan = result.render_clan();
    // Verify it contains expected section titles
    assert!(clan.contains("Number of words of each length in characters"));
    assert!(clan.contains("Number of utterances of each of these lengths in words"));
    assert!(clan.contains("Number of single turns of each of these lengths in utterances"));
    assert!(clan.contains("Number of single turns of each of these lengths in words"));
    assert!(clan.contains("Number of words of each of these morpheme lengths"));
    assert!(clan.contains("Number of utterances of each of these lengths in morphemes"));
    // Verify separator
    assert!(clan.contains("-------"));
    // Verify speaker labels
    assert!(clan.contains("*CHI:"));
    assert!(clan.contains("*MOT:"));
}
