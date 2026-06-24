//! CLAN argument pre-processor.
//!
//! Rewrites legacy CLAN `+flag`/`-flag` syntax into modern `--flag` equivalents
//! so that clap can parse them. This allows users to write either:
//!
//! ```text
//! clan analyze freq +t*CHI +s"want" +z25-125 file.cha
//! ```
//!
//! or the modern equivalent:
//!
//! ```text
//! clan analyze freq --speaker CHI --include-word want --range 25-125 file.cha
//! ```
//!
//! The rewriter is a pure function that operates on the raw argument list before
//! clap sees it. It only touches arguments that look like CLAN flags (`+` or `-`
//! prefix followed by a known flag letter); everything else passes through unchanged.

mod rewriter;

use rewriter::try_rewrite_clan_flag;

/// The set of CLAN analysis subcommands chatter knows about for the purpose of
/// per-subcommand `+`-flag dispatch.
///
/// CLAN's `+`-flag semantics depend on which analysis command the user invoked:
/// `+cN` is `--bullets` under CHECK, `--limit` under MAXWD, and
/// `--max-utterances` under IPSYN/DSS. The rewriter needs to know which
/// subcommand is active to pick the right rewrite. This enum captures the
/// subset of subcommand identities the rewriter currently branches on.
/// Subcommands not enumerated here use the inherited general semantic for every
/// flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClanSubcommandKind {
    Check,
    Maxwd,
    Ipsyn,
    Dss,
    Mortable,
    Script,
    Uniq,
    Mlu,
    Mlt,
    Sugar,
    Keymap,
    Makemod,
    Lines,
    Ort,
    Fixbullets,
    Combo,
    Freq,
    Vocd,
    Dist,
    Kwal,
    Wdsize,
    Freqpos,
    Cooccur,
    Lowcase,
    Combtier,
    Chains,
    Modrep,
    Trnfix,
    Gem,
    Gemfreq,
    Chstring,
    Chip,
    Flo,
    Wdlen,
    Eval,
    EvalD,
    Timedur,
    Dates,
    Flucalc,
    Kideval,
    Rely,
    Chat2elan,
    Lab2chat,
    Other,
}

impl ClanSubcommandKind {
    fn detect(args: &[String]) -> Self {
        // The CLAN subcommand is always the first non-flag token in
        // args after position 0 (typically index 1, but a leading
        // global flag can push it back). Scan from left to right for
        // the first known subcommand name.
        for arg in args {
            match arg.as_str() {
                "check" => return Self::Check,
                "maxwd" => return Self::Maxwd,
                "ipsyn" => return Self::Ipsyn,
                "dss" => return Self::Dss,
                "mortable" => return Self::Mortable,
                "script" => return Self::Script,
                "uniq" => return Self::Uniq,
                "mlu" => return Self::Mlu,
                "mlt" => return Self::Mlt,
                "sugar" => return Self::Sugar,
                "keymap" => return Self::Keymap,
                "makemod" => return Self::Makemod,
                "lines" => return Self::Lines,
                "ort" => return Self::Ort,
                "fixbullets" => return Self::Fixbullets,
                "combo" => return Self::Combo,
                "freq" => return Self::Freq,
                "vocd" => return Self::Vocd,
                "dist" => return Self::Dist,
                "kwal" => return Self::Kwal,
                "wdsize" => return Self::Wdsize,
                "freqpos" => return Self::Freqpos,
                "cooccur" => return Self::Cooccur,
                "lowcase" => return Self::Lowcase,
                "combtier" => return Self::Combtier,
                "chains" => return Self::Chains,
                "modrep" => return Self::Modrep,
                "trnfix" => return Self::Trnfix,
                "gem" => return Self::Gem,
                "gemfreq" => return Self::Gemfreq,
                "chstring" => return Self::Chstring,
                "chip" => return Self::Chip,
                "flo" => return Self::Flo,
                "wdlen" => return Self::Wdlen,
                "eval" => return Self::Eval,
                "eval-d" => return Self::EvalD,
                "timedur" => return Self::Timedur,
                "dates" => return Self::Dates,
                "flucalc" => return Self::Flucalc,
                "kideval" => return Self::Kideval,
                "rely" => return Self::Rely,
                "chat2elan" => return Self::Chat2elan,
                "lab2chat" => return Self::Lab2chat,
                _ => {}
            }
        }
        Self::Other
    }
}

