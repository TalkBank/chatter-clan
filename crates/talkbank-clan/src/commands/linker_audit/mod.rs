//! LINKER-AUDIT, Cross-utterance linker and special terminator analysis.
//!
//! Analyzes usage of CHAT utterance linkers (`+<`, `++`, `+^`, `+"`, `+,`,
//! `+≋`, `+≈`) and special terminators (`+...`, `+/.`, `+//.`, `+"/.`, `+".`,
//! etc.) across an entire corpus.
//!
//! For each file, extracts:
//! - Linker and terminator frequency counts
//! - Cross-utterance pairing correctness (e.g., `++` must follow `+...` from
//!   different speaker)
//! - Anomalies: same-speaker `++`, `+,` without prior `+/.`, `+"` without
//!   `+"/.`, orphaned special terminators, `+<` overlap block patterns
//!
//! # Output
//!
//! Per-file anomaly details plus a corpus-wide summary with:
//! - Frequency tables for all linker and terminator types
//! - Pairing statistics and violation rates
//! - `+<` block analysis (block sizes, speaker counts)
//! - Orphaned terminator counts

mod output;

use std::collections::HashMap;
use std::fmt;

use talkbank_model::model::{Linker, Terminator};
use talkbank_model::{Line, Utterance};

use crate::framework::{AnalysisCommand, FileContext};

pub use output::LinkerAuditResult;

/// Configuration for the LINKER-AUDIT command.
#[derive(Debug, Clone, Default)]
pub struct LinkerAuditConfig {}

/// Tracks linker/terminator statistics and anomalies for one file.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub(super) struct FileStats {
    pub(super) filename: String,
    pub(super) total_utterances: usize,

    // Linker counts
    pub(super) linker_lazy_overlap: usize,
    pub(super) linker_other_completion: usize,
    pub(super) linker_quick_uptake: usize,
    pub(super) linker_quotation_follows: usize,
    pub(super) linker_self_completion: usize,
    pub(super) linker_tcu_continuation: usize,
    pub(super) linker_no_break_tcu: usize,

    // Special terminator counts
    pub(super) term_trailing_off: usize,
    pub(super) term_trailing_off_question: usize,
    pub(super) term_interruption: usize,
    pub(super) term_interrupted_question: usize,
    pub(super) term_self_interruption: usize,
    pub(super) term_self_interrupted_question: usize,
    pub(super) term_broken_question: usize,
    pub(super) term_quotation_follows: usize,
    pub(super) term_quotation_precedes: usize,
    pub(super) term_break_for_coding: usize,
    pub(super) term_ca_technical_break: usize,
    pub(super) term_ca_no_break: usize,

    // ++ pairing analysis
    pub(super) pp_correct: usize,
    pub(super) pp_same_speaker: usize,
    pub(super) pp_wrong_terminator: usize,
    pub(super) pp_first_utterance: usize,

    // +, pairing analysis
    pub(super) sc_correct: usize,
    pub(super) sc_wrong_terminator: usize,
    pub(super) sc_no_prior: usize,

    // +" pairing analysis
    pub(super) qf_correct: usize,
    pub(super) qf_chained: usize,
    pub(super) qf_wrong_terminator: usize,
    pub(super) qf_no_prior: usize,

    // Quotation balance
    pub(super) quot_follows_terms: usize,
    pub(super) quot_follows_links: usize,

    // +< overlap block analysis
    pub(super) lo_blocks: usize,
    pub(super) lo_block_size_1: usize,
    pub(super) lo_block_size_2: usize,
    pub(super) lo_block_size_3plus: usize,
    pub(super) lo_same_speaker_start: usize,
    pub(super) lo_max_speakers_in_block: usize,
    pub(super) lo_combined_with_other: usize,

    // +^ analysis
    pub(super) qu_same_speaker: usize,
    pub(super) qu_diff_speaker: usize,

    // +≋/+≈ TCU analysis
    pub(super) tcu_tech_same_speaker: usize,
    pub(super) tcu_tech_diff_speaker: usize,
    pub(super) tcu_nb_same_speaker: usize,
    pub(super) tcu_nb_diff_speaker: usize,

    // Orphaned terminators
    pub(super) trailing_off_total: usize,
    pub(super) trailing_off_followed: usize,
    pub(super) interruption_total: usize,
    pub(super) interruption_followed: usize,
}

