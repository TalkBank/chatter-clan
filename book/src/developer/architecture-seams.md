# Current Architecture Seams

**Status:** Current
**Last modified:** 2026-05-30 21:19 EDT

This page documents the current internal seams that contributors should preserve when adding or restructuring CLAN-related functionality.

## CLI command registration

Top-level CLI argument wiring is no longer in one file.

- shared CLI args live in `crates/talkbank-cli/src/cli/args/core.rs`
- shared CLAN filters and common options live in `crates/talkbank-cli/src/cli/args/clan_common.rs`
- CLAN command variants live in `crates/talkbank-cli/src/cli/args/clan_commands.rs`

If you add a new CLAN command, register it in the appropriate split argument module instead of rebuilding a monolithic `args.rs`.

## CLAN dispatch

`run_clan` now lives in `crates/talkbank-cli/src/commands/clan/mod.rs` and dispatches into category files:

- `analysis.rs`
- `transforms.rs`
- `converters.rs`
- `compatibility.rs`
- `helpers.rs` (shared utilities consumed by the others)

Keep family-specific logic in those modules. Shared file resolution, filtering, and output helpers belong in `helpers.rs`, not copied into each family.

## Validation output

Parallel validation output now has a renderer seam:

- orchestration and stats live in `crates/talkbank-cli/src/commands/validate_parallel/runtime.rs`
- output shaping lives in `crates/talkbank-cli/src/commands/validate_parallel/renderer.rs`
- audit-specific behavior lives in `crates/talkbank-cli/src/commands/validate_parallel/audit.rs`

If you need a new output mode, add a renderer implementation instead of extending a large runtime `match`.

Audit-mode JSONL writing is also intentionally isolated. `crates/talkbank-cli/src/commands/validate/audit_reporter.rs` owns a dedicated writer thread and a cloneable reporting handle for workers, so future audit changes should preserve that explicit ownership boundary instead of reintroducing shared writer locks.

## Dashboard state ownership

The old `test-dashboard` note in this chapter referred to a module from an
earlier monorepo layout and does not correspond to a live module in this
repository. There is currently no `src/test_dashboard/` tree in
`TalkBank/chatter`, so treat any future dashboard work as new design work rather
than following those stale paths.

## Editor integration note

The VS Code extension and `talkbank-lsp` use a typed execute-command boundary. The contract surface is documented in:

- `book/src/vscode/reference/rpc-contracts.md`: the RPC contracts the extension and LSP speak
- `book/src/vscode/developer/custom-commands.md`: how to add a new custom command
- `crates/talkbank-lsp/CLAUDE.md`: invariants for the server side
