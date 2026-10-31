//! CHIP words: the comparison units built from an utterance (`chip_utl.cpp`
//! `parse_utter` / `new_word`).

use talkbank_model::alignment::helpers::visit_mor_positions;
use talkbank_model::{Mor, MorFeature, MorWord, PosCategory, Utterance};

use super::config::{ChipCaseMode, ChipConfig, Substitution, WordSource};
use crate::framework::countable_words;

/// A word stem normalized once for CHIP's configured comparison mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChipStemKey(String);

impl ChipStemKey {
    pub(super) fn from_stem(stem: String, case_mode: ChipCaseMode) -> Self {
        Self(case_mode.comparison_key(&stem))
    }

    #[cfg(test)]
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

/// One word with the morphology CHIP compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChipWord {
    /// Surface text, used in `$ADD`/`$DEL`/`$SUB` codes.
    pub text: String,
    /// Original stem, retained for CLAN-compatible output codes.
    pub stem: Option<String>,
    /// Case-normalized key; words without one never match.
    pub stem_key: Option<ChipStemKey>,
    /// Prefix morphology, when represented by the source model.
    pub prefix: Option<Vec<Vec<MorFeature>>>,
    /// Suffix morphology, retaining typed feature and clitic boundaries.
    pub suffix: Option<Vec<Vec<MorFeature>>>,
    /// Part of speech (only available from `%mor`).
    pub pos: Option<PosCategory>,
}

/// An utterance whose words cannot be derived.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "%mor: tier does not have one-to-one correspondence to its speaker tier \
     ({main} alignable main-tier positions, {mor} %mor items)"
)]
pub(super) struct MisalignedMor {
    main: usize,
    mor: usize,
}

struct AlignedMorPair<'a> {
    surface: String,
    item: &'a Mor,
}

/// Words of an utterance for the given source.
///
/// With `%mor` as the source an utterance lacking a `%mor` tier has no words,
/// as in CLAN where the tier is simply never read.
pub(super) fn utterance_words(
    utterance: &Utterance,
    config: &ChipConfig,
) -> Result<Vec<ChipWord>, MisalignedMor> {
    match config.source {
        WordSource::MainTier => Ok(main_tier_words(utterance, config.case_mode)),
        WordSource::Mor(subst) => {
            let Some(tier) = utterance.mor_tier() else {
                return Ok(Vec::new());
            };
            match subst {
                Substitution::Off => aligned_mor_words(utterance, tier.items(), config),
                Substitution::On if config.has_exclusion_patterns() => {
                    split_mor_words(utterance, tier.items(), config)
                }
                Substitution::On => Ok(split_mor_words_without_exclusions(
                    tier.items(),
                    config.case_mode,
                )),
            }
        }
    }
}

fn main_tier_positions(utterance: &Utterance) -> Vec<String> {
    let mut positions = Vec::new();
    visit_mor_positions(&utterance.main.content.content, &mut |position| {
        if let Some(word) = position.word() {
            positions.push(word.cleaned_text().to_owned());
        }
    });
    positions
}

fn aligned_mor_pairs<'a>(
    utterance: &Utterance,
    items: &'a [Mor],
) -> Result<Vec<AlignedMorPair<'a>>, MisalignedMor> {
    let positions = main_tier_positions(utterance);
    if positions.len() != items.len() {
        return Err(MisalignedMor {
            main: positions.len(),
            mor: items.len(),
        });
    }
    Ok(positions
        .into_iter()
        .zip(items)
        .map(|(surface, item)| AlignedMorPair { surface, item })
        .collect())
}

fn aligned_mor_words(
    utterance: &Utterance,
    items: &[Mor],
    config: &ChipConfig,
) -> Result<Vec<ChipWord>, MisalignedMor> {
    let mut words = Vec::new();
    for pair in aligned_mor_pairs(utterance, items)? {
        let item = pair.item;
        if item.main.is_punctuation_marker() {
            continue;
        }
        let stem = joined_mor_stem(item);
        if !config.excludes_mor_pair(Some(&pair.surface), &stem) {
            words.push(joined_mor_word(pair.surface, item, config.case_mode));
        }
    }
    Ok(words)
}

