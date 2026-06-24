//! COMPLEXITY, Syntactic complexity ratio from `%gra` dependency tier.
//!
//! Computes syntactic complexity by counting subordinating dependency
//! relations in the `%gra` tier and computing their ratio to total tokens.
//!
//! Complexity-contributing relations are clause-embedding dependencies
//! that indicate syntactic subordination. Two sets of relations are
//! supported:
//!
//! - **UD (Universal Dependencies)**: CSUBJ, CCOMP, XCOMP, ACL, ADVCL, APPOS, EXPL
//! - **Legacy CLAN**: CSUBJ, COMP, CPRED, CPOBJ, COBJ, CJCT, XJCT, NJCT, CMOD, XMOD
//!
//! The command auto-detects which set to use based on the relations found.
//!
//! Output per speaker: counts of each relation type, complexity tokens
//! (sum of all matched relations), total tokens (all non-PUNCT entries),
//! and the complexity ratio (complexity_tokens / total_tokens).
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) for the
//! original COMPLEXITY command specification.
//!
//! # Differences from CLAN
//!
//! - Uses typed AST `GraTier` with `GrammaticalRelation` entries rather than
//!   raw string scanning of `%gra` tier text.
//! - Auto-detects UD vs legacy relation names (CLAN requires compile-time config).
//! - Supports JSON and CSV output in addition to text/XLS.
//! - Relation matching includes sub-relations (e.g., `CSUBJ:pass` matches CSUBJ).

mod output;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use talkbank_model::{DependentTier, Utterance};

use crate::framework::{AnalysisCommand, FileContext};

pub use output::{ComplexityResult, RelationStyle, SpeakerComplexity};

/// Configuration for the COMPLEXITY command.
#[derive(Debug, Clone, Default)]
pub struct ComplexityConfig {}

/// Per-speaker accumulator.
#[derive(Debug, Default)]
struct SpeakerAccum {
    csubj: u64,
    ccomp: u64,
    xcomp: u64,
    acl: u64,
    advcl: u64,
    appos: u64,
    expl: u64,
    comp: u64,
    cpred: u64,
    cpobj: u64,
    cobj: u64,
    cjct: u64,
    xjct: u64,
    njct: u64,
    cmod: u64,
    xmod: u64,
    tokens: u64,
    total_tokens: u64,
    has_ud: bool,
    has_legacy: bool,
}

impl SpeakerAccum {
    /// Process a single dependency relation label.
    fn count_relation(&mut self, label: &str) {
        // Strip sub-type suffixes (e.g., "CSUBJ:pass" -> "CSUBJ", "ACL-relcl" -> "ACL")
        let base = label
            .split(['-', ':'])
            .next()
            .unwrap_or(label)
            .to_uppercase();

        if base == "PUNCT" {
            return;
        }
        self.total_tokens += 1;

        match base.as_str() {
            "CSUBJ" => {
                self.csubj += 1;
                self.tokens += 1;
            }
            "CCOMP" => {
                self.ccomp += 1;
                self.tokens += 1;
                self.has_ud = true;
            }
            "XCOMP" => {
                self.xcomp += 1;
                self.tokens += 1;
                self.has_ud = true;
            }
            "ACL" => {
                self.acl += 1;
                self.tokens += 1;
                self.has_ud = true;
            }
            "ADVCL" => {
                self.advcl += 1;
                self.tokens += 1;
                self.has_ud = true;
            }
            "APPOS" => {
                self.appos += 1;
                self.tokens += 1;
                self.has_ud = true;
            }
            "EXPL" => {
                self.expl += 1;
                self.tokens += 1;
                self.has_ud = true;
            }
            "COMP" => {
                self.comp += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            "CPRED" => {
                self.cpred += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            "CPOBJ" => {
                self.cpobj += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            "COBJ" => {
                self.cobj += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            "CJCT" => {
                self.cjct += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            "XJCT" => {
                self.xjct += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            "NJCT" => {
                self.njct += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            "CMOD" => {
                self.cmod += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            "XMOD" => {
                self.xmod += 1;
                self.tokens += 1;
                self.has_legacy = true;
            }
            _ => {}
        }
    }

    fn into_result(self, speaker: &str) -> SpeakerComplexity {
        SpeakerComplexity {
            speaker: speaker.to_owned(),
            csubj: self.csubj,
            ccomp: self.ccomp,
            xcomp: self.xcomp,
            acl: self.acl,
            advcl: self.advcl,
            appos: self.appos,
            expl: self.expl,
            comp: self.comp,
            cpred: self.cpred,
            cpobj: self.cpobj,
            cobj: self.cobj,
            cjct: self.cjct,
            xjct: self.xjct,
            njct: self.njct,
            cmod: self.cmod,
            xmod: self.xmod,
            tokens: self.tokens,
            total_tokens: self.total_tokens,
        }
    }
}

/// Accumulated state for COMPLEXITY across all files.
#[derive(Debug, Default)]
pub struct ComplexityState {
    by_speaker: BTreeMap<String, SpeakerAccum>,
}

/// COMPLEXITY command implementation.
#[derive(Debug, Clone, Default)]
pub struct ComplexityCommand;

impl AnalysisCommand for ComplexityCommand {
    type Config = ComplexityConfig;
    type State = ComplexityState;
    type Output = ComplexityResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        _file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let speaker = utterance.main.speaker.to_string();

        // Find %gra tier and use typed relations
        let gra_tier = utterance.dependent_tiers.iter().find_map(|dep| {
            if let DependentTier::Gra(gra) = dep {
                Some(gra)
            } else {
                None
            }
        });

        let Some(gra_tier) = gra_tier else { return };

        let accum = state.by_speaker.entry(speaker).or_default();

        for relation in gra_tier.relations().iter() {
            accum.count_relation(relation.relation.as_str());
        }
    }

    fn finalize(&self, state: Self::State) -> ComplexityResult {
        let mut has_ud = false;
        let mut has_legacy = false;
        let mut speakers = Vec::new();

        for (speaker, accum) in state.by_speaker {
            has_ud |= accum.has_ud;
            has_legacy |= accum.has_legacy;
            speakers.push(accum.into_result(&speaker));
        }

        let style = if has_legacy && !has_ud {
            RelationStyle::Legacy
        } else {
            RelationStyle::Ud
        };

        ComplexityResult { speakers, style }
    }
}
