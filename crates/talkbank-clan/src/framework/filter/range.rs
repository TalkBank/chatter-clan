//! CLAN `+z` utterance-range filtering.
//!
//! A validated inclusive 1-based [`UtteranceRange`] within a file, its
//! [`FromStr`]/[`fmt::Display`] round-trip, and the clap value parser
//! ([`parse_utterance_range`]). Extracted verbatim from the `filter` module;
//! the parent re-exports the public items so `filter::UtteranceRange` etc.
//! continue to resolve.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// Inclusive 1-based utterance range within a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtteranceRange {
    start: usize,
    end: usize,
}

/// Error returned when parsing an utterance range.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseUtteranceRangeError {
    /// The input was not in `start-end` form.
    #[error("invalid range format '{input}', expected 'start-end' (e.g., '25-125')")]
    InvalidFormat {
        /// Original input string.
        input: String,
    },
    /// The bounds were syntactically valid but semantically invalid.
    #[error("invalid range '{input}', start must be >= 1 and end >= start")]
    InvalidBounds {
        /// Original input string.
        input: String,
    },
}

impl UtteranceRange {
    /// Create a validated utterance range.
    pub fn new(start: usize, end: usize) -> Result<Self, ParseUtteranceRangeError> {
        if start == 0 || end < start {
            return Err(ParseUtteranceRangeError::InvalidBounds {
                input: format!("{start}-{end}"),
            });
        }

        Ok(Self { start, end })
    }

    /// Inclusive lower bound.
    pub const fn start(self) -> usize {
        self.start
    }

    /// Inclusive upper bound.
    pub const fn end(self) -> usize {
        self.end
    }

    /// Check whether a 1-based utterance index falls within the range.
    pub const fn contains(self, utterance_index: usize) -> bool {
        utterance_index >= self.start && utterance_index <= self.end
    }
}

impl fmt::Display for UtteranceRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

impl FromStr for UtteranceRange {
    type Err = ParseUtteranceRangeError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = input.splitn(2, '-').collect();
        if parts.len() != 2 {
            return Err(ParseUtteranceRangeError::InvalidFormat {
                input: input.to_owned(),
            });
        }

        let start =
            parts[0]
                .parse::<usize>()
                .map_err(|_| ParseUtteranceRangeError::InvalidFormat {
                    input: input.to_owned(),
                })?;
        let end =
            parts[1]
                .parse::<usize>()
                .map_err(|_| ParseUtteranceRangeError::InvalidFormat {
                    input: input.to_owned(),
                })?;

        Self::new(start, end).map_err(|_| ParseUtteranceRangeError::InvalidBounds {
            input: input.to_owned(),
        })
    }
}

/// Parse a clap-friendly utterance range argument.
pub fn parse_utterance_range(input: &str) -> Result<UtteranceRange, String> {
    input
        .parse::<UtteranceRange>()
        .map_err(|error| error.to_string())
}
