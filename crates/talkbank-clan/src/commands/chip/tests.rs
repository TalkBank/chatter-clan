use super::*;
use crate::framework::{CommandOutput, FileContext};
use talkbank_model::{ChatFile, Utterance, UtteranceContent};

fn command() -> ChipCommand {
    let config = ChipConfig::default()
        .with_main_tier()
        .expect("default CHIP morphology mode is not active")
        .with_adult(SpeakerCode::parse("MOT").expect("speaker code is valid"))
        .expect("adult role is unique")
        .with_child(SpeakerCode::parse("CHI").expect("speaker code is valid"))
        .expect("child role is unique");
    ChipCommand::new(config)
}

fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content = words
        .iter()
        .map(|word| UtteranceContent::Word(Box::new(crate::test_support::word(word))))
        .collect();
    Utterance::new(crate::test_support::main_tier(speaker, content))
}

fn file_context(chat_file: &ChatFile) -> FileContext<'_> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

#[test]
fn scores_adult_response_to_nearest_prior_child_not_only_adjacent_speaker() {
    let command = command();
    let chat_file = ChatFile::new(vec![]);
    let context = file_context(&chat_file);
    let mut state = ChipState::default();

    for utterance in [
        make_utterance("CHI", &["want", "cookie"]),
        make_utterance("MOT", &["good", "job"]),
        make_utterance("CHI", &["want", "cookie"]),
        make_utterance("MOT", &["want", "cookie"]),
    ] {
        command.process_utterance(&utterance, &context, &mut state);
    }

    let file = state.active_file.as_ref().expect("file was initialized");
    assert_eq!(file.tallies.adult.responses, 1);
    assert_eq!(file.tallies.adult.exact_match, 1);
    assert_eq!(file.tallies.child.responses, 0);
    assert_eq!(file.tallies.child.no_overlap, 1);
}

#[test]
fn unknown_speakers_are_not_assigned_a_role_by_default() {
    let command = ChipCommand::new(ChipConfig::default().with_main_tier().unwrap());
    let chat_file = ChatFile::new(vec![]);
    let context = file_context(&chat_file);
    let mut state = ChipState::default();

    for utterance in [
        make_utterance("MOT", &["hello"]),
        make_utterance("CHI", &["hello"]),
    ] {
        command.process_utterance(&utterance, &context, &mut state);
    }

    let file = state.active_file.as_ref().expect("file was initialized");
    assert_eq!(file.tallies.adult.responses, 0);
    assert_eq!(file.tallies.child.responses, 0);
    assert_eq!(file.tallies.adult.utters, 0);
    assert_eq!(file.tallies.child.utters, 0);
}

#[test]
fn ending_a_file_produces_a_matrix_and_echoes_typed_chat() {
    let command = command();
    let chat_file = ChatFile::new(vec![]);
    let context = file_context(&chat_file);
    let mut state = ChipState::default();

    command.process_utterance(&make_utterance("CHI", &["want"]), &context, &mut state);
    command.process_utterance(&make_utterance("MOT", &["want"]), &context, &mut state);
    command.end_file(&context, &mut state);

    let result = command.finalize(state);
    let [report] = result.files.as_slice() else {
        panic!("one input file should produce one report");
    };
    assert!(matches!(report.status, ChipFileStatus::Scored(_)));
    assert!(report.transcript.contains("*CHI:"));
    assert!(report.transcript.contains("%adu:"));
    assert!(result.render_clan().contains("File: test.cha"));
}

#[test]
fn empty_invocation_produces_no_file_reports() {
    assert!(command().finalize(ChipState::default()).files.is_empty());
}
