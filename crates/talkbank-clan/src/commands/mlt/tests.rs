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

fn file_ctx(chat_file: &talkbank_model::ChatFile) -> FileContext<'_> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

/// Consecutive same-speaker utterances should collapse into one turn.
#[test]
fn mlt_single_speaker_single_turn() {
    let command = MltCommand::default();
    let mut state = MltState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    // Three consecutive utterances by CHI = one turn
    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["me", "too"]);
    let u3 = make_utterance("CHI", &["please"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);
    command.process_utterance(&u3, &file_ctx, &mut state);
    command.end_file(&file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 1);

    let chi = &result.speakers[0];
    assert_eq!(chi.turns, 1);
    assert_eq!(chi.utterances, 3);
    assert_eq!(chi.words, 6); // 3 + 2 + 1
    assert!((chi.mlt_utterances - 3.0).abs() < 1e-10);
    assert!((chi.mlt_words - 6.0).abs() < 1e-10);
}

/// Speaker switches should close the previous turn and start a new one.
#[test]
fn mlt_turn_boundaries() {
    let command = MltCommand::default();
    let mut state = MltState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    // CHI → MOT → CHI creates 3 turns (CHI has 2 turns, MOT has 1)
    let u1 = make_utterance("CHI", &["I", "want"]);
    let u2 = make_utterance("CHI", &["more"]);
    let u3 = make_utterance("MOT", &["here", "you", "go"]);
    let u4 = make_utterance("CHI", &["thanks"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);
    command.process_utterance(&u3, &file_ctx, &mut state);
    command.process_utterance(&u4, &file_ctx, &mut state);
    command.end_file(&file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 2);

    // CHI: 2 turns (2 utterances/3 words, then 1 utterance/1 word)
    let chi = &result.speakers[0];
    assert_eq!(chi.speaker, "CHI");
    assert_eq!(chi.turns, 2);
    assert_eq!(chi.utterances, 3);
    assert_eq!(chi.words, 4); // 2 + 1 + 1
    assert!((chi.mlt_utterances - 1.5).abs() < 1e-10);
    assert!((chi.mlt_words - 2.0).abs() < 1e-10);

    // MOT: 1 turn (1 utterance/3 words)
    let mot = &result.speakers[1];
    assert_eq!(mot.speaker, "MOT");
    assert_eq!(mot.turns, 1);
    assert_eq!(mot.utterances, 1);
    assert_eq!(mot.words, 3);
}

/// Finalizing untouched state should return no speaker rows.
#[test]
fn mlt_empty_state() {
    let command = MltCommand::default();
    let state = MltState::default();

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 0);
}

/// Text rendering should include core MLT summary values.
#[test]
fn mlt_render_text_format() {
    let result = MltResult {
        speakers: vec![MltSpeakerResult {
            speaker: "CHI".to_owned(),
            turns: 2,
            utterances: 3,
            words: 6,
            mlt_words: 3.0,
            mlt_utterances: 1.5,
            words_per_utterance: 2.0,
            sd: 1.0,
        }],
    };

    let text = result.render_text();
    assert!(text.contains("Speaker: CHI"));
    assert!(text.contains("Turns: 2"));
    assert!(text.contains("MLT (words): 3.000"));
}

/// CLAN rendering should preserve legacy labels and ratios.
#[test]
fn mlt_render_clan_format() {
    let result = MltResult {
        speakers: vec![MltSpeakerResult {
            speaker: "CHI".to_owned(),
            turns: 2,
            utterances: 2,
            words: 3,
            mlt_words: 1.5,
            mlt_utterances: 1.0,
            words_per_utterance: 1.5,
            sd: 0.5,
        }],
    };

    let clan = result.render_clan();
    assert!(clan.contains("MLT for Speaker: *CHI:"));
    assert!(clan.contains("utterances = 2, turns = 2, words = 3"));
    assert!(clan.contains("Ratio of words over turns = 1.500"));
    assert!(clan.contains("Standard deviation = 0.500"));
}
