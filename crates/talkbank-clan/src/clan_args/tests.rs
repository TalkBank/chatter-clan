use super::*;

fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

/// Assert that `rewrite_clan_args` leaves the given invocation
/// byte-for-byte unchanged, the per-command pattern shared by
/// every passthrough arm. Pre-arm a passthrough test should
/// fail with the rewrite the arm is intended to suppress;
/// post-arm it passes by returning the input verbatim.
fn assert_passthrough(invocation: &str) {
    let input = args(invocation);
    let result = rewrite_clan_args(&input);
    assert_eq!(result, input);
}

#[test]
fn speaker_include() {
    let input = args("clan analyze freq +t*CHI file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq --speaker CHI file.cha"));
}

#[test]
fn speaker_exclude() {
    let input = args("clan analyze freq -t*MOT file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --exclude-speaker MOT file.cha")
    );
}

#[test]
fn multiple_speakers() {
    let input = args("clan analyze freq +t*CHI +t*MOT file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --speaker CHI --speaker MOT file.cha")
    );
}

/// CLAN silently treats `+tCHI` (no `*` sigil) the same as
/// `+t*CHI`, the sigil is implicit when the first character is
/// not `*`, `%`, or `@`. chatter must do the same so a user
/// pasting `freq +tCHI file.cha` from a CLAN script reaches the
/// `--speaker` field, not the fallthrough that drops the flag.
/// Asymmetrically true for `-tCHI` → `--exclude-speaker CHI`.
#[test]
fn speaker_include_no_asterisk() {
    let input = args("clan analyze freq +tCHI file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq --speaker CHI file.cha"));
}

#[test]
fn speaker_exclude_no_asterisk() {
    let input = args("clan analyze freq -tMOT file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --exclude-speaker MOT file.cha")
    );
}

#[test]
fn tier_include() {
    let input = args("clan analyze freq +t%mor file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq --tier mor file.cha"));
}

#[test]
fn tier_exclude() {
    let input = args("clan analyze freq -t%gra file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --exclude-tier gra file.cha")
    );
}

#[test]
fn search_word_quoted() {
    let input: Vec<String> = vec![
        "clan".into(),
        "analyze".into(),
        "freq".into(),
        "+s\"want\"".into(),
        "file.cha".into(),
    ];
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --include-word want file.cha")
    );
}

#[test]
fn search_word_unquoted() {
    let input = args("clan analyze freq +swant file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --include-word want file.cha")
    );
}

#[test]
fn exclude_word() {
    let input = args("clan analyze freq -swant file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --exclude-word want file.cha")
    );
}

/// FREQ has no `+g`/`-g` gem flag (CLAN rejects it; gem-limiting is the GEM
/// program). The rewriter routes both polarities to the `--reject-clan-gem`
/// sentinel, which the FREQ dispatch turns into a CLAN-style error. chatter's
/// gem convenience is reached via `--gem` / `--exclude-gem` directly.
#[test]
fn freq_gem_include_flag_rejected() {
    let input = args("clan analyze freq +gstory file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --reject-clan-gem story file.cha")
    );
}

#[test]
fn freq_gem_exclude_flag_rejected() {
    let input = args("clan analyze freq -gstory file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --reject-clan-gem story file.cha")
    );
}

#[test]
fn utterance_range() {
    let input = args("clan analyze freq +z25-125 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq --range 25-125 file.cha"));
}

#[test]
fn mlu_minus_bw_to_words() {
    // CLAN `-bw` on MLU/MLT switches the counting unit from
    // morphemes to words. The audit page lists this as a Done
    // mapping (`-bw` → `--words`), but the rewriter had no arm
    // for `-bw`, only a stale comment. clap parsed `-bw` as a
    // short-flag-with-value form and errored on the unknown
    // `-b`. This test guards the new Mlu/Mlt-scoped arm.
    let input = args("clan analyze mlu -bw file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze mlu --words file.cha"));
}

#[test]
fn mlt_minus_bw_to_words() {
    let input = args("clan analyze mlt -bw file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze mlt --words file.cha"));
}

#[test]
fn freq_minus_bw_unchanged() {
    // The `-bw` rewrite is scoped to MLU/MLT, other commands
    // don't share the morphemes-vs-words counting axis, so
    // `-bw` should fall through unchanged for them.
    assert_passthrough("clan analyze freq -bw file.cha");
}

#[test]
fn recurse_flag_dropped() {
    // CLAN `+re` requests subdirectory recursion. chatter
    // recurses by default for directory args, so the flag is a
    // global no-op. Without this drop, `+re` survives the
    // rewriter and lands in the path-arg list, triggering a
    // confusing `Warning: "+re" is not a file or directory`.
    let input = args("clan analyze freq +re corpus/");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq corpus/"));
}

#[test]
fn cooccur_sort_flag_dropped() {
    // CLAN COOCCUR `+o` enables a frequency-descending sort over
    // the cluster table. The semantic is encoded in
    // `OSX-CLAN/src/clan/cooccur.cpp`: `case 'o': isSort = TRUE;`
    // at line 337 toggles a BST whose invariant ("larger num_occ
    // goes left") makes in-order traversal emit clusters by
    // descending count.
    //
    // chatter's COOCCUR finalize step at
    // `crates/talkbank-clan/src/commands/cooccur.rs:292` already
    // sorts unconditionally by `count` descending (then
    // alphabetically as tiebreak), so `+o` is a no-op on the
    // chatter side. Drop the token rather than passing it to
    // clap, which would land it in the path-arg list and emit
    // `Warning: "+o" is not a file or directory`.
    let input = args("clan analyze cooccur +o file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze cooccur file.cha"));
}

#[test]
fn freq_o_routes_to_sort_frequency() {
    // CLAN FREQ `+o` (bare) requests descending-frequency sort
    // (freq.cpp:176, 815-817), mapped to chatter's `--sort frequency`.
    let input = args("clan analyze freq +o file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq --sort frequency file.cha"));
}

#[test]
fn freq_o0_routes_to_sort_frequency() {
    // CLAN FREQ `+o0` is the explicit form of `+o` (same
    // descending-frequency-sort semantic, freq.cpp:815-817).
    let input = args("clan analyze freq +o0 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq --sort frequency file.cha"));
}

#[test]
fn freq_o1_routes_to_sort_reverse_concordance() {
    // Regression guard: the `+o`/`+o0` frequency-sort arms must not
    // shadow the `+o1 → --sort reverse-concordance` arm. Match-arm
    // ordering matters here.
    let input = args("clan analyze freq +o1 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --sort reverse-concordance file.cha")
    );
}

#[test]
fn combtier_bare_tier_routes_to_tier_not_speaker() {
    // CLAN COMBTIER `+tS` selects the tier label to combine
    // (e.g. `+tcom` for `%com`) per `OSX-CLAN/src/clan/combtier.cpp`
    // usage: "+tS: Combine all tiers S into one tier." This
    // overrides the analysis-command convention where `+tCHI`
    // means "speaker filter", so the per-Combtier intercept
    // routes the bareword form to `--tier` instead of letting
    // `rewrite_tier_speaker`'s fallback emit `--speaker`.
    let input = args("clan analyze combtier +tcom file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze combtier --tier com file.cha"));
}

#[test]
fn combtier_percent_tier_form_still_works() {
    // Regression guard: the existing `+t%X → --tier X` rewrite
    // (via the `%` branch in `rewrite_tier_speaker`) must
    // continue to fire for COMBTIER too, so `combtier +t%com`
    // produces the same `--tier com` as the bareword form.
    // The combtier-specific intercept added for the bareword
    // case must not shadow the `%`-prefix path.
    let input = args("clan analyze combtier +t%com file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze combtier --tier com file.cha"));
}

#[test]
fn lowcase_d2_dropped() {
    // CLAN LOWCASE `+d2` = "ignore dict, lowercase everything"
    // per `OSX-CLAN/src/clan/lowcase.cpp` case 'd' (integer 0..=2
    // toggles dict-preserving / dict-capitalizing / ignore-dict).
    // chatter's `transforms/lowcase.rs` lowercases unconditionally,
    // matching the `+d2` semantic, no-op rewrite.
    let input = args("clan analyze lowcase +d2 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze lowcase file.cha"));
}

/// CHAINS `+d`/`+d0`/`+d1` are `onlydata` output-detail levels
/// per `OSX-CLAN/src/clan/chains.cpp:1089`, real CLAN behavior
/// chatter does not implement. The per-CHAINS rewriter arm
/// passes the token through unchanged so clap reports a clean
/// "unexpected argument '+d1'" error instead of the misleading
/// "--display-mode" rewrite from the catch-all.
#[test]
fn chains_dn_passes_through() {
    assert_passthrough("clan chains +d1 file.cha");
}

/// Bare `+d` on CHAINS also passes through.
#[test]
fn chains_d_bare_passes_through() {
    assert_passthrough("clan chains +d file.cha");
}

/// MODREP `+d` is a no-arg Excel toggle per
/// `OSX-CLAN/src/clan/modrep.cpp:1492`. Per-MODREP arm passes
/// it through; no `--format csv` for MODREP in chatter.
#[test]
fn modrep_d_passes_through() {
    assert_passthrough("clan modrep +d file.cha");
}

