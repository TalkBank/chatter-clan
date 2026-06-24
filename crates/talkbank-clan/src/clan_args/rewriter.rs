use super::ClanSubcommandKind;

mod d_flag;
mod helpers;

use d_flag::try_rewrite_d_flag;
// The value-shaping helpers live in the `helpers` submodule; the dispatch arms
// below call them by their bare names.
use helpers::{
    rest_is_digits, restore_marker_token, rewrite_check_error, rewrite_check_generic,
    rewrite_context_window, rewrite_gem, rewrite_range, rewrite_search_word,
    rewrite_subcommand_value_flag, rewrite_tier_speaker, rewrite_wdsize_length_filter,
};

pub(super) fn try_rewrite_clan_flag(
    arg: &str,
    subcommand: ClanSubcommandKind,
) -> Option<Vec<String>> {
    use ClanSubcommandKind::*;
    let bytes = arg.as_bytes();
    if bytes.len() < 2 {
        return None;
    }

    let polarity = bytes[0];
    if polarity != b'+' && polarity != b'-' {
        return None;
    }

    let flag_char = bytes[1];
    let rest = &arg[2..];

    match (polarity, flag_char) {
        // +t*CHI / -t*CHI, speaker include/exclude
        // MLU / MLT `-t%mor`, CLAN's documented escape hatch:
        // when `%mor` is present but the user wants word-mode
        // counts, `-t%mor` implies `--words`. Without this special-
        // case, the rewriter would emit `--exclude-tier mor` which
        // MLU's/MLT's clap doesn't accept. Scoped to `%mor` only;
        // other `-t%X` values fall through to the generic
        // exclude-tier path.
        (b'-', b't') if matches!(subcommand, Mlu | Mlt) && rest == "%mor" => {
            Some(vec!["--words".into()])
        }

        // COMBTIER `+tS` (bare-prefix form), tier label to combine,
        // per `OSX-CLAN/src/clan/combtier.cpp` usage
        // ("+tS: Combine all tiers S into one tier."). COMBTIER
        // overloads `+tS` away from the analysis-command convention
        // (`+tCHI` = speaker filter); route the bareword form to
        // `--tier S` instead of letting `rewrite_tier_speaker`
        // produce `--speaker S`. The `+t%X` form is handled by
        // `rewrite_tier_speaker`'s `%` branch, which also emits
        // `--tier X`, so only the bare-prefix case needs intercept.
        (b'+', b't')
            if subcommand == Combtier
                && !rest.is_empty()
                && !matches!(rest.as_bytes()[0], b'*' | b'%' | b'@' | b'#') =>
        {
            rewrite_subcommand_value_flag(rest, "--tier")
        }

        // LAB2CHAT `+tN` (digit-only N) is "Movie segment start
        // time offset" per
        // `book/src/clan-reference/commands/lab2chat.md:69`.
        // chatter does not implement movie-segment offsets yet.
        // Without this arm, `+t3` falls through to
        // `rewrite_tier_speaker` and becomes `--speaker 3`
        // (the default branch treats non-sigil-prefixed values as
        // implicit speaker codes), silently mis-routing to LAB-CHAT
        // speaker labeling. Pass-through (None) lets clap reject
        // the digit-only form honestly. Letter forms like
        // `lab2chat +tCHI` are not lab2chat semantics either but
        // are out of scope here; they continue to fall through.
        (b'+', b't') if subcommand == Lab2chat && rest_is_digits(rest) => None,

        // MLU `+t*` (bare star, NOTHING after it) is a hidden alias for `+o3`:
        // CLAN sets `mlu_isCombineSpeakers` when the `+t` argument is exactly
        // `*` with EOS following (mlu.cpp:735, `*f == '*' && *(f+1) == EOS`).
        // It must combine speakers, NOT select a speaker. `+t*CHI` (star then a
        // code) has `rest == "*CHI"` and falls through to `rewrite_tier_speaker`
        // (-> `--speaker CHI`), so only the exactly-`*` form is intercepted.
        (b'+', b't') if subcommand == Mlu && rest == "*" => Some(vec!["--combine-speakers".into()]),

        (b'+', b't') | (b'-', b't') => rewrite_tier_speaker(polarity, rest),

        // MLU / MLT `-bw`, switch the counting unit from morphemes
        // (`%mor`-based, the default) to words (main-tier-based).
        // Without this arm, `-bw` falls through to clap, which parses
        // it as a `-b -w` short-flag pair and errors on the unknown
        // `-b`. Scoped to Mlu | Mlt; other commands don't share the
        // morphemes-vs-words counting axis.
        (b'-', b'b') if matches!(subcommand, Mlu | Mlt) && rest == "w" => {
            Some(vec!["--words".into()])
        }

        // MLU / MLT `-b<x>` for any x other than `w`, fail closed. The unix CLAN
        // binary accepts only `-bw` (`-bc` characters is `#ifndef UNX`,
        // mlu.cpp:678-680) and otherwise prints "Please specify w - words after
        // -b option." (mlu.cpp:686). Returning None here surfaces a clean
        // "unrecognized option(s): '-bc'" instead of letting clap mis-split
        // `-bc` into a misleading bare `-b`. The message is chatter's own (a
        // documented divergence from CLAN's wording); both reject.
        (b'-', b'b') if matches!(subcommand, Mlu | Mlt) => None,

        // +s"word" / +sword / -s"word" / -sword, word include/exclude
        // `+sF` under SCRIPT is the template-file argument
        // (`--template F`); SCRIPT's `+s` is the only CLAN command
        // where the value is interpreted as a filesystem path
        // rather than a search keyword.
        (b'+', b's') if subcommand == Script => rewrite_subcommand_value_flag(rest, "--template"),
        // COMBO's `+s@FILE` / `-s@FILE` load boolean search
        // expressions from disk (one per line). Routed to
        // dedicated `--search-file` / `--exclude-search-file`
        // because COMBO's per-line value is a `SearchExpr`, not
        // a per-word pattern, must precede the generic
        // `+s@`/`-s@` word-file arms below.
        (b'+', b's') if subcommand == Combo && rest.starts_with('@') => {
            rewrite_subcommand_value_flag(&rest[1..], "--search-file")
        }
        (b'-', b's') if subcommand == Combo && rest.starts_with('@') => {
            rewrite_subcommand_value_flag(&rest[1..], "--exclude-search-file")
        }
        // COMBO's `+sS` / `-sS` are compound boolean expressions
        // (e.g. `want+cookie`, `want,milk`), distinct from the
        // general per-word `+s` include/exclude, route to
        // `--search` / `--exclude-search`.
        (b'+', b's') if subcommand == Combo => rewrite_subcommand_value_flag(rest, "--search"),
        (b'-', b's') if subcommand == Combo => {
            rewrite_subcommand_value_flag(rest, "--exclude-search")
        }
        // +s@FILE / -s@FILE, load word-list from file (CLAN's
        // `cutt.cpp::rdexclf`). Ordered after the SCRIPT and
        // COMBO command-specific arms because those commands'
        // `+s` value isn't a per-word pattern and the `@FILE`
        // semantic differs.
        (b'+', b's') if rest.starts_with('@') => {
            rewrite_subcommand_value_flag(&rest[1..], "--include-word-file")
        }
        (b'-', b's') if rest.starts_with('@') => {
            rewrite_subcommand_value_flag(&rest[1..], "--exclude-word-file")
        }

        // MLU `+s`/`-s` on the untranscribed markers `xxx`/`yyy`/`www` is NOT
        // the generic per-word filter: CLAN's MLU `+s` handler (mlu.cpp:744-786)
        // intercepts these to control the default whole-utterance exclusion.
        //   * `+sxxx` / `+syyy` RE-INCLUDE the xxx/yyy utterances (the marker is
        //     still kept out of the morpheme count); map to `--include-xxx` /
        //     `--include-yyy`.
        //   * `-sxxx` / `-syyy` ERROR in CLAN ("Excluding xxx is not allowed",
        //     mlu.cpp:768) -- you cannot exclude what is already excluded by
        //     default. `+swww` / `-swww` ERROR (mlu.cpp:784, "www ... not
        //     allowed"): www can never be re-included (manual §7.21 pt2).
        // The reject forms map to a self-documenting NON-EXISTENT long flag so
        // clap fails closed with a message that names the reason. (A plain `None`
        // would NOT fail closed for the `-s` forms: `--speaker` owns the `-s`
        // short flag, so `-sxxx` would be silently misread as `--speaker xxx`.)
        // Scoped to MLU; MLT's `+s` handling is a separate, untouched command.
        (b'+', b's') if subcommand == Mlu && rest.eq_ignore_ascii_case("xxx") => {
            Some(vec!["--include-xxx".into()])
        }
        (b'+', b's') if subcommand == Mlu && rest.eq_ignore_ascii_case("yyy") => {
            Some(vec!["--include-yyy".into()])
        }
        (b'-', b's') if subcommand == Mlu && rest.eq_ignore_ascii_case("xxx") => Some(vec![
            "--xxx-is-excluded-by-default-and-cannot-be-excluded".into(),
        ]),
        (b'-', b's') if subcommand == Mlu && rest.eq_ignore_ascii_case("yyy") => Some(vec![
            "--yyy-is-excluded-by-default-and-cannot-be-excluded".into(),
        ]),
        (b'+', b's') | (b'-', b's') if subcommand == Mlu && rest.eq_ignore_ascii_case("www") => {
            Some(vec!["--www-cannot-be-included-or-excluded".into()])
        }

        (b'+', b's') | (b'-', b's') => rewrite_search_word(polarity, rest),

        // +g: command-dependent.
        //   * CHECK       → generic options (`+g1`..`+g5` map to
        //                   `--check-target` / `--check-id` / etc.)
        //   * MLU / MLT   → solo-word elision (drop utterances
        //                   consisting solely of word S):
        //                   `+gS` → `--exclude-solo-word S`.
        //                   CLAN's MLU/MLT `getflag()` intercepts `+g`
        //                   before the inherited gem semantic; chatter
        //                   matches by routing here. Documented as the
        //                   "+g overload" pattern in the parity audit.
        //   * other       → gem-segment filter (`--gem S`).
        (b'+', b'g') if subcommand == Check => rewrite_check_generic(polarity, rest),
        // MLU/MLT `+g@F` loads the solo-word exclusion list from a
        // file (same idiom as `+s@F` → `--include-word-file`).
        // Must precede the per-word `+gS` arm so the `@`-prefix is
        // intercepted before being treated as a literal pattern.
        (b'+', b'g') if matches!(subcommand, Mlu | Mlt) && rest.starts_with('@') => {
            rewrite_subcommand_value_flag(&rest[1..], "--exclude-solo-word-file")
        }
        (b'+', b'g') if matches!(subcommand, Mlu | Mlt) => {
            rewrite_subcommand_value_flag(rest, "--exclude-solo-word")
        }
        // COMBO `+gN` search-mode switches (CLAN's `+g1..+g7`). Most
        // are documented gaps; the ones below are wired:
        //   * `+g3`, only the first matching expression per
        //     utterance → `--first-match-only`.
        //   * `+g4`, exclude utterance delimiters from search.
        //     chatter's COMBO operates on `countable_words`, which
        //     never returns terminators/separators, so `+g4` is
        //     the chatter default. No-op accept.
        //   * `+g5`, use `+` (or `^`) as AND operator. chatter's
        //     `+` is already AND by default, so `+g5` is a no-op
        //     accept; rewriter consumes the flag (`Some(vec![])`)
        //     so clap never sees it.
        //   * `+g7`, deduplicate repeated word matches within an
        //     utterance → `--dedupe-matches`.
        (b'+', b'g') if subcommand == Combo && rest == "3" => {
            Some(vec!["--first-match-only".into()])
        }
        (b'+', b'g') if subcommand == Combo && rest == "4" => Some(Vec::new()),
        (b'+', b'g') if subcommand == Combo && rest == "5" => Some(Vec::new()),
        (b'+', b'g') if subcommand == Combo && rest == "7" => Some(vec!["--dedupe-matches".into()]),
        // GEMFREQ `-wS`, CLAN's `gemfreq.cpp:296` literally
        // rewrites the flag char from `w` to `s` (`case 'w':
        // *(f-1) = 's'`) then calls `maingetflag`, so `-wS` is the
        // standard exclude-word semantic. Delegate to
        // `rewrite_search_word` to share polarity routing and
        // quote-stripping with the regular `-s`/`+s` path.
        // chatter's clap `-w` short is `--include-word` (OPPOSITE
        // polarity), so without this arm `-wS` would silently
        // mis-route to include-word.
        (b'-', b'w') if subcommand == Gemfreq && !rest.is_empty() => {
            rewrite_search_word(b'-', rest)
        }

        // MAXWD `+gN` (N in 1..=3) is the utterance-mode metric
        // selector ("find longest utterance instead of longest
        // word; N selects metric: 1=morph, 2=word, 3=char") per
        // `book/src/clan-reference/commands/maxwd.md:52`. chatter
        // does not implement utterance-mode yet. Without this
        // arm, the token falls through to `rewrite_gem` (next in
        // the chain) and becomes `--gem N`, silently mis-routing
        // to a literal gem-name filter. Pass-through (None) lets
        // clap reject `+gN` honestly. Maxwd's `+gX` (non-digit X)
        // gem filter is left for `rewrite_gem` to handle.
        (b'+', b'g') if subcommand == Maxwd && rest_is_digits(rest) => None,

        // COMBO `+g1` / `+g2` / `+g6` are search-mode switches
        // (string-oriented whole-tier / single-word search;
        // include tier code name in search) that chatter does not
        // yet implement, audit page status "Missing" per
        // `book/src/clan-reference/commands/combo.md:51-52,56`.
        // Without these arms, the tokens fall through to the
        // generic `+g` → `rewrite_gem` arm below and silently
        // re-route to `--gem 1` / `--gem 2` / `--gem 6` (literal
        // gem names), losing the user's intent. Pass-through
        // (None) lets clap reject the literal token honestly so
        // the user knows the flag is unimplemented rather than
        // running with wrong-but-silent behavior.
        (b'+', b'g') if subcommand == Combo && (rest == "1" || rest == "2" || rest == "6") => None,
        // DIST's bare `+g` is a counting policy ("one occurrence
        // per turn"), distinct from the inherited `+gLABEL` gem
        // filter. Only the no-rest form routes here; `+gLABEL`
        // falls through to the gem branch.
        (b'+', b'g') if subcommand == Dist && rest.is_empty() => {
            Some(vec!["--once-per-turn".into()])
        }
        // FREQ has NO `+g` gem flag: CLAN's `freq.cpp` getflag rejects `+g`/`-g`
        // ("Invalid option"); gem-limiting in CLAN is the separate GEM program
        // (`gem +sX +d +f` -> freq). Route to a reject sentinel so chatter
        // errors the same way instead of squatting the slot on `--gem` (the
        // chatter-only gem convenience stays reachable directly). (CLAN MLU/MLT
        // DO accept `+g`, handled above as `--exclude-solo-word`.)
        (b'+', b'g') | (b'-', b'g') if subcommand == Freq => {
            Some(vec!["--reject-clan-gem".into(), rest.to_string()])
        }
        (b'+', b'g') | (b'-', b'g') => rewrite_gem(polarity, rest),

        // +aN under SUGAR sets the minimum-utterance threshold
        // (CLAN docs: "set minimal utterances number limit to N
        // utterances (default: 50 minimal limit)"). Routes to
        // `--min-utterances N`. SUGAR is the only command with
        // this `+aN` semantic; other commands either don't use
        // `+a` or use it as a different flag.
        (b'+', b'a') if subcommand == Sugar => {
            rewrite_subcommand_value_flag(rest, "--min-utterances")
        }

        // `+a` under MAKEMOD is a no-value boolean, print all
        // alternative pronunciations (default: first only). Routes
        // to `--all-alternatives`.
        (b'+', b'a') if rest.is_empty() && subcommand == Makemod => {
            Some(vec!["--all-alternatives".into()])
        }

        // `+n` under LINES is a no-value boolean, remove existing
        // line numbers (default: add them). Routes to `--remove`.
        (b'+', b'n') if rest.is_empty() && subcommand == Lines => Some(vec!["--remove".into()]),

        // `+cF` under ORT specifies the homons-table dictionary.
        // Maps `+ceng.cut` → `--dictionary eng.cut`.
        (b'+', b'c') if !rest.is_empty() && subcommand == Ort => {
            rewrite_subcommand_value_flag(rest, "--dictionary")
        }

        // +bS under KEYMAP sets a key-code to track. Routes to
        // `--keyword S` (repeatable). KEYMAP's `+b` semantic is
        // distinct from FREQ's `+bN` (MATTR frame size), MLU's `-bw`
        // (word-mode toggle), WDLEN/MAXWD's `+bS`/`-bS` (morpheme
        // delimiters); those are documented audit gaps tracked
        // under Phase 1.7 follow-ups and remain unrewritten.
        // `+b@F` (key-codes-from-file) is also unrewritten today.
        (b'+', b'b') if subcommand == Keymap && !rest.starts_with('@') => {
            rewrite_subcommand_value_flag(rest, "--keyword")
        }

        // FREQ `+bN`: frame size for the Moving-Average TTR (MATTR). CLAN's
        // `+b` is command-specific (CHIP speaker code, DIST `+b:`, editor
        // checkpoint), so this rewrite is guarded to FREQ. The value (including
        // an empty or non-numeric one) is passed through to `--mattr`, where
        // `FrameSize` parsing rejects it exactly as CLAN does (freq.cpp:773-780).
        (b'+', b'b') if subcommand == Freq => Some(vec!["--mattr".into(), rest.to_string()]),

        // +z25-125, utterance range
        (b'+', b'z') => rewrite_range(rest),

        // +x utterance-length filter (FREQ). The count form `+x C N U` ->
        // `--utterance-length` (its value parser handles the w/c/m units). The
        // `+xS` content-INCLUDE form (no leading comparison) splits two ways:
        //  - the unintelligible-marker restores `+xxxx`/`+xyyy`/`+xwww` (and the
        //    `xx`/`yy`/`ww` aliases, `cutt.cpp:9890-9896`) -> the canonical
        //    `--utterance-length-restore <marker>`, re-including `xxx`/`yyy`/`www`
        //    in the length count.
        //  - any other `+xWORD` (general word include) or the `+x@FILE` form ->
        //    `--utterance-length WORD`, where the value parser REJECTS the missing
        //    comparison (`InvalidFormat`), so it fails-closed at parse rather than
        //    silently no-op (those forms are deferred).
        // `-xS` is content-EXCLUDE -> `--utterance-length-exclude` (manual 6405:
        // removes word S from the length count, NOT from FREQ's output). Guard
        // `== Freq`: per-command depth-first rollout (MAXWD overloads `+xN`).
        (b'+', b'x') if subcommand == Freq => match restore_marker_token(rest) {
            Some(marker) => Some(vec!["--utterance-length-restore".into(), marker.into()]),
            None => Some(vec!["--utterance-length".into(), rest.to_string()]),
        },
        // `-x@FILE`: load the exclude-from-count word list from a file
        // (`rdexclfUttLen('e', …)`, `cutt.cpp:5384`), the file analog of `-xWORD`
        // and the same idiom as `-s@F` → `--exclude-word-file`.
        (b'-', b'x') if subcommand == Freq && rest.starts_with('@') => {
            rewrite_subcommand_value_flag(&rest[1..], "--utterance-length-exclude-file")
        }
        (b'-', b'x') if subcommand == Freq => {
            Some(vec!["--utterance-length-exclude".into(), rest.to_string()])
        }

        // +r1/+r2/+r3 (and bare +r = +r1): the omitted-material parenthesis mode
        // (`Parans`, cutt.cpp:9530-9583; manual section 14.5). +r1 (DEFAULT)
        // removes the parens but keeps the omitted letters (`bein(g)` -> `being`);
        // +r2 keeps the parens; +r3 removes the omitted letters. FREQ-scoped per
        // the depth-first rollout (the `Parans` default is CLAN-wide, but only
        // FREQ carries `--parenthesis-mode` so far).
        (b'+', b'r') if subcommand == Freq && (rest.is_empty() || rest == "1") => {
            Some(vec!["--parenthesis-mode".into(), "remove-parens".into()])
        }
        (b'+', b'r') if subcommand == Freq && rest == "2" => {
            Some(vec!["--parenthesis-mode".into(), "keep-parens".into()])
        }
        (b'+', b'r') if subcommand == Freq && rest == "3" => {
            Some(vec!["--parenthesis-mode".into(), "remove-material".into()])
        }
        // +r5: `[: text]` replacement mode (`R5`, cutt.cpp:9549-9553). Default
        // counts the replacement; +r5 counts the original. `rest == "5"` exactly,
        // so +r50 (the distinct `R5_1` variant) is NOT matched and stays Missing.
        (b'+', b'r') if subcommand == Freq && rest == "5" => {
            Some(vec!["--replacement-mode".into(), "original".into()])
        }

        // +r6, include retracings
        (b'+', b'r') if rest == "6" => Some(vec!["--include-retracings".into()]),

        // MLU `+r1`/`+r2`/`+r3`/`+r4`/`+r5`/`+r7` are FAITHFUL NO-OPS: they tune
        // the word FORM (parentheses / clitic / replacement / prosody), but MLU
        // counts morphemes (from `%mor`) or words (`-bw`) and never displays a
        // form, so they change nothing (probe-confirmed: morpheme and word totals
        // identical with/without on mor-gra.cha and 000829.cha). MLU enables
        // `R_OPTION` (`option_flags[MLU]`, cutt.cpp:8674), so CLAN accepts them;
        // their effect is `+s`-search only, like `+k`. `+r6` (above) is the one
        // `+r` with a real MLU effect (retraced material counted). Consume them.
        (b'+', b'r') if subcommand == Mlu && matches!(rest, "1" | "2" | "3" | "4" | "5" | "7") => {
            Some(vec![])
        }

        // +r7: keep within-word prosodic `:`/`^`/`~` (`R7…`, cutt.cpp:9569-9574).
        // Default strips them; +r7 keeps them. FREQ-scoped per the depth-first
        // rollout (only FREQ carries `--prosody-mode` so far).
        (b'+', b'r') if subcommand == Freq && rest == "7" => {
            Some(vec!["--prosody-mode".into(), "keep".into()])
        }

        // +re, recurse subdirectories. chatter recurses by default
        // when given a directory argument, so the flag is a global
        // no-op (same shape as `+u` on non-CHECK commands above).
        // Drop the token rather than passing it through to clap,
        // which would land it in the path-arg list and emit a
        // confusing `Warning: "+re" is not a file or directory`.
        (b'+', b'r') if rest == "e" => Some(vec![]),

        // FREQ `+pS` (cutt.cpp:9798-9818; manual cutt.cpp:9204 "add S to word
        // delimiters"): the characters of `S` become extra word delimiters, so a
        // counted word is split at them and each piece counts on its own (`+p_`
        // breaks `choo_choo` into two `choo`). Maps to chatter's
        // `--word-delimiters S` (a chatter-only flag name carrying the CLAN slot;
        // faithfulness rule). The empty form `+p` passes an empty value, which
        // the FREQ dispatch rejects with CLAN's "specify word delimiter
        // characters" error (cutt.cpp:9802). FREQ enables `P_OPTION`
        // (`option_flags[FREQ]` includes `ALL_OPTIONS`, cutt.cpp:8648).
        (b'+', b'p') if subcommand == Freq => Some(vec!["--word-delimiters".into(), rest.into()]),

        // MLU `+pS` is a FAITHFUL NO-OP: MLU enables `P_OPTION` so CLAN accepts
        // it, but the extra word delimiters never reach MLU's morpheme/word count
        // (probe-confirmed: morphemes 196 and words 189 unchanged with `+p_` on
        // 000829.cha, where `choo_choo` stays one word and `%mor` already has
        // `choochoo`). Unlike FREQ, MLU does not re-tokenize on `+p`. Consume it.
        (b'+', b'p') if subcommand == Mlu => Some(vec![]),

        // +u: For CHECK, +u means validate UD features; for other commands, merge speakers (no-op)
        (b'+', b'u') if rest.is_empty() && subcommand == Check => Some(vec!["--check-ud".into()]),

        // FLUCALC `+u` enables per-utterance output in CLAN
        // (`flucalc.cpp:778-781`, `isUttList = TRUE`). chatter has
        // only `--per-file` (file granularity), NOT per-utterance,
        // audit page status "Partial". Pass through (None) instead
        // of letting the generic `+u` arm below silently drop it;
        // clap rejects the literal `+u` honestly. Per-FLUCALC arm
        // placed BEFORE the generic so flucalc's behavior diverges
        // from the merge-speakers no-op of other commands.
        (b'+', b'u') if rest.is_empty() && subcommand == Flucalc => None,

        (b'+', b'u') if rest.is_empty() => Some(vec![]),

        // COOCCUR `+nN` sets the cluster size (number of adjacent
        // words counted as a unit). Default 2 = bigrams; +n3 =
        // trigrams; etc. Rejected with no rest (just `+n`) because
        // CLAN requires the N value.
        (b'+', b'n') if subcommand == Cooccur && rest.parse::<u8>().is_ok() => {
            Some(vec!["--cluster-size".into(), rest.to_string()])
        }

        // GEMFREQ `+o`, bare no-value flag that turns on sort-by-
        // descending-frequency in CLAN (`OSX-CLAN/src/clan/gemfreq.cpp:260`:
        // `isSort = TRUE; no_arg_option(f)`). chatter's `gemfreq`
        // (a compatibility alias that adapts to `freq --gem`) already
        // sorts by descending frequency by default, so `+o` is
        // semantically a no-op. Drop the flag so it doesn't fall
        // through to the positional `<PATH>` slot.
        (b'+', b'o') if rest.is_empty() && subcommand == Gemfreq => Some(vec![]),

        // CHSTRING `+b`, bare-only "work only on text right of the
        // colon (CHAT format)" per
        // `OSX-CLAN/src/clan/chstring.cpp:1120` (`case 'b':
        // lineonly = TRUE; no_arg_option(f)`). chatter's `chstring`
        // already mutates only main-tier word content (never
        // speaker codes or dependent-tier text), so `+b` is
        // semantically a no-op. Drop the flag, without this arm
        // clap consumes the bare `+b` token as the positional
        // `<PATH>` slot.
        (b'+', b'b') if rest.is_empty() && subcommand == Chstring => Some(vec![]),

        // CHSTRING `+lx`, "do not show the list of changes" per
        // `OSX-CLAN/src/clan/chstring.cpp:1108-1111` (`case 'l': if
        // (*f == 'x') DispChanges = FALSE`). chatter never prints a
        // changes-list (silent by design), so `+lx` is semantically
        // a no-op. Drop the specific `lx` form; bare `+l` (header-
        // only mode) is genuinely unimplemented and falls through
        // to clap as before.
        (b'+', b'l') if rest == "x" && subcommand == Chstring => Some(vec![]),

        // CHSTRING `-w`, bare-only "string-oriented search and
        // replacement" per `OSX-CLAN/src/clan/chstring.cpp:1145-1147`
        // (`case 'w': if (*f == EOS) stringOriented = 1`). chatter's
        // word-leaf replacement is already string-oriented by
        // default, so `-w` is semantically a no-op. CLAN's `-w1`
        // (`stringOriented = 2`) is not documented in the chstring
        // audit page, so the specific `1` form is left to fall
        // through. Must stay BEFORE the generic `-w` context-window
        // arm below so the Chstring form is not mis-routed.
        (b'-', b'w') if rest.is_empty() && subcommand == Chstring => Some(vec![]),

        // `+d`/`+dN` is the most command-overloaded CLAN flag; its full
        // per-subcommand routing lives in `d_flag::try_rewrite_d_flag`
        // (extracted verbatim from this match). There is no generic `+d`
        // fallback: an unmatched subcommand returns `None`, leaving the
        // literal token for clap to reject, exactly as before. The five
        // non-`+d` arms above (`+n`/`+o`/`+b`/`+l`/`-w` for
        // Cooccur/Gemfreq/Chstring) were previously interleaved among the
        // `+d` arms; they are disjoint by flag char and subcommand, so
        // pulling them ahead of this delegation changes nothing.
        (b'+', b'd') => try_rewrite_d_flag(subcommand, rest),

        // +k, case sensitive
        (b'+', b'k') if rest.is_empty() => Some(vec!["--case-sensitive".into()]),

        // +fEXT, output extension
        (b'+', b'f') if !rest.is_empty() => Some(vec!["--output-ext".into(), rest.to_string()]),

        // WDSIZE `+w[>|<|=]N`, length-bounded histogram. Intercept
        // before the general `+wN` context-window arm: presence of
        // a leading comparator (`>`, `<`, or `=`) disambiguates
        // the length-filter form from the inherited context-window
        // form (`+w3` etc.). Match-guard binds the parsed result
        // so we parse `rest` exactly once.
        (b'+', b'w')
            if subcommand == Wdsize
                && let Some(args) = rewrite_wdsize_length_filter(rest) =>
        {
            Some(args)
        }

        // +wN / -wN, context window
        (b'+', b'w') => rewrite_context_window(polarity, rest),
        (b'-', b'w') => rewrite_context_window(polarity, rest),

        // `+cN` is subcommand-dependent:
        //   * CHECK       → bullet check level (`--bullets N`)
        //   * MAXWD       → number of longest items to display (`--limit N`)
        //   * IPSYN / DSS → max utterances to analyse (`--max-utterances N`)
        //   * other       → no rewrite today; FREQ's `+c0..7` (capitalised-
        //                   word and multi-word search variants) and VOCD's
        //                   `+c` (capitalised-only) are documented gaps,
        //                   tracked under Phase 1.7 follow-ups.
        (b'+', b'c') if subcommand == Maxwd => rewrite_subcommand_value_flag(rest, "--limit"),
        // MAXWD `+a`, restrict to words whose length is unique
        // within a speaker's lexicon (CLAN: "Consider ONLY unique-
        // length words"). No CLAN `+aN` variant exists.
        (b'+', b'a') if subcommand == Maxwd && rest.is_empty() => {
            Some(vec!["--unique-length-only".into()])
        }
        // MAXWD `+xN`, drop words of character length N from
        // output. Repeatable in CLAN argv (`+x5 +x7`); each rewrite
        // emits an `--exclude-length N` argv pair. The numeric
        // guard ensures non-numeric `+x<S>` (other-command futures)
        // doesn't accidentally route here.
        (b'+', b'x') if subcommand == Maxwd && rest.parse::<usize>().is_ok() => {
            rewrite_subcommand_value_flag(rest, "--exclude-length")
        }
        // KWAL `+b`, strict-match: keyword must be the *only*
        // item on the tier (single-word utterance). No CLAN `+bS`
        // variant exists for KWAL.
        (b'+', b'b') if subcommand == Kwal && rest.is_empty() => {
            Some(vec!["--strict-match".into()])
        }
        (b'+', b'c') if matches!(subcommand, Ipsyn | Dss) => {
            rewrite_subcommand_value_flag(rest, "--max-utterances")
        }
        (b'+', b'c') if subcommand == Check => rewrite_subcommand_value_flag(rest, "--bullets"),
        // FREQ / VOCD `+c` / `+c0` / `+c1`, capitalization-mode
        // filter. Both commands share the `--capitalization` enum-
        // valued clap field (`initial` or `mid`). CLAN spellings:
        //   * `+c` / `+c0` → `--capitalization initial`
        //   * `+c1`        → `--capitalization mid`
        // VOCD's manual lists only `+c`; FREQ extends to `+c1`.
        (b'+', b'c') if matches!(subcommand, Freq | Vocd) && (rest.is_empty() || rest == "0") => {
            Some(vec!["--capitalization".into(), "initial".into()])
        }
        (b'+', b'c') if matches!(subcommand, Freq | Vocd) && rest == "1" => {
            Some(vec!["--capitalization".into(), "mid".into()])
        }
        // FREQ `+c3` (`anyMultiOrder`, freq.cpp:792): relax multi-word `+s`
        // matching from the default adjacent-in-order sequence to "anywhere and
        // in any order" (manual CLAN.txt:5488). A multi-word-search mode, not a
        // capitalization filter, so it maps to `--multiword-order`.
        (b'+', b'c') if subcommand == Freq && rest == "3" => {
            Some(vec!["--multiword-order".into(), "any".into()])
        }
        // FREQ `+c4` (`onlySpecWsFound`, freq.cpp:794): a multi-word `+s` match
        // only counts when the utterance consists solely of the group (manual
        // CLAN.txt:5490-5491). A multi-word-search mode -> `--multiword-scope`.
        (b'+', b'c') if subcommand == Freq && rest == "4" => {
            Some(vec!["--multiword-scope".into(), "sole".into()])
        }
        // FREQ `+c2` (`capwd == 3`, freq.cpp:432-438): count a word once per
        // matching single-word `+s` pattern (manual CLAN.txt:5485-5486). NOT a
        // multi-word mode -> `--search-multiplicity`.
        (b'+', b'c') if subcommand == Freq && rest == "2" => {
            Some(vec!["--search-multiplicity".into(), "per-pattern".into()])
        }
        // FREQ `+c7` (`isMultiWordsActual`, freq.cpp:800, 2444): for multi-word
        // `+s` groups, display the actual matched words instead of the search
        // pattern (manual CLAN.txt:5498-5500) -> `--multiword-display matched`.
        (b'+', b'c') if subcommand == Freq && rest == "7" => {
            Some(vec!["--multiword-display".into(), "matched".into()])
        }

        // `+lF` is subcommand-dependent:
        //   * IPSYN / DSS → rules file (`--rules F`)
        //   * MORTABLE    → language script file (`--script F`)
        (b'+', b'l') if matches!(subcommand, Ipsyn | Dss) => {
            rewrite_subcommand_value_flag(rest, "--rules")
        }
        (b'+', b'l') if subcommand == Mortable => rewrite_subcommand_value_flag(rest, "--script"),

        // `-o` under UNIQ is the sort-by-frequency switch
        // (`--sort`). UNIQ is the only CLAN command with a meaningful
        // `-o` (other commands' `-o` excludes an extra output tier,
        // which is not yet wired in chatter).
        (b'-', b'o') if rest.is_empty() && subcommand == Uniq => Some(vec!["--sort".into()]),

        // FREQ `+o` / `+o0`, descending-frequency sort. chatter's
        // FREQ result sorts by count descending unconditionally
        // (`crates/talkbank-clan/src/commands/freq.rs` finalize),
        // so the flag is a no-op. Without this arm `+o` survives
        // to clap as a path arg and triggers
        // `Warning: "+o" is not a file or directory`. Match before
        // the `+o1` arm so the `rest.is_empty()` / `rest == "0"`
        // guards take precedence over the catch-all `rest == "1"`
        // check. `+o2` (non-CHAT spreadsheet output) is a separate
        // documented gap, falls through to default.
        (b'+', b'o') if subcommand == Freq && (rest.is_empty() || rest == "0") => {
            Some(vec!["--sort".into(), "frequency".into()])
        }

        // FREQ `+o1`, sort by reverse concordance. `+o` / `+o0`
        // handled above; `+o2` (CLAN `chatmode=0` plain-text line
        // counting, freq.cpp:820-830) is a documented deferral
        // incompatible with chatter's AST model, not handled here.
        (b'+', b'o') if subcommand == Freq && rest == "1" => {
            Some(vec!["--sort".into(), "reverse-concordance".into()])
        }

        // FREQ `+o3` (isCombineSpeakers, freq.cpp:832): pool all speakers into
        // one combined frequency table with no per-speaker header. Maps to the
        // `--combine-speakers` flag.
        (b'+', b'o') if subcommand == Freq && rest == "3" => {
            Some(vec!["--combine-speakers".into()])
        }

        // MLU `+o3` (mlu_isCombineSpeakers, mlu.cpp:721): pool all selected
        // speakers into one `*COMBINED*` MLU result. `+o3` is the ONLY valid
        // MLU `+o` form (any other `+o<x>` is "Invalid argument", mlu.cpp:723),
        // so other values fall through to fail-closed. Maps to the same shared
        // `--combine-speakers` flag (now on MLU's clap surface too).
        (b'+', b'o') if subcommand == Mlu && rest == "3" => Some(vec!["--combine-speakers".into()]),

        // COOCCUR `+o`, descending-frequency sort. chatter's
        // COOCCUR `finalize` step at
        // `crates/talkbank-clan/src/commands/cooccur.rs:292` already
        // sorts by `count` descending, then alphabetically; CLAN's
        // `cooccur.cpp` uses a BST with `larger num_occ goes left`
        // invariant so in-order traversal produces the same
        // descending order. No-op rewrite drops the token.
        (b'+', b'o') if subcommand == Cooccur && rest.is_empty() => Some(vec![]),

        // `+oN` / `-oN` under FIXBULLETS specify a signed time-offset
        // shift in milliseconds (`+o800` adds 800 ms, `-o800`
        // subtracts 800 ms). FIXBULLETS overloads `+o` here away from
        // the general "include extra output tier" semantic; the
        // numeric guard distinguishes the two, `+oS` with a non-
        // numeric `S` (extra tier code) falls through unchanged.
        //
        // Both forms emit `--offset=N` (`=` syntax) rather than two
        // tokens `["--offset", "N"]`. The `=` form is mandatory for
        // the negative case: clap parses a free-standing `-3` as a
        // short-flag attempt and rejects it before reading it as
        // `--offset`'s value. The positive case uses `=` purely for
        // symmetry, `["--offset", "3"]` would also work.
        (b'+', b'o') if subcommand == Fixbullets && rest.parse::<u32>().is_ok() => {
            Some(vec![format!("--offset={rest}")])
        }
        (b'-', b'o') if subcommand == Fixbullets && rest.parse::<u32>().is_ok() => {
            Some(vec![format!("--offset=-{rest}")])
        }
        // CHAT2ELAN `+eEXT`, media-file-name extension per
        // `OSX-CLAN/src/clan/chat2elan.cpp:117` (`case 'e'`).
        // Routes to `--media-extension`. Strips a leading dot:
        // CLAN concatenates the user-supplied suffix verbatim
        // onto the media basename (so users type `+e.wav`),
        // whereas chatter's `--media-extension` auto-prepends `.`
        // and expects the bare form. Must precede the generic
        // `+e` → `--error` arm below.
        (b'+', b'e') if subcommand == Chat2elan && !rest.is_empty() => {
            let ext = rest.strip_prefix('.').unwrap_or(rest);
            Some(vec!["--media-extension".into(), ext.to_string()])
        }

        // +eN, include error / +e, list errors
        (b'+', b'e') => rewrite_check_error(rest),
        // -eN, exclude error
        (b'-', b'e') if !rest.is_empty() => Some(vec!["--exclude-error".into(), rest.to_string()]),

        _ => None,
    }
}
