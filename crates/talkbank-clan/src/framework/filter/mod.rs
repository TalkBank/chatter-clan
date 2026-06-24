//! Shared filtering criteria for CLAN analysis commands.
//!
//! This module provides the Rust equivalent of CUTT's speaker selection (`+t`/`-t`),
//! word search (`+s`/`-s`), gem filtering (`+g`/`-g`), and utterance range
//! (`+z`). The [`AnalysisRunner`](super::AnalysisRunner) applies filters before
//! passing utterances to commands, so each command only sees relevant data --
//! exactly like CUTT's `checktier()` + `getwholeutter()`.
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) for the
//! original filter flag semantics.
//!
//! # Filter evaluation order
//!
//! 1. Utterance range (cheapest check, `+z`)
//! 2. Speaker inclusion/exclusion (`+t`/`-t`)
//! 3. Gem segment boundaries (`+g`/`-g`)
//! 4. Word pattern matching (`+s`/`-s`)

use std::borrow::Cow;

use talkbank_model::{Header, SpeakerCode, Utterance};

use super::word_filter::{countable_words_in_utterance, word_pattern_matches};

// The filtering criteria are split across sibling submodules to keep this
// file browseable; the public items are re-exported here so existing
// `filter::<Name>` paths (and the parent `framework`'s `pub use filter::{...}`)
// continue to resolve unchanged.
mod length;
mod range;
mod word_list;

pub use length::{
    CountUnit, LengthComparison, LengthThreshold, ParseUtteranceLengthError, RestoreMarkers,
    UtteranceLengthFilter, parse_restore_marker, parse_utterance_length,
};
pub use range::{ParseUtteranceRangeError, UtteranceRange, parse_utterance_range};
pub use word_list::{LoadWordListError, load_search_expr_file, load_word_list_file};

/// Shared filtering criteria applied before utterances reach a command.
///
/// Replaces CUTT's global filtering flags. The runner evaluates these
/// against each utterance and only passes matching utterances to the
/// command's `process_utterance`.
#[derive(Debug, Clone, Default)]
pub struct FilterConfig {
    /// Include/exclude speakers (CUTT: +t/-t @ID)
    pub speakers: SpeakerFilter,
    /// Include/exclude dependent tiers (CUTT: +t/-t %tier)
    pub tiers: TierFilter,
    /// Word/morpheme search patterns (CUTT: +s/-s)
    pub words: WordFilter,
    /// Gem segment filtering (CUTT: +g/-g)
    pub gems: GemFilter,
    /// Restrict to a 1-based utterance range within each file (CUTT: +z)
    /// inclusive, e.g., `25-125` processes utterances 25-125
    pub utterance_range: Option<UtteranceRange>,
    /// Include only utterances whose length satisfies a comparison (CLAN
    /// `+x C N U`), e.g. `+x>3w` keeps utterances with more than 3 countable
    /// words and `+x>20c` keeps those with more than 20 main-tier characters.
    /// `None` (default) imposes no length gate. The word (`w`), char (`c`), and
    /// morpheme (`m`) units are modeled; the `+xS` content-specification form is
    /// not yet (see [`CountUnit`]).
    pub utterance_length: Option<UtteranceLengthFilter>,
    /// Filter by `@ID` header pattern (CUTT: `+t@ID="lang|*|CHI|*"`).
    ///
    /// When `Some`, the analysis runner uses it twice:
    ///  - **file prefilter:** skip any file whose `@ID` headers all fail
    ///    the match;
    ///  - **utterance filter:** drop utterances whose speaker's `@ID` row
    ///    fails the match.
    ///
    /// `FilterConfig::matches` does not consult this field directly; the
    /// runner is responsible for both passes because it owns the parsed
    /// `@ID` headers.
    pub id_filter: Option<super::IdFilter>,
    /// Filter by participant role (CLAN: `+t#ROLE`).
    ///
    /// `FilterConfig::matches` does not consult this field directly;
    /// the runner reads the speaker's `ParticipantRole` from the
    /// `@ID:` header map and drops utterances whose role is not in
    /// the include list. When `include` is empty, role filtering is
    /// inactive.
    pub roles: RoleFilter,
}

/// Speaker inclusion/exclusion filter.
///
/// When `include` is non-empty, only those speakers are processed.
/// When `exclude` is non-empty, those speakers are skipped.
/// When both are empty, all speakers pass (default behavior).
#[derive(Debug, Clone, Default)]
pub struct SpeakerFilter {
    /// Speakers to include (empty = include all)
    pub include: Vec<SpeakerCode>,
    /// Speakers to exclude
    pub exclude: Vec<SpeakerCode>,
}

