// Fail-closed CLAN parity against committed golden files.
//
// Unlike the dual-snapshot `parity_case_tests!` mechanism (which records the
// real-CLAN output and the chatter output as two independent insta snapshots
// and never asserts they match), this mechanism asserts that chatter's
// CLAN-format output (`render(OutputFormat::Clan)`, the real CLI default
// boundary) is byte-identical to a committed golden seeded from the private
// CLAN build. A divergence turns the test RED.
//
// Verification needs NO CLAN binary: it reads the committed golden text file,
// so chatter's CI and any clone stay independent of CLAN. Only the regen step
// (seeding the golden) needs CLAN_BIN_DIR pointing at the private build; that
// is a deliberate, human-reviewed local act run via `just regen-clan-goldens`.

use std::path::PathBuf;

/// A reference to one CLAN-documented flag-row in the book's per-command audit
/// table, the denominator unit of the completeness metric (Phase 2). A passing
/// parity case marks every flag-row it `covers` as proven.
#[allow(dead_code)]
pub(crate) struct FlagRowRef {
    /// Command page stem, e.g. "freq" -> book/src/clan-reference/commands/freq.md.
    pub(crate) command: &'static str,
    /// The CLAN flag token exactly as it appears in the audit table's first
    /// backtick cell, e.g. "+t*X", "+c / +c0", "+d1". Bare invocation uses the
    /// sentinel "(bare)".
    pub(crate) flag: &'static str,
}

/// A typed link into the divergence ledger for an intentional CLAN-bug
/// correction. Not a bare string: Phase 2's metric generator validates that the
/// referenced ledger row actually exists in the book.
#[allow(dead_code)]
pub(crate) struct DivergenceRef {
    /// Ledger row id, e.g. "CLAN-DIV-001".
    pub(crate) ledger_row: &'static str,
    /// Book anchor documenting the source-grounded rationale.
    pub(crate) book_anchor: &'static str,
}

/// What a parity case asserts about chatter's CLAN-format output versus the
/// committed golden. There is no "snapshot both and assert nothing" state;
/// every case takes a position, and the default is byte-equality.
#[allow(dead_code)]
pub(crate) enum ParityExpectation {
    /// Byte-equality REQUIRED: the committed golden holds the private CLAN
    /// output, and chatter's CLAN-format output must equal it. The default,
    /// and the only way a flag-row counts as "proven".
    MatchesClan,
    /// chatter intentionally diverges (CLAN-bug correction): the committed
    /// golden holds chatter's *corrected* output. Verify still asserts chatter
    /// == golden (pins the correction); the regen step additionally asserts the
    /// corrected output differs from the recorded CLAN output, else the ledger
    /// row is stale.
    DivergesFromClan { rationale: DivergenceRef },
}

/// Whether a case compares CLAN-format stdout text or an aggregate
/// SpreadsheetML file.
#[allow(dead_code)]
pub(crate) enum GoldenCaseKind {
    /// chatter `render(OutputFormat::Clan)` stdout vs committed text golden.
    Text,
    /// chatter `FreqResult::to_spreadsheet` SpreadsheetML vs committed golden,
    /// over `fixture` + `extra_fixtures`. CLAN's side runs `+0` file-mode and
    /// the spreadsheet is captured from the `stat.frq*.xls` file it writes.
    Spreadsheet {
        /// `+d2` (per-word) or `+d3` (types/tokens/TTR only).
        mode: talkbank_clan::commands::freq::FreqSpreadsheetMode,
        /// Additional input files beyond `fixture` (the aggregate spreadsheet
        /// needs 2+ files to produce non-trivial rows).
        extra_fixtures: &'static [&'static str],
    },
}

/// One fail-closed parity case comparing chatter against a committed golden.
pub(crate) struct GoldenCase {
    /// Whether this is a text-stdout or spreadsheet-file case.
    pub(crate) kind: GoldenCaseKind,
    /// CLAN command name: the binary under `CLAN_BIN_DIR` at regen time, and
    /// the chatter dispatch key at verify time.
    pub(crate) command: &'static str,
    /// Reference-corpus fixture, resolved via `corpus_file`.
    pub(crate) fixture: &'static str,
    /// Legacy `+flag` args passed to the real CLAN binary when seeding.
    pub(crate) clan_args: &'static [&'static str],
    /// Modern `--flag` args passed to the chatter renderer when verifying.
    pub(crate) rust_args: &'static [&'static str],
    /// Speaker / word / range filter applied to the chatter run, mirroring
    /// CLAN selection flags like `+t*CHI`. The CLAN side gets the equivalent
    /// `+`-flag via `clan_args`.
    pub(crate) filter: crate::harness::FilterSpec,
    /// Golden file stem: `tests/golden/<command>/<case>.clan.txt`.
    pub(crate) case: &'static str,
    /// Parity position. Defaults to byte-equality.
    pub(crate) expectation: ParityExpectation,
    /// Flag-rows this case proves (Phase 2 metric numerator).
    #[allow(dead_code)]
    pub(crate) covers: &'static [FlagRowRef],
}

/// Resolve the committed golden file path for a case.
fn golden_path(command: &str, case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(command)
        .join(format!("{case}.clan.txt"))
}

/// Render chatter's CLAN-format output for a case (the real CLI default
/// boundary, `OutputFormat::Clan`).
fn render_chatter_clan(case: &GoldenCase) -> String {
    let file = crate::harness::corpus_file(case.fixture);
    crate::harness::run_rust_filtered(
        case.command,
        &file,
        case.rust_args,
        crate::harness::OutputFormat::Clan,
        crate::harness::build_filter(case.filter),
    )
}

/// Render chatter's output for a case, dispatching on its kind: CLAN-format
/// stdout text, or the aggregate SpreadsheetML the CLI writes for `+d2`/`+d3`.
fn render_chatter_output(case: &GoldenCase) -> String {
    match &case.kind {
        GoldenCaseKind::Text => render_chatter_clan(case),
        GoldenCaseKind::Spreadsheet {
            mode,
            extra_fixtures,
        } => {
            let mut files = vec![crate::harness::corpus_file(case.fixture)];
            files.extend(extra_fixtures.iter().map(|f| crate::harness::corpus_file(f)));
            crate::harness::run_rust_spreadsheet(&files, *mode, crate::harness::build_filter(case.filter))
        }
    }
}

/// Fail-closed assertion: chatter's CLAN-format output versus the committed
/// golden. RED on any byte difference. Needs no CLAN binary.
pub(crate) fn assert_clan_parity(case: &GoldenCase) {
    let golden_file = golden_path(case.command, case.case);
    let golden = std::fs::read_to_string(&golden_file).unwrap_or_else(|e| {
        panic!(
            "missing CLAN golden {}: {e}\n  Seed it with `just regen-clan-goldens` \
             (needs CLAN_BIN_DIR pointing at the private CLAN build).",
            golden_file.display()
        )
    });
    let actual = render_chatter_output(case);
    // Both MatchesClan and DivergesFromClan pin chatter's output to the golden;
    // the difference is what the golden holds (CLAN output vs chatter's
    // corrected output) and how the regen step treats it.
    assert_eq!(
        actual.trim_end(),
        golden.trim_end(),
        "{} {:?}: chatter output must byte-match the committed golden {}",
        case.command,
        case.clan_args,
        golden_file.display()
    );
}

