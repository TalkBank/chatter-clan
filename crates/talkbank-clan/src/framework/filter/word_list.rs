//! Loading CLAN-style `@FILE` word-list and search-expression files.
//!
//! Implements the `+s@FILE` / `-s@FILE` file-input convention for word
//! patterns ([`load_word_list_file`]) and COMBO boolean search expressions
//! ([`load_search_expr_file`]), both built on the shared
//! [`read_clan_list_file_lines`] reader (the OSX-CLAN `cutt.cpp::rdexclf`
//! file-format conventions). Extracted verbatim from the `filter` module; the
//! parent re-exports the public items so `filter::load_word_list_file` etc.
//! continue to resolve.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::framework::domain_types::WordPattern;

/// Failure modes when loading a CLAN-style word-list file
/// (`+s@FILE` / `-s@FILE`).
#[derive(Debug, Error)]
pub enum LoadWordListError {
    /// The file could not be opened or read.
    #[error("could not read word-list file {path}: {source}")]
    Io {
        /// Path that failed to open.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Read a CLAN-style `@FILE` list: one item per non-comment line,
/// with the conventions from OSX-CLAN's `cutt.cpp::rdexclf`:
///
/// * Leading UTF-8 BOM (`U+FEFF`) on the first line is stripped.
/// * Lines beginning with `# ` (hash + space) are skipped (human
///   comments).
/// * Lines beginning with `;%* ` are skipped (CLAN's annotation
///   prefix for grep-friendly notes).
/// * Blank or whitespace-only lines are skipped.
/// * Trailing whitespace (spaces, tabs) is stripped from each line.
///
/// Source order is preserved. Casing is preserved, callers that
/// want case-folding apply it downstream.
///
/// Shared between [`load_word_list_file`] (word patterns) and
/// [`load_search_expr_file`] (COMBO boolean expressions); the file
/// format is identical, only the per-line value type differs.
fn read_clan_list_file_lines(path: &Path) -> Result<Vec<String>, LoadWordListError> {
    let content = std::fs::read_to_string(path).map_err(|source| LoadWordListError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let body = content.strip_prefix('\u{feff}').unwrap_or(&content);
    Ok(body
        .lines()
        .map(|l| l.trim_end_matches([' ', '\t']))
        .filter(|l| !l.is_empty() && !l.starts_with("# ") && !l.starts_with(";%* "))
        .map(|l| l.to_owned())
        .collect())
}

/// Load a CLAN-style word-list file (`+s@FILE` / `-s@FILE` for
/// every command except COMBO and SCRIPT).
///
/// Each surviving line becomes one [`WordPattern`]. See
/// `read_clan_list_file_lines` for the file-format conventions.
pub fn load_word_list_file(path: &Path) -> Result<Vec<WordPattern>, LoadWordListError> {
    Ok(read_clan_list_file_lines(path)?
        .into_iter()
        .map(WordPattern::from)
        .collect())
}

/// Load a CLAN-style COMBO search-expression file (`+s@FILE` /
/// `-s@FILE` for COMBO only).
///
/// Each surviving line is returned verbatim, the caller parses
/// it into a `SearchExpr` (the boolean-expression AST defined in
/// `commands::combo`) before feeding the analysis runner. See
/// `read_clan_list_file_lines` for the file-format conventions.
pub fn load_search_expr_file(path: &Path) -> Result<Vec<String>, LoadWordListError> {
    read_clan_list_file_lines(path)
}
