//! COMBO, Boolean keyword search across utterances.
//!
//! Reimplements CLAN's COMBO command, which searches for utterances matching
//! boolean combinations of keywords. Supports AND (`+`) and OR (`,`) logic
//! with case-insensitive substring matching. This is the primary search tool
//! for finding utterances containing specific words or word combinations.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409095)
//! for the original COMBO command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                                | Rust equivalent                                          |
//! |---------------------------------------------|----------------------------------------------------------|
//! | `combo +s"want^cookie" file.cha`            | `chatter analyze combo file.cha -s "want+cookie"`        |
//! | `combo +s"want\|milk" file.cha`             | `chatter analyze combo file.cha -s "want,milk"`          |
//! | `combo +s"want^cookie" +t*CHI file.cha`     | `chatter analyze combo file.cha -s "want+cookie" -S CHI` |
//!
//! # Search Syntax
//!
//! - `+` between terms means AND (all terms must be present in the utterance)
//! - `,` between terms means OR (at least one term must be present)
//! - Terms are case-insensitive substring matches against countable words
//! - Multiple `-s` flags are combined with OR (any expression matching counts)
//! - AND takes precedence if both `+` and `,` appear in one expression
//!
//! # Differences from CLAN
//!
//! - CLAN uses `^` for AND and `\|` for OR; this implementation uses `+` and `,`
//!   respectively for shell-friendliness.
//!
//! # Output
//!
//! Each matching utterance with:
//! - Source filename
//! - Speaker code
//! - Full utterance text (CHAT format)
//! - Summary counts of matching vs. total utterances

mod output;
// Child test modules can intentionally inspect private ComboState fields.
#[cfg(test)]
mod tests;

use talkbank_model::{Utterance, WriteChat};

use crate::framework::word_filter::{countable_words, word_pattern_matches};
use crate::framework::{AnalysisCommand, FileContext, NormalizedWord};

pub use output::{ComboMatch, ComboResult, MatchedExpr};

/// A single search expression (terms joined by AND or OR).
///
/// # Examples
///
/// ```
/// use talkbank_clan::commands::combo::SearchExpr;
///
/// // AND: all terms must appear
/// let expr = SearchExpr::parse("want+cookie");
/// assert!(matches!(expr, SearchExpr::And(_)));
///
/// // OR: at least one term must appear
/// let expr = SearchExpr::parse("cookie,milk");
/// assert!(matches!(expr, SearchExpr::Or(_)));
///
/// // Bare term: treated as single-element AND
/// let expr = SearchExpr::parse("hello");
/// assert!(matches!(expr, SearchExpr::And(_)));
/// ```
#[derive(Debug, Clone)]
pub enum SearchExpr {
    /// All terms must be present in the utterance.
    And(Vec<String>),
    /// At least one term must be present in the utterance.
    Or(Vec<String>),
}

impl SearchExpr {
    /// Parse a search string into an expression.
    ///
    /// - `+` splits into AND terms
    /// - `,` splits into OR terms
    /// - If neither is present, treated as a single AND term
    ///
    /// AND takes precedence: if both `+` and `,` appear, the string
    /// is split on `+` first (matching CLAN's behavior).
    pub fn parse(s: &str) -> Self {
        Self::parse_with_case(s, false)
    }

    /// Parse a `+s`/`-s` expression, optionally preserving original
    /// case in the terms. CLAN `+k` (`case_sensitive = true`)
    /// suppresses the default lowercasing so the search becomes
    /// exact-case. Default `case_sensitive = false` matches CLAN's
    /// default and chatter's pre-`+k` behaviour.
    pub fn parse_with_case(s: &str, case_sensitive: bool) -> Self {
        let fold = |t: &str| -> String {
            if case_sensitive {
                t.trim().to_owned()
            } else {
                t.trim().to_lowercase()
            }
        };
        if s.contains('+') {
            let terms: Vec<String> = s.split('+').map(fold).collect();
            SearchExpr::And(terms)
        } else if s.contains(',') {
            let terms: Vec<String> = s.split(',').map(fold).collect();
            SearchExpr::Or(terms)
        } else {
            SearchExpr::And(vec![fold(s)])
        }
    }

