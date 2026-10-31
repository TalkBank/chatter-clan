# CHIP — Child Language Interaction Profile

**Status:** implementation resumed; CLAN parity is not verified
**Last updated:** 2026-10-31

CHIP scores child/adult and self-repetition interactions over a rolling
utterance window. The implementation is in
`crates/talkbank-clan/src/commands/chip/`; its inputs and options are wired
through the `talkbank-clan` service and `chatter-clan` CLI.

## Options

| CLAN option | Meaning | Status |
|---|---|---|
| `+bS` | Assign speaker `S` the adult role | Implemented |
| `+cS` | Assign speaker `S` the child role | Implemented |
| `+g` | Include substitution-class scoring | Implemented |
| `+k` | Fold case for word-stem comparisons (default preserves case) | Implemented |
| `-n[b\|c\|s]` | Disable adult/child, child/adult, or self scoring | Implemented |
| `+qN` | Set rolling-window size (2–7) | Implemented |
| `+wN` | Set minimum source-utterance word count | Implemented |
| `+xN` | Set minimum repetition index | Implemented |
| `-t%mor` | Score words aligned with the `%mor` tier | Implemented |
| `-hFILE` | Exclude words listed in a file | Partial; exact and `*` wildcard patterns |
| `+dN` | Select CLAN display mode | Not implemented |

The audit rows below track option implementation separately from golden
evidence. CLAN's option definitions are in
`OSX-CLAN/src/clan/chip.cpp:94-103, 1340-1401`.

| CLAN flag | Meaning | Chatter | Status | Notes |
|---|---|---|---|---|
| `+bS` | Assign adult speaker(s) | `--adult CODE` | Done | No flag-specific golden yet |
| `+cS` | Assign child speaker(s) | `--child CODE` | Done | No flag-specific golden yet |
| `+g` | Enable substitution-class scoring | `--substitution` | Done | No flag-specific golden yet |
| `+k` | Fold case when comparing stems; default preserves case | `--plus-k` | Done | Command-specific polarity; no flag-specific golden yet |
| `-nb / -nc / -ns` | Disable adult, child, or self scoring | `--disable-scoring b/c/s` | Done | No flag-specific golden yet |
| `+qN` | Set utterance window (2–7) | `--window N` | Done | No flag-specific golden yet |
| `+wN` | Set minimum source word count | `--min-words N` | Done | No flag-specific golden yet |
| `+xN` | Set minimum repetition index | `--min-index N` | Done | No flag-specific golden yet |
| `-t%mor` | Use main-tier words instead of `%mor` | `--main-tier` | Done | No flag-specific golden yet |
| `-hFILE` | Exclude words matching patterns in a file | `--exclusion-list FILE` | Partial | Exact and `*` patterns match either paired surface or `%mor` form; unsupported selectors are rejected |
| `+dN` | Select display mode | — | Missing | No CHIP display-mode consumer |

Unassigned speakers are not presumed to be adults or children. Conflicting
speaker-role assignments, invalid window sizes, and incompatible scoring
options are rejected during request construction.

The default word source is `%mor`. Chatter v0.33.0 provides UD morphology,
not legacy CLAN morphology. CHIP uses typed morphology values and checks
main-tier/`%mor` alignment; a mismatch rejects the file rather than producing
fabricated scores. `-t%mor` selects main-tier words instead. Main-tier words
are treated as opaque lexical items, not parsed for morphology-like strings.
The default preserves stem case; `+k` folds only the typed comparison key.
Original stem spelling remains available for generated CHIP codes.
`-hFILE` currently supports exact words and `*` wildcards only. In `%mor`
mode it excludes a paired word when either its surface form or joined
morphology stem matches; substitution mode removes all clitics of that word.
Main-tier-only mode does not apply the merged-tier exclusion rule. Unsupported
selector syntax is rejected rather than approximated.

## Scoring and output

For each utterance, CHIP considers the nearest prior qualifying utterance from
the opposite speaker role and the nearest qualifying utterance by the same
speaker within the configured window. It does not limit comparisons to
adjacent utterances. Default mode reports the base measure matrix;
substitution mode adds substitution-class rows. Files with no scorable
utterances do not receive zero-filled matrices.

The transcript echo retains main tiers and `%mor` tiers, then appends the
generated CHIP tiers. Other dependent tiers are not echoed.

## Parity status

This implementation has focused unit tests, but no current CHIP-specific
golden covers the newly implemented options. The existing
`CHIP_MOR_GRA` golden covers only bare invocation
(`crates/talkbank-clan/tests/clan_golden/golden_parity.rs:876-884`), so it
does not establish option parity. In particular, `-hFILE` supports only the documented subset above and `+dN`
remains unsupported. Do not treat implementation coverage or unit tests as
evidence of full CLAN parity. See `book/src/divergences/per-command.md` for the
command-level status.