/// Seed (or refresh) the committed golden for a case from the private CLAN
/// build. Only meaningful with `CLAN_BIN_DIR` set; invoked by the
/// `regenerate_*_goldens` ignored tests / `just regen-clan-goldens`.
#[allow(dead_code)]
pub(crate) fn seed_clan_golden(case: &GoldenCase) {
    // Spreadsheet goldens hold chatter's own (deterministic) SpreadsheetML.
    // The CLAN-binary parity is proven separately (the +d2/+d3 data cells are
    // byte-identical to CLAN's `stat.frq*.xls` modulo the documented `%%mor`
    // caveat, CLAN-DIV-004); the seed here just pins chatter's output.
    if matches!(case.kind, GoldenCaseKind::Spreadsheet { .. }) {
        let chatter = render_chatter_output(case);
        let golden_file = golden_path(case.command, case.case);
        if let Some(parent) = golden_file.parent() {
            std::fs::create_dir_all(parent).expect("create golden dir");
        }
        std::fs::write(&golden_file, format!("{}\n", chatter.trim_end()))
            .expect("write golden file");
        eprintln!("seeded {}", golden_file.display());
        return;
    }
    let file = crate::harness::corpus_file(case.fixture);
    // `run_clan` already strips the volatile CLAN banner (timestamp, version
    // date, "From pipe input"); the golden is the stable analysis body.
    let clan_output = crate::harness::run_clan(case.command, &file, case.clan_args)
        .unwrap_or_else(|| {
            panic!(
                "cannot seed golden for {} {:?}: CLAN binary unavailable or failed. \
                 Set CLAN_BIN_DIR to the private CLAN build.",
                case.command, case.clan_args
            )
        });
    // What the golden holds depends on the parity position: CLAN's own output
    // for MatchesClan, chatter's corrected output for an intentional divergence.
    let golden_contents = match &case.expectation {
        ParityExpectation::MatchesClan => clan_output,
        ParityExpectation::DivergesFromClan { rationale } => {
            let chatter = render_chatter_clan(case);
            assert_ne!(
                chatter.trim_end(),
                clan_output.trim_end(),
                "{} {:?}: marked DivergesFromClan ({}) but chatter now matches CLAN; \
                 ledger row {} is stale, reclassify the case as MatchesClan",
                case.command,
                case.clan_args,
                rationale.book_anchor,
                rationale.ledger_row
            );
            chatter
        }
    };
    let golden_file = golden_path(case.command, case.case);
    if let Some(parent) = golden_file.parent() {
        std::fs::create_dir_all(parent).expect("create golden dir");
    }
    // Trailing newline for clean diffs; verify trims trailing whitespace.
    std::fs::write(&golden_file, format!("{}\n", golden_contents.trim_end()))
        .expect("write golden file");
    eprintln!("seeded {}", golden_file.display());
}

/// Shorthand for a `freq` case on the shared eng-conversation fixture. A macro
/// (not a `const fn`) because the `covers` slice must promote to `'static`,
/// which only happens for literals in a const-item initializer, not for a
/// reference built from a `const fn` parameter.
macro_rules! freq_eng_case {
    ($case:literal, $clan:expr, $rust:expr, $filter:expr, $flag:literal) => {
        golden_case!(
            "freq",
            "languages/eng-conversation.cha",
            $case,
            $clan,
            $rust,
            $filter,
            $flag
        )
    };
}

/// General fail-closed case for any command/fixture/filter (a macro, not a
/// `const fn`, so the `covers` slice promotes to `'static`).
macro_rules! golden_case {
    ($command:literal, $fixture:literal, $case:literal, $clan:expr, $rust:expr, $filter:expr, $flag:literal) => {
        GoldenCase {
            kind: GoldenCaseKind::Text,
            command: $command,
            fixture: $fixture,
            clan_args: $clan,
            rust_args: $rust,
            filter: $filter,
            case: $case,
            expectation: ParityExpectation::MatchesClan,
            covers: &[FlagRowRef {
                command: $command,
                flag: $flag,
            }],
        }
    };
}

// The FREQ fail-closed parity backlog (first slice). Each case is seeded from
// the private CLAN build and verified against the committed golden with no
// CLAN present. All currently expect byte-equality (MatchesClan); a real
// divergence surfaces as a RED here, to be fixed in chatter (CLAN golden is
// authoritative) or reclassified as a documented divergence after adjudication.
const FREQ_BARE_ENG: GoldenCase = freq_eng_case!("bare_eng", &[], &[], FilterSpec::None, "(bare)");
const FREQ_D1_ENG: GoldenCase =
    freq_eng_case!("d1_eng", &["+d1"], &["--word-list-only"], FilterSpec::None, "+d1");
const FREQ_D4_ENG: GoldenCase =
    freq_eng_case!("d4_eng", &["+d4"], &["--types-tokens-only"], FilterSpec::None, "+d4");
const FREQ_C_ENG: GoldenCase = freq_eng_case!(
    "c_eng",
    &["+c"],
    &["--capitalization", "initial"],
    FilterSpec::None,
    "+c / +c0"
);
const FREQ_O1_ENG: GoldenCase =
    freq_eng_case!("o1_eng", &["+o1"], &["--sort", "reverse-concordance"], FilterSpec::None, "+o1");
// CLAN `+o3` (isCombineSpeakers, freq.cpp:832): pool ALL speakers into one
// frequency table, no per-speaker `Speaker:` header, summed counts, combined
// Types/Tokens/TTR. On eng-conversation (`*SPE` + `*GES`), `the` = 4 (2+2),
// Types=14, Tokens=28, TTR=0.500 (live CLAN probe). chatter flag:
// `--combine-speakers`. Completes the `+o` sort sub-cluster.
const FREQ_O3_COMBINE_ENG: GoldenCase = freq_eng_case!(
    "o3_combine_eng",
    &["+o3"],
    &["--combine-speakers"],
    FilterSpec::None,
    "+o3"
);
const FREQ_K_ENG: GoldenCase =
    freq_eng_case!("k_eng", &["+k"], &["--case-sensitive"], FilterSpec::None, "+k");
