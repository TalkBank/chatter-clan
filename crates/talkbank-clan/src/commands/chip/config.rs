//! Validated CHIP configuration (CLAN `getflag` for `chip.cpp`).
//!
//! Every closed choice is an enum and every numeric option is a validated
//! newtype, so an invalid combination (`+g` with `-t%mor`, a window outside
//! 2-7, a speaker list beyond CLAN's capacity) is rejected when the
//! configuration is built and cannot reach the scoring code.

use crate::framework::{WordPattern, word_pattern_matches};

/// CLAN stores at most this many speaker codes per role (`chip.h`).
const MAX_SPEAKERS_PER_ROLE: usize = 10;
/// CLAN truncates speaker codes to this many characters.
const MAX_SPEAKER_CODE_CHARS: usize = 8;
/// Smallest and largest utterance window accepted by `+qN`.
const WINDOW_RANGE: std::ops::RangeInclusive<u8> = 2..=7;

/// Case handling for CHIP's word-stem comparisons.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChipCaseMode {
    /// Preserve case, matching CHIP's CLAN default.
    #[default]
    Preserve,
    /// Fold case, matching CLAN's `+k` toggle.
    Fold,
}

impl ChipCaseMode {
    /// Convert the effective preserve-case state into CHIP's comparison mode.
    pub const fn from_case_sensitive(case_sensitive: bool) -> Self {
        if case_sensitive {
            Self::Preserve
        } else {
            Self::Fold
        }
    }

    pub(super) fn comparison_key(self, value: &str) -> String {
        match self {
            Self::Preserve => value.to_owned(),
            Self::Fold => value.to_lowercase(),
        }
    }
}

/// Why a CHIP configuration was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ChipConfigError {
    /// A `+b`/`+c` speaker code had no characters after stripping `*`.
    #[error("speaker code is empty")]
    EmptySpeakerCode,
    /// More speaker codes than CLAN's fixed table holds.
    #[error("at most {MAX_SPEAKERS_PER_ROLE} {role} speaker codes are allowed")]
    TooManySpeakers {
        /// Role whose list overflowed.
        role: &'static str,
    },
    /// One speaker was assigned both roles.
    #[error("speaker {0:?} cannot be both an adult and a child")]
    ConflictingSpeakerRoles(String),
    /// `+qN` outside 2-7.
    #[error("Range of utterance window is 2-7.")]
    WindowOutOfRange(u8),
    /// `-nX` where X is not b, c or s.
    #[error("The only allowed letters are: b, c, or s")]
    BadScoringLetter(char),
    /// `+xN` was not a finite number.
    #[error("repetition index threshold must be a finite number")]
    BadMinIndex,
    /// `+g` and `-t%mor` together.
    #[error("The substitution option (+g) can't be used on main speaker tiers.")]
    SubstitutionOnMainTier,
    /// An exclusion-list entry uses syntax outside exact/wildcard word patterns.
    #[error(
        "unsupported CHIP -h pattern {0:?}; only single-word patterns with `*` wildcards are supported"
    )]
    UnsupportedExclusionPattern(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChipExclusionPattern(String);

impl TryFrom<WordPattern> for ChipExclusionPattern {
    type Error = ChipConfigError;

    fn try_from(pattern: WordPattern) -> Result<Self, Self::Error> {
        let pattern = pattern.0;
        let has_unsupported_syntax = pattern.is_empty()
            || pattern.chars().any(char::is_whitespace)
            || pattern
                .chars()
                .any(|character| matches!(character, '[' | ']' | '<' | '>' | '@' | '|'));
        if has_unsupported_syntax {
            return Err(ChipConfigError::UnsupportedExclusionPattern(pattern));
        }
        Ok(Self(pattern))
    }
}

#[derive(Debug, Clone, Default)]
struct ChipExclusionList(Vec<ChipExclusionPattern>);

impl ChipExclusionList {
    fn from_patterns(patterns: Vec<WordPattern>) -> Result<Self, ChipConfigError> {
        let patterns = patterns
            .into_iter()
            .map(ChipExclusionPattern::try_from)
            .collect::<Result<_, _>>()?;
        Ok(Self(patterns))
    }

    fn matches(&self, word: &str, case_mode: ChipCaseMode) -> bool {
        let word = case_mode.comparison_key(word);
        self.0.iter().any(|pattern| {
            let pattern = case_mode.comparison_key(&pattern.0);
            word_pattern_matches(&word, &pattern)
        })
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A speaker code as CLAN compares it: no leading `*`, upper case, at most
/// eight characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpeakerCode(String);

impl SpeakerCode {
    /// Normalize a `+b`/`+c` argument.
    pub fn parse(raw: &str) -> Result<Self, ChipConfigError> {
        let code: String = raw
            .strip_prefix('*')
            .unwrap_or(raw)
            .to_uppercase()
            .chars()
            .take(MAX_SPEAKER_CODE_CHARS)
            .collect();
        if code.is_empty() {
            return Err(ChipConfigError::EmptySpeakerCode);
        }
        Ok(Self(code))
    }

    /// The normalized code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Number of utterances CHIP keeps in view (`+qN`, 2-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtteranceWindow(u8);

impl UtteranceWindow {
    /// Validate a `+qN` value.
    pub fn new(n: u8) -> Result<Self, ChipConfigError> {
        if WINDOW_RANGE.contains(&n) {
            Ok(Self(n))
        } else {
            Err(ChipConfigError::WindowOutOfRange(n))
        }
    }

    /// How many earlier utterances can serve as a source (window minus the
    /// current utterance).
    pub fn previous_capacity(self) -> usize {
        usize::from(self.0) - 1
    }

    /// Total number of utterances retained, including the current one.
    pub fn capacity(self) -> usize {
        usize::from(self.0)
    }
}

impl Default for UtteranceWindow {
    fn default() -> Self {
        Self(*WINDOW_RANGE.end())
    }
}

/// Minimum repetition index for an interaction to count (`+xN`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MinIndex(f64);

impl MinIndex {
    /// Validate a `+xN` value.
    pub fn new(value: f64) -> Result<Self, ChipConfigError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(ChipConfigError::BadMinIndex)
        }
    }

    /// The threshold.
    pub fn get(self) -> f64 {
        self.0
    }
}

/// Minimum source-utterance length in words (`+wN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MinWords(pub usize);

impl MinWords {
    /// Minimum number of source words required for scoring.
    pub fn get(self) -> usize {
        self.0
    }
}

/// Whether a measure column is computed (`-nb`, `-nc`, `-ns` disable one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scoring {
    /// Column is computed and printed.
    Enabled,
    /// Column prints its own name instead of values.
    Disabled,
}

/// Scoring switches for the three column groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnScoring {
    /// ADU column (adult responses to child).
    pub adult: Scoring,
    /// CHI column (child responses to adult).
    pub child: Scoring,
    /// ASR and CSR columns (self-repetition).
    pub self_rep: Scoring,
}