/// IPSYN `+d`/`+dN` are `onlydata` levels per
/// `OSX-CLAN/src/clan/ipsyn.cpp:3945`. Per-IPSYN arm passes
/// them through; no `--only-data` flag in chatter.
#[test]
fn ipsyn_dn_passes_through() {
    assert_passthrough("clan ipsyn +d1 file.cha");
}

/// TRNFIX `+d` (bare) sets `whichDopt = 1` and `+d<anything>`
/// sets `whichDopt = 2` per `OSX-CLAN/src/clan/TrnFix.cpp:132`
///, a bare-vs-non-bare toggle controlling speaker-tier
/// inclusion and a mismatches-summary file. chatter has no
/// consuming flag; pass through so clap reports a clean
/// "unexpected argument" error instead of the misleading
/// "--display-mode" rewrite from the catch-all.
#[test]
fn trnfix_d_bare_passes_through() {
    assert_passthrough("clan trnfix +d file.cha");
}

/// Non-bare TRNFIX `+dN` (`whichDopt = 2` branch) also passes
/// through unchanged.
#[test]
fn trnfix_dn_passes_through() {
    assert_passthrough("clan trnfix +d1 file.cha");
}

/// KEYMAP `+d` is a no-arg Excel/spreadsheet toggle per
/// `OSX-CLAN/src/clan/keymap.cpp:834` (`no_arg_option(f)` +
/// `isExcel = TRUE`), identical shape to MODREP `+d`. chatter
/// has no `--format csv` for KEYMAP; the per-KEYMAP rewriter
/// arm passes the token through so clap rejects the literal
/// flag.
#[test]
fn keymap_d_bare_passes_through() {
    assert_passthrough("clan keymap +d file.cha");
}

/// `+d1` for KEYMAP is malformed input, CLAN errors because
/// `no_arg_option` rejects any character following `+d`. Without
/// the per-KEYMAP arm, the generic catch-all rewrites `+d1` to
/// `--display-mode 1` and clap produces the misleading
/// "unexpected argument '--display-mode'" error. The per-KEYMAP
/// arm intercepts so the literal token survives to clap.
#[test]
fn keymap_dn_passes_through() {
    assert_passthrough("clan keymap +d1 file.cha");
}

/// DIST `+d`/`+dN` are `onlydata` output-detail levels routed
/// through the shared `maingetflag` path at
/// `OSX-CLAN/src/clan/cutt.cpp:9382`, `dist.cpp::getflag`'s
/// `default:` branch (line 545) delegates unknown flags to
/// `maingetflag`, which consumes `+d` when `option_flags[DIST] &
/// D_OPTION` is set (DIST appears in the per-program branch list
/// at `cutt.cpp:9437` with empty body, confirming DIST consumes
/// `+d` for its `onlydata` level effect). chatter has no
/// `--only-data` flag for DIST; per-DIST arm passes the token
/// through.
#[test]
fn dist_d_bare_passes_through() {
    assert_passthrough("clan dist +d file.cha");
}

/// Non-bare DIST `+dN` also passes through unchanged (currently
/// the catch-all rewrites it misleadingly to `--display-mode N`).
#[test]
fn dist_dn_passes_through() {
    assert_passthrough("clan dist +d1 file.cha");
}

/// DSS `+d` is a spreadsheet-output toggle with its own
/// `case 'd'` at `OSX-CLAN/src/clan/dss.cpp:2520` (bare `+d` →
/// `IsOutputSpreadsheet = 1`; `+d1` → `IsOutputSpreadsheet = 2`).
/// chatter has no `--format csv` for DSS; per-DSS arm passes
/// the token through.
#[test]
fn dss_d_bare_passes_through() {
    assert_passthrough("clan dss +d file.cha");
}

/// Non-bare DSS `+dN` (the `IsOutputSpreadsheet = 2` branch) also
/// passes through unchanged.
#[test]
fn dss_dn_passes_through() {
    assert_passthrough("clan dss +d1 file.cha");
}

/// GEM `+d2` is a local override at
/// `OSX-CLAN/src/clan/gem.cpp:130` (sets
/// `onlySelectedBG_EGHeaders = TRUE`); every other `+dN` value
/// falls through to the shared `maingetflag` path at
/// `cutt.cpp:9382` with empty per-program body (`cutt.cpp:9470`),
/// setting the `onlydata` level. chatter has neither
/// consumer; per-GEM arm passes through both forms.
#[test]
fn gem_d_bare_passes_through() {
    assert_passthrough("clan gem +d file.cha");
}

/// Non-bare GEM `+dN` (including the `+d2` local override and
/// the maingetflag-routed `+d0`/`+d1`) passes through unchanged.
#[test]
fn gem_dn_passes_through() {
    assert_passthrough("clan gem +d1 file.cha");
}

/// GEMFREQ has no local `case 'd'`; `+d`/`+dN` is consumed
/// entirely via the shared `maingetflag` path
/// (`cutt.cpp:9382`) with empty per-program body
/// (`cutt.cpp:9471`). chatter's `gemfreq` clap surface has no
/// `--display-mode` consumer; per-GEMFREQ arm passes through.
#[test]
fn gemfreq_d_bare_passes_through() {
    assert_passthrough("clan gemfreq +d file.cha");
}

/// Non-bare GEMFREQ `+dN` also passes through.
#[test]
fn gemfreq_dn_passes_through() {
    assert_passthrough("clan gemfreq +d1 file.cha");
}

/// GEMFREQ `-wS` is the exclude-word polarity per CLAN's
/// `gemfreq.cpp:296` (`case 'w': *(f-1) = 's'` then
/// `maingetflag`, i.e. CLAN rewrites `w` to `s` literally,
/// so `-wS` becomes `-sS` which is the exclude-word semantic).
/// chatter's clap `-w` short is `--include-word` (OPPOSITE
/// polarity), so without a per-gemfreq rewriter arm `-wS` is
/// silently mis-routed to include-word. Per-GEMFREQ arm routes
/// `-wS` → `--exclude-word S` to match CLAN's intent.
#[test]
fn gemfreq_minus_w_routes_to_exclude_word() {
    let input = args("clan gemfreq --gem TEST -wfoo file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan gemfreq --gem TEST --exclude-word foo file.cha")
    );
}

/// FLUCALC `+u` enables per-utterance output in CLAN
/// (`flucalc.cpp:778-781`, `isUttList = TRUE; no_arg_option(f)`).
/// chatter has only `--per-file` (file granularity), not
/// per-utterance, audit page status "Partial". Without a
/// per-flucalc arm, the generic `+u` arm at the global level
/// silently drops the flag (`Some(vec![])`) and chatter runs
/// with default aggregated output, user thinks they got
/// per-utterance results, actually got aggregated. The
/// per-flucalc arm returns None for honest rejection.
#[test]
fn flucalc_u_passes_through_for_honest_rejection() {
    assert_passthrough("clan flucalc +u file.cha");
}

/// MAXWD `+g1` / `+g2` / `+g3` are utterance-mode metric
/// selectors ("find longest utterance instead of longest word;
/// N selects metric: 1=morph, 2=word, 3=char"). chatter does
/// not implement utterance-mode yet (audit page status
/// "Missing"). Without per-command arms, `+g3` etc. fall
/// through to `rewrite_gem` and become `--gem 3` (literal gem
/// name), silently mis-routing. Pass-through (None) makes clap
/// reject the unimplemented flag honestly. Same pattern as
/// the combo `+g1`/`+g2`/`+g6` arms.
#[test]
fn maxwd_g1_passes_through_not_misrouted_to_gem() {
    assert_passthrough("clan maxwd +g1 file.cha");
}

#[test]
fn maxwd_g3_passes_through_not_misrouted_to_gem() {
    assert_passthrough("clan maxwd +g3 file.cha");
}

/// LAB2CHAT `+tN` is "Movie segment start time offset" per
/// `book/src/commands/lab2chat.md:69`. chatter
/// does not implement movie-segment offsets (audit page status
/// "Missing"). Without a per-command arm, `+t3` falls through
/// to `rewrite_tier_speaker` (default branch) and becomes
/// `--speaker 3`, silently mis-routing to LAB-CHAT speaker
/// labeling. Pass-through (None) makes clap reject the
/// digit-only `+tN` form honestly. Letter forms like `+tCHI`
/// are not lab2chat semantics either but are out of scope
/// here.
#[test]
fn lab2chat_t_digit_passes_through_not_speaker() {
    assert_passthrough("clan lab2chat +t3 file.lab");
}

/// COMBO `+g1` (string-oriented whole-tier search), `+g2`
/// (string-oriented single-word search), and `+g6` (include
/// tier code name in search) are unimplemented in chatter
/// (audit page status "Missing"). Without per-command arms,
/// they fall through to the generic `+g` → `rewrite_gem` arm
/// and get silently re-routed to `--gem 1` / `--gem 2` /
/// `--gem 6`, clap accepts those as literal gem names but the
/// user's intent (a search-mode switch) is lost. The per-
/// command arms preempt the gem-rewrite by returning None,
/// so the literal `+g1` token passes through to clap which
/// rejects it honestly. Same pattern as the existing
/// chstring `+d` passthrough arm.
#[test]
fn combo_g1_passes_through_not_misrouted_to_gem() {
    assert_passthrough("clan combo --search the +g1 file.cha");
}

#[test]
fn combo_g2_passes_through_not_misrouted_to_gem() {
    assert_passthrough("clan combo --search the +g2 file.cha");
}

