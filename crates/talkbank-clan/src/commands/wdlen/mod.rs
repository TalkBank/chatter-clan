//! WDLEN -- Word Length Distribution (6-section CLAN format).
//!
//! Computes six distribution tables matching CLAN's output:
//! 1. Word lengths in characters
//! 2. Utterance lengths in words
//! 3. Turn lengths in utterances
//! 4. Turn lengths in words
//! 5. Word lengths in morphemes (requires %mor)
//! 6. Utterance lengths in morphemes (requires %mor)
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409247)
//! for the original WDLEN command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command               | Rust equivalent                         |
//! |----------------------------|-----------------------------------------|
//! | `wdlen file.cha`           | `chatter analyze wdlen file.cha`        |
//! | `wdlen +t*CHI file.cha`    | `chatter analyze wdlen file.cha -s CHI` |
//!
//! # Differences from CLAN
//!
//! - **Brown's morpheme rules**: Section 5 = stem + Brown's suffix (no POS).
//!   Section 6 = POS + stem + Brown's suffix. Brown's suffixes are the same 7
//!   strings as MLU: `PL`, `PAST`, `Past`, `POSS`, `PASTP`, `Pastp`, `PRESP`.
//! - **Clitic handling**: Section 5 merges main+clitics as one word. Section 6
//!   counts POS only for main word.
//! - **Apostrophe stripping**: Characters counted after removing apostrophes,
//!   matching CLAN.
//! - **Reverse speaker order**: CLAN's linked-list prepend pattern replicated.
//! - **XML footer**: `</Table></Worksheet></Workbook>` appended to match CLAN.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use indexmap::IndexMap;
use talkbank_model::{SpeakerCode, Utterance};

use crate::framework::word_filter::{countable_words, has_countable_words};
use crate::framework::{AnalysisCommand, FileContext};

pub use output::{WdlenDistribution, WdlenResult};

/// Configuration for the WDLEN command.
#[derive(Debug, Clone, Default)]
pub struct WdlenConfig {}

/// Per-speaker accumulator for a single distribution dimension.
#[derive(Debug, Default)]
struct DistAccum {
    distribution: BTreeMap<usize, u64>,
    total_items: u64,
    total_value: u64,
}

impl DistAccum {
    fn record(&mut self, value: usize) {
        *self.distribution.entry(value).or_insert(0) += 1;
        self.total_items += 1;
        self.total_value += value as u64;
    }

    fn into_distribution(self, speaker: &str) -> WdlenDistribution {
        WdlenDistribution {
            speaker: speaker.to_owned(),
            distribution: self.distribution,
            total_items: self.total_items,
            total_value: self.total_value,
        }
    }
}

/// Per-speaker data tracking the current turn.
#[derive(Debug, Default)]
struct SpeakerAccum {
    /// Section 1: word char lengths.
    word_lengths: DistAccum,
    /// Section 2: utterance word counts.
    utt_word_counts: DistAccum,
    /// Section 5: per-word morpheme lengths.
    morph_lengths: DistAccum,
    /// Section 6: utterance morpheme counts.
    utt_morph_counts: DistAccum,
    /// Current turn: utterance count.
    current_turn_utts: u64,
    /// Current turn: word count.
    current_turn_words: u64,
    /// Turn utterance counts (section 3).
    turn_utt_counts: DistAccum,
    /// Turn word counts (section 4).
    turn_word_counts: DistAccum,
}

impl SpeakerAccum {
    /// Close the current turn and record its stats.
    fn close_turn(&mut self) {
        if self.current_turn_utts > 0 {
            self.turn_utt_counts.record(self.current_turn_utts as usize);
            self.turn_word_counts
                .record(self.current_turn_words as usize);
            self.current_turn_utts = 0;
            self.current_turn_words = 0;
        }
    }
}

/// Accumulated state for WDLEN across all files.
#[derive(Debug, Default)]
pub struct WdlenState {
    by_speaker: IndexMap<SpeakerCode, SpeakerAccum>,
    last_speaker: Option<SpeakerCode>,
}

/// WDLEN command implementation.
#[derive(Debug, Clone, Default)]
pub struct WdlenCommand;

/// Brown's (1973) counted suffixes, same as MLU.
const COUNTED_SUFFIXES: &[&str] = &["PL", "PAST", "Past", "POSS", "PASTP", "Pastp", "PRESP"];

/// Check if a MorWord has any Brown's counted suffix.
fn has_counted_suffix(word: &talkbank_model::MorWord) -> bool {
    word.features
        .iter()
        .any(|f| COUNTED_SUFFIXES.contains(&f.value()))
}

