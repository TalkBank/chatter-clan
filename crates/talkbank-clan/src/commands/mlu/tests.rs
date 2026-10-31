use super::*;
use crate::framework::{CommandOutput, FileContext};
use talkbank_model::model::content::word::UntranscribedStatus;
use talkbank_model::{Utterance, UtteranceContent};

/// Build an `MluResult` carrying a single speaker plus the given re-included
/// untranscribed statuses, for exercising the CLAN exclusion-header variants.
fn result_with_re_included(re_included: Vec<UntranscribedStatus>) -> MluResult {
    MluResult {
        speakers: vec![MluSpeakerResult {
            speaker: "CHI".to_owned(),
            utterances: 2,
            morphemes: 3,
            mlu: 1.5,
            sd: 0.5,
            min: 1,
            max: 2,
        }],
        combine_speakers: false,
        re_included_untranscribed: re_included,
        id_rows: None,
    }
}

/// All three CLAN exclusion-header variants (mlu.cpp:246-253), keyed on which of
/// `+sxxx`/`+syyy` are active. Strings are verbatim from CLAN source. `+syyy`
/// alone (and the default) use the same default line because CLAN has NO
/// yyy-only header branch, reproduced here as the `chatter-clan` parity oracle.
#[test]
fn mlu_clan_exclusion_header_variants() {
    use UntranscribedStatus::{Phonetic, Unintelligible};

    // Default: nothing re-included.
    let default = result_with_re_included(vec![]).render_clan();
    assert!(
        default.contains(
            "  MLU (xxx, yyy and www are EXCLUDED from the utterance and morpheme counts):\n"
        ),
        "{default}"
    );

    // +sxxx only: two-line header.
    let xxx = result_with_re_included(vec![Unintelligible]).render_clan();
    assert!(
        xxx.contains(
            "  MLU (xxx is EXCLUDED from the morpheme counts, but is INCLUDED in utterance counts):\n  MLU (yyy and www are EXCLUDED from the utterance and morpheme counts):\n"
        ),
        "{xxx}"
    );

    // +syyy ALONE: CLAN prints the DEFAULT header (no yyy-only branch), even
    // though the yyy utterances are re-included in the count.
    let yyy = result_with_re_included(vec![Phonetic]).render_clan();
    assert!(
        yyy.contains(
            "  MLU (xxx, yyy and www are EXCLUDED from the utterance and morpheme counts):\n"
        ),
        "{yyy}"
    );

    // +sxxx +syyy: combined two-line header.
    let both = result_with_re_included(vec![Unintelligible, Phonetic]).render_clan();
    assert!(
        both.contains(
            "  MLU (xxx and yyy are EXCLUDED from the morpheme counts, but are INCLUDED in utterance counts):\n  MLU (www is EXCLUDED from the utterance and morpheme counts):\n"
        ),
        "{both}"
    );
}

/// Build a minimal utterance with plain words for command tests.
fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(crate::test_support::word(*w))))
        .collect();
    let main = crate::test_support::main_tier(speaker, content);
    Utterance::new(main)
}

/// Without `%mor`, CLAN reports utterances = 0, morphemes = 0 (no fallback).
/// The speaker is still registered and appears in output.
#[test]
fn mlu_no_mor_reports_zero() {
    let command = MluCommand::default();
    let mut state = MluState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["me", "too"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 1);

    let chi = &result.speakers[0];
    assert_eq!(chi.utterances, 0);
    assert_eq!(chi.morphemes, 0);
    assert!((chi.mlu - 0.0).abs() < 1e-10);
}

