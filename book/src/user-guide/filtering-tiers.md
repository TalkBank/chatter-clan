# Tier Filtering

**Status:** Current
**Last updated:** 2026-06-15 12:54 EDT

Tier filters select which **dependent tiers** a command operates on, using
CLAN's `+t%`/`-t%` flag family. In legacy CLAN this is a shared option (the
`+t`/`-t` handler in `cutt.cpp`; `+t%X` includes a dependent tier, `-t%X`
excludes one), so a single mechanism covers two command styles:

- **Cluster-echo commands** (KWAL, COMBO, GEM): `+t%X` adds dependent tier `X`
  to each echoed cluster; `-t%X` removes it. CLAN's manual documents this
  directly for KWAL: the alias `kwal +t* +t@ +t% -t%mor +d +f *.cha` echoes
  every cluster but drops the `%mor` line (CLAN manual, "Using aliases").
- **Analysis commands** (FREQ, CHAINS): `+t%X` makes the analysis read tier `X`
  instead of the main tier (e.g. `freq +t%mor` counts morphemes).

CLAN semantics, source-grounded: the `+t`/`-t` handler routes `%`-prefixed
arguments to `maketierchoice(arg, sign, …)`
(`OSX-CLAN/src/clan/cutt.cpp:9749-9758`), and the run banner reports
`ONLY dependent tiers matching: …` for `+t%` and
`ALL dependent tiers EXCEPT the ones matching: …` for `-t%`
(`cutt.cpp:12173`, `cutt.cpp:12199`).

## chatter coverage

`--tier` / `--exclude-tier` (the chatter spelling of `+t%` / `-t%`, also
reachable via the legacy `+t%`/`-t%` forms through the flag rewriter) are
currently implemented on **FREQ** and **CHAINS**. The cluster-echo commands
**KWAL** and **COMBO** accept the legacy rewrite target but do **not** yet
expose the flag (it is "Rewriter only", see the
[KWAL command page](../commands/kwal.md) audit table),
so dependent-tier display selection on KWAL/COMBO is a known parity gap rather
than a finished feature. The runnable examples below therefore use FREQ.

## Include a dependent tier

Make the analysis read only the named dependent tier:

```bash
chatter clan freq --tier mor file.cha
chatter clan freq +t%mor file.cha
```

CLAN equivalent: `+t%mor`. On FREQ this counts items on the `%mor` tier; on a
cluster-echo command it would show only the `%mor` tier with each match.

## Exclude a dependent tier

Drop a dependent tier:

```bash
chatter clan freq --exclude-tier gra file.cha
chatter clan freq -t%gra file.cha
```

CLAN equivalent: `-t%gra`.

## Common dependent tiers

| Tier | Full name | Content |
|------|-----------|---------|
| `mor` | Morphology | POS tags and lemmas: `noun\|dog-PL` |
| `gra` | Grammar | Dependency relations: `1\|3\|NSUBJ` |
| `pho` | Phonology | Phonological transcription |
| `flo` | Fluent output | Simplified main-tier text |
| `cod` | Codes | Researcher-assigned codes |
| `mod` | Model | Target pronunciation |
| `ret` | Retrace | Copy of main tier |

## Notes

- Tier names are specified without the `%` prefix on the `--tier` /
  `--exclude-tier` form (`mor`, not `%mor`); the legacy `+t%mor` form keeps the
  `%`.
- Selecting a tier for analysis (FREQ/CHAINS) changes what is counted;
  selecting a tier for display (KWAL/COMBO, once landed) changes what is echoed.
- Speaker selection (`*CHI:`, `*MOT:`) uses the same `+t` flag with a `*`
  argument; see [Speaker Filtering](filtering-speakers.md).