impl FileStats {
    fn has_any_linker_or_special_terminator(&self) -> bool {
        self.linker_lazy_overlap > 0
            || self.linker_other_completion > 0
            || self.linker_quick_uptake > 0
            || self.linker_quotation_follows > 0
            || self.linker_self_completion > 0
            || self.linker_tcu_continuation > 0
            || self.linker_no_break_tcu > 0
            || self.term_trailing_off > 0
            || self.term_interruption > 0
            || self.term_self_interruption > 0
            || self.term_quotation_follows > 0
            || self.term_quotation_precedes > 0
    }

    fn total_anomalies(&self) -> usize {
        self.pp_same_speaker
            + self.pp_wrong_terminator
            + self.pp_first_utterance
            + self.sc_wrong_terminator
            + self.sc_no_prior
            + self.qf_wrong_terminator
            + self.qf_no_prior
    }
}

/// Corpus-wide aggregated statistics.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub(super) struct CorpusSummary {
    pub(super) files_total: usize,
    pub(super) files_with_linkers: usize,
    pub(super) files_with_anomalies: usize,

    // Linker totals
    pub(super) total_lazy_overlap: usize,
    pub(super) total_other_completion: usize,
    pub(super) total_quick_uptake: usize,
    pub(super) total_quotation_follows: usize,
    pub(super) total_self_completion: usize,
    pub(super) total_tcu_continuation: usize,
    pub(super) total_no_break_tcu: usize,

    // Terminator totals
    pub(super) total_trailing_off: usize,
    pub(super) total_trailing_off_question: usize,
    pub(super) total_interruption: usize,
    pub(super) total_interrupted_question: usize,
    pub(super) total_self_interruption: usize,
    pub(super) total_self_interrupted_question: usize,
    pub(super) total_broken_question: usize,
    pub(super) total_quotation_follows_term: usize,
    pub(super) total_quotation_precedes_term: usize,
    pub(super) total_break_for_coding: usize,
    pub(super) total_ca_technical_break: usize,
    pub(super) total_ca_no_break: usize,

    // ++ pairing
    pub(super) pp_correct: usize,
    pub(super) pp_same_speaker: usize,
    pub(super) pp_wrong_terminator: usize,
    pub(super) pp_first_utterance: usize,

    // +, pairing
    pub(super) sc_correct: usize,
    pub(super) sc_wrong_terminator: usize,
    pub(super) sc_no_prior: usize,

    // +" pairing
    pub(super) qf_correct: usize,
    pub(super) qf_chained: usize,
    pub(super) qf_wrong_terminator: usize,
    pub(super) qf_no_prior: usize,

    // +< blocks
    pub(super) lo_blocks_total: usize,
    pub(super) lo_isolated: usize,
    pub(super) lo_pairs: usize,
    pub(super) lo_large_blocks: usize,
    pub(super) lo_same_speaker_start: usize,
    pub(super) lo_combined_with_other: usize,

    // +^
    pub(super) qu_same_speaker: usize,
    pub(super) qu_diff_speaker: usize,

    // +≋/+≈
    pub(super) tcu_tech_same: usize,
    pub(super) tcu_tech_diff: usize,
    pub(super) tcu_nb_same: usize,
    pub(super) tcu_nb_diff: usize,

    // Orphans
    pub(super) trailing_off_total: usize,
    pub(super) trailing_off_followed: usize,
    pub(super) interruption_total: usize,
    pub(super) interruption_followed: usize,
}

/// Accumulated state across all files.
#[derive(Debug, Default)]
pub struct LinkerAuditState {
    files: Vec<FileStats>,
}

/// LINKER-AUDIT command.
#[derive(Debug, Clone, Default)]
pub struct LinkerAuditCommand;

/// Extract the linker kind(s) from an utterance.
fn get_linkers(utt: &Utterance) -> &[Linker] {
    utt.main.content.linkers.as_slice()
}

/// Extract the terminator from an utterance.
fn get_terminator(utt: &Utterance) -> Option<&Terminator> {
    utt.main.content.terminator.as_ref()
}

/// Check if a terminator is a trailing-off variant.
fn is_trailing_off(term: &Terminator) -> bool {
    matches!(
        term,
        Terminator::TrailingOff { .. } | Terminator::TrailingOffQuestion { .. }
    )
}

/// Check if a terminator is an interruption variant.
fn is_interruption(term: &Terminator) -> bool {
    matches!(
        term,
        Terminator::Interruption { .. } | Terminator::InterruptedQuestion { .. }
    )
}

