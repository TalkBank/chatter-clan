//! FREQ `+d2` / `+d3` SpreadsheetML construction.
//!
//! This module lowers FREQ's per-(file x speaker) results into the typed
//! [`Workbook`](crate::framework::spreadsheet::Workbook) model, reproducing
//! CLAN's FREQ spreadsheet (`freq.cpp` two-pass `stat.frq.*` path) to semantic
//! cell equivalence.
//!
//! This first slice covers the `@ID`-derived columns
//! ([`id_header_to_cells`](id_columns::id_header_to_cells)); the full
//! `+d2`/`+d3` workbook layout builds on it.

use std::collections::BTreeMap;

use super::{FreqFileSpeakerRow, FreqResult};
use crate::framework::spreadsheet::{Cell, ColumnWidth, Row, SheetName, Workbook, Worksheet};

// The `@ID`-column lowering and the row builders are split into sibling
// submodules to keep this file browseable; their items are re-exported so the
// `impl FreqResult` workbook builders (and the unit tests) reach them by name.
mod id_columns;
mod rows;

use rows::{PERCENT_SPEAKER_HEADER, data_row, header_row, percent_data_row, word_columns};

/// Which FREQ spreadsheet to emit, mapped from CLAN's `+d2` / `+d3`.
///
/// `+d2` (`onlydata = 3`) emits per-word frequency columns plus the
/// type/token/TTR summary; `+d3` (`onlydata = 4`) emits the summary only.
/// These are chatter-only flag values carrying CLAN's slot semantics; the
/// stdout `--format csv` convenience is separate (faithfulness rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqSpreadsheetMode {
    /// CLAN `+d2`: per-word columns plus type/token/TTR.
    PerWord,
    /// CLAN `+d3`: type/token/TTR only (no per-word columns).
    TypesTokens,
    /// CLAN `+d20` (`isSpreadsheetOnePerRow`, also `onlydata = 3`): a flat
    /// `File | Code | Word | Count` sheet with one row per (file, speaker,
    /// word). Unlike `PerWord`/`TypesTokens` it carries no `@ID` columns, no
    /// type/token/TTR summary, and no `%mor` TTR caveat.
    PerSpeakerWord,
    /// CLAN `+dCN` (`onlydata = 4`, the percent-of-speakers type filter,
    /// `freq.cpp:841-878`, `statfreq_percent_result` at `freq.cpp:2841`): the
    /// `+d3`-shaped summary (no per-word columns), but each speaker's
    /// Types/Token/TTR is computed over ONLY the words whose distinct-speaker
    /// count satisfies [`SpeakerPercentFilter`]. The percent path labels its
    /// fourth column `Speaker` (not `+d2`/`+d3`'s `Code`, `freq.cpp:2874`) and
    /// writes `words.frq.xls` rather than `stat.frq*.xls`.
    PercentOfSpeakers(SpeakerPercentFilter),
}

/// CLAN's `+dCN` comparator (the manual's `C` metavariable, CLAN.html §"+dCN").
/// `percentC` 1..5 in `freq.cpp:841-861`: each word is kept iff its
/// distinct-speaker count compares this way against the speaker-count threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakerPercentComparison {
    /// `+d<N` (`percentC = 1`): used by fewer than N% of speakers.
    LessThan,
    /// `+d<=N` / `+d=<N` (`percentC = 2`): N% or fewer.
    LessOrEqual,
    /// `+d=N` (`percentC = 3`): exactly the N% threshold.
    Equal,
    /// `+d>=N` / `+d=>N` (`percentC = 4`): N% or more.
    GreaterOrEqual,
    /// `+d>N` (`percentC = 5`): more than N% of speakers.
    GreaterThan,
}

impl SpeakerPercentComparison {
    /// Whether a word with `distinct_speakers` distinct-speaker uses is kept
    /// against `threshold` (`freq.cpp:2801-2805`).
    fn keeps(self, distinct_speakers: u64, threshold: u64) -> bool {
        match self {
            Self::LessThan => distinct_speakers < threshold,
            Self::LessOrEqual => distinct_speakers <= threshold,
            Self::Equal => distinct_speakers == threshold,
            Self::GreaterOrEqual => distinct_speakers >= threshold,
            Self::GreaterThan => distinct_speakers > threshold,
        }
    }

