# CLAN Command Reference

**Status:** Current
**Last modified:** 2026-06-13 16:25 EDT

> **`chatter clan` is EXPERIMENTAL and in active development. No single
> CLAN command yet achieves byte-for-byte CLAN parity.** If you are
> migrating from legacy CLAN and need bit-exact output today, you must
> keep running legacy CLAN. See **[Parity Status](parity-status.md)** for
> the honest current snapshot, the active command (FREQ), and how "done"
> is defined. The `chatter clan --help` output carries the same
> `[EXPERIMENTAL]` label.

**CLAN** (Computerized Language Analysis) is a suite of commands for analyzing transcripts in [CHAT format](https://talkbank.org/0info/manuals/CHAT.html) (Codes for the Human Analysis of Transcripts). This book documents the Rust reimplementation of CLAN, invoked via the `chatter clan` command.

## Relationship to the legacy CLAN manual

This book treats the
[CLAN manual](https://talkbank.org/0info/manuals/CLAN.html) as the primary
source for legacy command intent, examples, and option semantics when a
command is documented there. Our goal is to incorporate and improve the
non-GUI substance of that manual here, while making divergences from the
legacy C implementation explicit.

GUI-era material from the legacy manual does not belong in the CLI book. That material should instead be carried over into the documentation for the TalkBank VS Code extension, where editor workflows, inspection tools, and interactive affordances can be documented in the right place.

## What's in this book

- **Getting Started**: installation, first commands, migrating from legacy CLAN
- **User Guide**: filtering, output formats, directory workflows
- **Command Reference**: every analysis, transform, and converter command with examples
- **Architecture**: framework design, how to add commands, testing strategy
- **Developer Seams**: current CLI, validation, and dashboard boundaries to preserve while extending the system
- **Divergences**: where and why we differ from legacy CLAN

## Command overview

| Category | Representative commands | Notes |
|----------|-------------------------|-------|
| Analysis | FREQ, MLU, MLT, VOCD, DSS, EVAL, IPSYN | See the per-command pages and status matrix for the current supported set |
| Transform | FLO, CHSTRING, DELIM, DATES, POSTMORTEM | Transform behavior and output modes vary by command |
| Converter | ELAN2CHAT, PRAAT2CHAT, CHAT2SRT, SALT2CHAT | Converter coverage is tracked command-by-command rather than promised as a frozen count |

## Why a reimplementation?

Legacy CLAN is a large, long-lived C/C++ codebase. The Rust reimplementation provides:

- **Semantic AST processing**: works on parsed CHAT structure, not ad-hoc string manipulation
- **Type-safe filtering**: speaker, tier, word, gem, and ID filters via the framework
- **Multiple output formats**: text, JSON, and CSV from a single typed result
- **Golden-tested parity work**: output compared against legacy CLAN binaries, with divergences documented explicitly instead of hidden
- **Modern CLI**: `--flag` syntax with full backward compatibility for CLAN's `+flag` notation

When the Rust implementation differs from the legacy binary, this book tries to distinguish three cases clearly:

- semantic intent preserved, but implemented with typed AST operations instead of ad-hoc string manipulation
- deliberate modernization, such as structured JSON/CSV output or explicit errors instead of silent fallback
- unsupported legacy behavior, which should be documented as unsupported rather than imitated accidentally

## Quick example

```bash
# Word frequency for the CHI speaker
chatter clan freq --speaker CHI transcript.cha

# Mean length of utterance (JSON output)
chatter clan mlu --format json transcript.cha

# Convert ELAN annotation to CHAT
chatter clan elan2chat recording.eaf
```
