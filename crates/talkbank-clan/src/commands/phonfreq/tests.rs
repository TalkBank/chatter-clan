use super::*;
use crate::framework::CommandOutput;
use talkbank_model::Span;
use talkbank_model::{DependentTier, MainTier, PhoTier, Terminator, UtteranceContent, Word};

/// Build an utterance with a %pho tier for testing.
fn make_pho_utterance(words: &[&str], pho_tokens: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
    let mut utt = Utterance::new(main);

    let pho_items: Vec<PhoItem> = pho_tokens
        .iter()
        .map(|t| PhoItem::Word(PhoWord::new(t.to_string())))
        .collect();
    utt.dependent_tiers
        .push(DependentTier::Pho(PhoTier::new_pho(pho_items)));

    utt
}

fn file_ctx(chat_file: &talkbank_model::ChatFile) -> FileContext<'_> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

/// A simple `%pho` token should emit one row per lowercase character.
#[test]
fn phonfreq_counts_characters() {
    let cmd = PhonfreqCommand;
    let mut state = PhonfreqState::default();
    let utt = make_pho_utterance(&["hello"], &["abc"]);
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    cmd.process_utterance(&utt, &file_ctx, &mut state);
    let result = cmd.finalize(state);

    assert_eq!(result.entries.len(), 3);

    let a = &result.entries[0];
    assert_eq!(a.phone, "a");
    assert_eq!(a.total, 1);
    assert_eq!(a.initial, 1);
    assert_eq!(a.final_pos, 0);
    assert_eq!(a.other, 0);

    let c = &result.entries[2];
    assert_eq!(c.phone, "c");
    assert_eq!(c.total, 1);
    assert_eq!(c.initial, 0);
    assert_eq!(c.final_pos, 1);
    assert_eq!(c.other, 0);
}

/// Repeated characters should accumulate initial/final/other buckets correctly.
#[test]
fn phonfreq_position_tracking() {
    let cmd = PhonfreqCommand;
    let mut state = PhonfreqState::default();
    let utt = make_pho_utterance(&["word"], &["abcba"]);
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    cmd.process_utterance(&utt, &file_ctx, &mut state);
    let result = cmd.finalize(state);

    let a = result.entries.iter().find(|e| e.phone == "a").unwrap();
    assert_eq!(a.total, 2);
    assert_eq!(a.initial, 1);
    assert_eq!(a.final_pos, 1);
    assert_eq!(a.other, 0);

    let b = result.entries.iter().find(|e| e.phone == "b").unwrap();
    assert_eq!(b.total, 2);
    assert_eq!(b.initial, 0);
    assert_eq!(b.final_pos, 0);
    assert_eq!(b.other, 2);

    let c = result.entries.iter().find(|e| e.phone == "c").unwrap();
    assert_eq!(c.total, 1);
    assert_eq!(c.initial, 0);
    assert_eq!(c.final_pos, 0);
    assert_eq!(c.other, 1);
}

/// Utterances without `%pho` should not affect phone counts.
#[test]
fn phonfreq_skips_utterances_without_pho() {
    let cmd = PhonfreqCommand;
    let mut state = PhonfreqState::default();
    let content: Vec<UtteranceContent> =
        vec![UtteranceContent::Word(Box::new(Word::simple("hello")))];
    let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
    let utt = Utterance::new(main);
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    cmd.process_utterance(&utt, &file_ctx, &mut state);
    let result = cmd.finalize(state);

    assert!(result.entries.is_empty());
}

/// Counts should accumulate across multiple utterances in one state.
#[test]
fn phonfreq_accumulates_across_utterances() {
    let cmd = PhonfreqCommand;
    let mut state = PhonfreqState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    let utt1 = make_pho_utterance(&["one"], &["ab"]);
    let utt2 = make_pho_utterance(&["two"], &["ab"]);

    cmd.process_utterance(&utt1, &file_ctx, &mut state);
    cmd.process_utterance(&utt2, &file_ctx, &mut state);
    let result = cmd.finalize(state);

    let a = result.entries.iter().find(|e| e.phone == "a").unwrap();
    assert_eq!(a.total, 2);
    assert_eq!(a.initial, 2);
}

/// Text rendering should expose all positional counters for each phone.
#[test]
fn phonfreq_render_text() {
    let result = PhonfreqResult {
        entries: vec![PhonfreqEntry {
            phone: "a".to_string(),
            total: 5,
            initial: 2,
            final_pos: 1,
            other: 2,
        }],
    };
    let text = result.render_text();
    assert!(text.contains("5"));
    assert!(text.contains("a"));
    assert!(text.contains("initial =   2"));
    assert!(text.contains("final =   1"));
    assert!(text.contains("other =   2"));
}