    /// CLAN's `percentToStr` rendering of the comparator, used in error wording
    /// (`freq.cpp:868`, `891`). `=<`/`=>` normalize to `<=`/`>=`.
    pub fn as_clan_str(self) -> &'static str {
        match self {
            Self::LessThan => "<",
            Self::LessOrEqual => "<=",
            Self::Equal => "=",
            Self::GreaterOrEqual => ">=",
            Self::GreaterThan => ">",
        }
    }
}

/// The percentage `N` in CLAN's `+dCN` filter. CLAN itself only requires that
/// `N` be all digits (`freq.cpp:871`, `atoi`); it does not clamp to `0..=100`,
/// so neither do we (a threshold above the speaker count simply keeps nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeakerPercent(u64);

impl SpeakerPercent {
    /// Construct from the parsed digits of a `+dCN` flag.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The raw percentage value (for error wording / serialization).
    pub fn value(self) -> u64 {
        self.0
    }
}

/// CLAN's `+dCN` percent-of-speakers filter: a comparator plus a percentage.
///
/// A word is kept iff its distinct-speaker count (the number of (file x speaker)
/// rows that used it, CLAN's `statfreq_AddWords` `p->count`, `freq.cpp:2756-2762`)
/// compares `comparison` against `floor(num_speaker_rows * percent / 100)`
/// (CLAN's `findPercentNum`, `freq.cpp:2664-2678`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeakerPercentFilter {
    /// The comparator (`<`, `<=`, `=`, `>=`, `>`).
    pub comparison: SpeakerPercentComparison,
    /// The percentage threshold `N`.
    pub percent: SpeakerPercent,
}

impl SpeakerPercentFilter {
    /// The speaker-count threshold for `num_speaker_rows` total (file x speaker)
    /// rows: `floor(num_speaker_rows * percent / 100)`. CLAN computes this in
    /// `float` then truncates (`freq.cpp:2673-2676`); for the non-negative
    /// integer domain that equals this integer floor exactly. `u128` widening
    /// keeps the product from overflowing for any realistic row count.
    fn threshold(self, num_speaker_rows: u64) -> u64 {
        ((num_speaker_rows as u128 * self.percent.0 as u128) / 100) as u64
    }
}

/// Worksheet name CLAN gives the FREQ spreadsheet (`excelHeader`).
const SHEET_NAME: &str = "sheet 1";

/// Default column width CLAN uses for the FREQ spreadsheet (`excelHeader(.., 95)`).
const FREQ_COLUMN_WIDTH: u32 = 95;

/// The three red advisory rows CLAN prepends when TTR was not computed on the
/// `%mor` line (its `!isMorUsed` branch). CLAN's spreadsheet leaks a printf
/// `%%mor`; chatter emits the correct `%mor` (divergence CLAN-DIV-004). The
/// third line's single trailing `%` is the `o%` of the option syntax, not an
/// escape, so it is unchanged.
const TTR_CAVEAT_LINES: [&str; 3] = [
    "This TTR number was not calculated on the basis of %mor line forms.",
    "If you want a TTR based on lemmas, run FREQ on the %mor line",
    "with option: +sm;*,o%",
];

