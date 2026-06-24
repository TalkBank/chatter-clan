# FREQ -- Word Frequency

**Status:** Current
**Last updated:** 2026-06-04 21:40 EDT

## Purpose

Counts word tokens and types and computes type-token ratio (TTR). The legacy manual describes `FREQ` as one of CLAN's most powerful and easiest-to-use programs, producing word-frequency counts and lexical-diversity measures over selected files and speakers.

In `talkbank-clan`, `FREQ` counts words on the main tier by default, or morphemes from the `%mor` tier when `--mor` is set.

See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409093) for the original FREQ command specification.

## Usage

```bash
chatter clan freq file.cha
chatter clan freq --speaker CHI file.cha
chatter clan freq --format json corpus/
chatter clan freq --mor file.cha
chatter clan freq --include-word "the" file.cha
```

> **`+k` / `--case-sensitive` is wired as of 2026-05-22 (pattern
> matching) + 2026-05-23 (frequency-table keying).** Without the
> flag, word matching is case-insensitive (CLAN's default and
> chatter's default). With `+k` (or `--case-sensitive`):
> - `+s`/`--include-word` patterns and the searched words skip
>   lower-casing, so an exact-case match is required;
> - the frequency-table key preserves original case, so `Want`,
>   `want`, and `WANT` produce three separate entries (each with
>   count 1) instead of collapsing to one.

## Options (chatter-native)

| Option | CLAN Flag | Description |
|--------|-----------|-------------|
| `--speaker <CODE>` | `+t*CHI` | Include speaker |
| `--exclude-speaker <CODE>` | `-t*CHI` | Exclude speaker |
| `--include-word <WORD>` | `+s"word"` | Only count matching word |
| `--exclude-word <WORD>` | `-s"word"` | Skip matching word |
| `--gem <LABEL>` | `+g"label"` | Restrict to gem segment |
| `--range <START-END>` | `+z25-125` | Utterance range |
| `--id-filter <PATTERN>` | `+t@ID="..."` | Filter by @ID pattern |
| `--include-retracings` | `+r6` | Include retraced words in counting (a retracing retraces the single preceding word; default drops it). Byte-parity golden `freq_r6_retrace`. |
| `--case-sensitive` | `+k` | Match `+s` / `--include-word` patterns case-sensitively (default: case-insensitive) |
| `--format <FMT>` | -- | Output format: clan (default), text, json, csv |
| `--mor` | -- | Count morphemes from `%mor` tier instead of words from main tier |

## CLAN `+`-flag coverage audit

Authoritative enumeration of every CLAN `freq` flag, mapped against
chatter's coverage. Sources:

* `OSX-CLAN/src/clan/freq.cpp`: `usage()` at line 152 and the
  command-specific `getflag()` intercept at line 621.
* `OSX-CLAN/src/clan/cutt.cpp`: `mainusage()` at line 9090
  (program-keyed `FREQ` branches throughout).
* `crates/talkbank-clan/src/clan_args.rs`: chatter's `+flag` to
  `--flag` rewriter.
* `crates/talkbank-cli/src/cli/args/clan_common.rs` and
  `crates/talkbank-cli/src/cli/args/clan_commands.rs::Freq`,
  chatter's clap field surface for FREQ.

### Status legend

* **Done**: chatter accepts the flag and the semantic is implemented.
* **Partial**: chatter accepts a related abstraction with non-identical
  semantics; gap noted in the *Notes* column.
* **Rewriter only**: `clan_args.rs` rewrites the `+flag` to a chatter
  flag, but no clap field on `Freq` consumes that token; passing the
  flag errors out at parse time.
* **Missing**: neither rewriter nor clap field handles it.

### Freq-specific `+`-flags (from `freq.cpp::getflag`)

| CLAN flag | Meaning | Chatter | Status | Notes |
|---|---|---|---|---|
| `+a` | (FREQ `typeForms`) |, | Deferred (design + manual-clarity, MOR-interacting) | FREQ's `+a` sets `typeForms = TRUE` (freq.cpp:768, a no-arg toggle) and is then woven through ~10 output sites (freq.cpp:439, 1064, 1077, 1179, 1202, 1209, 1507, 1530, 1794, 1852), gating on `nonSpTierSpecified` (dependent-tier) and the `onlydata`/`%mor` output paths, so it is not a localized main-tier flag. The CLAN manual has no FREQ-specific `+a` entry (its `+a` sections at CLAN.txt:6532 and 9454 document OTHER commands, MAXWD "unique-length only" and a `%wor` pause flag), so the INTENT is unclear from the manual, and the behaviour is `%mor`/dependent-tier-coupled. Per the bug-localization rule this is a manual-clarity + design + MOR question, not a solo implementation; deferred to the FREQ porting report. chatter rejects `+a` at the fail-closed boundary (correct interim). |
| `+bN` | Frame size for MATTR (Moving-Average TTR) | `--mattr N` | Done | Landed 2026-06-02. Per-speaker Moving-Average TTR: the mean over every length-`N` sliding window of that window's TTR (`distinct_types / N`), across all `T - N + 1` windows (`comute_MATTR`, freq.cpp:1742-1781; manual CLAN.txt:5473-5479). `N` must be a positive integer (`FrameSize` newtype rejects `+b0`, matching freq.cpp:777-780). Rendered per speaker after the TTR caveat as `%5.3f  MATTR`, or `-      MATTR` when `T < N` (`NMATTRs == 0`, freq.cpp:1542-1547). The windowed average is the shared `framework::moving_average_ttr` primitive (reusable by VOCD and future diversity measures). Golden-proven by `freq_b_mattr_eng` (CLAN `+b3` on the 15-token `*SPE` = `1.000  MATTR`); the `T<N` `-` path and `+b0` rejection are pinned by the `freq_mattr_tests` subprocess tests. With `+d2`/`+d3` a trailing `MATTR` column is appended to the spreadsheet (header + per-row `%.3f` Number cell, `-` when undefined; freq.cpp:3303, 1558-1562), pinned by `freq_d2_with_mattr_appends_mattr_column` and verified byte-for-byte against CLAN's `+b3 +d2`. |
| `+c` / `+c0` | Find capitalised words only | `--capitalization initial` | Done | Landed 2026-05-22. Skips any countable word whose first character is not uppercase before frequency accumulation. Both `+c` and `+c0` are accepted as aliases (CLAN treats them identically); subcommand-guarded so MAXWD/CHECK/IPSYN/DSS keep their existing `+cN` meaning. Shares the `CapitalizationFilter` enum with VOCD. |
| `+c1` | Find words with upper-case letters in the middle | `--capitalization mid` | Done (diverges, CLAN-DIV-003) | Landed 2026-05-22. Keeps only words whose surface form contains an uppercase letter AFTER position 0 (e.g. `McDonald`, `iPhone`). Initial-capital words like `Cookie` are dropped. Shares the `CapitalizationFilter::MidUpper` predicate with VOCD. **Deliberate CLAN-bug divergence (CLAN-DIV-003):** CLAN's `+c1` (`freq.cpp` `isRightUpper`, `capwd == 2`) loops from position 0 and keeps initial-caps too, identical to `+c`, contradicting the manual's "middle only"; chatter implements the documented semantic. Pinned by the `freq_c1_mid_capitalization_eng` `DivergesFromClan` golden. |
| `+c2` | Count a word once per matching `+s` pattern (not just once) | `--search-multiplicity per-pattern` | Done | Landed 2026-06-02. **Correction:** this is a SINGLE-word `+s` multiplicity flag, NOT a multi-word search variant (the prior note was wrong). `capwd == 3` (freq.cpp:432-438): a word that matches several `+s` patterns is counted once per matching pattern, where the default counts it once regardless (manual CLAN.txt:5485-5486). CLAN requires the `+s` patterns to carry wildcards (`isFoundWildCard`) and rejects `+c2` combined with a multi-word `+s` group (freq.cpp:455-459); chatter reproduces both errors at dispatch. Implemented via `WordFilter::count_matching_includes` + the `IncludeMultiplicity` enum. Golden-proven by `freq_c2_perpattern_eng` (CLAN `+c2 +s"t*" +s"*p"` on `*SPE`: "top" matches both patterns -> counted 2); the default-once contrast and both guard errors are pinned by `freq_c2_multiplicity_tests`. |
| `+c3` | Match multi-word groups in any order on a tier | `--multiword-order any` | Done | Landed 2026-06-02 (Phase 2 of the multi-word cluster). Relaxes multi-word `+s` matching from the default adjacent in-order sequence to "anywhere and in any order" (`anyMultiOrder`, freq.cpp:792, 2389-2464; manual CLAN.txt:5488): each token fills the first unfilled slot it matches; the group counts once when every slot is filled, then resets. The item is still displayed as the search pattern, not the matched data order. Implemented as the `MatchOrder::AnyOrder` mode on the shared `framework::multiword` matcher. Golden-proven by `freq_c3_anyorder_eng` (CLAN `+c3 +s"going Triangle"` on `*SPE` = 1 `going Triangle`, reversed and non-adjacent); contrast with the without-`+c3` 0-match pinned by `freq_c3_anyorder_tests`. |
| `+c4` | Match only if tier is *solely* the multi-word group | `--multiword-scope sole` | Done | Landed 2026-06-02 (Phase 3 of the multi-word cluster). A multi-word `+s` match counts only when the utterance consists solely of the group: its word count must equal the group's slot count (`onlySpecWsFound`, freq.cpp:794, 2381-2388; manual CLAN.txt:5490-5491). Implemented as `MatchScope::SoleContent` on the shared `framework::multiword` matcher; composes with `+c3` (`+c3 +c4` = any-order sole-content). Golden-proven by `freq_c4_sole_eng` (CLAN `+c4 +s"Triangle kept going"` on the 3-word `*SPE` utterance = 1); subset/contrast pinned by `freq_c4_sole_content_tests`. **Deferred edge:** on a *non-matching* `+c4` search, CLAN suppresses the per-speaker banner entirely when no utterance has the group's word length (the `cntItems`-gated section print, freq.cpp), whereas chatter renders the speaker with `0` totals; the positive match is byte-identical, only this no-match presentation differs. |
| `+c5` | With `+d7`, reverse source/target tier priority |, | Deferred (gated on `+d7`) | CLAN itself refuses `+c5` without `+d7`: `freq.cpp:561-563` errors `"The +c5 option can only be used with +d7 option"` (`isMorTierFirst`). `+d7` is the PI-gated source/target linked-tier (`%mor` cross-tier) row, so `+c5` cannot be implemented before it; both go in the FREQ porting report. chatter rejects `+c5` at the fail-closed boundary (correct interim). |
| `+c6` | Count only repeat segments |, | Deferred (CA repeat-segment, needs parser support) | CLAN's `isRepeatSegment` (freq.cpp:153, 798) filters the main tier to the CA repeat-segment spans delimited by `↫` (U+21AB), via `separateRepeatSegments`/`filterRepeatSegs` (freq.cpp:1857-1992), and is meant to combine with a `+s` search ("count only repeat segments when searching"). chatter's typed AST treats `↫` as a satellite delimiter (not word content; see CLAN-DIV-006 in the `+r7` row), so it has no structural repeat-segment *span* to restrict to: implementing `+c6` needs a parser/model addition (a repeat-segment span type) plus the `+s`-search coupling, i.e. a CA-data/parser question, not a FREQ-command-only change. Deferred to the FREQ porting report; chatter rejects `+c6` at the fail-closed boundary (correct interim). Single fixture with `↫`: `corpus/reference/ca/nonvocal-and-long-features.cha`. |
| `+c7` | For multi-word searches output actual words matched | `--multiword-display matched` | Done | Landed 2026-06-03 (Phase 5, last implementable multi-word mode). For a multi-word `+s` group, display the words that actually matched rather than the search pattern (`isMultiWordsActual`, freq.cpp:800, 2444; manual CLAN.txt:5498-5500): `+s"the *"` collapses every match into one `the *` entry by default, while `+c7` reveals each (`the hill`, `the top`). The shared `framework::multiword` matcher now returns a `Match` carrying the token index per slot (reusable by KWAL context / COMBO), and FREQ keys each match by `Match::matched_words` under `+c7`. Golden-proven by `freq_c7_matched_eng`; the default-pattern contrast pinned by `freq_c7_matched_words_tests`. |
| `+o` / `+o0` | Sort by descending frequency | `--sort frequency` | Done | chatter's default per-word order is alphabetical (CLAN's default, its BST in-order traversal); `+o`/`+o0` map to `--sort frequency`, switching to descending count with an alphabetical tiebreak (`freq.cpp:176`; `freq.cpp:815-817`: `*f == EOS \|\| *f == '0'` sets `isSort`). Pinned by the `freq_o_descending_frequency_eng` byte-parity golden. The 2026-06-01 golden probe corrected the earlier mis-audit that called `+o` a no-op: chatter was actually rendering alphabetical while CLAN `+o` is frequency-descending, so the sort mode was implemented (the `FreqSort` enum replaced the `reverse_concordance` bool). |
| `+o1` | Sort by reverse concordance | `--sort reverse-concordance` | Done | Landed 2026-05-23. Replaces the default frequency-descending sort with a sort by the reversed character sequence of each word, groups words sharing a suffix. Pinned by `freq_reverse_concordance_groups_by_suffix` (with `cat`/`bat`/`dog`/`log` input, the sorted result reflects reversed-string comparison) and `freq_default_sort_is_alphabetical_when_freqs_equal` (regression companion). End-to-end smoke: `cat bat dog log apple maple` with `+o1` clusters maple/apple, dog/log, bat/cat. `+o2` (reverse concordance + non-CHAT output) is a separate Missing item. |
| `+o2` | Sort by reverse concordance of first word, preserve full line |, | Deferred | CLAN `+o2` sets `chatmode = 0` (freq.cpp:820-830), reinterpreting the CHAT file as **plain text** and counting whole input *lines* as tokens, `%mor:`/`%gra:`/`@ID:`/`@Begin` lines included (verified by live probe). This is antithetical to chatter's AST-first model: reproducing it requires bypassing the parser to count raw lines, which chatter deliberately does not do. Deferred with this source-grounded reason; revisit only if a plain-text input mode is ever added. |
| `+o3` | Combine all speakers into one list | `--combine-speakers` | Done | Landed 2026-06-03. CLAN `isCombineSpeakers` (freq.cpp:832): pool ALL speakers' counts into one frequency table with no per-speaker `Speaker:` header (freq.cpp:1451 prints the banner only when `!isCombineSpeakers`), summed counts, combined Types/Tokens/TTR. On eng-conversation (`*SPE` + `*GES`), `the` = 4 (2+2), Types=14, Tokens=28, TTR=0.500 (live CLAN probe). Golden-proven by `freq_o3_combine_eng`; the rewrite + header suppression + pooling are pinned by `freq_o3_combine_tests`. Completes the `+o` sort sub-cluster (`+o`/`+o0`/`+o1`/`+o3` Done, `+o2` Deferred). |
| `+d` | All selected words + freq + line numbers |, | Missing | Bare `+d` reaches `maingetflag` (freq.cpp:838 -> cutt.cpp:9402) and sets `onlydata = atoi("")+1 = 1`, i.e. CLAN source treats `+d` IDENTICALLY to `+d0`; the manual calls `+d` the no-flag default but `+d0` a concordance. That manual-vs-source discrepancy (and any `getfarg` next-arg consumption) must be reconciled before implementing. No rewriter arm consumes it, so it now ERRORS fail-closed at the file-discovery boundary (`DiscoveredChatFiles`), no longer silently swallowed as a bogus file. |
| `+d0` | Concordance with frequencies and line text |, | Missing | PI-gated (concordance, overlaps KWAL); `onlydata = 1` (freq.cpp:838 -> cutt.cpp:9402, same as bare `+d`). Now ERRORS fail-closed at the boundary; do not implement solo. |
| `+d1` | One word per line, no frequencies | `--word-list-only` | Done | Rewriter maps bare `+d1`. Emits an alphabetized deduped word list merged across all speakers, suitable as `kwal +s@FILE` input. |
| `+d2` | Spreadsheet output (Excel-ready) | `--spreadsheet per-word` | Done | Rewriter maps bare `+d2` to `--spreadsheet per-word` (un-squatted from `--format csv`). Writes an aggregate SpreadsheetML file (`stat.frq.xls`), one row per (file x speaker) keyed by `@ID`, with per-word columns + Types/Token/TTR. Golden-proven (`freq_d2_spreadsheet_manchester`) as a documented divergence: byte-identical to CLAN's cells except the `%%mor`->`%mor` caveat fix (CLAN-DIV-004). `+d20` (one-row-per-speaker+word variant) is a separate Rewriter-only item. |
| `+d20` | Spreadsheet with one row per speaker+word | `--spreadsheet per-speaker-word` | Done | Landed 2026-06-04. CLAN `isSpreadsheetOnePerRow` + `onlydata = 3` (freq.cpp:881-887): a flat `stat.frq.xls` with one row per (file, speaker, word), columns `File | Code | Word | Count`, NO `@ID` columns, NO Types/Token/TTR summary, NO `%mor` TTR caveat (so unlike `+d2`/`+d3` there is no `%%mor` leak to diverge on). Words byte-sorted within each (file, speaker) (`BTreeMap` order = CLAN's WordsHead `strcmp`). Rewriter maps `+d20` -> `--spreadsheet per-speaker-word` (`FreqSpreadsheetMode::PerSpeakerWord`). Byte-identical to CLAN's `+d20` data cells on the Manchester fixtures (`+t*CHI`): anne CHI = baby(1)/fit(1)/it(2), aran CHI = down/duck/get/there(1) (live CLAN diff). Golden-proven by `freq_d20_spreadsheet_manchester` (MatchesClan); the CLI file-write path is pinned by `freq_d20_writes_one_row_per_speaker_word_spreadsheet`. |
| `+d3` | Spreadsheet, types/tokens/TTR only | `--spreadsheet summary` | Done | Rewriter maps bare `+d3` to `--spreadsheet summary` (un-squatted from `--types-tokens-only --format csv`). Same SpreadsheetML file output as `+d2` (`stat.frq0.xls`) restricted to Types/Token/TTR (no per-word or speaker pseudo-word columns). Golden-proven (`freq_d3_spreadsheet_manchester`), CLAN-DIV-004 divergence. |
| `+d4` | Type/token info only | `--types-tokens-only` | Done | Rewriter maps bare `+d4`. Per-speaker banner + separator + totals + TTR note all kept; per-word frequency lines dropped. `+d3` (same content, CSV/spreadsheet form) is a separate item. |
| `+d5` | Output `+s` words including those with 0 freq | `--include-zero-frequency` | Done | Landed 2026-06-03. CLAN `zeroMatch` (freq.cpp:894): each LITERAL `+s` word is injected into every output speaker's table at count 0 when it never matched (freq.cpp:1473-1491 via `freq_tree_add_zeros`, freq.cpp:1259, which adds only when absent). The zero word is DISPLAYED but excluded from types/tokens/TTR: `+d5 +skept +szzz +t*SPE` on `*SPE` shows `1 kept` + `0 zzz` yet Types=1, Tokens=1, TTR=1.000 (live CLAN probe). Single- and multi-word `+s` both inject (`0 big dog`). CLAN rejects wildcards (`* % _`) or duplicate `+s` words (freq.cpp:444, `isFoundWildCard(TRUE)`) and requires at least one `+s` word (freq.cpp:449); both guards are enforced at the FREQ dispatch (`analysis.rs`), mirroring `+c2`. Golden-proven by `freq_d5_zerofreq_eng`; the error and CLI paths are pinned by `freq_d5_zerofreq_tests`. |
| `+d6` | Limited search-word surrounding context |, | Missing | PI-gated (`%mor` replaced/error/POS tabulation per manual); freq.cpp:900-901 sets `R5_1` when `mwdptr == NULL`. Now ERRORS fail-closed at the boundary; do not implement solo. |
| `+d7` | Frequencies linked between dependent tier and speaker |, | Missing | PI-gated (links a source tier to a target tier; overlaps KWAL/freqpos); freq.cpp:902-903 sets `linkDep2Other`. Now ERRORS fail-closed at the boundary; do not implement solo. |
| `+d8` | Cross-tabulation of one dependent tier with another |, | Missing | PI-gated (`%mor` cross-tab, overlaps mortable/freqpos); freq.cpp:895-896 sets `isCrossTabulation`. Now ERRORS fail-closed at the boundary; do not implement solo. |
| `+dCN` | Output words used by `<`, `<=`, `=`, `=>`, `>` than N percent of speakers (the spellings `+d<N`/`+d<=N`/`+d=N`/`+d>=N`/`+d>N`, plus the `=<`/`=>` aliases) | `--speaker-percentage <spec>` | Done | Landed 2026-06-04. CLAN `onlydata = 4`, the percent-of-speakers filter (freq.cpp:841-878 parse; `statfreq_percent_result` at freq.cpp:2841 emits). The manual's `C` (CLAN.html "+dCN") is the **comparator metavariable**, not a literal `C`: `percentC` 1..5 for `<`/`<=`(`=<`)/`=`/`>=`(`=>`)/`>` (freq.cpp:841-861); `percent = atoi(N)` (freq.cpp:876). **Output shape (live-probed, NOT the `+d2` per-word sheet the prior audit guessed):** the `+d3`-shaped SUMMARY (no per-word columns), but each speaker's Types/Token/TTR is recomputed over ONLY the kept words; written to `words.frq.xls` (a distinct filename from `+d2`'s `stat.frq.xls` / `+d3`'s `stat.frq0.xls`); and the fourth column header is `Speaker`, NOT `+d2`/`+d3`'s `Code` (freq.cpp:2874 vs the `+d2`/`+d3` path). A word is kept iff its DISTINCT-SPEAKER count (number of (file x speaker) rows that used it, CLAN `statfreq_AddWords` `p->count`, freq.cpp:2756-2762) compares against `percentNum = floor(num_rows * N / 100)` (`findPercentNum`, freq.cpp:2664-2678). Rewritten from `+d<=50` -> `--speaker-percentage <=50` (a chatter-only flag name carrying the CLAN slot; faithfulness rule); the value parser rejects a missing/non-digit N (CLAN "Please specify percentage value", freq.cpp:871-874); the `+d5`-combination and `--spreadsheet`-combination guards live at the FREQ dispatch (freq.cpp:867-870/890-893). Modeled as a typed `FreqSpreadsheetMode::PercentOfSpeakers(SpeakerPercentFilter { comparison, percent })` (`commands/freq/spreadsheet.rs`), so the exhaustive-match discipline forced every spreadsheet consumer (renderer, filename, service) to handle it. **`DivergesFromClan` (CLAN-DIV-004):** the percent path re-emits the SAME `%%mor` TTR caveat as `+d2`/`+d3` (the only difference); every data cell (the `@ID` columns, the `Speaker` header, the filtered Types/Token/TTR for both CHI and MOT) is byte-identical to CLAN's `words.frq.xls` (live diff over all five comparators on manchester-anne `+t*CHI +t*MOT`). Golden `freq_d_percent_lt_eq_manchester` (`+d<=50`: CHI 0/0/`-` since all its words are shared with MOT; MOT 5/5/1.000); CLI paths pinned by `freq_d_percent_lt_eq_writes_speaker_filtered_summary` / `freq_d_percent_gt_keeps_shared_words` / `freq_d_percent_rejects_d5_combination_and_missing_number`; rewriter by `freq_d_percent_maps_to_speaker_percentage`; logic by `percent_*` unit tests. |

### General `+`-flags FREQ inherits (from `cutt.cpp::mainusage`)

| CLAN flag | Meaning | Chatter | Status | Notes |
|---|---|---|---|---|
| `+t*X` | Include speaker `*X` | `--speaker X` | Done | |
| `-t*X` | Exclude speaker `*X` | `--exclude-speaker X` | Done | |
| `+t%X` | Include dependent tier `%X` | `--tier X` | Done | Counts the whitespace-delimited tokens of dependent tier `%X` (`freq.cpp:914-938` `case 't'`: `nomain=TRUE`, `fDepTierName`), modeled as `CountSource::DependentTierTokens`. A clitic `v|go~aux|be` is ONE token (raw line split, NOT chatter's structural `--mor`, which splits post-clitics). CLAN's default exclude list, seeded at FREQ init (`freq.cpp:405-409`), drops the bare `.` terminator, the `mor_initwords` markup categories (`beg|*`,`end|*`,`cm|*`,…, `cutt.cpp:8827-8833`), and the `gra_initwords` punctuation relations (`*|*|{BEGP,ENDP,LP,PUNCT}`, `cutt.cpp:8835-8840`, whose only caller is FREQ). The "%mor line forms" TTR advisory is gated on `!isMorUsed` (`freq.cpp:1536`): kept for non-`%mor` tiers (e.g. `+t%gra`), suppressed for `+t%mor`. chatter's separate `--mor` stays structural (a chatter-only flag, not this CLAN slot; faithfulness rule). Reuses the shared `framework::dependent_tier_tokens` primitive (KWAL/COMBO `+t%X` reuse it). Golden-proven by `freq_t_gra_eng` (advisory kept) + `freq_t_mor_eng` (advisory suppressed); CLI/rewriter/guard by `freq_t_tier_tests`. |
| `-t%X` | Exclude dependent tier `%X` | `--exclude-tier X` | Done | The EXCLUDE form flips FREQ to count the main tier PLUS every present dependent tier EXCEPT `%X` (`freq.cpp` `case 't'` tier-selection; manual "Limiting by including or excluding dependent tiers"; banner "ALL speaker tiers / and those speakers' ALL dependent tiers EXCEPT ..."), modeled as `CountSource::MainPlusDependentTiersExcept` and pooled into one table. A composition of the main-tier count and the `+t%X` per-tier count over all present tier kinds (each kind counted once); repeatable (`-t%X -t%Y`). The "%mor line forms" TTR advisory STAYS ON even when `%mor` is swept in: CLAN sets `isMorUsed` only for the explicit `+t%mor` include, not the `-t` sweep. Golden-proven by `freq_neg_t_gra_eng` (main + `%mor`) + `freq_neg_t_mor_eng` (main + `%gra`); CLI/guard by `freq_t_tier_tests`. Deferred as separate rows: the `+t`/`-t` *combination* state machine (an include and exclude together) and `-t*` (main-line exclusion); any `--mor`/`--tier`/`--exclude-tier` mix errors at dispatch. |
| `+t@ID="..."` | Filter by @ID pattern | `--id-filter` | Done | Whole-string, case-insensitive `*`-wildcard glob over the raw `@ID` line (CLAN `uS.patmat`), NOT a field-by-field match: `*|Target_Child|*` selects by role. Banner ("ONLY speaker main tiers with IDs matching: ...; *:;") now matches CLAN. Golden-proven by `freq_id_filter_target_child_eng`. |
| `+t#ROLE` | Filter by role | `--role` | Done | Fixed 2026-05-22. Rewriter routes `+t#ROLE` → `--role ROLE`; the per-utterance filter checks the speaker's `@ID:` role field case-insensitively; banner-scope renders `ONLY speaker main tiers with role(s): ROLE;`. |
| `+s"word"` / `+sword` | Search for word (single or multi-word group) | `--include-word` | Done | Single-word per-word count golden-proven by `freq_s_word_eng`. **Multi-word groups** (a `+s` value with more than one word, e.g. `+s"the hill"`) landed 2026-06-02 as Phase 1 of the multi-word search cluster: matched as an adjacent, in-order, non-overlapping sequence on the main tier (`freq.cpp:2465-2548`), counted once per occurrence under the search pattern. Built on the shared `framework::multiword::MultiWordGroup` primitive (whose `matches()` returns spans, so KWAL/COMBO reuse the same engine). Golden-proven by `freq_s_multiword_eng`; reversed/non-adjacent negatives pinned by `freq_multiword_tests`. The `+c2`..`+c7` multi-word *modes* (any-order, sole-content, multiplicity, literal-wildcard) are the remaining phases. |
| `-s"word"` / `-sword` | Exclude word | `--exclude-word` | Done | |
| `+s@F` / `-s@F` | Search / exclude words listed in file F | `--include-word-file` / `--exclude-word-file` | Done | Landed 2026-05-22. Universal across non-SCRIPT, non-COMBO commands. File format matches CLAN's `cutt.cpp::rdexclf`: one pattern per line; blank lines, `# `-comments, and `;%* `-annotation lines skipped; UTF-8 BOM stripped. Repeatable. Golden-proven by `freq_s_include_word_file_eng` / `freq_neg_s_exclude_word_file_eng` (byte-parity with CLAN's `+s@`/`-s@`, including identical #-comment / blank-line skipping). |
| `+gX` | Include gem labelled X | -- (errors) | Rewriter only | CLAN FREQ has NO `+g` gem flag: `freq +gX` returns "Invalid option" (no `case 'g'` in `freq.cpp`). Gem-limiting in CLAN is the separate GEM program (`gem +sX +d +f` -> `freq`). chatter errors `+gX`/`-gX` as inapplicable (talkbank-clan/CLAUDE.md), routing them to a reject sentinel. chatter's `--gem` / `--exclude-gem` remain a **chatter-only** convenience (limit FREQ to a gem directly), reachable via the `--` flag only, never via the CLAN `+gX` slot (faithfulness rule). (CLAN MLU/MLT DO accept `+g`, mapped to `--exclude-solo-word`.) |
| `-gX` | Exclude gem labelled X | -- (errors) | Rewriter only | See `+gX`: same inapplicable-flag rejection. |
| `+rN` (N=1..8, +r50) | Word-form treatment for counting: parentheses / prosodic symbols / `[: text]` replacement / retracings / `%mor`-combine (NOT output formatting) | `--parenthesis-mode` (`+r1`/`+r2`/`+r3`); `--include-retracings` (`+r6`); `--replacement-mode` (`+r5`); `--prosody-mode` (`+r7`) | Partial (parens + retracings + replacement + prosody done) | **`+r` is a word-normalization parity cluster, not a cheap flag tail.** Parsed at `cutt.cpp:9530-9583` (needs `R_OPTION`); bare `+r` = `+r1`; CLAN errors "Choose N … between 1-7" for out-of-range N. Per sub-flag (manual §14.5 @ line 12379; source `cutt.cpp`): **`+r1`/`+r2`/`+r3` (omitted-material parentheses, `Parans` 1/2/3) DONE (landed 2026-06-04), with a default-parity FIX:** CLAN's DEFAULT is `+r1` = remove parens, KEEP the omitted letters (`bein(g)` → `being`); `+r2` keeps the parens literally (`bein(g)`); `+r3` drops the omitted letters (`bein`). Live-probed on `1082.cha`. chatter's default previously diverged AND was internally inconsistent: the grouping key (`cleaned_text()`, `Text`+`Shortening`) was already `+r1`-correct (`being`), but the DISPLAY (`framework::clan_display_form` → `Word::raw_text()`) kept the parens (`bein(g)`, i.e. `+r2`). Now a typed `framework::ParenthesisMode` (`RemoveParens` default / `KeepParens` / `RemoveMaterial`) drives BOTH the key (`parans_normalized_key`) and the display (`parans_display`), rendered AST-first from `Word::content` (only words containing a `Shortening` take the new path, so zero blast radius elsewhere, and no freq golden changed). Rewritten from `+r1`/`+r2`/`+r3` via `--parenthesis-mode remove-parens|keep-parens|remove-material`. Pinned by `freq_r_paren_tests` (default + `+r1`/`+r2`/`+r3` on `1082.cha`) and the `normalized_word` renderer unit tests. **`+r4` (`R4` + `isPre/PostcliticUse`) is a NO-OP for FREQ counting (probe-confirmed 2026-06-04):** the manual frames it as making `#`/`/`/`:` significant in *searches* (and it sets the pre/postclitic-use flags), but on a `ca:t`/`do^g` probe `freq +r4` == `freq` default (both strip `:`/`^`, so `ca:t`==`cat`). For the base FREQ count `+r4` changes nothing; its effect is `+s`-search / clitic-counting only. Treat as a documented FREQ no-op, or wire only if a `+s`-search interaction is later needed. **`+r5` (`R5` toggle) / `+r50` (`R5_1` toggle):** `[: text]` text replacement. Default replaces the preceding material with the bracket content (counts the replacement); `+r5` = no replacement (count the ORIGINAL). Live-probed on `1082.cha`: `gots [: got]` and `wɨspatsing@u [: whispering]` count as `got`/`whispering` by default but as `gots`/`wɨspatsing@u` under `+r5`. chatter's default already counts the replacement (matches CLAN). **`+r5` DONE (landed 2026-06-04):** count the original via `framework::ReplacementChoice::Original`, bundled with `+r6` into a typed `RetraceReplaceMode` axis threaded through the shared word-walker's new `push_replaced_word` helper; rewritten from `+r5` via `--replacement-mode original`. On `retrace.cha`, `male [: female] [/] male [: female]` counts `female` by default but `male` under `+r5` (a clean `female`→`male` swap). Byte-parity golden `freq_r5_replace` (MatchesClan) + subprocess `freq_r5_counts_original_not_replacement`. `+r50` (`R5_1`) is a DISTINCT variant, **deferred as MOR / cross-tier (characterized 2026-06-04)**: parsed at `cutt.cpp:9551-9552` (`R5_1 = !R5_1`), it toggles `[= text]` replacement in the LINKED-TIER (`::`) / `%mor` path, not the base main-tier count. Its only consumer is `cutt.cpp:3038` (`if (!R5_1)` ... `else ExcludeMORScope(...)`) inside the `linkTiers` + `uS.patmat(templineC3, ":: *")` branch, so it bites only when a `[= ]` scope is being excluded from a `%mor`/linked tier. That is why it had NO observable effect on `1082.cha` (no such linked-tier construct). It belongs with the `+d6`/`+d7`/`+d8` MOR/cross-tier cluster, NOT the main-tier `+r` work; deferred to the FREQ porting report. **`+r6` (`R6`, `cutt.cpp:9554`) ↔ `--include-retracings` DONE (FIXED 2026-06-04, was a no-op bug).** A retracing marker (`[/]`/`[//]`/`[///]`/`[/-]`) retraces the single immediately-preceding word (when not angle-bracketed); the default drops it, `+r6` keeps it (`retrace.cha` 13→18 / 13→16, `1082.cha` 908→948). The bug: FREQ counted via the non-retrace `framework::countable_words` and `FreqConfig` had no `include_retracings` field, so the parsed flag was never consumed (a rewriter-MAPPING test passed but no behavior golden existed). Fix: `FreqConfig.include_retracings` (from the shared `--include-retracings`), FREQ now counts via `countable_words_with_retracings`. Also corrected a latent conflation in that walker: `[: text]` `ReplacedWord`s ALWAYS count the replacement (a retraced replaced word `tika@u [: kitty] [//] kitty` → `kitty` ×2, never the original `tika@u`); the old `include_retracings`-counts-both branch (dead until now) was removed. Byte-parity golden `freq_r6_retrace` (MatchesClan, retrace.cha) + subprocess `freq_r6_includes_retraced_material`/`freq_default_excludes_retraced_material`. (1082.cha still has one base-normalization diff unrelated to `+r6`: the `@u`-form `ah@u`→`ah`; the prosodic `hm:`→`hm` diff that also showed here is now FIXED, see `+r7` below.) **Default prosodic-strip FIX + `+r7` keep-toggle DONE (landed 2026-06-04).** CLAN's default ignores within-word prosodic / CA markers (probe-confirmed STRIPPED: Lengthening `:`, SyllablePause `^`, StressMarker `ˈ`, CAElement `↑`, so `ca:t`/`hm:`/`ˈwater`/`wa↑ter` → `cat`/`hm`/`water`). chatter's grouping key (`cleaned_text()`) already stripped them, but the DISPLAY (`raw_text`-based) KEPT them, diverging from CLAN (`hm:`≠`hm`, the SAME key/display split as parens, the root of the pre-existing `1082.cha` `hm:` diff). Default FIXED: `parans_display` now renders the display AST-first as `Text + Shortening(per parens) + CompoundMarker`, EXCLUDING every prosodic/CA/overlap/underline/clitic `WordContent` element (exhaustive match, no catch-all); scoped via `has_prosodic`/`has_shortening` so a word with neither keeps the byte-identical `raw_text` path (ZERO golden blast radius, full suite green). Pinned by `freq_default_strips_within_word_lengthening` (`1082.cha` `hm:`→`hm`). **`+r7` (KEEP `:`/`^`/`~`) DONE:** a typed `framework::ProsodyMode` (`Strip` default / `Keep`) threads a unified `render_word_element` helper that re-includes Lengthening `:` / SyllablePause `^` / CliticBoundary `~` (the modelled subset of `R7Slash/Tilda/Caret/Colon`) in BOTH key and display, so `ca:t`!=`cat` and `1082.cha` `hm:` stays `hm:`. Rewritten from `+r7` via `--prosody-mode keep`. Pinned by `freq_r7_keeps_within_word_lengthening` (+r7 keeps `hm:`) / `freq_default_strips_where_r7_keeps` (control). Two DELIBERATE `+r7` divergences from CLAN's binary, both documented: (a) Stress (`ˈ`) / CA (`↑`) stay STRIPPED under `+r7` (the manual scopes `+r7` to `/~^:`; CLAN strips them too but with buggy `r`/`er` byte-fragment artifacts chatter does not reproduce); (b) **CLAN-DIV-006**: CLAN's `+r7` over-retains the whole CA / satellite-delimiter family (‡ U+2021 `NOTCA_VOCATIVE`, „ U+201E, ≠ U+2260, ↫ U+21AB) as counted tokens, beyond its documented `/~^:` scope, and emits the `E2 80`-lead ones (‡, „) as INVALID UTF-8 (`0x80 0xA1` / `0x80 0x9E`, lead byte dropped); on `1082.cha` that is 12 (`*INV1`) + 3 (`*CHI`) corrupt ‡ tokens CLAN adds under `+r7`. chatter never counts a satellite delimiter (not word content in the typed AST) and never emits invalid UTF-8; pinned by `freq_r7_does_not_retain_corrupt_ca_delimiters`. Mechanism: `+r7` clears all four `R7*` flags (`cutt.cpp:9569-9576`), flipping the per-word cleanup gate (`cutt.cpp:7258`) from `HandleSlash` (`:7264`) to `HandleSpCAs` (`:7273`), which retains the delimiter family; `/` of `/~^:` has no chatter element (the slash is structural, not within-word content). **`+r8` (`R8`) deferred as MOR / cross-tier (characterized 2026-06-04):** it combines `%mor` items with the replacement word `[: …]` and error code `[* …]`; in source it forces FREQ onto the `%mor` tier (`freq.cpp:568-570`: `R8` -> `nomain = TRUE; maketierchoice("%mor",'+',FALSE)`). That is the same `%mor`-analysis territory as `+d6`/`+d7`/`+d8`, so it is deferred to the FREQ porting report, not implemented from the main-tier side. **Remaining (row stays Partial):** `+r8` and `+r50` are both **deferred (MOR/cross-tier)**, characterized above; `+r4` is a documented FREQ no-op. So every main-tier `+r` form is Done (`+r1`/`+r2`/`+r3`/`+r5`/`+r6`/`+r7` plus the default prosodic-strip); the row stays Partial only on the two MOR/cross-tier sub-flags, which are scope-gated, not effort-gated. The other `+rN` (`+r8`/`+r50`) currently fall through to clap as a positional file arg, a separate fail-open worth tightening at the boundary (the `+d`/`+QQQ` fix did not cover bare `+r8`). |
| `+zN-M` | Utterance range | `--range` | Done | |
| `+pS` | Add `S` to word delimiters | `--word-delimiters S` | Done | Landed 2026-06-04. CLAN appends each non-space character of `S` to its global word-delimiter set and re-tokenizes (`cutt.cpp:9798-9818`; manual `cutt.cpp:9204` "add S to word delimiters. (+p_ will break New_York into two words)"); FREQ enables `P_OPTION` (`option_flags[FREQ]` includes `ALL_OPTIONS`, `cutt.cpp:8648`). A counted word containing a delimiter is split at it and each piece is counted on its own; a trailing word-form marker (`@o`) stays on the FINAL segment (CLAN re-tokenizes the cleaned text, so the suffix rides its piece). Modeled as a reusable typed `framework::WordDelimiters` newtype (whitespace dropped on construction, `!isSpace`), threaded onto `FreqConfig.word_delimiters` and applied in `count_main_tier` AFTER the `+r` word-form treatment: when non-empty, the rendered display form is split on the delimiter chars and each non-empty segment is recorded (key + display), leaving the default path byte-identical (no existing golden moved). Rewritten from `+pS` -> `--word-delimiters S` (a chatter-only flag name; faithfulness rule); the empty form `+p` errors at the FREQ dispatch with CLAN's "specify word delimiter characters" message (`cutt.cpp:9802`). **MatchesClan:** on `word-features/000829.cha` `*MOT` with `+p_`, `choo_choo` (x3) -> `choo` (6) and `chup_chup_chup_chup@o` (x2) -> `chup` (6) + `chup@o` (2); chatter's full 79-line per-speaker output is byte-identical to CLAN's (live diff). Golden `freq_p_word_delimiter_mot`; CLI paths pinned by `freq_p_underscore_splits_joined_words` / `freq_default_keeps_underscore_joined_words` / `freq_p_empty_delimiters_errors`; rewriter by `freq_p_maps_to_word_delimiters`; the split primitive by `word_delimiters_split_behaviour`. **Scope:** the `+s`/`+c` per-word filters still gate on the whole (pre-split) word, so `+p` combined with `+s`/`+c` is a documented deferral; dependent-tier (`+t%X`) splitting under `+p` is also not yet wired. |
| `+f` / `+fEXT` | Output to file with extension | `--output-ext` (rewriter target) | Rewriter only | chatter writes to stdout by default; sidecar-file pattern is Phase 1.1. |
| `+u` | Combine across files (no per-file split) | (default) | Done | chatter combines by default; `--per-file` opts in to per-file output. Inverse default vs CLAN: explicit `+u` is a faithful no-op. Proven by `freq_combines_across_files_by_default_and_plus_u_is_a_noop`; the combined CHI table is byte-identical to CLAN's `+0 +u` (verified against `OSX-CLAN/src/clan/freq.cpp`). Structural flag (multi-file orchestration), so the proof is a subprocess test, not a single-file render golden. |
| `+re` | Recurse subdirectories | (default for directory args) | Done | chatter's path argument accepts a directory and recurses by default; explicit `+re` is a faithful no-op. Proven by `freq_recurses_subdirectories_by_default_and_plus_re_is_a_noop` (a file in a nested subdir lifts the combined total from 4 to 8 tokens). Structural flag (directory traversal), so the proof is a subprocess test, not a single-file render golden. |
| `+oS` / `-oS` | Include / exclude extra output tier `S` | -- (errors) | Done (rejects, matching CLAN) | **The "inherited" description is wrong for FREQ.** FREQ OWNS `case 'o'` (freq.cpp:815-836): it is the SORT/output-mode flag (`+o`/`+o0` -> `--sort frequency`, `+o1` -> `--sort reverse-concordance`, `+o3` -> `--combine-speakers`), and any other `+o<x>` falls into its `else` branch -> `"Invalid argument for option"` (freq.cpp:834). There is no "extra output tier" `+oS` form in FREQ; `+o2` (reverse-concordance, documented "non-CHAT", manual 5604) and `+oCHA` both error. chatter rejects every non-sort `+o` form at the fail-closed boundary, matching CLAN's rejection. Pinned by `freq_rejects_non_sort_o_forms` (`inapplicable_flag_tests.rs`). |
| `+x C N U` | Include only utterances whose length is `C` (`>`/`<`/`=`) than `N` items of unit `U` | `--utterance-length` (rewritten from `+x…w`/`+x…c`/`+x…m`) | Partial | **Word (`w`), char (`c`), and morpheme (`m`) units done** (manual 6405; `cutt.cpp:16508` word, `cutt.cpp:16343` char, `cutt.cpp:16409` morpheme). `+x>3w` keeps utterances with >3 countable words; `+x>20c` keeps those with >20 main-tier characters; `+x=5m` keeps those with exactly 5 traced morphemes. All three are the reusable `FilterConfig::utterance_length` gate carrying a typed `CountUnit` axis (`w`/`c`/`m`), not a scalar suffix. The morpheme unit counts §7.21-traced morphemes via the shared `framework::count_traced_morphemes_in_utterance` (the SAME counter MLU uses), so it is correct on UD `%mor` including the present participle `-Ger`. **`DivergesFromClan` (two CLAN defects, see field guide):** CLAN's `+x…m` (`CntFUttLen==2`) over-counts (its raw `ismorfchar` delimiter walk treats UD features like `Past`/`S3` as separate morphemes) AND doubles FREQ's output (it leaks the `%mor` tier into the harvest, `+x>0m` makes `*SPE` 15→30, emitting both `Triangle` and `noun\|triangle`), neither sanctioned by manual 6405. chatter filters by the correct count and emits normal FREQ output. Pinned by `freq_x_morpheme_filter_keeps_matching_utterances` (`-Ger`-sensitive); word/char by `freq_x_wordlen_eng`/`freq_x_charlen_eng`. The shared counter also corrected MLU's matching `-Ger` undercount. **Content-specification forms `+xS`/`-xS`** (manual 6405; `wdUttLen`/`filterwords`/`excludeUttLen`, `cutt.cpp:5380`,`5421`,`16340`): probe-confirmed that they tune only the `+x` LENGTH count, not FREQ's word output (`+x>0w +xxxx` makes a 2-word `xxx` utterance count 3 yet `xxx` stays out of the output). **`-xS` (exclude) DONE** for word/char units: removes word `S` from the length count, modeled as `UtteranceLengthFilter::exclude_from_count` (`WordPattern`/`word_pattern_matches`, the `+s` matcher); rewritten from `-x<word>` via `--utterance-length-exclude`. Pinned by `freq_x_exclude_word_from_length_count` (`+x>3w -xup` drops `*GES`'s "twirled up the hill", GES 10→6). **`+xxxx`/`+xyyy`/`+xwww` (unintelligible-marker restore) DONE as a third `DivergesFromClan` (CLAN-DIV-005):** by default the count strips `xxx`/`yyy`/`www`; these flags re-include them in the length count (rewritten to `--utterance-length-restore <marker>`, folded into a typed `RestoreMarkers` set on the filter). chatter restores UNCONDITIONALLY through the shared AST walker `framework::words_for_utterance_length`, so restored markers obey the same group-recursion / retrace rules as countable words. **CLAN's restore is a byte-position bug:** `correctForXXXYYYWWW` (`cutt.cpp:16260-16311`) advances its scan index `i += 2` once per marker check (xxx, then yyy, then www) plus the `for`-loop's `i++` = a stride of 7, so it only ever inspects `xxx` at byte offsets ≡ 0 (mod 7), `yyy` at ≡ 2, `www` at ≡ 4. Whether a marker restores therefore depends on its byte position in the cleaned line, contradicting manual 6405's unconditional-restore intent (verified by a 7-utterance probe: `xxx` restored only at offset 7, not at offsets 2-6 or 8). On `1082.cha` `+x=4w +xxxx`, chatter's correct restore lifts `<we xxx bein(g) in Rochester> [?]` from 4 to 5 words and drops it (`*CHI` 94 tokens); CLAN's stride bug leaves that `xxx` un-restored and keeps the utterance (`*CHI` 98). Pinned by `freq_x_restore_xxx_into_length_count` + `freq_x_restore_xxx_diverges_from_clan_byte_stride_bug`; the default-strip control is `freq_x_default_strips_xxx_from_length_count`. **`-x@FILE` (exclude-from-file) DONE:** loads the exclude word list from a file (`rdexclfUttLen('e', …)`, `cutt.cpp:5384`; same `@FILE` idiom as `-s@F`), folded into the same `exclude_from_count`; rewritten via `--utterance-length-exclude-file`. Pinned by `freq_x_exclude_words_from_file` (a one-line `up` list reproduces `-xup`, GES 10→6). **`+xWORD`/`+x@FILE` (general include) ESCALATED (maintainer-report item), fail-closed:** the manual (`cutt.cpp:9886` usage text) calls `+xWORD` "count only this word", but the binary's `excludeUttLen` (`cutt.cpp:5421`) default-counts every unmatched word, so the include list is a near-no-op that only bites as an exception to an overlapping exclude pattern, with an arbitrary descending-alphabetical precedence (`InsertUttLenWord`, `cutt.cpp:5326`). The manual intent contradicts the binary's behavior, so the correct semantic needs a maintainer call (bucket-3 escalation); kept fail-closed (`freq_x_include_forms_fail_closed`) pending it. The row stays Partial only on this one escalated sub-form; everything else implementable is done. |
| `+k` | Case-fold toggle: FREQ preserves case by default, `+k` folds to lowercase | `--case-sensitive` (carries `+k`) | Done | FREQ is in CLAN's `mmaininit` `nomap=TRUE` set (cutt.cpp:7845), so it PRESERVES case by default; `+k` TOGGLES `nomap` (cutt.cpp:13816) to fold to lowercase, so `Want`/`want`/`WANT` collapse to one entry and the displayed word is lowercased (freq.cpp:1892-1909). This is the INVERSE of the fold-by-default commands (KWAL/COMBO/FREQPOS/DIST/MAXWD); chatter sets FREQ's keying preserve-state to `!(+k present)`. Pinned by `freq_default_preserves_case_variants`, `freq_plus_k_folds_case_variants`, and the `freq_k_folds_case_eng` golden. |
| `+wN` / `-wN` | Context window (KWAL/COMBO keyword-context) | -- | Rewriter only | Inapplicable to FREQ (per-word frequency totals, no per-match context to surround). The rewriter maps `+wN`/`-wN` to `--context-after`/`--context-before`, but FREQ has no such clap field, so it errors at parse time, the correct outcome for an inapplicable flag (talkbank-clan/CLAUDE.md). CLAN's binary instead empties the output (a context-machinery artifact). |
| `+y` / `+yN` | Treat input as NON-CHAT plain text (so FREQ "sees" header / non-tier lines) | -- (errors) | Done (rejects, inapplicable) | **The audit's "include all utterances including non-tier" gloss was imprecise.** CLAN's `+y` sets `chatmode = 0` (cutt.cpp:9834), switching FREQ to analyze the input as raw plain text rather than parsed CHAT (manual 5044-5047: `freq +y +s"\**:" *.cha` to count header lines); `+yN` (N=0/1) picks line-vs-utterance granularity for that raw mode (cutt.cpp:9848) and only fires when `!chatmode`. On a CHAT file this empties the normal FREQ output (verified: `freq +y +t*MOT` yields no per-speaker body), exactly the `+w` shape, a flag whose machinery makes FREQ produce nothing useful for the typed-CHAT case. chatter is CHAT-only with no raw-text mode, so `+y` is inapplicable; per the inapplicable-flags rule (`talkbank-clan/CLAUDE.md`) chatter rejects it rather than fabricate a non-CHAT mode or reproduce empty output. CLAN also errors `+y` with `+/-t` (if `-t` precedes it), `-u`, or `+d2`/`+d3` (cutt.cpp:9822-9833). Pinned by `freq_rejects_non_chat_mode_y_flag`. |

### Audit summary

| Bucket | Count |
|---|---|
| Done (byte-parity or in scope) | 27 |
| Partial (chatter abstraction differs, MOR/cross-tier sub-flags deferred) | 3 |
| Rewriter only (would error at parse time) | 12 |
| Deferred (scope-gated: MOR / cross-tier / non-CHAT / escalated / infra) | 6 |
| Missing (no rewriter, no source-grounded reason yet) | 0 |

**FREQ is depth-complete (2026-06-04).** Every FREQ flag-row is now in
one of: **Done** (byte-parity, or rejects-matching-CLAN for inapplicable
flags), **Partial** (a chatter abstraction differs, with the residual
sub-flags themselves deferred-with-reason), **Rewriter only** (errors at
parse, the correct outcome for an inapplicable flag), or **Deferred**
(scope-gated with a source-grounded reason: PI / `%mor` / cross-tier
decisions, flags that switch CLAN to a non-CHAT raw-text mode chatter
has no analog for, a manual-vs-binary contradiction escalated to the
maintainer, or output-routing infra). There are **zero rows left
"Missing" without a source-grounded reason.** The deferred rows are
consolidated for the maintainers in the FREQ porting report. The
deferred set: `+d0`/`+d6`/`+d7`/`+d8` and bare
`+d` (concordance / `%mor` / cross-tabulation), `+r8`/`+r50` (`%mor` /
linked-tier), `+c5` (gated on `+d7`), `+c6` (CA repeat-segment, needs a
parser span type), `+a` (`typeForms`, manual-unclear + MOR-coupled),
`+xWORD`/`+x@FILE` (manual-vs-binary contradiction), `+f`/`+fEXT`
(sidecar-file output infra).

**Resolved 2026-06-04 (fail-closed boundary).** The former "Rewriter
only" hazard, a CLAN user pasting `freq +d20 file.cha` getting silent
default output, is closed. Any `+`/`-` token that no rewriter arm
consumes (`+d20`, `+d8`, `+QQQ`, ...) now ERRORS at the shared
file-discovery boundary (`DiscoveredChatFiles::into_files`,
`talkbank-clan/src/framework/input.rs`) with
`Error: unrecognized option(s): '+d20' (a '+'/'-' argument is a CLAN flag, not a file)`,
rather than being collected as a bogus positional file and warn-skipped
to a default-output exit 0. This fixed a systemic fail-open hole across
every `chatter clan` command at once; see the parity field guide. The
remaining `+d` rows above are tracked individually for per-row TDD
implementation (several PI-gated). The per-category counts above are
hand-maintained and superseded by the generated golden-parity metric on
the [parity status page](../parity-status.md).

### Confirmed-broken invocations (2026-05-21)

These were exercised end-to-end during the audit and produced wrong
output for a CLAN-equivalent invocation:

| Invocation | What chatter does | What CLAN does |
|---|---|---|
| ~~`chatter clan freq +d2 file.cha`~~ | **fixed 2026-06-02**: `+d2` writes an aggregate SpreadsheetML file (`stat.frq.xls`); see the `+d2`/`+d3` rows above | spreadsheet output |
| ~~`chatter clan freq +k file.cha`~~ | **fixed**: `+k` rewrites to `--case-sensitive` (case-fold toggle; see the `+k` row above) | case-sensitive search |
| ~~`chatter clan freq +t%mor file.cha`~~ | **fixed 2026-06-03**: `+t%mor` rewrites to `--tier mor` and counts the `%mor` tier's tokens (see the `+t%X` row above) | analyses `%mor` dependent tier |
| ~~`chatter clan freq +tCHI file.cha` (no `*`)~~ | **fixed 2026-05-21**: `+tCHI` and `-tMOT` now rewrite identically to `+t*CHI` / `-t*MOT` | identical to `+t*CHI` (silently prepends the `*`) |

The `+tCHI` case was a `clan_args.rs::rewrite_tier_speaker` gap: the
function required the first byte of `rest` to be `*`, `%`, or `@`,
and fell through to `None` otherwise. The default branch now treats
`+t<word>` as an implicit speaker code, matching CLAN's behaviour
exactly. Closed in commit landed alongside this audit, with two new
unit tests (`speaker_include_no_asterisk`,
`speaker_exclude_no_asterisk`) in `clan_args::tests`.

## CLAN Equivalence

| CLAN command | Rust equivalent |
|---|---|
| `freq file.cha` | `chatter clan freq file.cha` |
| `freq +t*CHI file.cha` | `chatter clan freq file.cha --speaker CHI` |
| `freq +s"the" file.cha` | `chatter clan freq file.cha --include-word "the"` (case-sensitive matching not currently supported, see callout above) |
| `freq *.cha` | `chatter clan freq corpus/` |

## Display Modes (`+dN` / `--display-mode N`), DRAFT, awaiting PI review

> **Status: partially implemented.** `+d` is not a single scalar "display
> mode": CLAN's `case 'd'` (freq.cpp:838-913) overloads the prefix onto
> several independent state variables (`onlydata`, `zeroMatch`,
> `isCrossTabulation`, `isSpreadsheetOnePerRow`, the `percentC`/`percent`
> filter, ...). Each is rewritten to its own chatter flag, NOT a generic
> `--display-mode N`. **Done:** `+d1` (`--word-list-only`), `+d2`/`+d3`
> (`--spreadsheet per-word`/`summary`), `+d4` (`--types-tokens-only`), `+d5`
> (`--include-zero-frequency`, zeroMatch), `+d20` (`--spreadsheet
> per-speaker-word`), and the `+dCN` percent-of-speakers filters
> (`--speaker-percentage <spec>`, e.g. `+d<=50`; see the `+dCN` row above).
> **Remaining:** `+d0` (concordance), `+d6`/`+d7`/`+d8` (`%mor` / cross-tier),
> and bare `+d`. The per-N table below quotes CLAN manual §7.10.15 verbatim;
> the open questions that follow are the genuinely PI-gated scope decisions for
> the remaining rows. **As of 2026-06-04 every not-yet-implemented `+d` form
> (bare `+d`, `+d0`, `+d6`/`+d7`/`+d8`) ERRORS fail-closed** at the
> file-discovery boundary rather than silently producing default output;
> implementing each is its own per-row TDD. Note CLAN source treats bare `+d`
> identically to `+d0` (`onlydata = 1`, cutt.cpp:9402), which the manual's
> "`+d` = no-flag default" line below contradicts: reconcile before
> implementing either.

`FREQ` uses `+d` to switch output format, *not* to vary verbosity. Each
value of N selects a different report shape. Quoted from CLAN manual
§7.10.15:

| N | CLAN behavior (verbatim from manual) |
|---|---|
| `+d` (no number) | "Perform a particular level of data analysis. By default, the output consists of all selected words found in the input data file(s) and their corresponding frequencies." (Equivalent to no-flag default.) |
| `+d0` | "Output provides a concordance with the frequencies of each word, the files and line numbers where each word, and the text in the line that matches." |
| `+d1` | "Outputs each of the words found in the input data file(s) one word per line with no further information about frequency. Later this output could be used as a word list file for `kwal` or `combo` programs." |
| `+d2` | "Output is sent to a file in a form that can be opened directly in Excel. To do this, you must include information about the speaker roles you wish to include in the output spreadsheet." (Manual example: `freq +d2 +t@ID="*|Target_Child|*" *.cha`.) |
| `+d3` | "Essentially the same as that for `+d2`, but with only the statistics on types, tokens, and the type-token ratio. Word frequencies are not placed into the output." (Note: `+d2` and `+d3` assume `+f`; no need to pass it explicitly.) |
| `+d4` | "Allows you to output just the type-token information." |
| `+d5` | "Output all words you are searching for, including those that occur with zero frequency. ... Can be combined with other `+d` switches." |
| `+d6` | "When used for searches on the main line, outputs matched forms with a separate tabulation of replaced forms, errors, partial omissions, and full forms." Also `+d6 +sm\|n*,o%` on `%mor` line: produces separate counts per part-of-speech instantiation. |
| `+d7` | "Links forms on a 'source' tier with their corresponding words on a 'target' tier." Default source is `%mor`; pass a tier name to change source. Items on the two tiers must be in one-to-one correspondence. `+c5` swaps source ↔ target. |
| `+d8` | "Outputs words and frequencies of cross tabulation of one dependent tier with another." |

### Open questions for PI review

1. `+d0`: emits a concordance, overlaps with `KWAL` semantically. Should
   chatter's `freq --display-mode 0` reuse the `kwal` output path
   internally, or produce its own concordance shape?
2. `+d1`: word-list output suitable as input to `kwal +s@file`. Should
   the file be auto-named (`<basename>.fre`?) or printed to stdout by
   default?
3. `+d2`/`+d3`: "form that can be opened directly in Excel" maps to
   `--format csv` in chatter. Is this duplication acceptable, or should
   `--display-mode 2` *imply* `--format csv` (and conflict-error
   otherwise)?
4. `+d4`: "type-token information only", same content as the existing
   text/json default, minus the word frequencies. Add a new
   `Truncated` variant to the output struct, or emit a CSV row with
   just types/tokens/TTR?
5. `+d5`: combinable with other `+d` values. How should this combine in
   clap, a `Vec<DisplayMode>` rather than scalar `Option<u8>`?
6. `+d6`/`+d7`/`+d8`: deeply specific to `%mor` and cross-tier
   tabulation. Are these in scope for chatter's freq, or are they
   future work (probably alongside or instead of `mortable` /
   `freqpos`)?

The `+dCN` form ("output only words used by <, <=, =, => or > than N
percent of speakers") is implemented (2026-06-04; see the `+dCN` row in
the flag table above). The manual's `C` is the **comparator metavariable**
(`<`/`<=`/`=`/`>=`/`>`), not a literal capital `C`: CLAN's source has no
`case 'C'`, only the `<`/`=`/`>` parse at freq.cpp:841-861. The rewriter
maps `+d<=50` -> `--speaker-percentage <=50` (its own clap field, not an
overload of `--display-mode`).

## Output

Per-speaker frequency tables with:

- Word frequency counts (sorted by count descending, then alphabetically)
- Total types (unique words) and tokens (total words)
- TTR (type-token ratio = types / tokens)

### Example output (text)

```text
Speaker: CHI
  the       12
  I         8
  want      6
  a         5
  go        4
  ...
Types: 45
Tokens: 127
TTR: 0.354
```

### Example output (JSON)

```json
{
  "speakers": {
    "CHI": {
      "words": { "the": 12, "I": 8, "want": 6, ... },
      "types": 45,
      "tokens": 127,
      "ttr": 0.354
    }
  }
}
```

## Word Normalization

Words are grouped using `NormalizedWord`, which lowercases and strips compound markers (`+`) for counting purposes, while preserving the original CLAN display form (with `+`) for output. This means `wanna+go` and `Wanna+Go` are counted as the same word.

## Differences from CLAN

### Word identification

The legacy manual says `FREQ` ignores `xxx`, `www`, and words beginning with `0`, `&`, `+`, `-`, or `#` by default, and also ignores header and code tiers unless selected. CLAN implements much of this with character-level string-prefix matching:

```c
if (word[0] == '0') continue;     // omitted words
if (word[0] == '&') continue;     // fillers/nonwords
if (word[0] == '+') continue;     // terminators
```

Our implementation uses AST-based `is_countable_word()`, which checks semantic type rather than string prefixes. This is more precise -- a filler (`&-um`) and a phonological fragment (`&+fr`) have distinct semantic types in our model, even though CLAN lumps them together under the `&` prefix.

### Manual features not yet mirrored directly

The legacy manual documents several advanced `FREQ` workflows, including `+s@file` lexical-group lists, `%mor`/`%gra` combined search with `+d7`, and multilingual searches. Some of those behaviors are covered in `talkbank-clan` through broader filtering infrastructure, but the command chapter should not imply one-for-one flag parity unless explicitly implemented.

### Output ordering

Output is deterministic via sorted collections (count descending, then alphabetically). CLAN's ordering can vary across runs.

### Output formats

Supports text, JSON, and CSV formats. CLAN produces text only. Use `--format clan` for character-level CLAN-compatible output.

### Multi-file behavior

Results are merged across files by default (`+u` behavior). CLAN requires explicit `+u` flag. Use `chatter clan freq dir/` for recursive directory traversal (CLAN uses shell globs).

### Golden test parity

Verified against CLAN C binary output. 100% parity.
