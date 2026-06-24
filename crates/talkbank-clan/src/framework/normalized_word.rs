//! Normalized word text for use as frequency-counting map keys.
//!
//! The transformation `word.cleaned_text().to_lowercase()` is the canonical
//! form used as map keys across all analysis commands. [`NormalizedWord`]
//! encapsulates this transformation so it is always applied consistently
//! across FREQ, MAXWD, DIST, COOCCUR, KWAL, COMBO, and all other commands
//! that need word-level deduplication or matching.
//!
//! [`clan_display_form()`] provides an alternative form that preserves `+` in
//! compound words (`ice+cream`) for CLAN-compatible output rendering.
//!
//! # Forward-compatibility
//!
//! As the grammar moves toward a looser `grammar.js` that pushes more checks
//! from parsing into validation, words that previously would not reach the AST
//! may start arriving as less-classified `Word` nodes. Centralizing the
//! normalization here means only [`NormalizedWord::from_word()`] needs updating
//! when that happens -- no command code changes are required.

use std::borrow::Borrow;
use std::fmt;

use serde::Serialize;
use talkbank_model::{Word, WordContent, WriteChat};

/// Lowercased, cleaned word text suitable for frequency counting.
///
/// Encapsulates `word.cleaned_text().to_lowercase()`, the canonical form
/// used as map keys across all analysis commands:
/// - `freq.rs` word counts
/// - `maxwd.rs` unique-word deduplication
/// - `dist.rs` per-word distribution tracking
/// - `cooccur.rs` word-pair keys
/// - `kwal.rs` / `combo.rs` keyword matching
///
/// # Using as a map key
///
/// ```
/// use std::collections::HashMap;
/// use talkbank_clan::framework::NormalizedWord;
/// let mut map: HashMap<NormalizedWord, u64> = HashMap::new();
/// // `map.get("hello")` works because NormalizedWord: Borrow<str>
/// ```
///
/// # Invariant
///
/// The inner `String` is always `Word::cleaned_text()` (CHAT markup stripped),
/// optionally lower-cased. Case-folding is controlled by the caller via
/// [`Self::from_word`] (always lowercased, CLAN default) versus
/// [`Self::from_word_cased`] (lowercased iff `case_sensitive` is `false`,
/// preserved otherwise, controls CLAN `+k`).
///
/// Never construct with `NormalizedWord(raw_string)` directly outside of this
/// module; always go through one of the constructors so the invariant stays
/// in one place.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct NormalizedWord(pub(crate) String);

impl NormalizedWord {
    /// Construct the canonical lowercased form of a word, CLAN's default.
    ///
    /// Applies `word.cleaned_text()` (strips CHAT markup / trailing punctuation)
    /// then `to_lowercase()`. Equivalent to `from_word_cased(word, false)`.
    ///
    /// # Precondition
    ///
    /// `word` must pass [`crate::framework::word_filter::is_countable_word`].
    /// Results are unspecified (but safe) for non-countable words.
    pub fn from_word(word: &Word) -> Self {
        Self::from_word_cased(word, false)
    }

    /// Construct the word key honouring CLAN's `+k` flag. When
    /// `case_sensitive` is `false` (CLAN default), the result is lower-cased
    /// `cleaned_text`; when `true`, original case is preserved.
    ///
    /// Used by every analysis command's `process_utterance` so the `+k`
    /// branch lives in one place rather than being inlined per command.
    ///
    /// # Precondition
    ///
    /// `word` must pass [`crate::framework::word_filter::is_countable_word`].
    pub fn from_word_cased(word: &Word, case_sensitive: bool) -> Self {
        Self::from_text_cased(word.cleaned_text(), case_sensitive)
    }

    /// Construct the word key from an already-cleaned `&str` (e.g. `%mor`
    /// serializations), honouring `+k`. Sibling of [`Self::from_word_cased`]
    /// for callers that don't have a `&Word`.
    pub fn from_text_cased(text: &str, case_sensitive: bool) -> Self {
        let key = if case_sensitive {
            text.to_owned()
        } else {
            text.to_lowercase()
        };
        NormalizedWord(key)
    }

