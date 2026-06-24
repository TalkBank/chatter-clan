//! Multi-word `+s` search groups, shared across CLAN analysis commands.
//!
//! CLAN's `+s` flag can carry a *group* of words (e.g. `+s"the hill"`), not
//! just a single pattern. FREQ counts each adjacent occurrence of the group as
//! one frequency item displayed as the search pattern; KWAL surrounds each
//! match with its context window; COMBO reports the co-occurrence. The matching
//! engine is the same for all three, so it lives in the framework rather than in
//! any one command.
//!
//! This module is the Phase-1 spine of the multi-word search cluster: a group
//! parser and the **default** matcher, which is non-overlapping, consecutive,
//! and in-order on the main tier (`freq.cpp:2465-2548`). CLAN models groups as
//! an `IEMWORDS` linked list (cutt.cpp:357) built by `InsertMulti`
//! (cutt.cpp:4724), one group per `+s` argument containing a `^` or space.
//! The `+c3` any-order (`anyMultiOrder`, freq.cpp:2373), `+c4` sole-content
//! (`onlySpecWsFound`, freq.cpp:2381), `+c2` multiplicity, and `+c7`
//! literal-wildcard (`isMultiWordsActual`, freq.cpp:2444) modes build on this
//! and are added in later phases as typed modes, never as bare bools.
//!
//! Returning match *spans* (not a bool) is the load-bearing design choice that
//! lets one engine serve count (FREQ), context (KWAL), and combination (COMBO).

use super::word_filter::word_pattern_matches;

/// One multi-word `+s` search group: an ordered list of slot patterns parsed by
/// splitting the `+s` string on whitespace. Each slot is a `*`-wildcard pattern
/// over a single token (CLAN's `word_arr`, matched with `uS.patmat`). A group
/// always has at least two slots; a single word is the ordinary per-word `+s`
/// filter, not a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiWordGroup {
    /// The slot patterns, cased for matching: lower-cased when matching is
    /// case-insensitive (CLAN's default for `+s`), preserved otherwise (`+k`).
    slots: Vec<String>,
    /// Whether slot matching is case-sensitive; drives whether tokens are
    /// lower-cased before comparison.
    case_sensitive: bool,
    /// The search pattern as typed (slots re-joined by a single space). This is
    /// the FREQ frequency-table item: CLAN counts under `word_arr` joined by
    /// spaces, not under the matched data words (that is the `+c7` mode).
    display: String,
}

/// A matched occurrence of a group in a token stream: the start index and the
/// number of tokens spanned, so callers can recover the matched region (KWAL
/// context, COMBO scope). For the default sequence match `len` is the slot
/// count; for any-order it is the distance from the first to the last token
/// that filled a slot. FREQ only needs the count, but the span keeps the
/// primitive reusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchSpan {
    /// Index of the first matched token in the stream.
    pub start: usize,
    /// Number of tokens spanned.
    pub len: usize,
}

/// One matched occurrence of a group: the token index that filled each slot, in
/// slot order (length equals the group's slot count, always >= 2). This is the
/// reusable currency of the matcher: FREQ derives the count (and, under `+c7`,
/// the matched words) from it, KWAL derives the context span, COMBO the
/// combination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    slot_tokens: Vec<usize>,
}

impl Match {
    /// The matched word for each slot, in slot order. Used by FREQ `+c7`
    /// (`isMultiWordsActual`, freq.cpp:2444): the displayed item is the words
    /// that actually matched, not the search pattern.
    pub fn matched_words<'a>(&self, tokens: &[&'a str]) -> Vec<&'a str> {
        self.slot_tokens
            .iter()
            .filter_map(|&i| tokens.get(i).copied())
            .collect()
    }

    /// The span from the first to the last matched token (KWAL context window).
    /// `slot_tokens` is never empty, so the `unwrap_or(0)` fallbacks are unused.
    pub fn span(&self) -> MatchSpan {
        let min = self.slot_tokens.iter().copied().min().unwrap_or(0);
        let max = self.slot_tokens.iter().copied().max().unwrap_or(0);
        MatchSpan {
            start: min,
            len: max.saturating_sub(min) + 1,
        }
    }
}

/// How a multi-word group's slots must line up against the token stream.
///
/// CLAN's default is an adjacent, in-order sequence; `+c3` (`anyMultiOrder`,
/// freq.cpp:792) relaxes this to "anywhere and in any order" (manual
/// CLAN.txt:5488). This is the order axis of the multi-word match mode (see
/// [`MultiWordMatch`]); the scope axis is [`MatchScope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchOrder {
    /// CLAN default: the slots must match consecutive tokens, in order.
    #[default]
    Sequence,
    /// CLAN `+c3`: each token fills the first unfilled slot it matches; the
    /// group counts once when every slot is filled (any order, non-adjacent).
    AnyOrder,
}

