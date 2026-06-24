use super::*;
use talkbank_model::Span;
use talkbank_model::{
    CodTier, DependentTier, MainTier, Terminator, Utterance, UtteranceContent, Word,
};

fn make_utterance_with_cod(speaker: &str, cod_text: &str) -> Utterance {
    let content = vec![UtteranceContent::Word(Box::new(Word::simple("hello")))];
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    let mut utt = Utterance::new(main);
    utt.dependent_tiers
        .push(DependentTier::Cod(CodTier::from_text(cod_text)));
    utt
}

#[test]
fn codes_basic() {
    let cmd = CodesCommand::new(CodesConfig::default());
    let mut state = CodesState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u1 = make_utterance_with_cod("CHI", "AC:DI IC:DI");
    let u2 = make_utterance_with_cod("CHI", "AC:DI");

    cmd.process_utterance(&u1, &ctx, &mut state);
    cmd.process_utterance(&u2, &ctx, &mut state);

    let result = cmd.finalize(state);
    assert_eq!(result.speakers.len(), 1);
    assert_eq!(result.speakers[0].total, 3);
}

#[test]
fn codes_do_not_count_selectors_as_codes() {
    let cmd = CodesCommand::new(CodesConfig::default());
    let mut state = CodesState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance_with_cod("CHI", "<w4> $WR <w5> seep");
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    let chi = &result.speakers[0];
    assert_eq!(chi.total, 2);
    assert!(chi.entries.iter().any(|e| e.code == "$WR" && e.count == 1));
    assert!(chi.entries.iter().any(|e| e.code == "seep" && e.count == 1));
    assert!(
        !chi.entries
            .iter()
            .any(|e| e.code == "<w4>" || e.code == "<w5>")
    );
}

#[test]
fn codes_empty() {
    let cmd = CodesCommand::new(CodesConfig::default());
    let state = CodesState::default();
    let result = cmd.finalize(state);
    assert_eq!(result.total, 0);
    assert!(result.speakers.is_empty());
}