    /// Check whether the given normalized word set satisfies this expression.
    ///
    /// Matching is case-insensitive with exact word matching (wildcards `*`
    /// supported). Words are already lowercased via [`NormalizedWord`].
    fn matches(&self, words: &[NormalizedWord]) -> bool {
        match self {
            SearchExpr::And(terms) => terms.iter().all(|term| {
                words
                    .iter()
                    .any(|w| word_pattern_matches(w.as_str(), term.as_str()))
            }),
            SearchExpr::Or(terms) => terms.iter().any(|term| {
                words
                    .iter()
                    .any(|w| word_pattern_matches(w.as_str(), term.as_str()))
            }),
        }
    }

    /// Return the set of word forms in `words` that contributed to a
    /// successful match. For And, returns one word per term (the
    /// first occurrence). For Or, returns every word whose form
    /// matches any term. Lowercased forms.
    ///
    /// Used by CLAN-format rendering to wrap matched words as
    /// `(N)<word>` in the utterance echo.
    fn matched_words(&self, words: &[NormalizedWord]) -> Vec<String> {
        let mut out = Vec::new();
        match self {
            SearchExpr::And(terms) => {
                for term in terms {
                    if let Some(w) = words
                        .iter()
                        .find(|w| word_pattern_matches(w.as_str(), term.as_str()))
                    {
                        out.push(w.as_str().to_owned());
                    }
                }
            }
            SearchExpr::Or(terms) => {
                for w in words {
                    if terms
                        .iter()
                        .any(|t| word_pattern_matches(w.as_str(), t.as_str()))
                    {
                        out.push(w.as_str().to_owned());
                    }
                }
            }
        }
        out
    }
}

/// Configuration for the COMBO command.
#[derive(Debug, Clone, Default)]
pub struct ComboConfig {
    /// Include search expressions. An utterance must match at least
    /// one of these to be output (any-of semantics; multiple
    /// `--search` flags act as OR at the expression level).
    pub search: Vec<SearchExpr>,
    /// Exclude search expressions. An utterance matching *any* of
    /// these is dropped, even if it would otherwise match an
    /// include expression. Maps CLAN's `-sS` for COMBO.
    pub exclude: Vec<SearchExpr>,
    /// CLAN's `+g3`: when `true`, an utterance that matches multiple
    /// expressions contributes only its first match to the output,
    /// remaining expressions are not evaluated. Default `false`
    /// reports every matching expression per utterance (CLAN's
    /// default).
    pub first_match_only: bool,
    /// CLAN's `+g7`: when `true`, repeated word forms within a
    /// single utterance contribute at most one entry to each
    /// expression's `matched_words` list. Mainly affects OR
    /// expressions (`cookie,milk`) where the same surface form can
    /// appear multiple times in one utterance. Default `false`
    /// records every occurrence.
    pub dedupe_matches: bool,
    /// CLAN `+k`: case-sensitive matching. Default (`false`)
    /// lowercases both `+s` terms (at parse time) and the word
    /// stream (via `NormalizedWord::from_word`). When `true`, neither
    /// side is lowercased, `Want`/`want`/`WANT` count as distinct
    /// words. Must agree with `SearchExpr::parse_with_case` at the
    /// time the search expressions are built.
    pub case_sensitive: bool,
    /// CLAN `-wN`: number of utterances immediately preceding each
    /// match to include as pre-context. Default `0`.
    pub context_before: u32,
    /// CLAN `+wN`: number of utterances immediately following each
    /// match to include as post-context. Default `0`.
    pub context_after: u32,
}

/// Accumulated state for COMBO across all files.
#[derive(Debug, Default)]
pub struct ComboState {
    /// All matches found
    matches: Vec<ComboMatch>,
    /// Total utterances examined
    total_utterances: u64,
    /// Ring buffer of recent utterance CHAT texts (capacity =
    /// `config.context_before`). See KWAL for the design, same
    /// `-wN` pre-context machinery.
    recent: std::collections::VecDeque<String>,
    /// Matches still collecting post-context (`+wN`) lines. Pair
    /// is `(match_index, remaining_after_lines)`.
    awaiting_after: Vec<(usize, u32)>,
}

