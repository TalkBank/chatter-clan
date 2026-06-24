//! `--id-filter PATTERN`, CLAN `+t@ID="..."` selection by `@ID` header.
//!
//! The rewriter in `crates/talkbank-clan/src/clan_args/` translates
//! `+t@ID=PATTERN` into `--id-filter PATTERN`; this module is the
//! clap-consumable side and the matcher.
//!
//! ## Matching semantics (reproduces CLAN `uS.patmat`)
//!
//! CLAN matches the pattern against the **raw pipe-delimited `@ID` content** as
//! a whole-string, case-insensitive wildcard glob:
//!
//! - `*` matches any run of characters, including `|` and the empty run.
//! - Every other character matches itself, case-insensitively.
//! - The pattern must match the **entire** `@ID` string. Bare tokens do NOT
//!   substring-match (`+t@ID=Target_Child` selects nothing); callers anchor
//!   with `*`, e.g. `+t@ID="*|Target_Child|*"` selects any participant whose
//!   `@ID` contains `|Target_Child|` (the leading/trailing `*` absorb the
//!   surrounding fields).
//!
//! This is deliberately NOT a field-by-field, column-aware match: the `*`
//! wildcards span field boundaries exactly as CLAN's do, so the manual's
//! canonical `*|Target_Child|*` (a role glob, role being the 8th field) works.
//!
//! chatter does **not** reproduce `patmat`'s CHAT-word metacharacters
//! (`% _ \ ( )` and its morpheme-character awareness): those are word-tier
//! search features inapplicable to `@ID` content, which carries no CHAT
//! morpheme markup. The CLAN manual documents `@ID` selection as a plain
//! wildcard match. An empty pattern is a no-op filter (matches every `@ID`).

use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use talkbank_model::{IDHeader, WriteChat};

/// CLAN `+t@ID` filter: a case-insensitive `*`-wildcard pattern matched against
/// the rendered raw `@ID` content.
///
/// A newtype over the raw pattern (rather than a parsed column model) because
/// CLAN's match is a whole-string glob, not a positional field match.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdFilter {
    pattern: String,
}

impl IdFilter {
    /// Borrow the raw pattern string (used for the CLAN-style scope banner).
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Whether the given `@ID` header satisfies this filter.
    pub fn matches(&self, header: &IDHeader) -> bool {
        // An empty `+t@ID=` is a no-op filter.
        if self.pattern.is_empty() {
            return true;
        }
        glob_match_ci(&self.pattern, &render_id_content(header))
    }
}

impl FromStr for IdFilter {
    type Err = Infallible;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            pattern: input.to_owned(),
        })
    }
}

impl fmt::Display for IdFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.pattern)
    }
}

/// Render an `@ID` header to its raw pipe-delimited content (the bytes CLAN's
/// `+t@ID` pattern matches against), e.g.
/// `eng|Manchester|CHI|1;10.07|female|TD|MC|Target_Child|||`.
fn render_id_content(header: &IDHeader) -> String {
    let mut rendered = String::new();
    // `WriteChat` emits `@ID:\t<content>`; the pattern matches the content.
    let _ = header.write_chat(&mut rendered);
    match rendered.strip_prefix("@ID:\t") {
        Some(content) => content.to_owned(),
        None => rendered,
    }
}

/// Case-insensitive full-string wildcard match: `*` matches any run of
/// characters (including the empty run); every other character matches itself
/// (case-insensitively). The pattern must match the entire `text`.
///
/// Iterative two-pointer match with greedy-`*` backtracking, O(pattern x text)
/// worst case, which is irrelevant at `@ID`-string lengths.
fn glob_match_ci(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.to_lowercase().chars().collect();
    let txt: Vec<char> = text.to_lowercase().chars().collect();

    let mut p = 0usize;
    let mut t = 0usize;
    // The most recent `*` in the pattern and the text position it began at, so
    // a failed match can backtrack and let that `*` consume one more character.
    let mut last_star: Option<usize> = None;
    let mut star_match_start = 0usize;

    while t < txt.len() {
        if p < pat.len() && pat[p] == '*' {
            last_star = Some(p);
            star_match_start = t;
            p += 1;
        } else if p < pat.len() && pat[p] == txt[t] {
            p += 1;
            t += 1;
        } else if let Some(star) = last_star {
            // Backtrack: the `*` absorbs one more text character.
            p = star + 1;
            star_match_start += 1;
            t = star_match_start;
        } else {
            return false;
        }
    }

    // Any unmatched pattern tail must be all `*` for a full match.
    while p < pat.len() && pat[p] == '*' {
        p += 1;
    }
    p == pat.len()
}

