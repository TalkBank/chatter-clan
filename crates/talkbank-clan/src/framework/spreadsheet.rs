//! Typed SpreadsheetML (Excel 2003 XML) model and writer.
//!
//! A few CLAN commands emit a spreadsheet FILE rather than text to stdout:
//! FREQ `+d2` / `+d3` write an Excel-openable `.xls` whose payload is the
//! "SpreadsheetML 2003" XML dialect (`urn:schemas-microsoft-com:office:spreadsheet`).
//! This module is the typed, pre-serialization model for that format, lowered
//! once to XML at the boundary via [`Workbook::write_xml`].
//!
//! ## Parity standard: semantic equivalence, not byte equality
//!
//! Per `crates/talkbank-clan/CLAUDE.md`, spreadsheet parity is judged on the
//! **parsed cells** (sheet name, headers, rows, cell values and their
//! String-vs-Number type), NOT on byte-identical XML. CLAN's `.xls` carries
//! window-geometry and styling boilerplate that is not semantically meaningful;
//! this writer reproduces the same cell grid and the styles cells actually
//! reference (`RedText`), and produces a well-formed, Excel-openable document.
//!
//! ## Number vs text
//!
//! CLAN decides a cell's `ss:Type` by sniffing the rendered string at emit time
//! (`isAllDigit`). chatter decides it from the typed source at model
//! construction: a `WordCount`/`TypeCount` or a ratio is a [`CellValue::Number`];
//! a speaker code, word, `@ID` field, or empty placeholder is text. This is
//! strictly more principled and produces the same parsed cells.

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

/// Worksheet name. Excel truncates sheet names to 31 characters; CLAN's
/// `excelHeader` truncates at 30, so we follow CLAN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetName(String);

impl SheetName {
    /// Maximum sheet-name length CLAN emits (`excelHeader`, `cutt.cpp`).
    const MAX_LEN: usize = 30;

    /// Build a sheet name, truncating to `Self::MAX_LEN` characters.
    pub fn new(name: &str) -> Self {
        Self(name.chars().take(Self::MAX_LEN).collect())
    }

    /// The (possibly truncated) sheet name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Default column width for a worksheet, in Excel's point units. CLAN's FREQ
/// spreadsheet uses 95 (`excelHeader(fp, name, 95)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnWidth(pub u32);

/// Number of digits a numeric cell renders after the decimal point. Counts use
/// 0 (`%.0f` in CLAN); a type/token ratio uses 3 (`%.3f`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalPlaces(pub u8);

/// The typed value of a single spreadsheet cell.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// A text cell (`ss:Type="String"`): words, speaker codes, `@ID` fields.
    Text(String),
    /// A numeric cell (`ss:Type="Number"`), rendered with a fixed number of
    /// decimal places.
    Number {
        /// The numeric value (counts are exact in `f64`; ratios are computed).
        value: f64,
        /// How many decimal places to render.
        decimals: DecimalPlaces,
    },
    /// CLAN's empty-field placeholder: a `.` rendered as a String cell. Used
    /// for absent `@ID` fields (`excelOutputID` emits `.` for empties).
    Empty,
}

impl CellValue {
    /// `ss:Type` attribute value for this cell.
    fn type_attr(&self) -> &'static str {
        match self {
            CellValue::Number { .. } => "Number",
            CellValue::Text(_) | CellValue::Empty => "String",
        }
    }

    /// The rendered cell text (the `<Data>` element body).
    fn render(&self) -> String {
        match self {
            CellValue::Text(s) => s.clone(),
            CellValue::Empty => ".".to_owned(),
            CellValue::Number { value, decimals } => {
                format!("{value:.*}", decimals.0 as usize)
            }
        }
    }
}

/// A named cell style. Only the styles CLAN's FREQ spreadsheet references are
/// modeled; [`CellStyle::Default`] emits no `ss:StyleID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellStyle {
    /// No explicit style.
    Default,
    /// Red font, used for the `+d2`/`+d3` TTR-caveat advisory rows.
    RedText,
    /// Bottom-aligned wrapped text (declared by CLAN; reserved for parity).
    TallText,
}

impl CellStyle {
    /// The `ss:ID` of this style, or `None` for the default (unstyled) cell.
    fn style_id(self) -> Option<&'static str> {
        match self {
            CellStyle::Default => None,
            CellStyle::RedText => Some("RedText"),
            CellStyle::TallText => Some("TallText"),
        }
    }
}

