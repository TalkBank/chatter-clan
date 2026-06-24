# POSTMODRULES -- Modify POST Database Rules (deliberately not implemented)

**Status:** Reference -- stub command
**Last updated:** 2026-06-14 19:57 EDT

## Purpose

In legacy CLAN, `postmodrules` edits the POST disambiguation rule
database -- it allows researchers to update the context-sensitive
rules that [`post`](post.md) applies, without retraining the full
model.

This project does **not** implement `postmodrules`. The POST rule
database does not exist here -- the neural morphotag pipeline
([see `mor`](mor.md)) supersedes it.

## What to use instead

There is no direct replacement. Custom morphology rules are not part
of the upstream `batchalign3` neural pipeline; the Stanza models are
trained end-to-end. For per-language overrides, the upstream
`batchalign3` project documents its own contributor entry point for
non-English workarounds.

## Behavior

Invoking `chatter clan postmodrules` prints an error and exits
non-zero.

## See also

- [MOR](mor.md), [POST](post.md), [POSTLIST](postlist.md),
  [POSTTRAIN](posttrain.md) -- companion stubs

## CLAN `+`-flag coverage audit

**Exempt**: see [MOR](mor.md#clan--flag-coverage-audit) for the
shared MOR-pipeline policy: chatter does not consume CLAN's
`+`-flags for any command in this family. CLAN's own usage text
documents the legacy flag surface; chatter emits a refusal that
points users to batchalign.