#[test]
fn combo_g6_passes_through_not_misrouted_to_gem() {
    assert_passthrough("clan combo --search the +g6 file.cha");
}

/// CHAT2SRT `+v` is the first "subcommand alias" rewrite:
/// CLAN's chat2srt unifies SRT and WebVTT output under one
/// command, flipped by `+v`; chatter splits the two formats
/// into sibling subcommands `chat2srt` (SRT) and `chat2vtt`
/// (WebVTT). The `resolve_subcommand_alias` pre-pass swaps the
/// subcommand token and removes the trigger flag before the
/// per-arg rewriter runs.
/// Subprocess regression guard:
/// `legacy_chat2srt_v_switches_to_chat2vtt`.
#[test]
fn chat2srt_v_resolves_to_chat2vtt() {
    let input = args("clan chat2srt +v file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan chat2vtt file.cha"));
}

/// `chat2srt` without `+v` passes through unchanged (no
/// subcommand alias triggers).
#[test]
fn chat2srt_without_v_stays_chat2srt() {
    assert_passthrough("clan chat2srt file.cha");
}

/// CHAT2ELAN `+e.EXT` (with the CLAN-canonical leading dot)
/// rewrites to `--media-extension EXT` (bare). The leading-dot
/// strip is the semantic bridge between CLAN's verbatim-suffix
/// convention and chatter's auto-prepend-dot convention.
/// Subprocess regression guard:
/// `legacy_chat2elan_e_routes_to_media_extension`.
#[test]
fn chat2elan_e_dotted_strips_leading_dot() {
    let input = args("clan chat2elan +e.wav file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan chat2elan --media-extension wav file.cha")
    );
}

/// CHAT2ELAN `+eEXT` (without dot) routes verbatim to
/// `--media-extension EXT`.
#[test]
fn chat2elan_e_bare_routes_directly() {
    let input = args("clan chat2elan +ewav file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan chat2elan --media-extension wav file.cha")
    );
}

/// CHSTRING `+b` is "work only on text right of the colon (CHAT
/// format)" per `OSX-CLAN/src/clan/chstring.cpp:1120` (`case 'b':
/// lineonly = TRUE; no_arg_option(f)`). chatter's `chstring`
/// already only mutates main-tier word content (never speaker
/// codes or header/dependent-tier text), so `+b` is semantically
/// a no-op. Without this arm `+b` falls through to clap, where
/// the bare `+`-prefixed token is consumed as the positional
/// `<PATH>` slot, orphaning the real `.cha` file.
#[test]
fn chstring_b_drops_redundant_main_tier_only_flag() {
    let input = args("clan chstring --changes c.txt +b file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan chstring --changes c.txt file.cha"));
}

/// CHSTRING `+lx` is "do not show the list of changes" per
/// `OSX-CLAN/src/clan/chstring.cpp:1108-1111` (`case 'l': if (*f
/// == 'x') { DispChanges = FALSE; }`). chatter never prints a
/// changes-list (operates silently by design), so `+lx` is
/// semantically a no-op. Same fall-through-to-positional bug as
/// `+b` without this arm.
#[test]
fn chstring_lx_drops_redundant_silent_flag() {
    let input = args("clan chstring --changes c.txt +lx file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan chstring --changes c.txt file.cha"));
}

/// CHSTRING `-w` is "string-oriented search and replacement"
/// per `OSX-CLAN/src/clan/chstring.cpp:1145-1147` (`case 'w': if
/// (*f == EOS) stringOriented = 1`). chatter's word-leaf
/// replacement is already string-oriented by default, so `-w`
/// is semantically a no-op. Unlike `+b`/`+lx`, the bare `-w`
/// form fails by clap rejecting `-w` directly as an unknown
/// short flag rather than falling through to the positional.
#[test]
fn chstring_w_drops_redundant_string_oriented_flag() {
    let input = args("clan chstring --changes c.txt -w file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan chstring --changes c.txt file.cha"));
}

/// GEMFREQ `+o` is a no-value sort-by-descending-frequency flag
/// in CLAN (`OSX-CLAN/src/clan/gemfreq.cpp:260`: `isSort = TRUE;
/// no_arg_option(f)`). chatter's `gemfreq` (which adapts to
/// `freq --gem`) already sorts by descending frequency by
/// default, `+o` would be a no-op semantic but without this
/// arm the rewriter doesn't touch it, clap doesn't know `+o`,
/// and `+o` falls through to the positional `<PATH>` slot
/// (causing the "not a file or directory, skipping" warning
/// and silently dropping the flag from the invocation). The
/// per-command arm consumes-and-drops it cleanly.
#[test]
fn gemfreq_o_drops_redundant_sort_flag() {
    let input = args("clan gemfreq --gem TEST +o file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan gemfreq --gem TEST file.cha"));
}

/// VOCD `+d`/`+dN` are `onlydata` output-detail levels per
/// `OSX-CLAN/src/clan/vocd/vocd.cpp:311`
/// (`onlydata = atoi(getfarg(...))+1`, bounded by
/// `OnlydataLimit`, with `onlydata == 4` rejected in CLAN_SRV
/// builds). Audit lists `+d`, `+d1`, `+d2`, `+d3` as documented
/// levels. chatter has no `--display-mode` consumer for VOCD;
/// per-VOCD arm passes through.
#[test]
fn vocd_d_bare_passes_through() {
    assert_passthrough("clan vocd +d file.cha");
}

/// Non-bare VOCD `+dN` also passes through (strict-RED case).
#[test]
fn vocd_dn_passes_through() {
    assert_passthrough("clan vocd +d1 file.cha");
}

/// CHSTRING `+d` is bare-only per
/// `OSX-CLAN/src/clan/chstring.cpp:1087` (sets
/// `NO_CHANGE = TRUE`, "do not re-wrap tiers"; calls
/// `no_arg_option(f)` so anything following errors).
/// chatter never wraps on output, semantically a no-op.
/// Per-CHSTRING arm passes through.
#[test]
fn chstring_d_bare_passes_through() {
    assert_passthrough("clan chstring +d file.cha");
}

/// Malformed CHSTRING `+dN` (CLAN errors per `no_arg_option`)
/// passes through unchanged via the per-CHSTRING arm rather
/// than hitting the misleading `--display-mode` rewrite.
#[test]
fn chstring_dn_passes_through() {
    assert_passthrough("clan chstring +d1 file.cha");
}

/// CHIP has no local `case 'd'`; `+d`/`+dN` is consumed via the
/// shared `maingetflag` path at `cutt.cpp:9382` with non-empty
/// per-program body at `cutt.cpp:9427` (`onlydata == 2` →
/// `puredata = 0`; CLAN_SRV rejects `onlydata == 3`). Same
/// `onlydata`-level semantic as the empty-body commands;
/// chatter has no `--display-mode` consumer for CHIP. Per-CHIP
/// arm passes through.
#[test]
fn chip_d_bare_passes_through() {
    assert_passthrough("clan chip +d file.cha");
}

/// Non-bare CHIP `+dN` (strict-RED case).
#[test]
fn chip_dn_passes_through() {
    assert_passthrough("clan chip +d1 file.cha");
}

/// FLO `+d` has multi-value local semantics at
/// `OSX-CLAN/src/clan/flo.cpp:197`:
/// - bare `+d` or `+d0` → `substitute_flag = 1` (flo line
///   replaces main line)
/// - `+d1` → `substitute_flag = 2`
/// - `+d2` → no-op (empty branch)
/// - anything else → CLAN errors
///
/// chatter emits `%flo:` as a new dependent tier alongside the
/// main line; no main-line-substitute consumer. Per-FLO arm
/// passes through.
#[test]
fn flo_d_bare_passes_through() {
    assert_passthrough("clan flo +d file.cha");
}

/// Non-bare FLO `+dN` (strict-RED case).
#[test]
fn flo_dn_passes_through() {
    assert_passthrough("clan flo +d1 file.cha");
}

/// MAXWD has no local `case 'd'`; consumption via shared
/// `maingetflag` path at `OSX-CLAN/src/clan/cutt.cpp:9382`
/// with non-empty per-program body at `cutt.cpp:9475`
/// (`onlydata == 1` → `puredata = 0`). Same `onlydata`-level
/// semantic; chatter has no `--display-mode` consumer for
/// MAXWD. Per-MAXWD arm passes through.
#[test]
fn maxwd_d_bare_passes_through() {
    assert_passthrough("clan maxwd +d file.cha");
}

/// Non-bare MAXWD `+dN` (strict-RED case).
#[test]
fn maxwd_dn_passes_through() {
    assert_passthrough("clan maxwd +d1 file.cha");
}

/// MLU/MLUMOR have no local `case 'd'`; consumption via shared
/// `maingetflag` path at `cutt.cpp:9382` with non-empty
/// per-program body at `cutt.cpp:9485` (`onlydata == 1 || 3`
/// rejected only under CLAN_SRV; otherwise pure level effect).
/// chatter has no `--display-mode` consumer for MLU.
#[test]
fn mlu_d_bare_passes_through() {
    assert_passthrough("clan mlu +d file.cha");
}

/// Non-bare MLU `+dN` (strict-RED case).
#[test]
fn mlu_dn_passes_through() {
    assert_passthrough("clan mlu +d1 file.cha");
}