/// In words_only mode, without %mor, word counting is used as fallback.
#[test]
fn mlu_words_only_counts_words() {
    let config = MluConfig {
        words_only: true,
        ..MluConfig::default()
    };
    let command = MluCommand::new(config);
    let mut state = MluState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // 3 words, 2 words, 4 words → mean = 3.0
    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["me", "too"]);
    let u3 = make_utterance("CHI", &["I", "want", "more", "cookie"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);
    command.process_utterance(&u3, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 1);

    let chi = &result.speakers[0];
    assert_eq!(chi.utterances, 3);
    assert_eq!(chi.morphemes, 9);
    assert!((chi.mlu - 3.0).abs() < 1e-10);
}

/// Solo-word exclusion drops utterances that consist *only* of
/// listed filler words, matching CLAN's `mlu +gum` semantics.
/// Utterances containing the filler plus other words are kept
/// (with the filler counted normally).
#[test]
fn mlu_solo_word_exclusion_drops_solo_um() {
    let config = MluConfig {
        words_only: true,
        solo_word_exclusions: vec!["um".into()],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        ..MluConfig::default()
    };
    let command = MluCommand::new(config);
    let mut state = MluState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // u1: "um" alone, should be dropped (matches `+gum` elision)
    // u2: 3 words including a non-filler, counted, includes "um"
    // u3: 2 words, counted normally
    let u1 = make_utterance("CHI", &["um"]);
    let u2 = make_utterance("CHI", &["um", "I", "see"]);
    let u3 = make_utterance("CHI", &["me", "too"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);
    command.process_utterance(&u3, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 1);

    let chi = &result.speakers[0];
    // Without the solo-word filter we'd count all 3 utterances.
    // With it, u1 is elided ⇒ 2 utterances, 3 + 2 = 5 words.
    assert_eq!(chi.utterances, 2);
    assert_eq!(chi.morphemes, 5);
}

/// Solo-word exclusion is case-insensitive (matches CLAN's
/// case-insensitive default for `+gS`).
#[test]
fn mlu_solo_word_exclusion_is_case_insensitive() {
    let config = MluConfig {
        words_only: true,
        solo_word_exclusions: vec!["UM".into()],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        ..MluConfig::default()
    };
    let command = MluCommand::new(config);
    let mut state = MluState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u1 = make_utterance("CHI", &["um"]);
    let u2 = make_utterance("CHI", &["hello"]);
    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = &result.speakers[0];
    assert_eq!(chi.utterances, 1);
    assert_eq!(chi.morphemes, 1);
}

/// Finalizing empty state should produce no speaker entries.
#[test]
fn mlu_handles_empty_speaker() {
    let command = MluCommand::default();
    let state = MluState::default();

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 0);
}

