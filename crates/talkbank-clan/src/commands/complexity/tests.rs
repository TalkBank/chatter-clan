use super::*;
use talkbank_model::Span;
use talkbank_model::{
    DependentTier, GraTier, GrammaticalRelation, MainTier, Terminator, UtteranceContent, Word,
};

use crate::framework::FileContext;

fn make_utterance_with_gra(
    speaker: &str,
    words: &[&str],
    relations: Vec<GrammaticalRelation>,
) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    let mut utt = Utterance::new(main);
    utt.dependent_tiers
        .push(DependentTier::Gra(GraTier::new_gra(relations)));
    utt
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
fn counts_ud_relations() {
    let cmd = ComplexityCommand;
    let mut state = ComplexityState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance_with_gra(
        "CHI",
        &["he", "said", "wants", "go"],
        vec![
            GrammaticalRelation::new(1, 2, "APPOS"),
            GrammaticalRelation::new(2, 0, "ROOT"),
            GrammaticalRelation::new(3, 2, "CCOMP"),
            GrammaticalRelation::new(4, 3, "XCOMP"),
        ],
    );
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    assert_eq!(result.style, RelationStyle::Ud);
    assert_eq!(result.speakers.len(), 1);

    let sp = &result.speakers[0];
    assert_eq!(sp.speaker, "CHI");
    assert_eq!(sp.appos, 1);
    assert_eq!(sp.ccomp, 1);
    assert_eq!(sp.xcomp, 1);
    assert_eq!(sp.tokens, 3);
    assert_eq!(sp.total_tokens, 4); // ROOT counted too
}

#[test]
fn counts_legacy_relations() {
    let cmd = ComplexityCommand;
    let mut state = ComplexityState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance_with_gra(
        "PAR",
        &["he", "said", "that"],
        vec![
            GrammaticalRelation::new(1, 2, "SUBJ"),
            GrammaticalRelation::new(2, 0, "ROOT"),
            GrammaticalRelation::new(3, 2, "COMP"),
        ],
    );
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    assert_eq!(result.style, RelationStyle::Legacy);

    let sp = &result.speakers[0];
    assert_eq!(sp.comp, 1);
    assert_eq!(sp.tokens, 1);
    assert_eq!(sp.total_tokens, 3);
}

#[test]
fn skips_punct() {
    let cmd = ComplexityCommand;
    let mut state = ComplexityState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance_with_gra(
        "CHI",
        &["go"],
        vec![
            GrammaticalRelation::new(1, 0, "ROOT"),
            GrammaticalRelation::new(2, 1, "PUNCT"),
        ],
    );
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    let sp = &result.speakers[0];
    assert_eq!(sp.total_tokens, 1); // PUNCT excluded
}

#[test]
fn no_gra_tier_skips_utterance() {
    let cmd = ComplexityCommand;
    let mut state = ComplexityState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let content = vec![UtteranceContent::Word(Box::new(Word::simple("hello")))];
    let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
    let u = Utterance::new(main);
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
}

#[test]
fn ratio_calculation() {
    let cmd = ComplexityCommand;
    let mut state = ComplexityState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance_with_gra(
        "CHI",
        &["he", "said", "he", "wants", "go"],
        vec![
            GrammaticalRelation::new(1, 2, "APPOS"),
            GrammaticalRelation::new(2, 0, "ROOT"),
            GrammaticalRelation::new(3, 4, "EXPL"),
            GrammaticalRelation::new(4, 2, "CCOMP"),
            GrammaticalRelation::new(5, 4, "XCOMP"),
        ],
    );
    cmd.process_utterance(&u, &ctx, &mut state);

    let result = cmd.finalize(state);
    let sp = &result.speakers[0];
    // APPOS(1) + EXPL(1) + CCOMP(1) + XCOMP(1) = 4 tokens, 5 total
    assert_eq!(sp.tokens, 4);
    assert_eq!(sp.total_tokens, 5);
    assert!((sp.ratio() - 0.8).abs() < 0.001);
}