/// `pron|it~aux|be-Fin` becomes stem `it~be`, pos `pron~aux`, suffix
/// `~-Fin` (CLAN's `~` joins the clitic pieces of one word).
fn joined_mor_word(text: String, item: &Mor, case_mode: ChipCaseMode) -> ChipWord {
    let stem = joined_mor_stem(item);
    let parts: Vec<&MorWord> = std::iter::once(&item.main)
        .chain(item.post_clitics.iter())
        .collect();
    let suffix = mor_features(&parts);
    ChipWord {
        text,
        stem: Some(stem.clone()),
        stem_key: Some(ChipStemKey::from_stem(stem, case_mode)),
        prefix: None,
        suffix,
        pos: Some(item.main.pos.clone()),
    }
}

fn joined_mor_stem(item: &Mor) -> String {
    std::iter::once(&item.main)
        .chain(item.post_clitics.iter())
        .map(|word| &*word.lemma)
        .collect::<Vec<_>>()
        .join("~")
}

/// With `+g` every `%mor` main word and clitic is its own word.
fn split_mor_words(
    utterance: &Utterance,
    items: &[Mor],
    config: &ChipConfig,
) -> Result<Vec<ChipWord>, MisalignedMor> {
    let mut words = Vec::new();
    for pair in aligned_mor_pairs(utterance, items)? {
        let item = pair.item;
        if item.main.is_punctuation_marker() {
            continue;
        }
        let stem = joined_mor_stem(item);
        if !config.excludes_mor_pair(Some(&pair.surface), &stem) {
            words.extend(split_mor_item(item, config.case_mode));
        }
    }
    Ok(words)
}

fn split_mor_words_without_exclusions(items: &[Mor], case_mode: ChipCaseMode) -> Vec<ChipWord> {
    items
        .iter()
        .filter(|item| !item.main.is_punctuation_marker())
        .flat_map(|item| split_mor_item(item, case_mode))
        .collect()
}

fn split_mor_item(item: &Mor, case_mode: ChipCaseMode) -> Vec<ChipWord> {
    std::iter::once(&item.main)
        .chain(item.post_clitics.iter())
        .map(|word| {
            let stem = word.lemma.to_string();
            ChipWord {
                text: stem.clone(),
                stem: Some(stem.clone()),
                stem_key: Some(ChipStemKey::from_stem(stem, case_mode)),
                prefix: None,
                suffix: (!word.features.is_empty())
                    .then(|| vec![word.features.iter().cloned().collect()]),
                pos: Some(word.pos.clone()),
            }
        })
        .collect()
}

fn mor_features(parts: &[&MorWord]) -> Option<Vec<Vec<MorFeature>>> {
    let features: Vec<Vec<MorFeature>> = parts
        .iter()
        .map(|word| word.features.iter().cloned().collect())
        .collect();
    features
        .iter()
        .any(|part| !part.is_empty())
        .then_some(features)
}

