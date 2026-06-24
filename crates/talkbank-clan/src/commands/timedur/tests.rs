use super::output::format_duration_ms;
use super::*;
use crate::framework::{CommandOutput, FileContext};
use talkbank_model::Span;
use talkbank_model::content::Bullet;
use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

/// Build a minimal utterance fixture with explicit bullet timing.
fn make_timed_utterance(speaker: &str, words: &[&str], start_ms: u64, end_ms: u64) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let mut main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    main.content.bullet = Some(Bullet::new(start_ms, end_ms));
    Utterance::new(main)
}

/// Timed utterances should contribute to totals, means, min/max, and summary.
#[test]
fn timedur_basic_timing() {
    let command = TimedurCommand;
    let mut state = TimedurState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // 2 seconds, then 1.5 seconds
    let u1 = make_timed_utterance("CHI", &["hello"], 0, 2000);
    let u2 = make_timed_utterance("CHI", &["world"], 2000, 3500);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);

    let result = command.finalize(state);
    // 1 speaker + summary
    assert_eq!(result.speakers.len(), 1);
    assert!(result.summary.is_some());

    let chi = &result.speakers[0];
    assert_eq!(chi.timed_utterances, 2);
    assert_eq!(format_duration_ms(chi.total_ms), "3.500s");
    assert_eq!(format_duration_ms(chi.mean_ms), "1.750s");
    assert_eq!(format_duration_ms(chi.min_ms), "1.500s");
    assert_eq!(format_duration_ms(chi.max_ms), "2.000s");
}

/// Speaker-specific aggregates should stay separate while summary spans all speakers.
#[test]
fn timedur_multiple_speakers() {
    let command = TimedurCommand;
    let mut state = TimedurState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u1 = make_timed_utterance("CHI", &["hi"], 0, 1000);
    let u2 = make_timed_utterance("MOT", &["hello"], 1000, 3000);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);

    let result = command.finalize(state);
    // CHI + MOT speakers, plus summary
    assert_eq!(result.speakers.len(), 2);
    let summary = result.summary.as_ref().expect("expected summary");
    assert_eq!(summary.total_utterances, 2);
    assert_eq!(format_duration_ms(summary.span_ms), "3.000s");
}

/// Untimed utterances should not produce speaker rows or a summary section.
#[test]
fn timedur_skips_untimed() {
    let command = TimedurCommand;
    let mut state = TimedurState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // Utterance without bullet timing
    let content = vec![UtteranceContent::Word(Box::new(Word::simple("hello")))];
    let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
    let utterance = Utterance::new(main);

    command.process_utterance(&utterance, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert!(result.speakers.is_empty());
    assert!(result.summary.is_none());
}

/// Short durations should stay in seconds-only format.
#[test]
fn format_duration_short() {
    assert_eq!(format_duration_ms(0), "0.000s");
    assert_eq!(format_duration_ms(1500), "1.500s");
    assert_eq!(format_duration_ms(500), "0.500s");
}

/// Longer durations should include a minute component.
#[test]
fn format_duration_with_minutes() {
    assert_eq!(format_duration_ms(65000), "1m 5.000s");
    assert_eq!(format_duration_ms(120000), "2m 0.000s");
}

/// Interaction matrix header should match CLAN format for two speakers.
#[test]
fn timedur_render_clan_header_two_speakers() {
    let result = TimedurResult {
        speakers: vec![],
        summary: None,
        seen_speakers: vec!["CHI".to_owned(), "MOT".to_owned()],
    };
    let clan = result.render_clan();
    assert_eq!(clan, " #  Cur|  CHI  |CHI-CHI|CHI-MOT|  MOT  |MOT-MOT|\n");
}

/// Untimed utterances should still track seen speakers for the interaction matrix.
#[test]
fn timedur_untimed_tracks_speakers() {
    let command = TimedurCommand;
    let mut state = TimedurState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // Two untimed utterances from different speakers
    let content1 = vec![UtteranceContent::Word(Box::new(Word::simple("hello")))];
    let main1 = MainTier::new("CHI", content1, Terminator::Period { span: Span::DUMMY });
    let u1 = Utterance::new(main1);

    let content2 = vec![UtteranceContent::Word(Box::new(Word::simple("hi")))];
    let main2 = MainTier::new("MOT", content2, Terminator::Period { span: Span::DUMMY });
    let u2 = Utterance::new(main2);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert!(result.speakers.is_empty());
    assert_eq!(result.seen_speakers, vec!["CHI", "MOT"]);

    let clan = result.render_clan();
    assert_eq!(clan, " #  Cur|  CHI  |CHI-CHI|CHI-MOT|  MOT  |MOT-MOT|\n");
}

/// Empty result with no speakers should produce empty clan output.
#[test]
fn timedur_render_clan_no_speakers() {
    let result = TimedurResult {
        speakers: vec![],
        summary: None,
        seen_speakers: vec![],
    };
    assert_eq!(result.render_clan(), "");
}