/// Count morphemes per word for section 5 (Brown's rules: stem + counted suffix).
fn word_morpheme_count(word: &talkbank_model::MorWord) -> u64 {
    1 + if has_counted_suffix(word) { 1 } else { 0 }
}

/// Count morphemes per word for section 6 (POS + stem + counted suffix).
///
/// CLAN includes the POS tag as a morpheme in per-utterance totals.
fn word_morpheme_count_with_pos(word: &talkbank_model::MorWord) -> u64 {
    2 + if has_counted_suffix(word) { 1 } else { 0 }
}

impl AnalysisCommand for WdlenCommand {
    type Config = WdlenConfig;
    type State = WdlenState;
    type Output = WdlenResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        if !has_countable_words(&utterance.main.content.content) {
            return;
        }

        let speaker = utterance.main.speaker.clone();

        // Detect turn boundary: close previous speaker's turn on speaker change.
        if state.last_speaker.as_ref() != Some(&speaker) {
            if let Some(ref prev) = state.last_speaker
                && let Some(prev_data) = state.by_speaker.get_mut(prev)
            {
                prev_data.close_turn();
            }
            state.last_speaker = Some(speaker.clone());
        }

        let data = state
            .by_speaker
            .entry(speaker)
            .or_insert_with(SpeakerAccum::default);

        // Section 1: per-word character lengths (CLAN strips apostrophes).
        let mut word_count: u64 = 0;
        for word in countable_words(&utterance.main.content.content) {
            let char_len = word.cleaned_text().chars().filter(|&c| c != '\'').count();
            data.word_lengths.record(char_len);
            word_count += 1;
        }

        // Section 2: utterance word count.
        data.utt_word_counts.record(word_count as usize);

        // Turn tracking (sections 3 & 4).
        data.current_turn_utts += 1;
        data.current_turn_words += word_count;

        // Sections 5 & 6: morpheme counts (only if %mor tier present).
        // CLAN treats clitic pairs (main~clitic) as single words for section 5.
        // Section 5: per-word = stem + Brown's suffix, clitics merged into one word.
        // Section 6: per-utterance = POS(main only) + stems + Brown's suffixes.
        // CLAN excludes punctuation-class `%mor` items (`cm`, `punct`,
        // `end`, `beg`) from morpheme counts, see
        // `MorWord::is_punctuation_marker`. They remain real `%gra`
        // chunks; just not counted for word/morpheme metrics.
        if let Some(mor_tier) = utterance.mor_tier() {
            let mut utt_morphemes: u64 = 0;
            for mor_item in mor_tier.items().iter() {
                if mor_item.main.is_punctuation_marker() {
                    continue;
                }
                // Section 5: entire Mor item (main + clitics) = one word entry.
                let mut word_morphs = word_morpheme_count(&mor_item.main);
                for clitic in &mor_item.post_clitics {
                    word_morphs += word_morpheme_count(clitic);
                }
                data.morph_lengths.record(word_morphs as usize);

                // Section 6: POS counted only for main word, not clitics.
                utt_morphemes += word_morpheme_count_with_pos(&mor_item.main);
                for clitic in &mor_item.post_clitics {
                    // Clitic: stem + Brown's suffix, no POS.
                    utt_morphemes += word_morpheme_count(clitic);
                }
            }
            data.utt_morph_counts.record(utt_morphemes as usize);
        }
    }

    /// Close open turns at file boundary.
    fn end_file(&self, _file_context: &FileContext<'_>, state: &mut Self::State) {
        for data in state.by_speaker.values_mut() {
            data.close_turn();
        }
        state.last_speaker = None;
    }

    fn finalize(&self, state: Self::State) -> WdlenResult {
        let mut word_lengths = Vec::new();
        let mut utt_word_lengths = Vec::new();
        let mut turn_utt_lengths = Vec::new();
        let mut turn_word_lengths = Vec::new();
        let mut morph_lengths = Vec::new();
        let mut utt_morph_lengths = Vec::new();

        for (speaker, data) in state.by_speaker {
            let name = speaker.as_str();
            word_lengths.push(data.word_lengths.into_distribution(name));
            utt_word_lengths.push(data.utt_word_counts.into_distribution(name));
            turn_utt_lengths.push(data.turn_utt_counts.into_distribution(name));
            turn_word_lengths.push(data.turn_word_counts.into_distribution(name));
            morph_lengths.push(data.morph_lengths.into_distribution(name));
            utt_morph_lengths.push(data.utt_morph_counts.into_distribution(name));
        }

        WdlenResult {
            word_lengths,
            utt_word_lengths,
            turn_utt_lengths,
            turn_word_lengths,
            morph_lengths,
            utt_morph_lengths,
        }
    }
}
