//! RELY, Inter-rater reliability (Cohen's kappa).
//!
//! Compares coded data on a specified dependent tier (default `%cod`)
//! across two parallel CHAT files to compute per-code agreement
//! statistics, overall agreement percentage, and Cohen's kappa
//! coefficient.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409232)
//! for the original RELY command specification.
//!
//! # Algorithm
//!
//! 1. Parse both input files and extract codes per utterance from the
//!    specified tier.
//! 2. Align utterances by position (index).
//! 3. For each aligned pair, count per-code agreements (minimum of the
//!    two counts for each code in that utterance).
//! 4. Compute overall observed agreement (Po) and expected agreement
//!    (Pe) for Cohen's kappa: `k = (Po - Pe) / (1 - Pe)`.
//!
//! # Differences from CLAN
//!
//! - RELY does not use the `AnalysisCommand` trait because it requires
//!   two-file input. It is invoked directly via [`run_rely`].

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::Path;

use talkbank_model::ParseValidateOptions;
use talkbank_model::Utterance;

use crate::framework::{TransformError, cod_item_values, dependent_tier_content_text};

pub use output::{CodeAgreement, RelyResult};

/// Configuration for the RELY command.
#[derive(Debug, Clone)]
pub struct RelyConfig {
    /// Tier kind to compare (e.g., %spa, %cod).
    pub tier: crate::framework::TierKind,
}

impl Default for RelyConfig {
    fn default() -> Self {
        Self {
            tier: crate::framework::TierKind::Cod,
        }
    }
}

/// Extract space-separated code tokens from a specific dependent tier.
///
/// Returns an empty vector if the tier is not present on the utterance.
/// Punctuation terminators (`.`) are excluded.
fn extract_tier_codes(utterance: &Utterance, tier_label: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for dep in &utterance.dependent_tiers {
        let kind = dep.kind();
        if kind == tier_label {
            if let talkbank_model::DependentTier::Cod(tier) = dep {
                codes.extend(cod_item_values(tier));
            } else {
                codes.extend(
                    dependent_tier_content_text(dep)
                        .split_whitespace()
                        .filter(|token| !token.is_empty() && *token != ".")
                        .map(str::to_owned),
                );
            }
        }
    }
    codes
}

/// Run RELY comparison between two CHAT files.
///
/// Parses both files, aligns utterances by position, and computes
/// per-code agreement statistics and Cohen's kappa. Returns an error
/// if either file cannot be read or parsed.
pub fn run_rely(
    config: &RelyConfig,
    file1: &Path,
    file2: &Path,
) -> Result<RelyResult, TransformError> {
    let content1 = std::fs::read_to_string(file1).map_err(TransformError::Io)?;
    let content2 = std::fs::read_to_string(file2).map_err(TransformError::Io)?;

    let chat1 = talkbank_transform::parse_and_validate(&content1, ParseValidateOptions::default())
        .map_err(|e| TransformError::Parse(format!("File 1: {e}")))?;
    let chat2 = talkbank_transform::parse_and_validate(&content2, ParseValidateOptions::default())
        .map_err(|e| TransformError::Parse(format!("File 2: {e}")))?;

    // Extract codes per utterance from both files
    let codes1: Vec<Vec<String>> = chat1
        .utterances()
        .map(|u| extract_tier_codes(u, config.tier.as_str()))
        .collect();
    let codes2: Vec<Vec<String>> = chat2
        .utterances()
        .map(|u| extract_tier_codes(u, config.tier.as_str()))
        .collect();

    // Build frequency maps
    let mut freq1: BTreeMap<String, u64> = BTreeMap::new();
    let mut freq2: BTreeMap<String, u64> = BTreeMap::new();
    let mut agreed: BTreeMap<String, u64> = BTreeMap::new();

    let n = codes1.len().min(codes2.len());
    for i in 0..n {
        // Count codes in each utterance
        let mut utt1_counts: BTreeMap<&str, u64> = BTreeMap::new();
        let mut utt2_counts: BTreeMap<&str, u64> = BTreeMap::new();

        for c in &codes1[i] {
            *utt1_counts.entry(c.as_str()).or_insert(0u64) += 1;
        }
        for c in &codes2[i] {
            *utt2_counts.entry(c.as_str()).or_insert(0u64) += 1;
        }

        // Accumulate totals
        for (code, count) in &utt1_counts {
            *freq1.entry((*code).to_owned()).or_insert(0) += count;
        }
        for (code, count) in &utt2_counts {
            *freq2.entry((*code).to_owned()).or_insert(0) += count;
        }

        // Count agreements (min of both counts per code per utterance)
        let all_codes: std::collections::BTreeSet<&str> = utt1_counts
            .keys()
            .chain(utt2_counts.keys())
            .copied()
            .collect();
        for code in all_codes {
            let c1 = utt1_counts.get(code).copied().unwrap_or(0);
            let c2 = utt2_counts.get(code).copied().unwrap_or(0);
            *agreed.entry(code.to_owned()).or_insert(0) += c1.min(c2);
        }
    }

    // Build results
    let all_codes: std::collections::BTreeSet<&str> = freq1
        .keys()
        .chain(freq2.keys())
        .map(|s| s.as_str())
        .collect();

    let mut code_results = Vec::new();
    let mut total_agreed = 0u64;
    let mut total_f1 = 0u64;
    let mut total_f2 = 0u64;

    for code in all_codes {
        let c1 = freq1.get(code).copied().unwrap_or(0);
        let c2 = freq2.get(code).copied().unwrap_or(0);
        let ag = agreed.get(code).copied().unwrap_or(0);
        let total = c1.max(c2);
        let pct = if total > 0 {
            ag as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        total_agreed += ag;
        total_f1 += c1;
        total_f2 += c2;

        code_results.push(CodeAgreement {
            code: code.to_owned(),
            count_file1: c1,
            count_file2: c2,
            agreed: ag,
            agreement_pct: pct,
        });
    }

    let total = total_f1.max(total_f2);
    let overall_agreement = if total > 0 {
        total_agreed as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    // Cohen's kappa: k = (Po - Pe) / (1 - Pe)
    let n_total = total as f64;
    let po = if n_total > 0.0 {
        total_agreed as f64 / n_total
    } else {
        0.0
    };
    let pe = if n_total > 0.0 {
        code_results
            .iter()
            .map(|c| {
                let p1 = c.count_file1 as f64 / n_total;
                let p2 = c.count_file2 as f64 / n_total;
                p1 * p2
            })
            .sum::<f64>()
    } else {
        0.0
    };
    let kappa = if (1.0 - pe).abs() > f64::EPSILON {
        (po - pe) / (1.0 - pe)
    } else {
        1.0
    };

    Ok(RelyResult {
        codes: code_results,
        overall_agreement,
        kappa,
        total_file1: total_f1,
        total_file2: total_f2,
    })
}

// RELY doesn't use the AnalysisCommand trait since it needs two-file input.
// It's invoked directly via run_rely().

/// Placeholder for CLI registration (unused -- RELY uses [`run_rely`] directly
/// because it requires two-file input rather than the single-file
/// `AnalysisCommand` trait).
pub struct RelyCommand;
