//! Typed VOCD results and rendering logic.

use std::fmt::Write;

use serde::Serialize;

use crate::framework::{AnalysisScore, CommandOutput, TypeCount, WordCount};

/// A single (N, TTR) data point with statistics.
#[derive(Debug, Clone, Serialize)]
pub struct NtEntry {
    /// Sample size (number of tokens drawn).
    pub n: usize,
    /// Number of samples drawn at this N.
    pub samples: usize,
    /// Mean TTR across all samples.
    pub mean_ttr: f64,
    /// Standard deviation of TTR across samples.
    pub std_dev: f64,
    /// D value computed from this single (N, TTR) pair via the inverse equation.
    pub d_value: f64,
}

/// Results from a single VOCD trial.
#[derive(Debug, Clone, Serialize)]
pub struct VocdTrial {
    /// Per-N sampling results.
    pub entries: Vec<NtEntry>,
    /// Average D across all N values in this trial.
    pub d_average: f64,
    /// Standard deviation of D values across N values.
    pub d_std_dev: f64,
    /// Optimal D found by least-squares curve fitting.
    pub d_optimum: f64,
    /// Minimum least-squares error at d_optimum.
    pub min_least_sq: f64,
}

/// Per-speaker VOCD result.
#[derive(Debug, Clone, Serialize)]
pub struct VocdSpeakerResult {
    /// Speaker code.
    pub speaker: String,
    /// Total unique word types.
    pub types: TypeCount,
    /// Total word tokens.
    pub tokens: WordCount,
    /// Overall TTR (types/tokens).
    pub ttr: AnalysisScore,
    /// Individual trial results.
    pub trials: Vec<VocdTrial>,
    /// D_optimum values from each trial.
    pub d_optimum_values: Vec<f64>,
    /// Final averaged D_optimum across all trials.
    pub d_optimum_average: AnalysisScore,
}

/// Warning for speakers with insufficient tokens.
#[derive(Debug, Clone, Serialize)]
pub struct VocdWarning {
    /// Speaker code.
    pub speaker: String,
    /// Number of tokens available.
    pub token_count: WordCount,
    /// Minimum required.
    pub minimum_required: WordCount,
    /// Per-utterance %mor lemma strings (CLAN echoes these for low-token speakers).
    pub mor_lemma_lines: Vec<String>,
}

/// Typed output from the VOCD command.
#[derive(Debug, Clone, Serialize)]
pub struct VocdResult {
    /// Per-speaker VOCD results (only for speakers with enough tokens).
    pub speakers: Vec<VocdSpeakerResult>,
    /// Warnings for speakers without enough tokens.
    pub warnings: Vec<VocdWarning>,
}

impl CommandOutput for VocdResult {
    /// Render warnings and per-trial VOCD tables in CLAN-compatible text.
    fn render_text(&self) -> String {
        let mut out = String::new();

        for warning in &self.warnings {
            writeln!(out, "****** Speaker: *{}:", warning.speaker).ok();
            writeln!(
                out,
                "WARNING: Not enough tokens for random sampling without replacement."
            )
            .ok();
            writeln!(
                out,
                "  ({} tokens available, {} required)\n",
                warning.token_count, warning.minimum_required
            )
            .ok();
        }

        for speaker in &self.speakers {
            writeln!(out, "****** Speaker: *{}:", speaker.speaker).ok();

            for (i, trial) in speaker.trials.iter().enumerate() {
                if i > 0 {
                    writeln!(out).ok();
                }

                writeln!(
                    out,
                    "D_optimum     <{:.2}; min least sq val = {:.3}>\n",
                    trial.d_optimum, trial.min_least_sq
                )
                .ok();

                writeln!(out, "tokens  samples    ttr     st.dev      D").ok();
                for entry in &trial.entries {
                    writeln!(
                        out,
                        "  {:>2}      {:>3}    {:.4}    {:.3}     {:.3}",
                        entry.n, entry.samples, entry.mean_ttr, entry.std_dev, entry.d_value,
                    )
                    .ok();
                }

                writeln!(
                    out,
                    "\nD: average = {:.3}; std dev. = {:.3}",
                    trial.d_average, trial.d_std_dev,
                )
                .ok();
            }

            writeln!(out, "\nVOCD RESULTS SUMMARY").ok();
            writeln!(out, "====================").ok();
            writeln!(
                out,
                "   Types,Tokens,TTR:  <{},{},{:.6}>",
                speaker.types, speaker.tokens, speaker.ttr,
            )
            .ok();

            let d_strs: Vec<String> = speaker
                .d_optimum_values
                .iter()
                .map(|d| format!("{d:.2}"))
                .collect();
            writeln!(out, "  D_optimum  values:  <{}>", d_strs.join(", ")).ok();
            writeln!(
                out,
                "  D_optimum average:  {:.2}",
                speaker.d_optimum_average
            )
            .ok();
            writeln!(out).ok();
        }

        out
    }

    /// CLAN-compatible output: warnings show only the speaker header (no warning text).
    fn render_clan(&self) -> String {
        let mut out = String::new();

        for warning in &self.warnings {
            writeln!(out, "****** Speaker: *{}:", warning.speaker).ok();
            for line in &warning.mor_lemma_lines {
                writeln!(out, "{line} ").ok();
            }
        }

        for speaker in &self.speakers {
            writeln!(out, "****** Speaker: *{}:", speaker.speaker).ok();

            for (i, trial) in speaker.trials.iter().enumerate() {
                if i > 0 {
                    writeln!(out).ok();
                }

                writeln!(
                    out,
                    "D_optimum     <{:.2}; min least sq val = {:.3}>\n",
                    trial.d_optimum, trial.min_least_sq
                )
                .ok();

                writeln!(out, "tokens  samples    ttr     st.dev      D").ok();
                for entry in &trial.entries {
                    writeln!(
                        out,
                        "  {:>2}      {:>3}    {:.4}    {:.3}     {:.3}",
                        entry.n, entry.samples, entry.mean_ttr, entry.std_dev, entry.d_value,
                    )
                    .ok();
                }

                writeln!(
                    out,
                    "\nD: average = {:.3}; std dev. = {:.3}",
                    trial.d_average, trial.d_std_dev,
                )
                .ok();
            }

            writeln!(out, "\nVOCD RESULTS SUMMARY").ok();
            writeln!(out, "====================").ok();
            writeln!(
                out,
                "   Types,Tokens,TTR:  <{},{},{:.6}>",
                speaker.types, speaker.tokens, speaker.ttr,
            )
            .ok();

            let d_strs: Vec<String> = speaker
                .d_optimum_values
                .iter()
                .map(|d| format!("{d:.2}"))
                .collect();
            writeln!(out, "  D_optimum  values:  <{}>", d_strs.join(", ")).ok();
            writeln!(
                out,
                "  D_optimum average:  {:.2}",
                speaker.d_optimum_average
            )
            .ok();
            writeln!(out).ok();
        }

        out
    }
}
