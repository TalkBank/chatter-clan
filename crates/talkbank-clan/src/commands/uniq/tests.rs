use super::*;
use talkbank_model::Span;
use talkbank_model::{Line, MainTier, Terminator, UtteranceContent, Word};

fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    Utterance::new(main)
}

#[test]
fn uniq_basic() {
    let cmd = UniqCommand::new(UniqConfig::default());
    let mut state = UniqState::default();
    let u1 = make_utterance("CHI", &["hello", "world"]);
    let u2 = make_utterance("CHI", &["hello", "world"]);
    let u3 = make_utterance("CHI", &["goodbye"]);
    let chat_file = talkbank_model::ChatFile::new(vec![
        Line::utterance(u1.clone()),
        Line::utterance(u2.clone()),
        Line::utterance(u3.clone()),
    ]);
    let ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    cmd.process_utterance(&u1, &ctx, &mut state);
    cmd.process_utterance(&u2, &ctx, &mut state);
    cmd.process_utterance(&u3, &ctx, &mut state);
    cmd.end_file(&ctx, &mut state);

    let result = cmd.finalize(state);
    // 3 utterance lines + 0 header lines (empty ChatFile)
    assert_eq!(result.total, 3);
    assert_eq!(result.unique, 2);
}

#[test]
fn uniq_sort_by_frequency() {
    let cmd = UniqCommand::new(UniqConfig {
        sort_by_frequency: true,
    });
    let mut state = UniqState::default();
    let u1 = make_utterance("CHI", &["a"]);
    let u2 = make_utterance("CHI", &["b"]);
    let u3 = make_utterance("CHI", &["b"]);
    let u4 = make_utterance("CHI", &["b"]);
    let chat_file = talkbank_model::ChatFile::new(vec![
        Line::utterance(u1.clone()),
        Line::utterance(u2.clone()),
        Line::utterance(u3.clone()),
        Line::utterance(u4.clone()),
    ]);
    let ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    cmd.process_utterance(&u1, &ctx, &mut state);
    cmd.process_utterance(&u2, &ctx, &mut state);
    cmd.process_utterance(&u3, &ctx, &mut state);
    cmd.process_utterance(&u4, &ctx, &mut state);
    cmd.end_file(&ctx, &mut state);

    let result = cmd.finalize(state);
    assert_eq!(result.entries[0].count, 3); // "*chi:\tb ." first (higher frequency)
    assert_eq!(result.entries[1].count, 1); // "*chi:\ta ." second
}

#[test]
fn uniq_empty() {
    let cmd = UniqCommand::new(UniqConfig::default());
    let state = UniqState::default();
    let result = cmd.finalize(state);
    assert_eq!(result.total, 0);
    assert_eq!(result.unique, 0);
    assert!(result.entries.is_empty());
}
