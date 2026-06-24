//! VOCD, Vocabulary diversity (D statistic).
//!
//! Computes the D statistic for lexical diversity using bootstrap
//! sampling of type-token ratios (TTR). The D statistic provides a
//! more stable measure of vocabulary diversity than raw TTR because
//! it accounts for sample size effects.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409241)
//! for the original VOCD command specification.
//!
//! # Algorithm
//!
//! 1. Collect all countable word tokens per speaker from the main tier.
//! 2. For each of 3 independent trials:
//!    - For each sample size N in \[35..50\], draw 100 random samples of N
//!      tokens (without replacement) and compute mean TTR across samples.
//!    - Fit the empirical (N, TTR) curve to the theoretical D-curve using
//!      gradient-descent least-squares optimization.
//!    - Record the optimal D value.
//! 3. Report per-trial D values and their average.
//!
//! # Theoretical TTR Curve
//!
//! `TTR(N) = (D/N) * [sqrt(1 + 2*N/D) - 1]`
//!
//! This models the expected type-token ratio for a sample of size N given
//! a lexical diversity parameter D. Higher D means greater diversity.
//!
//! # CLAN Equivalence
//!
//! | CLAN command              | Rust equivalent                        |
//! |---------------------------|----------------------------------------|
//! | `vocd file.cha`           | `chatter analyze vocd file.cha`        |
//! | `vocd +t*CHI file.cha`    | `chatter analyze vocd file.cha -s CHI` |
//!
//! # Output
//!
//! Per-speaker D statistic with per-trial breakdown tables showing
//! (N, samples, TTR, std_dev, D) for each sample size.
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Token collection operates on parsed AST content rather than raw text.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
mod stats;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use indexmap::IndexMap;
use rand::prelude::*;
use rand::rngs::StdRng;
use talkbank_model::{SpeakerCode, Utterance, WriteChat};

use crate::framework::word_filter::{CapitalizationFilter, countable_words};
use crate::framework::{AnalysisCommand, FileContext, TypeCount, WordCount};

pub use output::{NtEntry, VocdResult, VocdSpeakerResult, VocdTrial, VocdWarning};

use self::stats::run_trial;

/// Strip fusional feature from a %mor lemma (e.g., "be&PRES" → "be").
///
/// CLAN echoes only the base lemma for insufficient-token speakers.
fn strip_fusional(lemma: &str) -> String {
    lemma.split('&').next().unwrap_or(lemma).to_owned()
}

/// Default range of sample sizes for VOCD.
const DEFAULT_SAMPLE_FROM: usize = 35;
const DEFAULT_SAMPLE_TO: usize = 50;
/// Number of random samples drawn per sample size N.
const DEFAULT_NUM_SAMPLES: usize = 100;
/// Number of independent trials to average.
const NUM_TRIALS: usize = 3;

/// Configuration for the VOCD command.
#[derive(Debug, Clone)]
pub struct VocdConfig {
    /// Lower bound of sample size range (default: 35).
    pub sample_from: usize,
    /// Upper bound of sample size range (default: 50).
    pub sample_to: usize,
    /// Number of random samples per sample size (default: 100).
    pub num_samples: usize,
    /// CLAN's `+c` / `+c0` / `+c1`: restrict the token stream fed
    /// to the D-statistic sampler to words whose surface form
    /// matches a capitalization predicate. `Any` (default) feeds
    /// every countable word.
    pub capitalization: CapitalizationFilter,
    /// VOCD token case keying. `true` preserves each token's original case so
    /// `Want`/`want`/`WANT` are three distinct types in the D computation;
    /// `false` folds to lowercase, collapsing them.
    ///
    /// CLAN VOCD is in `mmaininit`'s `nomap=TRUE` set (cutt.cpp:7845), so it
    /// PRESERVES case by default and `+k` TOGGLES it to folding
    /// (cutt.cpp:13816). Callers set this to `!(+k present)`; `Default` is
    /// `true` to match CLAN's VOCD default. This is the inverse of the
    /// fold-by-default commands; see the `+k` case-polarity investigation.
    pub case_sensitive: bool,
}

impl Default for VocdConfig {
    /// Use CLAN-style sampling defaults (N=35..50, 100 samples each). VOCD
    /// preserves case by default (`nomap=TRUE`, cutt.cpp:7845), so
    /// `case_sensitive` defaults to `true`.
    fn default() -> Self {
        Self {
            sample_from: DEFAULT_SAMPLE_FROM,
            sample_to: DEFAULT_SAMPLE_TO,
            num_samples: DEFAULT_NUM_SAMPLES,
            capitalization: CapitalizationFilter::Any,
            case_sensitive: true,
        }
    }
}

