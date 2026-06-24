use super::*;
use crate::framework::CommandOutput;
use talkbank_model::Span;
use talkbank_model::{
    DependentTier, MainTier, PhoTier, Terminator, Utterance, UtteranceContent, Word,
};

/// Build an utterance with %mod and %pho tiers.
fn make_mod_pho_utterance(words: &[&str], mod_tokens: &[&str], pho_tokens: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
    let mut utt = Utterance::new(main);

    // Add %mod tier
    let mod_items: Vec<PhoItem> = mod_tokens
        .iter()
        .map(|t| PhoItem::Word(PhoWord::new(t.to_string())))
        .collect();
    utt.dependent_tiers
        .push(DependentTier::Mod(PhoTier::new_mod(mod_items)));

    // Add %pho tier
    let pho_items: Vec<PhoItem> = pho_tokens
        .iter()
        .map(|t| PhoItem::Word(PhoWord::new(t.to_string())))
        .collect();
    utt.dependent_tiers
        .push(DependentTier::Pho(PhoTier::new_pho(pho_items)));

    utt
}

/// Build a minimal FileContext for testing.
fn file_ctx(chat_file: &talkbank_model::ChatFile) -> FileContext<'_> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

/// Matching `%mod`/`%pho` lengths should pair tokens positionally one-to-one.
#[test]
fn modrep_basic_pairing() {
    let cmd = ModrepCommand;
    let mut state = ModrepState::default();
    let utt = make_mod_pho_utterance(&["A", "B", "C"], &["d", "e", "f"], &["a", "b", "c"]);
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    cmd.process_utterance(&utt, &file_ctx, &mut state);
    let result = cmd.finalize(state);

    assert_eq!(result.speakers.len(), 1);
    let speaker = &result.speakers[0];
    assert_eq!(speaker.speaker, "CHI");
    assert_eq!(speaker.entries.len(), 3);

    // BTreeMap sorts alphabetically: d, e, f
    let d = &speaker.entries[0];
    assert_eq!(d.model, "d");
    assert_eq!(d.total, 1);
    assert_eq!(d.replicas.len(), 1);
    assert_eq!(d.replicas[0].word, "a");
    assert_eq!(d.replicas[0].count, 1);
}

/// Repeated model words should aggregate counts across multiple replica forms.
#[test]
fn modrep_accumulates_replicas() {
    let cmd = ModrepCommand;
    let mut state = ModrepState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    // Same model word "dog" with two different replicas
    let utt1 = make_mod_pho_utterance(&["dog"], &["dɔɡ"], &["dɔ"]);
    let utt2 = make_mod_pho_utterance(&["dog"], &["dɔɡ"], &["dɑɡ"]);
    let utt3 = make_mod_pho_utterance(&["dog"], &["dɔɡ"], &["dɔ"]);

    cmd.process_utterance(&utt1, &file_ctx, &mut state);
    cmd.process_utterance(&utt2, &file_ctx, &mut state);
    cmd.process_utterance(&utt3, &file_ctx, &mut state);

    let result = cmd.finalize(state);
    let speaker = &result.speakers[0];
    assert_eq!(speaker.entries.len(), 1);

    let dog = &speaker.entries[0];
    assert_eq!(dog.model, "dɔɡ");
    assert_eq!(dog.total, 3);
    assert_eq!(dog.replicas.len(), 2);

    let replica_counts: Vec<(&str, u64)> = dog
        .replicas
        .iter()
        .map(|r| (r.word.as_str(), r.count))
        .collect();
    assert!(replica_counts.contains(&("dɔ", 2)));
    assert!(replica_counts.contains(&("dɑɡ", 1)));
}

/// Utterances missing either `%mod` or `%pho` should be ignored.
#[test]
fn modrep_skips_without_both_tiers() {
    let cmd = ModrepCommand;
    let mut state = ModrepState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    // Utterance with only main tier, no %mod or %pho
    let content: Vec<UtteranceContent> =
        vec![UtteranceContent::Word(Box::new(Word::simple("hello")))];
    let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
    let utt = Utterance::new(main);

    cmd.process_utterance(&utt, &file_ctx, &mut state);
    let result = cmd.finalize(state);

    assert!(result.speakers.is_empty());
}

/// Pairing should stop at the shorter tier length (`zip` truncation).
#[test]
fn modrep_truncates_to_shorter_tier() {
    let cmd = ModrepCommand;
    let mut state = ModrepState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = file_ctx(&chat_file);

    // %mod has 3 words, %pho has 2, should pair only 2
    let utt = make_mod_pho_utterance(&["A", "B", "C"], &["d", "e", "f"], &["a", "b"]);

    cmd.process_utterance(&utt, &file_ctx, &mut state);
    let result = cmd.finalize(state);

    let speaker = &result.speakers[0];
    assert_eq!(speaker.entries.len(), 2); // Only d→a and e→b, not f
}

/// Text rendering should include speaker header, model totals, and replica rows.
#[test]
fn modrep_render_text() {
    let result = ModrepResult {
        speakers: vec![ModrepSpeakerResult {
            speaker: "CHI".to_string(),
            entries: vec![ModelEntry {
                model: "dɔɡ".to_string(),
                total: 3,
                replicas: vec![
                    ReplicaEntry {
                        word: "dɔ".to_string(),
                        count: 2,
                    },
                    ReplicaEntry {
                        word: "dɑɡ".to_string(),
                        count: 1,
                    },
                ],
            }],
        }],
    };

    let text = result.render_text();
    assert!(text.contains("Speaker *CHI:"));
    assert!(text.contains("3 dɔɡ"));
    assert!(text.contains("2 dɔ"));
    assert!(text.contains("1 dɑɡ"));
}
