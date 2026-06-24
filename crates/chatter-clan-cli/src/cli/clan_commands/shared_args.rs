use clap::ValueEnum;

/// CLI capitalization-mode argument for FREQ and VOCD.
///
/// Maps directly to `talkbank_clan::framework::CapitalizationFilter`
/// at the dispatch site; clap deliberately stays unaware of the
/// domain enum so users get a stable kebab-case CLI surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CapitalizationArg {
    /// CLAN `+c` / `+c0`, first character uppercase.
    Initial,
    /// CLAN `+c1`, uppercase letter after position 0.
    Mid,
}

/// CLI sort-mode argument for FREQ.
///
/// Maps to `talkbank_clan::commands::freq::FreqSort` at the dispatch
/// site; clap stays unaware of the domain enum so users get a stable
/// kebab-case CLI surface (`--sort reverse-concordance`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortArg {
    /// CLAN default: alphabetical by display form.
    Alphabetical,
    /// CLAN `+o` / `+o0`: descending frequency, ties alphabetical.
    Frequency,
    /// CLAN `+o1`: reverse concordance (shared suffixes cluster).
    ReverseConcordance,
}

/// CLI omitted-material parenthesis mode for FREQ (CLAN `+r1`/`+r2`/`+r3`).
///
/// Maps to `talkbank_clan::framework::ParenthesisMode` at the dispatch site;
/// clap stays unaware of the domain enum so users get a stable kebab-case CLI
/// surface (`--parenthesis-mode remove-parens`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ParenthesisModeArg {
    /// CLAN `+r1` (DEFAULT): remove the parentheses but keep the omitted letters
    /// (`bein(g)` -> `being`).
    RemoveParens,
    /// CLAN `+r2`: keep the parentheses literally (`bein(g)`).
    KeepParens,
    /// CLAN `+r3`: remove the omitted (parenthesized) letters (`bein(g)` -> `bein`).
    RemoveMaterial,
}

/// CLI `[: text]` replacement mode for FREQ (CLAN `+r5`).
///
/// Maps to `talkbank_clan::framework::ReplacementChoice` at the dispatch site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReplacementModeArg {
    /// CLAN default: count the replacement / corrected form (`gots [: got]` -> `got`).
    Replacement,
    /// CLAN `+r5`: count the original (replaced) surface form (`gots`).
    Original,
}

/// CLI within-word prosody mode for FREQ (CLAN `+r7`).
///
/// Maps to `talkbank_clan::framework::ProsodyMode` at the dispatch site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProsodyModeArg {
    /// CLAN default: strip within-word prosodic symbols (`ca:t` -> `cat`).
    Strip,
    /// CLAN `+r7`: keep `:`/`^`/`~` so `ca:t` stays distinct from `cat`.
    Keep,
}

/// CLI position-classification argument for FREQPOS.
///
/// Maps to `talkbank_clan::commands::freqpos::PositionClassification`
/// at the dispatch site. `last` is the CLAN default (first/last/
/// other classification); `second` is CLAN `+d` (first/second/other).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FreqposPositionArg {
    /// CLAN default: first / last / other classification.
    Last,
    /// CLAN `+d`: first / second / other classification.
    Second,
}

/// CLI spreadsheet-mode argument for FREQ.
///
/// Maps to `talkbank_clan::commands::freq::FreqSpreadsheetMode` at the dispatch
/// site. These are chatter-only flag values carrying CLAN's `+d2`/`+d3` slot
/// semantics (aggregate SpreadsheetML file output); the stdout `--format csv`
/// convenience stays separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SpreadsheetArg {
    /// CLAN `+d2`: per-word columns plus type/token/TTR.
    PerWord,
    /// CLAN `+d3`: type/token/TTR only (no per-word columns).
    Summary,
    /// CLAN `+d20`: flat one-row-per-(file, speaker, word) layout
    /// (`File | Code | Word | Count`), no `@ID` columns or summary.
    PerSpeakerWord,
}

