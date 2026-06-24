# MLU -- Mean Length of Utterance

**Status:** Current
**Last updated:** 2026-06-05 10:23 EDT

## Purpose

Calculates mean length of utterance in morphemes from the `%mor` tier. When no `%mor` tier is available and `--words` was not passed, reports "utterances = 0, morphemes = 0" (matching CLAN behavior -- no fallback to word counting).

See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409094) for the original MLU command specification.

## Usage

```bash
chatter clan mlu file.cha
chatter clan mlu --speaker CHI file.cha
chatter clan mlu --words file.cha
chatter clan mlu --format json corpus/
```

## Options (chatter-native)

| Option | CLAN Flag | Description |
|--------|-----------|-------------|
| `--speaker <CODE>` | `+t*CHI` (or `+tCHI`) | Include speaker |
| `--exclude-speaker <CODE>` | `-t*CHI` (or `-tCHI`) | Exclude speaker |
| `--words` | `-bw` | Count words from main tier instead of morphemes from `%mor` |
| `--gem <LABEL>` | `+g"label"` | Restrict to gem segment |
| `--range <START-END>` | `+z25-125` | Utterance range |
| `--id-filter <PATTERN>` | `+t@ID="..."` | Filter by @ID pattern |
| `--include-retracings` | `+r6` | Include retraced words in counting |
| `--include-xxx` | `+sxxx` | Re-admit `xxx` (unintelligible) utterances to the count |
| `--include-yyy` | `+syyy` | Re-admit `yyy` (phonological) utterances to the count |
| `--combine-speakers` | `+o3` (or bare `+t*`) | Pool selected speakers into one `*COMBINED*` result |
| `--format <FMT>` | -- | Output format: clan (default), text, json, csv |

## CLAN Equivalence

| CLAN command | Rust equivalent |
|---|---|
| `mlu file.cha` | `chatter clan mlu file.cha` |
| `mlu +t*CHI file.cha` | `chatter clan mlu file.cha --speaker CHI` |
| `mlu -bw file.cha` | `chatter clan mlu file.cha --words` |

## CLAN `+`-flag coverage audit

Authoritative enumeration of every CLAN `mlu` flag, mapped against
chatter's coverage. Sources:

* `OSX-CLAN/src/clan/mlu.cpp`: `usage()` at line 51 and the
  command-specific `getflag()` intercept at line 669.
* `OSX-CLAN/src/clan/cutt.cpp`: `mainusage()` MLU branches.
* `crates/talkbank-clan/src/clan_args.rs`: chatter's `+flag` to
  `--flag` rewriter.
* `crates/talkbank-cli/src/cli/args/clan_common.rs` and
  `clan_commands.rs::Mlu`, chatter's clap field surface.