/// Participant-role inclusion filter (CLAN: `+t#ROLE`).
///
/// When `include` is non-empty, only utterances from speakers whose
/// `@ID:` role field matches one of the listed roles (case-
/// insensitive) are processed. When `include` is empty, role
/// filtering is inactive (every speaker passes).
///
/// Files with no `@ID:` headers cannot have role filtering applied
///, the runner processes them unchanged, matching CLAN's behaviour
/// (no `@ID` data ⇒ no `+t#ROLE` match information).
#[derive(Debug, Clone, Default)]
pub struct RoleFilter {
    /// Role names to include. Stored as raw user-supplied strings;
    /// the matcher in `runner.rs` compares case-insensitively against
    /// the speaker's `ParticipantRole` from `@ID:`.
    pub include: Vec<String>,
}

/// Dependent tier inclusion/exclusion filter.
///
/// Controls which dependent tiers are visible to commands.
/// By default all tiers are visible.
#[derive(Debug, Clone, Default)]
pub struct TierFilter {
    /// Tier kinds to include (empty = include all)
    pub include: Vec<super::TierKind>,
    /// Tier kinds to exclude
    pub exclude: Vec<super::TierKind>,
}

/// Word/morpheme pattern filter (CUTT: +s/-s).
///
/// When `include` is non-empty, only utterances containing at least
/// one matching word are processed. When `exclude` is non-empty,
/// utterances containing any matching word are skipped.
///
/// `case_sensitive` (CLAN `+k`) defaults to `false`, patterns and
/// words are lower-cased before matching. When `true`, both sides
/// keep their original casing and an exact-case match is required.
#[derive(Debug, Clone)]
pub struct WordFilter {
    /// Word patterns to include (empty = include all)
    pub include: Vec<super::WordPattern>,
    /// Word patterns to exclude
    pub exclude: Vec<super::WordPattern>,
    /// Case-sensitive matching (CLAN `+k`). `false` lower-cases
    /// both pattern and word before comparison.
    pub case_sensitive: bool,
    /// Where this filter applies in the pipeline. See
    /// [`WordFilterMode`]. Required: every construction site names
    /// the mode explicitly; there is no default, because picking
    /// the wrong mode silently produces non-CLAN output (over- or
    /// under-counting).
    pub mode: WordFilterMode,
}

impl Default for WordFilter {
    /// Empty utterance-gate filter, all utterances pass, no
    /// include/exclude. This is the safe baseline used by tests and
    /// by commands that have no `+sWORD` / `-sWORD` involvement.
    fn default() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
            case_sensitive: false,
            mode: WordFilterMode::UtteranceContext,
        }
    }
}

/// Where a [`WordFilter`] applies in the analysis pipeline.
///
/// CLAN's `+sWORD` / `-sWORD` flag has a per-command semantic;
/// this enum makes that explicit at the type level. There is no
/// `Default` impl, every construction site must name the variant.
///
/// Why the mode exists: a single utterance-level word gate silently
/// produces non-CLAN output for per-word commands. CLAN registers
/// `+s` words in its common search table but each command consults
/// the table at its own output stage; FREQ gates each word at
/// count-emit time and still prints every speaker block
/// (freq.cpp:1234-1249), whereas an utterance-level gate counts
/// every word of a matching utterance and drops non-matching
/// speakers entirely. Verified empirically against the CLAN freq
/// binary (2026-05-27): `freq +s"the"` printed `1 the` for the
/// matching speaker plus a zero-total block for the other, while
/// the utterance-gated implementation printed all three words of
/// the matching utterance and no second speaker block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordFilterMode {
    /// Filter at the utterance level via [`FilterConfig::matches`].
    /// Utterances containing no matching word are skipped entirely.
    /// Right for KWAL, COMBO, and any command whose output unit is
    /// the utterance.
    UtteranceContext,
    /// Filter at the per-word emit level via
    /// [`WordFilter::word_matches`]. The utterance gate is a no-op;
    /// the command applies the filter to each word at counting time.
    /// Right for FREQ, UNIQ, PHONFREQ, and any command whose output
    /// unit is the (speaker, word) pair.
    PerWordEmit,
}