/// Where in the utterance a multi-word group is allowed to match.
///
/// CLAN's default lets the group match anywhere within a longer utterance;
/// `+c4` (`onlySpecWsFound`, freq.cpp:794) restricts a match to utterances that
/// consist *solely* of the group, i.e. whose token count equals the slot count
/// (manual CLAN.txt:5490-5491).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchScope {
    /// CLAN default: the group may match anywhere in the utterance.
    #[default]
    Anywhere,
    /// CLAN `+c4`: the utterance must consist solely of the group (its token
    /// count equals the group's slot count).
    SoleContent,
}

/// How a multi-word group is matched against an utterance's tokens: the bundle
/// of independent mode axes (currently `+c3` order and `+c4` scope; `+c2`
/// multiplicity and `+c7` wildcard capture are added here as further fields in
/// later phases, never as bare bools). Bundling them keeps the matcher
/// signature and the FREQ config stable as axes are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MultiWordMatch {
    /// CLAN `+c3`: order/adjacency requirement.
    pub order: MatchOrder,
    /// CLAN `+c4`: whole-utterance restriction.
    pub scope: MatchScope,
}

impl MultiWordGroup {
    /// Parse one `+s` value into a group, or `None` when it has fewer than two
    /// words (a single word is the per-word `+s` filter, handled elsewhere).
    /// `case_sensitive` is the `+s` matching polarity (CLAN `+k`).
    pub fn parse(raw: &str, case_sensitive: bool) -> Option<Self> {
        let words: Vec<&str> = raw.split_whitespace().collect();
        if words.len() < 2 {
            return None;
        }
        let display = words.join(" ");
        let slots = words
            .iter()
            .map(|w| {
                if case_sensitive {
                    (*w).to_owned()
                } else {
                    w.to_lowercase()
                }
            })
            .collect();
        Some(Self {
            slots,
            case_sensitive,
            display,
        })
    }

    /// The space-joined search pattern, which is the FREQ frequency-table item.
    pub fn display(&self) -> &str {
        &self.display
    }

    /// The number of slots (words) in the group.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Whether the slot at `k` matches `token`, honouring case sensitivity and
    /// the slot's `*` wildcards (shared `word_pattern_matches`).
    fn slot_matches(&self, k: usize, token: &str) -> bool {
        if self.case_sensitive {
            word_pattern_matches(token, &self.slots[k])
        } else {
            word_pattern_matches(&token.to_lowercase(), &self.slots[k])
        }
    }

    /// All matches of the group in `tokens`, left to right, under the given
    /// [`MultiWordMatch`] mode. Returns one [`Match`] per counted occurrence.
    pub fn matches(&self, tokens: &[&str], mode: MultiWordMatch) -> Vec<Match> {
        // CLAN `+c4`: a sole-content match requires the utterance to be exactly
        // the group (token count == slot count); otherwise nothing matches.
        if mode.scope == MatchScope::SoleContent && tokens.len() != self.slots.len() {
            return Vec::new();
        }
        match mode.order {
            MatchOrder::Sequence => self.matches_sequence(tokens),
            MatchOrder::AnyOrder => self.matches_any_order(tokens),
        }
    }

    /// CLAN's default matcher (`freq.cpp:2465-2548`): non-overlapping,
    /// consecutive, in-order. Scan for a position where every slot matches the
    /// consecutive tokens; on a full match, emit it and resume *after* it
    /// (non-overlapping); otherwise advance by one. Counts every match, not just
    /// the first, matching the default branch which resets and continues.
    fn matches_sequence(&self, tokens: &[&str]) -> Vec<Match> {
        let n = self.slots.len();
        let mut out = Vec::new();
        let mut i = 0;
        while i + n <= tokens.len() {
            if (0..n).all(|k| self.slot_matches(k, tokens[i + k])) {
                out.push(Match {
                    slot_tokens: (i..i + n).collect(),
                });
                i += n;
            } else {
                i += 1;
            }
        }
        out
    }

    /// CLAN's `+c3` matcher (`anyMultiOrder`, freq.cpp:2389-2464): each token
    /// fills the first still-unfilled slot whose pattern it matches; when every
    /// slot is filled the group counts once and all slots reset for the next
    /// occurrence. Order and adjacency are both irrelevant. `slot_tokens` records
    /// which token filled each slot, in slot order (so `+c7` can show them).
    fn matches_any_order(&self, tokens: &[&str]) -> Vec<Match> {
        let n = self.slots.len();
        let mut out = Vec::new();
        let mut slot_tokens: Vec<Option<usize>> = vec![None; n];
        for (idx, token) in tokens.iter().enumerate() {
            for (k, slot) in slot_tokens.iter_mut().enumerate() {
                if slot.is_none() && self.slot_matches(k, token) {
                    *slot = Some(idx);
                    break;
                }
            }
            if slot_tokens.iter().all(Option::is_some) {
                // Every slot is filled, so `flatten` collects all `n` indices.
                out.push(Match {
                    slot_tokens: slot_tokens.iter().flatten().copied().collect(),
                });
                slot_tokens.iter_mut().for_each(|s| *s = None);
            }
        }
        out
    }