/// Parse CLAN's `+dCN` percent-of-speakers spec (`<=50`, `>33`, `=100`, ...)
/// into a typed `SpeakerPercentFilter`. Used as the `--speaker-percentage`
/// value parser (the rewriter routes `+d<=50` -> `--speaker-percentage <=50`).
///
/// The comparator is `<`, `<=`/`=<`, `=`, `>=`/`=>`, or `>` (the manual's `C`
/// metavariable, CLAN.html "+dCN"); the rest must be all digits. A missing or
/// non-digit N is rejected, mirroring CLAN's `freq.cpp:871-874` ("Please specify
/// percentage value").
pub fn parse_speaker_percentage(
    spec: &str,
) -> Result<talkbank_clan::commands::freq::SpeakerPercentFilter, String> {
    use talkbank_clan::commands::freq::{
        SpeakerPercent, SpeakerPercentComparison, SpeakerPercentFilter,
    };

    // Two-char comparators first so `<=`/`=<`/`>=`/`=>` win over `<`/`>`/`=`.
    let (comparison, digits) =
        if let Some(rest) = spec.strip_prefix("<=").or_else(|| spec.strip_prefix("=<")) {
            (SpeakerPercentComparison::LessOrEqual, rest)
        } else if let Some(rest) = spec.strip_prefix(">=").or_else(|| spec.strip_prefix("=>")) {
            (SpeakerPercentComparison::GreaterOrEqual, rest)
        } else if let Some(rest) = spec.strip_prefix('<') {
            (SpeakerPercentComparison::LessThan, rest)
        } else if let Some(rest) = spec.strip_prefix('>') {
            (SpeakerPercentComparison::GreaterThan, rest)
        } else if let Some(rest) = spec.strip_prefix('=') {
            (SpeakerPercentComparison::Equal, rest)
        } else {
            return Err(format!(
                "expected a comparator (<, <=, =, >=, >) before the percentage, got {spec:?}"
            ));
        };

    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        // CLAN: "Please specify percentage value" (freq.cpp:873).
        return Err("please specify a percentage value (digits) after the comparator".to_owned());
    }
    let value: u64 = digits
        .parse()
        .map_err(|_| format!("percentage value {digits:?} is out of range"))?;

    Ok(SpeakerPercentFilter {
        comparison,
        percent: SpeakerPercent::new(value),
    })
}

/// CLI multi-word match-order argument for FREQ.
///
/// Maps to `talkbank_clan::framework::MatchOrder` at the dispatch site. Controls
/// how a multi-word `+s` group is matched against the token stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum MultiWordOrderArg {
    /// CLAN default: adjacent, in-order sequence.
    #[default]
    Sequence,
    /// CLAN `+c3`: anywhere and in any order.
    Any,
}

/// CLI multi-word match-scope argument for FREQ.
///
/// Maps to `talkbank_clan::framework::MatchScope` at the dispatch site. Controls
/// whether a multi-word `+s` group may match within a longer utterance or must
/// be the utterance's sole content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum MultiWordScopeArg {
    /// CLAN default: the group may match anywhere in the utterance.
    #[default]
    Anywhere,
    /// CLAN `+c4`: the utterance must consist solely of the group.
    Sole,
}

/// CLI search-multiplicity argument for FREQ.
///
/// Maps to `talkbank_clan::commands::freq::IncludeMultiplicity` at the dispatch
/// site. Controls whether a word that matches several `--include-word` patterns
/// is counted once or once per matching pattern (CLAN `+c2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum SearchMultiplicityArg {
    /// CLAN default: a word counts once if it matches any pattern.
    #[default]
    Once,
    /// CLAN `+c2`: a word counts once per matching pattern.
    PerPattern,
}

/// CLI multi-word display argument for FREQ.
///
/// Maps to `talkbank_clan::commands::freq::MultiWordDisplay` at the dispatch
/// site. Controls whether a multi-word `--include-word` match is shown as the
/// search pattern or the actual matched words (CLAN `+c7`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum MultiWordDisplayArg {
    /// CLAN default: show the search pattern (one entry per group).
    #[default]
    Pattern,
    /// CLAN `+c7`: show the actual matched words.
    Matched,
}

#[cfg(test)]
mod tests {
    use super::parse_speaker_percentage;
    use talkbank_clan::commands::freq::SpeakerPercentComparison;

    /// Every comparator spelling parses to the right comparator + percentage,
    /// including CLAN's `=<` / `=>` aliases for `<=` / `>=` (freq.cpp:841,845).
    #[test]
    fn parses_each_comparator() {
        let cases = [
            ("<50", SpeakerPercentComparison::LessThan, 50),
            ("<=50", SpeakerPercentComparison::LessOrEqual, 50),
            ("=<50", SpeakerPercentComparison::LessOrEqual, 50),
            ("=100", SpeakerPercentComparison::Equal, 100),
            (">=33", SpeakerPercentComparison::GreaterOrEqual, 33),
            ("=>33", SpeakerPercentComparison::GreaterOrEqual, 33),
            (">0", SpeakerPercentComparison::GreaterThan, 0),
        ];
        for (spec, comparison, percent) in cases {
            let filter = parse_speaker_percentage(spec).unwrap_or_else(|e| panic!("{spec}: {e}"));
            assert_eq!(filter.comparison, comparison, "comparator for {spec}");
            assert_eq!(filter.percent.value(), percent, "percentage for {spec}");
        }
    }

    /// A comparator with no digits (CLAN `+d<`) is rejected; the message points
    /// at the missing percentage value (CLAN: "Please specify percentage value").
    #[test]
    fn rejects_missing_percentage() {
        let err = parse_speaker_percentage("<").expect_err("must reject bare comparator");
        assert!(err.contains("percentage value"), "got: {err}");
    }

    /// A non-digit percentage is rejected.
    #[test]
    fn rejects_non_digit_percentage() {
        assert!(parse_speaker_percentage("<=5x").is_err());
        assert!(parse_speaker_percentage("<= 50").is_err());
    }

    /// A spec with no leading comparator is rejected.
    #[test]
    fn rejects_missing_comparator() {
        let err = parse_speaker_percentage("50").expect_err("must reject bare number");
        assert!(err.contains("comparator"), "got: {err}");
    }
}
