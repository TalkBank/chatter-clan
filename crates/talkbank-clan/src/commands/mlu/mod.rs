//! MLU, Mean Length of Utterance.
//!
//! Calculates mean length of utterance in morphemes from the `%mor` tier.
//! When no `%mor` tier is available and not in `words_only` mode, reports
//! "utterances = 0, morphemes = 0" (matching CLAN behavior, no fallback
//! to word counting).
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409094)
//! for the original MLU command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command              | Rust equivalent                        |
//! |---------------------------|----------------------------------------|
//! | `mlu file.cha`            | `chatter analyze mlu file.cha`         |
//! | `mlu +t*CHI file.cha`     | `chatter analyze mlu file.cha -s CHI`  |
//!
//! # MLU Calculation
//!
//! For each utterance:
//! 1. Count morphemes on the `%mor` tier: 1 per stem + 1 per `-` suffix
//!    (bound morpheme) + 1 per `~` clitic stem + 1 per clitic `-` suffix.
//!    Fusional features (`&`) do NOT count.
//! 2. If no `%mor` tier, skip the utterance (report 0 utterances for the speaker)
//!
//! Per speaker, compute:
//! - Number of utterances
//! - Total morphemes
//! - MLU (mean)
//! - Standard deviation (sample, n-1)
//! - Range (min, max)
//!
//! # Differences from CLAN
//!
//! - Word identification uses AST-based `is_countable_word()` instead of
//!   CLAN's string-prefix matching (`word[0] == '&'`, etc.).
//! - Morpheme counting uses parsed `%mor` tier structure (MorWord features
//!   and post-clitics) rather than text splitting on spaces and delimiters.
//! - Output supports text, JSON, and CSV formats (CLAN produces text only).
//! - Deterministic output ordering via sorted collections.

mod output;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use indexmap::IndexMap;
use talkbank_model::{IDHeader, SpeakerCode, Utterance};

use talkbank_model::model::content::word::UntranscribedStatus;

use crate::framework::word_filter::{
    countable_words_in_utterance, has_countable_words, main_tier_has_excluded_untranscribed,
    utterance_is_solo_excluded,
};
use crate::framework::{AnalysisCommand, FileContext, MorphemeCount, population_sd};

pub use output::{MluCountUnit, MluIdRow, MluResult, MluSpeakerResult};

/// CLAN MLU output shape.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MluOutputMode {
    /// Normal per-speaker summary output.
    #[default]
    Summary,
    /// CLAN `+d1`, one `@ID`-expanded row per speaker and input file.
    IdRowsPerFile,
    /// CLAN `+d1 +u`, one `@ID`-expanded row per speaker across all files.
    IdRowsCombinedFiles,
    /// CLAN `+d`, one SpreadsheetML workbook with a row per file and speaker.
    Spreadsheet,
}

impl MluOutputMode {
    const fn uses_id_rows(self) -> bool {
        !matches!(self, Self::Summary)
    }
}

/// Configuration for the MLU command.
#[derive(Debug, Clone, Default)]
pub struct MluConfig {
    /// Selects normal summary, CLAN `+d1` ID rows, or CLAN `+d` SpreadsheetML.
    pub output_mode: MluOutputMode,
    /// Use word count from main tier instead of morpheme count from %mor
    pub words_only: bool,
    /// Words that, when an utterance consists *solely* of them, cause
    /// the whole utterance to be excluded from the MLU count.
    /// Maps CLAN's command-specific `+gS` (e.g. `mlu +gum`) which is
    /// distinct from the inherited general `+gX` gem-segment filter.
    /// Comparison is by lower-cased word text after `NormalizedWord`
    /// normalization (same form chatter uses for countable-word
    /// iteration).
    pub solo_word_exclusions: Vec<String>,
    /// CLAN `+o3` (`mlu_isCombineSpeakers`, mlu.cpp:721): pool every selected
    /// speaker's utterances into one `*COMBINED*` MLU result instead of a
    /// per-speaker breakdown. Default `false` keeps the per-speaker layout.
    pub combine_speakers: bool,
    /// CLAN `+sxxx`/`+syyy`: untranscribed statuses re-admitted to the utterance
    /// count. By default MLU excludes any utterance containing `xxx`/`yyy`/`www`
    /// (manual §7.21 pt2); `+sxxx` re-includes the `xxx` utterances
    /// (`Unintelligible`) and `+syyy` the `yyy` utterances (`Phonetic`), with the
    /// marker still kept out of the morpheme count. `www` (`Untranscribed`) is
    /// never re-includable, so it never appears here. Empty = the default.
    pub re_included_untranscribed: Vec<UntranscribedStatus>,
}

