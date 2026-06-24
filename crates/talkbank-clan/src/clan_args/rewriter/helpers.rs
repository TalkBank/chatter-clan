//! Value-shaping helpers for the CLAN flag rewriter.
//!
//! These free functions take the already-classified `(polarity, rest)` of a
//! CLAN `+flag`/`-flag` and produce the rewritten `--long-flag value` argv
//! tokens (or `None` when the value is empty / unrecognized, which the caller
//! treats as "not this branch"). They were extracted verbatim from
//! `rewriter.rs`; the parent dispatch (`try_rewrite_clan_flag`) calls them by
//! name via the `use helpers::*;` re-export in the parent module.

/// Rewrite `+t*CHI` → `--speaker CHI`, `-t*MOT` → `--exclude-speaker MOT`,
/// `+t%mor` → `--tier mor`, `-t%gra` → `--exclude-tier gra`.
///
/// CLAN also accepts `+tCHI` (no `*` sigil) and treats it identically
/// to `+t*CHI`; this function does the same, when the first character
/// of the value is not one of `*`, `%`, or `@`, the value is taken as
/// an implicit speaker code.
pub(super) fn rewrite_tier_speaker(polarity: u8, rest: &str) -> Option<Vec<String>> {
    if rest.is_empty() {
        return None;
    }

    match rest.as_bytes()[0] {
        b'*' => {
            let speaker = &rest[1..];
            if speaker.is_empty() {
                return None;
            }
            let flag = if polarity == b'+' {
                "--speaker"
            } else {
                "--exclude-speaker"
            };
            Some(vec![flag.into(), speaker.to_string()])
        }
        b'%' => {
            let tier = &rest[1..];
            if tier.is_empty() {
                return None;
            }
            let flag = if polarity == b'+' {
                "--tier"
            } else {
                "--exclude-tier"
            };
            Some(vec![flag.into(), tier.to_string()])
        }
        b'@' => {
            // +t@ID="eng|*|CHI|*" → --id-filter "eng|*|CHI|*"
            if rest.len() >= 4 && rest[1..].starts_with("ID=") {
                let value = strip_quotes(&rest[4..]);
                if value.is_empty() {
                    return None;
                }
                Some(vec!["--id-filter".into(), value])
            } else {
                None
            }
        }
        b'#' => {
            // `+t#Target_Child` → `--role Target_Child`.
            // The `-t#ROLE` exclude-by-role form is not currently
            // supported by CLAN (per `mainusage()` the role flag is
            // include-only), so polarity `b'-'` falls through to the
            // default branch below and is treated as a literal
            // speaker code, matching CLAN's `+tCHI`/`-tCHI` shape.
            if polarity != b'+' {
                return None;
            }
            let role = &rest[1..];
            if role.is_empty() {
                return None;
            }
            Some(vec!["--role".into(), role.to_string()])
        }
        _ => {
            // `+tCHI` / `-tMOT`, CLAN treats the value as an implicit
            // speaker code (equivalent to `+t*CHI` / `-t*MOT`). Match
            // that behaviour.
            let flag = if polarity == b'+' {
                "--speaker"
            } else {
                "--exclude-speaker"
            };
            Some(vec![flag.into(), rest.to_string()])
        }
    }
}

/// Rewrite `+s"word"` or `+sword` → `--include-word word`,
/// `-s"word"` or `-sword` → `--exclude-word word`.
pub(super) fn rewrite_search_word(polarity: u8, rest: &str) -> Option<Vec<String>> {
    if rest.is_empty() {
        return None;
    }
    let word = strip_quotes(rest);
    if word.is_empty() {
        return None;
    }
    let flag = if polarity == b'+' {
        "--include-word"
    } else {
        "--exclude-word"
    };
    Some(vec![flag.into(), word])
}

/// Rewrite `+glabel` → `--gem label`, `-glabel` → `--exclude-gem label`.
pub(super) fn rewrite_gem(polarity: u8, rest: &str) -> Option<Vec<String>> {
    if rest.is_empty() {
        return None;
    }
    let label = strip_quotes(rest);
    if label.is_empty() {
        return None;
    }
    let flag = if polarity == b'+' {
        "--gem"
    } else {
        "--exclude-gem"
    };
    Some(vec![flag.into(), label])
}

/// True when `rest` is a non-empty all-ASCII-digit string, used by
/// per-command pass-through arms (`+gN`, `+tN`, etc.) that need to
/// distinguish digit-suffix forms from other shapes.
pub(super) fn rest_is_digits(rest: &str) -> bool {
    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
}

