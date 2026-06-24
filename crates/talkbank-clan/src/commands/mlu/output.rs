//! Typed MLU results and rendering logic.

use std::fmt;

use serde::Serialize;
use talkbank_model::model::content::word::UntranscribedStatus;

use crate::framework::{CommandOutput, MorphemeCount, UtteranceCount};

/// Typed output from the MLU command.
///
/// Contains per-speaker MLU statistics with strongly-typed numeric fields,
/// replacing the stringly-typed `AnalysisResult` for programmatic access.
#[derive(Debug, Clone, Serialize)]
pub struct MluResult {
    /// Per-speaker MLU statistics, in encounter order.
    pub speakers: Vec<MluSpeakerResult>,
    /// CLAN `+o3`: the single `speakers` entry is the pooled result, rendered
    /// `*COMBINED*` instead of `*<code>:`. Internal render flag, not serialized,
    /// so default-mode JSON is unchanged.
    #[serde(skip)]
    pub combine_speakers: bool,
    /// CLAN `+sxxx`/`+syyy` re-included untranscribed statuses, drives which of
    /// the three CLAN header variants (mlu.cpp:246-253) is printed. Internal
    /// render flag, not serialized.
    #[serde(skip)]
    pub re_included_untranscribed: Vec<UntranscribedStatus>,
}

impl MluResult {
    /// The CLAN exclusion-header line(s) under the `MLU for Speaker:` row.
    ///
    /// CLAN picks one of three forms (mlu.cpp:246-253), keyed only on whether
    /// `xxx`/`yyy` were re-included (`www` is never includable). Note CLAN has
    /// NO `yyy`-only branch: `+syyy` alone still prints the default header even
    /// though it re-includes the `yyy` utterances in the count. We reproduce
    /// that exactly (the binary is the `chatter clan` parity oracle); see the
    /// MLU audit's documented-divergence note on the `+syyy` manual conflict.
    fn exclusion_header(&self) -> &'static str {
        let xxx = self
            .re_included_untranscribed
            .contains(&UntranscribedStatus::Unintelligible);
        let yyy = self
            .re_included_untranscribed
            .contains(&UntranscribedStatus::Phonetic);
        match (xxx, yyy) {
            (true, true) => concat!(
                "  MLU (xxx and yyy are EXCLUDED from the morpheme counts, but are INCLUDED in utterance counts):\n",
                "  MLU (www is EXCLUDED from the utterance and morpheme counts):\n",
            ),
            (true, false) => concat!(
                "  MLU (xxx is EXCLUDED from the morpheme counts, but is INCLUDED in utterance counts):\n",
                "  MLU (yyy and www are EXCLUDED from the utterance and morpheme counts):\n",
            ),
            // `(false, true)` (yyy-only) falls here too: CLAN has no yyy-only
            // header branch, so it prints the default line.
            _ => "  MLU (xxx, yyy and www are EXCLUDED from the utterance and morpheme counts):\n",
        }
    }
}

/// MLU statistics for a single speaker.
#[derive(Debug, Clone, Serialize)]
pub struct MluSpeakerResult {
    /// Speaker code (e.g., "CHI", "MOT")
    pub speaker: String,
    /// Number of utterances included in the calculation
    pub utterances: UtteranceCount,
    /// Total morphemes (or words, if `--words` mode) across all utterances
    pub morphemes: MorphemeCount,
    /// Mean length of utterance (morphemes / utterances)
    pub mlu: f64,
    /// Population standard deviation of utterance lengths (/ n denominator).
    /// NAN when n=1 (rendered as "NA" in CLAN format).
    pub sd: f64,
    /// Minimum utterance length
    pub min: MorphemeCount,
    /// Maximum utterance length
    pub max: MorphemeCount,
}

impl CommandOutput for MluResult {
    /// Our clean text format.
    fn render_text(&self) -> String {
        let mut out = String::new();
        for (i, s) in self.speakers.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            fmt::write(
                &mut out,
                format_args!(
                    "Speaker: {}\n\
                     \x20 Utterances: {}\n\
                     \x20 Total morphemes: {}\n\
                     \x20 MLU: {:.3}\n\
                     \x20 SD: {:.3}\n\
                     \x20 Range: {}-{}\n",
                    s.speaker, s.utterances, s.morphemes, s.mlu, s.sd, s.min, s.max
                ),
            )
            .ok();
        }
        out
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// Format (from CLAN snapshot):
    /// ```text
    /// MLU for Speaker: *CHI:
    ///   MLU (xxx, yyy and www are EXCLUDED from the utterance and morpheme counts):
    /// \tNumber of: utterances = 2, morphemes = 3
    /// \tRatio of morphemes over utterances = 1.500
    /// \tStandard deviation = 0.500
    /// ```
    ///
    /// When utterances = 0, only header + counts are emitted (no Ratio/SD).
    /// When n = 1, SD is printed as "NA" (sample SD undefined).
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for (i, s) in self.speakers.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            // Header and counts are always present. CLAN `+o3` labels the pooled
            // result `*COMBINED*` (no `:`); per-speaker rows use `*<code>:`.
            let label = if self.combine_speakers {
                "*COMBINED*".to_owned()
            } else {
                format!("*{}:", s.speaker)
            };
            fmt::write(
                &mut out,
                format_args!(
                    "MLU for Speaker: {}\n{}\tNumber of: utterances = {}, morphemes = {}\n",
                    label,
                    self.exclusion_header(),
                    s.utterances,
                    s.morphemes
                ),
            )
            .ok();

            // When utterances = 0, CLAN omits Ratio and SD lines entirely
            if s.utterances > 0 {
                fmt::write(
                    &mut out,
                    format_args!("\tRatio of morphemes over utterances = {:.3}\n", s.mlu),
                )
                .ok();

                if s.sd.is_nan() {
                    // n=1: sample SD is undefined
                    fmt::write(&mut out, format_args!("\tStandard deviation = NA\n")).ok();
                } else {
                    fmt::write(
                        &mut out,
                        format_args!("\tStandard deviation = {:.3}\n", s.sd),
                    )
                    .ok();
                }
            }
        }
        // CLAN emits a trailing blank line after the last per-speaker
        // block; match that so a hex-level diff against legacy mlu
        // output ends cleanly.
        if !self.speakers.is_empty() {
            out.push('\n');
        }
        out
    }
}