/// Per-speaker MLU data accumulated during processing.
#[derive(Debug, Default)]
struct SpeakerMlu {
    /// Morpheme (or word) counts per utterance
    utterance_lengths: Vec<MorphemeCount>,
    /// First parsed `@ID` header for this speaker in the input file.
    id_header: Option<IDHeader>,
}

/// Accumulated state for MLU across all files.
#[derive(Debug, Default)]
pub struct MluState {
    /// Per-speaker MLU data, keyed by speaker code
    by_speaker: IndexMap<SpeakerCode, SpeakerMlu>,
    /// Per-file speaker data needed only by CLAN `+d1`.
    by_file_speaker: IndexMap<(PathBuf, SpeakerCode), SpeakerMlu>,
    /// Input order is significant for CLAN `+d1 +u`'s filename column.
    processed_files: Vec<PathBuf>,
}

/// MLU command implementation.
///
/// Counts morphemes per utterance from the %mor tier (or words from
/// the main tier), computing mean, SD, and range per speaker.
#[derive(Debug, Clone, Default)]
pub struct MluCommand {
    config: MluConfig,
    /// `config.solo_word_exclusions` lower-cased once at construction so
    /// the per-utterance hot path in [`utterance_is_solo_excluded`] does
    /// not re-allocate. Empty when the user did not pass
    /// `--exclude-solo-word`.
    solo_words_normalized: Vec<String>,
}

impl MluCommand {
    /// Create an MLU command with the given configuration.
    pub fn new(config: MluConfig) -> Self {
        let solo_words_normalized = config
            .solo_word_exclusions
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        Self {
            config,
            solo_words_normalized,
        }
    }
}

impl AnalysisCommand for MluCommand {
    type Config = MluConfig;
    type State = MluState;
    type Output = MluResult;

    /// Record one utterance length for the current speaker when lexical material exists.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        // Skip utterances with no countable lexical content (e.g., "xxx .")
        // These would deflate MLU by adding zero-morpheme utterances to the
        // denominator. CLAN achieves this by string-prefix exclusion; we use
        // the AST's semantic word classification instead.
        if !has_countable_words(&utterance.main.content.content) {
            return;
        }

        // CLAN MLU excludes the WHOLE utterance when its main tier carries a
        // standalone `xxx`/`yyy`/`www` token, by default (manual §7.21 pt2;
        // `mlu_excludeUtter`, mllib.cpp:303-348, invoked on the main tier at
        // mlu.cpp:509). This is distinct from the no-countable-words skip above:
        // `*CHI: it xxx xxx` HAS a countable word (`it`) yet must be dropped
        // entirely. `+sxxx`/`+syyy` re-admit the xxx/yyy utterances via
        // `re_included_untranscribed` (the marker still stays out of the
        // `%mor`-based morpheme count); `www` can never be re-included.
        if main_tier_has_excluded_untranscribed(utterance, &self.config.re_included_untranscribed) {
            return;
        }

        // CLAN MLU excludes any utterance carrying the `[+ mlue]` postcode by
        // default (`isMLUEpostcode` defaults TRUE, mlu.cpp:108; the per-utterance
        // `isPostCodeOnUtt(line, "[+ mlue]")` -> `isSkip = TRUE`, mlu.cpp:503).
        // It is the explicit "exclude this utterance from MLU" researcher tag
        // (CHAT postcodes); chatter stores the payload (`mlue`) on
        // `MainTier::content.postcodes`. (CLAN's `+s"[+ mlue]"` can flip
        // `isMLUEpostcode` off to FORCE inclusion, manual §7.21 pt5; that
        // override is not yet surfaced, the default always excludes.)
        if utterance
            .main
            .content
            .postcodes
            .iter()
            .any(|postcode| postcode.text() == Some(MLUE_EXCLUDE_POSTCODE))
        {
            return;
        }

        // CLAN's `mlu +gS` (filler-word elision) drops an utterance when
        // every countable word is in the user's solo-word list. Empty
        // list ⇒ no-op (fast path inside the helper).
        if utterance_is_solo_excluded(utterance, &self.solo_words_normalized) {
            return;
        }

        // Arc<str> clone, cheap atomic ref-count increment, no allocation
        let speaker = utterance.main.speaker.clone();

        let count = if self.config.words_only {
            Some(count_words_in_utterance(utterance))
        } else {
            crate::framework::count_traced_morphemes_in_utterance(utterance)
        };

        let file_speaker = self.config.output_mode.uses_id_rows().then(|| {
            state
                .by_file_speaker
                .entry((
                    file_context.path.to_path_buf(),
                    utterance.main.speaker.clone(),
                ))
                .or_default()
        });

