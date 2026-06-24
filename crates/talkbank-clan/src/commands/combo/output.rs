//! Typed COMBO results and rendering logic.

use std::collections::HashMap;
use std::fmt::Write;

use serde::Serialize;

use crate::framework::{
    AnalysisResult, CommandOutput, OutputFormat, Section, TableRow, UtteranceCount,
};

/// A single match found during COMBO processing.
#[derive(Debug, Clone, Serialize)]
pub struct ComboMatch {
    /// Speaker code.
    pub speaker: String,
    /// Full utterance text (CHAT format).
    pub utterance_text: String,
    /// Source filename.
    pub filename: String,
    /// 1-based source line number of the utterance, used by
    /// CLAN-compatible rendering to emit
    /// `*** File "pipeout": line N.`. `0` when no line map is
    /// available.
    pub line_number: usize,
    /// Per-search-expression hits: for each configured `SearchExpr`,
    /// the 1-based index of the expression and the set of lowercased
    /// word tokens that contributed to its match. CLAN-format
    /// rendering wraps each matched word as `(N)<word>` where `N` is
    /// the expression index.
    pub expr_hits: Vec<MatchedExpr>,
    /// CLAN `-wN` pre-context: up to `context_before` preceding
    /// utterance texts, oldest-first. Default empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_context: Vec<String>,
    /// CLAN `+wN` post-context: up to `context_after` following each
    /// match to include as post-context. Default empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_context: Vec<String>,
}

/// One search expression's contribution to a `ComboMatch`.
#[derive(Debug, Clone, Serialize)]
pub struct MatchedExpr {
    /// 1-based index of the expression in `ComboConfig.search`.
    pub index: usize,
    /// Lowercased word forms that the expression matched against this
    /// utterance.
    pub matched_words: Vec<String>,
}

/// Typed output for the COMBO command.
#[derive(Debug, Clone, Serialize)]
pub struct ComboResult {
    /// All matching utterances in order encountered.
    pub matches: Vec<ComboMatch>,
    /// Total utterances examined (including non-matches).
    pub total_utterances: UtteranceCount,
}

impl ComboResult {
    /// Convert typed matches into the shared table-based rendering container.
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("combo");
        if !self.matches.is_empty() {
            let rows: Vec<TableRow> = self
                .matches
                .iter()
                .map(|m| TableRow {
                    values: vec![
                        m.filename.clone(),
                        m.speaker.clone(),
                        m.utterance_text.clone(),
                    ],
                })
                .collect();

            let mut section = Section::with_table(
                "Matches".to_owned(),
                vec![
                    "File".to_owned(),
                    "Speaker".to_owned(),
                    "Utterance".to_owned(),
                ],
                rows,
            );
            section.fields.insert(
                "Matching utterances".to_owned(),
                self.matches.len().to_string(),
            );
            section.fields.insert(
                "Total utterances".to_owned(),
                self.total_utterances.to_string(),
            );
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for ComboResult {
    /// Render via the shared tabular text formatter.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// CLAN-compatible output matching legacy `combo` character-for-character.
    ///
    /// Format (from CLAN snapshot):
    /// ```text
    /// ----------------------------------------
    /// *** File "pipeout": line 6.
    /// *MOT:    (1)the (1)cat is on the mat .
    /// ----------------------------------------
    /// *** File "pipeout": line 12.
    /// *MOT:    yes , (1)the (1)cat .
    /// ----------------------------------------
    ///
    ///     Strings matched 3 times
    /// ```
    ///
    /// CLAN's combo wraps each word that matched the configured
    /// search expression with `(N)` where `N` is the 1-based index
    /// of the expression. Multiple expressions can match in the
    /// same utterance; each contributing word gets its own
    /// `(<expression-index>)` prefix.
    fn render_clan(&self) -> String {
        let mut out = String::new();

        for m in &self.matches {
            writeln!(out, "----------------------------------------").ok();
            // CLAN uses "pipeout" as the filename when reading from
            // stdin (chatter follows the same convention for
            // CLAN-format output to match the byte stream).
            writeln!(out, "*** File \"pipeout\": line {}.", m.line_number).ok();
            for line in &m.pre_context {
                writeln!(out, "{line}").ok();
            }
            // utterance_text already carries the `*SPK:\t...` prefix
            // (`Utterance::Main::to_chat_string()` includes it), so
            // we don't add another speaker prefix here. Wrap each
            // matched word as (N)<word> in place.
            let annotated = annotate_combo_matches(&m.utterance_text, &m.expr_hits);
            writeln!(out, "{annotated}").ok();
            for line in &m.post_context {
                writeln!(out, "{line}").ok();
            }
        }
        // Summary line. CLAN emits:
        //   <last match line>\n\n    Strings matched N times\n\n
        // No trailing `----` after the last match (the separators
        // appear *before* each match, not between or after).
        if !self.matches.is_empty() {
            writeln!(out).ok();
            writeln!(out, "    Strings matched {} times", self.matches.len()).ok();
            writeln!(out).ok();
        }
        out
    }
}

/// Wrap matched words in `text` with their expression-index prefix
/// `(N)`. CLAN's combo annotates the **first occurrence** of each
/// matched word per expression, not every occurrence, so for the
/// AND search `the+cat` against `the cat is on the mat`, only the
/// first `the` gets `(1)the`; the second `the` is left bare.
///
/// Implementation: keep a per-word "still owed" count for each
/// `(expr_index, lowercased_word)` pair, decrement on each match
/// during token walk, stop annotating that word once the budget is
/// exhausted.
fn annotate_combo_matches(text: &str, expr_hits: &[MatchedExpr]) -> String {
    if expr_hits.is_empty() {
        return text.to_owned();
    }
    // (lowercased_word) -> (expr_index, remaining_budget).
    // Lower expr indices "win" when multiple expressions matched
    // the same word, matching CLAN's first-expression-wins shape.
    let mut budget: HashMap<String, (usize, usize)> = HashMap::new();
    for hit in expr_hits {
        for w in &hit.matched_words {
            let entry = budget.entry(w.clone()).or_insert((hit.index, 0));
            // Lower expression index wins.
            if hit.index < entry.0 {
                *entry = (hit.index, entry.1 + 1);
            } else {
                entry.1 += 1;
            }
        }
    }
    // Preserve the leading `*SPK:\t` prefix verbatim, CLAN emits
    // a real tab between speaker and content; `split_whitespace`
    // would collapse it to a single space. Only the body after the
    // tab is rewritten with the `(N)` prefixes.
    let (prefix, body) = match text.find('\t') {
        Some(tab_pos) => text.split_at(tab_pos + 1),
        None => ("", text),
    };
    let mut out = String::with_capacity(text.len());
    out.push_str(prefix);
    // Token-walk the body, prefixing each matched token with `(N)`
    // while it still has budget. Token boundaries are whitespace;
    // punctuation tokens (`,`, `.`, etc.) are left untouched and
    // don't consume budget.
    let mut first = true;
    for tok in body.split_whitespace() {
        if !first {
            out.push(' ');
        }
        first = false;
        let lower = tok.to_lowercase();
        if let Some(slot) = budget.get_mut(lower.as_str())
            && slot.1 > 0
        {
            out.push_str(&format!("({}){tok}", slot.0));
            slot.1 -= 1;
            continue;
        }
        out.push_str(tok);
    }
    out
}
