//! FLUCALC, Fluency calculation (disfluency metrics).
//!
//! Detects and quantifies disfluencies in speech transcripts, producing
//! per-speaker counts of stuttering-like disfluencies (SLD) and typical
//! disfluencies (TD). FLUCALC is the standard tool in CLAN for analyzing
//! fluency in stuttering research.
//!
//! Disfluency categories detected:
//!
//! **Stuttering-Like Disfluencies (SLD):**
//! - Prolongations (`:` within a word, e.g., `wa:nt`)
//! - Broken words (`^` notation)
//! - Blocks (not yet fully implemented)
//! - Part-word repetitions (not yet fully implemented)
//! - Whole-word repetitions (consecutive identical words)
//!
//! **Typical Disfluencies (TD):**
//! - Phrase repetitions (`[/]`)
//! - Revisions (`[//]`)
//! - Filled pauses (`&-uh`, `&-um`, etc.)
//! - Phonological fragments (`&+` prefix)
//!
//! All counts are reported as raw values and as percentages per 100 words.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409273)
//! for the original FLUCALC command specification.
//!
//! # Differences from CLAN
//!
//! - Detection is based on recursive AST traversal rather than serialized
//!   CHAT text scanning.
//! - Part-word repetitions and blocks are counted via CHAT notation markers
//!   rather than acoustic analysis.

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::{
    BracketedItem, ContentAnnotation, RetraceKind, Utterance, UtteranceContent, WordCategory,
};

use crate::framework::{AnalysisCommand, FileContext};

pub use output::{FlucalcResult, SpeakerFluency};

/// Configuration for the FLUCALC command.
#[derive(Debug, Clone, Default)]
pub struct FlucalcConfig {
    /// Use syllable-based metrics instead of word-based.
    pub syllable_mode: bool,
}

/// Accumulated state for FLUCALC across all files.
#[derive(Debug, Default)]
pub struct FlucalcState {
    /// Per-speaker fluency data.
    speakers: BTreeMap<String, SpeakerFluency>,
}

/// FLUCALC command implementation.
///
/// Processes each utterance by serializing the main tier to CHAT text and
/// scanning for disfluency markers. Results are accumulated per speaker.
pub struct FlucalcCommand {
    _config: FlucalcConfig,
}

impl FlucalcCommand {
    /// Create a new FLUCALC command with the given config.
    pub fn new(config: FlucalcConfig) -> Self {
        Self { _config: config }
    }
}

/// Check if a word contains a prolongation marker (`:` used for stretching).
fn has_prolongation(word: &str) -> bool {
    // In CHAT, prolongation is marked by `:` within a word (not in speaker codes)
    word.contains(':') && !word.starts_with('*') && !word.starts_with('%')
}

/// Check if a word is a broken word (contains `^`).
fn has_broken_word(word: &str) -> bool {
    word.contains('^')
}

/// Count disfluencies in the main tier AST.
fn count_disfluencies(content: &[UtteranceContent], fluency: &mut SpeakerFluency) {
    let mut prev_word: Option<String> = None;
    count_disfluencies_content(content, fluency, &mut prev_word);
}

fn count_disfluencies_content(
    content: &[UtteranceContent],
    fluency: &mut SpeakerFluency,
    prev_word: &mut Option<String>,
) {
    for item in content {
        match item {
            UtteranceContent::Word(word) => {
                count_word(word.raw_text(), word.category.as_ref(), fluency, prev_word);
            }
            UtteranceContent::AnnotatedWord(annotated) => {
                count_scoped_annotations(&annotated.scoped_annotations, fluency);
                count_word(
                    annotated.inner.raw_text(),
                    annotated.inner.category.as_ref(),
                    fluency,
                    prev_word,
                );
            }
            UtteranceContent::ReplacedWord(replaced) => {
                count_scoped_annotations(&replaced.scoped_annotations, fluency);
                count_word(
                    replaced.word.raw_text(),
                    replaced.word.category.as_ref(),
                    fluency,
                    prev_word,
                );
            }
            UtteranceContent::Group(group) => {
                count_disfluencies_bracketed(&group.content.content, fluency, prev_word);
            }
            UtteranceContent::AnnotatedGroup(annotated) => {
                count_scoped_annotations(&annotated.scoped_annotations, fluency);
                count_disfluencies_bracketed(&annotated.inner.content.content, fluency, prev_word);
            }
            UtteranceContent::PhoGroup(group) => {
                count_disfluencies_bracketed(&group.content.content, fluency, prev_word);
            }
            UtteranceContent::SinGroup(group) => {
                count_disfluencies_bracketed(&group.content.content, fluency, prev_word);
            }
            UtteranceContent::Quotation(group) => {
                count_disfluencies_bracketed(&group.content.content, fluency, prev_word);
            }
            UtteranceContent::Retrace(retrace) => {
                count_retrace_kind(retrace.kind, fluency);
                count_disfluencies_bracketed(&retrace.content.content, fluency, prev_word);
            }
            _ => {}
        }
    }
}

