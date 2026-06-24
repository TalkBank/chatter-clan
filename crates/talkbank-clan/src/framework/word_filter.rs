//! Semantic word filtering for analysis commands.
//!
//! CLAN's original analysis commands exclude certain "words" from counting by
//! checking raw string prefixes: `0`, `&`, `+`, `-`, `#`. We use typed AST
//! fields instead, representing each of these categories as distinct types:
//!
//! | CLAN text pattern | Semantic intent | AST representation |
//! |---|---|---|
//! | `word[0] == '#'` | Skip pauses | `Pause` (not a `Word` at all) |
//! | `word[0] == '+'` | Skip terminators | `Terminator` (separate AST level) |
//! | `word == "xxx"` | Skip unintelligible | `Word { untranscribed: Some(Unintelligible) }` |
//! | `word == "yyy"` | Skip phonetic coding | `Word { untranscribed: Some(Phonetic) }` |
//! | `word == "www"` | Skip untranscribable | `Word { untranscribed: Some(Untranscribed) }` |
//! | `word[0] == '0'` | Skip omitted words | `Word { category: Some(Omission) }` |
//! | `word[0] == '&'` | Skip fillers/nonwords | `Word { category: Some(Filler\|Nonword\|Fragment) }` |
//! | `word[0] == '-'` | (unclear) | Not a meaningful CHAT category |
//!
//! Pauses, terminators, events, and actions are already separate AST node
//! types that our tree walk never visits. The only filtering needed is on
//! `Word` nodes that carry semantic annotations indicating they are not
//! countable lexical items.

use talkbank_model::model::content::word::UntranscribedStatus;
use talkbank_model::{Utterance, UtteranceContent, Word, WordCategory};

// The recursive collectors, presence predicates, and the `+s` pattern matcher
// are split into sibling submodules to keep this file browseable; the public
// items are re-exported so existing `word_filter::<Name>` paths (and the parent
// `framework` re-export) continue to resolve unchanged.
mod collect;
mod pattern;
mod presence;

use collect::collect_countable;
pub use pattern::word_pattern_matches;
pub use presence::{has_countable_words, main_tier_has_excluded_untranscribed};

/// Which word a `[: text]` replacement contributes to the count (CLAN `+r5`).
///
/// `[: text]` annotations (`gots [: got]`) record both the original surface form
/// and a corrected replacement. CLAN's default counts the replacement; `+r5`
/// (`R5`, `cutt.cpp:9549-9553`) counts the original instead. Independent of
/// retracings (`+r6`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplacementChoice {
    /// CLAN default: count the replacement (corrected form), e.g. `got`.
    #[default]
    Replacement,
    /// CLAN `+r5`: count the original (replaced) surface form, e.g. `gots`.
    Original,
}

/// How the word walker treats CHAT retracings and `[: text]` replacements: two
/// independent CLAN axes, `+r6` (retracings) and `+r5` (replacement choice).
///
/// Bundled so the walker carries one mode rather than parallel flags. The
/// default (no retracings, count the replacement) is CLAN's FREQ default and is
/// byte-identical to [`countable_words`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetraceReplaceMode {
    /// CLAN `+r6`: include the retraced material (words inside `Retrace` groups).
    pub include_retracings: bool,
    /// CLAN `+r5`: which word a `[: text]` replacement contributes.
    pub replacement: ReplacementChoice,
}

/// Extra characters that split a counted word into separate tokens (CLAN `+pS`,
/// `cutt.cpp:9798-9818`; manual `cutt.cpp:9204` "add S to word delimiters").
///
/// CLAN appends the characters of `S` to its global word-delimiter set and
/// re-tokenizes, so a word containing one of them is broken at that point and
/// each piece is counted on its own (`+p_` breaks `New_York` into `New` and
/// `York`). Whitespace characters are dropped on construction, matching CLAN's
/// `!isSpace` skip. Empty by default: no extra delimiters, no splitting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WordDelimiters(Vec<char>);

impl WordDelimiters {
    /// Build from the characters of a `+pS` argument, dropping whitespace.
    pub fn new(chars: impl IntoIterator<Item = char>) -> Self {
        Self(chars.into_iter().filter(|c| !c.is_whitespace()).collect())
    }

