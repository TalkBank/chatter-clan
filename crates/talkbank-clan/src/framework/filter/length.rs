//! CLAN `+x C N U` utterance-length filtering.
//!
//! Models the comparison operator ([`LengthComparison`]), the count unit
//! ([`CountUnit`]: word / char / morpheme), the threshold
//! ([`LengthThreshold`]), the unintelligible-marker restore set
//! ([`RestoreMarkers`]), and the assembled filter ([`UtteranceLengthFilter`])
//! plus its `+x` spec parser ([`parse_utterance_length`]). Extracted verbatim
//! from the `filter` module; the parent re-exports the public items so
//! `filter::UtteranceLengthFilter` etc. continue to resolve.

use talkbank_model::Utterance;
use talkbank_model::Word;
use talkbank_model::model::content::word::UntranscribedStatus;
use thiserror::Error;

use crate::framework::chat_ast::count_traced_morphemes_in_utterance;
use crate::framework::domain_types::WordPattern;
use crate::framework::word_filter::{word_pattern_matches, words_for_utterance_length};

/// Comparison operator for CLAN `+x C N` utterance-length filtering. An enum
/// rather than a sign integer so the three cases are explicit at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthComparison {
    /// CLAN `+x>N`: keep utterances strictly longer than the threshold.
    GreaterThan,
    /// CLAN `+x<N`: keep utterances strictly shorter than the threshold.
    LessThan,
    /// CLAN `+x=N`: keep utterances of exactly the threshold length.
    Equal,
}

/// The unit CLAN `+x C N U` counts to measure utterance length. The suffix
/// letter (`w`/`c`/`m`) is a typed axis, not a scalar: each unit measures a
/// different quantity, so the choice is modeled explicitly rather than carried
/// as an opaque flag-prefix character (the field guide's
/// flag-prefix-is-not-a-scalar rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountUnit {
    /// CLAN `+x…w`: count the main-tier countable words (`cutt.cpp:16508`,
    /// `CntFUttLen == 1`).
    Word,
    /// CLAN `+x…c`: count the characters of the main-tier countable words
    /// (`cutt.cpp:16343`, `CntFUttLen == 3`). A clean main-tier measure with no
    /// `%mor` involvement.
    Char,
    /// CLAN `+x…m`: count the §7.21-traced morphemes on the `%mor` tier, via the
    /// shared MLU counter ([`count_traced_morphemes_in_utterance`]). chatter
    /// computes the CORRECT UD count (including `-Ger`) and **DivergesFromClan**:
    /// CLAN's `+x…m` (`cutt.cpp:16409` `CntFUttLen == 2`) both misreads UD
    /// features as morphemes via the raw delimiter walk AND leaks the `%mor` tier
    /// into FREQ's output (the documented doubling bug).
    Morpheme,
}

/// Length threshold for [`UtteranceLengthFilter`] (the `N` in `+x C N U`).
/// Newtyped so it is not confused with a count, index, or limit at the seam.
/// Unit-agnostic: the same threshold compares against a word count, a character
/// count, or a morpheme count per the filter's [`CountUnit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthThreshold(pub usize);

/// CLAN `+x C N U`: include only utterances whose length in `unit` satisfies
/// `comparison` against `threshold` (e.g. `+x>3w` keeps utterances with more
/// than 3 countable words; `+x>20c` keeps those with more than 20 main-tier
/// characters; `+x=5m` keeps those with exactly 5 traced morphemes).
///
/// `exclude_from_count` is CLAN's `-xS` content-specification list (manual
/// 6405): it tunes WHICH words count toward the length, NOT what FREQ outputs
/// (`-xS` removes matching words from the length count). `restore` is the `+xS`
/// *include* counterpart for the unintelligible markers (`+xxxx`/`+xyyy`/`+xwww`
/// re-include `xxx`/`yyy`/`www`, which the count strips by default). Not `Copy`
/// because of the `Vec` / [`RestoreMarkers`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtteranceLengthFilter {
    /// The comparison operator (`>`, `<`, `=`).
    pub comparison: LengthComparison,
    /// The threshold the comparison is against, measured in [`Self::unit`].
    pub threshold: LengthThreshold,
    /// The unit the length is measured in (`w` word / `c` char / `m` morpheme).
    pub unit: CountUnit,
    /// CLAN `-xS`: words removed from the length count (word/char units).
    pub exclude_from_count: Vec<WordPattern>,
    /// CLAN `+xxxx`/`+xyyy`/`+xwww`: unintelligible markers restored INTO the
    /// length count (word/char units). Default empty = CLAN's default of
    /// stripping `xxx`/`yyy`/`www` from the count.
    pub restore: RestoreMarkers,
}