fn count_disfluencies_bracketed(
    items: &[BracketedItem],
    fluency: &mut SpeakerFluency,
    prev_word: &mut Option<String>,
) {
    for item in items {
        match item {
            BracketedItem::Word(word) => {
                count_word(word.raw_text(), word.category.as_ref(), fluency, prev_word);
            }
            BracketedItem::AnnotatedWord(annotated) => {
                count_scoped_annotations(&annotated.scoped_annotations, fluency);
                count_word(
                    annotated.inner.raw_text(),
                    annotated.inner.category.as_ref(),
                    fluency,
                    prev_word,
                );
            }
            BracketedItem::ReplacedWord(replaced) => {
                count_scoped_annotations(&replaced.scoped_annotations, fluency);
                count_word(
                    replaced.word.raw_text(),
                    replaced.word.category.as_ref(),
                    fluency,
                    prev_word,
                );
            }
            BracketedItem::AnnotatedGroup(annotated) => {
                count_scoped_annotations(&annotated.scoped_annotations, fluency);
                count_disfluencies_bracketed(&annotated.inner.content.content, fluency, prev_word);
            }
            BracketedItem::PhoGroup(group) => {
                count_disfluencies_bracketed(&group.content.content, fluency, prev_word);
            }
            BracketedItem::SinGroup(group) => {
                count_disfluencies_bracketed(&group.content.content, fluency, prev_word);
            }
            BracketedItem::Quotation(group) => {
                count_disfluencies_bracketed(&group.content.content, fluency, prev_word);
            }
            BracketedItem::Retrace(retrace) => {
                count_retrace_kind(retrace.kind, fluency);
                count_disfluencies_bracketed(&retrace.content.content, fluency, prev_word);
            }
            _ => {}
        }
    }
}

/// Count retrace-type disfluencies from first-class Retrace content.
///
/// `[/]` (Partial) counts as a phrase repetition; `[//]` (Full) counts as
/// a revision. Other retrace kinds are not currently counted by FLUCALC.
fn count_retrace_kind(kind: RetraceKind, fluency: &mut SpeakerFluency) {
    match kind {
        RetraceKind::Partial => fluency.phrase_reps += 1,
        RetraceKind::Full => fluency.revisions += 1,
        RetraceKind::Multiple | RetraceKind::Reformulation => {}
    }
}

fn count_scoped_annotations(_annotations: &[ContentAnnotation], _fluency: &mut SpeakerFluency) {
    // Retrace markers are now Retrace content variants, not ContentAnnotation.
    // Retained for future non-retrace annotation counting if needed.
}

fn count_word(
    word: &str,
    category: Option<&WordCategory>,
    fluency: &mut SpeakerFluency,
    prev_word: &mut Option<String>,
) {
    match category {
        Some(WordCategory::Filler) => {
            fluency.filled_pauses += 1;
            return;
        }
        Some(WordCategory::PhonologicalFragment) => {
            fluency.phon_fragments += 1;
            return;
        }
        Some(WordCategory::Omission | WordCategory::CAOmission) => return,
        _ => {}
    }

    if has_prolongation(word) {
        fluency.prolongations += 1;
    }

    if has_broken_word(word) {
        fluency.broken_words += 1;
    }

    let current = normalize_repetition_word(word);
    if let Some(prev) = prev_word.as_ref()
        && !current.is_empty()
        && prev == &current
    {
        fluency.whole_word_reps += 1;
    }

    fluency.total_words += 1;
    *prev_word = Some(current);
}

fn normalize_repetition_word(word: &str) -> String {
    word.to_lowercase().chars().filter(|c| *c != ':').collect()
}

impl AnalysisCommand for FlucalcCommand {
    type Config = FlucalcConfig;
    type State = FlucalcState;
    type Output = FlucalcResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let speaker = utterance.main.speaker.to_string();
        let fluency = state
            .speakers
            .entry(speaker.clone())
            .or_insert_with(|| SpeakerFluency {
                speaker,
                ..Default::default()
            });

        fluency.utterances += 1;

        count_disfluencies(&utterance.main.content.content, fluency);
    }

    fn finalize(&self, state: Self::State) -> FlucalcResult {
        let speakers: Vec<SpeakerFluency> = state.speakers.into_values().collect();
        FlucalcResult { speakers }
    }
}