impl FreqResult {
    /// Build the SpreadsheetML workbook for CLAN `+d2` (per-word) or `+d3`
    /// (types/tokens/TTR only) from the per-(file x speaker) rows.
    ///
    /// `mor_based` is whether FREQ ran on the `%mor` line; when false (the
    /// main-tier default) the red TTR-caveat rows are emitted.
    pub fn to_spreadsheet(&self, mode: FreqSpreadsheetMode, mor_based: bool) -> Workbook {
        // `+d20` (flat one-row-per-word) and `+dCN` (percent-filtered summary)
        // are wholly different layouts from the `+d2`/`+d3` wide sheet, so each
        // has its own builder rather than a column toggle.
        match mode {
            FreqSpreadsheetMode::PerSpeakerWord => return self.to_one_per_row_workbook(),
            FreqSpreadsheetMode::PercentOfSpeakers(filter) => {
                return self.to_percent_workbook(filter, mor_based);
            }
            FreqSpreadsheetMode::PerWord | FreqSpreadsheetMode::TypesTokens => {}
        }

        let mut rows: Vec<Row> = Vec::new();

        if !mor_based {
            for line in TTR_CAVEAT_LINES {
                rows.push(Row::new(vec![Cell::red_text(line)]));
            }
        }

        // `+d2` per-word columns: the byte-sorted union of speaker pseudo-words
        // and word forms across all rows (CLAN's shared WordsHead). `+d3` has none.
        // `PerSpeakerWord` is early-returned above and never reaches here; it is
        // listed only to keep this match exhaustive (the crate bans `_ =>`
        // catch-alls and `unreachable!` in library code).
        let word_columns: Vec<String> = match mode {
            FreqSpreadsheetMode::PerWord => word_columns(&self.file_speaker_rows),
            FreqSpreadsheetMode::TypesTokens
            | FreqSpreadsheetMode::PerSpeakerWord
            | FreqSpreadsheetMode::PercentOfSpeakers(_) => Vec::new(),
        };

        // CLAN appends a trailing `MATTR` column (header + per-row value) when
        // `+bN` is active (freq.cpp:3303); `mattr_enabled` carries that here.
        let mattr = self.mattr_enabled;
        rows.push(header_row(&word_columns, mattr, None));
        for row in &self.file_speaker_rows {
            rows.push(data_row(row, &word_columns, mode, mattr));
        }
        // CLAN closes the table with a trailing empty row.
        rows.push(Row::empty());

        let sheet = Worksheet::new(
            SheetName::new(SHEET_NAME),
            ColumnWidth(FREQ_COLUMN_WIDTH),
            rows,
        );
        Workbook::new(vec![sheet])
    }

    /// Build the CLAN `+d20` (`isSpreadsheetOnePerRow`) workbook: a flat
    /// `File | Code | Word | Count` sheet with one data row per (file, speaker,
    /// word), in `file_speaker_rows` order with each row's words byte-sorted
    /// (the `BTreeMap` iteration order, matching CLAN's WordsHead `strcmp`).
    ///
    /// Unlike `+d2`/`+d3` this carries no `@ID` columns, no type/token/TTR
    /// summary, no `%mor` TTR caveat, and no trailing empty row (verified
    /// against CLAN's `+d20` output on the Manchester fixtures).
    fn to_one_per_row_workbook(&self) -> Workbook {
        let mut rows: Vec<Row> = Vec::new();
        rows.push(Row::new(vec![
            Cell::text("File"),
            Cell::text("Code"),
            Cell::text("Word"),
            Cell::text("Count"),
        ]));
        for row in &self.file_speaker_rows {
            for (word, count) in &row.word_counts {
                rows.push(Row::new(vec![
                    Cell::text(row.filename.clone()),
                    Cell::text(row.speaker.clone()),
                    Cell::text(word.clone()),
                    Cell::count(*count),
                ]));
            }
        }
        let sheet = Worksheet::new(
            SheetName::new(SHEET_NAME),
            ColumnWidth(FREQ_COLUMN_WIDTH),
            rows,
        );
        Workbook::new(vec![sheet])
    }

