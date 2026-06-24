//! Golden tests comparing talkbank-clan output against CLAN CLI output.
//!
//! These tests are split by concern:
//! - `harness` contains shared CLAN/Rust execution helpers and generated test runners
//! - `golden_parity` is the canonical fail-closed parity mechanism: chatter's
//!   CLAN-format output asserted byte-for-byte against committed `.clan.txt`
//!   goldens (`MatchesClan` / `DivergesFromClan`), needing no CLAN binary to verify
//! - `rust_only` keeps bespoke temp-file coverage alongside chatter-only snapshots
//! - `legacy_audit` is the (CLAN-gated, `#[ignore]`d) MATCH/DIVERGE backlog audit
//!
//! The old dual-snapshot `parity_case_tests!` mechanism (`baseline`, `check`,
//! `variants_*`) was retired: its `@clan` reference snapshots were never
//! committed, so it failed locally with CLAN and silently skipped without it,
//! protecting nothing. Its real coverage lives in `golden_parity`; the
//! remaining command/flag backlog lives in `legacy_audit` and the book's
//! per-command audit tables.

mod common;
#[path = "clan_golden/harness.rs"]
mod harness;

use crate::harness::{
    FilterSpec, OutputFormat, RustSnapshotCase, corpus_file, rust_snapshot_tests,
};

include!("clan_golden/rust_only.rs");
include!("clan_golden/golden_parity.rs");
include!("clan_golden/book_audit.rs");
include!("clan_golden/metric.rs");
include!("clan_golden/legacy_audit.rs");
