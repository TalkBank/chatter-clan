//! SCRIPT, Compare utterances to a template script.
//!
//! Compares subject CHAT data against an ideal template file to compute
//! accuracy metrics: words produced vs. expected, correct matches,
//! omissions (in template but not produced), and additions (produced but
//! not in template). Useful for evaluating scripted language samples
//! such as picture descriptions or story retells.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409234)
//! for the original SCRIPT command specification.
//!
//! # Differences from CLAN
//!
//! - Template file is parsed into a typed AST (not raw text comparison).
//! - Word matching uses `NormalizedWord` for case-insensitive comparison.
//! - Omissions and additions are computed from frequency maps rather than
//!   positional alignment, which may produce different results when word
//!   order matters.
//! - Output supports text, JSON, and CSV formats.
//!
//! # Algorithm
//!
//! 1. Parse the template CHAT file and build a word frequency map (ideal
//!    counts).
//! 2. For each subject utterance, accumulate word frequency counts.
//! 3. At finalization, compute per-word matches (minimum of ideal and
//!    actual), omissions, and additions.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::PathBuf;

use talkbank_model::ParseValidateOptions;
use talkbank_model::Utterance;

use crate::framework::{AnalysisCommand, FileContext, TransformError, countable_words};

pub use output::{FileMetrics, ScriptResult};

/// Configuration for the SCRIPT command.
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Path to template/script file.
    pub template_path: PathBuf,
}

/// Load ideal word counts from a template CHAT file.
///
/// Parses the template, extracts all words from main tiers (lowercased),
/// and returns a frequency map. Punctuation terminators are excluded.
fn load_template(path: &std::path::Path) -> Result<BTreeMap<String, u64>, TransformError> {
    let content_str = std::fs::read_to_string(path).map_err(TransformError::Io)?;
    let chat =
        talkbank_transform::parse_and_validate(&content_str, ParseValidateOptions::default())
            .map_err(|e| TransformError::Parse(format!("Template: {e}")))?;

    let mut word_counts: BTreeMap<String, u64> = BTreeMap::new();
    for utt in chat.utterances() {
        for word in countable_words(&utt.main.content.content) {
            *word_counts
                .entry(word.cleaned_text().to_lowercase())
                .or_insert(0u64) += 1;
        }
    }

    Ok(word_counts)
}

/// Accumulated state for SCRIPT.
#[derive(Debug, Default)]
pub struct ScriptState {
    /// Per-word counts in subject data.
    word_counts: BTreeMap<String, u64>,
    /// Total words produced.
    total_produced: u64,
}

/// SCRIPT command implementation.
///
/// Holds the parsed template word counts and compares subject utterances
/// against them at finalization.
pub struct ScriptCommand {
    _config: ScriptConfig,
    /// Ideal word counts from template.
    template: BTreeMap<String, u64>,
}

impl ScriptCommand {
    /// Create a new SCRIPT command, loading the template file.
    pub fn new(config: ScriptConfig) -> Result<Self, TransformError> {
        let template = load_template(&config.template_path)?;
        Ok(Self {
            _config: config,
            template,
        })
    }
}

impl AnalysisCommand for ScriptCommand {
    type Config = ScriptConfig;
    type State = ScriptState;
    type Output = ScriptResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        for word in countable_words(&utterance.main.content.content) {
            *state
                .word_counts
                .entry(word.cleaned_text().to_lowercase())
                .or_insert(0) += 1;
            state.total_produced += 1;
        }
    }

    fn finalize(&self, state: Self::State) -> ScriptResult {
        let mut correct = 0u64;
        let mut ideal_total = 0u64;

        for (word, ideal_count) in &self.template {
            ideal_total += ideal_count;
            let actual = state.word_counts.get(word).copied().unwrap_or(0);
            correct += actual.min(*ideal_count);
        }

        let omitted = ideal_total.saturating_sub(correct);
        let added = state.total_produced.saturating_sub(correct);
        let pct = if ideal_total > 0 {
            correct as f64 / ideal_total as f64 * 100.0
        } else {
            0.0
        };

        let file_metrics = FileMetrics {
            filename: "aggregated".to_owned(),
            words_produced: state.total_produced,
            words_ideal: ideal_total,
            words_correct: correct,
            words_omitted: omitted,
            words_added: added,
            pct_correct: pct,
        };

        ScriptResult {
            files: vec![file_metrics],
            total_produced: state.total_produced,
            total_ideal: ideal_total,
            total_correct: correct,
            total_omitted: omitted,
            total_added: added,
            overall_pct: pct,
        }
    }
}