/// Classify a terminator for display/counting.
///
/// The `Ca*` variants cover Conversation Analysis terminators that the
/// classifier currently never produces from the CHAT AST but exist for
/// exhaustive coverage of the typed terminator surface. They are
/// reachable when CA-corpus support is wired into the audit.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TerminatorKind {
    Period,
    Question,
    Exclamation,
    TrailingOff,
    TrailingOffQuestion,
    Interruption,
    InterruptedQuestion,
    SelfInterruption,
    SelfInterruptedQuestion,
    BrokenQuestion,
    QuotationFollows,
    QuotationPrecedes,
    BreakForCoding,
    CaTechnicalBreak,
    CaTechnicalBreakLinker,
    CaNoBreak,
    CaNoBreakLinker,
    CaIntonation,
}

impl fmt::Display for TerminatorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Period => write!(f, "."),
            Self::Question => write!(f, "?"),
            Self::Exclamation => write!(f, "!"),
            Self::TrailingOff => write!(f, "+..."),
            Self::TrailingOffQuestion => write!(f, "+..?"),
            Self::Interruption => write!(f, "+/."),
            Self::InterruptedQuestion => write!(f, "+/?"),
            Self::SelfInterruption => write!(f, "+//."),
            Self::SelfInterruptedQuestion => write!(f, "+//?"),
            Self::BrokenQuestion => write!(f, "+!?"),
            Self::QuotationFollows => write!(f, "+\"/."),
            Self::QuotationPrecedes => write!(f, "+\"."),
            Self::BreakForCoding => write!(f, "+."),
            Self::CaTechnicalBreak => write!(f, "≋"),
            Self::CaTechnicalBreakLinker => write!(f, "+≋"),
            Self::CaNoBreak => write!(f, "≈"),
            Self::CaNoBreakLinker => write!(f, "+≈"),
            Self::CaIntonation => write!(f, "(CA intonation)"),
        }
    }
}

fn classify_terminator(term: &Terminator) -> TerminatorKind {
    match term {
        Terminator::Period { .. } => TerminatorKind::Period,
        Terminator::Question { .. } => TerminatorKind::Question,
        Terminator::Exclamation { .. } => TerminatorKind::Exclamation,
        Terminator::TrailingOff { .. } => TerminatorKind::TrailingOff,
        Terminator::TrailingOffQuestion { .. } => TerminatorKind::TrailingOffQuestion,
        Terminator::Interruption { .. } => TerminatorKind::Interruption,
        Terminator::InterruptedQuestion { .. } => TerminatorKind::InterruptedQuestion,
        Terminator::SelfInterruption { .. } => TerminatorKind::SelfInterruption,
        Terminator::SelfInterruptedQuestion { .. } => TerminatorKind::SelfInterruptedQuestion,
        Terminator::BrokenQuestion { .. } => TerminatorKind::BrokenQuestion,
        Terminator::QuotedNewLine { .. } => TerminatorKind::QuotationFollows,
        Terminator::QuotedPeriodSimple { .. } => TerminatorKind::QuotationPrecedes,
        Terminator::BreakForCoding { .. } => TerminatorKind::BreakForCoding,
    }
}

