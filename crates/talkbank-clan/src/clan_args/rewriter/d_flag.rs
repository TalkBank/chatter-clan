//! Per-subcommand routing for the `+d` flag.
//!
//! CLAN's `+d`/`+dN` is the most heavily command-overloaded flag in the suite:
//! it is the `onlydata` display-detail level for most analysis commands, an
//! Excel/spreadsheet toggle for a few (MODREP, KEYMAP, DSS), a string-argument
//! flag for others (EVAL, DATES, KIDEVAL), and a real chatter-mapped option for
//! FREQ / FREQPOS / COOCCUR / KWAL / LOWCASE. Because there is no generic
//! `+d`/`-d` fallback arm, an unmatched `+d` simply passes through unchanged
//! (clap then rejects the literal token at the file-discovery boundary).
//!
//! This routing was extracted verbatim from the giant `try_rewrite_clan_flag`
//! match (every arm here was a `(b'+', b'd') if subcommand == X => ...` arm
//! there). Each subcommand's behavior is documented inline with its CLAN-source
//! citation; the per-subcommand arms are mutually exclusive and any subcommand
//! not listed falls to the final `_ => None` pass-through, exactly as the
//! original interleaved arms did.

use super::ClanSubcommandKind;

/// Route a `+d<rest>` flag for `subcommand` to its rewritten argv tokens, or
/// `None` to leave the literal token in place (pass-through).
///
/// `rest` is the substring after the `+d` (so `+d2` has `rest == "2"`, bare
/// `+d` has `rest == ""`). The polarity is always `+` here: CLAN's `-d` is not
/// a recognized analysis flag, so only `(b'+', b'd')` reaches this routing in
/// the parent match.
pub(super) fn try_rewrite_d_flag(
    subcommand: ClanSubcommandKind,
    rest: &str,
) -> Option<Vec<String>> {
    use ClanSubcommandKind::*;
    match subcommand {
        // FREQPOS `+d` (no N) switches position classification
        // from first/last/other to first/second/other. Intercepted
        // before the generic +dN display-mode arm so the bare-`d`
        // form isn't lost to the empty-rest short-circuit.
        Freqpos if rest.is_empty() => {
            Some(vec!["--position-classification".into(), "second".into()])
        }

        // FREQPOS `+dN` passthrough, CLAN's `case 'd'` at
        // `OSX-CLAN/src/clan/freqpos.cpp` is a **no-arg flag**:
        // `DC = TRUE; no_arg_option(f);`. Any `+dN` form errors
        // in CLAN itself at `no_arg_option`. chatter has no
        // consumer; pass through so clap rejects with the
        // literal token rather than the misleading
        // `--display-mode N` rewrite from the catch-all.
        Freqpos => None,

        // COOCCUR `+d` (no N) strips the leading count column from
        // the output. Same empty-rest intercept pattern.
        Cooccur if rest.is_empty() => Some(vec!["--no-frequency-counts".into()]),

        // COOCCUR `+dN` passthrough, COOCCUR has **no local
        // `case 'd'`** in `OSX-CLAN/src/clan/cooccur.cpp`;
        // falls through to `maingetflag` for the shared
        // `onlydata`-level path via `cutt.cpp:9382`. chatter has
        // no `--display-mode` consumer for COOCCUR; pass through.
        Cooccur => None,

        // KWAL `+d` (no N) switches the output from CLAN's
        // location-annotated default to a legal CHAT fragment
        // (drop the `---` separator and `*** File ... Keyword: X`
        // decoration).
        Kwal if rest.is_empty() => Some(vec!["--legal-chat".into()]),

        // KWAL `+dN` passthrough, CLAN's `case 'd'` at
        // `OSX-CLAN/src/clan/kwal.cpp` has 7+ specific `+dN`
        // branches with break: `+d7` → `linkDep2Other = TRUE`;
        // `+d40` → `isDuplicateTiers`, `isKeywordOneColumn`,
        // `onlydata = 5`, `combinput` (CLAN_SRV-rejected);
        // `+d4` → `combinput`, `isKeywordOneColumn` (no break;
        // falls through into `case 's'`); `+d90` →
        // `isExpendX`/`isExpandXForAll`/`OverWriteFile`;
        // `+d99` → `isExpendX`; `+d30` → `outputOnlyMatched = 3`
        // plus various flag resets; `+d31` →
        // `outputOnlyMatched = 2`; `+d3` → `outputOnlyMatched = 1`.
        // All other `+dN` values fall through to `case 's'`
        // (search-pattern handling). None of these are display
        // modes; none have chatter consumers. Pass through.
        Kwal => None,

        // FREQ `+d1` emits one word per line with no frequency or
        // other info, suitable as a `kwal +s@FILE` input. Other
        // `+dN` FREQ display modes (0, 2..8) still fall through to
        // the generic `--display-mode N` rewrite below.
        Freq if rest == "1" => Some(vec!["--word-list-only".into()]),

        // FREQ `+d4` emits only per-speaker type/token/TTR summary
        // (no per-word entries). `+d3` (same content, spreadsheet
        // form) is a separate item that combines this with CSV
        // output.
        Freq if rest == "4" => Some(vec!["--types-tokens-only".into()]),

        // FREQ `+d3` (CLAN onlydata=4): the type/token/TTR-only aggregate
        // SpreadsheetML file. This is a real CLAN file-output slot, NOT the
        // chatter-only stdout CSV; it maps to the `--spreadsheet summary`
        // mode. (The earlier `--types-tokens-only --format csv` mapping
        // squatted the slot on a stdout convenience; un-squatted here.)
        Freq if rest == "3" => Some(vec!["--spreadsheet".into(), "summary".into()]),

        // FREQ `+d2` (CLAN onlydata=3): the per-word aggregate SpreadsheetML
        // file. Maps to `--spreadsheet per-word`. `--format csv` stays a
        // chatter-only stdout convenience, never a `+d2` target (faithfulness
        // rule: CLAN slots carry CLAN semantics).
        Freq if rest == "2" => Some(vec!["--spreadsheet".into(), "per-word".into()]),

        // FREQ `+d5` (CLAN zeroMatch, freq.cpp:894): show each LITERAL `+s`
        // search word even when it never matched, with count 0. Maps to the
        // `--include-zero-frequency` flag. CLAN rejects wildcards/duplicates in
        // `+s` under `+d5` (freq.cpp:444) and requires at least one `+s` word
        // (freq.cpp:449); those guards are enforced at the FREQ analysis
        // dispatch (analysis.rs), not in this stateless flag rewrite.
        Freq if rest == "5" => Some(vec!["--include-zero-frequency".into()]),

        // FREQ `+d20` (CLAN `onlydata = 3` + `isSpreadsheetOnePerRow`,
        // freq.cpp:881-887): the one-row-per-(file, speaker, word) SpreadsheetML
        // layout, sibling of `+d2` (per-word) / `+d3` (summary). Maps to the
        // `--spreadsheet per-speaker-word` mode. Matched as the exact token
        // `20`, distinct from `+d2`'s `2`.
        Freq if rest == "20" => Some(vec!["--spreadsheet".into(), "per-speaker-word".into()]),

        // FREQ `+dCN` percent-of-speakers type filter (CLAN `case 'd'` percent
        // branch, freq.cpp:841-878; `onlydata = 4`, the `statfreq_percent_result`
        // spreadsheet path). The manual's `C` (CLAN.html "+dCN") is a comparator
        // metavariable: `+d<N`/`+d<=N`/`+d=N`/`+d>=N`/`+d>N`, with `=<`/`=>` as
        // accepted spellings of `<=`/`>=`. Maps to chatter's
        // `--speaker-percentage <spec>` (a chatter-only flag name carrying the
        // CLAN slot's semantics; faithfulness rule). The raw `<spec>` (e.g.
        // `<=50`) is parsed + validated by that arg's value parser, which rejects
        // a missing or non-digit N exactly as CLAN does (freq.cpp:871-874).
        // Matched by the leading comparator so it never collides with the
        // digit-led `+d0`..`+d8`/`+d20` arms above.
        Freq if rest.starts_with(['<', '>', '=']) => {
            Some(vec!["--speaker-percentage".into(), rest.into()])
        }

        // FREQ `+dN` for all remaining unmapped values (bare `+d`,
        // `+d0`, `+d6`-`+d8`, percent forms `+d<=N` /
        // `+d>=N` / `+d<N` / `+d=N` / `+d>N`): local `case 'd'`
        // at `OSX-CLAN/src/clan/freq.cpp:838` is the richest in
        // CLAN, a percent-bounded type filter (`percentC`/`percent`)
        // and `+d8` cross-tabulation (`isCrossTabulation`), among others
        // (`+d20`/`+d2`/`+d3` ARE mapped above). chatter has no typed
        // consumer for any still-unmapped value yet.
        // Returning `None` leaves the literal token in place. clap
        // does NOT treat a `+`-prefixed token as an option, so the
        // token reaches the shared file-discovery boundary, which
        // rejects it fail-closed (`DiscoveredChatFiles::into_files`
        // -> `UnrecognizedClanFlagArgs`) rather than swallowing it as
        // a bogus file and exiting 0 with default output (the prior
        // fail-open bug). Adding typed consumers for these values is
        // per-row feature work tracked in the FREQ audit table.
        Freq => None,

        // LOWCASE `+d2`, "ignore dict file, lowercase everything",
        // per `OSX-CLAN/src/clan/lowcase.cpp` case 'd' (integer 0..=2
        // toggles dict-preserving / dict-capitalizing / ignore-dict).
        // chatter's `transforms/lowcase.rs` lowercases unconditionally,
        // matching the `+d2` semantic, so the flag is a no-op.
        // Intercepted before the generic `+dN → --display-mode N`
        // catch-all; lowcase has no `--display-mode` clap field.
        // `+d`/`+d0`/`+d1` (dict-using modes) are documented Missing
        // and intentionally still fall through to fail clap.
        Lowcase if rest == "2" => Some(vec![]),

        // CHAINS `+d`/`+dN`, `onlydata` output-detail level (0-1
        // per `OSX-CLAN/src/clan/chains.cpp:1089`: `+d` → 1,
        // `+d0` → 1, `+d1` → 2). chatter has no `--only-data`
        // flag; pass through so clap rejects the literal token
        // rather than the misleading `--display-mode` rewrite from
        // the catch-all below.
        Chains => None,

        // MODREP `+d`, no-arg Excel/spreadsheet toggle per
        // `OSX-CLAN/src/clan/modrep.cpp:1492` (`no_arg_option(f)`
        // + `isExcel = TRUE`). chatter has no `--format csv` for
        // MODREP; pass through.
        Modrep => None,

        // IPSYN `+d`/`+dN`, `onlydata` output-detail level
        // bounded by `OnlydataLimit` per `OSX-CLAN/src/clan/ipsyn.cpp:3945`.
        // chatter has no `--only-data` flag; pass through.
        Ipsyn => None,

        // TRNFIX `+d` is a bare-vs-non-bare toggle per
        // `OSX-CLAN/src/clan/TrnFix.cpp:132`: bare `+d` sets
        // `whichDopt = 1` (include speaker tier in output);
        // `+d<anything>` sets `whichDopt = 2` (also write a
        // mismatches-summary file). chatter has no consuming flag;
        // pass through so clap rejects the literal token rather
        // than the misleading `--display-mode` rewrite from the
        // catch-all below.
        Trnfix => None,

        // KEYMAP `+d`, no-arg Excel/spreadsheet toggle per
        // `OSX-CLAN/src/clan/keymap.cpp:834` (`no_arg_option(f)`
        // + `isExcel = TRUE`), identical shape to MODREP `+d`.
        // chatter has no `--format csv` for KEYMAP; pass through
        // so clap rejects the literal token (including malformed
        // `+dN` forms that would otherwise hit the catch-all and
        // surface as a misleading `--display-mode` error).
        Keymap => None,

        // DIST `+d`/`+dN`, `onlydata` output-detail level routed
        // through the shared `maingetflag` path at
        // `OSX-CLAN/src/clan/cutt.cpp:9382` via
        // `dist.cpp::getflag`'s `default:` (line 545). DIST is in
        // the per-program list at `cutt.cpp:9437` with an empty
        // body, confirming it consumes `+d` for the level effect.
        // chatter has no `--only-data` flag for DIST; pass through.
        Dist => None,

        // DSS `+d`, spreadsheet-output toggle with its own
        // `case 'd'` at `OSX-CLAN/src/clan/dss.cpp:2520` (bare `+d`
        // → `IsOutputSpreadsheet = 1`; `+d1` → `IsOutputSpreadsheet
        // = 2`). chatter has no `--format csv` for DSS; pass
        // through.
        Dss => None,

        // GEM `+d`, hybrid: `+d2` is a local override at
        // `OSX-CLAN/src/clan/gem.cpp:130` (sets
        // `onlySelectedBG_EGHeaders = TRUE`); every other `+dN`
        // value falls through to `maingetflag` at `cutt.cpp:9382`
        // (empty per-program body at `cutt.cpp:9470`), setting the
        // shared `onlydata` level. chatter has neither consumer;
        // pass through.
        Gem => None,

        // GEMFREQ `+d`, no local `case 'd'` in `gemfreq.cpp`;
        // `+d`/`+dN` is consumed entirely via `maingetflag` at
        // `cutt.cpp:9382` (empty per-program body at
        // `cutt.cpp:9471`), setting the shared `onlydata` level.
        // chatter has no `--display-mode` consumer on the `gemfreq`
        // clap surface; pass through.
        Gemfreq => None,

        // VOCD `+d`/`+dN`, `onlydata` output-detail level per
        // `OSX-CLAN/src/clan/vocd/vocd.cpp:311` (same `+1`-offset
        // pattern as chains/ipsyn; bounded by `OnlydataLimit`).
        // chatter has no `--display-mode` consumer for VOCD; pass
        // through.
        Vocd => None,

        // CHSTRING `+d`, bare-only "do not re-wrap tiers" per
        // `OSX-CLAN/src/clan/chstring.cpp:1087` (`NO_CHANGE =
        // TRUE`, `no_arg_option(f)`). chatter never wraps on
        // output; semantically a no-op. Pass through.
        Chstring => None,

        // CHIP `+d`/`+dN`, `onlydata`-level via shared
        // `maingetflag` path at `OSX-CLAN/src/clan/cutt.cpp:9382`
        // with non-empty per-program body at `cutt.cpp:9427`
        // (`onlydata == 2` → `puredata = 0`; CLAN_SRV rejects
        // `onlydata == 3`). chatter has no `--display-mode`
        // consumer for CHIP; pass through.
        Chip => None,

        // FLO `+d`, multi-value local at
        // `OSX-CLAN/src/clan/flo.cpp:197`: bare `+d` or `+d0` sets
        // `substitute_flag = 1` (flo line replaces main line);
        // `+d1` sets it to 2; `+d2` is a no-op; anything else
        // errors. chatter emits `%flo:` as a new dependent tier
        // alongside the main line, no main-line-substitute
        // consumer. Pass through.
        Flo => None,

        // MAXWD `+d`/`+dN`, `onlydata`-level via shared
        // `maingetflag` path at `cutt.cpp:9382` with non-empty
        // per-program body at `cutt.cpp:9475` (`onlydata == 1` →
        // `puredata = 0`). chatter has no `--display-mode`
        // consumer for MAXWD; pass through.
        Maxwd => None,

        // MLU/MLUMOR `+d`/`+dN`, `onlydata`-level via shared
        // `maingetflag` path at `cutt.cpp:9382` with non-empty
        // per-program body at `cutt.cpp:9485` (CLAN_SRV-only
        // rejection of `onlydata == 1 || 3`; otherwise pure
        // level effect). chatter has no `--display-mode`
        // consumer for MLU; pass through.
        Mlu => None,

        // MLT `+d`/`+dN`, `onlydata`-level via shared
        // `maingetflag` path at `cutt.cpp:9382` with non-empty
        // per-program body at `cutt.cpp:9478` (CLAN_SRV-only
        // rejection of `onlydata == 1`). chatter has no
        // `--display-mode` consumer for MLT; pass through.
        Mlt => None,

        // CHECK `+d`/`+dN`, no local `case 'd'` in
        // `OSX-CLAN/src/clan/check.cpp`; consumption via shared
        // `maingetflag` path at `cutt.cpp:9382` (CHECK_P has
        // `D_OPTION` per `cutt.cpp:8722`) with the CHECK-specific
        // per-program body at `cutt.cpp:9422` (`onlydata == 3` →
        // `puredata = 2`; else `puredata = 0`). The `onlydata`
        // level additionally short-circuits `check_adderror` at
        // `check.cpp:852` (`onlydata == 0 || 3` returns early,
        // skipping the error). chatter has no `--display-mode` or
        // `--suppress-repeats` consumer for CHECK; the existing
        // CHECK audit page documents the gap. Pass through so
        // clap rejects with the literal token rather than the
        // misleading `--display-mode` rewrite from the catch-all
        // below.
        Check => None,

        // COMBO `+d`/`+dN`/`+d7`/`+d8`/`+dv`, full local handler
        // at `OSX-CLAN/src/clan/combo.cpp:2858`. Four distinct
        // branches: `+dv`/`+dV` → `isEchoFlatmac = TRUE` (search
        // debug echo); `+d7` → `linkDep2Other = TRUE` (cross-tier
        // linkage); `+d8` → `onlydata = 9` (special override);
        // `+d`/`+d0`..`+d6` → `onlydata = atoi+1` with `+d2`
        // (onlydata==3) also resetting `puredata = 0`. chatter has
        // no consumer for any branch. Pass through so clap rejects
        // with the literal token rather than the misleading
        // `--display-mode` rewrite from the catch-all below.
        Combo => None,

        // WDSIZE `+d`/`+dN`, local `case 'd'` at
        // `OSX-CLAN/src/clan/wdsize.cpp:239` with intentional
        // fallthrough. Bare `+d` (empty rest) sets
        // `combinput = TRUE`, then falls through to `default:`
        // which calls `maingetflag` for the `onlydata`-level effect
        // via `cutt.cpp:9382`. `+dN` skips the combinput assignment
        // (rest non-empty) and falls straight to maingetflag.
        // chatter has no `--combine-input` or `--display-mode`
        // consumer for WDSIZE. Pass through so clap rejects with
        // the literal token rather than the misleading
        // `--display-mode` rewrite from the catch-all below.
        Wdsize => None,

        // WDLEN `+d`/`+dN`, same shape as WDSIZE at
        // `OSX-CLAN/src/clan/wdlen.cpp:322`: bare `+d` sets
        // `combinput = TRUE`, then falls through to `default:` →
        // `maingetflag` for the `onlydata`-level effect via
        // `cutt.cpp:9382`. chatter has no consumer for either
        // effect; pass through.
        Wdlen => None,

        // EVAL `+d`/`+dKEY`, local `case 'd'` at
        // `OSX-CLAN/src/clan/eval.cpp:3595`. Bare `+d` errors
        // ("Missing argument for option") and exits; `+dKEY`
        // calls `addDBKeys(KEY)` to register comma-separated DB
        // key names. Unlike WDSIZE/MLU/etc. this is *not* an
        // `onlydata`-level setter, `+d1` in CLAN means
        // `addDBKeys("1")`, treating "1" as a database key. The
        // catch-all's `--display-mode` rewrite would be doubly
        // wrong here (wrong semantics AND no chatter consumer).
        // Pass through so clap rejects with the literal token.
        Eval => None,

        // EVAL-D `+d`/`+dKEY`, identical `case 'd'` handler at
        // `OSX-CLAN/src/clan/eval-d.cpp:3565` to EVAL (same
        // `addDBKeys` string-arg semantics). chatter has no
        // consumer; pass through.
        EvalD => None,

        // TIMEDUR `+d`/`+dN`, local `case 'd'` at
        // `OSX-CLAN/src/clan/timedur.cpp:157`. IS an `onlydata`-
        // level setter but with TIMEDUR-specific semantics: bare
        // `+d` / `+d0` → `onlydata = 1`; `+d1` → `onlydata = 2`;
        // `+d10` → `onlydata = 3`; anything else errors;
        // duplicate `+d` also errors. CLAN_SRV additionally
        // rejects `onlydata == 1 || 3`. chatter has no
        // `--display-mode` consumer for TIMEDUR; pass through.
        Timedur => None,

        // DATES `+d`/`+dDATE`, local `case 'd'` at
        // `OSX-CLAN/src/clan/dates.cpp:837`. NOT a level setter
        //, `+dDATE` (or `+d DATE` two-token form, consuming the
        // next arg) calls `getdate(DATE)` to register a literal
        // date string. Same shape as EVAL: string-arg flag, not
        // numeric level. chatter has no consumer; pass through.
        Dates => None,

        // FLUCALC `+d`/`+dN<s|w>`, local `case 'd'` at
        // `OSX-CLAN/src/clan/flucalc.cpp:752`. Bare `+d` errors
        // ("Invalid argument for option"). `+dN<s|w>` parses N
        // as a sample size and the trailing character as a unit
        // (`s` = syllables, `w` = words); `+d100s` means "first
        // 100 syllables". Not a level setter, `+d1` in CLAN
        // would fail because `1` lacks the required unit suffix.
        // chatter has no consumer; pass through.
        Flucalc => None,

        // KIDEVAL `+d`/`+dTYPE~ARG`, local `case 'd'` at
        // `OSX-CLAN/src/clan/kideval.cpp:5245`. Bare `+d` errors
        // ("Missing argument for option"). `+dTYPE~ARG` parses
        // the string as a tilde-separated TYPE/ARG pair, with
        // TYPE prefixed by `_` and stored in `DB_type`. Same
        // string-arg shape as EVAL, just with internal `~`
        // structure. chatter has no consumer; pass through.
        Kideval => None,

        // RELY `+d`/`+dm[N]`/`+dN`, multi-mode local `case 'd'`
        // at `OSX-CLAN/src/clan/rely.cpp:243`. Three distinct
        // sub-modes in one switch arm:
        //   * bare `+d` → `isComputeAphasia = TRUE`
        //   * `+dm` / `+dm1` / `+dm2` → `isComputeStudent-
        //     Correctness` (1 for bare/`m1`, 2 for `m2`; any
        //     other `+dmX` errors)
        //   * `+dN` (digit) → `KappaCats = atoi(N)` with
        //     `KappaCats > 1` validation; `+d1` in CLAN would
        //     trigger the validation error.
        // chatter has no consumer for any of the three sub-modes.
        // Pass through.
        Rely => None,

        // SUGAR `+d`, no-arg debug toggle, local `case 'd'` at
        // `OSX-CLAN/src/clan/sugar.cpp:756`:
        // `no_arg_option(f); isDebug = TRUE`. Only bare `+d` is
        // valid in CLAN; `+dN` (non-empty rest) would fail
        // `no_arg_option`. The simplest `case 'd'` shape across
        // P-3, pure boolean flag. chatter has no `--debug`
        // consumer for SUGAR (the workflow already runs in CLI
        // debug context); pass through.
        Sugar => None,

        // UNIQ `+d5`/`+dN`, local `case 'd'` at
        // `OSX-CLAN/src/clan/uniq.cpp:238` with one special-cased
        // branch and a fallthrough:
        //   * `+d5` → `zeroMatch = TRUE` (special, suppresses
        //     fallthrough)
        //   * any other `+d` form → `maingetflag(f-2, f1, i)`
        //     for the `onlydata`-level effect via `cutt.cpp:9382`.
        // Same fallthrough family as WDSIZE/WDLEN, but with the
        // `+d5` intercept before the fallthrough. chatter has no
        // `--zero-match` or `--display-mode` consumer; pass
        // through.
        Uniq => None,

        // Every other subcommand: `+d` is not a recognized flag, so
        // the literal token passes through unchanged (matching the
        // original match's fall-through to `_ => None`).
        _ => None,
    }
}