/// MLT has no local `case 'd'`; consumption via shared
/// `maingetflag` path at `cutt.cpp:9382` with non-empty
/// per-program body at `cutt.cpp:9478` (`onlydata == 1`
/// rejected only under CLAN_SRV). chatter has no
/// `--display-mode` consumer for MLT.
#[test]
fn mlt_d_bare_passes_through() {
    assert_passthrough("clan mlt +d file.cha");
}

/// Non-bare MLT `+dN` (strict-RED case).
#[test]
fn mlt_dn_passes_through() {
    assert_passthrough("clan mlt +d1 file.cha");
}

/// COMBO has a full local `case 'd'` at `combo.cpp:2858` with
/// four branches (`+dv`, `+d7`, `+d8`, and the generic
/// `+d`/`+dN` onlydata-level path). chatter has no consumer
/// for any branch. Per-COMBO arm passes them all through.
/// Bare `+d` is the regression guard (catch-all already
/// returns None for empty rest, so this passes pre-arm too).
#[test]
fn combo_d_bare_passes_through() {
    assert_passthrough("clan combo +d file.cha");
}

/// Non-bare COMBO `+dN` (strict-RED). Pre-arm, this rewrites
/// to `["--display-mode", "1"]` which clap then mis-suggests
/// as `--tui-mode` (no `--display-mode` consumer exists). The
/// arm restores the literal-flag error path.
#[test]
fn combo_dn_passes_through() {
    assert_passthrough("clan combo +d1 file.cha");
}

/// CHECK has no local `case 'd'`; consumption via shared
/// `maingetflag` path at `cutt.cpp:9382` with the CHECK-
/// specific per-program body at `cutt.cpp:9422`
/// (`onlydata == 3` → `puredata = 2`; else `puredata = 0`)
/// and additional short-circuit at `check.cpp:852`. chatter
/// has no `--display-mode` / `--suppress-repeats` consumer
/// for CHECK. Per-CHECK arm passes through.
#[test]
fn check_d_bare_passes_through() {
    assert_passthrough("clan check +d file.cha");
}

/// Non-bare CHECK `+dN` (strict-RED).
#[test]
fn check_dn_passes_through() {
    assert_passthrough("clan check +d1 file.cha");
}

/// WDSIZE has a local `case 'd'` at
/// `OSX-CLAN/src/clan/wdsize.cpp:239` with intentional
/// fallthrough: bare `+d` (empty rest) sets `combinput = TRUE`,
/// then falls into `default:` which calls `maingetflag` for the
/// `onlydata`-level effect via `cutt.cpp:9382`. `+dN` skips the
/// combinput assignment and falls straight to maingetflag.
/// chatter has no `--combine-input` or `--display-mode`
/// consumer for WDSIZE. Bare `+d` is the regression guard
/// (catch-all already returns None for empty rest, so this
/// passes pre-arm too).
#[test]
fn wdsize_d_bare_passes_through() {
    assert_passthrough("clan wdsize +d file.cha");
}

/// Non-bare WDSIZE `+dN` (strict-RED). Pre-arm, the catch-all
/// rewrites to `["--display-mode", "1"]` which clap then
/// mis-suggests as `--tui-mode` (no `--display-mode` consumer
/// exists). The arm restores the literal-flag error path.
#[test]
fn wdsize_dn_passes_through() {
    assert_passthrough("clan wdsize +d1 file.cha");
}

/// WDLEN has the same `case 'd'` fallthrough at
/// `OSX-CLAN/src/clan/wdlen.cpp:322` as WDSIZE, bare `+d`
/// sets `combinput = TRUE`, then falls through to `default:`
/// → `maingetflag`. chatter has no consumer for either effect.
/// Bare `+d` is the regression guard.
#[test]
fn wdlen_d_bare_passes_through() {
    assert_passthrough("clan wdlen +d file.cha");
}

/// Non-bare WDLEN `+dN` (strict-RED).
#[test]
fn wdlen_dn_passes_through() {
    assert_passthrough("clan wdlen +d1 file.cha");
}

/// EVAL has a local `case 'd'` at
/// `OSX-CLAN/src/clan/eval.cpp:3595`: bare `+d` errors with
/// "Missing argument for option" and exits; `+dKEY` calls
/// `addDBKeys(KEY)` (string-arg, comma-separated DB key list).
/// Unlike WDSIZE/MLU/etc. this is not an `onlydata`-level
/// setter at all, `+d1` in CLAN is `addDBKeys("1")`, not a
/// display mode. chatter has no `--db-keys` consumer. Pass
/// through. Bare `+d` is the regression guard (catch-all
/// already returns None for empty rest).
#[test]
fn eval_d_bare_passes_through() {
    assert_passthrough("clan eval +d file.cha");
}

/// Non-bare EVAL `+dN` (strict-RED). Pre-arm, the catch-all
/// rewrites to `["--display-mode", "1"]` which clap then
/// mis-suggests as `--tui-mode`. In CLAN this would be
/// `addDBKeys("1")`, entirely unrelated to display mode.
#[test]
fn eval_dn_passes_through() {
    assert_passthrough("clan eval +d1 file.cha");
}

/// EVAL-D has the same `case 'd'` handler as EVAL at
/// `OSX-CLAN/src/clan/eval-d.cpp:3565` (both share the
/// `addDBKeys` string-arg semantics). Bare `+d` regression
/// guard.
#[test]
fn evald_d_bare_passes_through() {
    assert_passthrough("clan eval-d +d file.cha");
}

/// Non-bare EVAL-D `+dN` (strict-RED).
#[test]
fn evald_dn_passes_through() {
    assert_passthrough("clan eval-d +d1 file.cha");
}

/// TIMEDUR has a local `case 'd'` at
/// `OSX-CLAN/src/clan/timedur.cpp:157` that IS an
/// `onlydata`-level setter but with TIMEDUR-specific
/// semantics: bare `+d` / `+d0` → `onlydata = 1`; `+d1` →
/// `onlydata = 2`; `+d10` → `onlydata = 3`; anything else
/// errors. Duplicate `+d` also errors. CLAN_SRV additionally
/// rejects `onlydata == 1 || 3`. chatter has no
/// `--display-mode` consumer for TIMEDUR. Bare `+d` is the
/// regression guard.
#[test]
fn timedur_d_bare_passes_through() {
    assert_passthrough("clan timedur +d file.cha");
}

/// Non-bare TIMEDUR `+dN` (strict-RED).
#[test]
fn timedur_dn_passes_through() {
    assert_passthrough("clan timedur +d1 file.cha");
}

/// DATES has a local `case 'd'` at
/// `OSX-CLAN/src/clan/dates.cpp:837` that is *not* a level
/// setter, `+dDATE` (or `+d DATE` two-token form) calls
/// `getdate(DATE)` to register a literal date string. Same
/// general shape as EVAL: `+d` takes a string argument, not
/// a numeric level. chatter has no `--date-filter` or
/// `--display-mode` consumer; pass through. Bare `+d` is
/// the regression guard.
#[test]
fn dates_d_bare_passes_through() {
    assert_passthrough("clan dates +d file.cha");
}

/// Non-bare DATES `+dN` (strict-RED). In CLAN this would
/// be `getdate("1")`, entirely unrelated to display mode;
/// the catch-all's rewrite would be doubly wrong.
#[test]
fn dates_dn_passes_through() {
    assert_passthrough("clan dates +d1 file.cha");
}

/// FLUCALC has a local `case 'd'` at
/// `OSX-CLAN/src/clan/flucalc.cpp:752`. Bare `+d` errors
/// ("Invalid argument for option"); `+dN<s|w>` parses N as a
/// sample size and the trailing character as a unit (`s` =
/// syllables, `w` = words). Example: `+d100s` means "first
/// 100 syllables". Not a level setter, `+d1` in CLAN would
/// fail because `1` lacks the required unit suffix. chatter
/// has no `--sample-size`/`--sample-unit` consumer; pass
/// through. Bare `+d` is the regression guard.
#[test]
fn flucalc_d_bare_passes_through() {
    assert_passthrough("clan flucalc +d file.cha");
}

/// Non-bare FLUCALC `+dN` (strict-RED).
#[test]
fn flucalc_dn_passes_through() {
    assert_passthrough("clan flucalc +d1 file.cha");
}

/// KIDEVAL has a local `case 'd'` at
/// `OSX-CLAN/src/clan/kideval.cpp:5245`. Bare `+d` errors
/// ("Missing argument for option"); `+dTYPE~ARG` parses the
/// string as a tilde-separated TYPE/ARG pair, with TYPE
/// prefixed by `_` and stored in `DB_type`. Not a level
/// setter, `+d1` in CLAN would attempt to parse "1" as
/// TYPE~ARG and error because there's no `~` separator.
/// chatter has no consumer; pass through. Bare `+d` is the
/// regression guard.
#[test]
fn kideval_d_bare_passes_through() {
    assert_passthrough("clan kideval +d file.cha");
}

/// Non-bare KIDEVAL `+dN` (strict-RED).
#[test]
fn kideval_dn_passes_through() {
    assert_passthrough("clan kideval +d1 file.cha");
}