        // Always register the speaker so they appear in output, even when
        // no %mor tier is available (CLAN parity: shows "utterances = 0").
        let speaker_mlu = state
            .by_speaker
            .entry(speaker)
            .or_insert_with(SpeakerMlu::default);

        // When no %mor tier exists and not in words_only mode, CLAN reports
        // "utterances = 0, morphemes = 0"; it does NOT fall back to counting
        // words. We skip adding the utterance length but still register the
        // speaker above.
        if let Some(count) = count {
            speaker_mlu.utterance_lengths.push(count);
            if let Some(file_speaker) = file_speaker {
                file_speaker.utterance_lengths.push(count);
            }
        }
    }

    /// Capture the first `@ID` header for each processed speaker in this file.
    fn end_file(&self, file_context: &FileContext<'_>, state: &mut Self::State) {
        if !self.config.output_mode.uses_id_rows() {
            return;
        }

        let path = file_context.path.to_path_buf();
        state.processed_files.push(path.clone());

        let mut id_by_speaker = IndexMap::<SpeakerCode, IDHeader>::new();
        for id in file_context.chat_file.id_headers() {
            id_by_speaker
                .entry(id.speaker.clone())
                .or_insert_with(|| id.clone());
        }

        for ((entry_path, speaker), data) in &mut state.by_file_speaker {
            if entry_path == &path
                && let Some(id) = id_by_speaker.get(speaker)
            {
                data.id_header = Some(id.clone());
            }
        }
    }

    /// Compute per-speaker MLU aggregates (mean, SD, min, max) from collected
    /// lengths, or a single pooled `*COMBINED*` aggregate under CLAN `+o3`.
    fn finalize(&self, state: Self::State) -> MluResult {
        let speakers = if self.config.combine_speakers {
            // CLAN `+o3`: pool every selected speaker's utterance lengths into
            // one combined result. The label is rendered as `*COMBINED*`
            // (output.rs), so the speaker code here is just a placeholder.
            let pooled: Vec<MorphemeCount> = state
                .by_speaker
                .values()
                .flat_map(|m| m.utterance_lengths.iter().copied())
                .collect();
            vec![aggregate_mlu(COMBINED_SPEAKER_LABEL, &pooled)]
        } else {
            state
                .by_speaker
                .iter()
                .map(|(speaker, mlu_data)| {
                    aggregate_mlu(speaker.as_str(), &mlu_data.utterance_lengths)
                })
                .collect()
        };
        let id_rows = build_id_rows(
            state.by_file_speaker,
            &state.processed_files,
            self.config.output_mode,
            self.config.combine_speakers,
        );
        MluResult {
            speakers,
            combine_speakers: self.config.combine_speakers,
            re_included_untranscribed: self.config.re_included_untranscribed.clone(),
            id_rows,
        }
    }
}

#[derive(Debug)]
struct IdRowSpeaker {
    speaker: SpeakerCode,
    id_header: Option<IDHeader>,
    utterance_lengths: Vec<MorphemeCount>,
}

#[derive(Debug)]
struct IdRowScope {
    file: PathBuf,
    speakers: Vec<IdRowSpeaker>,
}

