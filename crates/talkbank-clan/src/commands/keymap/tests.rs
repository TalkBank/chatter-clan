use super::*;
use talkbank_model::Span;
use talkbank_model::{CodTier, DependentTier, MainTier, Terminator, UtteranceContent, Word};

fn make_utt_with_cod(speaker: &str, cod_text: &str) -> Utterance {
    let content = vec![UtteranceContent::Word(Box::new(Word::simple("hello")))];
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    let mut utt = Utterance::new(main);
    utt.dependent_tiers
        .push(DependentTier::Cod(CodTier::from_text(cod_text)));
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

#[test]
fn keymap_basic() {
    let cmd = KeymapCommand::new(KeymapConfig {
        keywords: vec![crate::framework::KeywordPattern::from("A")],
        tier: crate::framework::TierKind::Cod,
    });
    let mut state = KeymapState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // Keyword "A" followed by "B" in next utterance
    let u1 = make_utt_with_cod("CHI", "A");
    let u2 = make_utt_with_cod("MOT", "B");

    cmd.process_utterance(&u1, &ctx, &mut state);
    cmd.process_utterance(&u2, &ctx, &mut state);

    let result = cmd.finalize(state);
    assert_eq!(result.data.len(), 1);
    assert_eq!(result.data[0].keyword, "A");
    assert_eq!(result.data[0].total, 1);
}

#[test]
fn keymap_does_not_treat_selectors_as_keywords() {
    let cmd = KeymapCommand::new(KeymapConfig {
        keywords: vec![crate::framework::KeywordPattern::from("$WR")],
        tier: crate::framework::TierKind::Cod,
    });
    let mut state = KeymapState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utt_with_cod("CHI", "<w4> $WR");
    let u2 = make_utt_with_cod("MOT", "B");

    cmd.process_utterance(&u1, &ctx, &mut state);
    cmd.process_utterance(&u2, &ctx, &mut state);

    let result = cmd.finalize(state);
    assert_eq!(result.data.len(), 1);
    assert_eq!(result.data[0].keyword, "$WR");
    assert_eq!(result.data[0].total, 1);
    assert_eq!(result.data[0].following[0].code, "B");
}

#[test]
fn keymap_empty() {
    let cmd = KeymapCommand::new(KeymapConfig::default());
    let state = KeymapState::default();
    let result = cmd.finalize(state);
    assert!(result.data.is_empty());
}
