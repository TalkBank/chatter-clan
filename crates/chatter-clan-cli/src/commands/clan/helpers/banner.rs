use std::path::Path;

use talkbank_clan::framework::IdFilter;

/// Legacy-CLAN version string emitted in the banner.
///
/// CLAN's `VersionNumber()` prints a build date string in
/// `(DD-Mon-YYYY)` shape, the second `(` and `)` come from CLAN's
/// `printf` template. chatter's banner template adds the parens too,
/// so the constant here is the bare version content with no
/// surrounding parens.
///
/// The value is injected by [`build.rs`](../../../../build.rs) using
/// chrono's `%e-%b-%Y` format (e.g. `21-May-2026`). CLAN itself uses
/// a hardcoded string updated by hand at release time; we substitute
/// the chatter build date, which is more honest and still matches the
/// `DD-Mon-YYYY` shape researchers parse out of the banner.
pub(super) const CLAN_BANNER_VERSION: &str = env!("CLAN_BUILD_DATE");

/// Return the CLAN-style timestamp string for the current moment, matching
/// the `ctime()` format CLAN's mainloop uses (e.g.
/// `"Thu May 21 17:47:15 2026"`).
pub(super) fn clan_timestamp_now() -> String {
    use chrono::Local;
    Local::now().format("%a %b %e %H:%M:%S %Y").to_string()
}

/// Build the CLAN-style line-1 invocation echo from chatter's full argv.
///
/// CLAN's banner line 1 echoes the user's command-line argv verbatim:
///
/// ```text
/// freq +scat path/to/file.cha
/// ```
///
/// chatter is invoked as `chatter clan <command> <args...>`. The CLAN
/// analog drops the `chatter clan` prefix and starts at the CLAN
/// subcommand name (`<command>`). The slice from `clan_pos + 1` onward
/// is what we want, joined by single spaces.
///
/// **Chatter-only flag filtering.** Some chatter flags have no CLAN
/// analog and should not appear in the echo. Today we filter:
/// * `--format <X>` / `--format=<X>` / `-f <X>` / `-f=<X>`, chatter-
///   specific output-format selector. The CLAN banner only emits when
///   format is `clan`, so the flag is noise even when present.
///
/// Other chatter-only flags (`--per-file`, `--output`, `--id-filter`,
/// …) are not filtered yet; they will be addressed per-command in
/// Phase 1.7 of the CLAN parity plan
/// (`scripts/clan-parity/PLAN.md`). For the typical migration use
/// case, researchers pasting CLAN-style `+flag` arguments into
/// `chatter clan`, no filtering kicks in.
///
/// Pure function: takes argv + clan position; the caller threads in
/// `std::env::args()` and the `clan` index from the dispatcher.
pub(super) fn build_clan_invocation_echo(args: &[String], clan_pos: Option<usize>) -> String {
    let Some(clan_pos) = clan_pos else {
        return String::new();
    };
    let tail = match args.get(clan_pos + 1..) {
        Some(slice) => slice,
        None => return String::new(),
    };

    let mut out: Vec<&str> = Vec::with_capacity(tail.len());
    let mut i = 0;
    while i < tail.len() {
        let arg = tail[i].as_str();
        match arg {
            "--format" | "-f" => {
                // Skip the flag and its value if present.
                i += if i + 1 < tail.len() { 2 } else { 1 };
            }
            _ if arg.starts_with("--format=") || arg.starts_with("-f=") => {
                i += 1;
            }
            _ => {
                out.push(arg);
                i += 1;
            }
        }
    }
    out.join(" ")
}

/// Find the position of the `clan` subcommand in chatter's argv.
///
/// Returns `None` if `clan` does not appear after `argv[0]`. Used only
/// from the banner-emission path, so by construction the value is
/// always `Some(_)` when reached, but we return `Option` for
/// testability and defensive programming.
pub(super) fn find_clan_subcommand_position(args: &[String]) -> Option<usize> {
    args.iter()
        .enumerate()
        .skip(1)
        .find_map(|(i, arg)| if arg == "clan" { Some(i) } else { None })
}

