# CLAUDE.md

**Status:** Dormant but buildable and resumable.
**Last updated:** 2026-06-24 10:44 EDT

Guidance for Claude Code when working in this repository
(`TalkBank/chatter-clan`). Read this before making any change.

## What this repository is

`chatter-clan` is the **dormant, byte-for-byte Rust reimplementation of CLAN's
analysis commands**, extracted from `TalkBank/chatter` on 2026-06-15. The
reimplementation was **halted**: the project direction shifted to building web
frontends on the original C CLAN, so there is no current demand for the Rust
rewrite. This repo exists to keep that code **buildable and
resumable**, not as a site of active development. Do not resume FREQ/KWAL/MLU
parity work or re-open the porting-cost question without a maintainer decision.

The canonical recovery point, with the full integrated git history (including
the CLI `clan` subcommand and the LSP `talkbank/analyze` analysis bridge that
were removed with the extraction), is the `pre-clan-vscode-extraction` tag in
`TalkBank/chatter`.

## Scope boundary: this repo is CLAN command OUTPUT, not CHAT validity

The 2026-06-15 split was done by moving folders, which is the wrong unit:
concerns are intermingled within folders and files. The binding rule going
forward is **partition by semantics, never by moving folders wholesale**.

| Concern | Home |
|---|---|
| CLAN command-**output** reproduction (reproduce CLAN stdout: freq / kwal / mlu / dss / gem and the `check` command's output formatting; command-output goldens) | **here (chatter-clan, halted)** |
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
(`[workspace.dependencies]`), currently `tag = "v0.2.1"`, the same pattern
talkbank-tools uses. `TalkBank/chatter` is public, so the fetch needs no auth.

**Bumping the pin is a deliberate step.** A newer chatter tag can surface API
drift the halted clan code has not tracked (for example the `@ID`-corpus and
Xphoint changes already fixed when adopting v0.2.0). When you bump the tag,
update `Cargo.lock` and fix any drift in the same change; do not bump it as a
drive-by.

## Build, test, run

```bash
cargo build --workspace --locked
cargo test --workspace --locked        # in CI; locally prefer -p <crate>
./target/debug/chatter-clan --help     # the CLI runs the CLAN analysis suite
```

CI (`.github/workflows/ci.yml`) builds the workspace, runs the tests, and
smoke-runs the `chatter-clan` binary, so the README CI badge reflects real
buildability. The CHAT core is fetched from the pinned chatter tag.

## Coding standards

This code inherits the chatter coding standards (newtypes over primitives, typed
domain errors via `thiserror`, no panics in long-lived paths, exhaustive matches
on CHAT content types, Mermaid diagrams in architecture docs). Because the repo
is halted, the operative rule is **keep it buildable**, not extend it: make the
minimal change needed to track a chatter bump or fix a build break, and leave a
note rather than pouring effort into parity gaps.
