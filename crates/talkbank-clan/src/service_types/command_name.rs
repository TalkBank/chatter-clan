//! Typed CLAN command-name identifiers and parsing.

use std::fmt;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Typed identifier for one supported CLAN analysis command.
///
/// Outer adapters such as the CLI and LSP should parse raw command-name
/// strings into this enum at their boundary, then pass the typed identifier
/// through the shared builder and service layers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisCommandName {
    /// `freq`
    Freq,
    /// `mlu`
    Mlu,
    /// `mlt`
    Mlt,
    /// `wdlen`
    Wdlen,
    /// `wdsize`
    Wdsize,
    /// `maxwd`
    Maxwd,
    /// `freqpos`
    Freqpos,
    /// `timedur`
    Timedur,
    /// `kwal`
    Kwal,
    /// `gemlist`
    Gemlist,
    /// `combo`
    Combo,
    /// `cooccur`
    Cooccur,
    /// `dist`
    Dist,
    /// `chip`
    Chip,
    /// `phonfreq`
    Phonfreq,
    /// `modrep`
    Modrep,
    /// `vocd`
    Vocd,
    /// `codes`
    Codes,
    /// `chains`
    Chains,
    /// `complexity`
    Complexity,
    /// `corelex`
    Corelex,
    /// `dss`
    Dss,
    /// `eval`
    Eval,
    /// `eval-d`
    #[serde(rename = "eval-d")]
    EvalDialect,
    /// `flucalc`
    Flucalc,
    /// `ipsyn`
    Ipsyn,
    /// `keymap`
    Keymap,
    /// `kideval`
    Kideval,
    /// `mortable`
    Mortable,
    /// `rely`
    Rely,
    /// `script`
    Script,
    /// `sugar`
    Sugar,
    /// `trnfix`
    Trnfix,
    /// `uniq`
    Uniq,
}