/// Capture the runtime argv and build the CLAN-style invocation echo.
///
/// Public wrapper around [`build_clan_invocation_echo`] that handles
/// the `std::env::args` lookup. Pure function lives below for tests.
pub(super) fn clan_invocation_echo() -> String {
    let args: Vec<String> = std::env::args().collect();
    let clan_pos = find_clan_subcommand_position(&args);
    build_clan_invocation_echo(&args, clan_pos)
}

/// Build the banner's "main-tier scope" sentence, the way CLAN's
/// `cutt.cpp` mainloop renders it.
///
/// CLAN's wording is tightly fixed; this enumeration is exhaustive
/// over what chatter's CLI lets the user express today via
/// `--speaker` / `--exclude-speaker` / `--role`:
///
/// | Includes | Excludes | Roles | Banner sentence |
/// |---------|----------|-------|---------------------------------------------------------------------|
/// | empty   | empty    | empty | `ALL speaker tiers` |
/// | one+    | _any_    | _any_ | `ONLY speaker main tiers matching: *CHI;` (`+t…` wins over `-t…`) |
/// | empty   | one+     | _any_ | `ALL speaker main tiers EXCEPT the ones matching: *MOT;` |
/// | empty   | empty    | one+  | `ONLY speaker main tiers with role(s): TARGET_CHILD;` |
///
/// CLAN precedence: speaker codes outrank role names. When both
/// `+t*CHI` and `+t#Target_Child` are supplied, the banner uses the
/// speaker-code shape, the role only fires when no `+t*` is
/// present. Matches the order of precedence in
/// `clan_args::rewrite_tier_speaker`.
///
/// Multiple values are joined with single spaces and each entry
/// trails its own semicolon (CLAN's per-pattern delimiter). The
/// `*` prefix is the CLAN speaker-tier sigil; chatter's rewriter
/// strips the `*` when it rewrites `+t*CHI` → `--speaker CHI`, so
/// we re-prepend it here. Role names are uppercased.
///
/// The `… with IDs matching: …` shape for `+t@ID="…"` filters is
/// out of scope here, `--id-filter` lives on a separate banner
/// pass that lowercases the pattern and emits an extra `*:;`
/// continuation (Phase 1.6 follow-up).
///
/// Pure function for testability, no I/O, no env lookup, no
/// command-specific branching (the caller's `clan_scope_for`
/// wraps this with the dep-tier suffix).
pub(super) fn build_main_scope(
    includes: &[String],
    excludes: &[String],
    roles: &[String],
    id_filter: Option<&IdFilter>,
) -> String {
    // CLAN `+t@ID`: select speakers by an `@ID` glob. CLAN lowercases the
    // pattern in the banner and appends its implicit `*:;` speaker default.
    if let Some(id_filter) = id_filter {
        return format!(
            "ONLY speaker main tiers with IDs matching: {}; *:;",
            id_filter.pattern().to_lowercase()
        );
    }
    if !includes.is_empty() {
        let body = clan_speaker_pattern_list(includes);
        return format!("ONLY speaker main tiers matching: {body}");
    }
    if !excludes.is_empty() {
        let body = clan_speaker_pattern_list(excludes);
        return format!("ALL speaker main tiers EXCEPT the ones matching: {body}");
    }
    if !roles.is_empty() {
        let body = roles
            .iter()
            .map(|r| format!("{};", r.to_uppercase()))
            .collect::<Vec<_>>()
            .join(" ");
        return format!("ONLY speaker main tiers with role(s): {body}");
    }
    "ALL speaker tiers".to_owned()
}

/// Render a list of bare speaker codes (no `*` prefix, as
/// `clan_args::rewrite_tier_speaker` stores them) into CLAN's
/// `*CHI; *MOT;` banner shape: each code gets a leading `*` and a
/// trailing `;`, joined by single spaces.
fn clan_speaker_pattern_list(codes: &[String]) -> String {
    codes
        .iter()
        .map(|c| format!("*{c};"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build the `From file <basename>` line that CLAN emits below the
/// `****` separator. CLAN truncates to the path's basename; we follow
/// the same convention so the banner matches.
pub(super) fn clan_source_for(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    format!("From file <{name}>")
}