/// A single spreadsheet cell: a value plus an optional style.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    value: CellValue,
    style: CellStyle,
}

impl Cell {
    /// A plain text cell.
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            value: CellValue::Text(s.into()),
            style: CellStyle::Default,
        }
    }

    /// CLAN's empty-`@ID`-field placeholder (`.`).
    pub fn empty() -> Self {
        Self {
            value: CellValue::Empty,
            style: CellStyle::Default,
        }
    }

    /// An integer-count cell (`%.0f`): word frequencies, types, tokens,
    /// utterance counts.
    pub fn count(n: u64) -> Self {
        Self {
            value: CellValue::Number {
                value: n as f64,
                decimals: DecimalPlaces(0),
            },
            style: CellStyle::Default,
        }
    }

    /// A type/token-ratio cell (`%.3f`).
    pub fn ratio(r: f64) -> Self {
        Self {
            value: CellValue::Number {
                value: r,
                decimals: DecimalPlaces(3),
            },
            style: CellStyle::Default,
        }
    }

    /// A red-styled advisory text cell (TTR caveat rows).
    pub fn red_text(s: impl Into<String>) -> Self {
        Self {
            value: CellValue::Text(s.into()),
            style: CellStyle::RedText,
        }
    }
}

/// A spreadsheet row. An empty row (`cells` is empty) serializes as CLAN's
/// `<Row></Row>`, which appears as the trailing blank row in FREQ output.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    cells: Vec<Cell>,
}

impl Row {
    /// A row of cells.
    pub fn new(cells: Vec<Cell>) -> Self {
        Self { cells }
    }

    /// CLAN's trailing empty row.
    pub fn empty() -> Self {
        Self { cells: Vec::new() }
    }
}

/// A single worksheet: a named table of rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Worksheet {
    name: SheetName,
    column_width: ColumnWidth,
    rows: Vec<Row>,
}

impl Worksheet {
    /// Build a worksheet.
    pub fn new(name: SheetName, column_width: ColumnWidth, rows: Vec<Row>) -> Self {
        Self {
            name,
            column_width,
            rows,
        }
    }
}

/// A SpreadsheetML workbook: one or more worksheets. Lowered to XML once by
/// [`Workbook::write_xml`].
#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    sheets: Vec<Worksheet>,
}