/// Analyze one file's linker and terminator usage.
fn analyze_file(utterances: &[&Utterance], filename: &str) -> FileStats {
    let mut stats = FileStats {
        filename: filename.to_owned(),
        total_utterances: utterances.len(),
        ..Default::default()
    };

    let mut last_term_by_speaker: HashMap<&str, TerminatorKind> = HashMap::new();
    let mut last_linker_by_speaker: HashMap<&str, Option<Linker>> = HashMap::new();

    let mut prev_speaker: Option<&str> = None;
    let mut prev_terminator: Option<TerminatorKind> = None;
    let mut _prev_had_lazy_overlap = false;

    let mut in_lazy_block = false;
    let mut lazy_block_size: usize = 0;
    let mut lazy_block_speakers: Vec<&str> = Vec::new();

    for (idx, utt) in utterances.iter().enumerate() {
        let speaker = utt.main.speaker.as_str();
        let linkers = get_linkers(utt);
        let terminator = get_terminator(utt);
        let term_kind = terminator.map(classify_terminator);

        let mut has_lazy_overlap = false;
        let mut has_other_linker = false;
        for linker in linkers {
            match linker {
                Linker::LazyOverlapPrecedes => {
                    stats.linker_lazy_overlap += 1;
                    has_lazy_overlap = true;
                }
                Linker::OtherCompletion => stats.linker_other_completion += 1,
                Linker::QuickUptakeOverlap => stats.linker_quick_uptake += 1,
                Linker::QuotationFollows => stats.linker_quotation_follows += 1,
                Linker::SelfCompletion => stats.linker_self_completion += 1,
                Linker::TcuContinuation => stats.linker_tcu_continuation += 1,
                Linker::NoBreakTcuContinuation => stats.linker_no_break_tcu += 1,
            }
            if !matches!(linker, Linker::LazyOverlapPrecedes) {
                has_other_linker = true;
            }
        }

        if has_lazy_overlap && has_other_linker {
            stats.lo_combined_with_other += 1;
        }

        if let Some(term) = terminator {
            match classify_terminator(term) {
                TerminatorKind::TrailingOff => stats.term_trailing_off += 1,
                TerminatorKind::TrailingOffQuestion => stats.term_trailing_off_question += 1,
                TerminatorKind::Interruption => stats.term_interruption += 1,
                TerminatorKind::InterruptedQuestion => stats.term_interrupted_question += 1,
                TerminatorKind::SelfInterruption => stats.term_self_interruption += 1,
                TerminatorKind::SelfInterruptedQuestion => {
                    stats.term_self_interrupted_question += 1
                }
                TerminatorKind::BrokenQuestion => stats.term_broken_question += 1,
                TerminatorKind::QuotationFollows => stats.term_quotation_follows += 1,
                TerminatorKind::QuotationPrecedes => stats.term_quotation_precedes += 1,
                TerminatorKind::BreakForCoding => stats.term_break_for_coding += 1,
                TerminatorKind::CaTechnicalBreak => stats.term_ca_technical_break += 1,
                TerminatorKind::CaNoBreak => stats.term_ca_no_break += 1,
                _ => {}
            }
        }

        if linkers.iter().any(|l| matches!(l, Linker::OtherCompletion)) {
            if idx == 0 {
                stats.pp_first_utterance += 1;
            } else if let Some(ps) = prev_speaker {
                if ps == speaker {
                    stats.pp_same_speaker += 1;
                } else if prev_terminator.is_some_and(|t| {
                    matches!(
                        t,
                        TerminatorKind::TrailingOff | TerminatorKind::TrailingOffQuestion
                    )
                }) {
                    stats.pp_correct += 1;
                } else {
                    stats.pp_wrong_terminator += 1;
                }
            }
        }

        if linkers.iter().any(|l| matches!(l, Linker::SelfCompletion)) {
            match last_term_by_speaker.get(speaker) {
                None => stats.sc_no_prior += 1,
                Some(TerminatorKind::Interruption | TerminatorKind::InterruptedQuestion) => {
                    stats.sc_correct += 1;
                }
                Some(_) => stats.sc_wrong_terminator += 1,
            }
        }

        if linkers
            .iter()
            .any(|l| matches!(l, Linker::QuotationFollows))
        {
            stats.quot_follows_links += 1;
            match last_term_by_speaker.get(speaker) {
                None => stats.qf_no_prior += 1,
                Some(TerminatorKind::QuotationFollows) => stats.qf_correct += 1,
                _ => {
                    if last_linker_by_speaker
                        .get(speaker)
                        .and_then(|l| l.as_ref())
                        .is_some_and(|l| matches!(l, Linker::QuotationFollows))
                    {
                        stats.qf_chained += 1;
                    } else {
                        stats.qf_wrong_terminator += 1;
                    }
                }
            }
        }

        if term_kind == Some(TerminatorKind::QuotationFollows) {
            stats.quot_follows_terms += 1;
        }

        if has_lazy_overlap {
            if in_lazy_block {
                lazy_block_size += 1;
                if !lazy_block_speakers.contains(&speaker) {
                    lazy_block_speakers.push(speaker);
                }
            } else {
                flush_lazy_block(&mut stats, lazy_block_size, &lazy_block_speakers);
                in_lazy_block = true;
                lazy_block_size = 1;
                lazy_block_speakers.clear();
                lazy_block_speakers.push(speaker);

                if let Some(ps) = prev_speaker
                    && ps == speaker
                {
                    stats.lo_same_speaker_start += 1;
                }
            }
        } else if in_lazy_block {
            flush_lazy_block(&mut stats, lazy_block_size, &lazy_block_speakers);
            in_lazy_block = false;
            lazy_block_size = 0;
            lazy_block_speakers.clear();
        }

        if linkers
            .iter()
            .any(|l| matches!(l, Linker::QuickUptakeOverlap))
        {
            if prev_speaker.is_some_and(|ps| ps == speaker) {
                stats.qu_same_speaker += 1;
            } else {
                stats.qu_diff_speaker += 1;
            }
        }

        if linkers.iter().any(|l| matches!(l, Linker::TcuContinuation)) {
            if prev_speaker.is_some_and(|ps| ps == speaker) {
                stats.tcu_tech_same_speaker += 1;
            } else {
                stats.tcu_tech_diff_speaker += 1;
            }
        }
        if linkers
            .iter()
            .any(|l| matches!(l, Linker::NoBreakTcuContinuation))
        {
            if prev_speaker.is_some_and(|ps| ps == speaker) {
                stats.tcu_nb_same_speaker += 1;
            } else {
                stats.tcu_nb_diff_speaker += 1;
            }
        }

        if let Some(term) = terminator {
            if is_trailing_off(term) {
                stats.trailing_off_total += 1;
            }
            if is_interruption(term) {
                stats.interruption_total += 1;
            }
        }
        if let Some(pt) = prev_terminator {
            if matches!(
                pt,
                TerminatorKind::TrailingOff | TerminatorKind::TrailingOffQuestion
            ) && linkers
                .iter()
                .any(|l| matches!(l, Linker::OtherCompletion | Linker::SelfCompletion))
            {
                stats.trailing_off_followed += 1;
            }
            if matches!(
                pt,
                TerminatorKind::Interruption | TerminatorKind::InterruptedQuestion
            ) && linkers.iter().any(|l| matches!(l, Linker::SelfCompletion))
            {
                stats.interruption_followed += 1;
            }
        }

        prev_speaker = Some(speaker);
        prev_terminator = term_kind;
        _prev_had_lazy_overlap = has_lazy_overlap;
        if let Some(tk) = term_kind {
            last_term_by_speaker.insert(speaker, tk);
        }
        let primary_linker = linkers
            .iter()
            .find(|l| !matches!(l, Linker::LazyOverlapPrecedes))
            .cloned();
        last_linker_by_speaker.insert(speaker, primary_linker);
    }

    if in_lazy_block {
        flush_lazy_block(&mut stats, lazy_block_size, &lazy_block_speakers);
    }

    stats
}

