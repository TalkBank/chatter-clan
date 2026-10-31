//! The CHIP scoring core: source/response comparison, interaction codes and
//! the per-column tallies (`chip.cpp` `single_interaction`, `process_codes`,
//! `update_special`, `update_data`, `update_responses`, `update_morph`).

use std::fmt;

use super::config::{MinIndex, MinWords, Substitution};
use super::word::ChipWord;
#[cfg(test)]
use super::{config::ChipCaseMode, word::ChipStemKey};
use talkbank_model::MorFeature;

/// What an utterance is, relative to the `+b`/`+c` speaker lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Adult,
    Child,
    Other,
}

/// The four result columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Column {
    Adult,
    Child,
    AdultSelf,
    ChildSelf,
}

/// An utterance in the comparison window.
#[derive(Debug, Clone)]
pub(super) struct Utter {
    pub speaker: String,
    pub role: Role,
    pub words: Vec<ChipWord>,
}

/// Per-word result of comparing a source with a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Unmarked,
    Added,
    Deleted,
    Subst,
    Exact,
}

/// A kind of word run in an interaction code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RunKind {
    Exa,
    Add,
    Sub,
    Del,
}

/// Whole-utterance relationship (`$EXACT` etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Special {
    Exact,
    Expan,
    Reduc,
    Subst,
}

/// Morpheme-level relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MorphKind {
    Exa,
    Sub,
    Add,
    Del,
}

/// One generated interaction code.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Code {
    Run(RunKind, Vec<String>),
    Fronted,
    Special(Special),
    Morph(MorphKind, String),
    NoRep,
    LoRep,
    Dist(usize),
    Rep(f64),
}

impl fmt::Display for Code {
    /// Every code is rendered with its leading space, as CLAN concatenates them.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Run(kind, words) => {
                let tag = match kind {
                    RunKind::Exa => "EXA",
                    RunKind::Add => "ADD",
                    RunKind::Sub => "SUB",
                    RunKind::Del => "DEL",
                };
                write!(f, " ${tag}:{}", words.join("-"))
            }
            Self::Fronted => write!(f, " $FRO"),
            Self::Special(s) => write!(
                f,
                " ${}",
                match s {
                    Special::Exact => "EXACT",
                    Special::Expan => "EXPAN",
                    Special::Reduc => "REDUC",
                    Special::Subst => "SUBST",
                }
            ),
            Self::Morph(kind, text) => {
                let tag = match kind {
                    MorphKind::Exa => "MEXA",
                    MorphKind::Sub => "MSUB",
                    MorphKind::Add => "MADD",
                    MorphKind::Del => "MDEL",
                };
                write!(f, " ${tag}:{text}")
            }
            Self::NoRep => write!(f, " $NO_REP"),
            Self::LoRep => write!(f, " $LO_REP"),
            Self::Dist(n) => write!(f, " $DIST = {n}"),
            Self::Rep(r) => write!(f, " $REP = {r:.2}"),
        }
    }
}

/// Result of comparing one response with one source.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Outcome {
    /// Source too short or response empty: nothing scored, nothing printed.
    Empty,
    /// Scored; `codes` excludes the trailing `$DIST`/`$REP`.
    Scored { rep_index: f64, codes: Vec<Code> },
}

/// Accumulated measures for one column.
#[derive(Debug, Clone, Default)]
pub(super) struct Tally {
    pub utters: u64,
    pub responses: u64,
    pub clresponses: u64,
    pub no_overlap: u64,
    pub dist: u64,
    pub rep_sum: f64,
    pub add_ops: u64,
    pub del_ops: u64,
    pub exa_ops: u64,
    pub sub_ops: u64,
    pub add_words: u64,
    pub del_words: u64,
    pub exa_words: u64,
    pub sub_words: u64,
    pub fro_words: u64,
    pub madd: u64,
    pub mdel: u64,
    pub mexa: u64,
    pub msub: u64,
    pub exact_match: u64,
    pub expanded_match: u64,
    pub reduced_match: u64,
    pub subst_match: u64,
}

