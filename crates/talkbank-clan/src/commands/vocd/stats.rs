//! Statistical helpers for VOCD sampling and D-curve fitting.

use std::collections::HashSet;

use rand::prelude::*;
use rand::rngs::StdRng;

use super::output::{NtEntry, VocdTrial};

/// Step size for gradient-descent D optimization (matches CLAN's 0.001).
const OPTIMIZATION_STEP: f64 = 0.001;

/// Run a single VOCD trial: sample at each N, compute TTR, fit D.
///
/// # Precondition
///
/// `tokens.len() >= sample_to`, caller must verify sufficient tokens.
///
/// # Postcondition
///
/// Produces one `NtEntry` for each sample size in `[from..=to]` plus fitted D statistics.
pub(super) fn run_trial(
    tokens: &[String],
    sample_from: usize,
    sample_to: usize,
    num_samples: usize,
    rng: &mut StdRng,
) -> VocdTrial {
    let mut entries = Vec::with_capacity(sample_to - sample_from + 1);

    for n in sample_from..=sample_to {
        let (mean_ttr, std_dev) = average_ttr(tokens, n, num_samples, rng);
        let d_value = d_from_ttr(n, mean_ttr);

        entries.push(NtEntry {
            n,
            samples: num_samples,
            mean_ttr,
            std_dev,
            d_value,
        });
    }

    let d_sum: f64 = entries.iter().map(|e| e.d_value).sum();
    let d_count = entries.len() as f64;
    let d_average = d_sum / d_count;

    let d_variance: f64 = entries
        .iter()
        .map(|e| (e.d_value - d_average).powi(2))
        .sum::<f64>()
        / d_count;
    let d_std_dev = d_variance.sqrt();

    let (d_optimum, min_least_sq) = find_min_d(d_average, &entries);

    VocdTrial {
        entries,
        d_average,
        d_std_dev,
        d_optimum,
        min_least_sq,
    }
}

/// Compute average TTR by random sampling without replacement.
///
/// Draws `num_samples` random subsets of size `n` from `tokens` (without
/// replacement within each sample), computes TTR for each, and returns
/// the (mean, std_dev) of TTR values.
///
/// # Precondition
///
/// `n <= tokens.len()`, sample size must not exceed available tokens.
///
/// # Postcondition
///
/// Produces `(mean_ttr, std_dev)` with mean TTR bounded to `[0.0, 1.0]`.
pub(super) fn average_ttr(
    tokens: &[String],
    n: usize,
    num_samples: usize,
    rng: &mut StdRng,
) -> (f64, f64) {
    let total_tokens = tokens.len();
    let mut ttr_values = Vec::with_capacity(num_samples);

    for _ in 0..num_samples {
        let mut selected = HashSet::with_capacity(n);
        let mut sample_types = HashSet::new();

        while selected.len() < n {
            let idx = rng.random_range(0..total_tokens);
            if selected.insert(idx) {
                sample_types.insert(tokens[idx].as_str());
            }
        }

        let ttr = sample_types.len() as f64 / n as f64;
        ttr_values.push(ttr);
    }

    let mean = ttr_values.iter().sum::<f64>() / ttr_values.len() as f64;
    let variance =
        ttr_values.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / ttr_values.len() as f64;
    let std_dev = variance.sqrt();

    (mean, std_dev)
}

/// Theoretical TTR as a function of D and N.
///
/// `TTR(N) = (D/N) * [sqrt(1 + 2*N/D) - 1]`
///
/// # Precondition
///
/// `d > 0.0` and `n > 0`, D must be positive, N must be at least 1.
///
/// # Postcondition
///
/// Output is theoretically bounded near `[0, 1]`.
pub(super) fn ttr_equation(d: f64, n: usize) -> f64 {
    let n_f = n as f64;
    (d / n_f) * ((1.0 + 2.0 * n_f / d).sqrt() - 1.0)
}

/// Compute D from an observed (N, TTR) pair using the inverse equation.
///
/// `D = 0.5 * (N * T^2) / (1 - T)` where T is the observed TTR.
///
/// # Precondition
///
/// `ttr < 1.0`, a TTR of exactly 1.0 would divide by zero.
///
/// # Postcondition
///
/// Produces `D >= 0.0`; for degenerate `TTR >= 1.0` the function returns `0.0`.
pub(super) fn d_from_ttr(n: usize, ttr: f64) -> f64 {
    let tmp = 1.0 - ttr;
    if tmp == 0.0 {
        return 0.0;
    }
    0.5 * (n as f64 * ttr * ttr) / tmp
}

/// Compute sum of squared errors between observed TTR values and
/// predicted TTR values for a given D.
///
/// # Precondition
///
/// `d > 0.0`, D must be positive for the TTR equation to be valid.
///
/// # Postcondition
///
/// Produces a non-negative least-squares error value.
fn d_least_squares(d: f64, entries: &[NtEntry]) -> f64 {
    entries
        .iter()
        .map(|e| {
            let predicted = ttr_equation(d, e.n);
            (e.mean_ttr - predicted).powi(2)
        })
        .sum()
}

/// Find the D value that minimizes the least-squares error.
///
/// Uses CLAN's gradient-descent approach: determine direction from
/// the initial estimate (`d_avg`), then walk in steps of 0.001 until
/// the error starts increasing.
///
/// # Precondition
///
/// `d_avg > 0.0` and `entries` is non-empty.
///
/// # Postcondition
///
/// Produces `(d_optimum, min_least_sq_error)`.
pub(super) fn find_min_d(d_avg: f64, entries: &[NtEntry]) -> (f64, f64) {
    if d_avg <= 0.0 {
        return (0.0, f64::MAX);
    }

    let current_ls = d_least_squares(d_avg, entries);
    let slightly_lower_ls = d_least_squares(d_avg - OPTIMIZATION_STEP, entries);

    let diff = current_ls - slightly_lower_ls;
    let direction: f64 = if diff > 0.0 {
        -1.0
    } else if diff < 0.0 {
        1.0
    } else {
        return (d_avg, current_ls);
    };

    let mut prev_ls = current_ls;
    let mut d = d_avg;
    let upper_bound = 2.0 * d_avg;

    loop {
        d += direction * OPTIMIZATION_STEP;
        if d <= 0.0 || d >= upper_bound {
            break;
        }

        let next_ls = d_least_squares(d, entries);
        if prev_ls < next_ls {
            d -= direction * OPTIMIZATION_STEP;
            break;
        }
        prev_ls = next_ls;
    }

    (d, prev_ls)
}
