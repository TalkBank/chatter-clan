# Command Status Matrix

**Status:** Current
**Last updated:** 2026-06-15 12:23 EDT

Status of all CLAN commands in the Rust reimplementation.

> **Parity authority.** This matrix is a *coverage* cut: which CLAN commands
> are implemented and which have golden tests. It is **not** a byte-for-byte
> parity claim. Byte-for-byte CLAN parity is tracked authoritatively in
> [Parity Status](../parity-status.md), and per that page **no command yet
> achieves byte-for-byte CLAN parity**. FREQ is the active depth-first command
> (25/44 flag-rows byte-parity proven as of the last sweep). A row below that
> reads "Implemented" with "Golden Tests: Yes" means the command exists and has
> golden coverage; it does **not** mean its output byte-matches legacy CLAN.
> Consult [Parity Status](../parity-status.md) for the honest per-command
> signal.

**Coverage: 70/70 CLAN binaries** (100%). Every CLAN binary in `OSX-CLAN/src/unix/bin/` has a corresponding module. The 6 NLP commands (MOR, POST, MEGRASP, etc.) are deliberately not implemented but produce clear error messages. Additionally, 7 implemented commands are outside that local binary inventory: COMPLEXITY, CORELEX, WDSIZE, LAB2CHAT, RTF2CHAT, ROLES, and TRIM.

| Category | Count | Golden Tests |
|----------|-------|-------------|
| Validation | 1 |, |
| Analysis | 34 | 53 |
| Transforms | 23 | 30 |
| Converters | 15 | 15 |
| Not implemented | 6 |, |
| **Total** | **79** | **98** |

## Validation Commands (1)

| Command | Status | Notes |
|---------|--------|-------|
| CHECK | Implemented | Full flag support (+cN, +eN, +gN, +u), 161 error numbers mapped |

## Analysis Commands (34)

The "Parity" column was removed: its per-command "Verified" / "100%" values
asserted byte-for-byte parity that [Parity Status](../parity-status.md) does
not support for any command. This table now records only the verifiable
coverage facts (implemented + golden tests present). For the honest byte-parity
signal, see [Parity Status](../parity-status.md).

| Command | Status | Golden Tests |
|---------|--------|-------------|
| CHAINS | Implemented | Yes |
| COMPLEXITY | Implemented | Yes (new; auto-detects UD/legacy) |
| CORELEX | Implemented | Yes (new command, not in CLAN) |
| CHIP | Implemented | Yes |
| CODES | Implemented | Yes |
| COMBO | Implemented | Yes |
| COOCCUR | Implemented | Yes |
| DIST | Implemented | Yes |
| DSS | Implemented | Yes |
| EVAL | Implemented | Yes |
| EVAL-D | Implemented | Variant of EVAL (DementiaBank norms) |
| FLUCALC | Implemented | Yes |
| FREQ | Implemented | Yes (active depth-first parity target; see [Parity Status](../parity-status.md)) |
| FREQPOS | Implemented | Yes |
| GEMLIST | Implemented | Yes |
| IPSYN | Implemented | Yes |
| KEYMAP | Implemented | Yes |
| KIDEVAL | Implemented | Yes |
| KWAL | Implemented | Yes |
| MAXWD | Implemented | Yes |
| MODREP | Implemented | Yes |
| MLT | Implemented | Yes |
| MLU | Implemented | Yes |
| MORTABLE | Implemented | Yes |
| PHONFREQ | Implemented | Yes |
| RELY | Implemented | Yes |
| SCRIPT | Implemented | Yes |
| SUGAR | Implemented | Yes |
| TIMEDUR | Implemented | Yes |
| TRNFIX | Implemented | Yes |
| UNIQ | Implemented | Yes |
| VOCD | Implemented | Yes |
| WDLEN | Implemented | Yes |
| WDSIZE | Implemented | Yes (new; mor stem lengths) |

## Transform Commands (23)

