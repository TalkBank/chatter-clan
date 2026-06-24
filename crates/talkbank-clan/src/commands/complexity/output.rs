//! Typed COMPLEXITY results and rendering logic.

use std::fmt::Write;

use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// Per-speaker complexity metrics.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SpeakerComplexity {
    /// Speaker identifier.
    pub speaker: String,
    /// CSUBJ (clausal subject) count.
    pub csubj: u64,
    /// CCOMP (clausal complement) count.
    pub ccomp: u64,
    /// XCOMP (open clausal complement) count.
    pub xcomp: u64,
    /// ACL (adnominal clause) count.
    pub acl: u64,
    /// ADVCL (adverbial clause modifier) count.
    pub advcl: u64,
    /// APPOS (appositional modifier) count, UD only.
    pub appos: u64,
    /// EXPL (expletive) count, UD only.
    pub expl: u64,
    /// COMP (complement) count, legacy only.
    pub comp: u64,
    /// CPRED (clausal predicate) count, legacy only.
    pub cpred: u64,
    /// CPOBJ (clausal object of preposition) count, legacy only.
    pub cpobj: u64,
    /// COBJ (clausal object) count, legacy only.
    pub cobj: u64,
    /// CJCT (clausal adjunct) count, legacy only.
    pub cjct: u64,
    /// XJCT (non-finite clausal adjunct) count, legacy only.
    pub xjct: u64,
    /// NJCT (nominal adjunct) count, legacy only.
    pub njct: u64,
    /// CMOD (clausal modifier) count, legacy only.
    pub cmod: u64,
    /// XMOD (non-finite clausal modifier) count, legacy only.
    pub xmod: u64,
    /// Total complexity tokens (sum of matched relations).
    pub tokens: u64,
    /// Total tokens (all non-PUNCT entries).
    pub total_tokens: u64,
}

impl SpeakerComplexity {
    /// Complexity ratio: complexity tokens / total tokens.
    pub(crate) fn ratio(&self) -> f64 {
        if self.total_tokens == 0 {
            0.0
        } else {
            self.tokens as f64 / self.total_tokens as f64
        }
    }
}

/// Whether the corpus uses UD or legacy dependency relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RelationStyle {
    /// Universal Dependencies (CSUBJ, CCOMP, XCOMP, ACL, ADVCL, APPOS, EXPL).
    Ud,
    /// Legacy CLAN (CSUBJ, COMP, CPRED, CPOBJ, COBJ, CJCT, XJCT, NJCT, CMOD, XMOD).
    Legacy,
}

/// Result of the COMPLEXITY command.
#[derive(Debug, Clone, Serialize)]
pub struct ComplexityResult {
    /// Per-speaker complexity metrics.
    pub speakers: Vec<SpeakerComplexity>,
    /// Detected relation style.
    pub style: RelationStyle,
}

impl ComplexityResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("complexity");
        for sp in &self.speakers {
            let mut rows = vec![TableRow {
                values: vec!["CSUBJ".to_owned(), sp.csubj.to_string()],
            }];
            if self.style == RelationStyle::Ud {
                rows.extend([
                    TableRow {
                        values: vec!["CCOMP".to_owned(), sp.ccomp.to_string()],
                    },
                    TableRow {
                        values: vec!["XCOMP".to_owned(), sp.xcomp.to_string()],
                    },
                    TableRow {
                        values: vec!["ACL".to_owned(), sp.acl.to_string()],
                    },
                    TableRow {
                        values: vec!["ADVCL".to_owned(), sp.advcl.to_string()],
                    },
                    TableRow {
                        values: vec!["APPOS".to_owned(), sp.appos.to_string()],
                    },
                    TableRow {
                        values: vec!["EXPL".to_owned(), sp.expl.to_string()],
                    },
                ]);
            } else {
                rows.extend([
                    TableRow {
                        values: vec!["COMP".to_owned(), sp.comp.to_string()],
                    },
                    TableRow {
                        values: vec!["CPRED".to_owned(), sp.cpred.to_string()],
                    },
                    TableRow {
                        values: vec!["CPOBJ".to_owned(), sp.cpobj.to_string()],
                    },
                    TableRow {
                        values: vec!["COBJ".to_owned(), sp.cobj.to_string()],
                    },
                    TableRow {
                        values: vec!["CJCT".to_owned(), sp.cjct.to_string()],
                    },
                    TableRow {
                        values: vec!["XJCT".to_owned(), sp.xjct.to_string()],
                    },
                    TableRow {
                        values: vec!["NJCT".to_owned(), sp.njct.to_string()],
                    },
                    TableRow {
                        values: vec!["CMOD".to_owned(), sp.cmod.to_string()],
                    },
                    TableRow {
                        values: vec!["XMOD".to_owned(), sp.xmod.to_string()],
                    },
                ]);
            }
            rows.extend([
                TableRow {
                    values: vec!["Tokens".to_owned(), sp.tokens.to_string()],
                },
                TableRow {
                    values: vec!["TotalTokens".to_owned(), sp.total_tokens.to_string()],
                },
                TableRow {
                    values: vec!["Ratio".to_owned(), format!("{:.6}", sp.ratio())],
                },
            ]);
            result.add_section(Section::with_table(
                format!("Speaker: {}", sp.speaker),
                vec!["Relation".to_owned(), "Count".to_owned()],
                rows,
            ));
        }
        result
    }
}

impl CommandOutput for ComplexityResult {
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    fn render_clan(&self) -> String {
        let mut out = String::new();
        let _ = write!(
            out,
            "File,Language,Corpus,Code,Age,Sex,Group,Race,SES,Role,Education,Custom_field"
        );
        let _ = write!(out, ",CSUBJ");
        if self.style == RelationStyle::Ud {
            let _ = write!(out, ",CCOMP,XCOMP,ACL,ADVCL,APPOS,EXPL");
        } else {
            let _ = write!(out, ",COMP,CPRED,CPOBJ,COBJ,CJCT,XJCT,NJCT,CMOD,XMOD");
        }
        let _ = writeln!(out, ",Tokens,TotalTokens,Ratio");

        for sp in &self.speakers {
            let _ = write!(out, ".,.,.,{},.,.,.,.,.,.,.,.", sp.speaker);
            let _ = write!(out, ",{}", sp.csubj);
            if self.style == RelationStyle::Ud {
                let _ = write!(
                    out,
                    ",{},{},{},{},{},{}",
                    sp.ccomp, sp.xcomp, sp.acl, sp.advcl, sp.appos, sp.expl
                );
            } else {
                let _ = write!(
                    out,
                    ",{},{},{},{},{},{},{},{},{}",
                    sp.comp,
                    sp.cpred,
                    sp.cpobj,
                    sp.cobj,
                    sp.cjct,
                    sp.xjct,
                    sp.njct,
                    sp.cmod,
                    sp.xmod
                );
            }
            let _ = writeln!(out, ",{},{},{:.6}", sp.tokens, sp.total_tokens, sp.ratio());
        }
        out
    }
}
