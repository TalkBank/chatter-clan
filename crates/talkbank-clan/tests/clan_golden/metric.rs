// Phase 2 golden-parity completeness metric: combine the book DENOMINATOR
// (book_audit's parsed flag-rows) with the golden NUMERATOR
// (`GoldenCase.covers`) into honest per-command numbers, plus the
// anti-overclaim gap (book rows claiming Done that no golden yet proves).
//
// Computed in the clan_golden test binary so it can read the golden coverage
// declarations directly. In a green build every golden case passes, so a
// covered flag-row is a proven flag-row.

use std::collections::BTreeSet;

/// The bare-invocation sentinel used in `GoldenCase.covers`. Not a CLAN flag,
/// so it is excluded from the flag-row denominator and numerator (a separate
/// "base behaviour proven" fact).
const BARE_SENTINEL: &str = "(bare)";

/// Per-command golden-parity completeness.
#[derive(Debug)]
struct CommandMetric {
    command: &'static str,
    /// Total CLAN flag-rows the book documents for the command (denominator).
    total_flag_rows: usize,
    /// Flag-rows with a passing `MatchesClan` golden (numerator).
    proven_flag_rows: usize,
    /// Flag-rows with a passing `DivergesFromClan` golden (also "done").
    diverged_flag_rows: usize,
    /// Rows the book claims Done but no golden proves: the overclaim gap that
    /// the acceptance-based audit could never see.
    claimed_done_unproven: Vec<String>,
    /// Covered flags that resolve to NO book flag-row: the mirror of
    /// `claimed_done_unproven` (a golden proves a flag the book doesn't list).
    /// A cover typo or book/cover drift; must be empty.
    dangling_covers: Vec<String>,
}

/// Collect the flag tokens `cases` cover, split by parity expectation, dropping
/// the bare sentinel. Assumes a green build (every case passes), so a covered
/// flag is a proven flag.
fn covered_flags(cases: &[&GoldenCase]) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut matches = BTreeSet::new();
    let mut diverges = BTreeSet::new();
    for case in cases {
        let bucket = match case.expectation {
            ParityExpectation::MatchesClan => &mut matches,
            ParityExpectation::DivergesFromClan { .. } => &mut diverges,
        };
        for cover in case.covers {
            if cover.flag != BARE_SENTINEL {
                bucket.insert(cover.flag.to_string());
            }
        }
    }
    (matches, diverges)
}

/// Compute the metric for one command from its book page + golden cases.
fn compute_command_metric(
    command: &'static str,
    book_markdown: &str,
    cases: &[&GoldenCase],
) -> CommandMetric {
    let rows = parse_audit_rows(book_markdown);
    let book_flags: BTreeSet<String> = rows.iter().map(|row| row.flag.clone()).collect();
    let (proven, diverged) = covered_flags(cases);

    let dangling: Vec<String> = proven
        .iter()
        .chain(diverged.iter())
        .filter(|flag| !book_flags.contains(flag.as_str()))
        .cloned()
        .collect();

    let proven_flag_rows = rows.iter().filter(|row| proven.contains(&row.flag)).count();
    let diverged_flag_rows = rows
        .iter()
        .filter(|row| diverged.contains(&row.flag))
        .count();
    let claimed_done_unproven = rows
        .iter()
        .filter(|row| {
            row.status == BookStatus::Done
                && !proven.contains(&row.flag)
                && !diverged.contains(&row.flag)
        })
        .map(|row| row.flag.clone())
        .collect();

    CommandMetric {
        command,
        // A `Deferred` row is a documented non-goal (e.g. FREQ `+o2`, whose
        // `chatmode=0` plain-text line counting is incompatible with chatter's
        // AST model), so it is excluded from the provable denominator rather than
        // counted as outstanding work that a golden could ever close.
        total_flag_rows: rows
            .iter()
            .filter(|row| row.status != BookStatus::Deferred)
            .count(),
        proven_flag_rows,
        diverged_flag_rows,
        claimed_done_unproven,
        dangling_covers: dangling,
    }
}

/// Read a command's book page and compute its golden-parity metric.
/// Centralizes the page path + read + parse shared by the metric tests and the
/// block renderer; pages live at `book/src/clan-reference/commands/<page>`.
fn load_command_metric(command: &'static str, page: &str, cases: &[&GoldenCase]) -> CommandMetric {
    let path = crate::common::workspace_root()
        .join("book/src/clan-reference/commands")
        .join(page);
    let md = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    compute_command_metric(command, &md, cases)
}

#[test]
fn freq_metric_covers_resolve_and_report_gap() {
    let metric = load_command_metric("freq", "freq.md", FREQ_GOLDEN_CASES);

    // Every golden cover (besides the bare sentinel) must resolve to a real
    // book flag-row, otherwise the numerator silently rots against book drift.
    assert!(
        metric.dangling_covers.is_empty(),
        "FREQ golden covers do not resolve to book flag-rows: {:?}",
        metric.dangling_covers
    );

    // The seven first-slice goldens prove six flag-rows (`(bare)` is not a
    // flag-row). The point of the honest metric: that is far fewer than the
    // rows the page claims Done, and the gap is now visible rather than hidden
    // behind the acceptance-based audit.
    assert!(
        metric.proven_flag_rows >= 6,
        "expected >= 6 proven FREQ flag-rows, got {}",
        metric.proven_flag_rows
    );
    assert!(metric.total_flag_rows >= 30);
    assert!(
        !metric.claimed_done_unproven.is_empty(),
        "freq.md claims Done rows that no golden yet proves; surfacing that gap is the goal"
    );
    eprintln!(
        "{}: {}/{} flag-rows proven by byte-parity golden ({} diverged-documented); \
         {} more claimed Done but unproven: {:?}",
        metric.command,
        metric.proven_flag_rows,
        metric.total_flag_rows,
        metric.diverged_flag_rows,
        metric.claimed_done_unproven.len(),
        metric.claimed_done_unproven
    );
}

