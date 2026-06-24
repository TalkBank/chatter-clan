# Changelog

All notable changes to chatter-clan are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

chatter-clan is a **dormant, experimental** reimplementation kept buildable so it
can be resumed or forked. No releases have been cut yet.

## [Unreleased]

### Added

- Standalone `chatter-clan` binary (recovered from the pre-extraction history).
  The CLAN analysis commands now run as `chatter-clan <command>` rather than the
  former `chatter clan <command>` subcommand.
- Workspace manifest, CI (build, test, smoke-run the binary), repository
  `CLAUDE.md`, and this changelog, so the crate extracted from chatter on
  2026-06-15 builds standalone again.
- Reconstructed the standalone book (its `book.toml` and `SUMMARY.md` were not
  carried over by the extraction).

### Changed

- The CHAT core (`talkbank-model`, `talkbank-transform`, and their transitive
  crates) is consumed from the public chatter repo via git dependencies pinned
  to the `v0.2.0` tag (the `@ID`-corpus-required and Xphoint API changes were
  tracked when adopting it).

[Unreleased]: https://github.com/TalkBank/chatter-clan/commits/main
