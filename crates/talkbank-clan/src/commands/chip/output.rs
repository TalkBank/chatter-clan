//! Typed CHIP results, the 36-measure matrix and its rendering
//! (`chip.cpp` `print_data`, `chip_utl.cpp` `pr_*`).

use std::fmt::Write;

use serde::Serialize;

use super::config::{ChipConfig, ColumnScoring, Scoring, Substitution};
use super::interaction::{Column, Tallies, Tally};
use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

const SEPARATOR: &str = "===========================================================";
const RULE: &str = "-----------------------------------------------------------";

/// One matrix row: a label and the ADU/CHI/ASR/CSR cells.
#[derive(Debug, Clone, Serialize)]
pub struct MatrixRow {
    /// Row label, including CLAN's padding.
    pub label: String,
    /// Cells for ADU, CHI, ASR and CSR.
    pub cells: [String; 4],
    /// CLAN prints a stray tab after the last cell of one row; kept for
    /// byte-compatibility.
    pub trailing_tab: bool,
}

/// The per-file measure matrix.
#[derive(Debug, Clone, Serialize)]
pub struct ChipMatrix {
    /// Header text for the adult speaker list.
    pub adult_codes: String,
    /// Header text for the child speaker list (CLAN quirk: one slot per adult).
    pub child_codes: String,
    /// Scored adult utterances.
    pub adult_utterances: u64,
    /// Scored child utterances.
    pub child_utterances: u64,
    /// Measure rows in CLAN order.
    pub rows: Vec<MatrixRow>,
}

/// Scoring result for one input file.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "result", rename_all = "snake_case")]
pub enum ChipFileStatus {
    /// There were no utterances with words, so CLAN emits no matrix.
    NoScorableUtterances,
    /// Scoring completed.
    Scored(ChipMatrix),
    /// The input could not be scored without inventing word alignment.
    Rejected(String),
}

/// CHIP output for one input file.
#[derive(Debug, Clone, Serialize)]
pub struct ChipFileReport {
    /// Input filename shown by CLAN.
    pub filename: String,
    /// Echoed utterances with the generated `%adu`/`%chi`/`%asr`/`%csr` tiers.
    #[serde(skip)]
    pub transcript: String,
    /// Distinguishes empty input, completed scores, and rejected input.
    pub status: ChipFileStatus,
}

/// Typed output for the CHIP command.
#[derive(Debug, Clone, Serialize)]
pub struct ChipResult {
    /// One report per input file.
    pub files: Vec<ChipFileReport>,
}

enum ChipRatio {
    Undefined,
    Defined(f64),
}

impl ChipRatio {
    fn from_parts(num: f64, den: f64) -> Self {
        if den > 0.0 {
            Self::Defined(num / den)
        } else {
            Self::Undefined
        }
    }

    fn render(self, precision: usize) -> String {
        match self {
            Self::Undefined => "0.00".to_owned(),
            Self::Defined(value) => format!("{value:.precision$}"),
        }
    }
}

fn ratio(num: f64, den: f64, precision: usize) -> String {
    ChipRatio::from_parts(num, den).render(precision)
}

fn count(n: u64) -> String {
    n.to_string()
}

fn own_utters(t: &Tallies, column: Column) -> f64 {
    match column {
        Column::Adult | Column::AdultSelf => t.adult.utters as f64,
        Column::Child | Column::ChildSelf => t.child.utters as f64,
    }
}

struct RowBuilder<'a> {
    tallies: &'a Tallies,
    scoring: ColumnScoring,
    subst: bool,
    rows: Vec<MatrixRow>,
}

impl RowBuilder<'_> {
    fn push_with(&mut self, label: &str, trailing_tab: bool, f: impl Fn(Column, &Tally) -> String) {
        let tallies = self.tallies;
        let cell = |column: Column, scoring: Scoring, name: &str| match scoring {
            Scoring::Enabled => {
                let t = match column {
                    Column::Adult => &tallies.adult,
                    Column::Child => &tallies.child,
                    Column::AdultSelf => &tallies.asr,
                    Column::ChildSelf => &tallies.csr,
                };
                f(column, t)
            }
            Scoring::Disabled => name.to_owned(),
        };
        let cells = [
            cell(Column::Adult, self.scoring.adult, "adu"),
            cell(Column::Child, self.scoring.child, "chi"),
            cell(Column::AdultSelf, self.scoring.self_rep, "asr"),
            cell(Column::ChildSelf, self.scoring.self_rep, "csr"),
        ];
        let trailing_tab = trailing_tab && self.scoring.self_rep == Scoring::Enabled;
        self.rows.push(MatrixRow {
            label: label.to_owned(),
            cells,
            trailing_tab,
        });
    }

    fn push(&mut self, label: &str, f: impl Fn(Column, &Tally) -> String) {
        self.push_with(label, false, f);
    }

    /// A row that exists only under `+g`.
    fn push_subst(&mut self, label: &str, f: impl Fn(Column, &Tally) -> String) {
        if self.subst {
            self.push(label, f);
        }
    }
}

