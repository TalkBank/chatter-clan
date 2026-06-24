//! FREQ token-counting engine: per-utterance accumulation into a speaker's
//! frequency table, dispatched on the configured [`CountSource`](super::CountSource).
//!
//! Extracted verbatim from `freq/mod.rs`. The `AnalysisCommand` impl in the
//! parent calls [`FreqCommand::count_utterance`](super::FreqCommand::count_utterance)
//! (re-exposed `pub(super)` for that reason); every other method here is private
//! to this module. The CLAN-source citations for each `+`-flag effect are kept
//! inline.

use std::collections::BTreeSet;

use talkbank_model::Utterance;

use super::{CountSource, FreqCommand, IncludeMultiplicity, MultiWordDisplay, SpeakerFreq};
use crate::framework::word_filter::countable_words_with_mode;
use crate::framework::{
    NormalizedWord, RetraceReplaceMode, TierKind, WordCount, dependent_tier_tokens, parans_display,
    parans_normalized_key,
};

/// Whether a dependent-tier token is dropped by CLAN's default exclude-word list
/// for FREQ (`+t%X` counting). CLAN seeds this list at FREQ init
/// (`freq.cpp:405-409`): the explicit `+?` / `+!`, then `mor_initwords`
/// (`cutt.cpp:8827-8833`) and `gra_initwords` (`cutt.cpp:8835-8840`, whose ONLY
/// caller is FREQ). Each is matched against every token via CLAN's `exclude()`,
/// so a matching token is never counted. (The bare `.` terminator is dropped
/// earlier, in [`dependent_tier_tokens`], matching CLAN's `getword`.)
///
/// CLAN's patterns are anchored (`beg|*` is a prefix, `*|*|PUNCT` a suffix), so
/// exact prefix/suffix tests reproduce them without a general wildcard matcher.
fn is_clan_default_excluded_token(tier: &TierKind, token: &str) -> bool {
    // Bare CHAT terminators. The split between this layer and the tokenizer is
    // faithful to CLAN's two mechanisms: `.` is dropped one layer down in
    // `dependent_tier_tokens` (CLAN's getword/tokenizer level, shared by every
    // dependent-tier consumer), whereas `?`/`!` are FREQ's own explicit init
    // excludes (`+?`/`+!`, freq.cpp:405-407), FREQ-only, so they belong here.
    // A `%mor` line may end in `?`/`!`.
    if matches!(token, "?" | "!") {
        return true;
    }
    match tier {
        // gra_initwords (cutt.cpp:8836-8839): grammatical-punctuation relations
        // `*|*|{BEGP,ENDP,LP,PUNCT}`. The label is the last `|`-segment.
        TierKind::Gra => {
            const GRA_EXCLUDED_LABELS: [&str; 4] = ["PUNCT", "BEGP", "ENDP", "LP"];
            token
                .rsplit('|')
                .next()
                .is_some_and(|label| GRA_EXCLUDED_LABELS.contains(&label))
        }
        // mor_initwords (cutt.cpp:8827-8833): `%mor` markup categories
        // `{beg,end,cm,bq,eq,bq2,eq2}|*`. The category is the first `|`-segment.
        TierKind::Mor => {
            const MOR_EXCLUDED_CATEGORIES: [&str; 7] =
                ["beg", "end", "cm", "bq", "eq", "bq2", "eq2"];
            token
                .split('|')
                .next()
                .is_some_and(|category| MOR_EXCLUDED_CATEGORIES.contains(&category))
        }
        // No CLAN-seeded default excludes for other dependent tiers.
        _ => false,
    }
}

impl FreqCommand {
    /// Record one already-extracted token (a structural `%mor` item, a
    /// post-clitic, or a dependent-tier `+t%X` token), keyed by its raw CHAT
    /// form, when it passes FREQ's `+c` capitalization filter and per-word
    /// `+s`/`-s` filter (both empty = pass-all, applied at emit time, not the
    /// utterance gate). Shared by the `MorStructural` and `DependentTierTokens`
    /// count paths; the `MainTier` path records itself because it additionally
    /// carries `+c2` multiplicity and CLAN display forms.
    fn record_filtered_token(
        &self,
        text: &str,
        speaker_freq: &mut SpeakerFreq,
        collect_order: bool,
    ) {
        if self.config.capitalization.includes(text) && self.config.word_filter.word_matches(text) {
            let key = NormalizedWord::from_text_cased(text, self.config.case_sensitive);
            speaker_freq.record(key, 1, collect_order);
        }
    }

