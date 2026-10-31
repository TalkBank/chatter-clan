# Word Filtering

**Status:** Current
**Last updated:** 2026-06-15 12:31 EDT

Word filters restrict analysis to utterances containing (or not containing)
specific words. The generic content filter is `--include-word` /
`--exclude-word` (CLAN `+s` / `-s`), available across the search and frequency
family.

Note the per-command split: KWAL and COMBO each have their *own* primary
search, and for those commands CLAN's `+s` maps to that search, not to the
generic filter. KWAL's `+s` is the required keyword
([`--keyword`](../commands/kwal.md)); COMBO's `+s` is the required boolean
search expression ([`--search`](../commands/combo.md)). On those commands
`--include-word` is still available as an *additional* content filter on top of
the primary search.

## Include words

Only process utterances containing a specific word:

```bash
chatter-clan freq --include-word "the" file.cha
chatter-clan freq --include-word "dog" --include-word "cat" file.cha
```

CLAN equivalent (FREQ): `+s"the"`, `+s"dog" +s"cat"`

Multiple `--include-word` flags use OR logic by default: utterances containing
*any* listed word are included.

The same `--include-word` filter works on KWAL and COMBO in addition to their
required primary search, for example
`chatter-clan kwal --keyword "want" --include-word "the" file.cha`.

## Exclude words

Skip utterances containing specific words:

```bash
chatter-clan freq --exclude-word "um" --exclude-word "uh" file.cha
```

CLAN equivalent: `-s"um" -s"uh"`

## Case sensitivity

By default, word matching is case-insensitive (`the` matches `The`, `THE`, `the`).

The CLAN `+k` flag (`--plus-k` after rewriting) toggles each command's default case mode. FREQ and VOCD preserve case by default and fold under `+k`; KWAL, COMBO, FREQPOS, DIST, and MAXWD fold by default and preserve under `+k`. Other commands (MLU/MLT/WDLEN/WDSIZE/CHAINS/CODES) inherit `+k` from `cutt.cpp::mainusage` but it's a semantic no-op since they don't word-match. Per-command status lives in each command page under `clan-reference/commands/`. `--case-sensitive` remains a compatibility alias. See also [`flag-translation.md`](../getting-started/flag-translation.md).

## What counts as a "word"

Word matching uses the same countable-word logic as other commands:
- Regular words and proper nouns match
- Untranscribed markers (`xxx`, `yyy`, `www`) do not match
- Zero words (`0word`) do not match
- Fillers and fragments (`&-um`, `&~frag`) do not match
- Events (`&=laughs`) do not match
