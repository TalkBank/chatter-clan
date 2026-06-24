//! Typed FREQ results and rendering logic.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Serialize;
use talkbank_model::IDHeader;

use crate::framework::{CommandOutput, Mattr, TypeCount, UtteranceCount, WordCount};

/// How FREQ orders the per-word entries. The three modes are mutually
/// exclusive, so they are an enum rather than separate sort bools.
///
/// - `Alphabetical` is CLAN's default (its BST in-order traversal by display
///   form).
/// - `Frequency` is CLAN `+o` / `+o0` (`freq.cpp:176`; `freq.cpp:815-817`:
///   `*f == EOS || *f == '0'` sets `isSort`): descending count, ties keeping
///   alphabetical order.
/// - `ReverseConcordance` is CLAN `+o1` (`freq.cpp` `RevWd`): sort by the
///   reversed display form so shared suffixes cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FreqSort {
    /// CLAN default: alphabetical by display form.
    #[default]
    Alphabetical,
    /// CLAN `+o` / `+o0`: descending frequency, ties alphabetical.
    Frequency,
    /// CLAN `+o1`: reverse concordance (sort by reversed display form).
    ReverseConcordance,
}

impl FreqSort {
    /// Whether this is the default (`Alphabetical`), used by serde to omit the
    /// field from default-mode JSON so the existing schema is unchanged.
    pub fn is_default(&self) -> bool {
        matches!(self, FreqSort::Alphabetical)
    }
}

/// Typed output from the FREQ command.
///
/// Contains per-speaker frequency tables with strongly-typed fields.
#[derive(Debug, Clone, Serialize)]
pub struct FreqResult {
    /// Per-speaker frequency data, in encounter order.
    pub speakers: Vec<FreqSpeakerResult>,
    /// CLAN `+d1`: render as an alphabetized deduped word list,
    /// one word per line, with no banners, counts, or totals.
    /// Default `false` preserves the standard per-speaker layout.
    /// Field is `#[serde(default, skip_serializing_if = ...)]` so
    /// the JSON schema for default-mode FREQ output is unchanged.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub word_list_only: bool,
    /// CLAN `+d4`: emit only per-speaker type/token/TTR summary;
    /// drop all per-word frequency entries. Same defaulting rules
    /// as `word_list_only` for serde compatibility.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub types_tokens_only: bool,
    /// How the per-word entries are ordered: `Alphabetical` (CLAN default),
    /// `Frequency` (`+o`/`+o0`), or `ReverseConcordance` (`+o1`). The
    /// CLAN-format renderer re-derives display order from this. Omitted from
    /// JSON when default, like the `+dN` flags above, so default-mode output is
    /// unchanged.
    #[serde(default, skip_serializing_if = "FreqSort::is_default")]
    pub sort: FreqSort,
    /// CLAN `+d2` / `+d3`: one aggregate row per (input file x analyzed
    /// speaker) for the SpreadsheetML output, keyed by the speaker's `@ID`.
    /// Empty (and omitted from JSON) unless a spreadsheet mode is active, so
    /// default-mode output is unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_speaker_rows: Vec<FreqFileSpeakerRow>,
    /// CLAN `+bN`: whether the MATTR line is rendered per speaker. Distinct
    /// from a speaker's `mattr` being `None` (which means "fewer than `N`
    /// tokens, render `-`"): with `+bN` off no line is rendered at all. This is
    /// an internal render flag derived from config, not output data, so it is
    /// not serialized.
    #[serde(skip)]
    pub mattr_enabled: bool,
    /// CLAN `+o3` (isCombineSpeakers): the `speakers` vec holds a single pooled
    /// result and the CLAN-format renderer suppresses the per-speaker
    /// `Speaker:` header. Internal render flag derived from config, not output
    /// data, so it is not serialized.
    #[serde(skip)]
    pub combine_speakers: bool,
    /// Whether counting is `%mor`-based (CLAN `isMorUsed`): true for chatter's
    /// structural `--mor` and the CLAN `+t%mor` slot. CLAN gates the "%mor line
    /// forms" TTR advisory on `!isMorUsed` (freq.cpp:1536), so this suppresses
    /// that advisory; main-tier and non-`%mor` dependent tiers (e.g. `+t%gra`)
    /// keep it. Internal render flag derived from config, not serialized.
    #[serde(skip)]
    pub mor_based: bool,
}