/// RELY has a multi-mode local `case 'd'` at
/// `OSX-CLAN/src/clan/rely.cpp:243`. Three distinct sub-modes
/// in one switch arm:
///   * bare `+d` (EOS)        → `isComputeAphasia = TRUE`
///   * `+dm` / `+dm1` / `+dm2` → `isComputeStudentCorrectness`
///     (1 for bare/`m1`, 2 for `m2`; any other `+dmX` errors)
///   * `+dN` (digit)          → `KappaCats = atoi(N)` with a
///     `KappaCats > 1` validation; otherwise errors.
///
/// chatter has no `--compute-aphasia`/`--student-correctness`/
/// `--kappa-categories` consumer for any of the three sub-
/// modes. Bare `+d` is the regression guard.
#[test]
fn rely_d_bare_passes_through() {
    assert_passthrough("clan rely +d file.cha");
}

/// Non-bare RELY `+dN` (strict-RED). In CLAN this would
/// be `KappaCats = 1` → validation error; `--display-mode 1`
/// rewrite would be doubly wrong (wrong semantics + no
/// chatter consumer).
#[test]
fn rely_dn_passes_through() {
    assert_passthrough("clan rely +d1 file.cha");
}

/// SUGAR has the simplest possible local `case 'd'` at
/// `OSX-CLAN/src/clan/sugar.cpp:756`:
/// `no_arg_option(f); isDebug = TRUE`. Pure no-arg debug
/// toggle, only bare `+d` is valid in CLAN; `+dN` (non-
/// empty rest) would fail `no_arg_option`. chatter has no
/// `--debug` consumer for SUGAR (the workflow already runs
/// in CLI debug context); pass through. Bare `+d` is the
/// regression guard.
#[test]
fn sugar_d_bare_passes_through() {
    assert_passthrough("clan sugar +d file.cha");
}

/// Non-bare SUGAR `+dN` (strict-RED). In CLAN this errors
/// at `no_arg_option`; the catch-all's `--display-mode 1`
/// rewrite would mask the real "no-arg flag with arg"
/// rejection behind a misleading `--tui-mode` suggestion.
#[test]
fn sugar_dn_passes_through() {
    assert_passthrough("clan sugar +d1 file.cha");
}

/// UNIQ has a local `case 'd'` at
/// `OSX-CLAN/src/clan/uniq.cpp:238` with one special-cased
/// branch and a fallthrough:
///   * `+d5` → `zeroMatch = TRUE`
///   * any other `+d` form → `maingetflag(f-2, f1, i)`,
///     i.e. the `onlydata`-level path via `cutt.cpp:9382`.
///
/// Same fallthrough family as WDSIZE/WDLEN but with a `+d5`
/// intercept before the fallthrough. chatter has no
/// `--zero-match` or `--display-mode` consumer; pass
/// through. Bare `+d` is the regression guard.
#[test]
fn uniq_d_bare_passes_through() {
    assert_passthrough("clan uniq +d file.cha");
}

/// Non-bare UNIQ `+dN` (strict-RED).
#[test]
fn uniq_dn_passes_through() {
    assert_passthrough("clan uniq +d1 file.cha");
}

/// KWAL bare `+d` regression: must still route to
/// `--legal-chat` via the existing arm at line ~407.
/// The new non-bare-`+d` passthrough arm must not steal
/// the empty-rest case.
#[test]
fn kwal_d_bare_still_routes_to_legal_chat() {
    let input = args("clan kwal +d file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan kwal --legal-chat file.cha"));
}

/// KWAL `+dN` passthrough (strict-RED). CLAN's `case 'd'`
/// at `OSX-CLAN/src/clan/kwal.cpp` has 7+ specific `+dN`
/// branches (`+d3`, `+d4`, `+d7`, `+d30`, `+d31`, `+d40`,
/// `+d90`, `+d99`) plus a fallthrough into `case 's'` for
/// unmatched values. None are display modes; none have
/// chatter consumers. The catch-all `--display-mode N`
/// rewrite is wrong for all of them.
#[test]
fn kwal_dn_passes_through() {
    assert_passthrough("clan kwal +d1 file.cha");
}

/// COOCCUR bare `+d` regression: must still route to
/// `--no-frequency-counts` via the existing arm at line ~389.
#[test]
fn cooccur_d_bare_still_routes_to_no_frequency_counts() {
    let input = args("clan cooccur +d file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan cooccur --no-frequency-counts file.cha"));
}

/// COOCCUR `+dN` passthrough (strict-RED). COOCCUR has NO
/// local `case 'd'` in `OSX-CLAN/src/clan/cooccur.cpp`;
/// falls through to `maingetflag` for the shared
/// `onlydata`-level path via `cutt.cpp:9382`. chatter has
/// no `--display-mode` consumer for COOCCUR.
#[test]
fn cooccur_dn_passes_through() {
    assert_passthrough("clan cooccur +d1 file.cha");
}

/// FREQPOS bare `+d` regression: must still route to
/// `--position-classification second` via the existing arm
/// at line ~383.
#[test]
fn freqpos_d_bare_still_routes_to_position_classification() {
    let input = args("clan freqpos +d file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan freqpos --position-classification second file.cha")
    );
}

/// FREQPOS `+dN` passthrough (strict-RED). CLAN's
/// `case 'd'` at `OSX-CLAN/src/clan/freqpos.cpp` is a
/// **no-arg flag**: `DC = TRUE; no_arg_option(f)`. Any
/// `+dN` form errors in CLAN itself at `no_arg_option`.
/// chatter has no consumer; the catch-all's
/// `--display-mode N` rewrite would mask the real
/// "no-arg flag with arg" rejection.
#[test]
fn freqpos_dn_passes_through() {
    assert_passthrough("clan freqpos +d1 file.cha");
}

#[test]
fn include_retracings() {
    let input = args("clan analyze mlu +r6 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze mlu --include-retracings file.cha")
    );
}

#[test]
fn merge_noop() {
    let input = args("clan analyze freq +u file.cha");
    let result = rewrite_clan_args(&input);
    // +u is a no-op (merge is default), so it's dropped
    assert_eq!(result, args("clan analyze freq file.cha"));
}

/// FREQ `+dN` values not mapped by a specific arm
/// (`+d1`/`+d2`/`+d3`/`+d4`) now pass through. CLAN's
/// `case 'd'` at `freq.cpp:690` has rich semantics for the
/// other values (`+d5` zeroMatch, `+d6`, `+d8` cross-
/// tabulation, `+d20` per-row spreadsheet, percent-bounded
/// `+d<=N`/`+d>=N`/...). chatter has no typed consumer for
/// any of them; the FREQ-specific catch-all arm at line ~471
/// passes them through so clap rejects the literal token
/// rather than the misleading `--display-mode N` rewrite.
/// Replaces the prior `display_mode_fallback` test that
/// pinned the now-dead catch-all behavior.
#[test]
fn freq_dn_unmapped_passes_through() {
    assert_passthrough("clan analyze freq +d6 file.cha");
}

#[test]
fn case_sensitive() {
    let input = args("clan analyze freq +k file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq --case-sensitive file.cha"));
}

/// FREQ's `+c` (and `+c0` alias) is the "count only capitalised
/// words" filter. CLAN treats them identically; chatter routes
/// both to `--capitalization initial`.
#[test]
fn freq_capitalized_only_bare() {
    let input = args("clan freq +c file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --capitalization initial file.cha"));
}

/// `+c0` is FREQ's documented alias for `+c`; same rewriter
/// target. Pinned separately so a future regression on either
/// spelling fails its own test.
#[test]
fn freq_capitalized_only_zero_suffix() {
    let input = args("clan freq +c0 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --capitalization initial file.cha"));
}

/// FREQ's `+c1` is the mid-word-uppercase variant: only count
/// words with an uppercase letter AFTER position 0
/// (e.g. `McDonald`, `iPhone`).
#[test]
fn freq_capitalized_mid_uppercase() {
    let input = args("clan freq +c1 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --capitalization mid file.cha"));
}

/// COOCCUR's `+d` (no N) strips the leading count column from
/// the output. Distinct from the generic `+dN` display-mode
/// rewrite, COOCCUR-specific arm intercepts before the
/// empty-rest fall-through.
#[test]
fn cooccur_cluster_size() {
    let input = args("clan cooccur +n3 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan cooccur --cluster-size 3 file.cha"));
}

#[test]
fn cooccur_no_frequency_counts() {
    let input = args("clan cooccur +d file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan cooccur --no-frequency-counts file.cha"));
}

/// FREQPOS's `+d` (no N) switches position classification
/// from first/last/other to first/second/other. Distinct from
/// the generic `+dN` display-mode rewrite (FREQPOS-specific
/// arm intercepts before the generic +dN routing).
#[test]
fn freqpos_second_mode_classification() {
    let input = args("clan freqpos +d file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan freqpos --position-classification second file.cha")
    );
}

/// `+d` under non-FREQPOS subcommands continues to fall
/// through to the generic display-mode handler (which itself
/// returns None for empty rest). Pinned with a different
/// subcommand to ensure scope-narrowing.
#[test]
fn freq_d_bare_does_not_match_position_classification() {
    // `+d` with empty rest under FREQ doesn't get rewritten,
    // it stays in the argv as-is (downstream clap will error
    // since there's no `+d` consumer).
    assert_passthrough("clan freq +d file.cha");
}

/// FREQ's `+o1` is the reverse-concordance sort: words are
/// sorted by their reversed character sequence (so words with
/// the same suffix cluster together). Routes to
/// `--sort reverse-concordance`.
#[test]
fn freq_reverse_concordance_sort() {
    let input = args("clan freq +o1 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan freq --sort reverse-concordance file.cha")
    );
}