/// Rewrite CLAN-style `+flag`/`-flag` arguments into modern `--flag`
/// equivalents.
///
/// The function scans `args` for patterns like `+t*CHI`, `+s"word"`,
/// `+z25-125`, etc., and replaces them with `--speaker CHI`,
/// `--include-word word`, `--range 25-125`, etc. Unrecognized arguments pass
/// through unchanged.
///
/// This is intentionally applied to the full argument list (including the
/// binary name and subcommand tokens). Subcommand names like `analyze`, `freq`,
/// etc. never start with `+` or `-` followed by a CLAN flag letter, so they
/// are never matched.
///
/// The rewriter is context-aware for the `check` subcommand: `+g1`-`+g5` are
/// CHECK generic options (not gem labels), so they are rewritten to
/// `--check-target`, `--check-id`, `--check-unused` etc. For all other
/// subcommands, `+g` is gem filtering as usual.
///
/// Handles the legacy `+flag` / `-flag` syntax → `--flag` translation, plus
/// the per-subcommand aliasing required when a single CLAN flag flips chatter's
/// output into a sibling subcommand. Used by the `chatter clan <cmd>`
/// dispatcher before clap parses the rewritten argv.
pub fn rewrite_clan_args(args: &[String]) -> Vec<String> {
    // Pre-pass: handle "CLAN unifies, chatter splits" subcommand
    // aliases where a single CLAN flag flips the output format in
    // a way that chatter exposes as a sibling subcommand. Swaps
    // the subcommand token and drops the trigger flag so the
    // regular per-arg rewriter sees a canonical args list.
    let resolved = resolve_subcommand_alias(args);
    let args = resolved.as_ref();

    let subcommand = ClanSubcommandKind::detect(args);

    let mut out = Vec::with_capacity(args.len());
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        // Only attempt rewriting on args starting with + or - that look like
        // CLAN flags (second char is a known flag letter, not a digit or '-').
        if let Some(rewritten) = try_rewrite_clan_flag(arg, subcommand) {
            out.extend(rewritten);
            i += 1;
            continue;
        }

        // Pass through unchanged.
        out.push(arg.clone());
        i += 1;
    }

    out
}

/// Handle "CLAN unifies, chatter splits" subcommand aliases by swapping the
/// subcommand token and removing the trigger flag. Returns the input borrowed
/// when no alias applies (no allocation for the common case) and an owned
/// `Vec` otherwise.
///
/// Current aliases:
/// - `chat2srt +v` → `chat2vtt` (drop `+v`). Per
///   `OSX-CLAN/src/clan/chat2srt.cpp:108` `case 'v'`, CLAN's chat2srt flips its
///   output to WebVTT when `+v` is present. chatter splits SRT and WebVTT into
///   distinct subcommands (`chat2srt` and `chat2vtt`), each with its own clap
///   surface.
fn resolve_subcommand_alias(args: &[String]) -> std::borrow::Cow<'_, [String]> {
    use std::borrow::Cow;

    // chat2srt + +v → chat2vtt (drop +v).
    let chat2srt_idx = args.iter().position(|a| a == "chat2srt");
    let v_idx = args.iter().position(|a| a == "+v");
    if let (Some(sc_idx), Some(flag_idx)) = (chat2srt_idx, v_idx) {
        // Both must be present; either ordering is allowed (the
        // subcommand always comes before the flag in well-formed
        // CLAN invocations, but the check is order-agnostic).
        let mut owned: Vec<String> = args.to_vec();
        owned[sc_idx] = "chat2vtt".to_string();
        owned.remove(flag_idx);
        return Cow::Owned(owned);
    }

    Cow::Borrowed(args)
}

#[cfg(test)]
mod tests;