fn flush_lazy_block(stats: &mut FileStats, size: usize, speakers: &[&str]) {
    if size == 0 {
        return;
    }
    stats.lo_blocks += 1;
    match size {
        1 => stats.lo_block_size_1 += 1,
        2 => stats.lo_block_size_2 += 1,
        _ => stats.lo_block_size_3plus += 1,
    }
    let distinct_speakers = speakers.len();
    if distinct_speakers > stats.lo_max_speakers_in_block {
        stats.lo_max_speakers_in_block = distinct_speakers;
    }
}

impl AnalysisCommand for LinkerAuditCommand {
    type Config = LinkerAuditConfig;
    type State = LinkerAuditState;
    type Output = LinkerAuditResult;

    fn process_utterance(
        &self,
        _utterance: &Utterance,
        _file_context: &FileContext<'_>,
        _state: &mut Self::State,
    ) {
    }

    fn end_file(&self, file_context: &FileContext<'_>, state: &mut Self::State) {
        let utterances: Vec<&Utterance> = file_context
            .chat_file
            .lines
            .iter()
            .filter_map(|line| match line {
                Line::Utterance(u) => Some(u.as_ref()),
                _ => None,
            })
            .collect();

        let stats = analyze_file(&utterances, file_context.filename);
        state.files.push(stats);
    }