/// FREQ's `+d1` emits one word per line with no frequencies or
/// other info, meant as input to `kwal +s@FILE`. Routes to
/// `--word-list-only`. The bare `+d` and the broader `+dN`
/// display-mode rewrites are separate items.
#[test]
fn freq_word_list_only() {
    let input = args("clan freq +d1 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --word-list-only file.cha"));
}

/// FREQ's `+d4` outputs only the per-speaker type/token/TTR
/// summary, dropping all per-word frequency entries. Routes to
/// `--types-tokens-only`. Distinct from `+d3` (same content,
/// but spreadsheet form via `+f`/CSV).
#[test]
fn freq_types_tokens_only() {
    let input = args("clan freq +d4 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --types-tokens-only file.cha"));
}

/// FREQ's `+d3` (CLAN onlydata=4) is the type/token/TTR-only aggregate
/// SpreadsheetML FILE, not a stdout CSV. It maps to `--spreadsheet summary`.
/// CLAN manual: "Essentially the same as that for `+d2`, but with only the
/// statistics on types, tokens, and the type-token ratio." Un-squatted from
/// the prior `--types-tokens-only --format csv` stdout mapping.
#[test]
fn freq_d3_maps_to_summary_spreadsheet() {
    let input = args("clan freq +d3 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --spreadsheet summary file.cha"));
}

/// FREQ's `+d2` (CLAN onlydata=3) is the per-word aggregate SpreadsheetML
/// FILE. It maps to `--spreadsheet per-word`. `--format csv` stays a
/// chatter-only stdout convenience, never a `+d2` target (faithfulness rule).
#[test]
fn freq_d2_maps_to_per_word_spreadsheet() {
    let input = args("clan freq +d2 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --spreadsheet per-word file.cha"));
}

/// MLU's `+o3` (combine speakers, mlu.cpp:721) maps to `--combine-speakers`,
/// the same shared flag FREQ's `+o3` uses. MLU's `+pS`/`+r1`/`+r5`/`+r7` are
/// faithful no-ops (consumed, the morpheme/word count is form-independent).
#[test]
fn mlu_o3_and_word_form_noop_flags_rewrite() {
    assert_eq!(
        rewrite_clan_args(&args("clan mlu +o3 file.cha")),
        args("clan mlu --combine-speakers file.cha")
    );
    // The word-form flags are dropped (no-op) under MLU.
    for flag in ["+p_", "+r1", "+r2", "+r3", "+r4", "+r5", "+r7"] {
        assert_eq!(
            rewrite_clan_args(&args(&format!("clan mlu {flag} file.cha"))),
            args("clan mlu file.cha"),
            "MLU {flag} must rewrite to a no-op"
        );
    }
    // `+r6` is NOT a no-op: it maps to --include-retracings even for MLU.
    assert_eq!(
        rewrite_clan_args(&args("clan mlu +r6 file.cha")),
        args("clan mlu --include-retracings file.cha")
    );
}

/// FREQ's `+pS` adds the characters of `S` to the word delimiters
/// (cutt.cpp:9798-9818). Maps to chatter's `--word-delimiters S`. The empty form
/// `+p` passes an empty value (the dispatch then errors with CLAN's message).
#[test]
fn freq_p_maps_to_word_delimiters() {
    let input = args("clan freq +p_ file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --word-delimiters _ file.cha"));

    // Multi-character S (e.g. `+p_-`) passes the whole string through.
    let input = args("clan freq +p_- file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --word-delimiters _- file.cha"));
}

/// FREQ's `+dCN` percent-of-speakers filter (CLAN onlydata=4, freq.cpp:841-878;
/// the manual's `C` is the comparator metavariable) maps to
/// `--speaker-percentage <spec>`. Every comparator spelling routes through,
/// including CLAN's `=<`/`=>` aliases for `<=`/`>=`. The digit-led `+d2`/`+d20`
/// arms above are unaffected (matched here only by a leading comparator).
#[test]
fn freq_d_percent_maps_to_speaker_percentage() {
    for (flag, spec) in [
        ("+d<50", "<50"),
        ("+d<=50", "<=50"),
        ("+d=<50", "=<50"),
        ("+d=100", "=100"),
        ("+d>=33", ">=33"),
        ("+d=>33", "=>33"),
        ("+d>0", ">0"),
    ] {
        let input = args(&format!("clan freq {flag} file.cha"));
        let result = rewrite_clan_args(&input);
        assert_eq!(
            result,
            args(&format!("clan freq --speaker-percentage {spec} file.cha")),
            "rewriting {flag}"
        );
    }
}

/// KWAL's bare `+d` switches output from CLAN's location-
/// annotated default to a legal CHAT fragment (just the
/// matching `*Speaker:` lines, no `---` separator, no `*** File
/// ... Keyword: X` line). Routes to `--legal-chat`.
#[test]
fn kwal_legal_chat_format() {
    let input = args("clan kwal +d file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan kwal --legal-chat file.cha"));
}

/// `+c` under non-FREQ subcommands keeps its existing meaning
/// (MAXWD: `--limit N`; CHECK: `--bullets N`; IPSYN/DSS:
/// `--max-utterances N`). Regression-pin for MAXWD so adding
/// the FREQ arm doesn't accidentally swallow `+c50`.
#[test]
fn maxwd_plus_c_still_maps_to_limit() {
    let input = args("clan maxwd +c50 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan maxwd --limit 50 file.cha"));
}

/// VOCD's `+c` has the same semantic as FREQ's: count only words
/// starting with an uppercase letter.
#[test]
fn vocd_capitalized_only_bare() {
    let input = args("clan vocd +c file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan vocd --capitalization initial file.cha"));
}

/// VOCD's `+c0` is the documented alias for `+c`.
#[test]
fn vocd_capitalized_only_zero_suffix() {
    let input = args("clan vocd +c0 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan vocd --capitalization initial file.cha"));
}

/// VOCD's `+c1` (mid-uppercase), sibling of FREQ `+c1`.
#[test]
fn vocd_capitalized_mid_uppercase() {
    let input = args("clan vocd +c1 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan vocd --capitalization mid file.cha"));
}

/// COMBO's `+g3` (first-match-per-utterance) routes to the
/// boolean `--first-match-only` flag on the Combo subcommand.
#[test]
fn combo_g3_routes_to_first_match_only() {
    let input = args("clan combo -S want +g3 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan combo -S want --first-match-only file.cha")
    );
}

/// COMBO's `+g5` is a no-op for chatter, `+` is already the
/// default AND operator. Rewriter consumes the flag silently;
/// downstream clap never sees a stale `+g5`.
#[test]
fn combo_g5_is_silently_consumed_as_noop() {
    let input = args("clan combo -S want +g5 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan combo -S want file.cha"));
}

/// COMBO's `+g4` is "Exclude utterance delimiters from the
/// search", chatter's COMBO already operates on
/// `countable_words`, which never returns terminators or
/// separators. So `+g4` is the chatter default; the rewriter
/// consumes the flag and clap never sees it. Same shape as
/// the `+g5` no-op accept.
#[test]
fn combo_g4_is_silently_consumed_as_noop() {
    let input = args("clan combo -S want +g4 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan combo -S want file.cha"));
}

/// COMBO's `+g7` (no-duplicate-matches) routes to the boolean
/// `--dedupe-matches` flag on the Combo subcommand.
#[test]
fn combo_g7_routes_to_dedupe_matches() {
    let input = args("clan combo -S want +g7 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan combo -S want --dedupe-matches file.cha"));
}

/// DIST's `+g` is a per-turn-deduplicate counting policy
/// (CLAN: "count only one occurrence of each word per turn"),
/// distinct from the inherited gem-segment filter. Routes to
/// `--once-per-turn` on the Dist subcommand; gem-label filters
/// still go through `+gLABEL`.
#[test]
fn dist_g_bare_routes_to_once_per_turn() {
    let input = args("clan dist +g file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan dist --once-per-turn file.cha"));
}

/// `+gLABEL` (gem filter) on DIST is unchanged by the new arm.
#[test]
fn dist_g_with_label_still_routes_to_gem() {
    let input = args("clan dist +gStory file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan dist --gem Story file.cha"));
}

/// COMBO's gem-segment filter `+gLABEL` is unaffected by the
/// new `+g3` / `+g5` arms.
#[test]
fn combo_g_with_label_still_routes_to_gem() {
    let input = args("clan combo -S want +gStory file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan combo -S want --gem Story file.cha"));
}

/// MAXWD's `+cN` selects the number of longest items to display
/// (CLAN's `+c50` ↔ chatter's `--limit 50`). Without this branch,
/// `+cN` falls through to the CHECK-style `--bullets N` rewrite,
/// which `Maxwd`'s clap struct does not accept.
#[test]
fn maxwd_limit() {
    let input = args("clan maxwd +c50 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan maxwd --limit 50 file.cha"));
}

/// MAXWD's `+a` restricts output to words whose length is
/// unique within a speaker's lexicon (CLAN: "Consider ONLY
/// unique-length words"). Routes to `--unique-length-only`.
#[test]
fn maxwd_unique_length_only() {
    let input = args("clan maxwd +a file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan maxwd --unique-length-only file.cha"));
}

/// MLU's `-t%mor` is CLAN's documented escape hatch when the
/// `%mor` tier is present but should be ignored, implies
/// `--words` semantics. Without this special-case, the rewriter
/// routes `-t%X` to the generic `--exclude-tier X` which MLU's
/// clap doesn't accept.
#[test]
fn mlu_exclude_mor_tier_maps_to_words() {
    let input = args("clan mlu -t%mor file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan mlu --words file.cha"));
}