fn build_id_rows(
    by_file_speaker: IndexMap<(PathBuf, SpeakerCode), SpeakerMlu>,
    processed_files: &[PathBuf],
    output_mode: MluOutputMode,
    combine_speakers: bool,
) -> Option<Vec<MluIdRow>> {
    let scopes = match output_mode {
        MluOutputMode::Summary => return None,
        MluOutputMode::IdRowsPerFile | MluOutputMode::Spreadsheet => {
            let mut by_file = IndexMap::<PathBuf, IndexMap<SpeakerCode, IdRowSpeaker>>::new();
            for ((file, speaker), data) in by_file_speaker {
                let row = by_file
                    .entry(file)
                    .or_default()
                    .entry(speaker.clone())
                    .or_insert_with(|| IdRowSpeaker {
                        speaker,
                        id_header: data.id_header.clone(),
                        utterance_lengths: Vec::new(),
                    });
                if row.id_header.is_none() {
                    row.id_header = data.id_header;
                }
                row.utterance_lengths.extend(data.utterance_lengths);
            }
            by_file
                .into_iter()
                .map(|(file, speakers)| IdRowScope {
                    file,
                    speakers: speakers.into_values().collect(),
                })
                .collect()
        }
        MluOutputMode::IdRowsCombinedFiles => {
            let file = processed_files
                .first()
                .cloned()
                .or_else(|| by_file_speaker.keys().next().map(|(file, _)| file.clone()));
            let Some(file) = file else {
                return Some(Vec::new());
            };

            let mut speakers = IndexMap::<SpeakerCode, IdRowSpeaker>::new();
            for ((_, speaker), data) in by_file_speaker {
                let row = speakers
                    .entry(speaker.clone())
                    .or_insert_with(|| IdRowSpeaker {
                        speaker,
                        id_header: data.id_header.clone(),
                        utterance_lengths: Vec::new(),
                    });
                if row.id_header.is_none() {
                    row.id_header = data.id_header;
                }
                row.utterance_lengths.extend(data.utterance_lengths);
            }
            vec![IdRowScope {
                file,
                speakers: speakers.into_values().collect(),
            }]
        }
    };

    let mut rows = Vec::new();
    for scope in scopes {
        for speaker in &scope.speakers {
            let result = if combine_speakers {
                aggregate_mlu(speaker.speaker.as_str(), &[])
            } else {
                aggregate_mlu(speaker.speaker.as_str(), &speaker.utterance_lengths)
            };
            rows.push(MluIdRow::Speaker {
                file: scope.file.clone(),
                id_header: speaker.id_header.clone().map(Box::new),
                result,
            });
        }

        if combine_speakers {
            let combined_lengths: Vec<MorphemeCount> = scope
                .speakers
                .iter()
                .flat_map(|speaker| speaker.utterance_lengths.iter().copied())
                .collect();
            rows.push(MluIdRow::Combined {
                file: scope.file,
                result: aggregate_mlu(COMBINED_SPEAKER_LABEL, &combined_lengths),
            });
        }
    }
    Some(rows)
}

/// Placeholder speaker code for the `+o3` pooled result; the CLAN renderer shows
/// it as `*COMBINED*` (gated on `MluResult::combine_speakers`).
const COMBINED_SPEAKER_LABEL: &str = "COMBINED";

/// The CHAT postcode payload (`[+ mlue]`, stored as `mlue`) that marks an
/// utterance for exclusion from MLU. CLAN matches the literal `[+ mlue]`
/// (mlu.cpp:503); chatter compares the parsed postcode text.
const MLUE_EXCLUDE_POSTCODE: &str = "mlue";

/// Build the [`MluConfig::re_included_untranscribed`] set from the two CLAN
/// `+sxxx`/`+syyy` re-include flags. `www` (`Untranscribed`) is never
/// includable, so it never appears. Centralizes the flag-to-status mapping so
/// the service builder and the golden-test harness stay in lock-step (one place
/// owns "xxx -> Unintelligible, yyy -> Phonetic").
pub fn re_included_untranscribed(include_xxx: bool, include_yyy: bool) -> Vec<UntranscribedStatus> {
    [
        (include_xxx, UntranscribedStatus::Unintelligible),
        (include_yyy, UntranscribedStatus::Phonetic),
    ]
    .into_iter()
    .filter_map(|(flag, status)| flag.then_some(status))
    .collect()
}

/// Build one [`MluSpeakerResult`] from a speaker's (or the pooled) per-utterance
/// lengths: mean, population SD (`/ n`, NaN when `n == 1` -> CLAN "NA"), min/max.
/// `n == 0` (a registered speaker with no `%mor`) yields all-zero counts and no
/// Ratio/SD lines, matching CLAN.
fn aggregate_mlu(speaker: &str, lengths: &[MorphemeCount]) -> MluSpeakerResult {
    let n = lengths.len() as u64;
    if n == 0 {
        return MluSpeakerResult {
            speaker: speaker.to_owned(),
            utterances: 0,
            morphemes: 0,
            mlu: 0.0,
            sd: 0.0,
            min: 0,
            max: 0,
        };
    }

    let total: u64 = lengths.iter().sum();
    let mean = total as f64 / n as f64;
    MluSpeakerResult {
        speaker: speaker.to_owned(),
        utterances: n,
        morphemes: total,
        mlu: mean,
        // Population SD (/ n), NaN at n=1 -> CLAN "NA"; shared with MLT.
        sd: population_sd(lengths),
        min: lengths.iter().copied().min().unwrap_or(0),
        max: lengths.iter().copied().max().unwrap_or(0),
    }
}

/// Count words in an utterance from the main tier (fallback when no %mor).
///
/// Uses the shared [`countable_words_in_utterance`] iterator to avoid
/// duplicating the tree-walking logic.
fn count_words_in_utterance(utterance: &Utterance) -> u64 {
    countable_words_in_utterance(utterance).count() as u64
}
