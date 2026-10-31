# `talkbank-clan` crate guidance

**Status:** Active
**Last updated:** 2026-10-31

Read the repository-root [`AGENTS.md`](../../AGENTS.md) first. It defines
project scope, current workflow, and shared coding policy. This file records
only crate-specific boundaries.

## Ownership

- This crate implements CLAN command behavior and output, plus shared
  analysis, transform, and converter machinery.
- The standalone CLI and argument dispatch live in
  `../chatter-clan-cli/`; keep their call sites consistent with library API
  changes.
- CHAT grammar, parsing, the typed model, and validity rules belong in the
  `TalkBank/chatter` repository. Do not move those concerns into this crate.
- CHECK work here concerns the CHECK command's output behavior. CHAT validity
  adjudication and parser/validator changes belong in chatter.

## Parity work

- Consult the relevant CLAN manual section for intended command behavior.
  Treat source or binary behavior as evidence of what CLAN does, not automatic
  proof that the behavior is correct.
- Use the command page under `../../book/src/commands/` as a status map, not as
  an authority. Distinguish implemented options from verified parity.
- Do not infer parity from unit tests alone. A skipped golden or a snapshot
  without a documented oracle does not establish CLAN equivalence.
- Unsupported or inapplicable CLAN options must fail visibly; do not silently
  ignore them or route them to a different option. An intentional no-op needs
  evidence that it is semantically a no-op.
- Preserve the typed CHAT model at command boundaries. Reject inputs whose
  structure cannot be handled faithfully rather than inventing output.

## Design and verification

- Prefer validated configuration types and explicit transitions over flags,
  sentinels, or repeated checks. Keep invalid states unrepresentable.
- Keep command logic in `talkbank-clan` and parsing/dispatch in the CLI crate;
  avoid command-specific CLI behavior hidden in generic framework code.
- Use the focused test command and build instructions in the repository-root
  `AGENTS.md`. Do not run overlapping Cargo commands in this workspace.
