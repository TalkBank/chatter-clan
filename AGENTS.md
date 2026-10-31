# AGENTS.md

**Status:** Active; modernized onto Chatter v0.34.3.
**Last updated:** 2026-11-13

Guidance for coding agents working in this repository
(`TalkBank/chatter-clan`). Read this before making any change.

## What this repository is

`chatter-clan` is the standalone Rust reimplementation of CLAN's analysis,
transform and converter commands, extracted from `TalkBank/chatter` on
2026-06-15. Active development resumed on 2026-10-30. The repository remains
standalone. The goal is a drop-in replacement for CLAN, with a standalone CLI and
CLAN-style `+flag`/`-flag` syntax. Command and flag coverage is incomplete, and
byte parity is established only where the parity status and tests explicitly
document it. The six NLP commands (mor, post, megrasp, postlist, postmodrules,
posttrain) are deliberate stubs that fail with an explanation.

The pre-extraction history is the `pre-clan-vscode-extraction` tag in
`TalkBank/chatter`.

## Scope boundary: this repo is CLAN command OUTPUT, not CHAT validity

The 2026-06-15 split was done by moving folders, which is the wrong unit:
concerns are intermingled within folders and files. The binding rule going
forward is **partition by semantics, never by moving folders wholesale**.

| Concern | Home |
|---|---|
| CLAN command-**output** reproduction (reproduce CLAN stdout: freq / kwal / mlu / dss / gem and the `check` command's output formatting; command-output goldens) | **here (chatter-clan)** |
| CHAT-format core (grammar / spec / parser / model / validation / transform) | chatter |
| CHECK-**validity** parity (CHECK-to-ErrorCode mapping, canonical CHECK messages, the behavioral validity-harness mechanism, check-vs-validate docs) | chatter |

CHECK-validity assets were **wrongly stranded here** by the folder-move and have
been **recovered into chatter** (the `error_map.rs` curated mapping and the
canonical message table, the golden-parity harness mechanism, and the
check-vs-validate / parity-field-guide docs). The `check` references that remain
in this repo are about reproducing CLAN's CHECK command **output**, not about
deciding CHAT validity. Do not re-add validity concerns here; if a change is
about whether bytes are valid CHAT, it belongs in chatter.

## Workspace layout

```
crates/talkbank-clan/      Library: the CLAN analysis engine (freq, kwal, mlu,
                           dss, gem, ...) plus the analysis / transform /
                           converter framework.
crates/chatter-clan-cli/   Binary crate (package `chatter-clan`, bin
                           `chatter-clan`): the standalone CLI driver over the
                           library.
book/                      The clan-reference mdBook (parity status / roadmap /
                           status matrix and the developer parity field guide).
```

## The chatter dependency: git deps pinned to a release tag

The CHAT core (`talkbank-model`, `talkbank-transform`, and their transitive
crates) is **not vendored**. It is consumed from the public chatter repo via git
dependencies pinned to a release tag in the root `Cargo.toml`
(`[workspace.dependencies]`), currently `tag = "v0.34.3"`, the same pattern
talkbank-tools uses. `TalkBank/chatter` is public, so the fetch needs no auth.

**Bumping the pin is a deliberate step.** A newer chatter tag can surface API
drift in this repository. When you bump the tag, update `Cargo.lock` and fix
any drift in the same change; do not bump it as a drive-by.

## Build, test, run

```bash
cargo build --workspace --locked
cargo test --workspace --lib --bins --locked
./target/debug/chatter-clan --help
```

The CLI crate is binary-only, so smoke-test it by running the binary. `main.rs`
rewrites `+flag`/`-flag` via `talkbank_clan::clan_args::rewrite_clan_args` and
dispatches on argv[0] (a `freq` symlink to `chatter-clan` acts as
`chatter-clan freq`). Golden fixtures come from the Chatter reference corpus;
skipped goldens are not parity evidence. Bind parity claims to the documented
CLAN binary or source-package identity. Existing `@clan` snapshots capture their
specific reference version.

## Known divergences

- `retrace` writes `%xret`, not CLAN's `%ret`: Chatter v0.33.0 requires new
  user-defined tiers to start with `x`.

## Coding standards

Typestate first: no sentinels or silent defaults,
propagate errors, never hand-parse CHAT, use typed accessors (`cleaned_text()`,
not `raw_text()`). Typed errors via `thiserror`; no unwrap/expect/panic outside
tests; clippy `-D warnings`. Known command-output gaps are CHIP `+dN` and
option-specific parity evidence (`-hFILE` supports exact and `*` patterns),
plus FREQ bare `+d` (manual/source discrepancy). CHECK `+g3` is a validity
concern for chatter, not this repository's command-output scope; `+g1` is
already a parser-recognized no-op.