fn main_tier_words(utterance: &Utterance, case_mode: ChipCaseMode) -> Vec<ChipWord> {
    countable_words(&utterance.main.content.content)
        .map(|word| {
            let text = word.cleaned_text().to_owned();
            ChipWord {
                stem: (!text.is_empty()).then(|| text.clone()),
                stem_key: (!text.is_empty())
                    .then(|| ChipStemKey::from_stem(text.clone(), case_mode)),
                text,
                prefix: None,
                suffix: None,
                pos: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::WordPattern;
    use talkbank_model::{DependentTier, Location, MorTier, UtteranceContent};

    fn mor_utterance(surface: &str, mor: Mor) -> Utterance {
        Utterance::new(crate::test_support::main_tier(
            "CHI",
            vec![UtteranceContent::Word(Box::new(crate::test_support::word(
                surface,
            )))],
        ))
        .add_dependent_tier(DependentTier::Mor(MorTier::new_mor(
            Location::Unlocated,
            vec![mor],
            crate::test_support::period(),
        )))
    }

    #[test]
    fn main_tier_morphology_looking_text_remains_opaque() {
        let word = crate::test_support::word("un#do-ing");
        let utterance = Utterance::new(crate::test_support::main_tier(
            "CHI",
            vec![UtteranceContent::Word(Box::new(word))],
        ));
        let words = main_tier_words(&utterance, ChipCaseMode::Preserve);
        let [word] = words.as_slice() else {
            panic!("one countable word should be returned");
        };
        assert_eq!(word.text, "un#do-ing");
        assert_eq!(word.stem.as_deref(), Some("un#do-ing"));
        assert_eq!(
            word.stem_key.as_ref().map(ChipStemKey::as_str),
            Some("un#do-ing")
        );
        assert_eq!(word.prefix, None);
        assert_eq!(word.suffix, None);
    }

    #[test]
    fn fold_mode_normalizes_only_the_comparison_stem() {
        let utterance = Utterance::new(crate::test_support::main_tier(
            "CHI",
            vec![UtteranceContent::Word(Box::new(crate::test_support::word(
                "WANT",
            )))],
        ));

        let config = ChipConfig::default()
            .with_main_tier()
            .expect("main tier is compatible with default substitution")
            .with_case_mode(ChipCaseMode::Fold);
        let [word] = utterance_words(&utterance, &config)
            .expect("main-tier words are always available")
            .try_into()
            .expect("one source word");

        assert_eq!(word.text, "WANT");
        assert_eq!(word.stem.as_deref(), Some("WANT"));
        assert_eq!(
            word.stem_key.as_ref().map(ChipStemKey::as_str),
            Some("want")
        );
    }

    #[test]
    fn misaligned_mor_is_rejected_with_cardinalities() {
        let utterance = Utterance::new(crate::test_support::main_tier(
            "CHI",
            vec![UtteranceContent::Word(Box::new(crate::test_support::word(
                "word",
            )))],
        ))
        .add_dependent_tier(DependentTier::Mor(MorTier::new_mor(
            Location::Unlocated,
            vec![],
            crate::test_support::period(),
        )));

        assert_eq!(
            utterance_words(&utterance, &ChipConfig::default()),
            Err(MisalignedMor { main: 1, mor: 0 })
        );
    }

    #[test]
    fn exclusions_match_paired_surface_or_mor_with_chip_case_mode() {
        let utterance = mor_utterance("WENT", Mor::new(MorWord::new("v", "go")));

        let preserve_surface = ChipConfig::default()
            .with_exclusion_patterns(vec![WordPattern::from("went")])
            .expect("single-word exclusion pattern is supported");
        assert_eq!(
            utterance_words(&utterance, &preserve_surface)
                .expect("aligned tiers")
                .len(),
            1
        );

        let folded_surface = preserve_surface.clone().with_case_mode(ChipCaseMode::Fold);
        assert!(
            utterance_words(&utterance, &folded_surface)
                .expect("aligned tiers")
                .is_empty()
        );

        let mor_wildcard = ChipConfig::default()
            .with_exclusion_patterns(vec![WordPattern::from("g*")])
            .expect("single-word wildcard is supported");
        assert!(
            utterance_words(&utterance, &mor_wildcard)
                .expect("aligned tiers")
                .is_empty()
        );

        let main_tier = mor_wildcard
            .with_main_tier()
            .expect("main-tier mode is compatible with default substitution");
        assert_eq!(
            utterance_words(&utterance, &main_tier)
                .expect("main-tier words are available")
                .len(),
            1
        );
    }

    #[test]
    fn substitution_exclusion_removes_all_clitics_for_the_paired_word() {
        let utterance = mor_utterance(
            "WENT",
            Mor::new(MorWord::new("v", "go")).with_post_clitic(MorWord::new("aux", "have")),
        );
        let config = ChipConfig::default()
            .with_substitution()
            .expect("default morphology source supports substitution")
            .with_exclusion_patterns(vec![WordPattern::from("WENT")])
            .expect("single-word exclusion pattern is supported");

        assert!(
            utterance_words(&utterance, &config)
                .expect("aligned morphology word")
                .is_empty()
        );
    }

    #[test]
    fn substitution_exclusion_rejects_misaligned_mor_instead_of_skipping_a_pair() {
        let utterance = Utterance::new(crate::test_support::main_tier(
            "CHI",
            vec![UtteranceContent::Word(Box::new(crate::test_support::word(
                "one",
            )))],
        ))
        .add_dependent_tier(DependentTier::Mor(MorTier::new_mor(
            Location::Unlocated,
            vec![],
            crate::test_support::period(),
        )));
        let config = ChipConfig::default()
            .with_substitution()
            .expect("default morphology source supports substitution")
            .with_exclusion_patterns(vec![WordPattern::from("one")])
            .expect("single-word exclusion pattern is supported");

        assert_eq!(
            utterance_words(&utterance, &config),
            Err(MisalignedMor { main: 1, mor: 0 })
        );
    }
}