    /// Return the normalized text as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Return the CLAN display form of a word (lowercased, overlap markers stripped).
///
/// CLAN preserves `+` in compound words (`ice+cream`, `choo+choo's`) and
/// uses lowercased raw text. Our `NormalizedWord` uses `cleaned_text()`
/// which strips `+`, so multiple commands need this alternative form for
/// CLAN-compatible output.
///
/// Note: CLAN `freq` is an exception; it preserves original case. Use
/// [`clan_display_form_preserve_case()`] for freq-style output.
pub fn clan_display_form(word: &Word) -> String {
    strip_overlap_markers(&word.raw_text().to_lowercase())
}

/// Return the CLAN display form of a word preserving original case.
///
/// Used by FREQ which displays words in their original casing.
pub fn clan_display_form_preserve_case(word: &Word) -> String {
    strip_overlap_markers(word.raw_text())
}

/// CLAN `+r1`/`+r2`/`+r3` (`Parans` 1/2/3, `cutt.cpp:9530-9583`; manual §14.5):
/// how a word's omitted-material parentheses (e.g. `(g)` in `bein(g)`) render
/// when the word is counted. CLAN applies the mode once, before counting, so it
/// drives BOTH the grouping key and the displayed form; this enum keeps the two
/// consistent. It affects only words containing a [`WordContent::Shortening`];
/// every other word renders identically in all three modes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ParenthesisMode {
    /// `+r1`, CLAN's DEFAULT: remove the parentheses but KEEP the omitted
    /// letters (`bein(g)` -> `being`). For the key this reproduces
    /// `Word::cleaned_text()` byte-for-byte.
    #[default]
    RemoveParens,
    /// `+r2`: keep the parentheses literally (`bein(g)`). For the display this
    /// reproduces `Word::raw_text()`.
    KeepParens,
    /// `+r3`: remove the omitted (parenthesized) material entirely
    /// (`bein(g)` -> `bein`).
    RemoveMaterial,
}

/// CLAN `+r7` (`R7Slash/Tilda/Caret/Colon`, `cutt.cpp:9569-9574`; manual §14.5):
/// whether within-word prosodic symbols are kept in the counted word form.
///
/// CLAN's default strips ALL within-word prosodic / CA annotation (`ca:t`==`cat`,
/// `hm:`==`hm`, `ˈwater`==`water`); `+r7` re-includes the `/~^:` set, so
/// `ca:t`!=`cat`. chatter models three of those: Lengthening (`:`), SyllablePause
/// (`^`), and CliticBoundary (`~`). Stress (`ˈ`) and CA (`↑`) stay STRIPPED in
/// both modes (CLAN strips them too, with buggy split artifacts under `+r7` that
/// chatter does not reproduce). Like the parenthesis mode, this drives BOTH the
/// grouping key and the display so they stay consistent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProsodyMode {
    /// CLAN's DEFAULT: strip within-word prosodic symbols (`ca:t` -> `cat`).
    #[default]
    Strip,
    /// CLAN `+r7`: keep `:` / `^` / `~` (Lengthening / SyllablePause /
    /// CliticBoundary), so `ca:t` stays distinct from `cat`.
    Keep,
}

/// Render one `WordContent` element of a counted word form into `out`, given the
/// parenthesis and prosody modes. `include_compound` keeps the compound marker
/// `+` (the display does; the key does not). The match is exhaustive (no
/// catch-all): a future element variant is a compile error here, not a silent
/// keep/drop. Stress / CA / underline / overlap are ALWAYS stripped; the
/// `+r7`-kept prosodic trio is kept only under [`ProsodyMode::Keep`].
fn render_word_element(
    out: &mut String,
    item: &WordContent,
    parens: ParenthesisMode,
    prosody: ProsodyMode,
    include_compound: bool,
) {
    match item {
        WordContent::Text(t) => out.push_str(t.as_ref()),
        WordContent::Shortening(s) => write_shortening(out, s.as_ref(), parens),
        WordContent::CompoundMarker(_) => {
            if include_compound {
                out.push('+');
            }
        }
        // CLAN `+r7` keeps these three (`:` / `^` / `~`); the default strips them.
        WordContent::Lengthening(_)
        | WordContent::SyllablePause(_)
        | WordContent::CliticBoundary(_) => {
            if prosody == ProsodyMode::Keep {
                // Infallible write to a String (the clan-crate `let _` idiom).
                let _ = item.write_chat(out);
            }
        }
        // Always stripped from the counted word form (CLAN's default and `+r7`).
        WordContent::StressMarker(_)
        | WordContent::CAElement(_)
        | WordContent::CADelimiter(_)
        | WordContent::UnderlineBegin(_)
        | WordContent::UnderlineEnd(_)
        | WordContent::OverlapPoint(_) => {}
    }
}