    /// Whether no extra delimiter is configured (the default; no splitting).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `c` is one of the extra delimiter characters.
    fn contains(&self, c: char) -> bool {
        self.0.contains(&c)
    }

    /// Split `text` into the non-empty segments between delimiter characters.
    /// With no delimiters configured this still yields the whole `text` as one
    /// segment (callers gate on [`is_empty`](Self::is_empty) before splitting).
    pub fn split<'a>(&'a self, text: &'a str) -> impl Iterator<Item = &'a str> {
        text.split(move |c| self.contains(c))
            .filter(|segment| !segment.is_empty())
    }
}

/// Determine whether a word contributes lexical material to analysis counts.
///
/// A word is **not countable** if it represents:
/// - Untranscribed material (`xxx`, `yyy`, `www`), unintelligible or
///   deliberately omitted speech that has no lexical content
/// - Omitted words (`0is`, `0det`), words the speaker should have produced
///   but didn't; they describe an absence, not a presence
/// - Fillers (`&-um`, `&-uh`), non-lexical vocalizations used for turn-holding
/// - Nonwords (`&~gaga`), babbling or invented sounds with no lexical status
/// - Phonological fragments (`&+fr`), incomplete word attempts
///
/// These correspond to CLAN's default exclusions, but expressed through the
/// type system rather than string prefix matching.
///
/// # What is already excluded by tree structure
///
/// The following are separate AST node types that the tree walk never reaches:
/// - **Pauses** (`Pause`), CLAN's `#` prefix check
/// - **Events** (`Event`), CLAN's `&=` prefix check (e.g., `&=laughs`)
/// - **Actions** (`Action`), standalone `0`
/// - **Terminators** (`Terminator`), CLAN's `+` prefix check
///
/// # Postcondition
///
/// If this returns `true`, the word has genuine lexical content suitable
/// for frequency counting, MLU computation, and other analyses.
pub fn is_countable_word(word: &Word) -> bool {
    // Untranscribed material has no lexical content
    if word.untranscribed().is_some() {
        return false;
    }

    // Omissions, fillers, nonwords, and fragments are not lexical items
    if let Some(ref category) = word.category
        && !is_countable_category(category)
    {
        return false;
    }

    // Defensive: empty cleaned_text means no lexical content.
    // The model currently prevents constructing empty words, but this
    // guard ensures correctness if that invariant ever relaxes.
    if word.cleaned_text().is_empty() {
        return false;
    }

    true
}

/// Determine whether this word category remains countable for analysis.
///
/// Only `CAOmission` is countable among categories; it represents uncertain
/// but present speech in CA transcription, unlike standard omissions which
/// represent absent speech.
fn is_countable_category(category: &WordCategory) -> bool {
    match category {
        // Standard omission: word was NOT produced (e.g., "0is" = missing copula)
        WordCategory::Omission => false,
        // Filler: non-lexical vocalization (e.g., "&-um")
        WordCategory::Filler => false,
        // Nonword: babbling with no lexical status (e.g., "&~gaga")
        WordCategory::Nonword => false,
        // Fragment: incomplete word attempt (e.g., "&+fr")
        WordCategory::PhonologicalFragment => false,
        // CA omission: uncertain but present speech in CA mode, countable
        // because the transcriber heard something and attempted to transcribe it
        WordCategory::CAOmission => true,
    }
}

/// Iterator over all countable words in utterance main-tier content.
///
/// Walks the `UtteranceContent` + `BracketedItem` tree recursively, yielding
/// each [`Word`] that passes [`is_countable_word`]. The caller receives
/// `&Word` references and decides how to use them (e.g., to extract
/// [`cleaned_text()`][Word::cleaned_text] for frequency keys).
///
/// Internally collects into a `Vec<&Word>` before iterating; this keeps the
/// borrow checker happy across the two-level tree and is negligible for the
/// 10-50 word utterances typical in CHAT.
///
/// # Usage
///
/// ```ignore
/// for word in countable_words(&utterance.main.content.content) {
///     let key = NormalizedWord::from_word(word);
///     // ...
/// }
/// ```
pub fn countable_words(content: &[UtteranceContent]) -> impl Iterator<Item = &Word> {
    let mut words: Vec<&Word> = Vec::new();
    collect_countable(
        content,
        &mut words,
        RetraceReplaceMode::default(),
        &is_countable_word,
    );
    words.into_iter()
}

