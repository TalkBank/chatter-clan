//! PHONFREQ, Phonological frequency analysis from `%pho` tier.
//!
//! Counts individual phone (character) occurrences from `%pho` tier
//! content, tracking positional distribution within each phonological
//! word: initial (first character), final (last character), and other
//! (middle positions). All alphabetic characters (including IPA) and
//! compound markers (`+`) are counted, matching CLAN's behavior.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409227)
//! for the original PHONFREQ command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                 | Rust equivalent                           |
//! |------------------------------|-------------------------------------------|
//! | `phonfreq file.cha`          | `chatter analyze phonfreq file.cha`       |
//! | `phonfreq +t*CHI file.cha`   | `chatter analyze phonfreq file.cha -s CHI`|
//!
//! # Output
//!
//! Per-phone frequency with positional breakdown (initial/final/other),
//! sorted alphabetically by phone character.
//!
//! # Differences from CLAN
//!
//! - Phone extraction uses parsed `%pho` tier structure from the AST
//!   rather than raw text character scanning.
//! - Positional classification operates on typed `PhoWord` content.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::{PhoItem, PhoWord, Utterance};

use crate::framework::{AnalysisCommand, FileContext};

pub use output::{PhonfreqEntry, PhonfreqResult};

/// Configuration for the PHONFREQ command.
#[derive(Debug, Clone, Default)]
pub struct PhonfreqConfig {}

/// Positional counts for a single phone (character).
#[derive(Debug, Default)]
struct PhoneCounts {
    /// Total occurrences
    total: u64,
    /// Occurrences as first character of a pho word
    initial: u64,
    /// Occurrences as last character of a pho word
    final_pos: u64,
    /// Occurrences in middle positions
    other: u64,
}

/// Accumulated state for PHONFREQ across all files.
#[derive(Debug, Default)]
pub struct PhonfreqState {
    /// Phone counts (BTreeMap for alphabetical ordering by phone character)
    counts: BTreeMap<char, PhoneCounts>,
}

/// PHONFREQ command: count phone frequencies from the `%pho` tier.
///
/// Iterates over `PhoItem`s (words and groups) on each utterance's
/// `%pho` tier, counting alphabetic characters (Unicode, including
/// IPA) plus the `+` compound marker, with positional tracking.
/// Stress marks (`ˈ`, `ˌ`), length marks (`ː`), digits, and other
/// non-letter symbols are skipped. Utterances without a `%pho`
/// tier are silently skipped.
pub struct PhonfreqCommand;

impl PhonfreqCommand {
    /// Create a new `PhonfreqCommand` with the given configuration.
    pub fn new(_config: PhonfreqConfig) -> Self {
        Self
    }
}

impl Default for PhonfreqCommand {
    /// Default command instance carries no runtime configuration.
    fn default() -> Self {
        Self
    }
}

impl AnalysisCommand for PhonfreqCommand {
    type Config = PhonfreqConfig;
    type State = PhonfreqState;
    type Output = PhonfreqResult;

    /// Count `%pho` character frequencies for one utterance when tier is present.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // Get %pho tier, skip utterances without one
        let pho_tier = match utterance.pho_tier() {
            Some(t) if t.is_pho() => t,
            _ => return,
        };

        for item in pho_tier.items.iter() {
            match item {
                PhoItem::Word(word) => {
                    count_pho_word(word, &mut state.counts);
                }
                PhoItem::Group(group) => {
                    for word in group.iter() {
                        count_pho_word(word, &mut state.counts);
                    }
                }
            }
        }
    }

    /// Convert accumulated phone maps into deterministic output rows.
    fn finalize(&self, state: Self::State) -> Self::Output {
        let mut entries: Vec<(char, PhonfreqEntry)> = state
            .counts
            .into_iter()
            .map(|(phone, counts)| {
                (
                    phone,
                    PhonfreqEntry {
                        phone: phone.to_string(),
                        total: counts.total,
                        initial: counts.initial,
                        final_pos: counts.final_pos,
                        other: counts.other,
                    },
                )
            })
            .collect();

        // CLAN's phonfreq groups phones by Unicode block before sorting
        // by codepoint within each group: Latin-1 Supplement (æ, ð) →
        // IPA Extensions (ɑ, ə, ɛ, ɪ) → Basic Latin (j, k, m, n, …).
        // Without an `alphabet.cut` file CLAN falls back to this
        // hardcoded bucket order; we match it byte-for-byte.
        entries.sort_by(|a, b| {
            clan_phone_bucket(a.0)
                .cmp(&clan_phone_bucket(b.0))
                .then_with(|| a.0.cmp(&b.0))
        });

        PhonfreqResult {
            entries: entries.into_iter().map(|(_, e)| e).collect(),
        }
    }
}

/// Assign a CLAN-style sort bucket to a phone character.
///
/// CLAN's `phonfreq` puts Latin-1 Supplement characters first, then
/// IPA Extensions, then Basic Latin. Anything outside those ranges
/// goes last in codepoint order. Returns a tuple-sortable bucket
/// index, lower wins.
fn clan_phone_bucket(c: char) -> u8 {
    let cp = c as u32;
    match cp {
        0x0080..=0x00FF => 0, // Latin-1 Supplement (æ, ð, …)
        0x0250..=0x02AF => 1, // IPA Extensions (ɑ, ə, ɛ, ɪ, …)
        0x0061..=0x007A => 2, // Basic Latin lowercase (j, k, m, …)
        _ => 3,               // anything else, sort by codepoint within
    }
}

/// Count each character in a phonological word, tracking position.
///
/// "Initial" = first character, "final" = last character, "other" = everything
/// in between. Single-character words count as both initial and final (matching
/// CLAN behavior where a single char has initial=1, final=0).
///
/// # Precondition
///
/// `word` should be a non-empty phonological transcription token.
fn count_pho_word(word: &PhoWord, counts: &mut BTreeMap<char, PhoneCounts>) {
    let text = word.as_str();
    if text.is_empty() {
        return;
    }

    // Count alphabetic characters (including IPA), plus `+` (compound
    // marker). Skip stress marks (ˈˌ), length marks (ː), digits, and
    // other non-letter symbols.
    let chars: Vec<char> = text
        .chars()
        .filter(|c| (c.is_alphabetic() || *c == '+') && !matches!(*c, 'ˈ' | 'ˌ' | 'ː'))
        .collect();
    let len = chars.len();
    if len == 0 {
        return;
    }

    for (i, &ch) in chars.iter().enumerate() {
        let entry = counts.entry(ch).or_default();
        entry.total += 1;

        if i == 0 {
            entry.initial += 1;
        } else if i == len - 1 {
            entry.final_pos += 1;
        } else {
            entry.other += 1;
        }
    }
}