// `+t*SPE` selects only the SPE speaker (CLAN `+t*SPE`); the chatter side gets
// the equivalent speaker-include filter (FREQ selection flows through the
// filter, not `rust_args`).
const FREQ_T_SPE_ENG: GoldenCase = freq_eng_case!(
    "t_spe_eng",
    &["+t*SPE"],
    &[],
    FilterSpec::SpeakerInclude(&["SPE"]),
    "+t*X"
);
// CLAN `+b3`: MATTR over a 3-token window. SPE has 15 tokens (mostly distinct),
// so every length-3 window is all-distinct and MATTR is 1.000; the line follows
// the TTR caveat. chatter's modern flag is `--mattr 3`; SPE selection flows
// through the filter (FREQ selection is not in `rust_args`).
const FREQ_B_MATTR_ENG: GoldenCase = freq_eng_case!(
    "b_mattr_eng",
    &["+b3", "+t*SPE"],
    &["--mattr", "3"],
    FilterSpec::SpeakerInclude(&["SPE"]),
    "+bN"
);
// CLAN `+o` and `+o0` both sort the frequency output by DESCENDING frequency
// (freq.cpp:176; freq.cpp:815-817: `*f == EOS || *f == '0'` sets `isSort`),
// ties keeping the default alphabetical order. chatter's modern flag is
// `--sort frequency`. Cover string matches the freq.md cell `+o / +o0`.
const FREQ_O_ENG: GoldenCase = freq_eng_case!(
    "o_eng",
    &["+o"],
    &["--sort", "frequency"],
    FilterSpec::None,
    "+o / +o0"
);
// CLAN FREQ `+s"word"` / `+sword` restricts the count to the matching word(s)
// (manual FREQ section: "frequency count of the words want and to" via
// `freq +swant +sto`); it is a PER-WORD count, not utterance selection. chatter
// maps it to `--include-word`, consumed as a `WordFilterMode::PerWordEmit`
// filter (NOT the utterance-gate `FilterSpec::WordInclude`). `-s` is the
// exclude polarity.
const FREQ_S_WORD_ENG: GoldenCase = freq_eng_case!(
    "s_word_eng",
    &["+skept"],
    &["--include-word", "kept"],
    FilterSpec::None,
    "+s\"word\" / +sword"
);
// Multi-word `+s` group: a space-separated `+s` value is a GROUP matched as an
// adjacent in-order sequence (freq.cpp:2465-2548), counted once per occurrence
// under the search pattern. SPE says "up the hill", so `+s"the hill"` counts 1
// `the hill`. This is the Phase-1 spine of the multi-word search cluster: the
// default matcher the `+c2`/`+c3`/`+c4`/`+c7` modes layer onto (see
// `framework/multiword.rs`).
const FREQ_S_MULTIWORD_ENG: GoldenCase = freq_eng_case!(
    "s_multiword_eng",
    &["+sthe hill", "+t*SPE"],
    &["--include-word", "the hill"],
    FilterSpec::SpeakerInclude(&["SPE"]),
    "+s\"word\" / +sword"
);
// `+c3` (anyMultiOrder): multi-word `+s` matches anywhere and in any order
// (freq.cpp:2389-2464). SPE's "Triangle kept going ..." makes `+s"going
// Triangle"` (reversed, non-adjacent) count 1 under the search pattern. Phase 2
// of the multi-word cluster.
const FREQ_C3_ANYORDER_ENG: GoldenCase = freq_eng_case!(
    "c3_anyorder_eng",
    &["+c3", "+sgoing Triangle", "+t*SPE"],
    &["--multiword-order", "any", "--include-word", "going Triangle"],
    FilterSpec::SpeakerInclude(&["SPE"]),
    "+c3"
);
// `+c4` (onlySpecWsFound): a multi-word `+s` match counts only when the
// utterance consists solely of the group (freq.cpp:2381-2388). SPE's "Triangle
// kept going" is exactly that three-word utterance, so `+c4 +s"Triangle kept
// going"` counts 1. Phase 3 of the multi-word cluster.
const FREQ_C4_SOLE_ENG: GoldenCase = freq_eng_case!(
    "c4_sole_eng",
    &["+c4", "+sTriangle kept going", "+t*SPE"],
    &["--multiword-scope", "sole", "--include-word", "Triangle kept going"],
    FilterSpec::SpeakerInclude(&["SPE"]),
    "+c4"
);
// `+c2` (capwd==3): a SINGLE-word `+s` flag (NOT multi-word), counts a word once
// per matching pattern (freq.cpp:432-438). On `*SPE`, "top" matches both `t*`
// and `*p`, so `+c2` counts it 2 (the default counts it 1).
const FREQ_C2_PERPATTERN_ENG: GoldenCase = freq_eng_case!(
    "c2_perpattern_eng",
    &["+c2", "+st*", "+s*p", "+t*SPE"],
    &[
        "--search-multiplicity",
        "per-pattern",
        "--include-word",
        "t*",
        "--include-word",
        "*p",
    ],
    FilterSpec::SpeakerInclude(&["SPE"]),
    "+c2"
);
// `+c7` (isMultiWordsActual): display the actual matched words for a multi-word
// `+s` group, not the search pattern (freq.cpp:2444). `+s"the *"` collapses to
// `the *` by default; `+c7` reveals `the hill` and `the top` on `*SPE`.
const FREQ_C7_MATCHED_ENG: GoldenCase = freq_eng_case!(
    "c7_matched_eng",
    &["+c7", "+sthe *", "+t*SPE"],
    &["--multiword-display", "matched", "--include-word", "the *"],
    FilterSpec::SpeakerInclude(&["SPE"]),
    "+c7"
);
// `+d5` (zeroMatch): emit each LITERAL `+s` search word even when it never
// matched, with count 0 (freq.cpp:894 sets zeroMatch; 1473-1491 injects the
// `+s` words via `freq_tree_add_zeros`, freq.cpp:1259, which adds count 0 only
// if absent). The zero word is DISPLAYED but excluded from types/tokens/TTR:
// on `*SPE`, `+skept` matches once and `+szzz` never, so the table shows
// `1 kept` + `0 zzz` yet Types=1, Tokens=1, TTR=1.000 (verified by live CLAN
// probe). chatter's modern flag is `--include-zero-frequency`. Wildcards and
// duplicates in `+s` are rejected by `+d5` (freq.cpp:444), and that error path
// is pinned by the CLI subprocess tests, not this render golden.
const FREQ_D5_ZEROFREQ_ENG: GoldenCase = freq_eng_case!(
    "d5_zerofreq_eng",
    &["+d5", "+skept", "+szzz", "+t*SPE"],
    &[
        "--include-zero-frequency",
        "--include-word",
        "kept",
        "--include-word",
        "zzz",
    ],
    FilterSpec::SpeakerInclude(&["SPE"]),
    "+d5"
);
const FREQ_NEG_S_WORD_ENG: GoldenCase = freq_eng_case!(
    "neg_s_word_eng",
    &["-skept"],
    &["--exclude-word", "kept"],
    FilterSpec::None,
    "-s\"word\" / -sword"
);
// CLAN `+c1` is documented "middle only" (manual 5483; freq.cpp:167), but its
// implementation (freq.cpp `isRightUpper`, the `capwd == 2` branch selected by
// freq.cpp:782) loops from character position 0 and matches an uppercase
// anywhere, so it ALSO keeps initial-capital words, identical to `+c`. chatter
// implements the documented mid-only semantic (`--capitalization mid`), which
// correctly drops initial-only caps like `Triangle`. This is a deliberate
// CLAN-bug divergence: the golden holds chatter's CORRECTED output, and the
// regen step asserts it differs from CLAN's. See CLAN-DIV-003.
const FREQ_C1_ENG: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Text,
    command: "freq",
    fixture: "languages/eng-conversation.cha",
    clan_args: &["+c1"],
    rust_args: &["--capitalization", "mid"],
    filter: FilterSpec::None,
    case: "c1_eng",
    expectation: ParityExpectation::DivergesFromClan {
        rationale: DivergenceRef {
            ledger_row: "CLAN-DIV-003",
            book_anchor: "clan-reference/divergences/per-command.md#clan-div-003-freq-c1-middle-only",
        },
    },
    covers: &[FlagRowRef {
        command: "freq",
        flag: "+c1",
    }],
};
// CLAN `-t*X` excludes speaker X from the analysis (manual: `-t*` drops the
// main speaker tiers). chatter maps it to the speaker-exclude filter.
const FREQ_NEG_T_ENG: GoldenCase = freq_eng_case!(
    "neg_t_ges_eng",
    &["-t*GES"],
    &[],
    FilterSpec::SpeakerExclude(&["GES"]),
    "-t*X"
);
// CLAN `+t#ROLE` selects only speakers whose `@ID:` role matches (manual 11.10
// ROLES). eng-conversation tags both SPE and GES as role `Child`.
const FREQ_T_ROLE_ENG: GoldenCase = freq_eng_case!(
    "t_role_child_eng",
    &["+t#Child"],
    &[],
    FilterSpec::Role(&["Child"]),
    "+t#ROLE"
);
// CLAN `+t%X` (freq.cpp:914-938 `case 't'`, the `+t%` arm) selects dependent
// tier `%X` for counting: it sets `nomain=TRUE` and counts the WHITESPACE-
// delimited tokens of that tier's raw line, per speaker (CLAN manual: "Dependent
// tiers can be included or excluded by using the +t option immediately followed
// by the tier code"). This is NOT chatter's structural `--mor` (which splits
// post-clitics into separate items); the CLAN slot whitespace-splits the raw
// tier, so a clitic `v|go~aux|be` is ONE token. CLAN-excluded tokens (bare
// punctuation, `{0,&,+,-,#}`-prefix, `xxx`/`www`) are dropped: the long *SPE
// %mor line has 16 whitespace tokens but CLAN counts 15 (the bare `.` is
// dropped). chatter's faithful partner for the CLAN slot is `--tier X`; the
// separate `--mor` flag stays as chatter's structural convenience.
//
// `+t%gra` is NOT %mor-based, so CLAN appends the "%mor line forms" TTR advisory
// (freq.cpp:1536 gates the note on `!isMorUsed`). chatter flag: `--tier gra`.
const FREQ_T_GRA_ENG: GoldenCase = freq_eng_case!(
    "t_gra_eng",
    &["+t%gra"],
    &["--tier", "gra"],
    FilterSpec::None,
    "+t%X"
);
// `+t%mor` IS %mor-based: identical dependent-tier whitespace counting, but CLAN
// SUPPRESSES the TTR advisory (`isMorUsed`, freq.cpp:1536). chatter flag:
// `--tier mor`. Together the two cases pin the `%mor`-conditional advisory in
// both directions.
const FREQ_T_MOR_ENG: GoldenCase = freq_eng_case!(
    "t_mor_eng",
    &["+t%mor"],
    &["--tier", "mor"],
    FilterSpec::None,
    "+t%X"
);
// CLAN `-t%X` (freq.cpp `case 't'` else -> maingetflag tier-selection): EXCLUDE
// dependent tier `%X`. The exclude form flips FREQ to count the main tier PLUS
// every present dependent tier EXCEPT `%X` (banner: "ALL speaker tiers / and
// those speakers' ALL dependent tiers EXCEPT the ones matching: %GRA;"; manual
// section "Limiting by including or excluding dependent tiers"). So on
// eng-conversation, `-t%gra` counts main words + the `%mor` tokens (minus
// `%gra`), and `-t%mor` counts main words + the `%gra` relations (minus the gra
// PUNCT relations, per `gra_initwords`). Unlike `+t%mor`, the "%mor line forms"
// TTR advisory STAYS ON: CLAN sets `isMorUsed` only for the explicit `+t%mor`
// include, not when `-t` sweeps `%mor` in. chatter flag: `--exclude-tier X`.
const FREQ_NEG_T_GRA_ENG: GoldenCase = freq_eng_case!(
    "neg_t_gra_eng",
    &["-t%gra"],
    &["--exclude-tier", "gra"],
    FilterSpec::None,
    "-t%X"
);
const FREQ_NEG_T_MOR_ENG: GoldenCase = freq_eng_case!(
    "neg_t_mor_eng",
    &["-t%mor"],
    &["--exclude-tier", "mor"],
    FilterSpec::None,
    "-t%X"
);
// CLAN `+zN-M` is an utterance/word/turn range; the unit is a REQUIRED suffix
// (manual: `+z51u-100u` for utterances, `+z10w` for words; `cutt.cpp:8843`
// `getrange`). chatter's `--range` is an utterance range, so the equivalent
// CLAN invocation is `+z1u-2u` (the bare `+z1-2` the probe used is malformed,
// CLAN prints nothing). Selects utterances 1-2.
const FREQ_Z_RANGE_ENG: GoldenCase = freq_eng_case!(
    "z_range_eng",
    &["+z1u-2u"],
    &[],
    FilterSpec::UtteranceRange { start: 1, end: 2 },
    "+zN-M"
);
// CLAN `+x C N w` (word-count utterance filter, manual 6405): include only
// utterances whose countable-word count satisfies the comparison. `+x>3w` on
// eng-conversation keeps the >3-word utterances (dropping `*SPE`'s 3-word
// "Triangle kept going"), so SPE falls to 12 tokens. This is the WORD UNIT; the
// char (`c`) unit is the sibling case below, and the morpheme (`m`) unit and the
// `+xS` content-specification form are deferred (the `m` unit diverges from a
// CLAN doubling bug, see the field guide), so the `+x` row stays **Partial**.
// The case therefore carries NO `covers` (it does not mark `+x` proven in the
// metric); it is a regression golden for the word form.
const FREQ_X_WORDLEN_ENG: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Text,
    command: "freq",
    fixture: "languages/eng-conversation.cha",
    clan_args: &["+x>3w"],
    rust_args: &[],
    filter: FilterSpec::UtteranceLength {
        comparison: talkbank_clan::framework::LengthComparison::GreaterThan,
        threshold: 3,
        unit: talkbank_clan::framework::CountUnit::Word,
    },
    case: "x_wordlen_eng",
    expectation: ParityExpectation::MatchesClan,
    covers: &[],
};
// CLAN `+x C N c` (CHARACTER-count utterance filter, manual 6405;
// `cutt.cpp:16343` `CntFUttLen == 3`): include only utterances whose main-tier
// character count satisfies the comparison. `+x>20c` on eng-conversation drops
// `*SPE`'s 17-char "Triangle kept going" (8+4+5) and keeps the 39-char long
// utterance, so SPE falls to 12 tokens. The char unit is a clean main-tier
// measure (no `%mor` involvement), so chatter byte-matches CLAN; the count is
// the sum of `cleaned_text().chars().count()` over countable words. Like the
// word case it carries NO `covers`: the `+x` row stays Partial until the `m`
// unit and `+xS` land.
const FREQ_X_CHARLEN_ENG: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Text,
    command: "freq",
    fixture: "languages/eng-conversation.cha",
    clan_args: &["+x>20c"],
    rust_args: &[],
    filter: FilterSpec::UtteranceLength {
        comparison: talkbank_clan::framework::LengthComparison::GreaterThan,
        threshold: 20,
        unit: talkbank_clan::framework::CountUnit::Char,
    },
    case: "x_charlen_eng",
    expectation: ParityExpectation::MatchesClan,
    covers: &[],
};