    /// The number of matches under `mode`, FREQ's per-group count.
    pub fn count_matches(&self, tokens: &[&str], mode: MultiWordMatch) -> usize {
        self.matches(tokens, mode).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_word_is_not_a_group() {
        assert_eq!(MultiWordGroup::parse("cookie", false), None);
        assert_eq!(MultiWordGroup::parse("   ", false), None);
    }

    #[test]
    fn parses_display_and_slots() {
        let g = MultiWordGroup::parse("the hill", false).expect("group");
        assert_eq!(g.display(), "the hill");
        assert_eq!(g.slot_count(), 2);
    }

    #[test]
    fn adjacent_in_order_matches_once() {
        let g = MultiWordGroup::parse("the hill", false).expect("group");
        assert_eq!(
            g.count_matches(&["up", "the", "hill", "now"], MultiWordMatch::default()),
            1
        );
    }

    #[test]
    fn reversed_or_non_adjacent_does_not_match() {
        let g = MultiWordGroup::parse("the hill", false).expect("group");
        // reversed order
        assert_eq!(
            g.count_matches(&["hill", "the"], MultiWordMatch::default()),
            0
        );
        // in order but not adjacent
        assert_eq!(
            g.count_matches(&["the", "big", "hill"], MultiWordMatch::default()),
            0
        );
    }

    #[test]
    fn counts_every_non_overlapping_occurrence() {
        let g = MultiWordGroup::parse("a b", false).expect("group");
        assert_eq!(
            g.count_matches(&["a", "b", "a", "b"], MultiWordMatch::default()),
            2
        );
        // "a a b": the second a starts the only match
        assert_eq!(
            g.count_matches(&["a", "a", "b"], MultiWordMatch::default()),
            1
        );
    }

    #[test]
    fn wildcard_slots_match() {
        let g = MultiWordGroup::parse("the h*", false).expect("group");
        assert_eq!(
            g.count_matches(&["the", "hill"], MultiWordMatch::default()),
            1
        );
        assert_eq!(
            g.count_matches(&["the", "dog"], MultiWordMatch::default()),
            0
        );
    }

    #[test]
    fn case_insensitive_by_default_sensitive_under_flag() {
        let insensitive = MultiWordGroup::parse("the hill", false).expect("group");
        assert_eq!(
            insensitive.count_matches(&["The", "Hill"], MultiWordMatch::default()),
            1
        );
        let sensitive = MultiWordGroup::parse("the hill", true).expect("group");
        assert_eq!(
            sensitive.count_matches(&["The", "Hill"], MultiWordMatch::default()),
            0
        );
        assert_eq!(
            sensitive.count_matches(&["the", "hill"], MultiWordMatch::default()),
            1
        );
    }

    #[test]
    fn any_order_ignores_order_and_adjacency() {
        let g = MultiWordGroup::parse("a b", false).expect("group");
        // reversed
        assert_eq!(
            g.count_matches(
                &["b", "a"],
                MultiWordMatch {
                    order: MatchOrder::AnyOrder,
                    ..Default::default()
                }
            ),
            1
        );
        // non-adjacent
        assert_eq!(
            g.count_matches(
                &["a", "x", "b"],
                MultiWordMatch {
                    order: MatchOrder::AnyOrder,
                    ..Default::default()
                }
            ),
            1
        );
        // two full sets reset between them
        assert_eq!(
            g.count_matches(
                &["b", "a", "a", "b"],
                MultiWordMatch {
                    order: MatchOrder::AnyOrder,
                    ..Default::default()
                }
            ),
            2
        );
        // a slot left unfilled is not a match
        assert_eq!(
            g.count_matches(
                &["a", "a"],
                MultiWordMatch {
                    order: MatchOrder::AnyOrder,
                    ..Default::default()
                }
            ),
            0
        );
    }

    #[test]
    fn sole_content_requires_the_whole_utterance() {
        let g = MultiWordGroup::parse("a b", false).expect("group");
        let sole = MultiWordMatch {
            scope: MatchScope::SoleContent,
            ..Default::default()
        };
        // the utterance is exactly the group, in order
        assert_eq!(g.count_matches(&["a", "b"], sole), 1);
        // a longer utterance is not sole content, even though it contains a b
        assert_eq!(g.count_matches(&["a", "b", "c"], sole), 0);
        // right length but wrong order needs +c3 too (sequence by default)
        assert_eq!(g.count_matches(&["b", "a"], sole), 0);
        let sole_any = MultiWordMatch {
            order: MatchOrder::AnyOrder,
            scope: MatchScope::SoleContent,
        };
        assert_eq!(g.count_matches(&["b", "a"], sole_any), 1);
    }

    #[test]
    fn matched_words_recover_the_actual_tokens() {
        // `+c7`: a wildcard slot reports the actual matched word.
        let g = MultiWordGroup::parse("the *", false).expect("group");
        let tokens = ["up", "the", "hill", "the", "top"];
        let matches = g.matches(&tokens, MultiWordMatch::default());
        let words: Vec<Vec<&str>> = matches.iter().map(|m| m.matched_words(&tokens)).collect();
        assert_eq!(words, vec![vec!["the", "hill"], vec!["the", "top"]]);
        // The span covers the matched run.
        assert_eq!(matches[0].span(), MatchSpan { start: 1, len: 2 });
    }
}
