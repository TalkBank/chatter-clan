# Installation

**Status:** Current
**Last modified:** 2026-10-31

`chatter-clan` is the standalone CLI that reimplements CLAN's analysis commands
over the chatter CHAT toolchain. It is not published on crates.io, so build it
from this repository.

> **Parity is in progress.** The project is active and standalone, but not every
> CLAN command or option is implemented or verified. See the repository
> [`README.md`](../../../README.md), [`AGENTS.md`](../../../AGENTS.md),
> and [parity status](../parity-status.md).

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