/// Gem segment filter (CUTT: +g/-g).
///
/// When `include` is non-empty, only utterances within matching
/// @BG/@EG segments are processed. Gem labels use case-insensitive matching.
#[derive(Debug, Clone, Default)]
pub struct GemFilter {
    /// Gem labels to include (empty = include all)
    pub include: Vec<super::GemLabel>,
    /// Gem labels to exclude
    pub exclude: Vec<super::GemLabel>,
}

impl FilterConfig {
    /// Check whether an utterance passes all filter criteria.
    ///
    /// # Preconditions
    /// - `utterance` is a valid parsed utterance from a ChatFile
    /// - `active_gems` is the set of currently open @BG labels
    /// - `utterance_index` is the 1-based index of this utterance within its file
    ///
    /// # Returns
    /// `true` if the utterance should be passed to the command
    pub fn matches(
        &self,
        utterance: &Utterance,
        active_gems: &[String],
        utterance_index: usize,
    ) -> bool {
        // Check utterance range first (cheapest check)
        if let Some(range) = self.utterance_range
            && !range.contains(utterance_index)
        {
            return false;
        }

        // CLAN `+x C N w`: drop utterances whose countable-word count fails the
        // length comparison (the gate counts words, so it follows the range
        // check but precedes the per-word `+s` work).
        if let Some(length) = &self.utterance_length
            && !length.matches(utterance)
        {
            return false;
        }

        self.speakers.matches(&utterance.main.speaker)
            && self.gems.matches(active_gems)
            && self.words.matches(utterance)
    }
}

impl SpeakerFilter {
    /// Check whether a speaker passes this filter.
    ///
    /// - If `include` is non-empty, speaker must be in the include list.
    /// - If `exclude` is non-empty, speaker must NOT be in the exclude list.
    /// - If both are empty, all speakers pass.
    pub fn matches(&self, speaker: &SpeakerCode) -> bool {
        if !self.include.is_empty() && !self.include.contains(speaker) {
            return false;
        }
        if self.exclude.contains(speaker) {
            return false;
        }
        true
    }
}

impl WordFilter {
    /// Utterance-level gate. Returns `true` (utterance passes) when:
    /// - [`WordFilterMode::PerWordEmit`]: always (filtering happens
    ///   at emit time via [`WordFilter::word_matches`]).
    /// - [`WordFilterMode::UtteranceContext`]: include is empty OR
    ///   at least one countable word matches an include pattern,
    ///   AND no countable word matches an exclude pattern.
    ///
    /// Patterns support `*` wildcards; case-insensitive unless
    /// [`WordFilter::case_sensitive`] is set (CLAN `+k`).
    pub fn matches(&self, utterance: &Utterance) -> bool {
        if self.mode == WordFilterMode::PerWordEmit {
            return true;
        }
        if self.include.is_empty() && self.exclude.is_empty() {
            return true;
        }

        // Normalize both sides identically. Case-insensitive (default,
        // CLAN's behaviour without `+k`) lower-cases both pattern and
        // word text; case-sensitive keeps the original casing on both
        // sides. On the case-sensitive path we skip the per-pattern
        // `to_lowercase` allocation by borrowing the originals.
        let include_folded: Vec<Cow<'_, str>> = self
            .include
            .iter()
            .map(|p| fold_case(p, self.case_sensitive))
            .collect();
        let exclude_folded: Vec<Cow<'_, str>> = self
            .exclude
            .iter()
            .map(|p| fold_case(p, self.case_sensitive))
            .collect();

        // The cleaned word text needs to outlive `word_texts`, so we
        // collect the owned `cleaned_text().to_string()` first, then
        // borrow from that vector.
        let words_owned: Vec<String> = countable_words_in_utterance(utterance)
            .map(|w| w.cleaned_text().to_string())
            .collect();
        let word_texts: Vec<Cow<'_, str>> = words_owned
            .iter()
            .map(|s| fold_case(s.as_str(), self.case_sensitive))
            .collect();

        // If include patterns specified, at least one word must match
        if !include_folded.is_empty() {
            let has_match = word_texts.iter().any(|text| {
                include_folded
                    .iter()
                    .any(|pattern| word_pattern_matches(text, pattern))
            });
            if !has_match {
                return false;
            }
        }

        // If exclude patterns specified, no word may match
        if !exclude_folded.is_empty() {
            let has_excluded = word_texts.iter().any(|text| {
                exclude_folded
                    .iter()
                    .any(|pattern| word_pattern_matches(text, pattern))
            });
            if has_excluded {
                return false;
            }
        }

        true
    }

