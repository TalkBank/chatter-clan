# CLAUDE.md, talkbank-clan crate

**Status:** Current
**Last updated:** 2026-06-01 17:52 EDT

This crate implements the `chatter clan` command family: the byte-for-byte
re-implementation of the CLAN analysis tools (FREQ, MLU, KWAL, COMBO, ...).
The parity strategy (depth-first, one command at a time) is documented in the
repo-root [`CLAUDE.md`](../../CLAUDE.md) under "CLAN Parity Strategy"; this file
adds the rules specific to doing that work correctly.

## Study the manual. It is a PRIMARY authority, not optional.

**Before reasoning about what any CLAN command "should" do, read the relevant
section of the manual.** The manual is the authoritative statement of *intent*;
the CLAN source code is only the statement of *behavior*. You need both, and
they answer different questions:

- CLAN **source** (`OSX-CLAN/src/clan/<cmd>.cpp`, `cutt.cpp`, ...) tells you
  exactly WHAT the binary does, down to the byte. Cite it `file:line`.
- The **manual** tells you what is INTENDED, and why. Cite it by section.

### The CHAT manual and the CLAN manual are completely different documents

Use the one that matches the work, and never conflate them:

- **CLAN manual** (https://talkbank.org/0info/manuals/CLAN.html): the analysis
  programs and their flags (FREQ, MLU, KWAL, COMBO, ...). THIS is the source
  for `chatter clan` command and `+flag` parity, and for the
  analysis-command semantics such as MLU morphemicization (section 7.21).
- **CHAT manual** (https://talkbank.org/0info/manuals/CHAT.html): the
  transcript format itself (`@UTF8`, `*CHI:`, `%mor:`, markers). It is the
  authority the grammar, parser, and data model were built against. It is NOT
  the source for CLAN command flags.

The `clan-reference/commands/<cmd>.md` book pages already carry the specific
CLAN.html TOC anchor for their command. READ that section when you work the
command; do not jump from the source code straight to a fix.

Grounding parity work only in source code is a documented failure mode: you can
reproduce a behavior perfectly and still not know whether it is correct, a bug,
or an artifact of how a particular input was tagged. Without the manual you
cannot localize a divergence (next section), so you fall back on guessing or on
personal linguistic inference. Do not.

### Do NOT trust this repo's own docs as an authority

The pages under `book/src/clan-reference/` are written by this project. They are
a useful map, but they are NOT the authority on CLAN intent and may be wrong or
out of date. When a question is "what is correct," resolve it against the CLAN
or CHAT manual and the CLAN source, never against our own book pages.

## Localizing a divergence: judgment, not reflex

When chatter's output differs from the CLAN binary, the answer is NOT
automatically "make chatter match." Use judgment, grounded in the manual, to
localize the divergence into exactly one of:

1. **chatter bug** -> fix chatter. (Default when the CLAN binary agrees with the
   manual and chatter does not.)
2. **CLAN-binary bug** -> chatter diverges deliberately. Mark the golden
   `DivergesFromClan` and document the rationale with BOTH a manual citation and
   a CLAN-source `file:line`. (The mission is to do the correct thing, not to
   reproduce CLAN's bugs.)
3. **CLAN-manual mistake or outdated text** -> escalate to a maintainer rather
   than guess. The manual can be wrong too; deciding it is wrong is not a
   solo call.
4. **Data / `%mor` tagging issue** -> the divergence is an artifact of how the
   input was morphemicized. Raise it as a MOR/data matter; do not bake a
   workaround into the command.

A divergence is never resolved by personal linguistic inference presented as
fact. If the manual does not settle it, stop and ask.

## How to read the manual (WebFetch is banned)

WebFetch returns a small-model summary, not the page. To read the manual:

```bash
curl -sS -o CLAN.html https://talkbank.org/0info/manuals/CLAN.html
pandoc -f html -t plain CLAN.html -o CLAN.txt   # or: lynx -dump CLAN.html
grep -niE 'morpheme|<topic>' CLAN.txt           # locate the section, then read it
```

Cache the converted text in a scratch directory (not under the repo tree) so
you can grep and re-read it across a session. Quote the lines you rely on.

## Citation rule for parity work in this crate

Every claim about CLAN's correct behavior carries TWO citations:

- the CLAN/CHAT **manual section** that states the intent, and
- the CLAN **source `file:line`** that implements it.

Every claim about chatter's behavior carries a chatter `file:line`. Speculation
is banned unless explicitly tagged unverified with the exact citation that would
settle it.

## Worked example: MLU morpheme counting (2026-06-01)

The bare `mlu` golden on `tiers/mor-gra.cha` was RED: chatter undercounted
morphemes by one per utterance. Source alone (`countMorphs`,
`cutt.cpp:10741-10799`) showed chatter's `COUNTED_SUFFIXES`
(`src/commands/mlu/mod.rs`) listed only the legacy all-caps `PL`, missing the UD
`Plur`. The tempting move was to either match the binary blindly or to diverge
on the inference that "a suppletive pronoun plural (them) should be one
morpheme." The manual settled it (CLAN.html section 7.21, point 6): the traced
morphemes for English are "Plural, Past, Possessive, Plural-Possessive, Present
Participle, clitic negative, and clitic auxiliary," and the manual explicitly
endorses UD `%mor` for MLU. In UD there is no `&`-fusion marker to exempt a
pronoun's `-Plur`, and the manual gives no POS carve-out, so `them` counting as
two is correct per the manual. Localization: chatter bug (missing UD feature
name), CLAN binary correct, no divergence. The fix was to trace the UD `Plur`
feature. Without the manual this would have been mis-localized.

## Inapplicable flags must ERROR, never silently no-op

When a CLAN flag does not apply to a command, `chatter clan <cmd>` must reject
it with a hard error, NOT silently accept-and-ignore it and NOT reproduce
whatever artifact CLAN happens to produce. This follows the fail-closed /
no-silent-defaults standard: a flag the command cannot honour is a user error
the user needs to see, not something to swallow.

Worked example (2026-06-01): `+wN` / `-wN` is a keyword-context window
documented only for KWAL/COMBO. It is inapplicable to the aggregate commands
(FREQ, MLU, MLT, WDLEN, MAXWD, FREQPOS), and CLAN's binary, when given `+w` on
FREQ, prints EMPTY output (a side-effect of the shared context machinery, not a
no-op). chatter previously accepted `+w` on these six commands and silently
ignored it via an `InheritedContextArgs` flatten, on the mistaken belief that
"CLAN no-ops it for parity." Both behaviours are wrong: the correct behaviour
is to reject `+w` on a command that cannot use it. Do not add accept-and-ignore
flattens for flags a command does not implement; let clap reject the rewritten
flag so the user gets an error.

The `+w`-on-aggregate fix landed in commit 3bc39db: the `InheritedContextArgs`
accept-and-ignore was removed from the six aggregate commands, so `+w`/`-w` now
error at parse time, pinned by the `chatter clan freq +w2` subprocess test.

## Spreadsheet / Excel output: semantic equivalence, not byte equality

A few CLAN commands emit a spreadsheet FILE rather than text to stdout (FREQ
`+d2`/`+d3` write `stat.frq.cex` Excel-XML SpreadsheetML; WDLEN appends a
SpreadsheetML footer). For these, the parity standard is **semantic equivalence
of the spreadsheet data**, NOT byte-for-byte identity with CLAN's exact XML
serialization. CLAN's `.cex` carries version strings, attribute ordering, and
styling boilerplate that are not semantically meaningful; chatter produces its
own well-formed SpreadsheetML with the same sheet name, headers, rows, and cell
values. The test compares the PARSED spreadsheet (cells), not the raw XML.

This is the one documented exception to the byte-for-byte rule, scoped to
spreadsheet output only; all text / CLAN-format output stays byte-for-byte.