impl Default for ColumnScoring {
    fn default() -> Self {
        Self {
            adult: Scoring::Enabled,
            child: Scoring::Enabled,
            self_rep: Scoring::Enabled,
        }
    }
}

/// Whether word-class substitution (`+g`) is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Substitution {
    /// Exact-stem matching only.
    Off,
    /// Same-POS words may substitute for one another.
    On,
}

/// Where the words being compared come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordSource {
    /// The `%mor` tier (CLAN default).
    Mor(Substitution),
    /// The main tier (`-t%mor`); substitution needs POS so it is not offered.
    MainTier,
}

impl Default for WordSource {
    fn default() -> Self {
        Self::Mor(Substitution::Off)
    }
}

impl WordSource {
    /// Substitution mode in effect.
    pub fn substitution(self) -> Substitution {
        match self {
            Self::Mor(s) => s,
            Self::MainTier => Substitution::Off,
        }
    }
}

/// Configuration for the CHIP command.
#[derive(Debug, Clone, Default)]
pub struct ChipConfig {
    adults: Vec<SpeakerCode>,
    children: Vec<SpeakerCode>,
    /// Column switches.
    pub scoring: ColumnScoring,
    /// Utterance window.
    pub window: UtteranceWindow,
    /// Minimum source length.
    pub min_words: MinWords,
    /// Minimum repetition index.
    pub min_index: MinIndex,
    /// Word source and substitution mode.
    pub source: WordSource,
    /// Case mode for word-stem comparisons.
    pub case_mode: ChipCaseMode,
    exclusion_list: ChipExclusionList,
}

impl ChipConfig {
    /// Select how word stems are compared.
    pub fn with_case_mode(mut self, case_mode: ChipCaseMode) -> Self {
        self.case_mode = case_mode;
        self
    }

