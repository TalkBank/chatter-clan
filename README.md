# chatter-clan (dormant preservation copy)

**Status:** Dormant for development, but kept buildable and green in CI.
**Extracted:** 2026-06-15, from `TalkBank/chatter`.

[![CI](https://github.com/TalkBank/chatter-clan/actions/workflows/ci.yml/badge.svg)](https://github.com/TalkBank/chatter-clan/actions/workflows/ci.yml)

This repository preserves the `talkbank-clan` crate (a byte-for-byte Rust
reimplementation of CLAN analysis commands) and its `clan-reference` book
section, which were removed from `TalkBank/chatter` on 2026-06-15 when active
development paused. The direction shifted toward integrating frontends with the
original C CLAN, and there is no current demand for the Rust reimplementation.
The work is preserved here so it can be resumed if demand reappears.

## Contents

- `crates/talkbank-clan/` : the CLAN-analysis crate (FREQ, MLU, KWAL, COMBO,
  CHECK, and the analysis/transform/converter framework).
- `book/src/clan-reference/` : the CLAN reference documentation, including the
  honest `parity-status.md` / `parity-roadmap.md` / `status-matrix.md` and the
  developer parity field guide.

## Recovery and history

This repository is a snapshot for discoverability. The canonical recovery point,
with the full git history of this code integrated in the chatter workspace
(including the CLI `clan` subcommand and the LSP `talkbank/analyze` analysis
family that drove it), is the `pre-clan-vscode-extraction` tag in
`TalkBank/chatter`.

## To resume

`talkbank-clan` depends on chatter's core crates (`talkbank-model`,
`talkbank-transform`, `talkbank-parser`), so it is not standalone-buildable as
extracted. To resume: re-add it as a workspace member of a chatter checkout (or
point its path/git dependencies at chatter's core crates), then restore the CLI
`clan` subcommand and, if wanted, the LSP analysis surface from the
`pre-clan-vscode-extraction` tag.
