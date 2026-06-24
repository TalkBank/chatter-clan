# Installation

**Status:** Current
**Last modified:** 2026-06-24 09:00 EDT

`chatter-clan` is the standalone CLI that reimplements CLAN's analysis commands
over the chatter CHAT toolchain. It is not published on crates.io, so build it
from this repository.

> **Dormant.** chatter-clan is an experimental, halted reimplementation kept
> buildable so it can be resumed or forked. See the repository `README.md` and
> `CLAUDE.md` for status and scope.

## From source

```bash
# Clone the repository
git clone https://github.com/TalkBank/chatter-clan.git
cd chatter-clan

# Install the chatter-clan binary onto your PATH
cargo install --path crates/chatter-clan-cli --locked

# Or just build it in place (binary at target/release/chatter-clan)
cargo build --release
```

The CHAT core (`talkbank-model`, `talkbank-transform`) is fetched automatically
from the public chatter repository at the pinned release tag; no separate
checkout of chatter is needed.

## Verify installation

```bash
chatter-clan --help
chatter-clan freq --help
```

## Requirements

- Rust 2024 edition (rustc 1.85+)
- macOS, Linux, or Windows
- A network connection on first build (to fetch the pinned chatter crates)
- No runtime dependencies beyond the binary
