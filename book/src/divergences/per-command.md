# Per-Command Divergences

**Status:** Current
**Last updated:** 2026-06-04 19:08 EDT

This page documents every known divergence between the Rust `chatter clan` commands and the original CLAN C binaries. Divergences are verified through golden tests that compare output character-by-character.

**9 accepted CLAN-bug divergences across 3 commands (DELIM, UNIQ, FREQ); each
is a deliberate, source-and-manual-grounded out-correction, not a parity gap.**

## Parity Summary

| Status | Commands |
|--------|----------|
| **100% parity** | MLU, MLT, VOCD, CHIP, DIST, MAXWD, TIMEDUR, WDLEN |
| **Verified** | DSS, EVAL, KIDEVAL, IPSYN, FLUCALC, SUGAR, CHAINS, CODES, COMBO, COOCCUR, FREQPOS, GEMLIST, KEYMAP, MODREP, MORTABLE, PHONFREQ, RELY, SCRIPT, TRNFIX, CHSTRING, FLO, POSTMORTEM |
| **Accepted divergences** | DELIM (4), UNIQ (1), FREQ (4) |

"Verified" means golden tests pass but character-level parity has not been exhaustively confirmed across all edge cases.

---

## Analysis Commands

### FREQ -- Word Frequency

- AST-based `is_countable_word()` replaces string-prefix matching
- The [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) explicitly
  says `FREQ` ignores `xxx`, `www`, and items beginning with `0`, `&`, `+`,
  `-`, or `#` by default; `talkbank-clan` preserves that intent through typed
  word classes rather than raw prefix checks.
- `NormalizedWord` lowercases and strips compound markers for grouping
- Deterministic sort (count descending, then alphabetical); CLAN's order varies
- JSON and CSV output; `--format clan` for character-level CLAN compatibility
- **Parity: 4 accepted CLAN-bug divergences with formal ledger rows (`+c1`;
  `+d2`/`+d3`/`+dCN`; `+xxxx`/`+xyyy`/`+xwww`; `+r7`), see CLAN-DIV-003, CLAN-DIV-004,
  CLAN-DIV-005, and CLAN-DIV-006 below. The `+x…m` morpheme-unit divergences
  (over-count + output-doubling) are documented in the `+x` row of
  `commands/freq.md` and are pending their own ledger rows.**

#### CLAN-DIV-003: FREQ `+c1` middle-only

`+c1` is documented as "find words with upper case letters in the middle only"
(CLAN manual section 5483; `freq.cpp:167`). CLAN's implementation (`freq.cpp`
`isRightUpper`, the `capwd == 2` branch, which `+c1` selects via `freq.cpp:782`)
loops from character position 0 and returns a match on any uppercase letter,
including the first, so it also keeps initial-capital words. Empirically
`freq +c`, `freq +c0`, and `freq +c1` produce identical output; `+c1` does not
filter to "middle" at all.

- **CLAN behaviour:** `+c1` keeps any word with an uppercase letter anywhere,
  including position 0 (e.g. `Triangle`), effectively identical to `+c`.
- **chatter behaviour:** `+c1` keeps only words with an uppercase letter after
  position 0 (e.g. `McDonald`, `iPhone`); initial-only caps like `Triangle`
  are dropped.
- **Reason chatter is correct:** the manual's "middle only" wording, plus the
  separate `+c` ("capitalised words only"), make the intent unambiguous, `+c1`
  must exclude position 0. CLAN's loop never advances past index 0 before its
  first `isupper` test, so `+c1` collapses into `+c`; reproducing the bug would
  make `+c1` a useless duplicate of `+c`.
- **Ledger row:** CLAN-DIV-003 (FREQ `+c1` middle-only).
- **Pinned by:** the `freq_c1_mid_capitalization_eng` golden
  (`ParityExpectation::DivergesFromClan`); the committed golden holds chatter's
  corrected output, and the regen step asserts it differs from CLAN's.

#### CLAN-DIV-004: FREQ +d2 +d3 mor caveat

FREQ `+d2` / `+d3` write an aggregate SpreadsheetML file (`stat.frq.xls` /
`stat.frq0.xls`), one row per (file x speaker) keyed by `@ID`. The same caveat
rows are emitted by the `+dCN` percent-of-speakers filter (`onlydata = 4`,
`words.frq.xls`, the `+d3`-shaped summary over a percent-filtered word subset),
so this divergence covers `+dCN` identically. When the run is
on the main tier (not `%mor`), CLAN prepends three red advisory rows about the
type-token ratio. CLAN's spreadsheet writes those rows with a literal `%%mor`
(its `excelRowOneStrCell` emits the C string `"...%%mor..."` verbatim, without
the printf un-doubling its stdout path performs), so the cell text reads
`...%%mor line forms.` / `run FREQ on the %%mor line`.

