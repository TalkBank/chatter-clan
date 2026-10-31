# Changelog

All notable changes to chatter-clan are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The standalone CLAN replacement project resumed active development on
2026-10-30. Parity remains in progress; no releases have been cut yet.

## [Unreleased]

### Changed

- Resumed parity work in this standalone repository; the project is not being
  merged back into the chatter workspace.
- Updated the CHAT core pin from `v0.2.1` to the latest published Chatter
  release, `v0.33.0`.

### Added

- Standalone `chatter-clan` binary (recovered from the pre-extraction history).
  The CLAN analysis commands now run as `chatter-clan <command>` rather than the
  former `chatter clan <command>` subcommand.
- Workspace manifest, CI (build, test, smoke-run the binary), repository
  `CLAUDE.md`, and this changelog, so the crate extracted from chatter on
  2026-06-15 builds standalone again.
- Reconstructed the standalone book (its `book.toml` and `SUMMARY.md` were not
  carried over by the extraction).

[Unreleased]: https://github.com/TalkBank/chatter-clan/commits/main
