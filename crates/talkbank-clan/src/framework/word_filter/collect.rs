//! Recursive main-tier word collectors shared by the countable-word iterators.
//!
//! These private walkers gather the [`Word`]s for which a caller-supplied `keep`
//! predicate returns `true`, descending through groups, quotations, retracings
//! (gated by CLAN `+r6`), and `[: text]` replacements (CLAN `+r5`). They were
//! extracted verbatim from `word_filter.rs`; the public iterators
//! ([`countable_words`](super::countable_words),
//! [`words_for_utterance_length`](super::words_for_utterance_length), ...) call
//! them by name through the `use collect::*;` re-export in the parent.

use talkbank_model::{BracketedItem, ReplacedWord, UtteranceContent, Word};

use super::{ReplacementChoice, RetraceReplaceMode};

/// Recursively collect the words from main-tier content for which `keep`
/// returns `true`, into `out`.
///
/// `keep` is the leaf predicate. The default callers pass
/// [`is_countable_word`](super::is_countable_word); the `+x` length walker
/// ([`words_for_utterance_length`](super::words_for_utterance_length)) passes a
/// predicate that additionally keeps restored unintelligible markers.
/// Parameterizing the predicate (rather than duplicating the walk) guarantees
/// restored markers obey the identical group-recursion and retrace/replacement
/// rules as countable words.
///
/// `mode` (CLAN `+r6`/`+r5`): when `mode.include_retracings` is set the words
/// inside `Retrace` groups are counted in addition to the corrections; a
/// `[: text]` `ReplacedWord` contributes the replacement or the original per
/// `mode.replacement` (independent of retracings, via [`push_replaced_word`]).
///
/// # Invariant
///
/// Every word appended to `out` satisfies `keep(word) == true`.
pub(super) fn collect_countable<'a>(
    content: &'a [UtteranceContent],
    out: &mut Vec<&'a Word>,
    mode: RetraceReplaceMode,
    keep: &dyn Fn(&Word) -> bool,
) {
    for item in content {
        match item {
            UtteranceContent::Word(word)
                if keep(word) => {
                    out.push(word);
                }
            UtteranceContent::AnnotatedWord(annotated)
                if keep(&annotated.inner) => {
                    out.push(&annotated.inner);
                }
            UtteranceContent::ReplacedWord(replaced) => {
                push_replaced_word(replaced, out, mode.replacement, keep);
            }
            UtteranceContent::Group(group) => {
                collect_countable_bracketed(&group.content.content, out, mode, keep);
            }
            UtteranceContent::AnnotatedGroup(annotated) => {
                collect_countable_bracketed(&annotated.inner.content.content, out, mode, keep);
            }
            UtteranceContent::Retrace(retrace)
                // Retrace targets are excluded by default. When `+r6`
                // (`mode.include_retracings`) is set, count the retraced words too.
                if mode.include_retracings => {
                    collect_countable_bracketed(&retrace.content.content, out, mode, keep);
                }
            UtteranceContent::PhoGroup(group) => {
                collect_countable_bracketed(&group.content.content, out, mode, keep);
            }
            UtteranceContent::SinGroup(group) => {
                collect_countable_bracketed(&group.content.content, out, mode, keep);
            }
            UtteranceContent::Quotation(group) => {
                collect_countable_bracketed(&group.content.content, out, mode, keep);
            }
            _ => {}
        }
    }
}

/// Push the word(s) a `[: text]` [`ReplacedWord`] contributes, per CLAN `+r5`
/// (`choice`). The default ([`ReplacementChoice::Replacement`]) counts the
/// replacement (corrected form); [`ReplacementChoice::Original`] counts the
/// replaced surface word. Shared by both walkers so they cannot drift.
fn push_replaced_word<'a>(
    replaced: &'a ReplacedWord,
    out: &mut Vec<&'a Word>,
    choice: ReplacementChoice,
    keep: &dyn Fn(&Word) -> bool,
) {
    match choice {
        ReplacementChoice::Replacement => {
            if !replaced.replacement.words.is_empty() {
                for w in &replaced.replacement.words {
                    if keep(w) {
                        out.push(w);
                    }
                }
            } else if keep(&replaced.word) {
                out.push(&replaced.word);
            }
        }
        // CLAN `+r5`: count the original (replaced) surface form, not the
        // replacement.
        ReplacementChoice::Original => {
            if keep(&replaced.word) {
                out.push(&replaced.word);
            }
        }
    }
}

/// Recursively collect, from bracketed (nested) content, the words for which
/// `keep` returns `true`. The bracketed twin of [`collect_countable`]; see it
/// for the `keep` predicate rationale.
fn collect_countable_bracketed<'a>(
    items: &'a [BracketedItem],
    out: &mut Vec<&'a Word>,
    mode: RetraceReplaceMode,
    keep: &dyn Fn(&Word) -> bool,
) {
    for item in items {
        match item {
            BracketedItem::Word(word) if keep(word) => {
                out.push(word);
            }
            BracketedItem::AnnotatedWord(annotated) if keep(&annotated.inner) => {
                out.push(&annotated.inner);
            }
            BracketedItem::ReplacedWord(replaced) => {
                push_replaced_word(replaced, out, mode.replacement, keep);
            }
            BracketedItem::AnnotatedGroup(annotated) => {
                collect_countable_bracketed(&annotated.inner.content.content, out, mode, keep);
            }
            BracketedItem::Retrace(retrace) if mode.include_retracings => {
                collect_countable_bracketed(&retrace.content.content, out, mode, keep);
            }
            BracketedItem::PhoGroup(group) => {
                collect_countable_bracketed(&group.content.content, out, mode, keep);
            }
            BracketedItem::SinGroup(group) => {
                collect_countable_bracketed(&group.content.content, out, mode, keep);
            }
            BracketedItem::Quotation(group) => {
                collect_countable_bracketed(&group.content.content, out, mode, keep);
            }
            _ => {}
        }
    }
}