/// Recognize the unintelligible-marker restore tokens of a `+x` content-include
/// flag, returning the canonical marker (`xxx`/`yyy`/`www`) or `None` for any
/// other `+xWORD`. CLAN (`cutt.cpp:9890-9896`) accepts the three-letter markers
/// AND their two-letter aliases (`xx`/`yy`/`ww`), case-insensitively
/// (`mStricmp`); both collapse to the canonical three-letter token here.
pub(super) fn restore_marker_token(rest: &str) -> Option<&'static str> {
    if rest.eq_ignore_ascii_case("xxx") || rest.eq_ignore_ascii_case("xx") {
        Some("xxx")
    } else if rest.eq_ignore_ascii_case("yyy") || rest.eq_ignore_ascii_case("yy") {
        Some("yyy")
    } else if rest.eq_ignore_ascii_case("www") || rest.eq_ignore_ascii_case("ww") {
        Some("www")
    } else {
        None
    }
}

/// Rewrite `+z25-125` → `--range 25-125`.
pub(super) fn rewrite_range(rest: &str) -> Option<Vec<String>> {
    if rest.is_empty() {
        return None;
    }
    Some(vec!["--range".into(), rest.to_string()])
}

/// Build a `[long_flag, value]` token pair for the simple `+X<value>`
/// shape shared by the per-subcommand routing branches
/// (`+cN`/`+lF`/`+sF`/`+gS`/`+aN`). Returns `None` when there is no
/// value (the caller treats that as "not this branch"); the caller is
/// responsible for the subcommand guard.
pub(super) fn rewrite_subcommand_value_flag(rest: &str, long_flag: &str) -> Option<Vec<String>> {
    if rest.is_empty() {
        return None;
    }
    Some(vec![long_flag.into(), rest.to_string()])
}

/// Rewrite `+wN` → `--context-after N`, `-wN` → `--context-before N`.
/// Parse WDSIZE's `+w[>|<|=]N` length-filter argument and emit an
/// `--length-filter <gt|lt|eq>:N` argv pair. Returns `None` when
/// the input doesn't lead with a recognized comparator, in which
/// case the caller falls through to the general `+wN` context-
/// window rewrite. CLAN's WDSIZE only documents these three
/// comparators.
pub(super) fn rewrite_wdsize_length_filter(rest: &str) -> Option<Vec<String>> {
    let bytes = rest.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let (tag, n_str) = match bytes[0] {
        b'>' => ("gt", &rest[1..]),
        b'<' => ("lt", &rest[1..]),
        b'=' => ("eq", &rest[1..]),
        _ => return None,
    };
    n_str.parse::<usize>().ok()?;
    Some(vec!["--length-filter".into(), format!("{tag}:{n_str}")])
}

pub(super) fn rewrite_context_window(polarity: u8, rest: &str) -> Option<Vec<String>> {
    if rest.is_empty() {
        return None;
    }
    if !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let flag = if polarity == b'+' {
        "--context-after"
    } else {
        "--context-before"
    };
    Some(vec![flag.into(), rest.to_string()])
}

/// Strip surrounding double quotes from a string value.
pub(super) fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Rewrite CHECK's `+g1`-`+g5` generic options.
///
/// | Flag | Meaning |
/// |------|---------|
/// | `+g1` | Check prosodic delimiters (no-op, always on) |
/// | `+g2` | Check CHI has Target_Child role |
/// | `+g3` | Word detail checks (partially implemented via parser) |
/// | `+g4` | Check for missing @ID tiers (on by default) |
/// | `+g5` | Check for unused speakers |
///
/// Falls back to gem rewriting if the rest is not a single digit 1-5.
pub(super) fn rewrite_check_generic(polarity: u8, rest: &str) -> Option<Vec<String>> {
    match rest {
        "1" => Some(vec![]), // no-op: prosodic delimiters always recognized
        "2" => Some(vec!["--check-target".into()]),
        "3" => Some(vec![]), // no-op: word checks via parser
        "4" => Some(vec!["--check-id".into(), "true".into()]),
        "5" => Some(vec!["--check-unused".into()]),
        // Not a CHECK generic option, fall back to gem
        _ => rewrite_gem(polarity, rest),
    }
}

/// Rewrite `+eN` → `--error N`, `+e` → `--list-errors`.
pub(super) fn rewrite_check_error(rest: &str) -> Option<Vec<String>> {
    if rest.is_empty() {
        Some(vec!["--list-errors".into()])
    } else {
        Some(vec!["--error".into(), rest.to_string()])
    }
}
