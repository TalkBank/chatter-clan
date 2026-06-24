use std::collections::BTreeMap;
use std::path::PathBuf;

use super::*;

#[test]
fn script_empty() {
    // Without a template file, test that finalize works with empty state
    let template = BTreeMap::new();
    let cmd = ScriptCommand {
        _config: ScriptConfig {
            template_path: PathBuf::from("nonexistent"),
        },
        template,
    };
    let state = ScriptState::default();
    let result = cmd.finalize(state);
    assert_eq!(result.total_produced, 0);
    assert_eq!(result.total_ideal, 0);
}

#[test]
fn script_perfect_match() {
    let mut template = BTreeMap::new();
    template.insert("hello".to_owned(), 2);
    template.insert("world".to_owned(), 1);

    let cmd = ScriptCommand {
        _config: ScriptConfig {
            template_path: PathBuf::from("test"),
        },
        template,
    };

    let mut state = ScriptState::default();
    state.word_counts.insert("hello".to_owned(), 2);
    state.word_counts.insert("world".to_owned(), 1);
    state.total_produced = 3;

    let result = cmd.finalize(state);
    assert_eq!(result.total_correct, 3);
    assert_eq!(result.total_omitted, 0);
    assert_eq!(result.total_added, 0);
    assert!((result.overall_pct - 100.0).abs() < f64::EPSILON);
}
