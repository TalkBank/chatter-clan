# chatter-clan

**Status:** Active standalone CLAN replacement project.
**Extracted:** 2026-06-15 from `TalkBank/chatter`; resumed 2026-10-30.

[![CI](https://github.com/TalkBank/chatter-clan/actions/workflows/ci.yml/badge.svg)](https://github.com/TalkBank/chatter-clan/actions/workflows/ci.yml)

`chatter-clan` is the standalone Rust CLI for CLAN's analysis, transform, and
converter commands. Its goal is to become a practical drop-in replacement for
CLAN, including familiar `+flag`/`-flag` arguments. Parity work is active but
incomplete: implemented options and passing unit tests are not, by themselves,
evidence of byte-for-byte parity. See the [parity status](book/src/parity-status.md)
and [roadmap](book/src/appendices/parity-roadmap.md).

The CHAT model and parser are consumed from the public `TalkBank/chatter`
repository, currently pinned to release `v0.34.3`. `chatter-clan` remains a
separate repository; its CLI and CLAN command implementations do not move back
into the chatter workspace.

## Contents

- `crates/talkbank-clan/`: the CLAN analysis engine and shared
  analysis/transform/converter framework.
- `crates/chatter-clan-cli/`: the standalone `chatter-clan` binary.
- `book/`: user documentation, command references, and parity tracking.

Six NLP commands (`mor`, `post`, `megrasp`, `postlist`, `postmodrules`, and
`posttrain`) are explicit stubs; they are not implemented replacements for
their CLAN counterparts.

## Build and run

```bash
cargo build --workspace
./target/debug/chatter-clan --help
./target/debug/chatter-clan freq --help
```

See [installation](book/src/getting-started/installation.md) for installing the
release binary from source, and [`AGENTS.md`](AGENTS.md) for repository
development guidance.