/// Every command wired into the book metric (`METRIC_COMMANDS`, the single
/// source of truth) must resolve its golden covers to real book flag-rows. A
/// cover that does not match the audit-table cell verbatim rots as a *dangling
/// cover* and the honest metric silently undercounts it as 0 proven, a failure
/// `golden_parity_metric_block_is_current` cannot see (it only diffs the
/// rendered counts, not the dangling set). KWAL and COMBO additionally each
/// carry a real (non-bare) flag cover, so each must prove >= 1 flag-row, which
/// holds only when the cover matches the book's combined-polarity cell
/// (`+s"word" / -s"word"`, `+sS / -sS`), exactly as FREQ's `+c` cover is written
/// `+c / +c0`. The bare-only migrated commands prove 0 flag-rows (the bare
/// golden is base behaviour, not a flag-row), which is correct.
#[test]
fn metric_commands_covers_resolve_to_book_rows() {
    for &(command, page, cases) in METRIC_COMMANDS {
        let metric = load_command_metric(command, page, cases);
        assert!(
            metric.dangling_covers.is_empty(),
            "{command} golden covers do not resolve to book flag-rows \
             (the cover string must match the audit-table cell verbatim): {:?}",
            metric.dangling_covers
        );
        // KWAL/COMBO carry a real flag cover; the rest cover only the bare
        // sentinel, which is 0 flag-rows by design.
        let min_proven = match command {
            "kwal" | "combo" => 1,
            _ => 0,
        };
        assert!(
            metric.proven_flag_rows >= min_proven,
            "{command}: expected >= {min_proven} proven flag-rows, got {}",
            metric.proven_flag_rows
        );
    }
}

/// Commands with golden coverage, for the generated parity block:
/// (command name, book page filename, golden cases). Grows as commands get
/// golden cases; the metric computation itself is already command-general.
const METRIC_COMMANDS: &[(&str, &str, &[&GoldenCase])] = &[
    ("freq", "freq.md", FREQ_GOLDEN_CASES),
    ("mlt", "mlt.md", &[&MLT_MOR_GRA]),
    ("dist", "dist.md", &[&DIST_MOR_GRA]),
    ("maxwd", "maxwd.md", &[&MAXWD_MOR_GRA]),
    ("chip", "chip.md", &[&CHIP_MOR_GRA]),
    ("timedur", "timedur.md", &[&TIMEDUR_BULLETS]),
    ("kwal", "kwal.md", &[&KWAL_MOR_GRA]),
    ("combo", "combo.md", &[&COMBO_MOR_GRA]),
];

const METRIC_BEGIN: &str = "<!-- BEGIN GENERATED: golden-parity-metric -->";
const METRIC_END: &str = "<!-- END GENERATED: golden-parity-metric -->";

/// Render the per-command golden-parity table (no sentinel markers).
fn render_metric_table() -> String {
    let mut out = String::new();
    out.push_str(
        "| Command | Flag-rows proven (byte-parity golden) | Diverged (documented) | Claimed Done, no golden |\n",
    );
    out.push_str("|---|---|---|---|\n");
    for &(command, page, cases) in METRIC_COMMANDS {
        let metric = load_command_metric(command, page, cases);
        out.push_str(&format!(
            "| {} | {} / {} | {} | {} |\n",
            command.to_uppercase(),
            metric.proven_flag_rows,
            metric.total_flag_rows,
            metric.diverged_flag_rows,
            metric.claimed_done_unproven.len(),
        ));
    }
    out
}

/// Generated-block gate: the golden-parity table in `parity-status.md` between
/// the sentinel markers must equal the freshly-computed table. Run with
/// `UPDATE_PARITY_METRIC=1` to regenerate the book block; otherwise (CI) this
/// asserts the committed block is current and fails RED if it has gone stale.
#[test]
fn golden_parity_metric_block_is_current() {
    let path = crate::common::workspace_root().join("book/src/clan-reference/parity-status.md");
    let doc = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let begin = doc
        .find(METRIC_BEGIN)
        .unwrap_or_else(|| panic!("{METRIC_BEGIN} marker missing in parity-status.md"));
    let end = doc
        .find(METRIC_END)
        .unwrap_or_else(|| panic!("{METRIC_END} marker missing in parity-status.md"));
    let content_start = begin + METRIC_BEGIN.len();
    let current = &doc[content_start..end];
    let desired = format!("\n{}", render_metric_table());

    if std::env::var("UPDATE_PARITY_METRIC").is_ok() {
        let updated = format!("{}{}{}", &doc[..content_start], desired, &doc[end..]);
        std::fs::write(&path, updated).expect("write parity-status.md");
        eprintln!("regenerated golden-parity-metric block in parity-status.md");
    } else {
        assert_eq!(
            current, desired,
            "golden-parity-metric block in parity-status.md is stale; \
             regenerate with UPDATE_PARITY_METRIC=1"
        );
    }
}