/// Like [`countable_words`], but with explicit retracing (`+r6`) and replacement
/// (`+r5`) modes (CLAN). The default [`RetraceReplaceMode`] is byte-identical to
/// [`countable_words`]. The FREQ main-tier count uses this.
pub fn countable_words_with_mode(
    content: &[UtteranceContent],
    mode: RetraceReplaceMode,
) -> impl Iterator<Item = &Word> {
    let mut words: Vec<&Word> = Vec::new();
    collect_countable(content, &mut words, mode, &is_countable_word);
    words.into_iter()
}

/// Convenience wrapper: iterate countable words in an utterance's main tier.
///
/// Equivalent to `countable_words(&utterance.main.content.content)`.
pub fn countable_words_in_utterance(utterance: &Utterance) -> impl Iterator<Item = &Word> {
    countable_words(&utterance.main.content.content)
}

/// Like [`countable_words`], but counting retraced material (CLAN `+r6`).
///
/// When `include_retracings` is true, the words inside `Retrace` groups (the
/// retraced material) are yielded in addition to the corrections. `[: text]`
/// `ReplacedWord`s always count the replacement regardless of this flag (a
/// retraced replaced word, `w [: x] [//] x`, is reached through the `Retrace`
/// recursion and contributes its replacement `x`). With `false` this is
/// byte-identical to [`countable_words`].
pub fn countable_words_with_retracings(
    content: &[UtteranceContent],
    include_retracings: bool,
) -> impl Iterator<Item = &Word> {
    countable_words_with_mode(
        content,
        RetraceReplaceMode {
            include_retracings,
            ..RetraceReplaceMode::default()
        },
    )
}

/// Like [`countable_words_in_utterance`], but with retracings control.
pub fn countable_words_in_utterance_with_retracings(
    utterance: &Utterance,
    include_retracings: bool,
) -> impl Iterator<Item = &Word> {
    countable_words_with_retracings(&utterance.main.content.content, include_retracings)
}

/// Walk the words that count toward a CLAN `+x` utterance-length measure: every
/// countable word PLUS any unintelligible marker (`xxx`/`yyy`/`www`) whose
/// status is listed in `restore` (CLAN `+xxxx`/`+xyyy`/`+xwww`, which re-include
/// markers the length count normally strips, `correctForXXXYYYWWW`
/// `cutt.cpp:16260`).
///
/// Restored markers are visited through the SAME recursive walker as countable
/// words, so they obey the identical group-recursion and retrace/replacement
/// rules and stay consistent with the base `+x` count chatter already validates
/// against CLAN. When `restore` is empty this yields exactly
/// [`countable_words_in_utterance`]. The `-xS` exclude list (which removes named
/// real words from the count) is applied by the caller, not here.
pub fn words_for_utterance_length<'a>(
    utterance: &'a Utterance,
    restore: &[UntranscribedStatus],
) -> impl Iterator<Item = &'a Word> {
    let keep = |word: &Word| {
        is_countable_word(word)
            || word
                .untranscribed()
                .is_some_and(|status| restore.contains(&status))
    };
    let mut words: Vec<&Word> = Vec::new();
    collect_countable(
        &utterance.main.content.content,
        &mut words,
        RetraceReplaceMode::default(),
        &keep,
    );
    words.into_iter()
}

/// Match CLAN's `+c` / `+c0` predicate: the input's first character
/// is uppercase. Returns `false` for empty input or for first
/// characters that have no notion of case (digits, punctuation),
/// matching CLAN's behaviour of dropping non-letter-led tokens from
/// a capitalised search.
///
/// Module-private: the public surface is
/// [`CapitalizationFilter::includes`], which dispatches to this.
fn starts_with_uppercase(text: &str) -> bool {
    text.chars().next().is_some_and(char::is_uppercase)
}