    /// Count one utterance's tokens into a single speaker accumulator,
    /// dispatching on the configured [`CountSource`]. Shared by the cross-file
    /// and per-file tables so the spreadsheet and stdout paths count identically.
    pub(super) fn count_utterance(&self, utterance: &Utterance, speaker_freq: &mut SpeakerFreq) {
        // Only retain the ordered token stream when MATTR (`+bN`) needs it.
        let collect_order = self.config.frame_size.is_some();
        match &self.config.count_source {
            CountSource::MainTier => self.count_main_tier(utterance, speaker_freq, collect_order),
            CountSource::MorStructural => {
                self.count_mor_structural(utterance, speaker_freq, collect_order)
            }
            CountSource::DependentTierTokens(tier) => {
                self.count_dependent_tier(utterance, tier, speaker_freq, collect_order)
            }
            // CLAN `-t%X` (the EXCLUDE form): count the main tier PLUS every
            // present dependent tier EXCEPT the named set, pooled into one table
            // (banner: "ALL speaker tiers / and those speakers' ALL dependent
            // tiers EXCEPT ..."). A composition of `count_main_tier` and
            // `count_dependent_tier`. Each distinct present tier kind is counted
            // once (`dependent_tier_tokens` already aggregates a kind). The "%mor
            // line forms" advisory stays on (see `MainPlusDependentTiersExcept`).
            CountSource::MainPlusDependentTiersExcept(excluded) => {
                self.count_main_tier(utterance, speaker_freq, collect_order);
                // Iterate DISTINCT present kinds: `count_dependent_tier` ->
                // `dependent_tier_tokens` already aggregates every tier of a kind,
                // so a set (not the raw tier list) avoids double-counting a kind
                // that appears twice. (`chatter validate` rejects duplicate-kind
                // tiers, but FREQ may run on unvalidated input.)
                let mut counted: BTreeSet<TierKind> = BTreeSet::new();
                for dep in &utterance.dependent_tiers {
                    let kind = TierKind::from(dep.kind());
                    if !excluded.contains(&kind) && counted.insert(kind.clone()) {
                        self.count_dependent_tier(utterance, &kind, speaker_freq, collect_order);
                    }
                }
            }
        }
    }

    /// chatter `--mor`: count `%mor` morphemes structurally, each `MorWord` (its
    /// main item and each post-clitic) a separate frequency item, matching CLAN's
    /// space-separated token counting on `%mor`, using the CHAT representation as
    /// the key.
    fn count_mor_structural(
        &self,
        utterance: &Utterance,
        speaker_freq: &mut SpeakerFreq,
        collect_order: bool,
    ) {
        if let Some(mor_tier) = utterance.mor_tier() {
            for mor_item in mor_tier.items().iter() {
                let mut raw = String::new();
                let _ = mor_item.main.write_chat(&mut raw);
                self.record_filtered_token(&raw, speaker_freq, collect_order);

                // Post-clitics are separate frequency items in CLAN.
                for clitic in &mor_item.post_clitics {
                    let mut craw = String::new();
                    let _ = clitic.write_chat(&mut craw);
                    self.record_filtered_token(&craw, speaker_freq, collect_order);
                }
            }
        }
    }

    /// CLAN `+t%X`: count the whitespace-delimited tokens of dependent tier `%X`
    /// (a clitic `v|go~aux|be` is ONE token; the bare `.` terminator is dropped
    /// in [`dependent_tier_tokens`]), minus CLAN's per-tier default-exclude list.
    fn count_dependent_tier(
        &self,
        utterance: &Utterance,
        tier: &TierKind,
        speaker_freq: &mut SpeakerFreq,
        collect_order: bool,
    ) {
        for token in dependent_tier_tokens(utterance, tier) {
            // CLAN seeds a default exclude-word list at FREQ init
            // (freq.cpp:405-409): bare `?`/`!`, the `mor_initwords` markup
            // categories, and the `gra_initwords` punctuation relations. A token
            // matching one is dropped before counting.
            if is_clan_default_excluded_token(tier, &token) {
                continue;
            }
            self.record_filtered_token(&token, speaker_freq, collect_order);
        }
    }

