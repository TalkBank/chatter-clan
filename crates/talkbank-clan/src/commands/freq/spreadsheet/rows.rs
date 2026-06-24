//! Row builders for the FREQ `+d2`/`+d3`/`+dCN` spreadsheets.
//!
//! The header row, per-(file x speaker) data rows (`+d2`/`+d3` and the `+dCN`
//! percent variant), the per-word column union, and the Types/Token/TTR/MATTR
//! cells. Extracted verbatim from `freq/spreadsheet.rs`; the parent's
//! `impl FreqResult` workbook builders call these by name through the
//! `use rows::*;` re-export.

use std::collections::BTreeSet;

use crate::framework::spreadsheet::{Cell, Row};

use super::id_columns::{ID_COLUMN_COUNT, ID_COLUMN_HEADERS, id_header_to_cells};
use super::{FreqFileSpeakerRow, FreqSpreadsheetMode};

/// Index of the speaker column within [`ID_COLUMN_HEADERS`] (`Code` for the
/// `+d2`/`+d3` path). [`header_row`]'s `speaker_label` override relabels this
/// column to `Speaker` for the `+dCN` percent path.
const SPEAKER_COLUMN_INDEX: usize = 2;

/// The `+dCN` percent path's label for the speaker column (`freq.cpp:2874`
/// writes `Speaker`, where `+d2`/`+d3` write `Code`).
pub(super) const PERCENT_SPEAKER_HEADER: &str = "Speaker";

/// One (file x speaker) percent-path data row: the `@ID` value cells (identical
/// to the `+d2`/`+d3` values), then Types/Token/TTR recomputed over the words
/// `is_kept` accepts (CLAN accumulates `diff`/`total`, `freq.cpp:2816-2818`).
pub(super) fn percent_data_row(row: &FreqFileSpeakerRow, is_kept: &impl Fn(&str) -> bool) -> Row {
    let mut types: u64 = 0;
    let mut tokens: u64 = 0;
    for (word, count) in &row.word_counts {
        if is_kept(word) {
            types += 1;
            tokens += *count;
        }
    }
    let mut cells = Vec::with_capacity(1 + ID_COLUMN_COUNT + 3);
    cells.push(Cell::text(row.filename.clone()));
    cells.extend(row_id_cells(row));
    cells.push(Cell::count(types));
    cells.push(Cell::count(tokens));
    // TTR = Types/Token, or CLAN's `-` for a zero-token speaker (`freq.cpp:2890`).
    let ratio = if tokens == 0 {
        0.0
    } else {
        types as f64 / tokens as f64
    };
    cells.push(ttr_cell_from(tokens, ratio));
    Row::new(cells)
}

/// The byte-sorted union of every row's speaker pseudo-word column (`*CHI:`)
/// and word-form columns, reproducing CLAN's WordsHead BST order (`strcmp`).
pub(super) fn word_columns(rows: &[FreqFileSpeakerRow]) -> Vec<String> {
    let mut cols: BTreeSet<String> = BTreeSet::new();
    for row in rows {
        cols.insert(speaker_label(&row.speaker));
        for word in row.word_counts.keys() {
            cols.insert(word.clone());
        }
    }
    cols.into_iter().collect()
}

/// CLAN's `*SPEAKER:` pseudo-word column label for a speaker code.
fn speaker_label(speaker: &str) -> String {
    format!("*{speaker}:")
}

/// The header row: `File`, the eleven `@ID` headers, the `+d2` word columns,
/// then `Types`, `Token`, `TTR`, and `MATTR` when `+bN` is active.
///
/// `speaker_label` overrides the speaker column header: `None` keeps
/// `ID_COLUMN_HEADERS`'s `Code` (`+d2`/`+d3`/`+d20`); `Some("Speaker")` is the
/// `+dCN` percent path's relabel (`freq.cpp:2874`).
pub(super) fn header_row(word_columns: &[String], mattr: bool, speaker_label: Option<&str>) -> Row {
    let mut cells = Vec::with_capacity(1 + ID_COLUMN_COUNT + word_columns.len() + 4);
    cells.push(Cell::text("File"));
    for (index, header) in ID_COLUMN_HEADERS.iter().enumerate() {
        let label = match speaker_label {
            Some(label) if index == SPEAKER_COLUMN_INDEX => label,
            _ => header,
        };
        cells.push(Cell::text(label));
    }
    for col in word_columns {
        cells.push(Cell::text(col.clone()));
    }
    cells.push(Cell::text("Types"));
    cells.push(Cell::text("Token"));
    cells.push(Cell::text("TTR"));
    if mattr {
        cells.push(Cell::text("MATTR"));
    }
    Row::new(cells)
}

/// One (file x speaker) data row.
pub(super) fn data_row(
    row: &FreqFileSpeakerRow,
    word_columns: &[String],
    mode: FreqSpreadsheetMode,
    mattr: bool,
) -> Row {
    let mut cells = Vec::with_capacity(1 + ID_COLUMN_COUNT + word_columns.len() + 4);
    cells.push(Cell::text(row.filename.clone()));
    cells.extend(row_id_cells(row));
    if mode == FreqSpreadsheetMode::PerWord {
        let label = speaker_label(&row.speaker);
        for col in word_columns {
            // This row's own speaker pseudo-word holds its utterance count;
            // other speakers' pseudo-words are absent (0), as are unused words.
            let count = if *col == label {
                row.utterance_count
            } else {
                row.word_counts.get(col).copied().unwrap_or(0)
            };
            cells.push(Cell::count(count));
        }
    }
    cells.push(Cell::count(row.total_types));
    cells.push(Cell::count(row.total_tokens));
    cells.push(ttr_cell(row));
    if mattr {
        cells.push(mattr_cell(row));
    }
    Row::new(cells)
}

/// The `@ID` columns for a row, or CLAN's `.|.|<sp>|.|...` fallback (Code only,
/// other columns `.`) when the speaker has no `@ID` for the file.
fn row_id_cells(row: &FreqFileSpeakerRow) -> Vec<Cell> {
    match &row.id {
        Some(id) => id_header_to_cells(id),
        None => {
            let mut cells = vec![Cell::empty(); ID_COLUMN_COUNT];
            // Column order is Language, Corpus, Code, ...; Code is index 2.
            cells[2] = Cell::text(row.speaker.clone());
            cells
        }
    }
}

/// The TTR cell from a token count and ratio: the ratio at three decimals, or
/// CLAN's `-` when there are no tokens (its `@ST:` writer prints `-` for a
/// zero-token speaker). Shared by the `+d2`/`+d3` path (precomputed `row.ttr`)
/// and the `+dCN` percent path (a ratio recomputed over the filtered subset);
/// `ratio` is ignored when `tokens == 0`.
fn ttr_cell_from(tokens: u64, ratio: f64) -> Cell {
    if tokens == 0 {
        Cell::text("-")
    } else {
        Cell::ratio(ratio)
    }
}

/// The TTR cell for a row's precomputed totals (`+d2`/`+d3`/`+d20`).
fn ttr_cell(row: &FreqFileSpeakerRow) -> Cell {
    ttr_cell_from(row.total_tokens, row.ttr)
}

/// The MATTR cell: the moving average at three decimals (`%.3f`), or CLAN's `-`
/// when MATTR is undefined for the speaker (`NMATTRs == 0`, i.e. fewer than `N`
/// tokens), matching the `@ST:` writer (`freq.cpp:1558-1562`).
fn mattr_cell(row: &FreqFileSpeakerRow) -> Cell {
    match row.mattr {
        Some(mattr) => Cell::ratio(mattr.value()),
        None => Cell::text("-"),
    }
}