/// One (file x speaker) aggregate row of the FREQ `+d2`/`+d3` spreadsheet.
///
/// CLAN emits one such row per (input file x analyzed speaker), keyed by the
/// speaker's `@ID` header (`freq.cpp` two-pass `stat.frq.*` path).
#[derive(Debug, Clone, Serialize)]
pub struct FreqFileSpeakerRow {
    /// Input file stem (the spreadsheet's `File` column).
    pub filename: String,
    /// Speaker code (e.g. `CHI`); rendered in `+d2` as the `*CHI:` pseudo-word
    /// column carrying this speaker's utterance count.
    pub speaker: String,
    /// The speaker's `@ID` header for this file, if present. `None` triggers
    /// CLAN's `.|.|<sp>|.|...` fallback (Code only, other columns `.`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<IDHeader>,
    /// Per-word counts (CLAN display form to count) for the `+d2` word columns.
    pub word_counts: BTreeMap<String, WordCount>,
    /// The speaker's utterance count (CLAN's `*SPEAKER:` pseudo-word column).
    pub utterance_count: UtteranceCount,
    /// Distinct word types for this (file, speaker).
    pub total_types: TypeCount,
    /// Total word tokens for this (file, speaker).
    pub total_tokens: WordCount,
    /// Type-token ratio (`-` rendered when there are zero tokens).
    pub ttr: f64,
    /// CLAN `+bN`: Moving-Average TTR for this (file, speaker), appended as the
    /// trailing `MATTR` spreadsheet column (`freq.cpp:3303`, `1558-1562`).
    /// `None` both when `+bN` is off and when the speaker has fewer than `N`
    /// tokens (rendered `-`); `FreqResult::mattr_enabled` says whether the
    /// column exists at all. Omitted from JSON when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mattr: Option<Mattr>,
}

/// Frequency statistics for a single speaker.
#[derive(Debug, Clone, Serialize)]
pub struct FreqSpeakerResult {
    /// Speaker code (e.g., "CHI", "MOT")
    pub speaker: String,
    /// Word frequency entries, sorted by count descending then alphabetically.
    pub entries: Vec<FreqEntry>,
    /// Number of unique word types
    pub total_types: TypeCount,
    /// Total word tokens
    pub total_tokens: WordCount,
    /// Type-token ratio (types / tokens)
    pub ttr: f64,
    /// CLAN `+bN`: Moving-Average Type-Token Ratio for this speaker. `None`
    /// both when `+bN` is off and when the speaker has fewer than `N` tokens;
    /// `FreqResult::mattr_enabled` disambiguates the two for rendering. Omitted
    /// from JSON when absent so default-mode output is unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mattr: Option<Mattr>,
}

/// A single word frequency entry.
#[derive(Debug, Clone, Serialize)]
pub struct FreqEntry {
    /// The word (lowercased, cleaned, `+` stripped from compounds)
    pub word: String,
    /// CLAN display form (lowercased but `+` preserved in compounds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_form: Option<String>,
    /// Number of occurrences
    pub count: WordCount,
}

impl FreqEntry {
    /// The CLAN display form, falling back to the normalized word when no
    /// distinct display form was recorded.
    fn display(&self) -> &str {
        self.display_form.as_deref().unwrap_or(&self.word)
    }
}