// CLAN `+r6` (`R6`, `cutt.cpp:9554`; manual section 14.5): include retraced
// material in the counts. A retracing marker (`[/]`/`[//]`/`[///]`/`[/-]`)
// retraces the single immediately-preceding word; the default drops it, `+r6`
// keeps it. On `retrace.cha` this lifts `*CHI` 13->18 and `*MOT` 13->16 tokens,
// and chatter byte-matches CLAN: the `[: text]` cases count the replacement
// (e.g. `tika@u [: kitty] [//] kitty` -> `kitty` x2), never the original. `+r6`
// was a no-op bug (FREQ ignored `--include-retracings`) until 2026-06-04. NO
// `covers`: the `+rN` row stays Partial until `+r4`/`+r5`/`+r7`/`+r8` land.
const FREQ_R6_RETRACE: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Text,
    command: "freq",
    fixture: "annotation/retrace.cha",
    clan_args: &["+r6"],
    rust_args: &["--include-retracings"],
    filter: FilterSpec::None,
    case: "r6_retrace",
    expectation: ParityExpectation::MatchesClan,
    covers: &[],
};

// CLAN `+r5` (`R5`, `cutt.cpp:9549-9553`; manual section 14.5): a `[: text]`
// replacement counts the ORIGINAL surface word, not the replacement. On
// `retrace.cha`, `male [: female] [/] male [: female]` counts `female` by
// default but `male` under `+r5` (the non-retraced correction; the retraced copy
// stays excluded without `+r6`), a clean `female`->`male` swap. chatter
// byte-matches CLAN. NO `covers`: the `+rN` row stays Partial until
// `+r4`/`+r50`/`+r7`/`+r8` land.
const FREQ_R5_REPLACE: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Text,
    command: "freq",
    fixture: "annotation/retrace.cha",
    clan_args: &["+r5"],
    rust_args: &["--replacement-mode", "original"],
    filter: FilterSpec::None,
    case: "r5_replace",
    expectation: ParityExpectation::MatchesClan,
    covers: &[],
};