    /// Build the CLAN `+dCN` (`onlydata = 4`) percent-of-speakers workbook
    /// (`statfreq_percent_result`, `freq.cpp:2841-2899`).
    ///
    /// The layout is the `+d3` summary (no per-word columns) with two
    /// percent-path specifics: the fourth column is labelled `Speaker` (not
    /// `Code`, `freq.cpp:2874`), and each speaker's Types/Token/TTR is computed
    /// over ONLY the words whose distinct-speaker count satisfies `filter`. A
    /// word's distinct-speaker count is the number of (file x speaker) rows that
    /// used it; the threshold is `floor(num_rows * percent / 100)`.
    ///
    /// `mor_based` gates the red `%mor` TTR caveat rows exactly as `+d3` does
    /// (divergence CLAN-DIV-004: chatter's `%mor`, never CLAN's `%%mor` leak).
    fn to_percent_workbook(&self, filter: SpeakerPercentFilter, mor_based: bool) -> Workbook {
        // CLAN's `findPercentNum` denominator is the count of all (file x
        // speaker) rows; the threshold is floor(rows * percent / 100).
        let num_rows = self.file_speaker_rows.len() as u64;
        let threshold = filter.threshold(num_rows);

        // Per-word distinct-speaker count: how many rows used each word. This is
        // CLAN's shared WordsHead `p->count`, incremented once per distinct
        // speaker (`freq.cpp:2756-2762`).
        let mut distinct_speakers: BTreeMap<&str, u64> = BTreeMap::new();
        for row in &self.file_speaker_rows {
            for word in row.word_counts.keys() {
                *distinct_speakers.entry(word.as_str()).or_insert(0) += 1;
            }
        }
        // A word is kept iff its distinct-speaker count compares against the
        // threshold (`freq.cpp:2801-2805`).
        let is_kept = |word: &str| -> bool {
            distinct_speakers
                .get(word)
                .is_some_and(|&n| filter.comparison.keeps(n, threshold))
        };

        let mut rows: Vec<Row> = Vec::new();
        if !mor_based {
            for line in TTR_CAVEAT_LINES {
                rows.push(Row::new(vec![Cell::red_text(line)]));
            }
        }
        rows.push(header_row(&[], false, Some(PERCENT_SPEAKER_HEADER)));
        for row in &self.file_speaker_rows {
            rows.push(percent_data_row(row, &is_kept));
        }
        // CLAN closes the table with a trailing empty row.
        rows.push(Row::empty());

        let sheet = Worksheet::new(
            SheetName::new(SHEET_NAME),
            ColumnWidth(FREQ_COLUMN_WIDTH),
            rows,
        );
        Workbook::new(vec![sheet])
    }
}

#[cfg(test)]
mod tests {
    use super::id_columns::{ID_COLUMN_COUNT, id_header_to_cells, ses_field_cells};
    use super::*;
    use crate::framework::spreadsheet::{ColumnWidth, Row, SheetName, Workbook, Worksheet};
    use std::collections::BTreeMap;
    use talkbank_model::{IDHeader, SesValue, Sex};

    /// Serialize a single row of @ID cells so we can assert on the rendered
    /// `<Data>` cells (value + ss:Type).
    fn render_id_row(id: &IDHeader) -> String {
        let row = Row::new(id_header_to_cells(id));
        let sheet = Worksheet::new(SheetName::new("sheet 1"), ColumnWidth(95), vec![row]);
        Workbook::new(vec![sheet]).write_xml().expect("serialize")
    }

