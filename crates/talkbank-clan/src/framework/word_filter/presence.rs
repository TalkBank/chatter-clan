//! Presence predicates: does an utterance carry any countable lexical word, and
//! does its main tier carry an excluded untranscribed marker.
//!
//! Extracted verbatim from `word_filter.rs`; the parent re-exports the public
//! items ([`has_countable_words`], [`main_tier_has_excluded_untranscribed`]) so
//! `word_filter::has_countable_words` etc. continue to resolve.

use talkbank_model::Utterance;
use talkbank_model::alignment::helpers::{TierDomain, WordItem, walk_words};
use talkbank_model::model::content::word::UntranscribedStatus;

use super::is_countable_word;

/// Check whether utterance content contains any countable lexical word.
///
/// This is used by MLU to exclude utterances that consist entirely of
/// untranscribed material (e.g., `*CHI: xxx .`) from the utterance count.
/// Such utterances would otherwise deflate MLU by adding zero-morpheme
/// utterances to the denominator.
///
/// # Precondition
///
/// `content` should be the main tier content of an utterance.
pub fn has_countable_words(content: &[talkbank_model::UtteranceContent]) -> bool {
    use talkbank_model::UtteranceContent;
    for item in content {
        match item {
            UtteranceContent::Word(word) if is_countable_word(word) => {
                return true;
            }
            UtteranceContent::AnnotatedWord(annotated) if is_countable_word(&annotated.inner) => {
                return true;
            }
            UtteranceContent::ReplacedWord(replaced) => {
                // Replacements represent corrected forms; they are countable
                if !replaced.replacement.words.is_empty() {
                    for w in &replaced.replacement.words {
                        if is_countable_word(w) {
                            return true;
                        }
                    }
                } else if is_countable_word(&replaced.word) {
                    return true;
                }
            }
            UtteranceContent::Group(group)
                if has_countable_words_bracketed(&group.content.content) =>
            {
                return true;
            }
            UtteranceContent::AnnotatedGroup(annotated)
                if has_countable_words_bracketed(&annotated.inner.content.content) =>
            {
                return true;
            }
            UtteranceContent::PhoGroup(group)
                if has_countable_words_bracketed(&group.content.content) =>
            {
                return true;
            }
            UtteranceContent::SinGroup(group)
                if has_countable_words_bracketed(&group.content.content) =>
            {
                return true;
            }
            UtteranceContent::Quotation(group)
                if has_countable_words_bracketed(&group.content.content) =>
            {
                return true;
            }
            // Non-word content (events, pauses, actions, etc.) doesn't count
            _ => {}
        }
    }
    false
}

/// Whether an utterance's MAIN tier carries a standalone untranscribed token
/// (`xxx` unintelligible, `yyy` phonological, `www` untranscribable) whose
/// status is NOT in `re_included`.
///
/// CLAN MLU (and MLT) exclude the ENTIRE utterance in which such a token
/// appears, by default, not just the token. The CLAN manual §7.21 point 2 is
/// explicit: "the symbols xxx, yyy, and www are also excluded by default, as
/// are the utterances in which they appear." CLAN implements this in
/// `mlu_excludeUtter` (`mllib.cpp:303-348`, returns TRUE for a standalone
/// `xxx`/`yyy`/`www` word), invoked on the MAIN tier (`mlu.cpp:509`). That is
/// why `*CHI: it xxx xxx` is dropped from MLU even though its `%mor` is just
/// `pron|it`: the count is driven off `%mor`, but the exclusion is driven off
/// the main tier.
///
/// `re_included` carries the statuses that CLAN `+sxxx`/`+syyy` re-admit to the
/// utterance count (the marker string itself stays out of the morpheme count,
/// but the utterance is no longer dropped, manual §7.21 pt5). An empty slice is
/// the default: every `xxx`/`yyy`/`www` triggers exclusion. `www`
/// (`Untranscribed`) is never re-includable, so it is never placed in this
/// slice.
///
/// This is intentionally MLU/MLT-scoped, NOT a general analysis predicate: FREQ
/// counts the words on a line that includes `xxx` (manual: "counts utterances
/// and words on a line that may include xxx (unlike MLU)"). Retrace groups are
/// skipped (`TierDomain::Mor`) because retraced material is already excluded
/// from the MLU computation by default (manual §7.21 point 3), so an `xxx` that
/// survives only inside a `[//]` retrace must not trigger whole-utterance
/// exclusion.
pub fn main_tier_has_excluded_untranscribed(
    utterance: &Utterance,
    re_included: &[UntranscribedStatus],
) -> bool {
    let mut found = false;
    walk_words(
        &utterance.main.content.content,
        Some(TierDomain::Mor),
        &mut |item| {
            if let WordItem::Word(word) = item
                && let Some(status) = word.untranscribed()
                && !re_included.contains(&status)
            {
                found = true;
            }
        },
    );
    found
}

/// Check whether bracketed content contains any countable words.
fn has_countable_words_bracketed(items: &[talkbank_model::BracketedItem]) -> bool {
    use talkbank_model::BracketedItem;
    for item in items {
        match item {
            BracketedItem::Word(word) if is_countable_word(word) => {
                return true;
            }
            BracketedItem::AnnotatedWord(annotated) if is_countable_word(&annotated.inner) => {
                return true;
            }
            BracketedItem::ReplacedWord(replaced) => {
                if !replaced.replacement.words.is_empty() {
                    for w in &replaced.replacement.words {
                        if is_countable_word(w) {
                            return true;
                        }
                    }
                } else if is_countable_word(&replaced.word) {
                    return true;
                }
            }
            BracketedItem::AnnotatedGroup(annotated)
                if has_countable_words_bracketed(&annotated.inner.content.content) =>
            {
                return true;
            }
            BracketedItem::PhoGroup(group)
                if has_countable_words_bracketed(&group.content.content) =>
            {
                return true;
            }
            BracketedItem::SinGroup(group)
                if has_countable_words_bracketed(&group.content.content) =>
            {
                return true;
            }
            BracketedItem::Quotation(group)
                if has_countable_words_bracketed(&group.content.content) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}
