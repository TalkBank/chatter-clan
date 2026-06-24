//! Small shared statistical primitives for CLAN analysis commands.
//!
//! These are deliberately general (they operate on a slice of counts, not on
//! any command-specific type) so that commands which report the same statistic
//! compute it identically. CLAN's analysis tools (MLU section 7.21, MLT section
//! 7.20) report a population standard deviation (`/ n`, not the sample `/ (n-1)`)
//! over per-utterance length counts, and render "NA"/"NaN" when there are fewer
//! than two values; [`population_sd`] is the single source of that formula.

/// Population standard deviation (`/ n`) of a list of non-negative counts, or
/// `f64::NAN` when there are fewer than two values.
///
/// The NaN case mirrors CLAN, which prints "NA" for a standard deviation it
/// cannot compute over a single observation (`num_utts <= 1`). Callers that
/// render CLAN-format output rely on `NaN` to trigger that "NA" branch.
///
/// Shared by:
/// - MLU ([`crate::commands::mlu`]): per-utterance morpheme (or word) lengths,
///   `mlu.cpp` SD over `utterance_lengths`.
/// - MLT ([`crate::commands::mlt`]): per-utterance word counts, `mlt.cpp` SD
///   over individual utterance word counts (NOT per-turn counts).
///
/// The two commands previously inlined byte-identical copies of this loop; the
/// mean computed here (`sum / n`) equals MLT's `words_per_utterance`
/// (`total_words / total_utterances`), so the extraction is behavior-preserving.
pub fn population_sd(values: &[u64]) -> f64 {
    let n = values.len();
    if n <= 1 {
        return f64::NAN;
    }
    let mean = values.iter().sum::<u64>() as f64 / n as f64;
    let sum_sq: f64 = values
        .iter()
        .map(|&v| {
            let diff = v as f64 - mean;
            diff * diff
        })
        .sum();
    (sum_sq / n as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::population_sd;

    /// Fewer than two observations -> NaN (CLAN renders "NA").
    #[test]
    fn fewer_than_two_values_is_nan() {
        assert!(population_sd(&[]).is_nan());
        assert!(population_sd(&[5]).is_nan());
    }

    /// Population SD (`/ n`) of the `+o3` MLU pooled example `[6, 5]`: mean 5.5,
    /// each deviates by 0.5, so SD = 0.5 exactly. Pins the MLU combine-speakers
    /// golden's expected `Standard deviation = 0.500`.
    #[test]
    fn two_values_population_sd() {
        assert!((population_sd(&[6, 5]) - 0.5).abs() < 1e-12);
    }

    /// Population SD (`/ n`), not sample SD (`/ (n-1)`): for `[2, 4, 4, 4, 5,
    /// 5, 7, 9]` the population SD is exactly 2.0 (variance 4.0), whereas the
    /// sample SD would be ~2.138. Guards against a `/ (n-1)` regression.
    #[test]
    fn uses_population_not_sample_denominator() {
        assert!((population_sd(&[2, 4, 4, 4, 5, 5, 7, 9]) - 2.0).abs() < 1e-12);
    }
}
