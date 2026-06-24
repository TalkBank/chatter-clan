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

#[test]
fn chains_basic() {
    let cmd = ChainsCommand::new(ChainsConfig::default());
    let mut state = ChainsState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // Three consecutive utterances with code "A" = chain of length 3
    let u1 = make_utt_with_cod("CHI", "A");
    let u2 = make_utt_with_cod("CHI", "A");
    let u3 = make_utt_with_cod("CHI", "A");
    // Then break the chain
    let u4 = make_utt_with_cod("CHI", "B");

    cmd.process_utterance(&u1, &ctx, &mut state);
    cmd.process_utterance(&u2, &ctx, &mut state);
    cmd.process_utterance(&u3, &ctx, &mut state);
    cmd.process_utterance(&u4, &ctx, &mut state);

    let result = cmd.finalize(state);
    assert_eq!(result.speakers.len(), 1);
    let chi = &result.speakers[0];
    let a_stats = chi.codes.iter().find(|c| c.code == "A").unwrap();
    assert_eq!(a_stats.num_chains, 1);
    assert_eq!(a_stats.max_length, 3);
}

#[test]
fn chains_empty() {
    let cmd = ChainsCommand::new(ChainsConfig::default());
    let state = ChainsState::default();
    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
}