impl AnalysisCommandName {
    /// Canonical inventory of supported analysis commands in stable wire-name
    /// order.
    pub const ALL: &'static [Self] = &[
        Self::Freq,
        Self::Mlu,
        Self::Mlt,
        Self::Wdlen,
        Self::Wdsize,
        Self::Maxwd,
        Self::Freqpos,
        Self::Timedur,
        Self::Kwal,
        Self::Gemlist,
        Self::Combo,
        Self::Cooccur,
        Self::Dist,
        Self::Chip,
        Self::Phonfreq,
        Self::Modrep,
        Self::Vocd,
        Self::Codes,
        Self::Chains,
        Self::Complexity,
        Self::Corelex,
        Self::Dss,
        Self::Eval,
        Self::EvalDialect,
        Self::Flucalc,
        Self::Ipsyn,
        Self::Keymap,
        Self::Kideval,
        Self::Mortable,
        Self::Rely,
        Self::Script,
        Self::Sugar,
        Self::Trnfix,
        Self::Uniq,
    ];

    /// Return the stable wire-format name used by CLI and editor adapters.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Freq => "freq",
            Self::Mlu => "mlu",
            Self::Mlt => "mlt",
            Self::Wdlen => "wdlen",
            Self::Wdsize => "wdsize",
            Self::Maxwd => "maxwd",
            Self::Freqpos => "freqpos",
            Self::Timedur => "timedur",
            Self::Kwal => "kwal",
            Self::Gemlist => "gemlist",
            Self::Combo => "combo",
            Self::Cooccur => "cooccur",
            Self::Dist => "dist",
            Self::Chip => "chip",
            Self::Phonfreq => "phonfreq",
            Self::Modrep => "modrep",
            Self::Vocd => "vocd",
            Self::Codes => "codes",
            Self::Chains => "chains",
            Self::Complexity => "complexity",
            Self::Corelex => "corelex",
            Self::Dss => "dss",
            Self::Eval => "eval",
            Self::EvalDialect => "eval-d",
            Self::Flucalc => "flucalc",
            Self::Ipsyn => "ipsyn",
            Self::Keymap => "keymap",
            Self::Kideval => "kideval",
            Self::Mortable => "mortable",
            Self::Rely => "rely",
            Self::Script => "script",
            Self::Sugar => "sugar",
            Self::Trnfix => "trnfix",
            Self::Uniq => "uniq",
        }
    }

    /// Whether this command preserves word case by default. These are the
    /// commands in CLAN's `mmaininit` `nomap=TRUE` set (cutt.cpp:7845): FREQ
    /// and VOCD. For them `+k` FOLDS to lowercase; every other command folds
    /// by default and `+k` preserves.
    pub const fn preserves_case_by_default(self) -> bool {
        matches!(self, Self::Freq | Self::Vocd)
    }

    /// Resolve the effective case-sensitive (preserve-case) keying and
    /// `+s`-matching state from whether `+k` was given. CLAN's `+k` TOGGLES
    /// the per-command `nomap` default (cutt.cpp:13816): preserve-by-default
    /// commands fold under `+k`; fold-by-default commands preserve under `+k`.
    ///
    /// This is the single source of the `+k` polarity. FREQ and VOCD (the
    /// preserve-by-default commands) route both their keying and `+s`-matching
    /// seams through it. The fold-by-default commands currently pass the raw
    /// flag through, which equals this helper's output for them (the identity
    /// branch); they should be converted to call it as each is driven to
    /// parity, so the invariant is enforced rather than relied upon.
    pub const fn effective_case_sensitive(self, plus_k_present: bool) -> bool {
        if self.preserves_case_by_default() {
            !plus_k_present
        } else {
            plus_k_present
        }
    }

    /// Return the CLAN banner scope mode for this command.
    ///
    /// CLAN's `cutt.cpp` mainloop emits one of three scope shapes
    /// depending on the `nomain` and `tct` flags the command sets in
    /// its arg parser. Each chatter command must select the same mode
    /// as its CLAN counterpart for byte-level banner parity.
    pub const fn clan_scope_mode(self) -> ClanScopeMode {
        match self {
            // Dependent-tier-only commands (CLAN `nomain=TRUE` and a
            // single `tct` entry): banner emits just `ONLY dependent
            // tiers matching: %X;` with no speaker-tier prefix.
            Self::Mlu | Self::Vocd => ClanScopeMode::DependentOnly("mor"),

            // Combined commands (CLAN `nomain=FALSE` with a `tct`
            // dependent-tier filter): banner emits `ALL speaker
            // tiers` followed by `and those speakers' ONLY dependent
            // tiers matching: %X;` on a continuation line.
            //
            // `maxwd` was previously here but its CLAN banner is
            // main-only (it counts characters on the main tier, not
            // morphemes on %mor), moved to MainOnly below.
            Self::Wdlen
            | Self::Wdsize
            | Self::Dss
            | Self::Ipsyn
            | Self::Mortable
            | Self::Corelex
            | Self::Eval
            | Self::EvalDialect
            | Self::Kideval
            | Self::Sugar => ClanScopeMode::MainAndDependent("mor"),
            // `complexity` reads %gra rather than %mor, and CLAN
            // emits a 4th-banner-shape `and ONLY header tiers
            // matching: @ID:;` continuation after the dep-tier line.
            // The header-filter continuation isn't yet modelled in
            // `ClanScopeMode`; the MainAndDependent("gra:") here
            // only captures the dep-tier dimension. Tracked in
            // scripts/clan-parity/STATUS.md.
            Self::Complexity => ClanScopeMode::MainAndDependent("gra:"),
            Self::Phonfreq => ClanScopeMode::MainAndDependent("pho:"),

            // Main-tier-only commands: banner emits just `ALL speaker
            // tiers` (or the explicit speaker-tier filter set).
            // `freqpos` is in this group despite consuming `%mor`
            // because CLAN's freqpos emits the main-only banner.
            Self::Freq
            | Self::Mlt
            | Self::Maxwd
            | Self::Kwal
            | Self::Combo
            | Self::Cooccur
            | Self::Dist
            | Self::Gemlist
            | Self::Chip
            | Self::Modrep
            | Self::Codes
            | Self::Chains
            | Self::Timedur
            | Self::Freqpos
            | Self::Flucalc
            | Self::Keymap
            | Self::Rely
            | Self::Script
            | Self::Trnfix
            | Self::Uniq => ClanScopeMode::MainOnly,
        }
    }
}

