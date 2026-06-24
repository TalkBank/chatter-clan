//! Column mapping between KidEval `.cut` database scores and `SpeakerKideval` fields.
//!
//! The `.cut` database stores ~25 fixed metrics followed by variable-length
//! morphosyntactic counts. This module defines the fixed column positions and
//! provides conversion functions between `SpeakerKideval` and flat score vectors.
//!
//! Column order is defined by the `0all_norms_with_columns.csv` reference file
//! in `lib/kideval/`.
//!
//! # Differences from CLAN
//!
//! This module has no direct CLAN equivalent, CLAN embeds column mapping
//! inline in `kideval.cpp`. Extracting it as a separate module enables reuse
//! by the VS Code extension and other API consumers.

mod mappings;
#[cfg(test)]
mod tests;

use serde::Serialize;

/// Fixed KidEval `.cut` column indices.
pub mod col;

pub use mappings::{map_kideval_comparison, speaker_to_score_vector};

/// A named comparison for a single KidEval measure.
#[derive(Debug, Clone, Serialize)]
pub struct KidevalMeasureComparison {
    /// Human-readable label (e.g., "MLU (words)").
    pub label: &'static str,
    /// The speaker's score.
    pub score: f64,
    /// Database population mean.
    pub db_mean: f64,
    /// Database population standard deviation.
    pub db_sd: f64,
    /// Z-score (standard deviations from norm). `None` if SD is zero.
    pub z_score: Option<f64>,
    /// Number of database entries used.
    pub db_n: usize,
}
