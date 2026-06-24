//! Build script: emits `CLAN_BUILD_DATE` for the CLAN banner's version slot.
//!
//! Recovered from talkbank-cli. The `chatter-clan` banner prints this in CLAN's
//! `DD-Mon-YYYY` shape (where CLAN hardcodes a `VersionNumber()` string), because
//! researchers' tooling parses that slot to recognize CLAN output; emitting the
//! real build date is more honest than a baked-in version.

fn main() {
    // chrono's `%e` pads single-digit days with a leading space (e.g.
    // ` 1-May-2026`); CLAN emits without padding, so trim the leading space.
    let formatted = chrono::Local::now().format("%e-%b-%Y").to_string();
    let clan_build_date = formatted.trim_start();
    println!("cargo:rustc-env=CLAN_BUILD_DATE={clan_build_date}");
}