/// Per-speaker accumulated token sequence.
#[derive(Debug, Default)]
struct SpeakerTokens {
    /// All countable word tokens (lowercased) in encounter order.
    tokens: Vec<String>,
    /// Per-utterance %mor lemma strings (for CLAN echo of insufficient-token speakers).
    mor_lemma_lines: Vec<String>,
}

/// Accumulated state for VOCD across all files.
#[derive(Debug, Default)]
pub struct VocdState {
    /// Per-speaker token sequences, keyed by speaker code.
    by_speaker: IndexMap<SpeakerCode, SpeakerTokens>,
}

/// VOCD command: compute vocabulary diversity D statistic.
///
/// Collects per-speaker token sequences during utterance processing.
/// At finalization, runs bootstrap trials per speaker (or emits warnings
/// for speakers with insufficient tokens). Requires at least
/// `sample_to` tokens (default: 50) per speaker.
#[derive(Default)]
pub struct VocdCommand {
    config: VocdConfig,
}

impl VocdCommand {
    /// Create a new `VocdCommand` with the given configuration.
    pub fn new(config: VocdConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for VocdCommand {
    type Config = VocdConfig;
    type State = VocdState;
    type Output = VocdResult;

    /// Append countable lexical tokens from one utterance to the speaker sequence.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let speaker_data = state
            .by_speaker
            .entry(utterance.main.speaker.clone())
            .or_default();

        // `+c` (`capitalization`) gates entry into the token stream;
        // `+k` (`case_sensitive`) toggles the lower-case fold.
        let cap_filter = self.config.capitalization;
        let case_sensitive = self.config.case_sensitive;
        for word in countable_words(&utterance.main.content.content) {
            if !cap_filter.includes(word.cleaned_text()) {
                continue;
            }
            let text = word.to_chat_string();
            if !text.is_empty() {
                let token = if case_sensitive {
                    text
                } else {
                    text.to_lowercase()
                };
                speaker_data.tokens.push(token);
            }
        }

        // Collect %mor lemmas for CLAN echo (used when speaker has insufficient tokens).
        // Strip fusional features (&PRES, &INF, etc.), CLAN echoes base lemmas only.
        // Skip punctuation-class items (`cm`, `punct`, `end`, `beg`), CLAN's
        // mor-lemma echo omits them; see `MorWord::is_punctuation_marker`.
        if let Some(mor_tier) = utterance.mor_tier() {
            let lemmas: Vec<String> = mor_tier
                .items()
                .iter()
                .filter(|mor| !mor.main.is_punctuation_marker())
                .flat_map(|mor| {
                    let mut words = vec![strip_fusional(&mor.main.lemma)];
                    for clitic in &mor.post_clitics {
                        words.push(strip_fusional(&clitic.lemma));
                    }
                    words
                })
                .collect();
            if !lemmas.is_empty() {
                speaker_data.mor_lemma_lines.push(lemmas.join(" "));
            }
        }
    }

    /// Run VOCD trials per speaker or emit warnings when token counts are insufficient.
    fn finalize(&self, state: Self::State) -> Self::Output {
        let min_required = self.config.sample_to;
        let mut speakers = Vec::new();
        let mut warnings = Vec::new();

        for (speaker_code, speaker_tokens) in state.by_speaker {
            let token_count = speaker_tokens.tokens.len();

            if token_count < min_required {
                warnings.push(VocdWarning {
                    speaker: speaker_code.to_string(),
                    token_count: token_count as WordCount,
                    minimum_required: min_required as WordCount,
                    mor_lemma_lines: speaker_tokens.mor_lemma_lines,
                });
                continue;
            }

            let unique_types: HashSet<&str> =
                speaker_tokens.tokens.iter().map(String::as_str).collect();
            let types = unique_types.len() as TypeCount;
            let tokens = token_count as WordCount;
            let ttr = types as f64 / tokens as f64;

            let mut rng = StdRng::from_rng(&mut rand::rng());
            let mut trials = Vec::with_capacity(NUM_TRIALS);
            let mut d_optimum_values = Vec::with_capacity(NUM_TRIALS);

            for _ in 0..NUM_TRIALS {
                let trial = run_trial(
                    &speaker_tokens.tokens,
                    self.config.sample_from,
                    self.config.sample_to,
                    self.config.num_samples,
                    &mut rng,
                );
                d_optimum_values.push(trial.d_optimum);
                trials.push(trial);
            }

            let d_optimum_average =
                d_optimum_values.iter().sum::<f64>() / d_optimum_values.len() as f64;

            speakers.push(VocdSpeakerResult {
                speaker: speaker_code.to_string(),
                types,
                tokens,
                ttr,
                trials,
                d_optimum_values,
                d_optimum_average,
            });
        }

        VocdResult { speakers, warnings }
    }
}

#[cfg(test)]
fn test_file_context() -> FileContext<'static> {
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let leaked: &'static talkbank_model::ChatFile = Box::leak(Box::new(chat_file));
    FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: leaked,
        filename: "test",
        line_map: None,
    }
}