/// Same escape hatch applies to MLT (clause-level mean length,
/// shares MLU's %mor-vs-main-tier choice).
#[test]
fn mlt_exclude_mor_tier_maps_to_words() {
    let input = args("clan mlt -t%mor file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan mlt --words file.cha"));
}

/// `-t%X` for a non-%mor tier still routes to the generic
/// `--exclude-tier` path even under MLU. The special-case is
/// scoped to `-t%mor` specifically.
#[test]
fn mlu_exclude_non_mor_tier_falls_through() {
    let input = args("clan mlu -t%pho file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan mlu --exclude-tier pho file.cha"));
}

/// KWAL's `+b` is the strict-match mode: an utterance matches
/// the keyword only when the keyword is the *only* item on
/// the tier. Routes to the boolean `--strict-match` flag.
#[test]
fn kwal_strict_match() {
    let input = args("clan kwal -s want +b file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan kwal -s want --strict-match file.cha"));
}

/// WDSIZE's `+w>N` filters the histogram to words with length
/// strictly greater than N. Distinct from the general `+wN`
/// context-window rewrite because the first character of rest
/// is a comparator (`>`, `<`, or `=`).
#[test]
fn wdsize_length_filter_greater_than() {
    let input = args("clan wdsize +w>4 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan wdsize --length-filter gt:4 file.cha"));
}

/// `+w<N` → strictly less than.
#[test]
fn wdsize_length_filter_less_than() {
    let input = args("clan wdsize +w<5 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan wdsize --length-filter lt:5 file.cha"));
}

/// `+w=N` → equal to.
#[test]
fn wdsize_length_filter_equal() {
    let input = args("clan wdsize +w=3 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan wdsize --length-filter eq:3 file.cha"));
}

/// MAXWD's `+xN` excludes words of length N. Repeatable
/// (`+x5 +x6` excludes both). Routes to argv-pair
/// `--exclude-length N`.
#[test]
fn maxwd_exclude_length_single() {
    let input = args("clan maxwd +x5 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan maxwd --exclude-length 5 file.cha"));
}

/// Repeated `+xN` flags produce repeated `--exclude-length N`
/// pairs in argv order.
#[test]
fn maxwd_exclude_length_multiple() {
    let input = args("clan maxwd +x5 +x7 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan maxwd --exclude-length 5 --exclude-length 7 file.cha")
    );
}

/// CHECK retains the existing `+cN` ↔ `--bullets N` behaviour
///, proving the new MAXWD branch is gated on subcommand.
#[test]
fn check_bullets_unchanged_by_maxwd_branch() {
    let input = args("clan check +c3 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan check --bullets 3 file.cha"));
}

/// IPSYN's `+cN` selects the number of unique utterances to
/// analyse (CLAN default 100; chatter's `--max-utterances 100`).
/// Without per-subcommand routing this fell through to the
/// CHECK-style `--bullets N`, which `Ipsyn`'s clap struct does
/// not accept.
#[test]
fn ipsyn_max_utterances() {
    let input = args("clan ipsyn +c50 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan ipsyn --max-utterances 50 file.cha"));
}

/// DSS's `+cN` selects the number of unique utterances to score
/// (CLAN default 50). Same routing as IPSYN.
#[test]
fn dss_max_utterances() {
    let input = args("clan dss +c30 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan dss --max-utterances 30 file.cha"));
}

/// IPSYN's `+lF` specifies the rules-file path
/// (CLAN: language script). Maps to `--rules <PATH>`.
#[test]
fn ipsyn_rules() {
    let input = args("clan ipsyn +leng.ips file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan ipsyn --rules eng.ips file.cha"));
}

/// DSS's `+lF` specifies the rules-file path. Same routing.
#[test]
fn dss_rules() {
    let input = args("clan dss +lengu.scr file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan dss --rules engu.scr file.cha"));
}

/// MORTABLE's `+lF` specifies the language script file
/// (CLAN: words-group definition with `.cut` extension).
/// Maps to `--script <PATH>`.
#[test]
fn mortable_script() {
    let input = args("clan mortable +leng.cut file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan mortable --script eng.cut file.cha"));
}

/// SCRIPT's `+sF` is the template-file argument (an exception
/// to the general `+sS` ↔ `--include-word S` rule, since
/// SCRIPT's `+s` value is a filesystem path).
#[test]
fn script_template() {
    let input = args("clan script +stemplate.cha file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan script --template template.cha file.cha"));
}

/// UNIQ's `-o` flag is the sort-by-descending-frequency switch.
/// Routes to `--sort`. UNIQ is the only command with a
/// meaningful `-o`.
#[test]
fn uniq_sort() {
    let input = args("clan uniq -o file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan uniq --sort file.cha"));
}

/// MLU's `+gS` is CLAN's command-specific solo-word elision
/// flag (drop utterances consisting solely of word S). The
/// general `+gS` ↔ `--gem S` semantic, gem-segment filter,
/// would silently produce wrong output for researchers
/// pasting `mlu +gum file.cha`; the MLU/MLT branch routes
/// here instead.
#[test]
fn mlu_solo_word() {
    let input = args("clan mlu +gum file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan mlu --exclude-solo-word um file.cha"));
}

/// MLT shares MLU's `+gS` semantic.
#[test]
fn mlt_solo_word() {
    let input = args("clan mlt +gum file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan mlt --exclude-solo-word um file.cha"));
}

/// MLU `+g@F` loads the solo-word exclusion list from a file,
/// same idiom as `+s@F` → `--include-word-file`. Must precede
/// the per-word `+gS` arm so the `@`-prefix is intercepted
/// before being treated as a literal solo-word pattern.
#[test]
fn mlu_solo_word_from_file() {
    let input = args("clan mlu +g@list.txt file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan mlu --exclude-solo-word-file list.txt file.cha")
    );
}

/// MLT shares MLU's `+g@F` semantic.
#[test]
fn mlt_solo_word_from_file() {
    let input = args("clan mlt +g@list.txt file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan mlt --exclude-solo-word-file list.txt file.cha")
    );
}

/// FREQ `+gS` routes to the reject sentinel (CLAN FREQ has no gem flag), NOT
/// the MLU/MLT `--exclude-solo-word` branch. Proves the MLU/MLT and reject
/// branches are both gated on subcommand.
#[test]
fn freq_gem_routes_to_reject_not_mlu_branch() {
    let input = args("clan freq +gstory file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --reject-clan-gem story file.cha"));
}

/// SUGAR's `+aN` sets the minimum-utterance threshold
/// (CLAN default 50). Routes to `--min-utterances N`.
#[test]
fn sugar_min_utterances() {
    let input = args("clan sugar +a30 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan sugar --min-utterances 30 file.cha"));
}

/// KEYMAP's `+bS` sets a key-code to track. Routes to
/// `--keyword S` (repeatable).
#[test]
fn keymap_keyword() {
    let input = args("clan keymap +b$CW file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan keymap --keyword $CW file.cha"));
}

/// KEYMAP's `+b@F` file-list form is documented as not-yet-
/// rewritten, passes through unchanged. The leading `@`
/// distinguishes it from the inline-value form.
#[test]
fn keymap_keyword_file_passes_through() {
    // `+b@codes.cut` unrewritten, clap rejects at parse time
    // (better than silently misinterpreting as an inline keyword
    // literally named "@codes.cut").
    assert_passthrough("clan keymap +b@codes.cut file.cha");
}

/// MAKEMOD's `+a` is the all-alternatives boolean.
#[test]
fn makemod_all_alternatives() {
    let input = args("clan makemod +a file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan makemod --all-alternatives file.cha"));
}

/// LINES's `+n` is the remove-line-numbers boolean.
#[test]
fn lines_remove() {
    let input = args("clan lines +n file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan lines --remove file.cha"));
}

/// ORT's `+cF` is the homons-table dictionary path.
#[test]
fn ort_dictionary() {
    let input = args("clan ort +ceng.cut file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan ort --dictionary eng.cut file.cha"));
}

/// COMBO's `+sS` and `-sS` are compound boolean expressions,
/// not per-word patterns. Route to `--search` / `--exclude-search`.
#[test]
fn combo_search_routes_to_search_not_include_word() {
    let input = args("clan combo +swant+cookie file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan combo --search want+cookie file.cha"));
}

#[test]
fn combo_exclude_search() {
    let input = args("clan combo +swant -scookie file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan combo --search want --exclude-search cookie file.cha")
    );
}

#[test]
fn include_word_file_from_at_sigil() {
    let input = args("clan freq +s@nouns.cut file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan freq --include-word-file nouns.cut file.cha")
    );
}

#[test]
fn exclude_word_file_from_at_sigil() {
    let input = args("clan freq -s@stopwords.cut file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan freq --exclude-word-file stopwords.cut file.cha")
    );
}

#[test]
fn include_word_file_for_kwal() {
    let input = args("clan kwal +s@queries.cut file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan kwal --include-word-file queries.cut file.cha")
    );
}

/// COMBO's `+s@FILE` loads search expressions from disk,
/// one boolean expression per line, parsed downstream by
/// `SearchExpr::parse`. Separate from the per-word
/// `--include-word-file` because COMBO's `+s` value is a
/// boolean expression, not a per-word pattern.
#[test]
fn combo_search_at_sigil_routes_to_search_file() {
    let input = args("clan combo +s@queries.cut file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan combo --search-file queries.cut file.cha")
    );
}

