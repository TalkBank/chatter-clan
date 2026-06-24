//! Lexical-diversity statistics shared across CLAN analysis commands.
//!
//! Today this is the Moving-Average Type-Token Ratio (MATTR), used by FREQ's
//! `+bN` flag (`freq.cpp:771-781`, `comute_MATTR` at `freq.cpp:1742-1781`).
//! It lives in the framework rather than the FREQ command because lexical
//! diversity is a cross-command concern: VOCD and future diversity measures
//! reuse the same windowed primitive over a token stream.

use std::collections::HashSet;
use std::fmt;
use std::num::NonZeroUsize;
use std::str::FromStr;

use serde::Serialize;

use super::NormalizedWord;

/// The sliding-window length `N` for MATTR (CLAN `+bN`). Always positive: CLAN
/// rejects `+b0` ("specify the frame size greater than zero",
/// `freq.cpp:777-780`), so the type makes a zero-length window unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSize(NonZeroUsize);

impl FrameSize {
    /// The window length as a `usize` (always `>= 1`).
    pub fn get(self) -> usize {
        self.0.get()
    }
}

/// Why a [`FrameSize`] could not be parsed from a flag value.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FrameSizeError {
    /// The value was not a non-negative integer.
    #[error("frame size must be a non-negative integer: {0:?}")]
    NotANumber(String),
    /// The value parsed to zero, which CLAN rejects.
    #[error("frame size must be greater than zero")]
    Zero,
}

impl FromStr for FrameSize {
    type Err = FrameSizeError;

    /// Parse a `+bN` value: a positive integer. Rejects non-numbers and zero,
    /// mirroring CLAN's two `+b` error paths (`freq.cpp:773-780`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n: usize = s
            .parse()
            .map_err(|_| FrameSizeError::NotANumber(s.to_owned()))?;
        NonZeroUsize::new(n)
            .map(FrameSize)
            .ok_or(FrameSizeError::Zero)
    }
}

/// A Moving-Average Type-Token Ratio value, in `[0.0, 1.0]`. Wrapping the float
/// keeps the diversity statistic from being confused with the plain TTR at call
/// sites (both are `f64` in `[0, 1]` but mean different things). Serializes
/// transparently as its numeric value for chatter's JSON output.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Mattr(f64);

impl Mattr {
    /// The underlying average window TTR, in `[0.0, 1.0]`.
    pub fn value(self) -> f64 {
        self.0
    }
}

impl fmt::Display for Mattr {
    /// CLAN renders MATTR with `%5.3f` (`freq.cpp:1544`); since the value is in
    /// `[0.0, 1.0]` that is always five characters (`0.XXX` / `1.000`), so a
    /// plain three-decimal format matches byte-for-byte.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// Moving-Average Type-Token Ratio over an ordered token stream.
///
/// For each length-`frame` window `tokens[i..i+N]`, the window TTR is
/// `distinct_types / N`; the result is the mean of those window TTRs across all
/// `T - N + 1` windows. Returns `None` when `T < N` (no full window exists),
/// matching CLAN's `NMATTRs == 0` case, which prints `-` rather than a number
/// (`freq.cpp:1520-1523`).
///
/// Distinctness is by [`NormalizedWord`], the same key FREQ counts types with,
/// so the window TTR uses FREQ's type definition (and its `+k` case polarity).
/// This mirrors CLAN, whose `comute_MATTR` feeds the already-processed word
/// form `w` into a per-window BST and averages `distinct / N`.
pub fn moving_average_ttr(tokens: &[NormalizedWord], frame: FrameSize) -> Option<Mattr> {
    let n = frame.get();
    if tokens.len() < n {
        return None;
    }
    let window_count = tokens.len() - n + 1;
    let mut sum = 0.0_f64;
    for start in 0..window_count {
        let distinct = tokens[start..start + n]
            .iter()
            .collect::<HashSet<_>>()
            .len();
        sum += distinct as f64 / n as f64;
    }
    Some(Mattr(sum / window_count as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(n: usize) -> FrameSize {
        format!("{n}").parse().expect("positive frame size")
    }

    fn tokens(words: &[&str]) -> Vec<NormalizedWord> {
        words
            .iter()
            .map(|w| NormalizedWord::from_text_cased(w, true))
            .collect()
    }

    #[test]
    fn frame_size_rejects_zero_and_non_numbers() {
        assert_eq!("0".parse::<FrameSize>(), Err(FrameSizeError::Zero));
        assert_eq!(
            "x".parse::<FrameSize>(),
            Err(FrameSizeError::NotANumber("x".to_owned()))
        );
        assert_eq!("3".parse::<FrameSize>().map(|f| f.get()), Ok(3));
    }

    #[test]
    fn fewer_tokens_than_frame_is_undefined() {
        assert_eq!(moving_average_ttr(&tokens(&["a", "b"]), frame(3)), None);
    }

    #[test]
    fn all_distinct_windows_average_to_one() {
        // 4 tokens, frame 3 -> windows [a,b,c],[b,c,d], both all-distinct:
        // (3/3 + 3/3) / 2 = 1.000.
        let m = moving_average_ttr(&tokens(&["a", "b", "c", "d"]), frame(3)).expect("defined");
        assert!((m.value() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn repeats_within_window_lower_the_average() {
        // tokens a a b, frame 2 -> windows [a,a] (1 distinct), [a,b] (2):
        // (1/2 + 2/2) / 2 = 0.75.
        let m = moving_average_ttr(&tokens(&["a", "a", "b"]), frame(2)).expect("defined");
        assert!((m.value() - 0.75).abs() < 1e-9);
        assert_eq!(m.to_string(), "0.750");
    }
}
