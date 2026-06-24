use super::*;
use talkbank_model::Line;

fn parse_content(chat: &str) -> Vec<UtteranceContent> {
    let parsed = talkbank_transform::parse_and_validate(
        chat,
        talkbank_model::ParseValidateOptions::default(),
    )
    .unwrap();
    parsed
        .lines
        .into_iter()
        .find_map(|line| match line {
            Line::Utterance(utt) => Some(utt.main.content.content.0),
            _ => None,
        })
        .expect("expected utterance")
}

#[test]
fn flucalc_empty() {
    let cmd = FlucalcCommand::new(FlucalcConfig::default());
    let state = FlucalcState::default();
    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
}

#[test]
fn count_filled_pauses() {
    let mut fluency = SpeakerFluency::default();
    let content =
        parse_content("@UTF8\n@Begin\n@Languages:\teng\n*CHI:\tI &-um want &-uh that .\n@End\n");
    count_disfluencies(&content, &mut fluency);
    assert_eq!(fluency.filled_pauses, 2);
    assert_eq!(fluency.total_words, 3);
}

#[test]
fn count_prolongations() {
    let mut fluency = SpeakerFluency::default();
    let content = parse_content("@UTF8\n@Begin\n@Languages:\teng\n*CHI:\tI wa:nt that .\n@End\n");
    count_disfluencies(&content, &mut fluency);
    assert_eq!(fluency.prolongations, 1);
    assert_eq!(fluency.total_words, 3);
}

#[test]
fn count_phrase_reps_and_revisions() {
    let mut fluency = SpeakerFluency::default();
    let content = parse_content(
        "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\t<I want> [/] want <that> [//] this .\n@End\n",
    );
    count_disfluencies(&content, &mut fluency);
    assert_eq!(fluency.phrase_reps, 1);
    assert_eq!(fluency.revisions, 1);
}

#[test]
fn sld_td_percentages() {
    let sp = SpeakerFluency {
        total_words: 100,
        prolongations: 3,
        whole_word_reps: 2,
        filled_pauses: 5,
        revisions: 3,
        ..Default::default()
    };
    assert_eq!(sp.total_sld(), 5);
    assert_eq!(sp.total_td(), 8);
    assert!((sp.sld_pct() - 5.0).abs() < f64::EPSILON);
    assert!((sp.td_pct() - 8.0).abs() < f64::EPSILON);
}
