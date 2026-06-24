//! DSS, Developmental Sentence Scoring.
//!
//! Assigns point values to utterances based on grammatical complexity,
//! using a configurable rule file that defines pattern-matching rules
//! for morphosyntactic categories. DSS is a clinical tool developed by
//! Laura Lee and Susan Canter for evaluating children's grammatical
//! development by scoring complete sentences on eight grammatical categories
//! (e.g., pronouns, verbs, negation, conjunctions, wh-questions).
//!
//! Scoring requires a `%mor` dependent tier on each utterance. Utterances
//! without `%mor` are silently skipped.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#DSS_Command)
//! for the original DSS command specification and the full rule set.
//!
//! # Differences from CLAN
//!
//! - The built-in default rules are a simplified subset of the canonical
//!   DSS rule set (10 categories). For full clinical scoring, supply a
//!   complete `.scr` rules file via `rules_path`.
//! - Sentence-point assignment uses a heuristic (presence of subject +
//!   verb POS tags) rather than full syntactic analysis.
//! - By default, up to 50 utterances per speaker are scored (configurable
//!   via `max_utterances`).

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::PathBuf;

use talkbank_model::{Mor, Utterance};

use crate::framework::mor;
use crate::framework::{
    AnalysisCommand, FileContext, ScorePoints, TransformError, UtteranceLimit, spoken_main_text,
};

pub use output::{DssResult, SpeakerDss, UtteranceScore};

/// Configuration for the DSS command.
#[derive(Debug, Clone)]
pub struct DssConfig {
    /// Path to DSS rules file (.scr).
    pub rules_path: Option<PathBuf>,
    /// Maximum number of unique utterances to score (default: 50).
    pub max_utterances: UtteranceLimit,
}

impl Default for DssConfig {
    fn default() -> Self {
        Self {
            rules_path: None,
            max_utterances: UtteranceLimit::new(50),
        }
    }
}

/// A DSS rule: pattern on %mor tier → point value.
#[derive(Debug, Clone)]
pub struct DssRule {
    /// Rule category name (e.g., "indefinite_pronoun").
    pub category: String,
    /// POS/morpheme patterns to match (simplified: list of POS tags).
    pub patterns: Vec<String>,
    /// Point value awarded.
    pub points: u32,
}

/// A loaded DSS rule set.
///
/// Contains all scoring rules for DSS analysis. If no custom rules file
/// is provided, the default English rules are used.
#[derive(Debug, Clone)]
pub struct DssRuleSet {
    /// All rules, typically grouped by grammatical category.
    pub rules: Vec<DssRule>,
}

impl Default for DssRuleSet {
    fn default() -> Self {
        Self {
            rules: default_english_rules(),
        }
    }
}

/// Default English DSS rules (simplified version of the canonical rule set).
///
/// The full DSS has ~20 categories with hundreds of patterns across 8 developmental
/// levels. This provides the core categories to demonstrate the scoring framework.
fn default_english_rules() -> Vec<DssRule> {
    vec![
        DssRule {
            category: "indefinite_pronouns".to_owned(),
            patterns: vec!["pro:indef".to_owned()],
            points: 1,
        },
        DssRule {
            category: "personal_pronouns".to_owned(),
            patterns: vec!["pro:sub".to_owned(), "pro:obj".to_owned()],
            points: 1,
        },
        DssRule {
            category: "main_verbs".to_owned(),
            patterns: vec!["v".to_owned()],
            points: 1,
        },
        DssRule {
            category: "copula".to_owned(),
            patterns: vec!["cop".to_owned()],
            points: 2,
        },
        DssRule {
            category: "auxiliaries".to_owned(),
            patterns: vec!["aux".to_owned()],
            points: 2,
        },
        DssRule {
            category: "past_tense".to_owned(),
            patterns: vec!["v-PAST".to_owned()],
            points: 2,
        },
        DssRule {
            category: "negation".to_owned(),
            patterns: vec!["neg".to_owned()],
            points: 1,
        },
        DssRule {
            category: "conjunctions".to_owned(),
            patterns: vec!["conj:coo".to_owned(), "conj:sub".to_owned()],
            points: 3,
        },
        DssRule {
            category: "wh_questions".to_owned(),
            patterns: vec!["pro:wh".to_owned(), "adv:wh".to_owned()],
            points: 2,
        },
        DssRule {
            category: "articles".to_owned(),
            patterns: vec!["det:art".to_owned()],
            points: 1,
        },
    ]
}