/// Utterance lengths should be tracked independently per speaker (words_only mode).
#[test]
fn mlu_per_speaker_separation() {
    let config = MluConfig {
        words_only: true,
        ..MluConfig::default()
    };
    let command = MluCommand::new(config);
    let mut state = MluState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u1 = make_utterance("CHI", &["me", "want"]);
    let u2 = make_utterance("MOT", &["you", "can", "have", "it"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 2);

    assert!((result.speakers[0].mlu - 2.0).abs() < 1e-10);
    assert!((result.speakers[1].mlu - 4.0).abs() < 1e-10);
}

/// Text rendering should include key MLU summary values.
#[test]
fn mlu_render_text_format() {
    let result = MluResult {
        speakers: vec![MluSpeakerResult {
            speaker: "CHI".to_owned(),
            utterances: 3,
            morphemes: 9,
            mlu: 3.0,
            sd: 0.816,
            min: 2,
            max: 4,
        }],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        id_rows: None,
    };

    let text = result.render_text();
    assert!(text.contains("Speaker: CHI"));
    assert!(text.contains("Utterances: 3"));
    assert!(text.contains("MLU: 3.000"));
}

/// CLAN rendering should retain legacy line labels and numeric formatting.
#[test]
fn mlu_render_clan_format() {
    let result = MluResult {
        speakers: vec![MluSpeakerResult {
            speaker: "CHI".to_owned(),
            utterances: 2,
            morphemes: 3,
            mlu: 1.5,
            sd: 0.707,
            min: 1,
            max: 2,
        }],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        id_rows: None,
    };

    let clan = result.render_clan();
    assert!(clan.contains("MLU for Speaker: *CHI:"));
    assert!(clan.contains("utterances = 2, morphemes = 3"));
    assert!(clan.contains("Ratio of morphemes over utterances = 1.500"));
    assert!(clan.contains("Standard deviation = 0.707"));
}

/// CLAN rendering with 0 utterances should omit Ratio and SD lines.
#[test]
fn mlu_render_clan_zero_utterances() {
    let result = MluResult {
        speakers: vec![MluSpeakerResult {
            speaker: "CHI".to_owned(),
            utterances: 0,
            morphemes: 0,
            mlu: 0.0,
            sd: 0.0,
            min: 0,
            max: 0,
        }],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        id_rows: None,
    };

    let clan = result.render_clan();
    assert!(clan.contains("MLU for Speaker: *CHI:"));
    assert!(clan.contains("utterances = 0, morphemes = 0"));
    assert!(!clan.contains("Ratio"));
    assert!(!clan.contains("Standard deviation"));
}

/// CLAN rendering with n=1 should show SD as "NA".
#[test]
fn mlu_render_clan_single_utterance() {
    let result = MluResult {
        speakers: vec![MluSpeakerResult {
            speaker: "CHI".to_owned(),
            utterances: 1,
            morphemes: 3,
            mlu: 3.0,
            sd: f64::NAN,
            min: 3,
            max: 3,
        }],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        id_rows: None,
    };

    let clan = result.render_clan();
    assert!(clan.contains("Ratio of morphemes over utterances = 3.000"));
    assert!(clan.contains("Standard deviation = NA"));
}

/// Guards: the CLAN ID-row field order and padded numeric output.
/// Kind: wire
/// Not a type: another program consumes these rendered column bytes.
/// State: an owned result fixture with a parsed header and explicit statistics.
/// Concurrency: none, rendering uses local immutable inputs.
#[test]
fn mlu_id_row_renders_ten_id_fields_and_padded_stats() {
    let id_header = crate::test_support::id_header("eng", "CHI", "Child")
        .with_corpus("corpus")
        .with_age("3;00.")
        .with_sex(talkbank_model::Sex::Female)
        .with_group("TD")
        .with_ses(talkbank_model::SesValue::from_text("MC"));
    let result = MluResult {
        speakers: vec![],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        id_rows: Some(vec![MluIdRow::Speaker {
            file: std::path::PathBuf::from("corpus/mor-gra.cha"),
            id_header: Some(Box::new(id_header)),
            result: MluSpeakerResult {
                speaker: "CHI".to_owned(),
                utterances: 1,
                morphemes: 6,
                mlu: 6.0,
                sd: f64::NAN,
                min: 6,
                max: 6,
            },
        }]),
    };

    assert_eq!(
        result.render_clan(),
        "corpus/mor-gra.cha\teng\tcorpus\tCHI\t3;00.\tfemale\tTD\tMC\tChild\t.\t.\t    1\t    6\t  6.000\tNA\n"
    );
}

/// Guards: SpreadsheetML ID columns and MLU numeric cells.
/// Kind: wire
/// Not a type: spreadsheet readers interpret serialized cell kinds and values.
/// State: a local speaker result and its nonblank ID header.
/// Concurrency: none, workbook generation is synchronous and isolated.
#[test]
fn mlu_spreadsheet_matches_clan_id_columns_and_statistics() {
    let id_header = crate::test_support::id_header("eng", "CHI", "Child")
        .with_corpus("corpus")
        .with_age("3;00.");
    let result = MluResult {
        speakers: vec![],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        id_rows: Some(vec![MluIdRow::Speaker {
            file: std::path::PathBuf::from("corpus/mor-gra.cha"),
            id_header: Some(Box::new(id_header)),
            result: MluSpeakerResult {
                speaker: "CHI".to_owned(),
                utterances: 1,
                morphemes: 6,
                mlu: 6.0,
                sd: f64::NAN,
                min: 6,
                max: 6,
            },
        }]),
    };

    let xml = result
        .to_spreadsheet("mor-gra.mlu.xls", MluCountUnit::Morphemes)
        .expect("spreadsheet mode produces file-speaker rows")
        .write_xml()
        .expect("workbook serializes");
    assert!(xml.contains(r#"<Worksheet ss:Name="mor-gra.mlu.xls">"#));
    assert!(xml.contains(r#"<Data ss:Type="String">Custom_field</Data>"#));
    assert!(xml.contains(r#"<Data ss:Type="String">#morphemes</Data>"#));
    assert!(xml.contains(r#"<Data ss:Type="String">morphemes/utterances</Data>"#));
    assert!(xml.contains(r#"<Data ss:Type="String">mor-gra.cha</Data>"#));
    assert!(xml.contains(r#"<Data ss:Type="Number">6.000</Data>"#));
    assert!(xml.contains(r#"<Data ss:Type="String">NA</Data>"#));
}

#[test]
fn mlu_spreadsheet_words_mode_uses_clan_missing_id_fallback() {
    let result = MluResult {
        speakers: vec![],
        combine_speakers: false,
        re_included_untranscribed: vec![],
        id_rows: Some(vec![MluIdRow::Speaker {
            file: std::path::PathBuf::from("input.cha"),
            id_header: None,
            result: MluSpeakerResult {
                speaker: "CHI".to_owned(),
                utterances: 1,
                morphemes: 3,
                mlu: 3.0,
                sd: f64::NAN,
                min: 3,
                max: 3,
            },
        }]),
    };

    let xml = result
        .to_spreadsheet("input.mlu.xls", MluCountUnit::Words)
        .expect("spreadsheet mode produces file-speaker rows")
        .write_xml()
        .expect("workbook serializes");
    assert!(xml.contains(r#"<Data ss:Type="String">#words</Data>"#));
    assert!(xml.contains(r#"<Data ss:Type="String">words/utterances</Data>"#));
    assert!(xml.contains(r#"<Data ss:Type="String">CHI</Data>"#));
    assert!(xml.contains(r#"<Data ss:Type="Number">3.000</Data>"#));
}

#[test]
fn mlu_id_rows_combine_files_and_emit_zero_speaker_rows_for_o3() {
    let file1 = std::path::PathBuf::from("mor-gra.cha");
    let file2 = std::path::PathBuf::from("second.cha");
    let id_header = crate::test_support::id_header("eng", "CHI", "Child").with_corpus("corpus");
    let speaker = SpeakerCode::from("CHI");
    let mut state = MluState::default();
    state.processed_files.extend([file1.clone(), file2.clone()]);
    state.by_file_speaker.insert(
        (file1.clone(), speaker.clone()),
        SpeakerMlu {
            utterance_lengths: vec![6],
            id_header: Some(id_header.clone()),
        },
    );
    state.by_file_speaker.insert(
        (file2, speaker),
        SpeakerMlu {
            utterance_lengths: vec![5],
            id_header: Some(id_header),
        },
    );

    let command = MluCommand::new(MluConfig {
        output_mode: MluOutputMode::IdRowsCombinedFiles,
        combine_speakers: true,
        ..MluConfig::default()
    });
    let result = command.finalize(state);

    assert_eq!(
        result.render_clan(),
        concat!(
            "mor-gra.cha\teng\tcorpus\tCHI\t.\t.\t.\t.\tChild\t.\t.\t    0\t    0\tNA\tNA\n",
            "mor-gra.cha\t.\t.\t*COMBINED*\t.\t.\t.\t.\t.\t.\t.\t    2\t   11\t  5.500\t  0.500\n",
        )
    );
}
