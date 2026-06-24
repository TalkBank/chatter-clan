//! CLAN subcommand definitions for the `chatter` CLI.
//!
//! Analysis commands deliberately keep their flag surface thin and typed so the
//! CLI can normalize into library-owned analysis models before delegating
//! defaults and validation back to `talkbank-clan`.

mod help_grouping;
mod shared_args;
#[cfg(test)]
mod tests;

pub use help_grouping::apply_clan_help_grouping;
pub use shared_args::{
    CapitalizationArg, FreqposPositionArg, MultiWordDisplayArg, MultiWordOrderArg,
    MultiWordScopeArg, ParenthesisModeArg, ProsodyModeArg, ReplacementModeArg,
    SearchMultiplicityArg, SortArg, SpreadsheetArg, parse_speaker_percentage,
};

use clap::{ArgGroup, Subcommand};
use std::path::PathBuf;
use talkbank_clan::commands::codes::CodesConfig;
use talkbank_clan::commands::corelex::CorelexConfig;
use talkbank_clan::commands::dss::DssConfig;
use talkbank_clan::commands::ipsyn::IpsynConfig;
use talkbank_clan::commands::keymap::KeymapConfig;
use talkbank_clan::commands::maxwd::MaxwdConfig;
use talkbank_clan::commands::rely::RelyConfig;
use talkbank_clan::commands::trnfix::TrnfixConfig;

use super::clan_common::CommonAnalysisArgs;