// CLAN `+pS` adds the characters of `S` to the word delimiters and re-tokenizes,
// so a word is split at them and each piece is counted on its own
// (`cutt.cpp:9798-9818`). On `word-features/000829.cha` `*MOT` with `+p_`,
// `choo_choo` (x3) -> `choo` (6) and `chup_chup_chup_chup@o` (x2) -> `chup` (6) +
// `chup@o` (2, the `@o` marker staying on the final segment). chatter's full
// per-speaker output byte-matches CLAN's (live diff), so MatchesClan. The CLAN
// side carries the speaker in `clan_args`; the chatter side selects it via the
// filter and maps `+p_` to `--word-delimiters _`.
const FREQ_P_DELIM: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Text,
    command: "freq",
    fixture: "word-features/000829.cha",
    clan_args: &["+p_", "+t*MOT"],
    rust_args: &["--word-delimiters", "_"],
    filter: FilterSpec::SpeakerInclude(&["MOT"]),
    case: "p_word_delimiter_mot",
    expectation: ParityExpectation::MatchesClan,
    covers: &[FlagRowRef {
        command: "freq",
        flag: "+pS",
    }],
};

// CLAN `+s@FILE` / `-s@FILE` search/exclude the words listed in FILE (one
// pattern per line; #-comments, `;%*` annotations, blanks skipped). The path is
// relative to the crate dir, which is both the test cwd (chatter loads it) and
// CLAN's working dir (`+s@<path>`). Both polarities cover the `+s@F / -s@F`
// row; chatter byte-matches CLAN.
const FREQ_S_FILE_ENG: GoldenCase = freq_eng_case!(
    "s_file_eng",
    &["+s@tests/fixtures/freq-include.cut"],
    &["--include-word-file", "tests/fixtures/freq-include.cut"],
    FilterSpec::None,
    "+s@F / -s@F"
);
const FREQ_NEG_S_FILE_ENG: GoldenCase = freq_eng_case!(
    "neg_s_file_eng",
    &["-s@tests/fixtures/freq-include.cut"],
    &["--exclude-word-file", "tests/fixtures/freq-include.cut"],
    FilterSpec::None,
    "+s@F / -s@F"
);

// CLAN `+t@ID="*|Target_Child|*"` selects participants by an `@ID` glob (a
// whole-string wildcard match, role being the 8th field). Manchester Anne's
// only Target_Child is CHI, so this selects CHI and byte-matches CLAN. chatter
// gets the equivalent `IdFilter` glob. Pins the id-filter glob fix.
const FREQ_T_ID_ENG: GoldenCase = golden_case!(
    "freq",
    "languages/manchester-anne.cha",
    "id_filter_target_child",
    &["+t@ID=*|Target_Child|*"],
    &[],
    FilterSpec::IdFilter("*|Target_Child|*"),
    "+t@ID=\"...\""
);

// CLAN FREQ `+d2` (onlydata=3) writes an aggregate SpreadsheetML file
// (`stat.frq.xls`), one row per (file x speaker) keyed by `@ID`, with per-word
// columns plus Types/Token/TTR. chatter's `+d2` reproduces every cell
// byte-identically EXCEPT the red TTR-caveat rows, where CLAN leaks a printf
// `%%mor`; chatter emits the correct `%mor` (CLAN-DIV-004). So the case is a
// documented divergence whose golden holds chatter's corrected SpreadsheetML.
// The CHI selection uses `+t*CHI` (equivalent to the manual's `+t@ID` for the
// Manchester fixtures; `+t@ID` id-filter glob is a separate deferred row).
const FREQ_D2_SPREADSHEET: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Spreadsheet {
        mode: talkbank_clan::commands::freq::FreqSpreadsheetMode::PerWord,
        extra_fixtures: &["languages/manchester-aran.cha"],
    },
    command: "freq",
    fixture: "languages/manchester-anne.cha",
    clan_args: &["+0", "+d2", "+t*CHI"],
    rust_args: &[],
    filter: FilterSpec::SpeakerInclude(&["CHI"]),
    case: "d2_spreadsheet_manchester",
    expectation: ParityExpectation::DivergesFromClan {
        rationale: DivergenceRef {
            ledger_row: "CLAN-DIV-004",
            book_anchor: "clan-reference/divergences/per-command.md#clan-div-004-freq-d2-d3-mor-caveat",
        },
    },
    covers: &[FlagRowRef {
        command: "freq",
        flag: "+d2",
    }],
};