/// COMBO's `-s@FILE` loads exclude search expressions from
/// disk, same file format, opposite polarity.
#[test]
fn combo_exclude_search_at_sigil_routes_to_exclude_search_file() {
    let input = args("clan combo -s@stopwords.cut file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan combo --exclude-search-file stopwords.cut file.cha")
    );
}

/// SCRIPT's `+s` carries a template-file path. `@`-prefixed
/// values stay routed to `--template`, not to the generic
/// word-list-from-file path.
#[test]
fn script_template_at_sigil_routes_to_template() {
    let input = args("clan script +s@list.cha file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan script --template @list.cha file.cha"));
}

/// FIXBULLETS' `+oN` adds N ms to all bullet timings. The
/// rewriter emits `--offset=N` (`=` syntax) as a single token,
/// symmetric with the negative-form rewrite which requires `=`
/// to keep clap from interpreting `-N` as a short-flag attempt.
#[test]
fn fixbullets_offset_positive() {
    let input = args("clan fixbullets +o800 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan fixbullets --offset=800 file.cha"));
}

/// FIXBULLETS' `-oN` subtracts N ms. The rewriter emits
/// `--offset=-N` (`=` syntax) rather than two tokens
/// `["--offset", "-N"]`; the `=` form is mandatory because clap
/// parses a free-standing `-N` as a short-flag attempt and
/// rejects it before reading it as `--offset`'s value.
/// Subprocess-level regression guard:
/// `legacy_fixbullets_negative_offset_runs_via_subprocess`.
#[test]
fn fixbullets_offset_negative() {
    let input = args("clan fixbullets -o800 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan fixbullets --offset=-800 file.cha"));
}

/// `+oS` with a non-numeric value should NOT rewrite under
/// FIXBULLETS (the numeric-only guard distinguishes the
/// time-offset use from the general "extra tier code"
/// semantic). The arg passes through unchanged.
#[test]
fn fixbullets_o_with_non_numeric_passes_through() {
    assert_passthrough("clan fixbullets +omor file.cha");
}

/// CLAN's `+t#ROLE` selects speakers by their `@ID:` role field.
/// Routes to `--role ROLE`; the role string is passed verbatim
/// (case-insensitive match happens at filter time).
#[test]
fn role_filter_include() {
    let input = args("clan freq +t#Target_Child file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --role Target_Child file.cha"));
}

/// CLAN does not support `-t#ROLE` exclude-by-role (per
/// `mainusage()` the `#ROLE` form is include-only). The `-t#…`
/// shape produces no rewrite, the arg passes through unchanged
/// to clap, which rejects it with a parse error. This is the
/// preferred failure mode: a loud parse error beats a silently-
/// wrong include semantic.
#[test]
fn role_exclude_polarity_not_rewritten() {
    // Arg passes through verbatim, no rewrite.
    assert_passthrough("clan freq -t#Target_Child file.cha");
}

/// Outside SCRIPT, `+s` keeps its general meaning (include-word
/// search keyword). Proves the SCRIPT branch is gated.
#[test]
fn freq_search_word_unchanged_by_script_branch() {
    let input = args("clan freq +scat file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan freq --include-word cat file.cha"));
}

#[test]
fn output_extension() {
    let input = args("clan analyze freq +fcex file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze freq --output-ext cex file.cha"));
}

#[test]
fn context_after() {
    let input = args("clan analyze kwal +w3 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("clan analyze kwal --context-after 3 file.cha"));
}

#[test]
fn context_before() {
    let input = args("clan analyze kwal -w2 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze kwal --context-before 2 file.cha")
    );
}

#[test]
fn id_filter() {
    let input: Vec<String> = vec![
        "clan".into(),
        "analyze".into(),
        "freq".into(),
        "+t@ID=\"eng|*|CHI|*\"".into(),
        "file.cha".into(),
    ];
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        vec![
            "clan".to_string(),
            "analyze".to_string(),
            "freq".to_string(),
            "--id-filter".to_string(),
            "eng|*|CHI|*".to_string(),
            "file.cha".to_string(),
        ]
    );
}

#[test]
fn mixed_clan_and_modern_flags() {
    let input = args("clan analyze freq +t*CHI --format json file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --speaker CHI --format json file.cha")
    );
}

#[test]
fn combined_flags() {
    let input = args("clan analyze freq +t*CHI +swant +z1-50 +r6 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args(
            "clan analyze freq --speaker CHI --include-word want --range 1-50 --include-retracings file.cha"
        )
    );
}

#[test]
fn unknown_flag_passes_through() {
    // A genuinely-unknown flag (no rewriter arm) is left verbatim. `+x` is no
    // longer a valid example: it now rewrites to the FREQ utterance-length
    // filter (`+x>3w` -> `--utterance-length >3w`), so use an unused letter.
    assert_passthrough("clan analyze freq +q123 file.cha");
}

#[test]
fn modern_flags_pass_through() {
    let input = args("clan analyze freq --speaker CHI --per-file file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(
        result,
        args("clan analyze freq --speaker CHI --per-file file.cha")
    );
}

#[test]
fn empty_args() {
    let result = rewrite_clan_args(&[]);
    assert!(result.is_empty());
}

#[test]
fn bare_plus_minus_pass_through() {
    assert_passthrough("+ -");
}

#[test]
fn r_unimplemented_passes_through() {
    // +r4 (prosodic-symbol mode) is not yet implemented, so it passes through
    // to clap unchanged. The implemented +r forms are the parenthesis modes
    // +r1/+r2/+r3 and retracings +r6.
    assert_passthrough("clan analyze freq +r4 file.cha");
}

#[test]
fn r_parenthesis_modes_rewrite_for_freq() {
    // +r1/+r2/+r3 (Parans, cutt.cpp:9530-9583) -> --parenthesis-mode, FREQ-scoped.
    assert_eq!(
        rewrite_clan_args(&args("clan analyze freq +r1 file.cha")),
        args("clan analyze freq --parenthesis-mode remove-parens file.cha")
    );
    assert_eq!(
        rewrite_clan_args(&args("clan analyze freq +r2 file.cha")),
        args("clan analyze freq --parenthesis-mode keep-parens file.cha")
    );
    assert_eq!(
        rewrite_clan_args(&args("clan analyze freq +r3 file.cha")),
        args("clan analyze freq --parenthesis-mode remove-material file.cha")
    );
}

#[test]
fn r5_replacement_mode_rewrites_but_r50_passes_through() {
    // +r5 (R5, cutt.cpp:9549) -> --replacement-mode original, FREQ-scoped.
    assert_eq!(
        rewrite_clan_args(&args("clan analyze freq +r5 file.cha")),
        args("clan analyze freq --replacement-mode original file.cha")
    );
    // +r50 (the distinct R5_1 variant) is NOT +r5; it stays unimplemented and
    // passes through (rest == "50" does not match the `rest == "5"` arm).
    assert_passthrough("clan analyze freq +r50 file.cha");
}

#[test]
fn display_mode_non_numeric_passes_through() {
    // +dabc is not a valid display mode
    assert_passthrough("clan analyze freq +dabc file.cha");
}

// CHECK-specific flag tests

#[test]
fn check_bullets() {
    let input = args("check +c0 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --bullets 0 file.cha"));
}

#[test]
fn check_list_errors() {
    let input = args("check +e file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --list-errors file.cha"));
}

#[test]
fn check_include_error() {
    let input = args("check +e6 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --error 6 file.cha"));
}

#[test]
fn check_exclude_error() {
    let input = args("check -e6 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --exclude-error 6 file.cha"));
}

#[test]
fn check_g2_target_child() {
    let input = args("check +g2 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --check-target file.cha"));
}

#[test]
fn check_g5_unused_speakers() {
    let input = args("check +g5 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --check-unused file.cha"));
}

#[test]
fn check_g4_check_id() {
    let input = args("check +g4 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --check-id true file.cha"));
}

#[test]
fn check_g1_noop() {
    let input = args("check +g1 file.cha");
    let result = rewrite_clan_args(&input);
    // +g1 is a no-op (prosodic delimiters always recognized)
    assert_eq!(result, args("check file.cha"));
}

#[test]
fn check_u_maps_to_check_ud() {
    let input = args("check +u file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --check-ud file.cha"));
}

#[test]
fn non_check_u_is_noop() {
    let input = args("freq +u file.cha");
    let result = rewrite_clan_args(&input);
    // +u is a no-op (merge is default) for non-CHECK commands
    assert_eq!(result, args("freq file.cha"));
}

#[test]
fn freq_gem_digit_also_rejected() {
    // For FREQ, even a digit-rest `+gN` rejects (CLAN FREQ has no gem flag);
    // it is not a mode selector. The MLU/MLT/Combo/Maxwd `+gN` overrides are
    // gated on their own subcommands above.
    let input = args("freq +g2 file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("freq --reject-clan-gem 2 file.cha"));
}

#[test]
fn check_g_with_label_falls_back_to_gem() {
    // +g with a non-digit label (even in check context) falls back to gem
    let input = args("check +gstory file.cha");
    let result = rewrite_clan_args(&input);
    assert_eq!(result, args("check --gem story file.cha"));
}
