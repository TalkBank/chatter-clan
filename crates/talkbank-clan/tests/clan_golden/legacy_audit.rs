// Migration audit for task #5: categorize the legacy `parity_case_tests!`
// (dual-snapshot, fail-open) commands as MATCH vs DIVERGE against real CLAN,
// comparing chatter's CLAN-format render (`render(OutputFormat::Clan)`, the
// real CLI default) to the stripped CLAN output, the same comparison the
// fail-closed mechanism enforces.
//
// MATCH commands are ready to migrate to committed-golden fail-closed cases
// (and feed the parity metric). DIVERGE commands are the per-command parity
// backlog (each a FREQ-sized push). This is a one-shot categorization, run
// with CLAN_BIN_DIR set; it asserts nothing, it reports.
//
// VOCD is intentionally excluded: its D-statistic uses random sampling, so its
// output is non-deterministic and cannot be byte-compared.

/// (command, fixture, CLAN `+`-args, chatter `--`-args). This list is the
/// surviving record of the retired dual-snapshot `baseline.rs` parity rows
/// (filters all `None` there); that file and its `variants_*` / `check`
/// siblings were deleted when the never-committed-`@clan`-baseline mechanism
/// was retired, so this self-contained list is now the canonical backlog. The
/// 7 MATCH commands from the first audit run were migrated to fail-closed
/// goldens and removed from this list, so a re-run reports the outstanding
/// DIVERGE backlog rather than re-flagging already-migrated commands.
const LEGACY_BASELINE_CASES: &[(&str, &str, &[&str], &[&str])] = &[
    ("mlu", "tiers/mor-gra.cha", &[], &[]),
    ("wdlen", "tiers/mor-gra.cha", &[], &[]),
    ("freqpos", "tiers/mor-gra.cha", &[], &[]),
    ("cooccur", "tiers/mor-gra.cha", &[], &[]),
    ("gemlist", "core/headers-episodes.cha", &[], &[]),
    ("phonfreq", "tiers/pho.cha", &[], &[]),
    ("modrep", "tiers/pho.cha", &["+b%mod", "+c%pho"], &[]),
    ("codes", "tiers/coding.cha", &[], &[]),
    ("chains", "tiers/coding.cha", &[], &[]),
    ("sugar", "tiers/mor-gra.cha", &[], &[]),
    ("sugar", "languages/eng-conversation.cha", &[], &[]),
    (
        "trnfix",
        "tiers/pho.cha",
        &["+b%pho", "+c%mod"],
        &["--tier1", "pho", "--tier2", "mod"],
    ),
    ("uniq", "core/basic-conversation.cha", &[], &[]),
    ("dss", "languages/eng-conversation.cha", &[], &[]),
    ("eval", "languages/eng-conversation.cha", &[], &[]),
    ("flucalc", "annotation/retrace.cha", &[], &[]),
    ("ipsyn", "languages/eng-conversation.cha", &[], &[]),
    ("kideval", "languages/eng-conversation.cha", &[], &[]),
    (
        "keymap",
        "tiers/coding.cha",
        &["+s$NOM", "+d%cod"],
        &["--keyword", "$NOM", "--tier", "cod"],
    ),
];

#[test]
#[ignore = "audit: categorize legacy commands as MATCH/DIVERGE vs CLAN (needs CLAN_BIN_DIR)"]
fn audit_legacy_command_parity() {
    let mut matched = Vec::new();
    let mut diverged = Vec::new();
    let mut skipped = Vec::new();

    for &(command, fixture, clan_args, rust_args) in LEGACY_BASELINE_CASES {
        let file = crate::harness::corpus_file(fixture);
        let Some(clan) = crate::harness::run_clan(command, &file, clan_args) else {
            skipped.push(command);
            continue;
        };
        let rust = crate::harness::run_rust_filtered(
            command,
            &file,
            rust_args,
            crate::harness::OutputFormat::Clan,
            None,
        );
        if clan.trim_end() == rust.trim_end() {
            matched.push(command);
        } else {
            diverged.push(command);
        }
    }

    eprintln!("== legacy command parity audit (chatter render(Clan) vs CLAN) ==");
    eprintln!("MATCH ({}): {matched:?}", matched.len());
    eprintln!("DIVERGE ({}): {diverged:?}", diverged.len());
    if !skipped.is_empty() {
        eprintln!("SKIP (CLAN binary missing) ({}): {skipped:?}", skipped.len());
    }
}
