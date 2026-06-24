use super::*;

#[test]
fn trnfix_empty() {
    let cmd = TrnfixCommand::new(TrnfixConfig::default());
    let state = TrnfixState::default();
    let result = cmd.finalize(state);
    assert_eq!(result.total_items, 0);
    assert_eq!(result.total_errors, 0);
    assert_eq!(result.accuracy, 100.0);
}