/// Tallies for all four columns.
#[derive(Debug, Clone, Default)]
pub(super) struct Tallies {
    pub adult: Tally,
    pub child: Tally,
    pub asr: Tally,
    pub csr: Tally,
}

impl Tallies {
    pub fn column_mut(&mut self, column: Column) -> &mut Tally {
        match column {
            Column::Adult => &mut self.adult,
            Column::Child => &mut self.child,
            Column::AdultSelf => &mut self.asr,
            Column::ChildSelf => &mut self.csr,
        }
    }
}

/// Scoring parameters from the configuration.
#[derive(Debug, Clone, Copy)]
pub(super) struct Params {
    pub subst: Substitution,
    pub min_words: MinWords,
    pub min_index: MinIndex,
}

fn stems_equal(a: &ChipWord, b: &ChipWord) -> bool {
    matches!((&a.stem_key, &b.stem_key), (Some(x), Some(y)) if x == y)
}

/// POS equality up to the first `:`, ignoring case.
fn pos_match(a: &ChipWord, b: &ChipWord) -> bool {
    a.pos
        .as_ref()
        .zip(b.pos.as_ref())
        .is_some_and(|(left, right)| left.category().eq_ignore_ascii_case(right.category()))
}

fn render_affixes(affixes: &[Vec<MorFeature>]) -> String {
    affixes
        .iter()
        .map(|part| {
            part.iter()
                .map(|feature| match feature.key() {
                    Some(key) => format!("-{key}={}", feature.value()),
                    None => format!("-{}", feature.value()),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("~")
}

fn morph_code(
    response: Option<&[Vec<MorFeature>]>,
    source: Option<&[Vec<MorFeature>]>,
) -> Option<(MorphKind, String)> {
    match (response, source) {
        (Some(r), Some(s)) if r == s => Some((MorphKind::Exa, render_affixes(r))),
        (Some(r), Some(_)) => Some((MorphKind::Sub, render_affixes(r))),
        (Some(r), None) => Some((MorphKind::Add, render_affixes(r))),
        (None, Some(s)) => Some((MorphKind::Del, render_affixes(s))),
        (None, None) => None,
    }
}

/// Counts and codes gathered while comparing one pair.
#[derive(Default)]
struct MorphCounts {
    add: u64,
    del: u64,
    exa: u64,
    sub: u64,
}

/// Compare `response` against `source`, updating `tally` as CLAN does.
pub(super) fn single_interaction(
    source: &[ChipWord],
    response: &[ChipWord],
    params: Params,
    tally: &mut Tally,
) -> Outcome {
    if source.len() < params.min_words.0 || response.is_empty() {
        return Outcome::Empty;
    }
    let subst_on = params.subst == Substitution::On;
    let mut rm = vec![Mark::Unmarked; response.len()];
    let mut sm = vec![Mark::Unmarked; source.len()];
    let mut overlap = 0u64;
    let mut do_morph = false;
    let mut posd = 0u64;
    let mut morph = MorphCounts::default();
    let mut morph_codes = Vec::new();

    for (i, r) in response.iter().enumerate() {
        for (j, s) in source.iter().enumerate() {
            if rm[i] == Mark::Exact || sm[j] == Mark::Exact || !stems_equal(r, s) {
                continue;
            }
            overlap += 1;
            rm[i] = Mark::Exact;
            sm[j] = Mark::Exact;
            if !subst_on || pos_match(r, s) {
                do_morph = true;
            } else {
                posd += 1;
            }
            for (rm_aff, sm_aff) in [
                (r.prefix.as_deref(), s.prefix.as_deref()),
                (r.suffix.as_deref(), s.suffix.as_deref()),
            ] {
                let Some((kind, text)) = morph_code(rm_aff, sm_aff) else {
                    continue;
                };
                if do_morph {
                    match kind {
                        MorphKind::Exa => morph.exa += 1,
                        MorphKind::Sub => morph.sub += 1,
                        MorphKind::Add => morph.add += 1,
                        MorphKind::Del => morph.del += 1,
                    }
                }
                morph_codes.push(Code::Morph(kind, text));
            }
            break;
        }
    }

    for (i, r) in response.iter().enumerate() {
        for (j, s) in source.iter().enumerate() {
            if stems_equal(r, s) && rm[i] == Mark::Exact && sm[j] == Mark::Exact {
                break;
            } else if subst_on
                && rm[i] == Mark::Unmarked
                && sm[j] == Mark::Unmarked
                && pos_match(r, s)
            {
                rm[i] = Mark::Subst;
                sm[j] = Mark::Subst;
            }
        }
    }

    for m in rm.iter_mut().filter(|m| **m == Mark::Unmarked) {
        *m = Mark::Added;
    }
    for m in sm.iter_mut().filter(|m| **m == Mark::Unmarked) {
        *m = Mark::Deleted;
    }

    let rep_index = overlap as f64 / response.len() as f64;
    let (codes, class_word) = if overlap == 0 {
        (vec![Code::NoRep], false)
    } else if rep_index < params.min_index.get() {
        (vec![Code::LoRep], false)
    } else {
        let (mut codes, class_word) = process_codes(
            source,
            &sm,
            response,
            &rm,
            params.subst,
            &morph,
            posd,
            tally,
        );
        codes.extend(morph_codes);
        (codes, class_word)
    };

    if rep_index > params.min_index.get() {
        tally.responses += 1;
        tally.rep_sum += rep_index;
        if class_word {
            tally.clresponses += 1;
        }
    } else {
        tally.no_overlap += 1;
    }
    tally.madd += morph.add;
    tally.mdel += morph.del;
    tally.mexa += morph.exa;
    tally.msub += morph.sub;

    Outcome::Scored { rep_index, codes }
}

/// Maximal runs of `target` in `marks`, as `(start, end_exclusive)`.
fn runs(marks: &[Mark], target: Mark) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < marks.len() {
        if marks[i] == target {
            let start = i;
            while i < marks.len() && marks[i] == target {
                i += 1;
            }
            out.push((start, i));
        } else {
            i += 1;
        }
    }
    out
}

#[derive(Default)]
struct OpCounts {
    ops: u64,
    words: u64,
}

#[allow(clippy::too_many_arguments)]
fn process_codes(
    source: &[ChipWord],
    sm: &[Mark],
    response: &[ChipWord],
    rm: &[Mark],
    subst: Substitution,
    morph: &MorphCounts,
    posd: u64,
    tally: &mut Tally,
) -> (Vec<Code>, bool) {
    let subst_on = subst == Substitution::On;
    let mut codes = Vec::new();
    let mut fronted = false;
    let (mut exa, mut add, mut sub, mut del) = (
        OpCounts::default(),
        OpCounts::default(),
        OpCounts::default(),
        OpCounts::default(),
    );

    let mut emit = |kind: RunKind,
                    words: &[ChipWord],
                    marks: &[Mark],
                    target: Mark,
                    counts: &mut OpCounts,
                    can_front: bool| {
        for (start, end) in runs(marks, target) {
            if can_front && subst_on && start == 0 {
                codes.push(Code::Fronted);
                fronted = true;
            }
            counts.ops += 1;
            counts.words += (end - start) as u64;
            let names = words[start..end]
                .iter()
                .filter_map(|w| match kind {
                    RunKind::Exa => w.stem.clone(),
                    _ => Some(w.text.clone()),
                })
                .collect();
            codes.push(Code::Run(kind, names));
        }
    };
    emit(RunKind::Exa, response, rm, Mark::Exact, &mut exa, true);
    emit(RunKind::Add, response, rm, Mark::Added, &mut add, true);
    if subst_on {
        emit(RunKind::Sub, response, rm, Mark::Subst, &mut sub, true);
    }
    emit(RunKind::Del, source, sm, Mark::Deleted, &mut del, false);

    tally.add_ops += add.ops;
    tally.del_ops += del.ops;
    tally.exa_ops += exa.ops;
    tally.sub_ops += sub.ops;
    tally.add_words += add.words;
    tally.del_words += del.words;
    tally.exa_words += exa.words;
    tally.sub_words += sub.words;
    if fronted {
        tally.fro_words += 1;
    }

    let any_ops = add.ops + del.ops + exa.ops + sub.ops > 0;
    if !any_ops {
        return (codes, false);
    }
    let special = special_code(
        subst,
        [add.ops, del.ops, exa.ops, sub.ops],
        [morph.add, morph.del, morph.sub],
        posd,
    );
    if let Some(special) = special {
        match special {
            Special::Exact => tally.exact_match += 1,
            Special::Expan => tally.expanded_match += 1,
            Special::Reduc => tally.reduced_match += 1,
            Special::Subst => tally.subst_match += 1,
        }
        codes.push(Code::Special(special));
    }
    (codes, subst_on)
}

/// `update_special`: classify the whole response. The chain is exclusive: a
/// branch whose inner test fails yields no code.
fn special_code(
    subst: Substitution,
    [add, del, exa, sub]: [u64; 4],
    [madd, mdel, msub]: [u64; 3],
    posd: u64,
) -> Option<Special> {
    let subst_on = subst == Substitution::On;
    if add + del + sub + madd + mdel + msub == 0 {
        ((!subst_on && exa > 0) || (subst_on && exa > 0 && posd == 0)).then_some(Special::Exact)
    } else if del + sub + mdel + msub == 0 && exa > 0 {
        (add > 0).then_some(Special::Expan)
    } else if add + sub + madd + msub == 0 && exa > 0 {
        (del > 0).then_some(Special::Reduc)
    } else if add + del + madd + mdel == 0 && exa > 0 && subst_on && sub > 0 {
        Some(Special::Subst)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::PosCategory;

    fn w(text: &str, stem: &str, pos: &str) -> ChipWord {
        w_with_case(text, stem, pos, ChipCaseMode::Preserve)
    }

    fn w_with_case(text: &str, stem: &str, pos: &str, case_mode: ChipCaseMode) -> ChipWord {
        ChipWord {
            text: text.into(),
            stem: Some(stem.into()),
            stem_key: Some(ChipStemKey::from_stem(stem.into(), case_mode)),
            prefix: None,
            suffix: None,
            pos: Some(PosCategory::new(pos)),
        }
    }

    fn words(stems: &[&str]) -> Vec<ChipWord> {
        stems.iter().map(|s| w(s, s, "n")).collect()
    }

    fn params(subst: Substitution) -> Params {
        Params {
            subst,
            min_words: MinWords(0),
            min_index: MinIndex::default(),
        }
    }

    #[test]
    fn plus_k_case_folded_stems_match_but_default_stems_do_not() {
        let mut tally = Tally::default();
        let preserve = single_interaction(
            &[w("Want", "Want", "n")],
            &[w("want", "want", "n")],
            params(Substitution::Off),
            &mut tally,
        );
        let folded = single_interaction(
            &[w_with_case("Want", "Want", "n", ChipCaseMode::Fold)],
            &[w_with_case("want", "want", "n", ChipCaseMode::Fold)],
            params(Substitution::Off),
            &mut tally,
        );

        let Outcome::Scored {
            rep_index: preserve_index,
            ..
        } = preserve
        else {
            panic!("non-empty utterances should be scored");
        };
        let Outcome::Scored {
            rep_index: folded_index,
            ..
        } = folded
        else {
            panic!("non-empty utterances should be scored");
        };
        assert_eq!(preserve_index, 0.0);
        assert_eq!(folded_index, 1.0);
    }

    fn render(outcome: &Outcome) -> String {
        match outcome {
            Outcome::Scored { codes, .. } => codes.iter().map(Code::to_string).collect(),
            Outcome::Empty => String::new(),
        }
    }

    #[test]
    fn expansion_has_exa_add_and_expan() {
        // Hand-traced: source "a b", response "a b c" -> overlap 2/3.
        let mut t = Tally::default();
        let out = single_interaction(
            &words(&["a", "b"]),
            &words(&["a", "b", "c"]),
            params(Substitution::Off),
            &mut t,
        );
        assert_eq!(render(&out), " $EXA:a-b $ADD:c $EXPAN");
        assert_eq!(
            (t.exa_ops, t.exa_words, t.add_ops, t.add_words),
            (1, 2, 1, 1)
        );
        assert_eq!((t.expanded_match, t.responses), (1, 1));
        assert!((t.rep_sum - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn reduction_lists_deleted_source_words() {
        let mut t = Tally::default();
        let out = single_interaction(
            &words(&["a", "b", "c"]),
            &words(&["b"]),
            params(Substitution::Off),
            &mut t,
        );
        assert_eq!(render(&out), " $EXA:b $DEL:a $DEL:c $REDUC");
        assert_eq!((t.del_ops, t.del_words), (2, 2));
    }

    #[test]
    fn exact_repetition_is_exact() {
        let mut t = Tally::default();
        let out = single_interaction(
            &words(&["a", "b"]),
            &words(&["a", "b"]),
            params(Substitution::Off),
            &mut t,
        );
        assert_eq!(render(&out), " $EXA:a-b $EXACT");
        assert_eq!(t.exact_match, 1);
    }

    #[test]
    fn disjoint_utterances_are_no_rep_and_count_as_no_overlap() {
        let mut t = Tally::default();
        let out = single_interaction(
            &words(&["a"]),
            &words(&["z"]),
            params(Substitution::Off),
            &mut t,
        );
        assert_eq!(render(&out), " $NO_REP");
        assert_eq!((t.no_overlap, t.responses), (1, 0));
    }

    #[test]
    fn short_source_is_not_scored() {
        let mut t = Tally::default();
        let p = Params {
            min_words: MinWords(3),
            ..params(Substitution::Off)
        };
        let out = single_interaction(&words(&["a"]), &words(&["a"]), p, &mut t);
        assert_eq!(out, Outcome::Empty);
        assert_eq!(t.no_overlap + t.responses, 0);
    }

    #[test]
    fn min_index_gives_lo_rep() {
        let mut t = Tally::default();
        let p = Params {
            min_index: MinIndex::new(0.9).unwrap(),
            ..params(Substitution::Off)
        };
        let out = single_interaction(&words(&["a", "b"]), &words(&["a", "x", "y"]), p, &mut t);
        assert_eq!(render(&out), " $LO_REP");
        assert_eq!(t.no_overlap, 1);
    }

    #[test]
    fn substitution_fronts_and_substitutes_same_pos() {
        // source "dog ran", response "cat ran": ran exact, cat/dog both noun.
        let source = vec![w("dog", "dog", "n"), w("ran", "run", "v")];
        let response = vec![w("cat", "cat", "n"), w("ran", "run", "v")];
        let mut t = Tally::default();
        let out = single_interaction(&source, &response, params(Substitution::On), &mut t);
        assert_eq!(render(&out), " $EXA:run $FRO $SUB:cat $SUBST");
        assert_eq!((t.sub_ops, t.fro_words, t.clresponses), (1, 1, 1));
        assert_eq!(t.subst_match, 1);
    }

    #[test]
    fn morphology_codes_follow_special() {
        let mut src = w("cookie", "cookie", "n");
        let mut resp = w("cookies", "cookie", "n");
        src.suffix = None;
        resp.suffix = Some(vec![vec![MorFeature::new("Plur")]]);
        let mut t = Tally::default();
        let out = single_interaction(&[src], &[resp], params(Substitution::Off), &mut t);
        assert_eq!(render(&out), " $EXA:cookie $MADD:-Plur");
        assert_eq!(t.madd, 1);
        assert_eq!(t.exact_match, 0);
    }
}