impl CommandOutput for FreqResult {
    /// Our clean text format with aligned table columns.
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
                     \x20 Total types: {}\n\
                     \x20 Total tokens: {}\n\
                     \x20 TTR: {:.3}\n",
                    s.speaker, s.total_types, s.total_tokens, s.ttr
                ),
            )
            .ok();

            // Table with aligned columns
            if !s.entries.is_empty() {
                let count_width = s
                    .entries
                    .iter()
                    .map(|e| e.count.to_string().len())
                    .max()
                    .unwrap_or(5)
                    .max(5); // "Count" header
                let word_width = s
                    .entries
                    .iter()
                    .map(|e| e.word.len())
                    .max()
                    .unwrap_or(4)
                    .max(4); // "Word" header

                fmt::write(
                    &mut out,
                    format_args!(
                        "  {:<cw$}  {:<ww$}\n  {:-<cw$}  {:-<ww$}\n",
                        "Count",
                        "Word",
                        "",
                        "",
                        cw = count_width,
                        ww = word_width
                    ),
                )
                .ok();

                for entry in &s.entries {
                    fmt::write(
                        &mut out,
                        format_args!(
                            "  {:<cw$}  {:<ww$}\n",
                            entry.count,
                            entry.word,
                            cw = count_width,
                            ww = word_width
                        ),
                    )
                    .ok();
                }
            }
        }
        out
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// Format (from CLAN snapshot):
    /// ```text
    /// Speaker: *CHI:
    ///   1 cookie
    ///   1 more
    /// ------------------------------
    ///     3  Total number of different item types used
    ///     3  Total number of items (tokens)
    /// 1.000  Type/Token ratio
    ///     This TTR number was not calculated on the basis of %mor line forms.
    ///     If you want a TTR based on lemmas, run FREQ on the %mor line
    ///     with option: +sm;*,o%
    /// ```
    fn render_clan(&self) -> String {
        if self.word_list_only {
            // CLAN `+d1`: alphabetized deduped word list, one per
            // line, per the manual, fodder for `kwal +s@FILE`,
            // which wants one global vocabulary, not a per-speaker
            // partition. Banners, counts, separators, and TTR are
            // intentionally omitted. `+d1` combines all speakers
            // (`isCombineSpeakers`), so CLAN prefixes the list with a
            // `;%* Combined Speakers output:` header (freq.cpp:1468-1471,
            // the `onlydata == 2 && chatmode` arm; no speaker-count
            // guard). Pinned by `freq_d1_word_list_eng`.
            let mut words: BTreeSet<&str> = BTreeSet::new();
            for s in &self.speakers {
                for entry in &s.entries {
                    words.insert(entry.display());
                }
            }
            let mut out = String::new();
            out.push_str(";%* Combined Speakers output:\n");
            for w in &words {
                out.push_str(w);
                out.push('\n');
            }
            return out;
        }
        let mut out = String::new();
        for (i, s) in self.speakers.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            // CLAN `+o3` (isCombineSpeakers) pools all speakers into one table
            // with no `Speaker:` banner (freq.cpp:1451 prints the banner only
            // when `!isCombineSpeakers`). The combined result is the sole entry.
            if !self.combine_speakers {
                fmt::write(&mut out, format_args!("Speaker: *{}:\n", s.speaker)).ok();
            }

            // Display order is re-derived here for CLAN parity (the
            // accumulator's freq-descending order is for chatter's own
            // text/JSON formats). CLAN `+o1` orders by the reversed
            // display form so shared suffixes cluster (freq.cpp `RevWd`,
            // `freq_treeprint`); the default is alphabetical by display
            // form (CLAN's BST in-order traversal). Pinned by
            // `freq_o1_reverse_concordance_eng`.
            let mut sorted_entries: Vec<&FreqEntry> = s.entries.iter().collect();
            match self.sort {
                FreqSort::ReverseConcordance => {
                    sorted_entries
                        .sort_by_cached_key(|e| e.display().chars().rev().collect::<String>());
                }
                FreqSort::Frequency => {
                    // CLAN `+o` / `+o0`: descending count, ties alphabetical by
                    // display form.
                    sorted_entries.sort_by(|a, b| {
                        b.count
                            .cmp(&a.count)
                            .then_with(|| a.display().cmp(b.display()))
                    });
                }
                FreqSort::Alphabetical => {
                    sorted_entries.sort_by(|a, b| a.display().cmp(b.display()));
                }
            }

            // Word list: " <count> <display_form>". CLAN's `+d4`
            // suppresses the per-word entries but keeps the
            // surrounding speaker banner, separator, totals, and
            // TTR note.
            if !self.types_tokens_only {
                for entry in &sorted_entries {
                    fmt::write(
                        &mut out,
                        format_args!("{:>3} {}\n", entry.count, entry.display()),
                    )
                    .ok();
                }
            }

            // Separator and per-speaker totals. CLAN (non-CLAN_SRV build,
            // freq.cpp:1528-1541) always prints the separator and the
            // type/token counts, but prints the Type/Token ratio line and the
            // TTR note ONLY when the speaker has at least one token: with 0
            // tokens the ratio t/t1 is undefined, so CLAN omits both. Pinned by
            // `freq_c_capitalization_eng`, where a `+c`-filtered speaker with no
            // capitalized words has 0 tokens.
            fmt::write(
                &mut out,
                format_args!(
                    "------------------------------\n\
                     {:>5}  Total number of different item types used\n\
                     {:>5}  Total number of items (tokens)\n",
                    s.total_types, s.total_tokens
                ),
            )
            .ok();

            if s.total_tokens > 0 {
                fmt::write(&mut out, format_args!("{:.3}  Type/Token ratio\n", s.ttr)).ok();

                // TTR note. CLAN gates this on `!isMorUsed` (freq.cpp:1536): a
                // %mor-based count (chatter `--mor` or CLAN `+t%mor`) suppresses
                // it, since the TTR then already reflects %mor forms; main-tier
                // counting and non-%mor dependent tiers (e.g. `+t%gra`) keep it.
                if !self.mor_based {
                    out.push_str(
                        "    This TTR number was not calculated on the basis of %mor line forms.\n\
                         \x20   If you want a TTR based on lemmas, run FREQ on the %mor line\n\
                         \x20   with option: +sm;*,o%\n",
                    );
                }
            }

            // CLAN `+bN`: the MATTR line, emitted per speaker whenever +bN is
            // active, AFTER the TTR caveat and (unlike TTR) regardless of token
            // count. Fewer than N tokens -> `-      MATTR` (dash + 6 spaces),
            // matching CLAN's `NMATTRs == 0` branch (freq.cpp:1542-1547). The
            // defined form is `%5.3f  MATTR`; `Mattr`'s Display gives the
            // five-character `0.XXX`/`1.000`.
            if self.mattr_enabled {
                match s.mattr {
                    Some(mattr) => {
                        fmt::write(&mut out, format_args!("{mattr}  MATTR\n")).ok();
                    }
                    None => out.push_str("-      MATTR\n"),
                }
            }
        }
        // CLAN emits a trailing blank line after the last per-speaker block;
        // match that so a hex-level diff against the legacy freq output ends
        // cleanly.
        if !self.speakers.is_empty() {
            out.push('\n');
        }
        out
    }

    /// CSV rendering with header row.
    fn render_csv(&self) -> String {
        let mut out = String::new();
        for s in &self.speakers {
            out.push_str(&format!("Speaker,{}\n", s.speaker));
            // CLAN `+d3`: drop the per-word `Count,Word` header and
            // rows, keep the summary statistics. CLAN `+d2` (and
            // default csv) keeps them.
            if !self.types_tokens_only {
                out.push_str("Count,Word\n");
                for entry in &s.entries {
                    out.push_str(&format!("{},{}\n", entry.count, entry.word));
                }
            }
            out.push_str(&format!(
                "Total types,{}\nTotal tokens,{}
TTR,{:.3}\n",
                s.total_types, s.total_tokens, s.ttr
            ));
        }
        out
    }
}