| Command | Status | Golden Tests |
|---------|--------|-------------|
| CHSTRING | Implemented | Yes |
| COMBTIER | Implemented | Yes |
| COMPOUND | Implemented | Yes |
| DATACLEAN | Implemented | Yes |
| DATES | Implemented | Yes |
| DELIM | Implemented | Yes (4 accepted divergences) |
| FIXIT | Implemented | Yes |
| FIXBULLETS | Implemented | Yes |
| FLO | Implemented | Yes |
| GEM | Implemented | Yes (2 tests: all gems + filtered) |
| INDENT | Implemented | Yes (CLAN binary has infinite-loop bug, Rust-only) |
| LINES | Implemented | Yes |
| LONGTIER | Implemented | Yes |
| LOWCASE | Implemented | Yes |
| MAKEMOD | Implemented | Yes |
| ORT | Implemented | Yes |
| POSTMORTEM | Implemented | Yes |
| QUOTES | Implemented | Yes |
| REPEAT | Implemented | Yes |
| RETRACE | Implemented | Yes |
| ROLES | Implemented | Yes |
| TIERORDER | Implemented | Yes |
| TRIM | Implemented | Yes (2 tests: exclude-mor + exclude-all) |

## Format Converters (15)

| Command | Status | Notes |
|---------|--------|-------|
| CHAT2TEXT | Implemented | Plain text export |
| CHAT2ELAN | Implemented | Reverse of ELAN2CHAT |
| CHAT2PRAAT | Implemented | Praat TextGrid export (bidirectional in praat2chat module) |
| CHAT2SRT | Implemented | SRT subtitle export |
| CHAT2VTT | Implemented | WebVTT subtitle export (Rust-side extension of the SRT converter; not in CLAN) |
| ELAN2CHAT | Implemented | ELAN XML import |
| LAB2CHAT | Implemented | LAB format import |
| LENA2CHAT | Implemented | LENA ITS import |
| LIPP2CHAT | Implemented | LIPP format import |
| PLAY2CHAT | Implemented | PLAY format import |
| PRAAT2CHAT | Implemented | Praat TextGrid import |
| RTF2CHAT | Implemented | Rich Text import |
| SALT2CHAT | Implemented | SALT format import |
| SRT2CHAT | Implemented | Subtitle import |
| TEXT2CHAT | Implemented | Plain text import |

## Deliberately Not Implemented (6)

These commands depend on the legacy CLAN MOR data model (trie-based lexicons, HMM/Brill taggers, MaxEnt parsers) which is incompatible with the UD-style morphological representation used in the current CHAT grammar. Use batchalign's neural pipeline instead.

| Command | Purpose | Rationale |
|---------|---------|-----------|
| MOR / MOR_P | Morphological analysis | ~11K lines C, trie lexicon + 5 rule engines, legacy format |
| POST | POS disambiguation | Requires ^-separated ambiguity format not in grammar |
| MEGRASP | Dependency parsing | Requires trained MaxEnt model weights |
| POSTLIST | POST database listing | Operates on proprietary binary format |
| POSTMODRULES | POST rule modification | Operates on proprietary binary format |
| POSTTRAIN | POST model training | Operates on proprietary binary format |

## Compatibility-Alias Subcommands

These CLAN binaries have their own `clap` subcommand variant, exposed
under the same name as the legacy CLAN binary, but are categorized
as compatibility aliases in
`crates/talkbank-cli/src/cli/args/clan_commands.rs` (search for
`CompatibilityAlias`). The dedicated subcommand exists so legacy
CLAN scripts that invoke the binary by name continue to work.

Five subcommand names are bucketed as `CompatibilityAlias`: `check`,
`fixit`, `indent`, `longtier`, and `gemfreq`. Only `gemfreq` is
documented in detail here because its dispatch target differs from
its name; the other four (`check`, `fixit`, `indent`, `longtier`)
are listed in the main category tables above and dispatch to their
namesakes through the standard pipeline.

| Subcommand | Dispatches to | Notes |
|---|---|---|
| `gemfreq` | `freq` (with `--gem` required) | See [GEMFREQ](../commands/gemfreq.md). Implemented via `Gemfreq` enum variant + clap `ArgGroup::required(true)` on `--gem`; routes to `AnalysisCommandName::Freq`. |