    /// Per-word predicate. Returns `true` (word passes) when:
    /// include is empty OR word matches an include pattern, AND
    /// exclude is empty OR word does not match any exclude pattern.
    /// Mode is not consulted; callers (FREQ, UNIQ, PHONFREQ, …) are
    /// responsible for choosing per-word semantics.
    ///
    // PERF: per-call this re-folds every include/exclude pattern
    // (one `String` allocation each when case-insensitive). For
    // hot-path FREQ over millions of words × M patterns, that
    // dominates. Future fix: pre-fold patterns once at construction
    // (a compiled-WordFilter newtype) so this method is allocation-
    // free per call. Tracked under the FREQ implementation audit.
    pub fn word_matches(&self, text: &str) -> bool {
        if self.include.is_empty() && self.exclude.is_empty() {
            return true;
        }

        let folded = fold_case(text, self.case_sensitive);

        if !self.include.is_empty() {
            let has_match = self.include.iter().any(|pattern| {
                let pattern_folded = fold_case(pattern.as_ref(), self.case_sensitive);
                word_pattern_matches(folded.as_ref(), pattern_folded.as_ref())
            });
            if !has_match {
                return false;
            }
        }

        if !self.exclude.is_empty() {
            let has_excluded = self.exclude.iter().any(|pattern| {
                let pattern_folded = fold_case(pattern.as_ref(), self.case_sensitive);
                word_pattern_matches(folded.as_ref(), pattern_folded.as_ref())
            });
            if has_excluded {
                return false;
            }
        }

        true
    }

    /// The number of include `+s` patterns this word matches, or 0 if it matches
    /// an exclude pattern. Uses the same case-folding and `*`-wildcard rules as
    /// [`WordFilter::word_matches`].
    ///
    /// CLAN `+c2` (`capwd == 3`, freq.cpp:432-438) counts a word once per
    /// matching `+s` pattern, whereas the default counts it once regardless of
    /// how many patterns match (`word_matches`). Only meaningful with a
    /// non-empty include list.
    pub fn count_matching_includes(&self, text: &str) -> usize {
        let folded = fold_case(text, self.case_sensitive);
        let matches_pattern = |pattern: &super::WordPattern| {
            let pattern_folded = fold_case(pattern.as_ref(), self.case_sensitive);
            word_pattern_matches(folded.as_ref(), pattern_folded.as_ref())
        };
        if self.exclude.iter().any(matches_pattern) {
            return 0;
        }
        self.include.iter().filter(|p| matches_pattern(p)).count()
    }
}

/// Apply CLAN's `+k` case-sensitivity rule to a pattern or word.
///
/// `case_sensitive = true` returns the input borrowed (matching CLAN's
/// behaviour with `+k`). `case_sensitive = false` returns a lower-
/// cased copy.
fn fold_case(s: &str, case_sensitive: bool) -> Cow<'_, str> {
    if case_sensitive {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(s.to_lowercase())
    }
}

impl GemFilter {
    /// Check whether the current gem context passes this filter.
    ///
    /// - If `include` is non-empty, at least one active gem must match.
    /// - If `exclude` is non-empty, no active gem must match.
    /// - If both are empty, all contexts pass.
    pub fn matches(&self, active_gems: &[String]) -> bool {
        if !self.include.is_empty() {
            let has_match = active_gems.iter().any(|gem| {
                self.include
                    .iter()
                    .any(|pattern| gem.eq_ignore_ascii_case(pattern))
            });
            if !has_match {
                return false;
            }
        }
        if !self.exclude.is_empty() {
            let has_excluded = active_gems.iter().any(|gem| {
                self.exclude
                    .iter()
                    .any(|pattern| gem.eq_ignore_ascii_case(pattern))
            });
            if has_excluded {
                return false;
            }
        }
        true
    }
}

/// Track @BG/@EG gem boundaries across utterances.
///
/// Call `update` for each utterance's preceding headers to maintain
/// the set of currently active gem labels.
pub fn update_active_gems(headers: &[Header], active_gems: &mut Vec<String>) {
    for header in headers {
        match header {
            Header::BeginGem { label: Some(label) } => {
                active_gems.push(label.as_str().to_owned());
            }
            Header::EndGem { label: Some(label) } => {
                // Remove the most recent matching @BG
                if let Some(pos) = active_gems
                    .iter()
                    .rposition(|g| g.eq_ignore_ascii_case(label.as_str()))
                {
                    active_gems.remove(pos);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