/// Selects which scope text CLAN's banner emits.
///
/// CLAN's `cutt.cpp` mainloop branches on `nomain` (does the command
/// consume main tier at all?) and `tct` (is a `+t%X` dependent-tier
/// filter active?). The combinations produce three distinct banner
/// shapes; chatter mirrors that taxonomy here so the
/// banner-emission code can stay in one helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanScopeMode {
    /// `ALL speaker tiers`, main-tier-only commands.
    MainOnly,
    /// `ONLY dependent tiers matching: %X;`, no main tier, single
    /// dependent tier. The carried `&str` is the tier name without
    /// the leading `%` (e.g. `"mor"`).
    DependentOnly(&'static str),
    /// `ALL speaker tiers\n\tand those speakers' ONLY dependent tiers
    /// matching: %X;`, main tier plus a dependent-tier filter. The
    /// carried `&str` is the tier name (e.g. `"mor"`); for phonfreq
    /// CLAN includes a trailing colon (`%PHO:;`), so the carried
    /// value is `"pho:"`.
    MainAndDependent(&'static str),
}

impl fmt::Display for AnalysisCommandName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a raw outer-layer command name is not recognized.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("Unknown analysis command: {command_name}")]
pub struct ParseAnalysisCommandNameError {
    command_name: String,
}

impl FromStr for AnalysisCommandName {
    type Err = ParseAnalysisCommandNameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "freq" => Ok(Self::Freq),
            "mlu" => Ok(Self::Mlu),
            "mlt" => Ok(Self::Mlt),
            "wdlen" => Ok(Self::Wdlen),
            "wdsize" => Ok(Self::Wdsize),
            "maxwd" => Ok(Self::Maxwd),
            "freqpos" => Ok(Self::Freqpos),
            "timedur" => Ok(Self::Timedur),
            "kwal" => Ok(Self::Kwal),
            "gemlist" => Ok(Self::Gemlist),
            "combo" => Ok(Self::Combo),
            "cooccur" => Ok(Self::Cooccur),
            "dist" => Ok(Self::Dist),
            "chip" => Ok(Self::Chip),
            "phonfreq" => Ok(Self::Phonfreq),
            "modrep" => Ok(Self::Modrep),
            "vocd" => Ok(Self::Vocd),
            "codes" => Ok(Self::Codes),
            "chains" => Ok(Self::Chains),
            "complexity" => Ok(Self::Complexity),
            "corelex" => Ok(Self::Corelex),
            "dss" => Ok(Self::Dss),
            "eval" => Ok(Self::Eval),
            "eval-d" => Ok(Self::EvalDialect),
            "flucalc" => Ok(Self::Flucalc),
            "ipsyn" => Ok(Self::Ipsyn),
            "keymap" => Ok(Self::Keymap),
            "kideval" => Ok(Self::Kideval),
            "mortable" => Ok(Self::Mortable),
            "rely" => Ok(Self::Rely),
            "script" => Ok(Self::Script),
            "sugar" => Ok(Self::Sugar),
            "trnfix" => Ok(Self::Trnfix),
            "uniq" => Ok(Self::Uniq),
            _ => Err(ParseAnalysisCommandNameError {
                command_name: value.to_owned(),
            }),
        }
    }
}

impl<'de> Deserialize<'de> for AnalysisCommandName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}