/// Match CLAN's `+c1` predicate: at least one uppercase letter
/// appears AFTER position 0 (e.g. `McDonald`, `iPhone`, `eBay`).
/// Words with only initial capitalization (`Cookie`) do NOT match.
///
/// Module-private; reached through [`CapitalizationFilter::includes`].
fn has_mid_uppercase(text: &str) -> bool {
    text.chars().skip(1).any(char::is_uppercase)
}

/// CLAN's `+c` / `+c0` / `+c1` capitalization-mode filter.
///
/// Used by FREQ and VOCD; shared so both commands agree on which
/// words are counted. Stored on each command's `Config` as
/// `capitalization: CapitalizationFilter`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CapitalizationFilter {
    /// No filter, every countable word is considered. Default;
    /// matches CLAN's default behaviour without `+c`/`+c0`/`+c1`.
    #[default]
    Any,
    /// CLAN `+c` / `+c0`, only count words whose first character
    /// is uppercase (proper nouns and sentence-initial capitals).
    InitialUpper,
    /// CLAN `+c1`, only count words with an uppercase letter
    /// AFTER position 0 (e.g. `McDonald`, `iPhone`, `eBay`). The
    /// initial character is irrelevant for this predicate.
    MidUpper,
}

impl CapitalizationFilter {
    /// Whether the given text passes this filter.
    pub fn includes(self, text: &str) -> bool {
        match self {
            CapitalizationFilter::Any => true,
            CapitalizationFilter::InitialUpper => starts_with_uppercase(text),
            CapitalizationFilter::MidUpper => has_mid_uppercase(text),
        }
    }
}