/// Failure to serialize a [`Workbook`] to SpreadsheetML.
#[derive(Debug, thiserror::Error)]
pub enum SpreadsheetError {
    /// The underlying `quick-xml` writer (writing into an in-memory buffer)
    /// returned an I/O error.
    #[error("spreadsheet XML serialization failed: {0}")]
    Write(#[from] std::io::Error),
    /// The serialized bytes were not valid UTF-8 (should be impossible given
    /// our inputs, surfaced rather than panicked).
    #[error("spreadsheet XML was not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

impl Workbook {
    /// Build a workbook from its worksheets.
    pub fn new(sheets: Vec<Worksheet>) -> Self {
        Self { sheets }
    }

    /// Serialize to a SpreadsheetML 2003 XML document.
    ///
    /// The output is compact (no indentation) so callers and tests can match
    /// `<Data ss:Type="Number">…</Data>` substrings; Excel and the cell parser
    /// are whitespace-insensitive.
    pub fn write_xml(&self) -> Result<String, SpreadsheetError> {
        let mut w = Writer::new(Vec::new());
        w.write_event(Event::Decl(BytesDecl::new("1.0", None, None)))?;
        self.write_workbook(&mut w)?;
        Ok(String::from_utf8(w.into_inner())?)
    }

    /// Emit `<Workbook>` with its namespaces, the boilerplate preamble, the
    /// styles cells reference, and every worksheet.
    fn write_workbook(&self, w: &mut Writer<Vec<u8>>) -> Result<(), SpreadsheetError> {
        let mut root = BytesStart::new("Workbook");
        root.push_attribute(("xmlns", "urn:schemas-microsoft-com:office:spreadsheet"));
        root.push_attribute(("xmlns:o", "urn:schemas-microsoft-com:office:office"));
        root.push_attribute(("xmlns:x", "urn:schemas-microsoft-com:office:excel"));
        root.push_attribute(("xmlns:ss", "urn:schemas-microsoft-com:office:spreadsheet"));
        root.push_attribute(("xmlns:html", "http://www.w3.org/TR/REC-html40"));
        w.write_event(Event::Start(root))?;

        write_preamble(w)?;
        write_styles(w)?;
        for sheet in &self.sheets {
            write_worksheet(w, sheet)?;
        }

        w.write_event(Event::End(BytesEnd::new("Workbook")))?;
        Ok(())
    }
}

/// The `<ExcelWorkbook>` window/protection boilerplate. Cosmetic, included so
/// the file opens cleanly in Excel; values mirror CLAN's `excelHeader`.
fn write_preamble(w: &mut Writer<Vec<u8>>) -> Result<(), SpreadsheetError> {
    let mut excel = BytesStart::new("ExcelWorkbook");
    excel.push_attribute(("xmlns", "urn:schemas-microsoft-com:office:excel"));
    w.write_event(Event::Start(excel))?;
    write_text_element(w, "WindowHeight", "15260")?;
    write_text_element(w, "WindowWidth", "25600")?;
    w.write_event(Event::Empty(BytesStart::new("Date1904")))?;
    write_text_element(w, "ProtectStructure", "False")?;
    write_text_element(w, "ProtectWindows", "False")?;
    w.write_event(Event::End(BytesEnd::new("ExcelWorkbook")))?;
    Ok(())
}

/// The `<Styles>` block. Only `RedText` (TTR-caveat rows) and `TallText` are
/// declared, matching CLAN.
fn write_styles(w: &mut Writer<Vec<u8>>) -> Result<(), SpreadsheetError> {
    w.write_event(Event::Start(BytesStart::new("Styles")))?;

    let mut red = BytesStart::new("Style");
    red.push_attribute(("ss:ID", "RedText"));
    w.write_event(Event::Start(red))?;
    let mut font = BytesStart::new("Font");
    font.push_attribute(("ss:FontName", "Calibri"));
    font.push_attribute(("ss:Size", "12"));
    font.push_attribute(("ss:Color", "#FF0000"));
    w.write_event(Event::Empty(font))?;
    w.write_event(Event::End(BytesEnd::new("Style")))?;

    let mut tall = BytesStart::new("Style");
    tall.push_attribute(("ss:ID", "TallText"));
    w.write_event(Event::Start(tall))?;
    let mut align = BytesStart::new("Alignment");
    align.push_attribute(("ss:Vertical", "Bottom"));
    align.push_attribute(("ss:WrapText", "1"));
    w.write_event(Event::Empty(align))?;
    w.write_event(Event::End(BytesEnd::new("Style")))?;

    w.write_event(Event::End(BytesEnd::new("Styles")))?;
    Ok(())
}

/// Emit one `<Worksheet>` / `<Table>` with its rows.
fn write_worksheet(w: &mut Writer<Vec<u8>>, sheet: &Worksheet) -> Result<(), SpreadsheetError> {
    let mut ws = BytesStart::new("Worksheet");
    ws.push_attribute(("ss:Name", sheet.name.as_str()));
    w.write_event(Event::Start(ws))?;

    let mut table = BytesStart::new("Table");
    table.push_attribute((
        "ss:DefaultColumnWidth",
        sheet.column_width.0.to_string().as_str(),
    ));
    table.push_attribute(("ss:DefaultRowHeight", "15"));
    w.write_event(Event::Start(table))?;

    for row in &sheet.rows {
        write_row(w, row)?;
    }

    w.write_event(Event::End(BytesEnd::new("Table")))?;
    w.write_event(Event::End(BytesEnd::new("Worksheet")))?;
    Ok(())
}

/// Emit one `<Row>` and its cells (empty rows serialize as `<Row></Row>`).
fn write_row(w: &mut Writer<Vec<u8>>, row: &Row) -> Result<(), SpreadsheetError> {
    w.write_event(Event::Start(BytesStart::new("Row")))?;
    for cell in &row.cells {
        write_cell(w, cell)?;
    }
    w.write_event(Event::End(BytesEnd::new("Row")))?;
    Ok(())
}

/// Emit one `<Cell><Data ss:Type=…>value</Data></Cell>`, with an optional
/// `ss:StyleID`.
fn write_cell(w: &mut Writer<Vec<u8>>, cell: &Cell) -> Result<(), SpreadsheetError> {
    let mut cell_start = BytesStart::new("Cell");
    if let Some(style_id) = cell.style.style_id() {
        cell_start.push_attribute(("ss:StyleID", style_id));
    }
    w.write_event(Event::Start(cell_start))?;

    let mut data = BytesStart::new("Data");
    data.push_attribute(("ss:Type", cell.value.type_attr()));
    w.write_event(Event::Start(data))?;
    w.write_event(Event::Text(BytesText::new(&cell.value.render())))?;
    w.write_event(Event::End(BytesEnd::new("Data")))?;

    w.write_event(Event::End(BytesEnd::new("Cell")))?;
    Ok(())
}

/// Emit `<Name>text</Name>` for the preamble's scalar elements.
fn write_text_element(
    w: &mut Writer<Vec<u8>>,
    name: &str,
    text: &str,
) -> Result<(), SpreadsheetError> {
    w.write_event(Event::Start(BytesStart::new(name)))?;
    w.write_event(Event::Text(BytesText::new(text)))?;
    w.write_event(Event::End(BytesEnd::new(name)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal workbook mirroring the shape of a FREQ `+d3` summary row.
    fn sample() -> Workbook {
        let header = Row::new(vec![
            Cell::text("File"),
            Cell::text("Types"),
            Cell::text("Token"),
            Cell::text("TTR"),
        ]);
        let data = Row::new(vec![
            Cell::text("manchester-anne"),
            Cell::count(3),
            Cell::count(4),
            Cell::ratio(0.75),
        ]);
        let sheet = Worksheet::new(
            SheetName::new("sheet 1"),
            ColumnWidth(95),
            vec![header, data, Row::empty()],
        );
        Workbook::new(vec![sheet])
    }

    #[test]
    fn renders_well_formed_spreadsheetml() {
        let xml = sample().write_xml().expect("serialize");
        assert!(xml.starts_with("<?xml version=\"1.0\"?>"));
        assert!(xml.contains("xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\""));
        assert!(xml.contains("<Worksheet ss:Name=\"sheet 1\">"));
        assert!(xml.contains("<Table ss:DefaultColumnWidth=\"95\""));
        assert!(xml.ends_with("</Workbook>"));
    }

    #[test]
    fn count_cells_are_number_typed_integers() {
        let xml = sample().write_xml().expect("serialize");
        assert!(xml.contains(r#"<Data ss:Type="Number">3</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="Number">4</Data>"#));
    }

    #[test]
    fn ratio_cell_renders_three_decimals() {
        let xml = sample().write_xml().expect("serialize");
        // 0.75 -> "0.750" (CLAN's %.3f), and Number-typed.
        assert!(xml.contains(r#"<Data ss:Type="Number">0.750</Data>"#));
    }

    #[test]
    fn text_cells_are_string_typed() {
        let xml = sample().write_xml().expect("serialize");
        assert!(xml.contains(r#"<Data ss:Type="String">manchester-anne</Data>"#));
        assert!(xml.contains(r#"<Data ss:Type="String">File</Data>"#));
    }

    #[test]
    fn empty_cell_renders_dot_string() {
        let row = Row::new(vec![Cell::empty()]);
        let sheet = Worksheet::new(SheetName::new("sheet 1"), ColumnWidth(95), vec![row]);
        let xml = Workbook::new(vec![sheet]).write_xml().expect("serialize");
        assert!(xml.contains(r#"<Data ss:Type="String">.</Data>"#));
    }

    #[test]
    fn red_style_emitted_and_referenced() {
        let row = Row::new(vec![Cell::red_text("caveat about %mor")]);
        let sheet = Worksheet::new(SheetName::new("sheet 1"), ColumnWidth(95), vec![row]);
        let xml = Workbook::new(vec![sheet]).write_xml().expect("serialize");
        assert!(xml.contains(r#"<Style ss:ID="RedText">"#));
        assert!(xml.contains(r#"<Cell ss:StyleID="RedText">"#));
        // Single percent, never CLAN's %%mor printf leak (CLAN-DIV-004).
        assert!(xml.contains("%mor"));
        assert!(!xml.contains("%%mor"));
    }

    #[test]
    fn zero_count_is_number_zero() {
        let row = Row::new(vec![Cell::count(0)]);
        let sheet = Worksheet::new(SheetName::new("sheet 1"), ColumnWidth(95), vec![row]);
        let xml = Workbook::new(vec![sheet]).write_xml().expect("serialize");
        assert!(xml.contains(r#"<Data ss:Type="Number">0</Data>"#));
    }

    #[test]
    fn sheet_name_truncated_to_thirty_chars() {
        let long = "x".repeat(40);
        let name = SheetName::new(&long);
        assert_eq!(name.as_str().chars().count(), 30);
    }
}
