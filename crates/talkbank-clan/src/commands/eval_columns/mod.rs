//! Column mapping between Eval `.cut` database scores and `SpeakerEval` fields.
//!
//! The Eval `.cut` database stores ~34 fixed metrics per gem group followed by
//! word frequency rankings. This module defines the fixed column positions and
//! provides conversion between `SpeakerEval` and the positional score vectors
//! used by the comparison engine.
//!
//! Column order is defined by `retrieveTS()` in `eval.cpp` (CLAN C source).
//!
//! # Differences from CLAN
//!
//! This module has no direct CLAN equivalent, CLAN embeds column mapping
//! inline in `eval.cpp`. Extracting it as a separate module enables reuse
//! by the VS Code extension and other API consumers.

mod mappings;
#[cfg(test)]
mod tests;

use serde::Serialize;

/// Fixed Eval `.cut` column indices.
pub mod col;

pub use mappings::{map_eval_comparison, speaker_to_score_vector};

/// A named comparison for a single Eval measure.
#[derive(Debug, Clone, Serialize)]
pub struct EvalMeasureComparison {
    /// Human-readable name of the measure (e.g. "MLU (words)").
    pub label: &'static str,
    /// The speaker's observed score.
    pub score: f64,
    /// Database mean for this measure.
    pub db_mean: f64,
    /// Database standard deviation.
    pub db_sd: f64,
    /// Z-score relative to the database, if SD > 0.
    pub z_score: Option<f64>,
    /// Number of database entries used for comparison.
    pub db_n: usize,
}
