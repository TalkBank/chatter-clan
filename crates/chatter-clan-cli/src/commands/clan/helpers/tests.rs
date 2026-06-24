use super::banner::{
    CLAN_BANNER_VERSION, build_clan_invocation_echo, find_clan_subcommand_position,
};

fn s(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|p| p.to_string()).collect()
}

#[test]
fn invocation_echo_strips_chatter_clan_prefix() {
    let args = s(&["chatter", "clan", "freq", "+scat", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(
        build_clan_invocation_echo(&args, pos),
        "freq +scat file.cha"
    );
}

#[test]
fn invocation_echo_handles_no_flags() {
    let args = s(&["chatter", "clan", "freq", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(build_clan_invocation_echo(&args, pos), "freq file.cha");
}

#[test]
fn invocation_echo_filters_dash_dash_format_with_value() {
    let args = s(&["chatter", "clan", "freq", "--format", "clan", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(build_clan_invocation_echo(&args, pos), "freq file.cha");
}

#[test]
fn invocation_echo_filters_dash_dash_format_equals() {
    let args = s(&["chatter", "clan", "freq", "--format=clan", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(build_clan_invocation_echo(&args, pos), "freq file.cha");
}

#[test]
fn invocation_echo_filters_short_dash_f_with_value() {
    let args = s(&["chatter", "clan", "freq", "-f", "clan", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(build_clan_invocation_echo(&args, pos), "freq file.cha");
}

#[test]
fn invocation_echo_filters_short_dash_f_equals() {
    let args = s(&["chatter", "clan", "freq", "-f=clan", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(build_clan_invocation_echo(&args, pos), "freq file.cha");
}

#[test]
fn invocation_echo_skips_global_flags_before_clan() {
    let args = s(&["chatter", "--verbose", "clan", "freq", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(pos, Some(2));
    assert_eq!(build_clan_invocation_echo(&args, pos), "freq file.cha");
}

#[test]
fn invocation_echo_empty_when_clan_absent() {
    let args = s(&["chatter", "validate", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(pos, None);
    assert_eq!(build_clan_invocation_echo(&args, pos), "");
}

#[test]
fn main_scope_no_filter() {
    assert_eq!(
        super::banner::build_main_scope(&[], &[], &[], None),
        "ALL speaker tiers"
    );
}

#[test]
fn main_scope_single_include() {
    assert_eq!(
        super::banner::build_main_scope(&["CHI".into()], &[], &[], None),
        "ONLY speaker main tiers matching: *CHI;"
    );
}

#[test]
fn main_scope_multi_include() {
    assert_eq!(
        super::banner::build_main_scope(&["CHI".into(), "MOT".into()], &[], &[], None),
        "ONLY speaker main tiers matching: *CHI; *MOT;"
    );
}

#[test]
fn main_scope_single_exclude() {
    assert_eq!(
        super::banner::build_main_scope(&[], &["MOT".into()], &[], None),
        "ALL speaker main tiers EXCEPT the ones matching: *MOT;"
    );
}

#[test]
fn main_scope_multi_exclude() {
    assert_eq!(
        super::banner::build_main_scope(&[], &["MOT".into(), "FAT".into()], &[], None),
        "ALL speaker main tiers EXCEPT the ones matching: *MOT; *FAT;"
    );
}

#[test]
fn main_scope_include_wins_over_exclude() {
    // CLAN observed behaviour: when both +t and -t are present, the
    // banner reports only the include side (exclude still filters
    // output, but the scope line stays silent about it).
    assert_eq!(
        super::banner::build_main_scope(&["CHI".into()], &["MOT".into()], &[], None),
        "ONLY speaker main tiers matching: *CHI;"
    );
}

#[test]
fn role_scope_single() {
    assert_eq!(
        super::banner::build_main_scope(&[], &[], &["Target_Child".into()], None),
        "ONLY speaker main tiers with role(s): TARGET_CHILD;"
    );
}

#[test]
fn role_scope_multi() {
    assert_eq!(
        super::banner::build_main_scope(&[], &[], &["Target_Child".into(), "Mother".into()], None),
        "ONLY speaker main tiers with role(s): TARGET_CHILD; MOTHER;"
    );
}

/// CLAN precedence: speaker codes outrank role names. When both
/// `+t*CHI` and `+t#Target_Child` are supplied, the banner uses
/// the speaker shape.
#[test]
fn role_scope_yields_to_speaker_include() {
    assert_eq!(
        super::banner::build_main_scope(&["CHI".into()], &[], &["Target_Child".into()], None),
        "ONLY speaker main tiers matching: *CHI;"
    );
}

#[test]
fn role_scope_empty_falls_back_to_all() {
    assert_eq!(
        super::banner::build_main_scope(&[], &[], &[], None),
        "ALL speaker tiers"
    );
}

#[test]
fn invocation_echo_preserves_clan_style_speaker_flag() {
    // The typical migration case: researchers paste CLAN-style
    // `+t*CHI` directly; chatter's argv rewriter expands it before
    // clap, but the echo path reads ORIGINAL argv so the CLAN-style
    // flag survives verbatim into the banner.
    let args = s(&["chatter", "clan", "freq", "+t*CHI", "file.cha"]);
    let pos = find_clan_subcommand_position(&args);
    assert_eq!(
        build_clan_invocation_echo(&args, pos),
        "freq +t*CHI file.cha"
    );
}

/// Verify the banner version is shaped as CLAN's `(DD-Mon-YYYY)` build
/// date, `D-Mon-YYYY` or `DD-Mon-YYYY`, with `Mon` being the
/// abbreviated English month name (`Jan`, `Feb`, …, `Dec`).
///
/// chrono's `%e-%b-%Y` format yields a *space-padded* day for
/// single-digit days (e.g. ` 1-May-2026`); we trim leading whitespace
/// before the constant is read, so the test allows 1 or 2 day digits
/// with no leading whitespace.
#[test]
fn banner_version_matches_clan_date_format() {
    const MONTHS: &[&str] = &[
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let parts: Vec<&str> = CLAN_BANNER_VERSION.split('-').collect();
    assert_eq!(
        parts.len(),
        3,
        "CLAN_BANNER_VERSION = {CLAN_BANNER_VERSION:?} \
         should split into [day, mon, year] on '-'"
    );

    let day = parts[0];
    let mon = parts[1];
    let year = parts[2];

    assert!(
        (1..=2).contains(&day.len()) && day.bytes().all(|b| b.is_ascii_digit()),
        "day {day:?} must be 1 or 2 ASCII digits"
    );
    let day_value: u32 = day.parse().expect("day parses");
    assert!(
        (1..=31).contains(&day_value),
        "day {day_value} out of range"
    );

    assert!(
        MONTHS.contains(&mon),
        "month {mon:?} must be one of {MONTHS:?}"
    );

    assert_eq!(year.len(), 4, "year {year:?} must be 4 digits");
    let year_value: i32 = year.parse().expect("year parses");
    // build.rs runs at compile time; this constant is a build date,
    // so accept any plausible build year window. 2026 is when this
    // test lands; future-proof to 2100.
    assert!(
        (2025..=2100).contains(&year_value),
        "year {year_value} out of plausible window"
    );
}
