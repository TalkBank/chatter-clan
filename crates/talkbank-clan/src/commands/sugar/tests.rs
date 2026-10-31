use super::*;

#[test]
fn count_morphemes_simple() {
    use talkbank_model::dependent_tier::mor::{Mor, MorWord};

    assert_eq!(
        mor_item_morpheme_count(&Mor::new(MorWord::new("n", "dog"))),
        1
    );
    assert_eq!(
        mor_item_morpheme_count(&Mor::new(MorWord::new("n", "dog").with_feature("PL"))),
        2
    );
    assert_eq!(
        mor_item_morpheme_count(&Mor::new(MorWord::new("v", "walk").with_feature("PAST"))),
        2
    );
    assert_eq!(
        mor_item_morpheme_count(
            &Mor::new(MorWord::new("pro", "it"))
                .with_post_clitic(MorWord::new("aux", "be").with_feature("3S"))
        ),
        3
    );
}

#[test]
fn is_verb_detects_verbs() {
    assert!(is_verb_pos("v"));
    assert!(is_verb_pos("cop"));
    assert!(is_verb_pos("aux"));
    assert!(is_verb_pos("mod"));
    assert!(is_verb_pos("mod:aux"));
    assert!(!is_verb_pos("n"));
    assert!(!is_verb_pos("adj"));
}

#[test]
fn count_clauses_basic() {
    let gra = talkbank_model::GraTier::new_gra(
        talkbank_model::Location::Unlocated,
        vec![
            crate::test_support::rel(1, 2, "SUBJ"),
            crate::test_support::rel(2, 0, "ROOT"),
            crate::test_support::rel(3, 2, "OBJ"),
            crate::test_support::rel(4, 2, "COMP"),
            crate::test_support::rel(5, 4, "SUBJ"),
        ],
    );
    assert_eq!(count_clauses_from_gra(&gra), 1);
}

#[test]
fn sugar_empty() {
    let cmd = SugarCommand::new(SugarConfig::default());
    let state = SugarState::default();
    let result = cmd.finalize(state);
    assert!(result.speakers.is_empty());
}

#[test]
fn is_verb_detects_ud_verb() {
    assert!(is_verb_pos("verb"));
    assert!(is_verb_pos("v"));
    assert!(is_verb_pos("aux"));
    assert!(!is_verb_pos("noun"));
    assert!(!is_verb_pos("propn"));
    assert!(!is_verb_pos("pron"));
}