/// Whether `word` contains any omitted-material shortening, the only content the
/// parenthesis mode affects. Words without one take the unchanged display path.
fn has_shortening(word: &Word) -> bool {
    word.content
        .iter()
        .any(|item| matches!(item, WordContent::Shortening(_)))
}

/// Render one shortening's `text` under `mode` into `out`.
fn write_shortening(out: &mut String, text: &str, mode: ParenthesisMode) {
    match mode {
        ParenthesisMode::RemoveParens => out.push_str(text),
        ParenthesisMode::KeepParens => {
            out.push('(');
            out.push_str(text);
            out.push(')');
        }
        ParenthesisMode::RemoveMaterial => {}
    }
}

/// The grouping-KEY surface for `word` under the parens + prosody modes. It
/// keeps `Text` and `Shortening` (the latter per `parens`), and keeps the
/// `+r7` prosodic trio (`:`/`^`/`~`) only when `prosody` is
/// [`ProsodyMode::Keep`]. The compound marker is excluded (the key drops `+`,
/// matching `cleaned_text()`). `(RemoveParens, Strip)` is byte-identical to
/// `Word::cleaned_text()`.
fn parans_key_surface(word: &Word, parens: ParenthesisMode, prosody: ProsodyMode) -> String {
    let mut out = String::new();
    for item in &word.content {
        render_word_element(&mut out, item, parens, prosody, false);
    }
    out
}

/// The DISPLAY surface for `word` under the parens + prosody modes: CLAN's
/// counted word form, `Text` + `Shortening` (per `parens`) + `CompoundMarker`
/// (`+`), plus the `+r7`-kept prosodic trio (`:`/`^`/`~`) when `prosody` is
/// [`ProsodyMode::Keep`]; Stress / CA / overlap / underline are always stripped.
/// Called for words with a shortening or a prosodic marker (see
/// [`parans_display`]); for a word with neither under default modes it equals
/// the `raw_text`-based [`clan_display_form`].
fn parans_display_surface(word: &Word, parens: ParenthesisMode, prosody: ProsodyMode) -> String {
    let mut out = String::new();
    for item in &word.content {
        render_word_element(&mut out, item, parens, prosody, true);
    }
    out
}

/// Whether `word` carries any within-word prosodic / CA / underline / clitic
/// annotation that CLAN's default counted form strips (everything other than
/// `Text`, `Shortening`, and `CompoundMarker`, excluding `OverlapPoint` which the
/// `clan_display_form` path already strips). Such words take the AST-render
/// display path in [`parans_display`]; all others keep the byte-identical
/// `raw_text`-based path, so the strip is scoped to exactly the affected words.
fn has_prosodic(word: &Word) -> bool {
    word.content.iter().any(|item| {
        matches!(
            item,
            WordContent::CAElement(_)
                | WordContent::CADelimiter(_)
                | WordContent::StressMarker(_)
                | WordContent::Lengthening(_)
                | WordContent::SyllablePause(_)
                | WordContent::UnderlineBegin(_)
                | WordContent::UnderlineEnd(_)
                | WordContent::CliticBoundary(_)
        )
    })
}

/// The FREQ grouping key for `word` under the parenthesis `mode`, honouring
/// `+k`. For the default `(RemoveParens, Strip)` modes this is identical to
/// [`NormalizedWord::from_word_cased`], so it takes that path directly and
/// reuses the word's `OnceLock`-cached `cleaned_text()` (no per-word surface
/// allocation in the common case). The non-default modes re-render the word's
/// shortenings / prosody so e.g. `+r2` groups `bein(g)` apart from a
/// spelled-out `being`, or `+r7` keeps `ca:t` apart from `cat`.
pub fn parans_normalized_key(
    word: &Word,
    parens: ParenthesisMode,
    prosody: ProsodyMode,
    case_sensitive: bool,
) -> NormalizedWord {
    if parens == ParenthesisMode::RemoveParens && prosody == ProsodyMode::Strip {
        return NormalizedWord::from_word_cased(word, case_sensitive);
    }
    NormalizedWord::from_text_cased(&parans_key_surface(word, parens, prosody), case_sensitive)
}