/// Load DSS rules from a `.scr` file.
///
/// The file format has one rule per section. Each section starts with a
/// header line `CATEGORY_NAME <points>` followed by one or more POS pattern
/// lines. Blank lines and lines starting with `#` are ignored.
pub fn load_dss_rules(path: &std::path::Path) -> Result<DssRuleSet, TransformError> {
    let content = std::fs::read_to_string(path).map_err(TransformError::Io)?;
    let mut rules = Vec::new();
    let mut current_category = String::new();
    let mut current_points = 0u32;
    let mut current_patterns = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((cat, pts_str)) = line.rsplit_once(' ')
            && let Ok(pts) = pts_str.parse::<u32>()
        {
            if !current_category.is_empty() && !current_patterns.is_empty() {
                rules.push(DssRule {
                    category: current_category.clone(),
                    patterns: current_patterns.clone(),
                    points: current_points,
                });
                current_patterns.clear();
            }
            current_category = cat.to_owned();
            current_points = pts;
            continue;
        }

        if !current_category.is_empty() {
            current_patterns.push(line.to_owned());
        }
    }

    if !current_category.is_empty() && !current_patterns.is_empty() {
        rules.push(DssRule {
            category: current_category,
            patterns: current_patterns,
            points: current_points,
        });
    }

    Ok(DssRuleSet { rules })
}

/// Accumulated state for DSS.
#[derive(Debug, Default)]
pub struct DssState {
    /// Per-speaker: list of (mor_items, main_text) pairs for scoring.
    utterances: BTreeMap<String, Vec<(Vec<Mor>, String)>>,
}

/// DSS command implementation.
///
/// Collects utterances with `%mor` tiers during processing, then scores
/// them against the loaded rule set during finalization. Up to
/// `config.max_utterances` utterances are scored per speaker.
pub struct DssCommand {
    /// Command configuration.
    config: DssConfig,
    /// Loaded DSS scoring rules.
    rules: DssRuleSet,
}

impl DssCommand {
    /// Create a new DSS command, optionally loading rules from a file.
    pub fn new(config: DssConfig) -> Result<Self, TransformError> {
        let rules = if let Some(ref path) = config.rules_path {
            load_dss_rules(path)?
        } else {
            DssRuleSet::default()
        };
        Ok(Self { config, rules })
    }
}

/// Score a single utterance's typed `%mor` items against the DSS rules.
pub fn score_utterance(
    items: &[Mor],
    rules: &DssRuleSet,
) -> (BTreeMap<String, ScorePoints>, ScorePoints) {
    let mut category_points: BTreeMap<String, ScorePoints> = BTreeMap::new();
    let mut total: ScorePoints = 0;

    for rule in &rules.rules {
        let matched = items.iter().any(|item| {
            rule.patterns
                .iter()
                .any(|pat| mor::mor_pattern_matches(item, pat))
        });

        if matched {
            let pts = rule.points;
            *category_points.entry(rule.category.clone()).or_insert(0) += pts;
            total += pts;
        }
    }

    (category_points, total)
}

/// Check if an utterance appears to be a complete sentence.
pub fn is_complete_sentence(items: &[Mor]) -> bool {
    let has_subject = mor::any_item_has_pos(items, "pro:sub")
        || mor::any_item_has_pos(items, "pro:per")
        || mor::any_item_has_pos(items, "pron")
        || mor::any_item_has_pos(items, "propn")
        || mor::any_item_has_pos(items, "n");
    let has_verb = mor::any_item_has_pos(items, "v")
        || mor::any_item_has_pos(items, "cop")
        || mor::any_item_has_pos(items, "aux");
    has_subject && has_verb
}

impl AnalysisCommand for DssCommand {
    type Config = DssConfig;
    type State = DssState;
    type Output = DssResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let speaker = utterance.main.speaker.to_string();
        let main_text = spoken_main_text(&utterance.main);

        if let Some(mor_tier) = mor::extract_mor_tier(utterance) {
            let items: Vec<Mor> = mor_tier.items().to_vec();
            state
                .utterances
                .entry(speaker)
                .or_default()
                .push((items, main_text));
        }
    }

    fn finalize(&self, state: Self::State) -> DssResult {
        let mut speakers = Vec::new();

        for (speaker, utts) in state.utterances {
            let max = self.config.max_utterances.get().min(utts.len());
            let mut scores = Vec::new();
            let mut grand_total = 0u32;

            for (i, (mor_items, main_text)) in utts.iter().take(max).enumerate() {
                let (category_points, total) = score_utterance(mor_items, &self.rules);
                let sentence_point = is_complete_sentence(mor_items);
                let utt_total = total + u32::from(sentence_point);
                grand_total += utt_total;

                const MAX_DISPLAY_LEN: usize = 60;
                let display_text = if main_text.len() > MAX_DISPLAY_LEN {
                    format!("{}...", &main_text[..MAX_DISPLAY_LEN - 3])
                } else {
                    main_text.clone()
                };

                scores.push(UtteranceScore {
                    index: i + 1,
                    text: display_text,
                    category_points,
                    total: utt_total,
                    sentence_point,
                });
            }

            let dss_score = if max > 0 {
                grand_total as f64 / max as f64
            } else {
                0.0
            };

            speakers.push(SpeakerDss {
                speaker,
                utterances_scored: max as u64,
                scores,
                grand_total,
                dss_score,
            });
        }

        DssResult { speakers }
    }
}
