use super::*;

#[test]
fn eval_empty() {
    let cmd = EvalCommand::new(EvalConfig::default());
    let state = EvalState::default();
    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
}