    /// Default / `-t%X` main-tier counting: countable main-tier words (with `+c2`
    /// multiplicity and CLAN display forms) plus the multi-word `+s` group pass.
    fn count_main_tier(
        &self,
        utterance: &Utterance,
        speaker_freq: &mut SpeakerFreq,
        collect_order: bool,
    ) {
        let cap_filter = self.config.capitalization;
        let case_sensitive = self.config.case_sensitive;
        let word_filter = &self.config.word_filter;
        // Count words from the main tier under the CLAN `+r5`/`+r6` modes: `+r6`
        // (`include_retracings`) additionally counts each retracing's retraced
        // word, `+r5` (`replacement_mode`) chooses replacement vs original for
        // `[: text]`. The default mode is byte-identical to `countable_words`.
        let word_mode = RetraceReplaceMode {
            include_retracings: self.config.include_retracings,
            replacement: self.config.replacement_mode,
        };
        for word in countable_words_with_mode(&utterance.main.content.content, word_mode) {
            if !cap_filter.includes(word.cleaned_text()) {
                continue;
            }
            // CLAN's `+sWORD` / `-sWORD` per-word filter, empty = pass-all.
            // CLAN `+c2` counts a word once per matching `+s` pattern; the
            // default counts it once if any pattern matches.
            let multiplicity: u64 = match self.config.include_multiplicity {
                IncludeMultiplicity::Once => {
                    u64::from(word_filter.word_matches(word.cleaned_text()))
                }
                IncludeMultiplicity::PerPattern => {
                    word_filter.count_matching_includes(word.cleaned_text()) as u64
                }
            };
            if multiplicity == 0 {
                continue;
            }
            // CLAN `+r1`/`+r2`/`+r3` (`Parans`): the parenthesis mode drives BOTH
            // the grouping key and the displayed form so they stay consistent
            // (default `+r1` renders `bein(g)` -> `being` for both). The display
            // tracks the same case treatment as the key: preserved when
            // `case_sensitive`, folded to lowercase otherwise (CLAN's `nomap`
            // lowercases the word text, freq.cpp:1892-1909).
            let parens = self.config.parenthesis_mode;
            let prosody = self.config.prosody_mode;
            let word_delimiters = &self.config.word_delimiters;
            if word_delimiters.is_empty() {
                // Default path (no `+p`): key and display from the AST word,
                // byte-identical to the pre-`+pS` behaviour.
                let key = parans_normalized_key(word, parens, prosody, case_sensitive);
                speaker_freq
                    .display_forms
                    .entry(key.clone())
                    .or_insert_with(|| parans_display(word, parens, prosody, case_sensitive));
                speaker_freq.record(key, multiplicity, collect_order);
            } else {
                // CLAN `+pS` (cutt.cpp:9798-9818): split the word form on the
                // extra delimiter characters and count each segment on its own.
                // The split applies to the rendered form (after the `+r`
                // treatment), so a trailing word-form marker (`@o`) stays on the
                // final segment, matching CLAN's re-tokenization.
                let display = parans_display(word, parens, prosody, case_sensitive);
                for segment in word_delimiters.split(&display) {
                    speaker_freq.record_with_display(
                        segment,
                        case_sensitive,
                        multiplicity,
                        collect_order,
                    );
                }
            }
        }

        // CLAN multi-word `+s` search (freq.cpp:2465-2548): a `+s` value
        // with more than one word is a GROUP matched as an adjacent,
        // in-order, non-overlapping sequence over this utterance's tokens;
        // each match is one frequency item. The groups are parsed once in
        // `FreqCommand::new`, so single-word-only `+s` runs leave
        // `multiword_groups` empty and skip this pass (and its token
        // collection) entirely. A multi-word pattern never matches a single
        // token in the per-word loop above, so the two passes are additive.
        // (Phase 1: main tier only; `%mor`/`%gra` group search is later.)
        if !self.multiword_groups.is_empty() {
            let tokens: Vec<&str> =
                countable_words_with_mode(&utterance.main.content.content, word_mode)
                    .map(|w| w.cleaned_text())
                    .collect();
            // Each `+s` match is recorded as one frequency item under its
            // display string (shared by the two `+c7` display modes), via the
            // same `record_with_display` the `+pS` segments use.
            for group in &self.multiword_groups {
                let matches = group.matches(&tokens, self.config.multiword_match);
                if matches.is_empty() {
                    continue;
                }
                match self.config.multiword_display {
                    // Default: one item per group, the search pattern (CLAN
                    // counts under `word_arr` joined by spaces).
                    MultiWordDisplay::Pattern => {
                        speaker_freq.record_with_display(
                            group.display(),
                            case_sensitive,
                            matches.len() as WordCount,
                            collect_order,
                        );
                    }
                    // CLAN `+c7`: one item per match, keyed by the actual
                    // matched words (`isMultiWordsActual`, freq.cpp:2444).
                    MultiWordDisplay::MatchedWords => {
                        for m in &matches {
                            speaker_freq.record_with_display(
                                &m.matched_words(&tokens).join(" "),
                                case_sensitive,
                                1,
                                collect_order,
                            );
                        }
                    }
                }
            }
        }
    }
}
