use super::*;
use talkbank_model::Span;
use talkbank_model::{ChatFile, GemLabel, MainTier, Terminator, UtteranceContent, Word};

/// Build a ChatFile with interleaved headers and utterances.
fn make_chat_file(lines: Vec<Line>) -> ChatFile {
    ChatFile::new(lines)
}

/// Build a test utterance line with simple lexical content.
fn utt_line(speaker: &str, words: &[&str]) -> Line {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    Line::utterance(talkbank_model::Utterance::new(main))
}

/// Build a `@Bg` header line fixture.
fn bg_line(label: &str) -> Line {
    Line::header(Header::BeginGem {
        label: Some(GemLabel::new(label)),
    })
}

/// Build an `@Eg` header line fixture.
fn eg_line(label: &str) -> Line {
    Line::header(Header::EndGem {
        label: Some(GemLabel::new(label)),
    })
}

/// Utterances between matching `@Bg/@Eg` should be attributed to the gem label.
#[test]
fn gemlist_collects_gem_segments() {
    let command = GemlistCommand;
    let mut state = GemlistState::default();

    let chat_file = make_chat_file(vec![
        utt_line("CHI", &["hello"]),        // before gem, not counted
        bg_line("Story"),                   // @Bg:Story
        utt_line("CHI", &["once", "upon"]), // inside gem
        utt_line("MOT", &["a", "time"]),    // inside gem
        eg_line("Story"),                   // @Eg:Story
        utt_line("CHI", &["the", "end"]),   // after gem, not counted
    ]);

    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    command.end_file(&file_ctx, &mut state);
    let result = command.finalize(state);

    assert_eq!(result.gems.len(), 1);
    assert_eq!(result.total_utterances, 2);
    let gem = &result.gems[0];
    assert_eq!(gem.label, "Story");
    assert_eq!(gem.occurrences, 1);
    assert_eq!(gem.utterance_count, 2);
}

/// Files without gem headers should produce an empty result.
#[test]
fn gemlist_no_gems_empty_result() {
    let command = GemlistCommand;
    let mut state = GemlistState::default();

    let chat_file = make_chat_file(vec![utt_line("CHI", &["hello"])]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    command.end_file(&file_ctx, &mut state);
    let result = command.finalize(state);
    assert!(result.gems.is_empty());
}

/// Distinct gem labels in one file should produce separate entries.
#[test]
fn gemlist_multiple_gems() {
    let command = GemlistCommand;
    let mut state = GemlistState::default();

    let chat_file = make_chat_file(vec![
        bg_line("Story"),
        utt_line("CHI", &["once"]),
        eg_line("Story"),
        bg_line("Freeplay"),
        utt_line("MOT", &["let's", "play"]),
        eg_line("Freeplay"),
    ]);

    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    command.end_file(&file_ctx, &mut state);
    let result = command.finalize(state);

    assert_eq!(result.gems.len(), 2);
}

/// Nested gem scopes should count utterances in both active labels as appropriate.
#[test]
fn gemlist_nested_gems() {
    let command = GemlistCommand;
    let mut state = GemlistState::default();

    let chat_file = make_chat_file(vec![
        bg_line("Story"),
        bg_line("Episode"),
        utt_line("CHI", &["hello"]), // in both Story and Episode
        eg_line("Episode"),
        utt_line("CHI", &["bye"]), // in Story only
        eg_line("Story"),
    ]);

    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    command.end_file(&file_ctx, &mut state);
    let result = command.finalize(state);

    assert_eq!(result.gems.len(), 2);

    // Story: "hello" + "bye" = 2 utterances
    let story = result.gems.iter().find(|g| g.label == "Story").unwrap();
    assert_eq!(story.utterance_count, 2);

    // Episode: "hello" = 1 utterance
    let episode = result.gems.iter().find(|g| g.label == "Episode").unwrap();
    assert_eq!(episode.utterance_count, 1);
}