// CLAN FREQ `+d3` (onlydata=4) is the same as `+d2` restricted to
// types/tokens/TTR (no per-word or speaker pseudo-word columns), written to
// `stat.frq0.xls`. Same CLAN-DIV-004 `%mor` divergence.
const FREQ_D3_SPREADSHEET: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Spreadsheet {
        mode: talkbank_clan::commands::freq::FreqSpreadsheetMode::TypesTokens,
        extra_fixtures: &["languages/manchester-aran.cha"],
    },
    command: "freq",
    fixture: "languages/manchester-anne.cha",
    clan_args: &["+0", "+d3", "+t*CHI"],
    rust_args: &[],
    filter: FilterSpec::SpeakerInclude(&["CHI"]),
    case: "d3_spreadsheet_manchester",
    expectation: ParityExpectation::DivergesFromClan {
        rationale: DivergenceRef {
            ledger_row: "CLAN-DIV-004",
            book_anchor: "clan-reference/divergences/per-command.md#clan-div-004-freq-d2-d3-mor-caveat",
        },
    },
    covers: &[FlagRowRef {
        command: "freq",
        flag: "+d3",
    }],
};

// CLAN FREQ `+d20` (`isSpreadsheetOnePerRow`, also onlydata=3) writes a flat
// `stat.frq.xls` with one row per (file, speaker, word): columns
// `File | Code | Word | Count`, no `@ID` columns, no Types/Token/TTR summary,
// and no `%mor` TTR caveat. Because it has no caveat rows, there is no
// `%%mor`-leak divergence: chatter's cells are byte-identical to CLAN's
// (verified on the Manchester fixtures), so this is MatchesClan, not a
// divergence like `+d2`/`+d3`.
const FREQ_D20_SPREADSHEET: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Spreadsheet {
        mode: talkbank_clan::commands::freq::FreqSpreadsheetMode::PerSpeakerWord,
        extra_fixtures: &["languages/manchester-aran.cha"],
    },
    command: "freq",
    fixture: "languages/manchester-anne.cha",
    clan_args: &["+0", "+d20", "+t*CHI"],
    rust_args: &[],
    filter: FilterSpec::SpeakerInclude(&["CHI"]),
    case: "d20_spreadsheet_manchester",
    expectation: ParityExpectation::MatchesClan,
    covers: &[FlagRowRef {
        command: "freq",
        flag: "+d20",
    }],
};

// CLAN FREQ `+dCN` (`onlydata = 4`, the percent-of-speakers filter): the
// `+d3`-shaped summary, but each speaker's Types/Token/TTR is computed over only
// the words used by `<`, `<=`, `=`, `>=`, or `>` than N percent of speakers, and
// the fourth column is labelled `Speaker` (not `+d2`/`+d3`'s `Code`,
// freq.cpp:2874). Written to `words.frq.xls`. `+d<=50` over manchester-anne's
// two speaker-rows (CHI, MOT) keeps words used by <= 1 speaker: CHI's words are
// all shared with MOT (count 2), so CHI collapses to 0/0/`-` while MOT keeps its
// 5 unique words (5/5/1.000). Verified byte-identical to CLAN's `words.frq.xls`
// data cells on a live run, modulo the same `%%mor` caveat as `+d2`/`+d3` (so
// DivergesFromClan, CLAN-DIV-004). The golden holds chatter's corrected output.
const FREQ_D_PERCENT_SPREADSHEET: GoldenCase = GoldenCase {
    kind: GoldenCaseKind::Spreadsheet {
        mode: talkbank_clan::commands::freq::FreqSpreadsheetMode::PercentOfSpeakers(
            talkbank_clan::commands::freq::SpeakerPercentFilter {
                comparison: talkbank_clan::commands::freq::SpeakerPercentComparison::LessOrEqual,
                percent: talkbank_clan::commands::freq::SpeakerPercent::new(50),
            },
        ),
        extra_fixtures: &[],
    },
    command: "freq",
    fixture: "languages/manchester-anne.cha",
    clan_args: &["+d<=50", "+t*CHI", "+t*MOT"],
    rust_args: &[],
    filter: FilterSpec::SpeakerInclude(&["CHI", "MOT"]),
    case: "d_percent_lt_eq_manchester",
    expectation: ParityExpectation::DivergesFromClan {
        rationale: DivergenceRef {
            ledger_row: "CLAN-DIV-004",
            book_anchor: "clan-reference/divergences/per-command.md#clan-div-004-freq-d2-d3-mor-caveat",
        },
    },
    covers: &[FlagRowRef {
        command: "freq",
        flag: "+dCN",
    }],
};

/// Every FREQ parity case, for the regen step to iterate.
const FREQ_GOLDEN_CASES: &[&GoldenCase] = &[
    &FREQ_BARE_ENG,
    &FREQ_D1_ENG,
    &FREQ_D4_ENG,
    &FREQ_C_ENG,
    &FREQ_O1_ENG,
    &FREQ_K_ENG,
    &FREQ_T_SPE_ENG,
    &FREQ_O_ENG,
    &FREQ_S_WORD_ENG,
    &FREQ_NEG_S_WORD_ENG,
    &FREQ_C1_ENG,
    &FREQ_NEG_T_ENG,
    &FREQ_T_ROLE_ENG,
    &FREQ_T_GRA_ENG,
    &FREQ_T_MOR_ENG,
    &FREQ_NEG_T_GRA_ENG,
    &FREQ_NEG_T_MOR_ENG,
    &FREQ_Z_RANGE_ENG,
    &FREQ_X_WORDLEN_ENG,
    &FREQ_X_CHARLEN_ENG,
    &FREQ_R6_RETRACE,
    &FREQ_R5_REPLACE,
    &FREQ_P_DELIM,
    &FREQ_T_ID_ENG,
    &FREQ_S_FILE_ENG,
    &FREQ_NEG_S_FILE_ENG,
    &FREQ_B_MATTR_ENG,
    &FREQ_S_MULTIWORD_ENG,
    &FREQ_C3_ANYORDER_ENG,
    &FREQ_C4_SOLE_ENG,
    &FREQ_C2_PERPATTERN_ENG,
    &FREQ_C7_MATCHED_ENG,
    &FREQ_D5_ZEROFREQ_ENG,
    &FREQ_O3_COMBINE_ENG,
    &FREQ_D2_SPREADSHEET,
    &FREQ_D3_SPREADSHEET,
    &FREQ_D20_SPREADSHEET,
    &FREQ_D_PERCENT_SPREADSHEET,
];

// Migrated from the legacy dual-snapshot baseline (task #5): commands whose
// CLAN-format render already byte-matches CLAN per the parity categorization
// audit, now enforced fail-closed. The 19 diverging commands stay as
// per-command parity work.
const MLT_MOR_GRA: GoldenCase =
    golden_case!("mlt", "tiers/mor-gra.cha", "mlt_mor_gra", &[], &[], FilterSpec::None, "(bare)");
