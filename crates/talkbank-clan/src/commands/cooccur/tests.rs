use super::*;
use crate::framework::{CommandOutput, FileContext};
use talkbank_model::Span;
use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

/// Build a minimal utterance with plain lexical tokens for tests.
fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    Utterance::new(main)
}

/// Build a stable `FileContext` fixture reused by command tests.
fn file_ctx(chat_file: &talkbank_model::ChatFile) -> FileContext<'_> {
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file,
        filename: "test",
        line_map: None,
    }
}

/// Adjacent tokens should produce one ordered bigram per sliding window.
/// `+d` (`no_frequency_counts`) strips the leading count
/// column from the CLAN-format output. Each row becomes
/// `display1 display2` instead of `  N  display1 display2`.
#[test]
fn cooccur_no_frequency_counts_strips_count_column() {
    let command = CooccurCommand::new(CooccurConfig {
        no_frequency_counts: true,
        ..CooccurConfig::default()
    });
    let mut state = CooccurState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let clan = result.render_clan();
    // Each non-empty row should be exactly two whitespace-
    // separated words, no leading count.
    for line in clan.lines().filter(|l| !l.is_empty()) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(
            parts.len(),
            2,
            "row should be exactly two tokens (word, word); got {parts:?} from line {line:?}"
        );
    }
    // And: no purely-digit token at the start of any row.
    assert!(
        !clan.lines().any(|l| {
            l.split_whitespace()
                .next()
                .is_some_and(|t| t.chars().all(|c| c.is_ascii_digit()))
        }),
        "+d output must not have a leading count column:\n{clan}"
    );
}

/// Default render keeps the count column. Companion test
/// pinning the contrast against identical input.
#[test]
fn cooccur_default_keeps_count_column() {
    let command = CooccurCommand::default();
    let mut state = CooccurState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let clan = result.render_clan();
    // Default has a leading count column, every non-empty row
    // starts with a digit-only token.
    for line in clan.lines().filter(|l| !l.is_empty()) {
        let first = line
            .split_whitespace()
            .next()
            .expect("expected leading token");
        assert!(
            first.chars().all(|c| c.is_ascii_digit()),
            "default render should start with count column; got {line:?}"
        );
    }
}

#[test]
fn cooccur_adjacent_pairs() {
    let command = CooccurCommand::default();
    let mut state = CooccurState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    // "I want cookie" → adjacent pairs: (i, want), (want, cookie), utterance order
    let u = make_utterance("CHI", &["I", "want", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    // Should have 2 adjacent pairs in utterance order
    assert_eq!(state.clusters.len(), 2);
    assert_eq!(state.clusters[&WordCluster::new(&["i", "want"])].count, 1);
    assert_eq!(
        state.clusters[&WordCluster::new(&["want", "cookie"])].count,
        1
    );
}

/// Pair counts should accumulate across multiple utterances.
#[test]
fn cooccur_accumulates_across_utterances() {
    let command = CooccurCommand::default();
    let mut state = CooccurState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u1 = make_utterance("CHI", &["I", "want"]);
    let u2 = make_utterance("CHI", &["I", "want", "more"]);
    command.process_utterance(&u1, &ctx, &mut state);
    command.process_utterance(&u2, &ctx, &mut state);

    // (i, want) should have count 2
    assert_eq!(state.clusters[&WordCluster::new(&["i", "want"])].count, 2);
    assert_eq!(state.total_utterances, 2);
}

/// One-token utterances should not emit any pair entries.
#[test]
fn cooccur_single_word_no_pairs() {
    let command = CooccurCommand::default();
    let mut state = CooccurState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["hello"]);
    command.process_utterance(&u, &ctx, &mut state);

    assert_eq!(state.clusters.len(), 0);
}

/// Finalizing untouched state should return an empty result set.
#[test]
fn cooccur_empty_state() {
    let command = CooccurCommand::default();
    let state = CooccurState::default();
    let result = command.finalize(state);
    assert!(result.clusters.is_empty());
}

/// Cluster keys are directional: `(a,b)` and `(b,a)` are distinct.
#[test]
fn word_cluster_preserves_utterance_order() {
    let p1 = WordCluster::new(&["want", "cookie"]);
    let p2 = WordCluster::new(&["cookie", "want"]);
    assert_ne!(p1, p2);
    assert_eq!(p1.0[0].as_str(), "want");
    assert_eq!(p1.0[1].as_str(), "cookie");
}

/// CLAN COOCCUR `+nN` / `--cluster-size N`: emit N-grams.
/// With `+n3` on a 4-word utterance, we get 2 trigram windows.
#[test]
fn cooccur_cluster_size_three_emits_trigrams() {
    let command = CooccurCommand::new(CooccurConfig {
        cluster_size: 3,
        ..CooccurConfig::default()
    });
    let mut state = CooccurState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "a", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.clusters.len(), 2);
    // Each cluster carries 3 words (the cluster_size).
    for c in &result.clusters {
        assert_eq!(c.words.len(), 3);
    }
    // Trigrams in utterance order.
    let trigrams: Vec<Vec<String>> = result.clusters.iter().map(|c| c.words.clone()).collect();
    assert!(trigrams.contains(&vec!["i".to_owned(), "want".to_owned(), "a".to_owned()]));
    assert!(trigrams.contains(&vec![
        "want".to_owned(),
        "a".to_owned(),
        "cookie".to_owned()
    ]));
}

/// `+nN` with N greater than the utterance length produces no
/// clusters from that utterance.
#[test]
fn cooccur_cluster_size_larger_than_utterance_is_skipped() {
    let command = CooccurCommand::new(CooccurConfig {
        cluster_size: 5,
        ..CooccurConfig::default()
    });
    let mut state = CooccurState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let ctx = file_ctx(&chat_file);

    let u = make_utterance("CHI", &["I", "want", "a", "cookie"]);
    command.process_utterance(&u, &ctx, &mut state);

    let result = command.finalize(state);
    assert!(result.clusters.is_empty());
}