/// Decide whether an utterance consists *solely* of `solo_words`.
///
/// Implements CLAN's command-specific `+gS` semantic for MLU and MLT:
/// an utterance with at least one countable word, all of which match an
/// entry in `solo_words`, is excluded from analysis. Distinct from the
/// inherited `+gX` gem-segment filter (CLAN docs: "exclude utterance
/// consisting solely of specified word S").
///
/// `solo_words` is expected to be **pre-normalized**: lower-cased to the
/// same form `NormalizedWord::from_word` produces. Callers (typically
/// `MluCommand::new` / `MltCommand::new`) normalize once at construction
/// so this per-utterance hot path does no per-call allocation.
///
/// # Returns
///
/// * `false` if the utterance has no countable words (caller should
///   reject earlier via [`has_countable_words`] anyway).
/// * `false` if `solo_words` is empty.
/// * `true` iff every countable word's normalized form appears in
///   `solo_words`.
pub fn utterance_is_solo_excluded(utterance: &Utterance, solo_words: &[String]) -> bool {
    if solo_words.is_empty() {
        return false;
    }

    let mut saw_any = false;
    for word in countable_words_in_utterance(utterance) {
        saw_any = true;
        let normalized = crate::framework::NormalizedWord::from_word(word);
        if !solo_words.iter().any(|s| s == normalized.as_str()) {
            return false;
        }
    }
    saw_any
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `WordDelimiters` splits on the configured characters, drops empty
    /// segments, ignores whitespace in the delimiter set, and keeps a trailing
    /// marker on the final segment (the `+p_` `choo_choo`/`chup@o` behaviour).
    #[test]
    fn word_delimiters_split_behaviour() {
        let underscore = WordDelimiters::new("_".chars());
        assert!(!underscore.is_empty());
        assert_eq!(
            underscore.split("choo_choo").collect::<Vec<_>>(),
            vec!["choo", "choo"]
        );
        // A trailing word-form marker rides on the final segment.
        assert_eq!(
            underscore
                .split("chup_chup_chup_chup@o")
                .collect::<Vec<_>>(),
            vec!["chup", "chup", "chup", "chup@o"]
        );
        // Consecutive / leading / trailing delimiters yield no empty segments.
        assert_eq!(
            underscore.split("_a__b_").collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        // Whitespace in the delimiter argument is dropped (CLAN's `!isSpace`).
        assert!(WordDelimiters::new(" \t".chars()).is_empty());
        // Multiple delimiter characters all split.
        let multi = WordDelimiters::new("_-".chars());
        assert_eq!(
            multi.split("New_York-City").collect::<Vec<_>>(),
            vec!["New", "York", "City"]
        );
    }

    /// `Any` (default) admits every input, including empty strings,
    /// digits-only tokens, and lowercase words.
    #[test]
    fn capitalization_any_admits_everything() {
        let f = CapitalizationFilter::Any;
        assert!(f.includes(""));
        assert!(f.includes("cookie"));
        assert!(f.includes("Cookie"));
        assert!(f.includes("McDonald"));
        assert!(f.includes("123"));
        assert!(f.includes("."));
    }

    /// `InitialUpper` (CLAN `+c` / `+c0`) admits only inputs whose
    /// first character has uppercase casing. Digits, punctuation,
    /// lowercase initials, and empty strings all fail.
    #[test]
    fn capitalization_initial_upper_requires_uppercase_first_char() {
        let f = CapitalizationFilter::InitialUpper;
        assert!(f.includes("Cookie"));
        assert!(f.includes("I"));
        assert!(f.includes("McDonald"));
        assert!(!f.includes("cookie"));
        assert!(!f.includes("iPhone")); // initial is lowercase
        assert!(!f.includes(""));
        assert!(!f.includes("123"));
        assert!(!f.includes("."));
    }

    /// `MidUpper` (CLAN `+c1`) admits only inputs with at least one
    /// uppercase letter AFTER position 0. `McDonald` and `iPhone`
    /// pass; `Cookie` (initial-only uppercase) and `cookie` (no
    /// uppercase at all) both fail.
    #[test]
    fn capitalization_mid_upper_requires_uppercase_after_first_char() {
        let f = CapitalizationFilter::MidUpper;
        assert!(f.includes("McDonald"));
        assert!(f.includes("iPhone"));
        assert!(f.includes("eBay"));
        assert!(!f.includes("Cookie")); // initial-only uppercase
        assert!(!f.includes("cookie"));
        assert!(!f.includes("I")); // only one character
        assert!(!f.includes(""));
        assert!(!f.includes("123"));
    }

    /// `Default` is `Any`, `#[default]` annotation on the enum.
    #[test]
    fn capitalization_default_is_any() {
        let f = CapitalizationFilter::default();
        assert_eq!(f, CapitalizationFilter::Any);
    }

    /// Plain lexical words should be countable.
    #[test]
    fn simple_word_is_countable() {
        let word = Word::simple("dog");
        assert!(is_countable_word(&word));
    }

    /// Untranscribed tokens (`xxx/yyy/www`) should be excluded.
    #[test]
    fn untranscribed_words_are_not_countable() {
        let xxx = Word::simple("xxx");
        let yyy = Word::simple("yyy");
        let www = Word::simple("www");

        assert!(!is_countable_word(&xxx));
        assert!(!is_countable_word(&yyy));
        assert!(!is_countable_word(&www));
    }

    /// Omission/filler/nonword/fragment categories should be excluded.
    #[test]
    fn omissions_fillers_nonwords_fragments_not_countable() {
        let omission = Word::simple("is").with_category(WordCategory::Omission);
        let filler = Word::simple("um").with_category(WordCategory::Filler);
        let nonword = Word::simple("gaga").with_category(WordCategory::Nonword);
        let fragment = Word::simple("fr").with_category(WordCategory::PhonologicalFragment);

        assert!(!is_countable_word(&omission));
        assert!(!is_countable_word(&filler));
        assert!(!is_countable_word(&nonword));
        assert!(!is_countable_word(&fragment));
    }

    /// CA omissions represent present-but-uncertain speech and remain countable.
    #[test]
    fn ca_omission_is_countable() {
        // CA omissions represent uncertain but present speech
        let ca = Word::simple("word").with_category(WordCategory::CAOmission);
        assert!(is_countable_word(&ca));
    }

    /// `has_countable_words` should differentiate lexical from non-lexical input.
    #[test]
    fn has_countable_words_detects_lexical_content() {
        use talkbank_model::UtteranceContent;

        // Utterance with a normal word has countable content
        let word = Word::simple("dog");
        let content = vec![UtteranceContent::Word(Box::new(word))];
        assert!(has_countable_words(&content));

        // Utterance with only untranscribed material has no countable content
        let xxx = Word::simple("xxx");
        let content = vec![UtteranceContent::Word(Box::new(xxx))];
        assert!(!has_countable_words(&content));
    }
}