(Status legend: same as
[FREQ](./freq.md#status-legend), Done / Partial / Rewriter only /
Missing.)

### MLU-specific `+`-flags (from `mlu.cpp::getflag`)

| CLAN flag | Meaning | Chatter | Status | Notes |
|---|---|---|---|---|
| `-bw` | Count words, not morphemes | `--words` | Done | Direct mapping; doc note above. |
| `-bc` | Count characters, not morphemes | (reject) | Done | The unix CLAN binary (our parity target) ERRORS on `-bc`: the character branch is `#ifndef UNX` (mlu.cpp:678-680), so `getflag` falls through to "Please specify w - words after -b option." (mlu.cpp:686). chatter is fail-closed too (clap rejects; documented message divergence, like the `+w`-on-aggregate reject). Counting characters is a legitimate measure but would live on a chatter-only flag, never the CLAN `-b` slot. Pinned by `mlu_bc_fails_closed_no_silent_count`. |
| `+cS` | Clause-marker delimiter `S` |, | Missing | Used to break utterances into clauses for MLU calculation. |
| `+c@F` | Clause-markers listed in file `F` |, | Missing | File-list workflow. |
| `+sxxx` | Re-include `xxx` utterances in the count | `--include-xxx` | Done | Landed 2026-06-05. CLAN `ml_isXXXFound` (mlu.cpp:766) stops dropping `xxx`-containing utterances (`mlu_excludeUtter` blanks the token instead of returning TRUE, mllib.cpp:337) and switches to the two-line `+sxxx` header (mlu.cpp:250-251); the `xxx` string still contributes no morpheme. Manual §7.21 pt5. Byte-parity golden `mlu_sxxx_include` (manchester-anne.cha CHI: utt 3 / morph 4). Modeled as `MluConfig.re_included_untranscribed`. |
| `+syyy` | Re-include `yyy` utterances in the count | `--include-yyy` | Done (binary-matched; **documented divergence**) | The CLAN binary accepts `+syyy` (sets `ml_isYYYFound`, mlu.cpp:776) and re-includes the `yyy` utterances, but (a) the manual §7.21 pt2 says "yyy and www are always excluded and cannot be included", and (b) CLAN has NO yyy-only header branch (mlu.cpp:246-253), so `+syyy` alone keeps the DEFAULT header while changing the count. chatter reproduces the binary (the `chatter clan` parity oracle), pinned by the header-variant unit test; the manual conflict is recorded under "Documented CLAN-bug divergences" below for PI adjudication. |
| `+sxxx`/`+syyy` default (no flag) | Exclude utterances containing `xxx`/`yyy`/`www` | (default) | Done | Landed 2026-06-05 (base-command bug fix). Manual §7.21 pt2: xxx/yyy/www and the utterances containing them are excluded by default; `mlu_excludeUtter` (mllib.cpp:303-348) returns TRUE on the MAIN tier (mlu.cpp:509). chatter previously counted utterances that mixed `xxx` with real words (e.g. `it xxx xxx`). Byte-parity golden `mlu_xxx_exclude`. |
| `-sxxx`/`-syyy`/`+swww`/`-swww` | (reject) | (reject) | Done | CLAN errors: "Excluding xxx is not allowed" (mlu.cpp:768), "Including/Excluding www is not allowed" (mlu.cpp:784); www is never includable. chatter fails closed (clap rejects a self-documenting non-existent flag, because `--speaker` owns `-s` and a plain pass-through would be misread as `--speaker`). Pinned by `mlu_s_untranscribed_guards_fail_closed`. |
| `[+ mlue]` postcode | Exclude this utterance from MLU | (default) | Done | Landed 2026-06-05 (base-command bug fix). `isMLUEpostcode` defaults TRUE (mlu.cpp:108); `isPostCodeOnUtt(line, "[+ mlue]")` -> `isSkip` (mlu.cpp:503). chatter checks `MainTier::content.postcodes` for the `mlue` payload. Byte-parity golden `mlu_mlue_postcode_exclude` (trimmed NINJAL-Okubo fixture). The `+s"[+ mlue]"` force-include override (manual §7.21 pt5) is not yet surfaced; the default always excludes. |
| `+gS` | Exclude utterances consisting solely of word `S` | `--exclude-solo-word S` | Done | Fixed 2026-05-22. CLAN's MLU `+gS` overload (vs the inherited gem-segment filter) is now routed via per-subcommand rewriter branch to a new clap field `--exclude-solo-word`. Drops utterances whose every countable word is in the list. Case-insensitive. |
| `+g@F` | `+g` from file | `--exclude-solo-word-file` | Done | Landed 2026-05-23. Same idiom as COMBO/KWAL `+s@F`, rewriter intercepts `+g@F` before the per-word `+gS` arm, dispatch loads via `load_search_expr_file` and extends `--exclude-solo-word`. File format matches CLAN's `cutt.cpp::rdexclf`: one pattern per line, skip blank lines, `#`-comments, and `;%*`-annotation lines. Repeatable. Pinned by `mlu_solo_word_from_file`. |
| `+o3` | Combine selected speakers into one result | `--combine-speakers` | Done | Landed 2026-06-05. CLAN `mlu_isCombineSpeakers` (mlu.cpp:721): pool every selected speaker's utterances into one result rendered `*COMBINED*` (no `:`), with the morpheme/utterance/SD aggregate over the merged length list. `+o3` is the ONLY valid MLU `+o` form (any other `+o<x>` is "Invalid argument", mlu.cpp:723), so other values fall through to fail-closed. Modeled as `MluConfig.combine_speakers`; `finalize` pools the per-speaker `utterance_lengths` and the renderer emits `*COMBINED*` (gated on `MluResult.combine_speakers`). Reuses the FREQ `+o3` `--combine-speakers` flag name + rewriter pattern. Byte-parity golden `mlu_o3_combine` (mor-gra.cha: CHI+MOT -> utt 2 / morph 11 / ratio 5.500 / SD 0.500); CLI pinned by `mlu_o3_combines_speakers_into_one_block`; rewriter by `mlu_o3_and_word_form_noop_flags_rewrite`. |
| `+t%mor` (implicit) | Switch to `%mor` tier (special handling) | (default) | Done | chatter reads `%mor` by default; `+t%mor` is a CLAN re-confirmation. |
| `-t%mor` | Exclude `%mor` tier, implies `--words` semantics | `--words` | Done | Landed 2026-05-23. Rewriter special-cases `-t%mor` under MLU/MLT to emit `--words` instead of the generic `--exclude-tier mor` (which MLU/MLT's clap doesn't accept). Pinned by `mlu_exclude_mor_tier_maps_to_words`, `mlt_exclude_mor_tier_maps_to_words`, and the fall-through `mlu_exclude_non_mor_tier_falls_through` (which confirms `-t%pho` and other non-`%mor` values still route to the generic `--exclude-tier`). |

### General `+`-flags MLU inherits (from `cutt.cpp::mainusage`)

| CLAN flag | Meaning | Chatter | Status | Notes |
|---|---|---|---|---|
| `+t*X` / `-t*X` | Include/exclude speaker | `--speaker` / `--exclude-speaker` | Done | `+tX` (no `*`) also accepted post-2026-05-21 fix. Bare `+t*` (star, NOTHING after) is a hidden alias for `+o3` (mlu.cpp:735, `*f == '*' && *(f+1) == EOS` sets `mlu_isCombineSpeakers`) -> `--combine-speakers`; pinned by `mlu_bare_star_t_combines_like_o3`. |
| `+t%X` / `-t%X` | Include/exclude dependent tier `%X` | `--tier` / `--exclude-tier` (rewriter target) | Rewriter only | No clap field on MLU; only `+t%mor` is handled implicitly by the default-mor logic. Other `%X` errors at parse time. |
| `+t@ID="..."` | Filter by @ID pattern | `--id-filter` | Done | Banner mapping deferred (PLAN §1.6). |
| `+t#ROLE` | Filter by role | `--role` | Done | Fixed 2026-05-22; see [FREQ](./freq.md) for the shared implementation. |
| `+s"word"` / `-s"word"` | Include/exclude word in counting | `--include-word` / `--exclude-word` | Partial | MLU's `+s` has command-specific scope. The untranscribed forms (`+sxxx`/`+syyy`/`-sxxx`/`+swww`/...) are now handled (rows above). Still Partial: the postcode force-include forms `+s"[+ mlue]"`/`+s"[+ trn]"` (manual §7.21 pt5) and the `[+ mlue]`-bracket / retrace-pattern (`+s+"</>"`) search forms; chatter's general word filter is the simple form only. |
| `+s@F` / `-s@F` | Search / exclude words from file | `--include-word-file` / `--exclude-word-file` | Done | Landed 2026-05-22. File format: one pattern per line; blank lines, `# `-comments, and `;%* `-annotation lines skipped. Repeatable. |
| `+gX` | (in MLU: utterance-elision, see above) | `--gem` (banner default) | Partial | `+g` collides between MLU's elision semantic and the general gem filter. CLAN's MLU `+g` is the *elision* meaning per `getflag`; `--gem` is the general filter (semantics differ). Confusing in CLAN itself. |
| `+zN-M` | Utterance range | `--range` | Done | |
| `+rN` | Various retrace / clitic / prosodic-symbol / replacement controls | `--include-retracings` (`+r6`); `+r1`/`+r2`/`+r3`/`+r4`/`+r5`/`+r7` no-op | Done | `+r6` (include retracings) is the only `+r` with a real MLU effect (`--include-retracings`). `+r1`/`+r2`/`+r3`/`+r4`/`+r5`/`+r7` are FAITHFUL NO-OPS: they tune the word FORM, but MLU counts morphemes (from `%mor`) or words (`-bw`) and never displays a form, so they change nothing (probe-confirmed 2026-06-05: morpheme total 6 and word total 189 identical with/without on mor-gra.cha and 000829.cha, both modes). MLU enables `R_OPTION` (`option_flags[MLU]`, cutt.cpp:8674), so CLAN accepts them; effect is `+s`-search only, like `+k`. Rewriter consumes them (`vec![]`); pinned by `mlu_word_form_flags_are_noops_*` and `mlu_o3_and_word_form_noop_flags_rewrite`. `+r8`/`+r50` carry over from FREQ as MOR/cross-tier deferred. |
| `+u` | Combine across files | (default) | Done | chatter combines by default. Inverse default vs CLAN. |
| `+re` | Recurse subdirectories | (default for directory args) | Done | |
| `+pS` | Add `S` to word delimiters | -- (no-op) | Done | FAITHFUL NO-OP. MLU enables `P_OPTION` (`option_flags[MLU]`, cutt.cpp:8674) so CLAN accepts `+pS`, but the extra word delimiters never reach MLU's morpheme/word count (probe-confirmed 2026-06-05: morphemes 196 and words 189 unchanged with `+p_` on 000829.cha, where `choo_choo` stays one word and the `%mor` tier already has `choochoo`). Unlike FREQ, MLU does not re-tokenize on `+p`. Rewriter consumes it (`vec![]`); pinned by the `mlu_word_form_flags_are_noops_*` tests. |
| `+k` | Case-sensitive matching | `--case-sensitive` (via `CommonAnalysisArgs`) | Done (no-op per CLAN) | MLU does no word-keying; `+k` is silently accepted per CLAN's `cutt.cpp::mainusage` no-op semantic. Covered by `CommonAnalysisArgs.case_sensitive` flatten on `ClanCommands::Mlu`. |
| `+wN` / `-wN` | Context window (KWAL/COMBO keyword-context) | -- | Rewriter only | Inapplicable to MLU (per-speaker morpheme means, no per-match context to surround). The rewriter maps `+wN`/`-wN` to `--context-after`/`--context-before`, but MLU has no such clap field, so it errors at parse time, the correct outcome for an inapplicable flag (talkbank-clan/CLAUDE.md). CLAN's binary instead empties the output (a context-machinery artifact). |
| `+f` / `+fEXT` | Output to file | `--output-ext` (rewriter target) | Rewriter only | Phase 1.1 sidecar work. |

### MLU `+d` display modes

See the "Display Modes (`+dN` / `--display-mode N`), DRAFT" section
below for the per-N table. All `+d` / `+d1` invocations are
**Missing** as of 2026-05-26: MLU has no local `case 'd'`;
consumption is via the shared `maingetflag` path at
`OSX-CLAN/src/clan/cutt.cpp:9382` with non-empty per-program body
at `cutt.cpp:9485` (CLAN_SRV-only rejection of `onlydata == 1 ||
3`; otherwise pure `onlydata`-level effect). chatter has no
`--display-mode` consumer for MLU. The per-MLU rewriter arm in
`clan_args.rs` passes the token through so clap reports the
literal `+d`/`+dN` argument rather than the misleading
`--display-mode` rewrite.

### Audit summary

| Bucket | Count |
|---|---|
| Done (byte-parity or in scope) | 19 |
| Partial (chatter abstraction differs) | 4 |
| Rewriter only (would error at parse time) | 2 |
| Missing (no rewriter, no clap field) | 4 (`+cS`, `+c@F`, `+d`, `+d1`) |

The remaining **Missing** rows are the clause-marker family (`+cS` / `+c@F`,
breaks utterances into clauses and relabels the output) and the Excel/@ID
display modes (`+d` / `+d1`). The **Documented CLAN-bug divergences** below
record the `+syyy` manual conflict.

### Documented CLAN-bug divergences

| Flag | Divergence | Adjudication |
|---|---|---|
| `+syyy` | The CLAN binary re-includes `yyy`-containing utterances (`ml_isYYYFound`, mlu.cpp:776) AND prints the DEFAULT exclusion header (no yyy-only branch, mlu.cpp:246-253). The manual §7.21 pt2 says yyy "cannot be included." chatter reproduces the binary exactly (the `chatter clan` parity oracle), so `+syyy` matches the binary including the header quirk. | The manual-vs-binary conflict is a genuine adjudication item for the PI / CLAN maintainer: should `+syyy` be an error (per the manual) or an include (per the binary)? chatter follows the binary until that is resolved, rather than diverging on a solo call. |

The `+g` overload is the most subtle issue: MLU's command-specific
`+g` means "exclude an utterance if it consists solely of the given
word" (a special-case elision filter), but chatter's `--gem`
inherited from `CommonAnalysisArgs` means "restrict to gem segment
labelled S" (a general gem filter). Identical syntax, different
semantics, a CLAN user pasting `mlu +gum file.cha` (skip
`um`-only utterances) gets gem-label filtering in chatter (a
no-op for files with no `@G um` gem). Tracked as a Phase 1.7
follow-up.

## Display Modes (`+dN` / `--display-mode N`), DRAFT, awaiting PI review

> **Status: drafted from CLAN manual; not yet implemented.** The
> rewriter at `crates/talkbank-clan/src/clan_args.rs:101` translates
> `+dN` → `--display-mode N`, but no `clap` field consumes that token
> today. Drafted from CLAN manual §7.21.2 (`Unique Options`, MLU) for
> PI review.

MLU's `+d` table is small, two N-values, both Excel-friendly output
formats. Quoted from CLAN manual §7.21.2:

| N | CLAN behavior (verbatim from manual) |
|---|---|
| `+d` (no number) | "You can use this switch, together with the ID specification to output data for Excel." Example: `mlu +d +tCHI sample.cha` produces a one-line @ID-keyed record: ``en\|sample\|CHI\|1;10.4\|female\|\|\|Target_Child\|\| 5  7 1.400 0.490`` (fields: @ID, utterance count, morpheme count, MLU, MLU std dev). Requires `@ID` headers per participant. |
| `+d1` | "This level of the `+d` switch outputs data in another systematic format, with data for each speaker on a single line. However, this form is less adapted to input to a statistical program than the output for the basic `+d` switch. Also, this switch works with the `+u` switch, whereas the basic `+d` switch does not." Example: ``*CHI:  5  7 1.400 0.490``. |

**Binary probe (2026-06-05): the manual is OUTDATED; the binary is the parity
oracle.** Probing `OSX-CLAN/src/unix/bin/mlu` on `mor-gra.cha` shows the actual
behavior differs from the manual's examples:

- `+d` (no number) writes a SpreadsheetML file `<basename>.mlu.xls` (stdout only
  reports `Output file <...>`); it does NOT print the `*CHI: ...` line. This is
  the same family as FREQ `+d2`/`+d3` (semantic-equivalence standard, not byte,
  per the talkbank-clan crate guide), and forces `@ID:` header selection (the
  banner adds "and ONLY header tiers matching: @ID:").
- `+d1` prints TAB-separated, `@ID`-FIELD-EXPANDED lines to stdout, one per
  speaker, NOT the `*CHI: 5 7 ...` form the manual shows. On `mor-gra.cha`:
  ``pipeout<TAB>eng<TAB>corpus<TAB>CHI<TAB>3;00.<TAB>.<TAB>.<TAB>.<TAB>Child<TAB>.<TAB>.<TAB>    1<TAB>    6<TAB>  6.000<TAB>NA``
  (filename, then the ten `@ID` fields with empties rendered `.`, then padded
  utterances / morphemes / MLU / SD). The filename is `pipeout` for stdin input.

Implementing these is the remaining MLU work (the only non-deferred Missing
rows). Both require `@ID` parsing + per-speaker `@ID`-field expansion and the
source filename in `FileContext`; `+d` additionally needs the `.mlu.xls`
SpreadsheetML writer (reuse the FREQ `+d2`/`+d3` spreadsheet path). Localize the
manual-vs-binary mismatch as "manual outdated" and reproduce the binary.

### Open questions for PI review

1. `+d` (no number) maps cleanly to `--format csv` in chatter. Should
   `--display-mode 0` (or absent N) imply `--format csv`, or remain a
   separate axis?
2. `+d1` is "less adapted to statistical input" yet combinable with
   `+u`. That combinability is the differentiating feature; should
   chatter expose it as a `--display-mode merged-by-speaker` enum
   variant?
3. The `+d` output requires `@ID` headers per participant. Should
   `--display-mode` error early if `@ID` rows are missing for any
   matched speaker, or fall back to the speaker-code-only form
   silently?

## Algorithm

For each utterance with a `%mor` tier:

1. Count **1 per stem** (the base morpheme word)
2. Count **+1 per bound morpheme suffix** -- but ONLY these 7 suffix strings: `PL`, `PAST`, `Past`, `POSS`, `PASTP`, `Pastp`, `PRESP`
3. Count **+1 per clitic stem** (`~` separated)
4. Count clitic suffixes using the same 7-string rule
5. **Fusional features** (`&PRES`, `&INF`, etc.) do NOT count

Per speaker, compute:
- Number of utterances
- Total morphemes
- **MLU** (mean = total morphemes / utterances)
- **Standard deviation** (population SD, dividing by n)
- **Range** (min, max morphemes per utterance)

### Brown's Morpheme Rules

This was a key discovery during parity verification. CLAN only counts 7 specific suffix strings as bound morphemes:

| Suffix | Meaning |
|--------|---------|
| `PL` | Plural |
| `PAST` | Past tense |
| `Past` | Past tense (alternate) |
| `POSS` | Possessive |
| `PASTP` | Past participle |
| `Pastp` | Past participle (alternate) |
| `PRESP` | Present participle |

All other suffixes (including fusional features like `&PRES`, `&INF`, `&3S`) are ignored for MLU counting. This matches Brown's (1973) original operationalization of "morpheme" for child language analysis.

### Example

Given `%mor: pro|I v|want-PAST det|a n|cookie-PL`:

- `pro|I` = 1 stem = **1**
- `v|want-PAST` = 1 stem + 1 suffix (PAST) = **2**
- `det|a` = 1 stem = **1**
- `n|cookie-PL` = 1 stem + 1 suffix (PL) = **2**
- Total: **6 morphemes**

## Output

```text
Speaker: CHI
  Utterances: 42
  Morphemes: 168
  MLU: 4.000
  SD: 1.732
  Range: 1-9
```

## Differences from CLAN

### Standard deviation

Uses **population SD** (dividing by n), not sample SD (dividing by n-1). Verified against CLAN output -- CLAN uses population SD too.

### Morpheme counting

Uses parsed `%mor` tier structure (`MorWord` features and post-clitics) rather than text splitting on spaces and delimiters. The semantic result is identical thanks to applying Brown's 7-suffix rule, but the mechanism is type-safe.

### No %mor tier behavior

When no `%mor` tier exists and `--words` was not passed, reports 0 utterances for the speaker (matching CLAN). Does not silently fall back to word counting.

### Output formats

Supports text, JSON, and CSV. CLAN produces text only.

### Golden test parity

100% parity with CLAN C binary output.