const DIST_MOR_GRA: GoldenCase = golden_case!(
    "dist",
    "tiers/mor-gra.cha",
    "dist_mor_gra",
    &[],
    &[],
    FilterSpec::None,
    "(bare)"
);
const MAXWD_MOR_GRA: GoldenCase = golden_case!(
    "maxwd",
    "tiers/mor-gra.cha",
    "maxwd_mor_gra",
    &[],
    &[],
    FilterSpec::None,
    "(bare)"
);
const CHIP_MOR_GRA: GoldenCase = golden_case!(
    "chip",
    "tiers/mor-gra.cha",
    "chip_mor_gra",
    &[],
    &[],
    FilterSpec::None,
    "(bare)"
);
const TIMEDUR_BULLETS: GoldenCase = golden_case!(
    "timedur",
    "content/media-bullets.cha",
    "timedur_bullets",
    &[],
    &[],
    FilterSpec::None,
    "(bare)"
);
const KWAL_MOR_GRA: GoldenCase = golden_case!(
    "kwal",
    "tiers/mor-gra.cha",
    "kwal_mor_gra",
    &["+scookie"],
    &["--keyword", "cookie"],
    FilterSpec::None,
    // The cover string matches the book audit cell verbatim. KWAL's keyword row
    // documents both polarities in one cell (`+s"word"` / `-s"word"`), exactly
    // like FREQ's `+c / +c0`; the proving case covers that combined row.
    "+s\"word\" / -s\"word\""
);
const COMBO_MOR_GRA: GoldenCase = golden_case!(
    "combo",
    "tiers/mor-gra.cha",
    "combo_mor_gra",
    &["+swant"],
    &["--search", "want"],
    FilterSpec::None,
    // Matches COMBO's combined-polarity audit cell (`+sS` / `-sS`) verbatim,
    // mirroring KWAL above and FREQ's `+c / +c0`.
    "+sS / -sS"
);

/// Migrated baseline commands now enforced fail-closed (task #5).
const MIGRATED_GOLDEN_CASES: &[&GoldenCase] = &[
    &MLT_MOR_GRA,
    &DIST_MOR_GRA,
    &MAXWD_MOR_GRA,
    &CHIP_MOR_GRA,
    &TIMEDUR_BULLETS,
    &KWAL_MOR_GRA,
    &COMBO_MOR_GRA,
];

// MLU: the next depth-first command. The legacy categorization audit tagged its
// CLAN-format render as DIVERGE, so the bare golden below is expected to start
// RED against CLAN; chatter is then driven to byte parity (CLAN is
// authoritative). Each additional MLU flag becomes a further case here.
const MLU_MOR_GRA: GoldenCase =
    golden_case!("mlu", "tiers/mor-gra.cha", "mlu_mor_gra", &[], &[], FilterSpec::None, "(bare)");
// CLAN MLU `+o3` (mlu_isCombineSpeakers, mlu.cpp:721): pool every selected
// speaker into one `*COMBINED*` result. On mor-gra.cha, CHI (utt 1, morph 6) +
// MOT (utt 1, morph 5) combine to utt 2 / morph 11 / ratio 5.500 / SD 0.500.
// chatter byte-matches CLAN.
const MLU_O3_COMBINE: GoldenCase = golden_case!(
    "mlu",
    "tiers/mor-gra.cha",
    "mlu_o3_combine",
    &["+o3"],
    &["--combine-speakers"],
    FilterSpec::None,
    "+o3"
);

// CLAN MLU excludes xxx/yyy/www and the utterances in which they appear, by
// DEFAULT (manual §7.21 pt2: "the symbols xxx, yyy, and www are also excluded by
// default, as are the utterances in which they appear"; mllib.cpp:303-348
// `mlu_excludeUtter` returns TRUE for a standalone xxx/yyy/www token, called on
// the MAIN tier at mlu.cpp:509). On manchester-anne.cha `*CHI`, the utterance
// `it xxx xxx` (main tier carries `xxx`) is dropped ENTIRELY even though its
// `%mor` is just `pron|it`: CHI -> utt 2 / morph 3 / ratio 1.500 / SD 0.500
// (vs utt 3 / morph 4 if the xxx utterance were counted). Base-behavior golden
// (cover `(bare)`); chatter previously only excluded utterances that were
// WHOLLY unintelligible.
const MLU_XXX_EXCLUDE: GoldenCase = golden_case!(
    "mlu",
    "languages/manchester-anne.cha",
    "mlu_xxx_exclude",
    &["+t*CHI"],
    &[],
    FilterSpec::SpeakerInclude(&["CHI"]),
    "(bare)"
);

// CLAN MLU `+sxxx` re-includes the utterances that contain `xxx` (manual §7.21
// pt5: "the program stops excluding sentences that have xxx from the count, but
// still excludes the specific string xxx"; `ml_isXXXFound`, mlu.cpp:766, blanks
// the xxx token in `mlu_excludeUtter` instead of returning TRUE, mllib.cpp:337).
// The header switches to the two-line `+sxxx` variant (mlu.cpp:250-251). On
// manchester-anne.cha `*CHI`, the `it xxx xxx` utterance comes back: utt 3 /
// morph 4 / ratio 1.333 / SD 0.471 (the count rises by the `it` morpheme; `xxx`
// itself contributes nothing because it is not on the `%mor` tier). chatter
// flag `--include-xxx`.
const MLU_SXXX_INCLUDE: GoldenCase = golden_case!(
    "mlu",
    "languages/manchester-anne.cha",
    "mlu_sxxx_include",
    &["+t*CHI", "+sxxx"],
    &["--include-xxx"],
    FilterSpec::SpeakerInclude(&["CHI"]),
    "+sxxx"
);

// CLAN MLU excludes, by default, any utterance carrying the `[+ mlue]` postcode
// (`isMLUEpostcode` defaults TRUE, mlu.cpp:108; `isPostCodeOnUtt(line, "[+
// mlue]")` -> `isSkip = TRUE`, mlu.cpp:503). On the trimmed NINJAL-Okubo fixture
// the second `*CHI` utterance (6 `n:let` morphemes) is `[+ mlue]`-tagged and
// dropped, leaving utt 1 / morph 2 (the clean `shi@k u@k`). chatter previously
// counted it (utt 2 / morph 8). Base-behavior golden (cover `(bare)`).
const MLU_MLUE_POSTCODE: GoldenCase = golden_case!(
    "mlu",
    "languages/jpn-mlue-postcode.cha",
    "mlu_mlue_postcode_exclude",
    &["+t*CHI"],
    &[],
    FilterSpec::SpeakerInclude(&["CHI"]),
    "(bare)"
);

/// Every MLU parity case, for the regen step and the book metric to iterate.
const MLU_GOLDEN_CASES: &[&GoldenCase] = &[
    &MLU_MOR_GRA,
    &MLU_O3_COMBINE,
    &MLU_XXX_EXCLUDE,
    &MLU_SXXX_INCLUDE,
    &MLU_MLUE_POSTCODE,
];

