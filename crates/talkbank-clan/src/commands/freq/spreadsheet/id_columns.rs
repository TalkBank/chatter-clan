//! The `@ID`-derived spreadsheet columns for FREQ `+d2`/`+d3`.
//!
//! Lowers one parsed `@ID` header into its fixed [`ID_COLUMN_COUNT`] cells
//! ([`id_header_to_cells`]), reproducing CLAN's `excelOutputID`
//! (`cutt.cpp:11230`). Extracted verbatim from `freq/spreadsheet.rs`; the parent
//! re-exports the public items so the workbook builders continue to reference
//! them by name.

use talkbank_model::{IDHeader, SesValue};

use crate::framework::spreadsheet::Cell;

/// Number of `@ID`-derived columns in the FREQ spreadsheet: the ten `@ID`
/// fields, with the SES slot split into Race + SES.
pub(super) const ID_COLUMN_COUNT: usize = 11;

/// The fixed `@ID` column headers CLAN emits, in order (`freq.cpp:3301`). The
/// SES `@ID` field expands to the two columns `Race` + `SES`, so there are 11
/// headers for 10 `@ID` fields.
pub(super) const ID_COLUMN_HEADERS: [&str; ID_COLUMN_COUNT] = [
    "Language",
    "Corpus",
    "Code",
    "Age",
    "Sex",
    "Group",
    "Race",
    "SES",
    "Role",
    "Education",
    "Custom field",
];

/// Lower one parsed `@ID` header to its [`ID_COLUMN_COUNT`] spreadsheet cells,
/// reproducing CLAN's `excelOutputID` (`cutt.cpp:11230`): one cell per `@ID`
/// field, empty fields rendered as `.`, all-digit fields rendered as Number
/// cells, and the SES field split into a Race cell and an SES cell.
pub(super) fn id_header_to_cells(id: &IDHeader) -> Vec<Cell> {
    let language = language_field(id);
    let mut cells = Vec::with_capacity(ID_COLUMN_COUNT);
    cells.push(id_field_cell(Some(&language)));
    cells.push(id_field_cell(Some(id.corpus.as_str())));
    cells.push(id_field_cell(Some(id.speaker.as_str())));
    cells.push(id_field_cell(id.age.as_ref().map(|a| a.as_str())));
    cells.push(id_field_cell(id.sex.as_ref().map(|s| s.as_str())));
    cells.push(id_field_cell(id.group.as_ref().map(|g| g.as_str())));
    cells.extend(ses_field_cells(id.ses.as_ref()));
    cells.push(id_field_cell(Some(id.role.as_str())));
    cells.push(id_field_cell(id.education.as_ref().map(|e| e.as_str())));
    cells.push(id_field_cell(id.custom_field.as_ref().map(|c| c.as_str())));
    debug_assert_eq!(cells.len(), ID_COLUMN_COUNT);
    cells
}

/// The Language `@ID` field as CLAN renders it: the comma-space-joined language
/// codes (e.g. `eng` or `spa, ara, fra`), matching `LanguageCodes`'s CHAT form.
fn language_field(id: &IDHeader) -> String {
    id.language
        .iter()
        .map(|code| code.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// One non-SES `@ID` field cell, per CLAN's `excelOutputID`: an absent or empty
/// field is the `.` placeholder; an all-digit field is a Number cell; anything
/// else is a String cell.
fn id_field_cell(field: Option<&str>) -> Cell {
    let Some(field) = field.filter(|s| !s.is_empty()) else {
        return Cell::empty();
    };
    // CLAN renders all-digit fields as Number cells. We model integer @ID
    // values (the realistic case: numeric corpus / group codes); the
    // (non-occurring) all-digit-with-decimal @ID field falls back to text.
    if is_all_digit_clan(field)
        && let Ok(n) = field.parse::<u64>()
    {
        return Cell::count(n);
    }
    Cell::text(field)
}

/// The SES `@ID` field split into (Race, SES) cells, reproducing CLAN's
/// `excelOutputID` `cnt == 6` branch (`cutt.cpp`): an all-digit value is a
/// single Number cell; a comma splits into Race,SES; exactly two uppercase
/// letters (followed by end or space) become `.` + value; anything else is
/// value + `.`. An absent SES yields two `.` cells.
///
/// We classify the SES *value string* ([`SesValue::as_str`], which renders the
/// `Combined` variant with a comma) rather than the raw `@ID` bytes; the only
/// divergence is a space-separated combined SES (`White UC`), which chatter has
/// already canonicalized to `White,UC` at parse time.
pub(super) fn ses_field_cells(ses: Option<&SesValue>) -> Vec<Cell> {
    let raw = ses.map(|s| s.as_str()).unwrap_or_default();
    if raw.is_empty() {
        return vec![Cell::empty(), Cell::empty()];
    }
    if is_all_digit_clan(&raw)
        && let Ok(n) = raw.parse::<u64>()
    {
        // Degenerate CLAN case: an all-digit SES is a single Number cell.
        return vec![Cell::count(n)];
    }
    if let Some((race, ses_code)) = raw.split_once(',') {
        return vec![Cell::text(race), Cell::text(ses_code)];
    }
    if is_two_uppercase(&raw) {
        return vec![Cell::empty(), Cell::text(raw)];
    }
    vec![Cell::text(raw), Cell::empty()]
}

/// CLAN's `isAllDigit` (`cutt.cpp`): every character is a digit or `.`, and at
/// least one digit is present (so `.` alone is not numeric).
fn is_all_digit_clan(s: &str) -> bool {
    let mut saw_digit = false;
    for b in s.bytes() {
        if b == b'.' {
            continue;
        }
        if !b.is_ascii_digit() {
            return false;
        }
        saw_digit = true;
    }
    saw_digit
}

/// CLAN's SES two-letter test (`excelOutputID`): the first two characters are
/// uppercase ASCII letters, and the third is the end of the string or a space.
fn is_two_uppercase(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 2
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_uppercase()
        && (b.len() == 2 || b[2].is_ascii_whitespace())
}