/// Flat enum of all CLAN analysis and transform commands.
#[derive(Subcommand)]
pub enum ClanCommands {
    // -- Analysis commands --
    /// Word/morpheme frequency counts with type-token ratio
    Freq {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Count morphemes from %mor tier instead of words from main tier
        #[arg(long)]
        mor: bool,

        /// Count the whitespace-delimited tokens of dependent tier `%X` instead
        /// of main-tier words (CLAN `+t%X`): `--tier gra` counts `%gra`
        /// relations, `--tier mor` counts the raw `%mor` tokens. Mutually
        /// exclusive with `--mor` (chatter's structural %mor counting); CLAN's
        /// `+t%mor` slot maps here, not to `--mor`.
        #[arg(long)]
        tier: Option<talkbank_clan::framework::TierKind>,

        /// Count the main tier PLUS every dependent tier EXCEPT the named one(s)
        /// (CLAN `-t%X`), pooled into one table. Repeatable. Mutually exclusive
        /// with `--mor` and `--tier`; CLAN's `-t%X` slot maps here.
        #[arg(long = "exclude-tier")]
        exclude_tier: Vec<talkbank_clan::framework::TierKind>,

        /// Capitalization filter: `initial` (CLAN `+c` / `+c0`,
        /// uppercase first letter) or `mid` (CLAN `+c1`, uppercase
        /// letter after position 0). Default: no filter.
        #[arg(long = "capitalization", value_enum)]
        capitalization: Option<CapitalizationArg>,

        /// Sort order for the per-word entries: `alphabetical` (CLAN default),
        /// `frequency` (CLAN `+o` / `+o0`, descending count), or
        /// `reverse-concordance` (CLAN `+o1`, groups words by suffix).
        #[arg(long = "sort", value_enum)]
        sort: Option<SortArg>,

        /// Emit only an alphabetized deduped word list, one word per
        /// line, with no banners, counts, or totals. Output is meant
        /// as fodder for `kwal +s@FILE`. Maps CLAN's FREQ `+d1`.
        #[arg(long = "word-list-only")]
        word_list_only: bool,

        /// Emit only the per-speaker type/token/TTR summary, dropping
        /// all per-word frequency entries. Maps CLAN's FREQ `+d4`.
        #[arg(long = "types-tokens-only")]
        types_tokens_only: bool,

        /// Emit an aggregate Excel/SpreadsheetML file (one row per file x
        /// speaker, keyed by @ID) instead of stdout text: `per-word` (CLAN
        /// `+d2`) includes per-word columns, `summary` (CLAN `+d3`) is
        /// type/token/TTR only. Distinct from `--format csv` (a stdout
        /// convenience).
        #[arg(long = "spreadsheet", value_enum)]
        spreadsheet: Option<SpreadsheetArg>,

        /// Restrict the spreadsheet to words used by `<`, `<=`, `=`, `>=`, or
        /// `>` than N percent of speakers, then report each speaker's
        /// Types/Token/TTR over only that word subset (CLAN `+dCN`, e.g.
        /// `+d<=50`). The value is a comparator followed by the percentage
        /// (`<=50`, `>33`, `=100`). Writes `words.frq.xls`; cannot be combined
        /// with `--spreadsheet`.
        #[arg(long = "speaker-percentage", value_name = "SPEC", value_parser = parse_speaker_percentage)]
        speaker_percentage: Option<talkbank_clan::commands::freq::SpeakerPercentFilter>,

        /// Internal reject sentinel: the rewriter routes CLAN `+g`/`-g` here
        /// because FREQ has no gem flag (CLAN rejects it; gem-limiting is the
        /// GEM program). The dispatch turns any value into a CLAN-style error.
        /// Hidden; chatter's gem convenience is reached via `--gem`.
        #[arg(long = "reject-clan-gem", hide = true)]
        reject_clan_gem: Vec<String>,

        /// Compute the Moving-Average Type-Token Ratio (MATTR) over a sliding
        /// window of N tokens (CLAN `+bN`). The per-speaker average of each
        /// window's TTR; the frame size must be a positive integer.
        #[arg(long = "mattr", value_name = "N")]
        mattr: Option<talkbank_clan::framework::FrameSize>,

        /// How a multi-word `--include-word` group is matched: `sequence`
        /// (adjacent, in-order, the default) or `any` (anywhere and in any
        /// order, CLAN `+c3`).
        #[arg(long = "multiword-order", value_enum, default_value_t = MultiWordOrderArg::Sequence)]
        multiword_order: MultiWordOrderArg,

        /// Where a multi-word `--include-word` group may match: `anywhere`
        /// (within a longer utterance, the default) or `sole` (the utterance
        /// must consist solely of the group, CLAN `+c4`).
        #[arg(long = "multiword-scope", value_enum, default_value_t = MultiWordScopeArg::Anywhere)]
        multiword_scope: MultiWordScopeArg,

        /// How a word matching several `--include-word` patterns is counted:
        /// `once` (the default) or `per-pattern` (once for each matching
        /// pattern, CLAN `+c2`; requires wildcard patterns, single-word only).
        #[arg(long = "search-multiplicity", value_enum, default_value_t = SearchMultiplicityArg::Once)]
        search_multiplicity: SearchMultiplicityArg,

        /// How a multi-word `--include-word` match is displayed: `pattern` (the
        /// search pattern, the default) or `matched` (the actual matched words,
        /// CLAN `+c7`, so a wildcard slot reveals what occurred).
        #[arg(long = "multiword-display", value_enum, default_value_t = MultiWordDisplayArg::Pattern)]
        multiword_display: MultiWordDisplayArg,

        /// Show every literal `--include-word` even when it never matched, with
        /// count 0 (CLAN `+d5`). The zero word is displayed but excluded from
        /// the types/tokens/TTR statistics. Requires at least one
        /// `--include-word`, and none may contain wildcards (`* % _`) or repeat.
        #[arg(long = "include-zero-frequency")]
        include_zero_frequency: bool,

        /// Pool all speakers into one combined frequency table with no
        /// per-speaker header, summing counts and combining the
        /// types/tokens/TTR statistics (CLAN `+o3`).
        #[arg(long = "combine-speakers")]
        combine_speakers: bool,

        /// How omitted-material parentheses (`bein(g)`) render: `remove-parens`
        /// (CLAN `+r1`, the default: drop the parens, keep the letters ->
        /// `being`), `keep-parens` (CLAN `+r2`: `bein(g)`), or `remove-material`
        /// (CLAN `+r3`: drop the letters -> `bein`).
        #[arg(long = "parenthesis-mode", value_enum, default_value_t = ParenthesisModeArg::RemoveParens)]
        parenthesis_mode: ParenthesisModeArg,

        /// Whether a `[: text]` replacement (`gots [: got]`) counts the
        /// `replacement` (corrected form `got`, the default) or the `original`
        /// (the replaced surface form `gots`, CLAN `+r5`).
        #[arg(long = "replacement-mode", value_enum, default_value_t = ReplacementModeArg::Replacement)]
        replacement_mode: ReplacementModeArg,

        /// Whether within-word prosodic symbols (`:` lengthening, `^` syllable
        /// pause, `~` clitic) are kept: `strip` (the default, `ca:t` -> `cat`) or
        /// `keep` (CLAN `+r7`, `ca:t` stays distinct).
        #[arg(long = "prosody-mode", value_enum, default_value_t = ProsodyModeArg::Strip)]
        prosody_mode: ProsodyModeArg,

        /// Extra characters that split a counted word into separate tokens (CLAN
        /// `+pS`, e.g. `+p_` breaks `New_York` into `New` and `York`). Each
        /// character of the value becomes an additional word delimiter;
        /// whitespace is ignored. An empty value is an error.
        #[arg(long = "word-delimiters", value_name = "CHARS")]
        word_delimiters: Option<String>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Mean length of utterance (morphemes or words)
    Mlu {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Count words from main tier instead of morphemes from %mor
        #[arg(long)]
        words: bool,

        /// Exclude utterances that consist solely of the given word.
        /// Maps CLAN's command-specific `+gS` (e.g. `mlu +gum`). Repeatable.
        #[arg(long = "exclude-solo-word")]
        exclude_solo_word: Vec<String>,

        /// Load solo-word exclusions from a file (one pattern per
        /// line; blank lines, `# `-comments, and `;%*`-annotation
        /// lines skipped). Maps CLAN's `+g@FILE`. Repeatable; entries
        /// extend `--exclude-solo-word`.
        #[arg(long = "exclude-solo-word-file", value_name = "PATH")]
        exclude_solo_word_file: Vec<PathBuf>,

        /// Pool all selected speakers into a single `*COMBINED*` MLU result
        /// instead of a per-speaker breakdown (CLAN `+o3`).
        #[arg(long = "combine-speakers")]
        combine_speakers: bool,

        /// Re-admit utterances containing `xxx` (unintelligible) to the count,
        /// which MLU excludes by default (CLAN `+sxxx`). The `xxx` marker itself
        /// still contributes no morpheme.
        #[arg(long = "include-xxx")]
        include_xxx: bool,

        /// Re-admit utterances containing `yyy` (phonological) to the count
        /// (CLAN `+syyy`). The `yyy` marker itself still contributes no morpheme.
        #[arg(long = "include-yyy")]
        include_yyy: bool,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Mean length of turn (utterances and words per turn)
    Mlt {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Exclude utterances that consist solely of the given word.
        /// Maps CLAN's command-specific `+gS` (e.g. `mlt +gum`). Repeatable.
        #[arg(long = "exclude-solo-word")]
        exclude_solo_word: Vec<String>,

        /// Load solo-word exclusions from a file (same format as
        /// `--include-word-file`). Maps CLAN's `+g@FILE`. Repeatable.
        #[arg(long = "exclude-solo-word-file", value_name = "PATH")]
        exclude_solo_word_file: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Word length distribution
    Wdlen {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Word size (character length) histogram from %mor stems
    Wdsize {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Use main tier words instead of %mor stems
        #[arg(long)]
        main_tier: bool,

        /// Length-bounded histogram (CLAN: `+w[>|<|=]N`).
        /// Format: `<gt|lt|eq>:<N>`. Examples: `gt:4` keeps
        /// length > 4; `lt:5` keeps length < 5; `eq:3` keeps
        /// length == 3.
        #[arg(long = "length-filter", value_name = "COMPARATOR:N")]
        length_filter: Option<talkbank_clan::commands::wdsize::LengthFilter>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Longest words per speaker
    Maxwd {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Maximum number of words to display
        #[arg(short = 'n', long, default_value_t = MaxwdConfig::default().limit)]
        limit: talkbank_clan::framework::WordLimit,

        /// Include only words whose length is unique within a
        /// speaker's lexicon (CLAN: `+a`).
        #[arg(long = "unique-length-only")]
        unique_length_only: bool,

        /// Drop words of length N from the output (CLAN: `+xN`,
        /// repeatable).
        #[arg(long = "exclude-length", value_name = "N")]
        exclude_length: Vec<usize>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Word frequency grouped by part of speech from %mor tier
    Freqpos {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Position classification: `last` (CLAN default,
        /// first/last/other) or `second` (CLAN `+d`,
        /// first/second/other).
        #[arg(long = "position-classification", value_enum, default_value_t = FreqposPositionArg::Last)]
        position_classification: FreqposPositionArg,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Time duration statistics from bullet timing marks
    Timedur {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Keyword-in-context search (matching utterances)
    Kwal {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Keywords to search for (case-insensitive substring match)
        #[arg(short, long, required = true)]
        keyword: Vec<String>,

        /// Strict match: keyword must be the only countable word
        /// on the tier (CLAN: `+b`).
        #[arg(long = "strict-match")]
        strict_match: bool,

        /// Emit matching utterances as legal CHAT, drop the
        /// `---` separator and the `*** File ... Keyword: X`
        /// location annotation. Maps CLAN's KWAL `+d` (no N).
        #[arg(long = "legal-chat")]
        legal_chat: bool,

        /// Pre-context lines: number of utterances immediately
        /// preceding each match to include with that match. Maps
        /// CLAN's KWAL `-wN`. Default `0`.
        #[arg(long = "context-before", default_value_t = 0, value_name = "N")]
        context_before: u32,

        /// Post-context lines: number of utterances immediately
        /// following each match to include with that match. Maps
        /// CLAN's KWAL `+wN`. Default `0`.
        #[arg(long = "context-after", default_value_t = 0, value_name = "N")]
        context_after: u32,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// List gem segments (@Bg/@Eg bracketed regions)
    Gemlist {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Boolean keyword search (AND/OR combinations)
    Combo {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Search expression(s): use + for AND, comma for OR (can be repeated)
        #[arg(short = 'S', long = "search", required_unless_present = "search_file")]
        search: Vec<String>,

        /// Exclude expression(s): same syntax as `--search` but
        /// drops utterances that match any of these (can be
        /// repeated). Maps CLAN's COMBO `-sS`.
        #[arg(long = "exclude-search")]
        exclude_search: Vec<String>,

        /// Load search expressions from FILE (one per line).
        /// Maps CLAN's COMBO `+s@FILE`. File format matches
        /// `cutt.cpp::rdexclf`: blank lines, `# `-comments, and
        /// `;%* `-annotation lines skipped. Repeatable.
        #[arg(long = "search-file", value_name = "PATH")]
        search_file: Vec<PathBuf>,

        /// Load exclude search expressions from FILE (one per
        /// line). Maps CLAN's COMBO `-s@FILE`. Same file format
        /// as `--search-file`. Repeatable.
        #[arg(long = "exclude-search-file", value_name = "PATH")]
        exclude_search_file: Vec<PathBuf>,

        /// Only report the first matching expression per utterance.
        /// Maps CLAN's COMBO `+g3`.
        #[arg(long = "first-match-only")]
        first_match_only: bool,

        /// Deduplicate repeated word matches within an utterance.
        /// Maps CLAN's COMBO `+g7`.
        #[arg(long = "dedupe-matches")]
        dedupe_matches: bool,

        /// Pre-context lines: utterances immediately preceding each
        /// match to include with that match. Maps CLAN's COMBO `-wN`.
        #[arg(long = "context-before", default_value_t = 0, value_name = "N")]
        context_before: u32,

        /// Post-context lines: utterances immediately following each
        /// match to include with that match. Maps CLAN's COMBO `+wN`.
        #[arg(long = "context-after", default_value_t = 0, value_name = "N")]
        context_after: u32,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Word co-occurrence counting (N-grams of words in same utterance)
    Cooccur {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Strip frequency-count column from CLAN-format output
        /// (CLAN: `+d`).
        #[arg(long = "no-frequency-counts")]
        no_frequency_counts: bool,

        /// Cluster size: number of adjacent words counted per row.
        /// Default `2` (bigrams). `+n3` ⇒ trigrams; etc. Maps CLAN's
        /// COOCCUR `+nN`.
        #[arg(long = "cluster-size", default_value_t = 2, value_name = "N")]
        cluster_size: u8,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Word distribution analysis (dispersion across utterances)
    Dist {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Count each word at most once per turn (CLAN: `+g`).
        #[arg(long = "once-per-turn")]
        once_per_turn: bool,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Child/parent interaction profile (imitation, overlap analysis)
    Chip {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Phonological frequency from %pho tier (phone character counts)
    Phonfreq {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Model/replica comparison from %mod and %pho tiers
    Modrep {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Vocabulary diversity (D statistic) via bootstrap sampling
    Vocd {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Capitalization filter: `initial` (CLAN `+c` / `+c0`,
        /// uppercase first letter) or `mid` (CLAN `+c1`, uppercase
        /// letter after position 0). Default: no filter.
        #[arg(long = "capitalization", value_enum)]
        capitalization: Option<CapitalizationArg>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Report repeated utterances with frequency counts
    Uniq {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Sort output by descending frequency (CLAN -o flag)
        #[arg(long)]
        sort: bool,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Frequency table of codes from %cod tier
    Codes {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Maximum depth of code parsing (0 = all levels)
        #[arg(long, default_value_t = CodesConfig::default().max_depth)]
        max_depth: talkbank_clan::framework::CodeDepth,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Compare two tiers word-by-word and report mismatches
    Trnfix {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// First tier to compare (default: mor)
        #[arg(long, default_value_t = TrnfixConfig::default().tier1)]
        tier1: talkbank_clan::framework::TierKind,

        /// Second tier to compare (default: trn)
        #[arg(long, default_value_t = TrnfixConfig::default().tier2)]
        tier2: talkbank_clan::framework::TierKind,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Morphosyntactic structure scoring (MLU-S, TNW, WPS, CPS)
    Sugar {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Minimum number of utterances required for SUGAR to produce
        /// output (CLAN default 50). Maps CLAN's `+aN`. Files below
        /// the threshold are reported as "insufficient sample size".
        #[arg(long = "min-utterances")]
        min_utterances: Option<talkbank_clan::framework::UtteranceLimit>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Cross-tabulation of morphological categories from %mor tier
    Mortable {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Path to language script file (`.cut`), required, matching
        /// CLAN's `+l` refusal. Left as `Option` here so the missing-
        /// flag case can emit CLAN's exact error message instead of
        /// clap's default `required` complaint.
        #[arg(short = 'f', long)]
        script: Option<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Clause chain analysis via code markers (consecutive code occurrences)
    Chains {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Tier label to read codes from (required, CLAN's chains
        /// refuses with `Please specify a code tier with "+t" option.`
        /// when no code tier is provided)
        #[arg(long)]
        tier: Option<talkbank_clan::framework::TierKind>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Syntactic complexity ratio from %gra dependency tier
    Complexity {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Core vocabulary analysis (words above frequency threshold)
    Corelex {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Minimum frequency threshold for core words (defaults to the library corelex threshold)
        #[arg(long, default_value_t = CorelexConfig::default().min_frequency)]
        threshold: talkbank_clan::framework::FrequencyThreshold,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Contingency tables for coded data (keyword-following-code frequencies)
    Keymap {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Keywords to track (can be repeated)
        #[arg(short, long, required = true)]
        keyword: Vec<String>,

        /// Tier label to read codes from (default: cod)
        #[arg(long, default_value_t = KeymapConfig::default().tier)]
        tier: talkbank_clan::framework::TierKind,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Compare utterances to a template script (accuracy metrics)
    Script {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Path to template/script CHAT file
        #[arg(short, long)]
        template: PathBuf,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Inter-rater agreement (Cohen's kappa) between two coded files
    Rely {
        /// First coded CHAT file
        file1: PathBuf,

        /// Second coded CHAT file
        file2: PathBuf,

        /// Tier label to compare (default: cod)
        #[arg(long, default_value_t = RelyConfig::default().tier)]
        tier: talkbank_clan::framework::TierKind,

        /// Output format: clan (default, character-for-character match with legacy CLAN), text, json, or csv
        #[arg(short, long, value_enum, default_value_t = super::clan_common::ClanOutputFormat::Clan)]
        format: super::clan_common::ClanOutputFormat,
    },

    /// Fluency calculation (disfluency metrics: SLD, TD)
    Flucalc {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Developmental Sentence Scoring
    Dss {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Path to DSS rules file (.scr)
        #[arg(long)]
        rules: Option<PathBuf>,

        /// Maximum utterances to score (default: 50)
        #[arg(long, default_value_t = DssConfig::default().max_utterances)]
        max_utterances: talkbank_clan::framework::UtteranceLimit,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Index of Productive Syntax
    Ipsyn {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Path to IPSYN rules file
        #[arg(long)]
        rules: Option<PathBuf>,

        /// Maximum utterances to analyze (default: 100)
        #[arg(long, default_value_t = IpsynConfig::default().max_utterances)]
        max_utterances: talkbank_clan::framework::UtteranceLimit,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Language sample evaluation (morphosyntactic analysis)
    Eval {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Combined child language evaluation (DSS + VOCD + IPSYN + EVAL)
    Kideval {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Path to DSS rules file
        #[arg(long)]
        dss_rules: Option<PathBuf>,

        /// Path to IPSYN rules file
        #[arg(long)]
        ipsyn_rules: Option<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    // -- Transform commands --
    /// Simplified fluent output (adds %flo tier, strips headers)
    Flo {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Lowercase all words on main tiers
    Lowcase {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// String replacement using a changes file
    Chstring {
        /// Path to input CHAT file
        path: PathBuf,

        /// Path to changes file (alternating find/replace lines)
        #[arg(short, long)]
        changes: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Compute ages from @Birth and @Date headers
    Dates {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Add missing terminators (default: period)
    Delim {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Fix timing bullet consistency
    Fixbullets {
        /// Path to input CHAT file
        path: PathBuf,

        /// Global millisecond offset to apply to parsed bullet timings
        #[arg(long)]
        offset: Option<i64>,

        /// Include only selected tier kinds (for example `cod`, `%com`, `*`)
        #[arg(long = "tier")]
        tier: Vec<String>,

        /// Exclude selected tier kinds (for example `mor`, `%cod`, `*`)
        #[arg(long = "exclude-tier")]
        exclude_tier: Vec<String>,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Add %ret tier copying main tier content verbatim
    Retrace {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Mark utterances with revisions using [+ rep] postcodes
    Repeat {
        /// Path to input CHAT file
        path: PathBuf,

        /// Target speaker code (required)
        #[arg(short, long)]
        speaker: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Combine multiple dependent tiers of the same type into one
    Combtier {
        /// Path to input CHAT file
        path: PathBuf,

        /// Tier label to combine (e.g., "com" for %com)
        #[arg(short, long)]
        tier: String,

        /// Separator between combined contents
        #[arg(long, default_value = " ")]
        separator: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Normalize compound word formatting (dashes to plus notation)
    Compound {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Reorder dependent tiers to canonical order
    Tierorder {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Add or remove line numbers on tier lines
    Lines {
        /// Path to input CHAT file
        path: PathBuf,

        /// Remove existing line numbers instead of adding them
        #[arg(short, long)]
        remove: bool,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Fix common CHAT formatting errors (bracket spacing, ellipsis, etc.)
    Dataclean {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Extract quoted text to separate utterances
    Quotes {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Apply orthographic conversion using a dictionary file
    Ort {
        /// Path to input CHAT file
        path: PathBuf,

        /// Path to orthographic conversion dictionary
        #[arg(short, long)]
        dictionary: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Apply pattern-matching rules to %mor tier post-processing
    Postmortem {
        /// Path to input CHAT file
        path: PathBuf,

        /// Path to rules file (from_pattern => to_replacement)
        #[arg(short, long)]
        rules: PathBuf,

        /// Target tier label (default: mor)
        #[arg(long, default_value = "mor")]
        target_tier: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate %mod tier from pronunciation lexicon
    Makemod {
        /// Path to input CHAT file
        path: PathBuf,

        /// Path to CMU-format pronunciation lexicon file
        #[arg(short, long)]
        lexicon: PathBuf,

        /// Show all alternative pronunciations
        #[arg(long)]
        all_alternatives: bool,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Remove selected dependent tiers while preserving the rest of the file
    Trim {
        /// Path to input CHAT file
        path: PathBuf,

        /// Keep only the selected dependent tier label(s), e.g. "mor" or "*"
        #[arg(long = "tier")]
        tier: Vec<String>,

        /// Remove the selected dependent tier label(s), e.g. "mor" or "*"
        #[arg(long = "exclude-tier")]
        exclude_tier: Vec<String>,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Rename speaker codes throughout a CHAT file
    Roles {
        /// Path to input CHAT file
        path: PathBuf,

        /// Rename mapping as OLD=NEW (can be repeated)
        #[arg(short, long, required = true)]
        rename: Vec<String>,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    // -- Converter commands --
    /// Convert CHAT file to plain text
    Chat2text {
        /// Path to CHAT file
        path: PathBuf,

        /// Include speaker codes in output
        #[arg(long)]
        include_speaker: bool,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert SRT subtitles to CHAT format
    Srt2chat {
        /// Path to SRT file
        path: PathBuf,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert CHAT file to SRT subtitle format
    Chat2srt {
        /// Path to CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert CHAT file to WebVTT subtitle format
    Chat2vtt {
        /// Path to CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert plain text to CHAT format
    Text2chat {
        /// Path to text file
        path: PathBuf,

        /// Speaker code (default: SPK)
        #[arg(short, long, default_value = "SPK")]
        speaker: String,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert LIPP phonetic profile to CHAT format
    Lipp2chat {
        /// Path to LIPP file
        path: PathBuf,

        /// Speaker code (default: CHI)
        #[arg(short, long, default_value = "CHI")]
        speaker: String,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert ELAN EAF file to CHAT format
    Elan2chat {
        /// Path to ELAN EAF file
        path: PathBuf,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert Praat TextGrid to CHAT format
    Praat2chat {
        /// Path to Praat TextGrid file
        path: PathBuf,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert CHAT file to Praat TextGrid format
    Chat2praat {
        /// Path to CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert LENA ITS device output to CHAT format
    Lena2chat {
        /// Path to LENA ITS file
        path: PathBuf,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert PLAY annotation to CHAT format
    Play2chat {
        /// Path to PLAY file
        path: PathBuf,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert LAB timing labels to CHAT format
    Lab2chat {
        /// Path to LAB file
        path: PathBuf,

        /// Speaker code (default: SPK)
        #[arg(short, long, default_value = "SPK")]
        speaker: String,

        /// Language code (default: eng)
        #[arg(short = 'L', long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert RTF file to CHAT format
    Rtf2chat {
        /// Path to RTF file
        path: PathBuf,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert SALT transcription to CHAT format
    Salt2chat {
        /// Path to SALT file
        path: PathBuf,

        /// Language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Extract gem segments (@Bg/@Eg bounded regions)
    Gem {
        /// Path to input CHAT file
        path: PathBuf,

        /// Gem labels to extract (if empty, extract all)
        #[arg(short, long)]
        gem: Vec<String>,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert CHAT file to ELAN EAF format
    Chat2elan {
        /// Path to CHAT file
        path: PathBuf,

        /// Media file extension (e.g., wav, mp4)
        #[arg(short, long)]
        media_extension: Option<String>,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Language sample evaluation with dialect support
    #[command(
        name = "eval-d",
        about = "Language sample evaluation with dialect support (EVAL variant)"
    )]
    EvalD {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },

    /// Morphological analysis, deliberately not implemented
    Mor {},

    /// POS disambiguation, deliberately not implemented
    Post {},

    /// Grammar relation parsing, deliberately not implemented
    Megrasp {},

    /// List POST database contents, deliberately not implemented
    Postlist {},

    /// Modify POST database rules, deliberately not implemented
    Postmodrules {},

    /// Train POST model, deliberately not implemented
    Posttrain {},

    // -- Compatibility aliases (CLAN command names) --
    /// Validate CHAT file(s) with CLAN CHECK-compatible output and flags
    #[command(about = "Validate CHAT file(s) (CLAN 'check' command)")]
    Check {
        /// Path to CHAT file(s) or directory (required unless --list-errors)
        paths: Vec<PathBuf>,

        /// Check bullet consistency (0=full, 1=missing only)
        #[arg(long)]
        bullets: Option<u8>,

        /// Only report this error number (can repeat)
        #[arg(long = "error", short = 'e')]
        include_errors: Vec<u16>,

        /// Exclude this error number (can repeat)
        #[arg(long = "exclude-error")]
        exclude_errors: Vec<u16>,

        /// List all error numbers and their messages
        #[arg(long)]
        list_errors: bool,

        /// Check for "CHI Target_Child" in @Participants (+g2)
        #[arg(long)]
        check_target: bool,

        /// Check for missing @ID tiers (+g4, on by default)
        #[arg(long)]
        check_id: Option<bool>,

        /// Check for unused speakers (+g5)
        #[arg(long)]
        check_unused: bool,

        /// Validate UD features on %mor tier (+u)
        #[arg(long)]
        check_ud: bool,
    },

    /// Normalize CHAT file, CLAN compatibility alias for `chatter normalize` (tier reordering + line wrapping)
    #[command(about = "Normalize CHAT file (CLAN 'fixit' equivalent, same as `chatter normalize`)")]
    Fixit {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Align CA overlap markers (⌈/⌊) by column position
    #[command(about = "Align CA overlap markers by column position")]
    Indent {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Normalize CHAT file, CLAN compatibility alias for `chatter normalize` (join continuation lines)
    #[command(
        about = "Normalize CHAT file (CLAN 'longtier' equivalent, same as `chatter normalize`)"
    )]
    Longtier {
        /// Path to input CHAT file
        path: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Word frequency within gem segments, CLAN compatibility alias for `freq --gem`
    #[command(
        about = "Word frequency within gem segments (CLAN 'gemfreq' equivalent, same as `freq --gem`)",
        group(ArgGroup::new("gemfreq-required-gem").args(["gem"]).required(true))
    )]
    Gemfreq {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Count morphemes from %mor tier instead of words from main tier
        #[arg(long)]
        mor: bool,

        #[command(flatten)]
        common: CommonAnalysisArgs,
    },
}