impl ChipMatrix {
    pub(super) fn build(tallies: &Tallies, config: &ChipConfig) -> Self {
        let subst = config.source.substitution() == Substitution::On;
        let mut b = RowBuilder {
            tallies,
            scoring: config.scoring,
            subst,
            rows: Vec::new(),
        };
        let ops_total = |t: &Tally| (t.add_ops + t.del_ops + t.exa_ops + t.sub_ops) as f64;
        let words_total =
            |t: &Tally| (t.add_words + t.del_words + t.exa_words + t.sub_words) as f64;
        let exa_words = |t: &Tally| t.exa_words as f64;

        b.push("Responses", |_, t| count(t.responses + t.no_overlap));
        b.push("Overlap  ", |_, t| count(t.responses));
        b.push_subst("Class_Overlap", |_, t| count(t.clresponses));
        b.push("No_Overlap", |_, t| count(t.no_overlap));
        b.push("%_Overlap", |c, t| {
            let num = if subst { t.clresponses } else { t.responses };
            ratio(num as f64, own_utters(tallies, c), 3)
        });
        b.push("Avg_Dist", |_, t| {
            ratio(t.dist as f64, t.responses as f64, 2)
        });
        b.push("Rep_Index", |_, t| ratio(t.rep_sum, t.responses as f64, 2));

        b.push("ADD_OPS  ", |_, t| count(t.add_ops));
        b.push("DEL_OPS  ", |_, t| count(t.del_ops));
        b.push("EXA_OPS  ", |_, t| count(t.exa_ops));
        b.push_subst("SUB_OPS  ", |_, t| count(t.sub_ops));
        b.push("%_ADD_OPS", |_, t| ratio(t.add_ops as f64, ops_total(t), 2));
        b.push("%_DEL_OPS", |_, t| ratio(t.del_ops as f64, ops_total(t), 2));
        b.push("%_EXA_OPS", |_, t| ratio(t.exa_ops as f64, ops_total(t), 2));
        b.push_subst("%_SUB_OPS", |_, t| ratio(t.sub_ops as f64, ops_total(t), 2));

        b.push("ADD_WORD", |_, t| count(t.add_words));
        b.push("DEL_WORD", |_, t| count(t.del_words));
        b.push("EXA_WORD", |_, t| count(t.exa_words));
        b.push_subst("SUB_WORD", |_, t| count(t.sub_words));
        b.push("%_ADD_WORDS", |_, t| {
            ratio(t.add_words as f64, words_total(t), 2)
        });
        b.push("%_DEL_WORDS", |_, t| {
            ratio(t.del_words as f64, words_total(t), 2)
        });
        b.push("%_EXA_WORDS", |_, t| {
            ratio(t.exa_words as f64, words_total(t), 2)
        });
        b.push_subst("%_SUB_WORDS", |_, t| {
            ratio(t.sub_words as f64, words_total(t), 2)
        });

        b.push("MORPH_ADD", |_, t| count(t.madd));
        b.push("MORPH_DEL", |_, t| count(t.mdel));
        b.push("MORPH_EXA", |_, t| count(t.mexa));
        b.push("MORPH_SUB", |_, t| count(t.msub));
        b.push("%_MORPH_ADD", |_, t| ratio(t.madd as f64, exa_words(t), 2));
        b.push("%_MORPH_DEL", |_, t| ratio(t.mdel as f64, exa_words(t), 2));
        b.push("%_MORPH_EXA", |_, t| ratio(t.mexa as f64, exa_words(t), 2));
        b.push("%_MORPH_SUB", |_, t| ratio(t.msub as f64, exa_words(t), 2));

        b.push("AV_WORD_ADD", |_, t| {
            ratio(t.add_words as f64, t.add_ops as f64, 2)
        });
        b.push("AV_WORD_DEL", |_, t| {
            ratio(t.del_words as f64, t.del_ops as f64, 2)
        });
        b.push("AV_WORD_EXA", |_, t| {
            ratio(t.exa_words as f64, t.exa_ops as f64, 2)
        });
        b.push_subst("AV_WORD_SUB", |_, t| {
            ratio(t.sub_words as f64, t.sub_ops as f64, 2)
        });

        b.push_subst("FRONTED  ", |_, t| count(t.fro_words));
        b.push_subst("%_FRONTED", |_, t| {
            ratio(t.fro_words as f64, t.clresponses as f64, 2)
        });

        let imitations =
            |t: &Tally| t.exact_match + t.expanded_match + t.reduced_match + t.subst_match;
        let denominator = |c: Column, t: &Tally| {
            if subst {
                t.clresponses as f64
            } else {
                own_utters(tallies, c)
            }
        };
        b.push("IMITAT   ", |_, t| count(imitations(t)));
        b.push_with("%_IMITAT", subst, |c, t| {
            ratio(imitations(t) as f64, denominator(c, t), 3)
        });
        b.push("EXACT    ", |_, t| count(t.exact_match));
        b.push("EXPAN    ", |_, t| count(t.expanded_match));
        b.push("REDUC    ", |_, t| count(t.reduced_match));
        b.push_subst("SUBST    ", |_, t| count(t.subst_match));
        b.push("%_EXACT  ", |c, t| {
            ratio(t.exact_match as f64, denominator(c, t), 3)
        });
        b.push("%_EXPAN  ", |c, t| {
            ratio(t.expanded_match as f64, denominator(c, t), 3)
        });
        b.push("%_REDUC  ", |c, t| {
            ratio(t.reduced_match as f64, denominator(c, t), 3)
        });
        b.push_subst("%_SUBST  ", |c, t| {
            ratio(t.subst_match as f64, denominator(c, t), 3)
        });

        // CLAN prints the child list by looping over the adult count.
        let child_codes = (0..config.adults().len())
            .map(|i| config.children().get(i).map_or("", |c| c.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let adult_codes = config
            .adults()
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Self {
            adult_codes,
            child_codes,
            adult_utterances: tallies.adult.utters,
            child_utterances: tallies.child.utters,
            rows: b.rows,
        }
    }

    fn render_clan(&self, out: &mut String) {
        let _ = writeln!(
            out,
            "Total {} scored utterances: {}",
            self.adult_codes, self.adult_utterances
        );
        let _ = writeln!(
            out,
            "Total {} scored utterances: {}\n",
            self.child_codes, self.child_utterances
        );
        let _ = writeln!(out, "Measure  \tADU\tCHI\tASR\tCSR");
        let _ = writeln!(out, "{RULE}");
        for row in &self.rows {
            let tab = if row.trailing_tab { "\t" } else { "" };
            let _ = writeln!(out, "{}\t{}{tab}", row.label, row.cells.join("\t"));
        }
    }
}

impl ChipResult {
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("chip");
        let multiple = self.files.len() > 1;
        for file in &self.files {
            let matrix = match &file.status {
                ChipFileStatus::Scored(matrix) => matrix,
                ChipFileStatus::NoScorableUtterances => {
                    result.add_section(Section::with_fields(
                        format!("File {}", file.filename),
                        [("Status".to_owned(), "no scorable utterances".to_owned())]
                            .into_iter()
                            .collect(),
                    ));
                    continue;
                }
                ChipFileStatus::Rejected(error) => {
                    result.add_section(Section::with_fields(
                        format!("File {}", file.filename),
                        [("Error".to_owned(), error.clone())].into_iter().collect(),
                    ));
                    continue;
                }
            };
            let rows = matrix
                .rows
                .iter()
                .map(|row| {
                    let mut values = vec![row.label.trim_end().to_owned()];
                    values.extend(row.cells.iter().cloned());
                    TableRow { values }
                })
                .collect();
            let heading = if multiple {
                format!("Measures ({})", file.filename)
            } else {
                "Measures".to_owned()
            };
            let mut section = Section::with_table(
                heading,
                ["Measure", "ADU", "CHI", "ASR", "CSR"]
                    .map(str::to_owned)
                    .to_vec(),
                rows,
            );
            section.fields.insert(
                "Adult scored utterances".to_owned(),
                matrix.adult_utterances.to_string(),
            );
            section.fields.insert(
                "Child scored utterances".to_owned(),
                matrix.child_utterances.to_string(),
            );
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for ChipResult {
    /// Render the matrix through the shared text table renderer.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// CLAN layout: per file, the echoed utterances with generated tiers, then
    /// the separator, speaker totals and measure matrix.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for file in &self.files {
            match &file.status {
                ChipFileStatus::NoScorableUtterances => {}
                ChipFileStatus::Scored(matrix) => {
                    out.push_str(&file.transcript);
                    let _ = writeln!(out, "{SEPARATOR}");
                    let _ = writeln!(out, "File: {}\n", file.filename);
                    matrix.render_clan(&mut out);
                }
                ChipFileStatus::Rejected(error) => {
                    let _ = writeln!(
                        out,
                        "Error: CHIP could not score {}: {error}",
                        file.filename
                    );
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::ratio;

    #[test]
    fn undefined_ratio_uses_clan_zero_format() {
        assert_eq!(ratio(0.0, 0.0, 3), "0.00");
    }

    #[test]
    fn defined_zero_ratio_uses_requested_precision() {
        assert_eq!(ratio(0.0, 1.0, 3), "0.000");
    }
}
