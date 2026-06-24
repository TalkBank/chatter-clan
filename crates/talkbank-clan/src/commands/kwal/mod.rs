//! KWAL, Keyword And Line (keyword-in-context search).
//!
//! Searches for utterances containing specified keywords and displays
//! matching lines with context. Keywords are matched as case-insensitive
//! exact words against countable words on the main tier. Wildcards (`*`)
//! are supported for partial matching (e.g., `cook*` matches `cookies`).
//!
//! # CLAN Equivalence
//!
//! | CLAN command                    | Rust equivalent                                  |
//! |---------------------------------|--------------------------------------------------|
//! | `kwal +s"want" file.cha`        | `chatter analyze kwal file.cha -k want`          |
//! | `kwal +s"want" +t*CHI file.cha` | `chatter analyze kwal file.cha -k want -s CHI`   |
//!
//! KWAL does not have a dedicated section in the CLAN manual; it is
//! described alongside other search commands.
//!
//! # Output
//!
//! Each matching utterance with:
//! - Speaker code
//! - Full utterance text
//! - File path (for multi-file searches)
//! - Match count summary per keyword
//!
//! # Differences from CLAN
//!
//! - Search operates on parsed AST word content rather than raw text lines.
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).

mod output;
#[cfg(test)]
mod tests;

use indexmap::IndexMap;
use talkbank_model::{Utterance, WriteChat};

use crate::framework::word_filter::{countable_words, word_pattern_matches};
use crate::framework::{AnalysisCommand, FileContext, NormalizedWord};

pub use output::{KwalMatch, KwalResult};

/// Configuration for the KWAL command.
#[derive(Debug, Clone, Default)]
pub struct KwalConfig {
    /// Keywords to search for (case-insensitive exact match, `*` wildcards supported)
    pub keywords: Vec<crate::framework::KeywordPattern>,
    /// CLAN `+b`: match only utterances whose tier consists of
    /// exactly one countable word, and that word matches one of
    /// the configured keywords. Default `false` reverts to "match
    /// anywhere on the tier."
    pub strict_match: bool,
    /// CLAN `+k`: keyword matching is case-sensitive. Default
    /// `false` (CLAN default) lowercases both sides before
    /// comparison. When true, neither keyword nor word is folded.
    pub case_sensitive: bool,
    /// CLAN `+d` (no N): emit matching utterances as legal CHAT
    /// (drop the `---` separator and `*** File ... Keyword: X`
    /// location annotation).
    pub legal_chat: bool,
    /// CLAN `-wN` / `--context-before`: number of utterances
    /// immediately preceding each match to include as
    /// pre-context. Default `0` ⇒ no leading context.
    pub context_before: u32,
    /// CLAN `+wN` / `--context-after`: number of utterances
    /// immediately following each match to include as
    /// post-context. Default `0` ⇒ no trailing context.
    pub context_after: u32,
}

/// Accumulated state for KWAL across all files.
#[derive(Debug, Default)]
pub struct KwalState {
    /// All matches found
    matches: Vec<KwalMatch>,
    /// Per-keyword match count
    keyword_counts: IndexMap<String, u64>,
    /// Ring buffer of recent utterance CHAT texts (capacity =
    /// `config.context_before`). Holds the most recent N
    /// non-matching utterances so a new match can snapshot them
    /// as `pre_context`. Empty when `context_before == 0`.
    recent: std::collections::VecDeque<String>,
    /// Matches still collecting post-context lines. Pair is
    /// `(match_index, remaining_after_lines)`. Each subsequent
    /// utterance appends to all entries and decrements; an entry
    /// is removed when its counter hits zero. Empty when
    /// `context_after == 0`.
    awaiting_after: Vec<(usize, u32)>,
}

/// KWAL command implementation.
///
/// For each utterance, extracts all countable words and checks whether
/// any match the configured keywords (case-insensitive). Matching
/// utterances are collected and displayed in the output.
#[derive(Debug, Clone, Default)]
pub struct KwalCommand {
    config: KwalConfig,
    /// Per-keyword string used for `word_pattern_matches`, folded
    /// once at construction time according to `config.case_sensitive`
    /// (lowercased when `false`, preserved when `true`). Hoisted out
    /// of the per-utterance hot path, `+s` keywords don't change
    /// mid-run, so this avoids `keyword.to_lowercase()` allocations
    /// on every utterance.
    keyword_match_forms: Vec<String>,
}

impl KwalCommand {
    /// Create a KWAL command with the given configuration.
    pub fn new(config: KwalConfig) -> Self {
        let keyword_match_forms = config
            .keywords
            .iter()
            .map(|k| {
                if config.case_sensitive {
                    k.as_str().to_owned()
                } else {
                    k.to_lowercase()
                }
            })
            .collect();
        Self {
            config,
            keyword_match_forms,
        }
    }
}

impl AnalysisCommand for KwalCommand {
    type Config = KwalConfig;
    type State = KwalState;
    type Output = KwalResult;

    /// Find keyword matches in one utterance and record match metadata.
    ///
    /// Context-window ordering invariant: post-context for *earlier*
    /// matches must drain BEFORE the current match is recorded (the
    /// current utterance shouldn't be its own post-context), and the
    /// pre-context ring update must happen AFTER (the current
    /// utterance shouldn't be its own pre-context either).
    fn process_utterance(
        &self,
        utterance: &Utterance,
        file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        if self.config.keywords.is_empty() {
            return;
        }

        // Detect match without the serialized text (cheap).
        let case_sensitive = self.config.case_sensitive;
        let words: Vec<String> = countable_words(&utterance.main.content.content)
            .map(|w| NormalizedWord::from_word_cased(w, case_sensitive).0)
            .collect();

        // `+b` doesn't early-return: even a strict-rejected utterance
        // still has to count as non-match for any open
        // `awaiting_after` and still has to feed the pre-context ring.
        let strict_rejects = self.config.strict_match && words.len() != 1;
        let mut matched = Vec::new();
        if !strict_rejects {
            for (keyword, kw_for_match) in
                self.config.keywords.iter().zip(&self.keyword_match_forms)
            {
                for word in &words {
                    if word_pattern_matches(word, kw_for_match) {
                        matched.push(keyword.clone());
                        break;
                    }
                }
            }
        }

        // Skip the allocating CHAT serialization in the common
        // zero-context, non-match path. Default-config callers (no
        // `+wN`/`-wN`) pay nothing extra per-utterance.
        let needs_text = !matched.is_empty()
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

        if !matched.is_empty() {
            for kw in &matched {
                *state.keyword_counts.entry(kw.to_string()).or_insert(0) += 1;
            }
            let line_number = file_context
                .line_map
                .map(|lm| lm.line_of(utterance.main.span.start))
                .unwrap_or(0);
            let pre_context: Vec<String> = state.recent.iter().cloned().collect();
            let match_idx = state.matches.len();
            state.matches.push(KwalMatch {
                speaker: utterance.main.speaker.as_str().to_owned(),
                utterance_text: utterance_text.clone(),
                filename: file_context.filename.to_owned(),
                keyword: matched[0].to_string(),
                line_number,
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

    /// Move collected match rows and keyword counters into typed output.
    fn finalize(&self, state: Self::State) -> KwalResult {
        KwalResult {
            matches: state.matches,
            keyword_counts: state.keyword_counts,
            legal_chat: self.config.legal_chat,
        }
    }
}