/// The FREQ display form for `word` under the parenthesis (`parens`) and prosody
/// (`prosody`, CLAN `+r7`) modes, honouring `+k` (case preserved iff
/// `case_sensitive`). Words with neither a shortening nor a within-word prosodic
/// marker take the unchanged [`clan_display_form`] path (zero blast radius); a
/// word with either is rendered AST-first per the modes.
pub fn parans_display(
    word: &Word,
    parens: ParenthesisMode,
    prosody: ProsodyMode,
    case_sensitive: bool,
) -> String {
    // Words with neither a shortening nor within-word prosodic markers take the
    // byte-identical `raw_text`-based path (zero blast radius). Words with a
    // shortening (parens mode) or prosodic markers (stripped by default, kept by
    // `+r7`) are rendered AST-first via `parans_display_surface`.
    if !has_shortening(word) && !has_prosodic(word) {
        return if case_sensitive {
            clan_display_form_preserve_case(word)
        } else {
            clan_display_form(word)
        };
    }
    let surface = parans_display_surface(word, parens, prosody);
    let cased = if case_sensitive {
        surface
    } else {
        surface.to_lowercase()
    };
    strip_overlap_markers(&cased)
}

/// Strip CA overlap markers (⌈⌉⌊⌋ and indexed variants) from text.
fn strip_overlap_markers(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '⌈' | '⌉' | '⌊' | '⌋'))
        .collect()
}

impl fmt::Display for NormalizedWord {
    /// Print the normalized token text without additional formatting.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for NormalizedWord {
    /// Expose the normalized token as `&str` for generic APIs.
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Enables `map.get("hello")` on `HashMap<NormalizedWord, _>`, no temporary
/// `NormalizedWord` allocation required at lookup sites.
impl Borrow<str> for NormalizedWord {
    /// Enable zero-allocation `&str` lookup against `NormalizedWord` map keys.
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::Word;

    /// Construction from `Word` should lowercase and clean CHAT markup.
    #[test]
    fn normalized_word_lowercases() {
        let word = Word::simple("HELLO");
        let nw = NormalizedWord::from_word(&word);
        assert_eq!(nw.as_str(), "hello");
    }

    /// `Ord` should provide deterministic lexical ordering for map iteration.
    #[test]
    fn normalized_word_ord_for_map_ordering() {
        let a = NormalizedWord(String::from("apple"));
        let b = NormalizedWord(String::from("banana"));
        assert!(a < b);
    }

    /// `Borrow<str>` should allow `HashMap::get` with plain `&str` keys.
    #[test]
    fn borrow_enables_str_lookup() {
        use std::collections::HashMap;
        let mut map: HashMap<NormalizedWord, u64> = HashMap::new();
        map.insert(NormalizedWord(String::from("hello")), 42);
        // Lookup with &str, works via Borrow<str>
        assert_eq!(map.get("hello"), Some(&42));
    }

    /// Build `bein(g)`: `Text("bein")` + `Shortening("g")`.
    fn bein_g() -> Word {
        use talkbank_model::{WordShortening, WordText};
        Word::new_unchecked("bein(g)", "being").with_content(vec![
            WordContent::Text(WordText::new_unchecked("bein")),
            WordContent::Shortening(WordShortening::new_unchecked("g")),
        ])
    }

    /// The three parenthesis modes render `bein(g)`'s display per CLAN
    /// `+r1`/`+r2`/`+r3`.
    #[test]
    fn parenthesis_modes_render_display() {
        let word = bein_g();
        assert_eq!(
            parans_display(
                &word,
                ParenthesisMode::RemoveParens,
                ProsodyMode::Strip,
                false
            ),
            "being"
        );
        assert_eq!(
            parans_display(
                &word,
                ParenthesisMode::KeepParens,
                ProsodyMode::Strip,
                false
            ),
            "bein(g)"
        );
        assert_eq!(
            parans_display(
                &word,
                ParenthesisMode::RemoveMaterial,
                ProsodyMode::Strip,
                false
            ),
            "bein"
        );
    }

    /// Zero-blast-radius invariant: the default `RemoveParens` key is identical
    /// to the existing `cleaned_text`-based key, so the default grouping is
    /// unchanged. The other modes re-key the shortening.
    #[test]
    fn remove_parens_key_matches_cleaned_text() {
        let word = bein_g();
        assert_eq!(
            parans_normalized_key(
                &word,
                ParenthesisMode::RemoveParens,
                ProsodyMode::Strip,
                false
            ),
            NormalizedWord::from_word(&word)
        );
        assert_eq!(
            parans_normalized_key(
                &word,
                ParenthesisMode::KeepParens,
                ProsodyMode::Strip,
                false
            )
            .as_str(),
            "bein(g)"
        );
        assert_eq!(
            parans_normalized_key(
                &word,
                ParenthesisMode::RemoveMaterial,
                ProsodyMode::Strip,
                false
            )
            .as_str(),
            "bein"
        );
    }
}
