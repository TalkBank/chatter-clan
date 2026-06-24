use super::*;
use crate::framework::FileContext;
use talkbank_model::Span;
use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

/// Build a minimal utterance with plain words for interaction tests.
fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    Utterance::new(main)
}

/// Build a stable `FileContext` fixture reused across test cases.
fn file_ctx(chat_file: &talkbank_model::ChatFile) -> FileContext<'_> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

/// Identical adjacent content across speakers should classify as exact repetition.
#[test]
fn chip_exact_repetition() {
    let command = ChipCommand;
    let mut state = ChipState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // MOT says "want cookie", CHI repeats "want cookie"
    let u1 = make_utterance("MOT", &["want", "cookie"]);
    let u2 = make_utterance("CHI", &["want", "cookie"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    let pair = SpeakerPair {
        from: "MOT".to_owned(),
        to: "CHI".to_owned(),
    };
    assert_eq!(state.by_pair[&pair].exact_repetitions, 1);
    assert_eq!(state.by_pair[&pair].overlaps, 0);
    assert_eq!(state.by_pair[&pair].no_overlaps, 0);
}

/// At least 50% overlap of the smaller unique-word set counts as overlap.
#[test]
fn chip_overlap() {
    let command = ChipCommand;
    let mut state = ChipState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // MOT says "I want cookie", CHI says "want cookie please"
    // Shared: "want", "cookie" (2 of 3 unique words in shorter) → ≥50%
    let u1 = make_utterance("MOT", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["want", "cookie", "please"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    let pair = SpeakerPair {
        from: "MOT".to_owned(),
        to: "CHI".to_owned(),
    };
    assert_eq!(state.by_pair[&pair].exact_repetitions, 0);
    assert_eq!(state.by_pair[&pair].overlaps, 1);
}

/// Disjoint vocabularies should classify as no-overlap.
#[test]
fn chip_no_overlap() {
    let command = ChipCommand;
    let mut state = ChipState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // MOT: "look at the dog", CHI: "I want milk"
    // No shared words
    let u1 = make_utterance("MOT", &["look", "at", "the", "dog"]);
    let u2 = make_utterance("CHI", &["I", "want", "milk"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    let pair = SpeakerPair {
        from: "MOT".to_owned(),
        to: "CHI".to_owned(),
    };
    assert_eq!(state.by_pair[&pair].no_overlaps, 1);
}

/// Consecutive utterances by the same speaker should not create an interaction edge.
#[test]
fn chip_same_speaker_no_interaction() {
    let command = ChipCommand;
    let mut state = ChipState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // Two consecutive utterances by same speaker, no interaction counted
    let u1 = make_utterance("CHI", &["hello"]);
    let u2 = make_utterance("CHI", &["hello"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    assert!(state.by_pair.is_empty());
}

/// The state machine should track multiple directed interactions in one file.
#[test]
fn chip_multiple_interactions() {
    let command = ChipCommand;
    let mut state = ChipState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utterance("MOT", &["want", "cookie"]);
    let u2 = make_utterance("CHI", &["want", "cookie"]); // exact
    let u3 = make_utterance("MOT", &["good", "job"]);
    let u4 = make_utterance("CHI", &["more", "cookie"]); // no overlap with "good job"

    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);
    command.process_utterance(&u3, &ctx, &mut state);
    command.process_utterance(&u4, &ctx, &mut state);
    command.end_file(&ctx, &mut state);

    let mot_to_chi = SpeakerPair {
        from: "MOT".to_owned(),
        to: "CHI".to_owned(),
    };
    assert_eq!(state.by_pair[&mot_to_chi].exact_repetitions, 1);
    assert_eq!(state.by_pair[&mot_to_chi].no_overlaps, 1);

    // CHI → MOT: "want cookie" → "good job" = no overlap
    let chi_to_mot = SpeakerPair {
        from: "CHI".to_owned(),
        to: "MOT".to_owned(),
    };
    assert_eq!(state.by_pair[&chi_to_mot].no_overlaps, 1);
}

/// Finalizing untouched state should produce an empty result.
#[test]
fn chip_empty_state() {
    let command = ChipCommand;
    let state = ChipState::default();
    let result = command.finalize(state);
    assert!(result.pairs.is_empty());
}

/// Word order differences alone should still be exact repetition.
#[test]
fn classify_interaction_exact() {
    assert_eq!(
        classify_interaction(
            &["want".to_owned(), "cookie".to_owned()],
            &["cookie".to_owned(), "want".to_owned()],
        ),
        Interaction::ExactRepetition
    );
}

/// Exactly 50% overlap should take the overlap branch.
#[test]
fn classify_interaction_overlap_threshold() {
    // 1 of 2 unique words shared = 50% → overlap
    assert_eq!(
        classify_interaction(
            &["want".to_owned(), "cookie".to_owned()],
            &["want".to_owned(), "milk".to_owned()],
        ),
        Interaction::Overlap
    );
}

/// Zero shared vocabulary should take the no-overlap branch.
#[test]
fn classify_interaction_no_overlap() {
    // 0 of 2 shared → no overlap
    assert_eq!(
        classify_interaction(
            &["hello".to_owned(), "world".to_owned()],
            &["goodbye".to_owned(), "moon".to_owned()],
        ),
        Interaction::NoOverlap
    );
}