- **CLAN behaviour:** the spreadsheet caveat rows contain `%%mor` (a printf
  `%%`-escaping leak; CLAN's own *text* output correctly shows `%mor`).
- **chatter behaviour:** emits the correct single-percent `%mor` in those rows.
- **Reason chatter is correct:** `%%mor` is unambiguously a doubled-percent bug;
  no `%mor` tier is named `%%mor`. Every other cell, the `@ID` columns, the
  `*SPEAKER:` pseudo-word counts, the per-word columns, and Types/Token/TTR, is
  reproduced byte-for-byte; this is the *only* divergence (verified by a direct
  diff of chatter's `+d2` output against CLAN's `stat.frq.xls`).
- **Ledger row:** CLAN-DIV-004 (FREQ `+d2`/`+d3`/`+dCN` `%mor` caveat).
- **Pinned by:** the `freq_d2_spreadsheet_manchester`,
  `freq_d3_spreadsheet_manchester`, and `freq_d_percent_lt_eq_manchester`
  goldens (`ParityExpectation::DivergesFromClan`); the committed golden holds
  chatter's corrected SpreadsheetML. Spreadsheet parity is judged on semantic
  cell equivalence (`talkbank-clan/CLAUDE.md`), not byte-identical XML. The
  `+dCN` data cells (the `@ID` columns, the `Speaker` header, and the
  percent-filtered Types/Token/TTR) were verified byte-identical to CLAN's
  `words.frq.xls` on a live run, the same as `+d2`/`+d3`.

#### CLAN-DIV-005: FREQ `+xxxx`/`+xyyy`/`+xwww` byte-stride restore bug

The `+x C N U` utterance-length filter strips the unintelligible markers
`xxx`/`yyy`/`www` from the length count by default; the content-include flags
`+xxxx`/`+xyyy`/`+xwww` are documented (manual section 6405) to *restore* a
marker into that count. CLAN applies the restore in `correctForXXXYYYWWW`
(`cutt.cpp:16260-16311`), which walks the cleaned line and, per loop iteration,
advances its index by `i += 2` three times (once each for the xxx, yyy, www
checks) on top of the `for` loop's own `i++` -- a net stride of 7. It therefore
only ever inspects `xxx` at byte offsets divisible by 7 (and `yyy` at offsets
≡ 2, `www` at ≡ 4). Whether a marker is restored thus depends on its byte
position in the line, with no basis in the manual.

- **CLAN behaviour:** `+xxxx` restores `xxx` into the `+x` length count only
  when the marker's byte offset in the cleaned line is ≡ 0 (mod 7); otherwise
  the marker is silently left out of the count. Verified by a 7-utterance probe
  (`a xxx end` … `abcdefg xxx end`): only the offset-7 case restored.
- **chatter behaviour:** restores the marker UNCONDITIONALLY, through the shared
  AST walker `framework::words_for_utterance_length` (so restored markers obey
  the same group-recursion and retrace/replacement rules as countable words).
- **Reason chatter is correct:** the manual states the restore without any
  byte-position qualifier; CLAN's stride is an off-by-`i+=2` C bug, not an
  intended semantic. On `1082.cha` `+x=4w +xxxx`, chatter correctly restores the
  `xxx` inside `<we xxx bein(g) in Rochester> [?]` (4 -> 5 words, so the
  utterance leaves the exactly-4 filter), giving `*CHI` 94 tokens; CLAN's bug
  leaves that `xxx` un-restored and keeps the utterance (`*CHI` 98).
- **Ledger row:** CLAN-DIV-005 (FREQ `+xxxx`/`+xyyy`/`+xwww` byte-stride restore).
- **Pinned by:** `freq_x_restore_xxx_into_length_count` (restore works; `*CHI`
  94, not CLAN's 98) and `freq_x_restore_xxx_diverges_from_clan_byte_stride_bug`
  (the `<we xxx …>` utterance is correctly dropped, so `we` is absent), with the
  default-strip control `freq_x_default_strips_xxx_from_length_count`.

#### CLAN-DIV-006: FREQ `+r7` CA/satellite-delimiter retention (invalid UTF-8)

`+r7` is documented (manual section 14.5, line 12418) as "Do not remove
prosodic symbols (`/~^:`) in words", a within-word scope: it keeps the
lengthening `:`, blocking `^`, clitic `~`, and slash `/` so `ca:t` stays
distinct from `cat`. CLAN's implementation reaches far past that scope. The
`+r7` parser (`cutt.cpp:9569-9576`) clears all four `R7*` flags, which flips the
per-word cleanup gate at `cutt.cpp:7258` from the default `HandleSlash`
(`cutt.cpp:7264`) to `HandleSpCAs` (`cutt.cpp:7273`); it also clears
`isRemoveCAChar[NOTCA_CROSSED_EQUAL]` and `isRemoveCAChar[NOTCA_LEFT_ARROW_CIRCLE]`
(`cutt.cpp:9573-9574`), two CA classes the manual never mentions. Empirically
that `+r7` cleanup branch retains the entire CA / satellite-delimiter family as
standalone counted "word" tokens, where the default run strips them: ‡
(`NOTCA_VOCATIVE`, U+2021, `check.h:83`, CLAN's own comment "Vocative or summons
- NOT CA"), „ (`NOTCA_DOUBLE_COMMA`, U+201E), ≠ (`NOTCA_CROSSED_EQUAL`, U+2260),
and ↫ (`NOTCA_LEFT_ARROW_CIRCLE`, U+21AB). None is a within-word prosodic
symbol. Worse, CLAN's legacy 8-bit output path mangles the two delimiters whose
UTF-8 starts `E2 80` (‡, „): it drops the `0xE2` lead byte and emits the
invalid-UTF-8 byte pairs `0x80 0xA1` (‡) and `0x80 0x9E` („) as the "word type".

- **CLAN behaviour:** under `+r7`, the CA / satellite delimiters survive into the
  frequency table as standalone tokens; the `E2 80` ones are emitted as invalid
  UTF-8 byte garbage. Verified by single-utterance probes (each of ‡, „, ≠, ↫
  flips a tier from 2 to 3 types under `+r7`, but not under `+r4` or the
  default) and on `1082.cha`, where `+r7` adds 12 (`*INV1`) + 3 (`*CHI`) corrupt
  ‡ tokens that the default run omits.
- **chatter behaviour:** the `--prosody-mode keep` (`+r7`) axis keeps only the
  modelled within-word prosodic elements (Lengthening `:`, SyllablePause `^`,
  CliticBoundary `~`). A satellite / tag delimiter is not word content in the
  typed AST, so it never enters the frequency map, and chatter never emits
  invalid UTF-8.
- **Reason chatter is correct:** the manual scopes `+r7` to within-word `/~^:`;
  the satellite delimiters are between-word markers CLAN's own source labels
  "NOT CA", so retaining them is outside the documented intent. Emitting a
  frequency "word type" that is not even valid UTF-8 is an unambiguous defect of
  CLAN's 8-bit pipeline, not a behaviour any reading of the manual intends.
- **Ledger row:** CLAN-DIV-006 (FREQ `+r7` CA/satellite-delimiter retention).
- **Pinned by:** `freq_r7_does_not_retain_corrupt_ca_delimiters` (chatter `+r7`
  on `1082.cha` is valid UTF-8, emits no `0x80 0xA1`, and never counts ‡),
  alongside `freq_r7_keeps_within_word_lengthening` /
  `freq_default_strips_where_r7_keeps` which pin the documented `/~^:` keep.

### MLU -- Mean Length of Utterance

- **Population SD** (/ n), not sample (/ n-1). Verified against CLAN output.
- **Brown's morpheme rules**: Only 7 suffix strings count: `PL`, `PAST`, `Past`, `POSS`, `PASTP`, `Pastp`, `PRESP`. Each adds +1 to the stem count. Fusional features (`&PRES`, `&INF`) do NOT count.
- When no `%mor` tier exists and not in `--words-only` mode, reports 0 utterances (matching CLAN).
- **Parity: 100%**

### MLT -- Mean Length of Turn

- **Population SD** (/ n), matching CLAN.
- **SD basis**: Computed over per-utterance word counts, not per-turn totals.
- Turn boundaries detected when a different speaker produces the next utterance.
- **Parity: 100%**

### DSS -- Developmental Sentence Scoring

- Built-in rules are a simplified subset; supply full `.scr` file for clinical scoring.
- Sentence-point assignment uses heuristic (subject + verb POS) rather than full syntax.
- Up to 50 utterances per speaker scored (configurable via `max_utterances`).
- **Parity: Verified**

### EVAL -- Language Sample Evaluation

- AST-based word/morpheme identification and typed POS categories.
- Error counts (`[*]`) extracted from parsed AST annotations, not text patterns.
- **Parity: Verified**

### KIDEVAL -- Child Language Evaluation

- Same AST-based approach as EVAL with combined metrics.
- VOCD uses simplified TTR-based D estimate in the combined report.
- **Parity: Verified**

### IPSYN -- Index of Productive Syntax

- Parses %mor tier structure for syntactic pattern matching.
- Built-in rule set is a simplified subset; supply rules file for full coverage.
- **Parity: Verified**

### VOCD -- Vocabulary Diversity (D Statistic)

- Bootstrap sampling of TTR across sample sizes 35-50, least-squares D-curve fitting.
- **Fusional feature stripping**: `&PRES`, `&INF` etc. stripped from lemmas in %mor echo output.
- D values may differ slightly due to random sampling (stochastic algorithm).
- **Parity: 100%** (within expected stochastic variation)

### FLUCALC -- Fluency Calculation

- Counts disfluency types from main tier annotations.
- Some categories detected via text pattern matching rather than full AST traversal.
- **Parity: Verified**

### SUGAR -- Grammatical Analysis

- SUGAR scoring from %mor tier POS categories.
- `%mor` post-clitics count as structured morphology, so clitic-bearing chunks contribute to morpheme totals and verb detection.
- Minimum utterance threshold is configurable (CLAN uses fixed value).
- **Parity: Verified**

### CHAINS -- Clause Chain Analysis

- `CHAINS` consumes a clan-local semantic `%cod` item layer, so selectors like `<w4>` scope codes instead of being treated as codes themselves.
- Uses sample SD (N-1), not population SD.
- **Parity: Verified**

### CHIP -- Child/Parent Interaction Profile

- **36-measure matrix format** matching CLAN exactly (ADU/CHI/ASR/CSR columns).
- **Echo**: Main tier + %mor only (not %gra tiers), matching CLAN.
- **Parity: 100%**

### CODES -- Code Frequency

- Codes extracted from parsed `%cod` tier, not raw text.
- `%cod` is interpreted through a clan-local semantic item layer derived from the AST. Optional selectors like `<w4>` or `<w4-5>` scope the next code item instead of being counted as codes themselves.
- **Parity: Verified**

### COMBO -- Boolean Search

- AST-based content matching rather than raw text pattern matching.
- Operator syntax: `+` for AND, `,` for OR (CLAN uses `^` and `|`).
- **Parity: Verified**

### COOCCUR -- Word Co-occurrence

- Bigram counting from countable words per utterance.
- **Parity: Verified**

### DIST -- Word Distribution

- **Every utterance is its own turn** (no speaker-continuity grouping), matching CLAN.
- **Parity: 100%**

### FREQPOS -- Positional Frequency

- Word frequency by utterance position (initial, final, other, one-word).
- **Parity: Verified**

### GEMLIST -- Gem Segments

- Lists `@Bg`/`@Eg` gem boundaries from file headers.
- **Parity: Verified**

### KEYMAP -- Contingency Tables

- Reads coded data from `%cod` tier, builds contingency matrix.
- `%cod` selector tokens are treated as item scope rather than keyword/following-code values; `KEYMAP` consumes semantic `%cod` items.
- **Parity: Verified**

### MAXWD -- Longest Words

- Reports **all occurrences with line numbers**, matching CLAN.
- **Parity: 100%**

### MODREP -- Model/Replica Comparison

- Compares `%mod` and `%pho` tiers phonologically via AST.
- **Parity: Verified**

### MORTABLE -- Morphology Tables

- Tabulates POS categories from %mor tier using script files.
- POS extraction reads typed `%mor` items directly instead of reparsing serialized `%mor` payload text.
- **Parity: Verified**

### KWAL -- Keyword and Line

- The
  [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) does document
  `KWAL`, including cluster-based search semantics.
- **Scope reduction**: The legacy manual describes richer cluster/tier selection and `%mor`/`%gra` combined searches than the current implementation exposes.
- **Parity: Verified**

### PHONFREQ -- Phonological Frequency

- Frequency counts from `%pho` tier.
- **Parity: Verified**

### RELY -- Inter-rater Agreement

- Cohen's kappa for inter-rater reliability.
- `%cod` comparison uses semantic code items from the AST-derived layer rather than whitespace tokens from flattened tier text.
- **Parity: Verified**

### SCRIPT -- Template Comparison

- Word matching uses frequency maps rather than positional alignment.
- **Parity: Verified**

### TIMEDUR -- Time Duration

- **Interaction matrix header** includes leading space, matching CLAN exactly.
- **Parity: 100%**

### TRNFIX -- Tier Comparison

- Uses `∅` for length mismatches between compared tiers.
- `%trn` is treated as a structural alias of `%mor`; `%grt` is treated as a structural alias
  of `%gra`.
- `%mor`/`%gra` comparison preserves typed token boundaries directly from the AST instead of comparing whitespace-split serialized payloads.
- **Parity: Verified**

### UNIQ -- Repeated Utterances

- **Includes %mor/%gra dependent tiers** in counts, matching CLAN.
- **Splits multi-line headers** for counting, matching CLAN.
- **1 accepted divergence**: Unicode sort order for `U+230A` (LEFT FLOOR) -- C-locale `strcoll()` vs Rust byte-order. Single line position swap, identical content and counts.
- **Parity: 99%**

### WDLEN -- Word Length Distribution

- **6-section format** matching CLAN exactly.
- **Brown's morpheme rules**: Section 5 = stem + Brown's suffix (no POS). Section 6 = POS + stem + Brown's suffix.
- **Clitic handling**: Section 5 merges main+clitics as one word. Section 6 counts POS only for main word.
- **Apostrophe stripping**: Characters counted after removing apostrophes.
- **Reverse speaker order**: CLAN's linked-list prepend pattern replicated.
- **XML footer**: `</Table></Worksheet></Workbook>` appended.
- **Parity: 100%**

---

## Transform Commands

All transforms use the AST pipeline: parse -> transform -> serialize -> write.

### DELIM -- Add Terminators

- **4 accepted divergences**: CLAN writes empty file when no changes needed; we always write the full file.
- **Parity: 4 accepted divergences**

### CHSTRING -- String Replacement

- Does not support CLAN's regex-based patterns in changes files.
- **Parity: Verified**

### FLO -- Fluent Output

- Walks AST nodes instead of regex-stripping annotation markers.
- **Parity: Verified**

### POSTMORTEM -- Mor Post-processing

- Typed `%mor` rewrites are intentionally rejected until an AST-native rewrite path exists. `POSTMORTEM` errors explicitly when a rule would modify parsed `%mor`.
- **Current status:** user-defined text tiers remain supported rewrite targets; typed `%mor` rewrite is intentionally unsupported until implemented through the AST.

### Other transforms

- `COMBTIER` preserves bullet/text tier variants such as `%cod` and `%com` instead of degrading them to user-defined tiers.
- `FIXBULLETS` supports global offsets and tier-scoped repair across parsed main-tier, `%wor`, and bullet-content-tier bullets.
- Bullet-bearing `@Comment` headers are parsed structurally, so `FIXBULLETS` can offset those header bullets through the AST as well.
- **Scope reduction remains:** old-format bullet conversion, `@Media` insertion, multi-bullet merge, and `+l` remain unsupported.
- **Scope reduction:** `TIERORDER` currently uses a built-in tier ordering,
  while the
  [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) documents
  `tierorder.cut` as a user-controlled ordering source.
- `TRIM` follows the
  [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) intent by
  removing selected dependent tiers instead of extracting utterance ranges or
  gem segments.
COMBTIER, COMPOUND, DATACLEAN, DATES, FIXBULLETS, LINES, LOWCASE, MAKEMOD, ORT, QUOTES, REPEAT, RETRACE, TIERORDER -- all operate on AST rather than raw text, except for the intentionally text-level formatting transforms (`DATACLEAN`, `LINES`) and layout transforms (`INDENT`, `LONGTIER`) discussed in the dependent-tier semantics audit.

---

## Key Discoveries

These findings were established during parity verification (golden tests comparing against CLAN C binaries):

1. **Brown's Morpheme Rules**: CLAN MLU/WDLEN count only 7 suffix strings as bound morphemes.
2. **Population SD**: Both MLU and MLT use population SD (/ n), not sample (/ n-1).
3. **MLT SD basis**: Computed over per-utterance word counts, not per-turn totals.
4. **DIST turns**: Every utterance is its own turn (no speaker-continuity grouping).
5. **Speaker ordering**: CLAN outputs speakers in reverse encounter order (linked-list prepend).
6. **Fusional features**: `&PRES`, `&INF` etc. are part of the lemma string; strip with `split('&')`.
7. **CHIP echo**: CLAN echoes main tiers + %mor only, not %gra tiers.
8. **WDLEN characters**: CLAN strips apostrophes before counting character length.