/// CLAN `+xxxx` / `+xyyy` / `+xwww`: which unintelligible markers are restored
/// into the `+x` length count. By default the count strips `xxx`/`yyy`/`www`
/// (`correctForXXXYYYWWW`, `cutt.cpp:16260`); each `+x<marker>` flag adds its
/// marker back (`cutt.cpp:9890-9896` set `restoreXXX`/`restoreYYY`/`restoreWWW`).
///
/// Modeled as a set of the typed [`UntranscribedStatus`] rather than three
/// parallel bools (no boolean blindness; the field guide's
/// flag-prefix-is-not-a-scalar discipline). Applies only to the word and char
/// units (the main-tier measures); the morpheme unit counts `%mor`-traced
/// morphemes, where these markers contribute nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreMarkers {
    markers: Vec<UntranscribedStatus>,
}

impl RestoreMarkers {
    /// Build a de-duplicated restore set from the parsed
    /// `--utterance-length-restore` values (insertion order preserved).
    pub fn from_statuses(statuses: &[UntranscribedStatus]) -> Self {
        let mut markers: Vec<UntranscribedStatus> = Vec::new();
        for &status in statuses {
            if !markers.contains(&status) {
                markers.push(status);
            }
        }
        Self { markers }
    }

    /// The restored markers, for the length walker's predicate.
    pub fn as_slice(&self) -> &[UntranscribedStatus] {
        &self.markers
    }

    /// Whether no marker is restored (the CLAN default).
    pub fn is_empty(&self) -> bool {
        self.markers.is_empty()
    }
}

/// Parse the marker token of a CLAN `+xxxx`/`+xyyy`/`+xwww` restore flag into
/// its typed status. The rewriter normalizes the flag to the canonical
/// `xxx`/`yyy`/`www` token (collapsing the `xx`/`yy`/`ww` aliases), so this only
/// sees those three; anything else is a hard error (fail-closed, no silent
/// no-op). Used as the `--utterance-length-restore` clap value parser.
pub fn parse_restore_marker(input: &str) -> Result<UntranscribedStatus, String> {
    match input {
        "xxx" => Ok(UntranscribedStatus::Unintelligible),
        "yyy" => Ok(UntranscribedStatus::Phonetic),
        "www" => Ok(UntranscribedStatus::Untranscribed),
        other => Err(format!(
            "unknown +x restore marker '{other}', expected one of xxx, yyy, www"
        )),
    }
}

impl UtteranceLengthFilter {
    /// Whether `utterance`'s length (in [`Self::unit`]) satisfies the
    /// comparison. The word and char units measure the main tier via chatter's
    /// AST word identity
    /// ([`countable_words_in_utterance`](crate::framework::word_filter::countable_words_in_utterance)),
    /// with any `-xS`
    /// words removed first; the morpheme unit counts §7.21-traced morphemes on
    /// the `%mor` tier (the shared MLU counter) and does not consult the
    /// main-line `-xS`/`+xS` list. The field guide's AST-vs-CLAN-char note
    /// applies: more precise on edge tokens.
    pub fn matches(&self, utterance: &Utterance) -> bool {
        let length = match self.unit {
            CountUnit::Word => self.length_words(utterance).count(),
            CountUnit::Char => self
                .length_words(utterance)
                .map(|word| word.cleaned_text().chars().count())
                .sum(),
            // No %mor tier means zero traced morphemes (the utterance is then
            // excluded by any positive `>`/`=` threshold), never a fallback to
            // word counting. The main-tier `+xS`/`-xS` tuning does not apply to
            // the morpheme measure (the markers carry no `%mor` morphemes).
            CountUnit::Morpheme => match count_traced_morphemes_in_utterance(utterance) {
                Some(n) => n as usize,
                None => 0,
            },
        };
        let threshold = self.threshold.0;
        match self.comparison {
            LengthComparison::GreaterThan => length > threshold,
            LengthComparison::LessThan => length < threshold,
            LengthComparison::Equal => length == threshold,
        }
    }