/// COMBO command implementation.
///
/// For each utterance, extracts all countable words and checks whether
/// any search expression is satisfied. Multiple search expressions are
/// combined with OR logic (any expression matching counts).
#[derive(Debug, Clone, Default)]
pub struct ComboCommand {
    config: ComboConfig,
}

impl ComboCommand {
    /// Create a COMBO command with the given configuration.
    pub fn new(config: ComboConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for ComboCommand {
    type Config = ComboConfig;
    type State = ComboState;
    type Output = ComboResult;

    /// Evaluate all configured boolean keyword expressions for one
    /// utterance. Context-window ordering invariant matches KWAL's
    /// (see `kwal::process_utterance`): post-context drains for
    /// earlier matches before the current match is recorded, ring
    /// updates afterward.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        if self.config.search.is_empty() {
            return;
        }

        state.total_utterances += 1;

        let case_sensitive = self.config.case_sensitive;
        let words: Vec<NormalizedWord> = countable_words(&utterance.main.content.content)
            .map(|w| NormalizedWord::from_word_cased(w, case_sensitive))
            .collect();

        // `-sS` excluded utterances still feed the windows; they
        // count as non-matches for context bookkeeping. Flag rather
        // than early-return.
        let excluded = self.config.exclude.iter().any(|expr| expr.matches(&words));

        let mut expr_hits: Vec<MatchedExpr> = Vec::new();
        if !excluded {
            let dedupe = self.config.dedupe_matches;
            let raw = self
                .config
                .search
                .iter()
                .enumerate()
                .filter_map(|(i, expr)| {
                    if !expr.matches(&words) {
                        return None;
                    }
                    let matched_words: Vec<String> = if dedupe {
                        indexmap::IndexSet::<String>::from_iter(expr.matched_words(&words))
                            .into_iter()
                            .collect()
                    } else {
                        expr.matched_words(&words)
                    };
                    Some(MatchedExpr {
                        index: i + 1,
                        matched_words,
                    })
                });
            expr_hits = if self.config.first_match_only {
                raw.take(1).collect()
            } else {
                raw.collect()
            };
        }

        // Skip the allocating CHAT serialization when there's no
        // window work AND no match to record. Default-config callers
        // (no `+wN`/`-wN`) pay nothing extra per non-matching
        // utterance.
        let needs_text = !expr_hits.is_empty()
            || !state.awaiting_after.is_empty()
            || self.config.context_before > 0;
        if !needs_text {
            return;
        }
        let utterance_text = utterance.main.to_chat_string();

        state.awaiting_after.retain_mut(|(match_idx, remaining)| {
            state.matches[*match_idx]
                .post_context
                .push(utterance_text.clone());
            *remaining -= 1;
            *remaining > 0
        });

        if !expr_hits.is_empty() {
            let line_number = file_context
                .line_map
                .map(|lm| lm.line_of(utterance.main.span.start))
                .unwrap_or(0);
            let pre_context: Vec<String> = state.recent.iter().cloned().collect();
            let match_idx = state.matches.len();
            state.matches.push(ComboMatch {
                speaker: utterance.main.speaker.as_str().to_owned(),
                utterance_text: utterance_text.clone(),
                filename: file_context.filename.to_owned(),
                line_number,
                expr_hits,
                pre_context,
                post_context: Vec::new(),
            });
            if self.config.context_after > 0 {
                state
                    .awaiting_after
                    .push((match_idx, self.config.context_after));
            }
        }

        let cap = self.config.context_before as usize;
        if cap > 0 {
            if state.recent.len() == cap {
                state.recent.pop_front();
            }
            state.recent.push_back(utterance_text);
        }
    }

    /// Move accumulated matches and counters into the typed result.
    fn finalize(&self, state: Self::State) -> ComboResult {
        ComboResult {
            matches: state.matches,
            total_utterances: state.total_utterances,
        }
    }
}