    /// Exclude matched surface or `%mor` words from CHIP comparisons.
    pub fn with_exclusion_patterns(
        mut self,
        patterns: Vec<WordPattern>,
    ) -> Result<Self, ChipConfigError> {
        self.exclusion_list = ChipExclusionList::from_patterns(patterns)?;
        Ok(self)
    }

    pub(super) fn has_exclusion_patterns(&self) -> bool {
        !self.exclusion_list.is_empty()
    }

    pub(super) fn excludes_mor_pair(&self, surface: Option<&str>, stem: &str) -> bool {
        self.exclusion_list.matches(stem, self.case_mode)
            || surface.is_some_and(|surface| self.exclusion_list.matches(surface, self.case_mode))
    }

    /// Add an adult speaker (`+bS`).
    pub fn with_adult(mut self, code: SpeakerCode) -> Result<Self, ChipConfigError> {
        if self.children.contains(&code) {
            return Err(ChipConfigError::ConflictingSpeakerRoles(code.0));
        }
        if self.adults.len() >= MAX_SPEAKERS_PER_ROLE {
            return Err(ChipConfigError::TooManySpeakers { role: "adult" });
        }
        self.adults.push(code);
        Ok(self)
    }

    /// Add a child speaker (`+cS`).
    pub fn with_child(mut self, code: SpeakerCode) -> Result<Self, ChipConfigError> {
        if self.adults.contains(&code) {
            return Err(ChipConfigError::ConflictingSpeakerRoles(code.0));
        }
        if self.children.len() >= MAX_SPEAKERS_PER_ROLE {
            return Err(ChipConfigError::TooManySpeakers { role: "child" });
        }
        self.children.push(code);
        Ok(self)
    }

    /// Enable `+g` substitution; rejected when words come from the main tier.
    pub fn with_substitution(mut self) -> Result<Self, ChipConfigError> {
        match self.source {
            WordSource::Mor(_) => {
                self.source = WordSource::Mor(Substitution::On);
                Ok(self)
            }
            WordSource::MainTier => Err(ChipConfigError::SubstitutionOnMainTier),
        }
    }

    /// Compare main-tier words (`-t%mor`); rejected when `+g` is on.
    pub fn with_main_tier(mut self) -> Result<Self, ChipConfigError> {
        match self.source {
            WordSource::Mor(Substitution::On) => Err(ChipConfigError::SubstitutionOnMainTier),
            WordSource::Mor(Substitution::Off) | WordSource::MainTier => {
                self.source = WordSource::MainTier;
                Ok(self)
            }
        }
    }

    /// Disable one column group from a `-nX` letter (`b`, `c` or `s`).
    pub fn without_scoring(mut self, letter: char) -> Result<Self, ChipConfigError> {
        match letter.to_ascii_lowercase() {
            'b' => self.scoring.adult = Scoring::Disabled,
            'c' => self.scoring.child = Scoring::Disabled,
            's' => self.scoring.self_rep = Scoring::Disabled,
            _ => return Err(ChipConfigError::BadScoringLetter(letter)),
        }
        Ok(self)
    }

    /// Set the utterance window after validating CLAN's 2-7 limit.
    pub fn with_window(mut self, window: u8) -> Result<Self, ChipConfigError> {
        self.window = UtteranceWindow::new(window)?;
        Ok(self)
    }

    /// Set the minimum source-utterance word count.
    pub fn with_min_words(mut self, min_words: usize) -> Self {
        self.min_words = MinWords(min_words);
        self
    }

    /// Set the minimum repetition index after validating finiteness.
    pub fn with_min_index(mut self, min_index: f64) -> Result<Self, ChipConfigError> {
        self.min_index = MinIndex::new(min_index)?;
        Ok(self)
    }

    /// Adult speaker codes in the order given.
    pub fn adults(&self) -> &[SpeakerCode] {
        &self.adults
    }

    /// Child speaker codes in the order given.
    pub fn children(&self) -> &[SpeakerCode] {
        &self.children
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_exclusion_patterns_accept_words_and_wildcards_but_reject_selectors() {
        assert!(ChipExclusionPattern::try_from(WordPattern::from("go")).is_ok());
        assert!(ChipExclusionPattern::try_from(WordPattern::from("g*")).is_ok());
        assert!(matches!(
            ChipExclusionPattern::try_from(WordPattern::from("n|go")),
            Err(ChipConfigError::UnsupportedExclusionPattern(_))
        ));
        assert!(matches!(
            ChipExclusionPattern::try_from(WordPattern::from("")),
            Err(ChipConfigError::UnsupportedExclusionPattern(_))
        ));
    }
}
