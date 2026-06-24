use super::*;

#[test]
fn kideval_empty() {
    let cmd = KidevalCommand::new(KidevalConfig::default()).unwrap();
    let state = KidevalState::default();
    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
    assert!(result.comparisons.is_none());
}

#[test]
fn vocd_score_basic() {
    // Not enough tokens
    let short: Vec<String> = (0..10).map(|i| format!("word{i}")).collect();
    assert_eq!(compute_vocd_score(&short), 0.0);

    // Enough tokens with some repetition
    let mut tokens: Vec<String> = Vec::new();
    for i in 0..100 {
        tokens.push(format!("word{}", i % 30)); // 30 unique words in 100 tokens
    }
    let score = compute_vocd_score(&tokens);
    assert!(score > 0.0);
}