    fn finalize(&self, state: Self::State) -> LinkerAuditResult {
        let files_with_linkers = state
            .files
            .iter()
            .filter(|f| f.has_any_linker_or_special_terminator())
            .count();
        let files_with_anomalies = state
            .files
            .iter()
            .filter(|f| f.total_anomalies() > 0)
            .count();

        let summary = CorpusSummary {
            files_total: state.files.len(),
            files_with_linkers,
            files_with_anomalies,
            total_lazy_overlap: state.files.iter().map(|f| f.linker_lazy_overlap).sum(),
            total_other_completion: state.files.iter().map(|f| f.linker_other_completion).sum(),
            total_quick_uptake: state.files.iter().map(|f| f.linker_quick_uptake).sum(),
            total_quotation_follows: state.files.iter().map(|f| f.linker_quotation_follows).sum(),
            total_self_completion: state.files.iter().map(|f| f.linker_self_completion).sum(),
            total_tcu_continuation: state.files.iter().map(|f| f.linker_tcu_continuation).sum(),
            total_no_break_tcu: state.files.iter().map(|f| f.linker_no_break_tcu).sum(),
            total_trailing_off: state.files.iter().map(|f| f.term_trailing_off).sum(),
            total_trailing_off_question: state
                .files
                .iter()
                .map(|f| f.term_trailing_off_question)
                .sum(),
            total_interruption: state.files.iter().map(|f| f.term_interruption).sum(),
            total_interrupted_question: state
                .files
                .iter()
                .map(|f| f.term_interrupted_question)
                .sum(),
            total_self_interruption: state.files.iter().map(|f| f.term_self_interruption).sum(),
            total_self_interrupted_question: state
                .files
                .iter()
                .map(|f| f.term_self_interrupted_question)
                .sum(),
            total_broken_question: state.files.iter().map(|f| f.term_broken_question).sum(),
            total_quotation_follows_term: state
                .files
                .iter()
                .map(|f| f.term_quotation_follows)
                .sum(),
            total_quotation_precedes_term: state
                .files
                .iter()
                .map(|f| f.term_quotation_precedes)
                .sum(),
            total_break_for_coding: state.files.iter().map(|f| f.term_break_for_coding).sum(),
            total_ca_technical_break: state.files.iter().map(|f| f.term_ca_technical_break).sum(),
            total_ca_no_break: state.files.iter().map(|f| f.term_ca_no_break).sum(),
            pp_correct: state.files.iter().map(|f| f.pp_correct).sum(),
            pp_same_speaker: state.files.iter().map(|f| f.pp_same_speaker).sum(),
            pp_wrong_terminator: state.files.iter().map(|f| f.pp_wrong_terminator).sum(),
            pp_first_utterance: state.files.iter().map(|f| f.pp_first_utterance).sum(),
            sc_correct: state.files.iter().map(|f| f.sc_correct).sum(),
            sc_wrong_terminator: state.files.iter().map(|f| f.sc_wrong_terminator).sum(),
            sc_no_prior: state.files.iter().map(|f| f.sc_no_prior).sum(),
            qf_correct: state.files.iter().map(|f| f.qf_correct).sum(),
            qf_chained: state.files.iter().map(|f| f.qf_chained).sum(),
            qf_wrong_terminator: state.files.iter().map(|f| f.qf_wrong_terminator).sum(),
            qf_no_prior: state.files.iter().map(|f| f.qf_no_prior).sum(),
            lo_blocks_total: state.files.iter().map(|f| f.lo_blocks).sum(),
            lo_isolated: state.files.iter().map(|f| f.lo_block_size_1).sum(),
            lo_pairs: state.files.iter().map(|f| f.lo_block_size_2).sum(),
            lo_large_blocks: state.files.iter().map(|f| f.lo_block_size_3plus).sum(),
            lo_same_speaker_start: state.files.iter().map(|f| f.lo_same_speaker_start).sum(),
            lo_combined_with_other: state.files.iter().map(|f| f.lo_combined_with_other).sum(),
            qu_same_speaker: state.files.iter().map(|f| f.qu_same_speaker).sum(),
            qu_diff_speaker: state.files.iter().map(|f| f.qu_diff_speaker).sum(),
            tcu_tech_same: state.files.iter().map(|f| f.tcu_tech_same_speaker).sum(),
            tcu_tech_diff: state.files.iter().map(|f| f.tcu_tech_diff_speaker).sum(),
            tcu_nb_same: state.files.iter().map(|f| f.tcu_nb_same_speaker).sum(),
            tcu_nb_diff: state.files.iter().map(|f| f.tcu_nb_diff_speaker).sum(),
            trailing_off_total: state.files.iter().map(|f| f.trailing_off_total).sum(),
            trailing_off_followed: state.files.iter().map(|f| f.trailing_off_followed).sum(),
            interruption_total: state.files.iter().map(|f| f.interruption_total).sum(),
            interruption_followed: state.files.iter().map(|f| f.interruption_followed).sum(),
        };

        LinkerAuditResult {
            files: state.files,
            summary,
        }
    }
}
