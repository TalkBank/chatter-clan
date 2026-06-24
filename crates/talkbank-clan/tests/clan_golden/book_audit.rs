// Parser for the per-command CLAN audit tables in the book, the DENOMINATOR
// of the golden-parity completeness metric (Phase 2).
//
// Each `book/src/clan-reference/commands/<cmd>.md` page carries one or more
// flag tables with the header `| CLAN flag | Meaning | Chatter | Status |
// Notes |`. This module parses those tables into typed `FlagRow`s. It uses the
// comrak GFM parser rather than splitting on `|`, so cell markdown (the
// `+k` backtick spans, escaped pipes) is handled structurally.

use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, parse_document};

/// The Status a command page CLAIMS for a flag-row. Hand-authored in the book;
/// the metric cross-checks these claims against golden-proven rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BookStatus {
    Done,
    Partial,
    RewriterOnly,
    Missing,
    /// Intentionally not implemented, with a source-grounded reason on the page
    /// (the project's third terminal "done" classification alongside matches and
    /// diverged). Distinct from `Missing` (an open gap): a `Deferred` row is a
    /// documented non-goal, so the metric excludes it from the provable
    /// denominator rather than counting it as outstanding work.
    Deferred,
}

impl BookStatus {
    /// Canonicalize a Status cell. Cells carry suffixes ("Done (no-op per
    /// CLAN)", "Rewriter only"), so match on the leading word. Returns `None`
    /// for an unrecognized status so the caller can surface book drift rather
    /// than silently bucketing it.
    fn parse(cell: &str) -> Option<Self> {
        let cell = cell.trim();
        if cell.starts_with("Done") {
            Some(Self::Done)
        } else if cell.starts_with("Partial") {
            Some(Self::Partial)
        } else if cell.starts_with("Rewriter") {
            Some(Self::RewriterOnly)
        } else if cell.starts_with("Missing") {
            Some(Self::Missing)
        } else if cell.starts_with("Deferred") {
            Some(Self::Deferred)
        } else {
            None
        }
    }
}

/// One CLAN flag-row from a command page's audit table.
#[derive(Debug, Clone)]
pub(crate) struct FlagRow {
    /// The CLAN flag token exactly as written in the first backtick cell,
    /// e.g. "+k", "+c / +c0", "+t*X".
    pub(crate) flag: String,
    /// The status the page claims for it.
    pub(crate) status: BookStatus,
}

/// The header that marks a flag-status table, distinguishing it from legend
/// or "confirmed-broken invocation" tables that share a page.
const AUDIT_HEADER: [&str; 5] = ["CLAN flag", "Meaning", "Chatter", "Status", "Notes"];

/// Zero-based index of the Status column in [`AUDIT_HEADER`].
const STATUS_COLUMN: usize = 3;

/// Concatenate the plain text of a table cell (Text + inline `Code`), trimmed.
fn cell_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut out = String::new();
    for descendant in node.descendants() {
        match &descendant.data.borrow().value {
            NodeValue::Text(text) => out.push_str(text),
            NodeValue::Code(code) => out.push_str(&code.literal),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// A table row's cells as trimmed text, in column order.
fn row_cells<'a>(row: &'a AstNode<'a>) -> Vec<String> {
    row.children()
        .filter(|cell| matches!(cell.data.borrow().value, NodeValue::TableCell))
        .map(cell_text)
        .collect()
}

/// Parse every audit-table flag-row from a command page's markdown. Tables
/// without the audit header are ignored; rows whose Status does not
/// canonicalize are dropped (use [`parse_audit_rows_strict`] to see them).
pub(crate) fn parse_audit_rows(markdown: &str) -> Vec<FlagRow> {
    parse_audit_rows_strict(markdown).0
}

/// Like [`parse_audit_rows`] but also returns the raw Status strings that did
/// NOT canonicalize, so a test can assert the book carries no unrecognized
/// status (drift detection).
pub(crate) fn parse_audit_rows_strict(markdown: &str) -> (Vec<FlagRow>, Vec<String>) {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.table = true;
    let root = parse_document(&arena, markdown, &options);

    let mut rows = Vec::new();
    let mut unrecognized = Vec::new();
    for node in root.descendants() {
        if !matches!(node.data.borrow().value, NodeValue::Table(_)) {
            continue;
        }
        let table_rows: Vec<&AstNode> = node
            .children()
            .filter(|child| matches!(child.data.borrow().value, NodeValue::TableRow(_)))
            .collect();
        let Some(header) = table_rows.first() else {
            continue;
        };
        let header_cells = row_cells(header);
        if header_cells.len() != AUDIT_HEADER.len()
            || header_cells
                .iter()
                .zip(AUDIT_HEADER.iter())
                .any(|(cell, expected)| cell != expected)
        {
            continue;
        }
        for row in &table_rows[1..] {
            let cells = row_cells(row);
            let (Some(flag), Some(status_cell)) = (cells.first(), cells.get(STATUS_COLUMN)) else {
                continue;
            };
            match BookStatus::parse(status_cell) {
                Some(status) => rows.push(FlagRow {
                    flag: flag.clone(),
                    status,
                }),
                None => unrecognized.push(status_cell.clone()),
            }
        }
    }
    (rows, unrecognized)
}

#[test]
fn book_audit_parses_minimal_table() {
    let md = "\
| CLAN flag | Meaning | Chatter | Status | Notes |
|---|---|---|---|---|
| `+k` | case | `--x` | Done | note |
| `+z` | range | -- | Missing | |
| `+s` | search | `--y` | Rewriter only | x |
| `+c` | cap | `--z` | Done (no-op per CLAN) | y |
";
    let rows = parse_audit_rows(md);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].flag, "+k");
    assert_eq!(rows[0].status, BookStatus::Done);
    assert_eq!(rows[1].status, BookStatus::Missing);
    assert_eq!(rows[2].status, BookStatus::RewriterOnly);
    // Suffix after the leading status word is tolerated.
    assert_eq!(rows[3].status, BookStatus::Done);
}

#[test]
fn book_audit_ignores_non_audit_tables() {
    let md = "\
| Some | Other | Table |
|---|---|---|
| a | b | c |

| CLAN flag | Meaning | Chatter | Status | Notes |
|---|---|---|---|---|
| `+k` | case | `--x` | Done | note |
";
    let rows = parse_audit_rows(md);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].flag, "+k");
}

#[test]
fn book_audit_parses_real_freq_page_without_drift() {
    let path = crate::common::workspace_root().join("book/src/clan-reference/commands/freq.md");
    let md = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let (rows, unrecognized) = parse_audit_rows_strict(&md);
    assert!(
        unrecognized.is_empty(),
        "freq.md has unrecognized Status cells (book drift): {unrecognized:?}"
    );
    // freq.md documents dozens of flag-rows across several tables; sanity-check
    // a substantial set parsed and the `+k` row (just corrected) is present.
    assert!(
        rows.len() >= 30,
        "expected >= 30 freq flag-rows, parsed {}",
        rows.len()
    );
    assert!(
        rows.iter().any(|row| row.flag.contains("+k")),
        "freq +k flag-row should parse"
    );
}