/// Generate one fail-closed verify test per case (runs with no CLAN present).
macro_rules! golden_verify_tests {
    ($($name:ident => $case:path;)+) => {
        $(
            #[test]
            fn $name() {
                assert_clan_parity(&$case);
            }
        )+
    };
}

golden_verify_tests! {
    freq_bare_eng => FREQ_BARE_ENG;
    freq_d1_word_list_eng => FREQ_D1_ENG;
    freq_d4_types_tokens_eng => FREQ_D4_ENG;
    freq_c_capitalization_eng => FREQ_C_ENG;
    freq_o1_reverse_concordance_eng => FREQ_O1_ENG;
    // CLAN FREQ `+k` FOLDS case to lowercase (default preserves; `nomap`
    // toggle at cutt.cpp:13816). chatter matches after the FREQ case-polarity
    // fix; the golden is CLAN's lowercased `+k` output.
    freq_k_folds_case_eng => FREQ_K_ENG;
    freq_t_speaker_eng => FREQ_T_SPE_ENG;
    freq_o_descending_frequency_eng => FREQ_O_ENG;
    freq_s_include_word_eng => FREQ_S_WORD_ENG;
    freq_neg_s_exclude_word_eng => FREQ_NEG_S_WORD_ENG;
    freq_c1_mid_capitalization_eng => FREQ_C1_ENG;
    freq_neg_t_exclude_speaker_eng => FREQ_NEG_T_ENG;
    freq_t_role_eng => FREQ_T_ROLE_ENG;
    // +t%X dependent-tier scoping: count the whitespace-delimited tokens of the
    // named dependent tier. `+t%gra` keeps the TTR advisory (not %mor-based);
    // `+t%mor` suppresses it (isMorUsed, freq.cpp:1536).
    freq_t_gra_eng => FREQ_T_GRA_ENG;
    freq_t_mor_eng => FREQ_T_MOR_ENG;
    // -t%X dependent-tier EXCLUSION: count main + all dependent tiers except X.
    // `-t%gra` keeps the TTR advisory (the -t mode does not set isMorUsed even
    // though %mor is swept in); `-t%mor` likewise.
    freq_neg_t_gra_eng => FREQ_NEG_T_GRA_ENG;
    freq_neg_t_mor_eng => FREQ_NEG_T_MOR_ENG;
    // +x>3w word-count utterance filter (word unit only; regression golden, the
    // `+x` row stays Partial so this carries no metric `covers`).
    freq_x_wordlen_eng => FREQ_X_WORDLEN_ENG;
    freq_x_charlen_eng => FREQ_X_CHARLEN_ENG;
    // +r6 retracing inclusion / +r5 replacement-original (regression goldens; the
    // `+rN` row stays Partial so these carry no metric `covers`).
    freq_r6_retrace => FREQ_R6_RETRACE;
    freq_r5_replace => FREQ_R5_REPLACE;
    // +pS word delimiters: split a word at the extra delimiter chars and count
    // each segment (choo_choo -> choo). Byte-matches CLAN (MatchesClan).
    freq_p_word_delimiter_mot => FREQ_P_DELIM;
    freq_z_range_eng => FREQ_Z_RANGE_ENG;
    freq_id_filter_target_child_eng => FREQ_T_ID_ENG;
    freq_s_include_word_file_eng => FREQ_S_FILE_ENG;
    freq_neg_s_exclude_word_file_eng => FREQ_NEG_S_FILE_ENG;
    freq_b_mattr_eng => FREQ_B_MATTR_ENG;
    freq_s_multiword_eng => FREQ_S_MULTIWORD_ENG;
    freq_c3_anyorder_eng => FREQ_C3_ANYORDER_ENG;
    freq_c4_sole_eng => FREQ_C4_SOLE_ENG;
    freq_c2_perpattern_eng => FREQ_C2_PERPATTERN_ENG;
    freq_c7_matched_eng => FREQ_C7_MATCHED_ENG;
    // +d5 zeroMatch: literal `+s` words shown with count 0 when unmatched.
    freq_d5_zerofreq_eng => FREQ_D5_ZEROFREQ_ENG;
    // +o3: all speakers pooled into one combined table, no Speaker: header.
    freq_o3_combine_eng => FREQ_O3_COMBINE_ENG;
    // +d2/+d3 aggregate SpreadsheetML; the golden holds chatter's corrected
    // output (CLAN-DIV-004 %mor caveat). Verify needs no CLAN binary.
    freq_d2_spreadsheet_manchester => FREQ_D2_SPREADSHEET;
    freq_d3_spreadsheet_manchester => FREQ_D3_SPREADSHEET;
    // +d20 one-row-per-(file,speaker,word) SpreadsheetML; byte-matches CLAN
    // (no %mor caveat to diverge on), so MatchesClan. Verify needs no CLAN binary.
    freq_d20_spreadsheet_manchester => FREQ_D20_SPREADSHEET;
    // +dCN percent-of-speakers filter: +d3-shaped summary over the words used by
    // a comparator of N% of speakers, `Speaker` header, words.frq.xls. Diverges
    // only on the %mor caveat (CLAN-DIV-004). Verify needs no CLAN binary.
    freq_d_percent_lt_eq_manchester => FREQ_D_PERCENT_SPREADSHEET;
}

// Migrated baseline commands (task #5): byte-match CLAN, now fail-closed.
golden_verify_tests! {
    mlt_mor_gra_matches_clan => MLT_MOR_GRA;
    dist_mor_gra_matches_clan => DIST_MOR_GRA;
    maxwd_mor_gra_matches_clan => MAXWD_MOR_GRA;
    chip_mor_gra_matches_clan => CHIP_MOR_GRA;
    timedur_bullets_matches_clan => TIMEDUR_BULLETS;
    kwal_mor_gra_matches_clan => KWAL_MOR_GRA;
    combo_mor_gra_matches_clan => COMBO_MOR_GRA;
}

// MLU: active depth-first command (task #6).
golden_verify_tests! {
    mlu_mor_gra_matches_clan => MLU_MOR_GRA;
    mlu_o3_combine_matches_clan => MLU_O3_COMBINE;
    mlu_xxx_exclusion_matches_clan => MLU_XXX_EXCLUDE;
    mlu_sxxx_include_matches_clan => MLU_SXXX_INCLUDE;
    mlu_mlue_postcode_matches_clan => MLU_MLUE_POSTCODE;
}

/// Regenerate every committed golden from the private CLAN build. Ignored by
/// default; run via `just regen-clan-goldens` (needs `CLAN_BIN_DIR`).
#[test]
#[ignore = "regen: seeds golden files from the private CLAN build (needs CLAN_BIN_DIR)"]
fn regenerate_clan_goldens() {
    for &case in FREQ_GOLDEN_CASES
        .iter()
        .chain(MIGRATED_GOLDEN_CASES)
        .chain(MLU_GOLDEN_CASES)
    {
        seed_clan_golden(case);
    }
}