    /// The main-tier words contributing to the word/char length measure: the
    /// countable words PLUS any `restore`d unintelligible markers (`+xxxx`…),
    /// MINUS the `-xS` excluded words. The two content-specification lists are
    /// composed here: restore widens the counted set ([`words_for_utterance_length`]
    /// adds the markers during the walk), then `-xS` narrows it.
    fn length_words<'a>(&'a self, utterance: &'a Utterance) -> impl Iterator<Item = &'a Word> {
        words_for_utterance_length(utterance, self.restore.as_slice())
            .filter(move |word| !self.is_count_excluded(word))
    }

    /// Whether `word` is removed from the length count by a CLAN `-xS` pattern.
    /// Matched on the word's cleaned text via [`word_pattern_matches`] (CLAN's
    /// `patmat` wildcard semantics), the same matcher the `+s` word filter uses.
    fn is_count_excluded(&self, word: &talkbank_model::Word) -> bool {
        if self.exclude_from_count.is_empty() {
            return false;
        }
        let text = word.cleaned_text();
        self.exclude_from_count
            .iter()
            .any(|pattern| word_pattern_matches(text, pattern.0.as_str()))
    }
}

/// Error from parsing a CLAN `+x` utterance-length spec like `>3w`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseUtteranceLengthError {
    /// Not in `C N U` form (a comparison `>`/`<`/`=`, a number, and a unit).
    #[error(
        "invalid +x spec '{input}', expected a comparison (>, <, =), a number, and a unit, e.g. '>3w'"
    )]
    InvalidFormat {
        /// Original input string.
        input: String,
    },
    /// An unrecognized unit letter. The word (`w`), char (`c`), and morpheme
    /// (`m`) units are implemented; any other trailing letter is rejected.
    #[error(
        "unsupported +x unit '{unit}' in '{input}': the supported units are \
         'w' (word), 'c' (char), and 'm' (morpheme)"
    )]
    UnsupportedUnit {
        /// Original input string.
        input: String,
        /// The offending unit character.
        unit: char,
    },
}

/// Parse a CLAN `+x C N U` spec into an [`UtteranceLengthFilter`]. `C` is
/// `>`/`<`/`=`, `N` a non-negative integer, unit `U` one of `w` (word), `c`
/// (char), or `m` (morpheme). The `+xS` content-specification form (no leading
/// comparison) returns an error rather than silently no-op, so an unsupported
/// `+x` surfaces as a CLI error.
pub fn parse_utterance_length(
    input: &str,
) -> Result<UtteranceLengthFilter, ParseUtteranceLengthError> {
    let invalid = || ParseUtteranceLengthError::InvalidFormat {
        input: input.to_owned(),
    };
    let mut chars = input.chars();
    let comparison = match chars.next() {
        Some('>') => LengthComparison::GreaterThan,
        Some('<') => LengthComparison::LessThan,
        Some('=') => LengthComparison::Equal,
        // No leading comparison: the `+xS` content-specification form, deferred.
        _ => return Err(invalid()),
    };
    let rest = chars.as_str();
    let Some(unit_char) = rest.chars().next_back().filter(|c| c.is_ascii_alphabetic()) else {
        return Err(invalid());
    };
    let number = &rest[..rest.len() - unit_char.len_utf8()];
    let threshold: usize = number.parse().map_err(|_| invalid())?;
    let unit = match unit_char {
        'w' => CountUnit::Word,
        'c' => CountUnit::Char,
        'm' => CountUnit::Morpheme,
        // Any other trailing letter is an unknown unit; `+xS` never reaches here
        // (it has no leading comparison and fails the InvalidFormat check above).
        _ => {
            return Err(ParseUtteranceLengthError::UnsupportedUnit {
                input: input.to_owned(),
                unit: unit_char,
            });
        }
    };
    Ok(UtteranceLengthFilter {
        comparison,
        threshold: LengthThreshold(threshold),
        unit,
        // `-xS` / `+xS` are separate argv tokens, merged into the filter by the
        // CLI dispatch; the count form parses with empty exclude/restore sets.
        exclude_from_count: Vec::new(),
        restore: RestoreMarkers::default(),
    })
}