/// Parse a clap-friendly `--id-filter` argument. The pattern is taken verbatim
/// (any string is a valid glob), so this never fails.
pub fn parse_id_filter(input: &str) -> Result<IdFilter, String> {
    Ok(IdFilter {
        pattern: input.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::{LanguageCode, LanguageCodes};

    fn header_chi() -> IDHeader {
        // Renders as `eng||CHI|||||Target_Child|||`.
        IDHeader::new("eng", "CHI", "Target_Child")
    }

    fn header_mot() -> IDHeader {
        IDHeader::new("eng", "MOT", "Mother")
    }

    fn matches(pattern: &str, header: &IDHeader) -> bool {
        let filter: IdFilter = pattern.parse().unwrap();
        filter.matches(header)
    }

    #[test]
    fn empty_pattern_matches_everything() {
        assert!(matches("", &header_chi()));
        assert!(matches("", &header_mot()));
    }

    #[test]
    fn lone_star_matches_everything() {
        assert!(matches("*", &header_chi()));
        assert!(matches("*", &header_mot()));
    }

    /// The bug this row fixes: a role glob whose `*`s span field boundaries.
    /// `Target_Child` is the 8th `@ID` field, so a field-by-field reading of
    /// `*|Target_Child|*` (corpus = Target_Child) never matched.
    #[test]
    fn role_glob_selects_by_at_id_role() {
        assert!(matches("*|Target_Child|*", &header_chi()));
        assert!(!matches("*|Target_Child|*", &header_mot()));
    }

    #[test]
    fn match_is_case_insensitive() {
        assert!(matches("*|target_child|*", &header_chi()));
        assert!(matches("*|TARGET_CHILD|*", &header_chi()));
    }

    /// Full match, not substring: a bare token does not match; the user anchors
    /// with `*`. `*Child*` matches because `Target_Child` contains `Child`.
    #[test]
    fn bare_token_requires_anchoring() {
        assert!(!matches("Target_Child", &header_chi()));
        assert!(matches("*Target_Child*", &header_chi()));
        assert!(matches("*Child*", &header_chi()));
    }

    #[test]
    fn speaker_glob_filters_by_speaker() {
        assert!(matches("*|CHI|*", &header_chi()));
        assert!(!matches("*|CHI|*", &header_mot()));
    }

    #[test]
    fn language_glob_anchors_at_start() {
        assert!(matches("eng*", &header_chi()));
        assert!(!matches("fra*", &header_chi()));
    }

    /// Multi-language `@ID` (`eng, yue|...`): the comma-joined language list is
    /// part of the matched string, so `*yue*` and `eng*` match it; there is no
    /// special set-membership rule (CLAN globs the raw content).
    #[test]
    fn multilingual_id_matched_by_glob() {
        let mut multi = header_chi();
        multi.language =
            LanguageCodes::new(vec![LanguageCode::from("eng"), LanguageCode::from("yue")]);
        assert!(matches("eng*", &multi));
        assert!(matches("*yue*", &multi));
        assert!(!matches("*fra*", &multi));
    }

    #[test]
    fn corpus_glob_matches_present_corpus() {
        let mut with_corpus = header_chi();
        with_corpus.corpus = talkbank_model::CorpusName::from("Manchester");
        assert!(matches("*|Manchester|*", &with_corpus));
        assert!(!matches("*|Manchester|*", &header_chi()));
    }

    #[test]
    fn display_is_the_raw_pattern() {
        for raw in ["", "*|Target_Child|*", "eng*", "*Child*"] {
            let filter: IdFilter = raw.parse().unwrap();
            assert_eq!(filter.to_string(), raw);
        }
    }
}