// These stay inline because they intentionally inspect private token-accumulation state.
#[cfg(test)]
mod state_tests {
    use super::*;
    use talkbank_model::Span;
    use talkbank_model::{MainTier, Terminator, Utterance, UtteranceContent, Word};

    /// `+c1` (`CapitalizationFilter::MidUpper`) keeps only words
    /// with an uppercase letter past position 0 in the VOCD token
    /// stream, proper-noun initial caps don't qualify.
    #[test]
    fn vocd_mid_upper_filter_admits_only_mid_uppercase() {
        let cmd = VocdCommand::new(VocdConfig {
            sample_from: 5,
            sample_to: 10,
            num_samples: 20,
            capitalization: CapitalizationFilter::MidUpper,
            case_sensitive: false,
        });
        let mut state = VocdState::default();
        let file_ctx = test_file_context();

        let content: Vec<UtteranceContent> = ["I", "Cookie", "McDonald", "iPhone"]
            .iter()
            .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
            .collect();
        let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
        let utt = Utterance::new(main);
        cmd.process_utterance(&utt, &file_ctx, &mut state);

        let speaker = state.by_speaker.get("CHI").expect("CHI speaker data");
        let tokens: Vec<&str> = speaker.tokens.iter().map(String::as_str).collect();
        assert_eq!(tokens, vec!["mcdonald", "iphone"]);
    }

    /// `+c` / `+c0` (`CapitalizationFilter::InitialUpper`) drops
    /// non-initial-upper words before the token stream reaches the
    /// sampler. A mixed utterance with two capitalized words yields
    /// exactly two tokens in the speaker token sequence.
    #[test]
    fn vocd_capitalized_only_filters_lowercase_words() {
        let cmd = VocdCommand::new(VocdConfig {
            sample_from: 5,
            sample_to: 10,
            num_samples: 20,
            capitalization: CapitalizationFilter::InitialUpper,
            case_sensitive: false,
        });
        let mut state = VocdState::default();
        let file_ctx = test_file_context();

        let content: Vec<UtteranceContent> = ["I", "want", "a", "Cookie", "and"]
            .iter()
            .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
            .collect();
        let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
        let utt = Utterance::new(main);
        cmd.process_utterance(&utt, &file_ctx, &mut state);

        let speaker = state.by_speaker.get("CHI").expect("CHI speaker data");
        let tokens: Vec<&str> = speaker.tokens.iter().map(String::as_str).collect();
        assert_eq!(tokens, vec!["i", "cookie"]);
    }

    /// CLAN VOCD's DEFAULT preserves token case: VOCD is in `mmaininit`'s
    /// `nomap=TRUE` set (cutt.cpp:7845), so `Want`/`want`/`WANT` are three
    /// distinct types feeding the D-statistic sampler. `+k` folds (see the
    /// companion). `VocdConfig::default()` is therefore preserve.
    #[test]
    fn vocd_default_preserves_case_in_tokens() {
        let cmd = VocdCommand::new(VocdConfig::default());
        let mut state = VocdState::default();
        let file_ctx = test_file_context();

        let content: Vec<UtteranceContent> = ["Want", "want", "WANT"]
            .iter()
            .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
            .collect();
        let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
        let utt = Utterance::new(main);
        cmd.process_utterance(&utt, &file_ctx, &mut state);

        let speaker = state.by_speaker.get("CHI").expect("CHI speaker data");
        let tokens: Vec<&str> = speaker.tokens.iter().map(String::as_str).collect();
        assert_eq!(tokens, vec!["Want", "want", "WANT"]);
    }

    /// Companion: CLAN VOCD `+k` folds token case to lowercase (it toggles
    /// `nomap` off, cutt.cpp:13816), collapsing the three case variants into
    /// one type. chatter represents the `+k`/fold state as
    /// `case_sensitive: false`.
    #[test]
    fn vocd_plus_k_folds_case_in_tokens() {
        let cmd = VocdCommand::new(VocdConfig {
            case_sensitive: false,
            ..VocdConfig::default()
        });
        let mut state = VocdState::default();
        let file_ctx = test_file_context();

        let content: Vec<UtteranceContent> = ["Want", "want", "WANT"]
            .iter()
            .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
            .collect();
        let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
        let utt = Utterance::new(main);
        cmd.process_utterance(&utt, &file_ctx, &mut state);

        let speaker = state.by_speaker.get("CHI").expect("CHI speaker data");
        let tokens: Vec<&str> = speaker.tokens.iter().map(String::as_str).collect();
        assert_eq!(tokens, vec!["want", "want", "want"]);
    }
}