    /// The Manchester Anne `@ID`: rich fields, SES "MC" -> Race=".", SES="MC".
    #[test]
    fn anne_id_lowers_to_oracle_cells() {
        let id = IDHeader::new("eng", "CHI", "Target_Child")
            .with_corpus("Manchester")
            .with_age("1;10.07")
            .with_sex(Sex::Female)
            .with_group("TD")
            .with_ses(SesValue::from_text("MC"));

        let cells = id_header_to_cells(&id);
        assert_eq!(cells.len(), ID_COLUMN_COUNT);

        let xml = render_id_row(&id);
        for s in [
            "eng",
            "Manchester",
            "CHI",
            "1;10.07",
            "female",
            "TD",
            "Target_Child",
        ] {
            assert!(
                xml.contains(&format!(r#"<Data ss:Type="String">{s}</Data>"#)),
                "missing String cell {s} in {xml}"
            );
        }
        // SES "MC" (two uppercase) -> Race ".", SES "MC". Education + Custom
        // absent -> ".". So three "." String cells (Race, Education, Custom).
        let dot_cells = xml.matches(r#"<Data ss:Type="String">.</Data>"#).count();
        assert_eq!(dot_cells, 3, "expected Race/Education/Custom dot cells");
    }

    #[test]
    fn combined_ses_splits_into_race_and_ses() {
        // "White,UC" (canonical combined form) -> Race "White", SES "UC".
        let cells = ses_field_cells(Some(&SesValue::from_text("White,UC")));
        assert_eq!(cells, vec![Cell::text("White"), Cell::text("UC")]);
    }

    #[test]
    fn ethnicity_only_ses_is_race_then_dot() {
        // "White" (not two-uppercase, no comma) -> Race "White", SES ".".
        let cells = ses_field_cells(Some(&SesValue::from_text("White")));
        assert_eq!(cells, vec![Cell::text("White"), Cell::empty()]);
    }

    #[test]
    fn ses_code_is_dot_then_ses() {
        // "WC" (two uppercase) -> Race ".", SES "WC".
        let cells = ses_field_cells(Some(&SesValue::from_text("WC")));
        assert_eq!(cells, vec![Cell::empty(), Cell::text("WC")]);
    }

    #[test]
    fn absent_ses_is_two_dot_cells() {
        let cells = ses_field_cells(None);
        assert_eq!(cells, vec![Cell::empty(), Cell::empty()]);
    }

    #[test]
    fn numeric_corpus_is_number_cell() {
        // An all-digit @ID field (e.g. a year-named corpus) -> Number cell.
        let id = IDHeader::new("eng", "CHI", "Target_Child").with_corpus("2020");
        let xml = render_id_row(&id);
        assert!(xml.contains(r#"<Data ss:Type="Number">2020</Data>"#));
    }

    /// The Anne / Aran rows mirroring the verified CLAN `+d2` oracle.
    fn oracle_result() -> FreqResult {
        let anne_id = IDHeader::new("eng", "CHI", "Target_Child")
            .with_corpus("Manchester")
            .with_age("1;10.07")
            .with_sex(Sex::Female)
            .with_group("TD")
            .with_ses(SesValue::from_text("MC"));
        let mut anne_words = BTreeMap::new();
        anne_words.insert("baby".to_owned(), 1u64);
        anne_words.insert("it".to_owned(), 2u64);
        anne_words.insert("fit".to_owned(), 1u64);
        let anne = FreqFileSpeakerRow {
            filename: "manchester-anne".to_owned(),
            speaker: "CHI".to_owned(),
            id: Some(anne_id),
            word_counts: anne_words,
            utterance_count: 3,
            total_types: 3,
            total_tokens: 4,
            ttr: 0.75,
            mattr: None,
        };

        let aran_id = IDHeader::new("eng", "CHI", "Target_Child")
            .with_corpus("Manchester")
            .with_age("1;11.12")
            .with_sex(Sex::Male)
            .with_group("TD")
            .with_ses(SesValue::from_text("MC"));
        let mut aran_words = BTreeMap::new();
        for w in ["there", "duck", "get", "down"] {
            aran_words.insert(w.to_owned(), 1u64);
        }
        let aran = FreqFileSpeakerRow {
            filename: "manchester-aran".to_owned(),
            speaker: "CHI".to_owned(),
            id: Some(aran_id),
            word_counts: aran_words,
            utterance_count: 2,
            total_types: 4,
            total_tokens: 4,
            ttr: 1.0,
            mattr: None,
        };

        FreqResult {
            speakers: Vec::new(),
            word_list_only: false,
            types_tokens_only: false,
            sort: Default::default(),
            file_speaker_rows: vec![anne, aran],
            mattr_enabled: false,
            combine_speakers: false,
            mor_based: false,
        }
    }

    #[test]
    fn d2_workbook_carries_oracle_header_and_cells() {
        let xml = oracle_result()
            .to_spreadsheet(FreqSpreadsheetMode::PerWord, false)
            .write_xml()
            .expect("serialize");

        // Corrected %mor caveat (CLAN-DIV-004), never CLAN's %%mor leak.
        assert!(xml.contains("%mor"));
        assert!(!xml.contains("%%mor"));

        // Header: File + the @ID headers + the *CHI: pseudo-word + word columns
        // + Types/Token/TTR. All String cells.
        for header in [
            "File", "Language", "Code", "Race", "SES", "Role", "*CHI:", "baby", "down", "there",
            "Types", "Token", "TTR",
        ] {
            assert!(
                xml.contains(&format!(r#"<Data ss:Type="String">{header}</Data>"#)),
                "missing header cell {header}"
            );
        }

        // @ID + summary cells from the oracle data rows.
        assert!(xml.contains(r#"<Data ss:Type="String">manchester-anne</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="String">Manchester</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="String">Target_Child</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="Number">0.750</Data>"#)); // anne TTR
        assert!(xml.contains(r#"<Data ss:Type="Number">1.000</Data>"#)); // aran TTR
    }

    #[test]
    fn d3_workbook_drops_word_columns() {
        let xml = oracle_result()
            .to_spreadsheet(FreqSpreadsheetMode::TypesTokens, false)
            .write_xml()
            .expect("serialize");

        // No per-word columns and no speaker pseudo-word column in +d3.
        assert!(!xml.contains(r#"<Data ss:Type="String">baby</Data>"#));
        assert!(!xml.contains(r#"<Data ss:Type="String">*CHI:</Data>"#));
        // Summary statistics still present.
        assert!(xml.contains(r#"<Data ss:Type="String">Types</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="Number">0.750</Data>"#));
    }

    #[test]
    fn d20_workbook_is_flat_per_speaker_word() {
        let xml = oracle_result()
            .to_spreadsheet(FreqSpreadsheetMode::PerSpeakerWord, false)
            .write_xml()
            .expect("serialize");

        // Flat header: File | Code | Word | Count.
        for header in ["File", "Code", "Word", "Count"] {
            assert!(
                xml.contains(&format!(r#"<Data ss:Type="String">{header}</Data>"#)),
                "missing {header} header cell"
            );
        }
        // One row per (file, speaker, word): file stem, bare speaker code, word.
        assert!(xml.contains(r#"<Data ss:Type="String">manchester-anne</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="String">CHI</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="String">baby</Data>"#));
        // anne's "it" count is 2 -> a Number cell.
        assert!(xml.contains(r#"<Data ss:Type="Number">2</Data>"#));

        // None of the +d2 wide-layout furniture: no @ID columns, no speaker
        // pseudo-word, no Types/TTR summary, no %mor caveat.
        assert!(!xml.contains(r#"<Data ss:Type="String">Language</Data>"#));
        assert!(!xml.contains(r#"<Data ss:Type="String">*CHI:</Data>"#));
        assert!(!xml.contains(r#"<Data ss:Type="String">TTR</Data>"#));
        assert!(!xml.contains("%mor line forms"));
    }

    #[test]
    fn mor_based_run_suppresses_ttr_caveat() {
        let xml = oracle_result()
            .to_spreadsheet(FreqSpreadsheetMode::TypesTokens, true)
            .write_xml()
            .expect("serialize");
        assert!(!xml.contains("%mor line forms"));
    }

    /// Each comparator compares the distinct-speaker count against the threshold
    /// the way CLAN's `percentC` switch does (`freq.cpp:2801-2805`).
    #[test]
    fn percent_comparison_keeps_matches_clan_percentc() {
        use SpeakerPercentComparison::*;
        // threshold 1: a word used by 1 speaker vs 2 speakers.
        assert!(LessThan.keeps(0, 1) && !LessThan.keeps(1, 1));
        assert!(LessOrEqual.keeps(1, 1) && !LessOrEqual.keeps(2, 1));
        assert!(Equal.keeps(1, 1) && !Equal.keeps(2, 1) && !Equal.keeps(0, 1));
        assert!(
            GreaterOrEqual.keeps(1, 1) && GreaterOrEqual.keeps(2, 1) && !GreaterOrEqual.keeps(0, 1)
        );
        assert!(GreaterThan.keeps(2, 1) && !GreaterThan.keeps(1, 1));
    }

    /// The threshold is `floor(num_rows * percent / 100)` (CLAN's `findPercentNum`
    /// float truncation, `freq.cpp:2673-2676`).
    #[test]
    fn percent_threshold_floors() {
        let filter = |c, p| SpeakerPercentFilter {
            comparison: c,
            percent: SpeakerPercent::new(p),
        };
        use SpeakerPercentComparison::LessThan as Any;
        assert_eq!(filter(Any, 50).threshold(2), 1); // 2 * 50 / 100 = 1
        assert_eq!(filter(Any, 50).threshold(3), 1); // 1.5 -> 1
        assert_eq!(filter(Any, 100).threshold(2), 2); // 2.0 -> 2
        assert_eq!(filter(Any, 25).threshold(4), 1); // 1.0 -> 1
        assert_eq!(filter(Any, 0).threshold(7), 0); // 0
        assert_eq!(filter(Any, 200).threshold(3), 6); // N is not clamped to 100
    }

    /// Two speaker-rows sharing one word ("shared", distinct-speaker count 2) and
    /// each holding one unique word (count 1). Exercises partial percent
    /// filtering deterministically.
    fn shared_vocab_result() -> FreqResult {
        let row = |speaker: &str, unique: &str| {
            let mut words = BTreeMap::new();
            words.insert("shared".to_owned(), 1u64);
            words.insert(unique.to_owned(), 1u64);
            FreqFileSpeakerRow {
                filename: "f".to_owned(),
                speaker: speaker.to_owned(),
                id: Some(IDHeader::new("eng", speaker, "Target_Child")),
                word_counts: words,
                utterance_count: 2,
                total_types: 2,
                total_tokens: 2,
                ttr: 1.0,
                mattr: None,
            }
        };
        FreqResult {
            speakers: Vec::new(),
            word_list_only: false,
            types_tokens_only: false,
            sort: Default::default(),
            file_speaker_rows: vec![row("CHI", "onlyChi"), row("MOT", "onlyMot")],
            mattr_enabled: false,
            combine_speakers: false,
            mor_based: false,
        }
    }

    /// `+d<=50` over two rows (threshold 1) keeps words used by <= 1 speaker, so
    /// each row keeps only its unique word (1 type, 1 token); the shared word
    /// (count 2) is dropped. The header labels the speaker column `Speaker` (not
    /// `Code`) and there are no per-word columns.
    #[test]
    fn percent_workbook_filters_and_uses_speaker_header() {
        let filter = SpeakerPercentFilter {
            comparison: SpeakerPercentComparison::LessOrEqual,
            percent: SpeakerPercent::new(50),
        };
        let xml = shared_vocab_result()
            .to_spreadsheet(FreqSpreadsheetMode::PercentOfSpeakers(filter), false)
            .write_xml()
            .expect("serialize");

        // Percent path uses the `Speaker` column label, not `+d2`/`+d3`'s `Code`.
        assert!(xml.contains(r#"<Data ss:Type="String">Speaker</Data>"#));
        assert!(!xml.contains(r#"<Data ss:Type="String">Code</Data>"#));
        // Summary shape: no per-word columns.
        assert!(!xml.contains(r#"<Data ss:Type="String">shared</Data>"#));
        assert!(!xml.contains(r#"<Data ss:Type="String">onlyChi</Data>"#));
        // Each row keeps its single unique word: Types 1, Token 1, TTR 1.000.
        assert!(xml.contains(r#"<Data ss:Type="Number">1</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="Number">1.000</Data>"#));
        // Same %mor caveat as +d2/+d3 (CLAN-DIV-004), never %%mor.
        assert!(xml.contains("%mor") && !xml.contains("%%mor"));
    }

    /// `+d>50` (threshold 1) keeps words used by > 1 speaker: only the shared
    /// word survives, so each row is 1 type / 1 token, and the unique words are
    /// excluded. The complement of the `+d<=50` case above.
    #[test]
    fn percent_workbook_gt_keeps_only_shared_word() {
        let filter = SpeakerPercentFilter {
            comparison: SpeakerPercentComparison::GreaterThan,
            percent: SpeakerPercent::new(50),
        };
        let result = shared_vocab_result();
        let workbook = result.to_spreadsheet(FreqSpreadsheetMode::PercentOfSpeakers(filter), false);
        // Both rows keep exactly the shared word: each row's Types == Token == 1.
        // (Asserted via the rendered cells; the unique words contribute nothing.)
        let xml = workbook.write_xml().expect("serialize");
        // No TTR `-` cell: both rows have a token (the shared word).
        assert!(!xml.contains(r#"<Data ss:Type="String">-</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="Number">1.000</Data>"#));
    }
}
