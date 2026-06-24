//! Builder that turns raw analysis options into typed service requests.

use crate::commands::chains::ChainsConfig;
use crate::commands::codes::CodesConfig;
use crate::commands::combo::{ComboConfig, SearchExpr};
use crate::commands::cooccur::CooccurConfig;
use crate::commands::corelex::CorelexConfig;
use crate::commands::dist::DistConfig;
use crate::commands::dss::DssConfig;
use crate::commands::eval::{EvalConfig, EvalVariant};
use crate::commands::flucalc::FlucalcConfig;
use crate::commands::freq::FreqConfig;
use crate::commands::freqpos::FreqposConfig;
use crate::commands::ipsyn::IpsynConfig;
use crate::commands::keymap::KeymapConfig;
use crate::commands::kideval::KidevalConfig;
use crate::commands::kwal::KwalConfig;
use crate::commands::maxwd::MaxwdConfig;
use crate::commands::mlt::MltConfig;
use crate::commands::mlu::MluConfig;
use crate::commands::mortable::MortableConfig;
use crate::commands::rely::RelyConfig;
use crate::commands::script::ScriptConfig;
use crate::commands::sugar::SugarConfig;
use crate::commands::trnfix::TrnfixConfig;
use crate::commands::uniq::UniqConfig;
use crate::commands::vocd::VocdConfig;
use crate::commands::wdsize::WdsizeConfig;

use super::{AnalysisOptions, AnalysisPlan, AnalysisRequest, AnalysisServiceError, RelyRequest};

/// Builder that turns raw outer-layer options into typed library requests.
pub struct AnalysisRequestBuilder {
    options: AnalysisOptions,
}

impl AnalysisRequestBuilder {
    /// Create a builder. The variant encodes the command identity;
    /// callers that previously passed an `AnalysisCommandName`
    /// separately should construct the matching variant instead.
    pub fn new(options: AnalysisOptions) -> Self {
        Self { options }
    }

    /// Validate, apply library defaults, and build the analysis plan.
    pub fn build(self) -> Result<AnalysisPlan, AnalysisServiceError> {
        match self.options {
            AnalysisOptions::Freq(o) => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Freq(FreqConfig {
                    count_source: o.count_source,
                    capitalization: o.capitalization,
                    sort: o.sort,
                    word_list_only: o.word_list_only,
                    types_tokens_only: o.types_tokens_only,
                    // Already the effective preserve-case state: the `+k`
                    // polarity is resolved per command at the dispatch boundary
                    // (AnalysisCommandName::effective_case_sensitive).
                    case_sensitive: o.case_sensitive,
                    word_filter: o.word_filter,
                    spreadsheet: o.spreadsheet,
                    frame_size: o.frame_size,
                    multiword_match: o.multiword_match,
                    include_multiplicity: o.include_multiplicity,
                    multiword_display: o.multiword_display,
                    include_zero_frequency: o.include_zero_frequency,
                    combine_speakers: o.combine_speakers,
                    parenthesis_mode: o.parenthesis_mode,
                    prosody_mode: o.prosody_mode,
                    include_retracings: o.include_retracings,
                    replacement_mode: o.replacement_mode,
                    word_delimiters: o.word_delimiters,
                })))
            }
            AnalysisOptions::Mlu(o) => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Mlu(MluConfig {
                    words_only: o.words,
                    solo_word_exclusions: o.solo_word_exclusions,
                    combine_speakers: o.combine_speakers,
                    // `+sxxx`/`+syyy` re-admit the xxx/yyy utterances; `www` is
                    // never re-includable. Mapping owned by the mlu module.
                    re_included_untranscribed: crate::commands::mlu::re_included_untranscribed(
                        o.include_xxx,
                        o.include_yyy,
                    ),
                })))
            }
            AnalysisOptions::Mlt(o) => Ok(AnalysisPlan::Service(AnalysisRequest::Mlt(MltConfig {
                solo_word_exclusions: o.solo_word_exclusions,
            }))),
            AnalysisOptions::Wdlen => Ok(AnalysisPlan::Service(AnalysisRequest::Wdlen)),
            AnalysisOptions::Wdsize(o) => Ok(AnalysisPlan::Service(AnalysisRequest::Wdsize(
                WdsizeConfig {
                    use_main_tier: o.main_tier,
                    length_filter: o.length_filter,
                },
            ))),
            AnalysisOptions::Maxwd(o) => {
                let default = MaxwdConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Maxwd(MaxwdConfig {
                    limit: o.limit.unwrap_or(default.limit),
                    unique_length_only: o.unique_length_only,
                    exclude_lengths: o.exclude_lengths,
                    case_sensitive: o.case_sensitive,
                })))
            }
            AnalysisOptions::Freqpos(o) => Ok(AnalysisPlan::Service(AnalysisRequest::Freqpos(
                FreqposConfig {
                    position_classification: o.position_classification,
                    case_sensitive: o.case_sensitive,
                },
            ))),
            AnalysisOptions::Timedur => Ok(AnalysisPlan::Service(AnalysisRequest::Timedur)),
            AnalysisOptions::Kwal(o) => {
                Ok(AnalysisPlan::Service(AnalysisRequest::kwal(KwalConfig {
                    keywords: o.keywords,
                    strict_match: o.strict_match,
                    case_sensitive: o.case_sensitive,
                    legal_chat: o.legal_chat,
                    context_before: o.context_before,
                    context_after: o.context_after,
                })?))
            }
            AnalysisOptions::Gemlist => Ok(AnalysisPlan::Service(AnalysisRequest::Gemlist)),
            AnalysisOptions::Combo(o) => {
                let case_sensitive = o.case_sensitive;
                let search: Vec<SearchExpr> = o
                    .search
                    .iter()
                    .map(|expr| SearchExpr::parse_with_case(expr, case_sensitive))
                    .collect();
                if search.is_empty() {
                    return Err(AnalysisServiceError::InvalidRequest(
                        "combo requires at least one search expression".to_owned(),
                    ));
                }
                let exclude: Vec<SearchExpr> = o
                    .exclude_search
                    .iter()
                    .map(|expr| SearchExpr::parse_with_case(expr, case_sensitive))
                    .collect();
                Ok(AnalysisPlan::Service(AnalysisRequest::Combo(ComboConfig {
                    search,
                    exclude,
                    first_match_only: o.first_match_only,
                    dedupe_matches: o.dedupe_matches,
                    case_sensitive,
                    context_before: o.context_before,
                    context_after: o.context_after,
                })))
            }
            AnalysisOptions::Cooccur(o) => {
                let default = CooccurConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Cooccur(
                    CooccurConfig {
                        no_frequency_counts: o.no_frequency_counts,
                        cluster_size: if o.cluster_size == 0 {
                            default.cluster_size
                        } else {
                            o.cluster_size
                        },
                    },
                )))
            }
            AnalysisOptions::Dist(o) => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Dist(DistConfig {
                    once_per_turn: o.once_per_turn,
                    case_sensitive: o.case_sensitive,
                })))
            }
            AnalysisOptions::Chip => Ok(AnalysisPlan::Service(AnalysisRequest::Chip)),
            AnalysisOptions::Phonfreq => Ok(AnalysisPlan::Service(AnalysisRequest::Phonfreq)),
            AnalysisOptions::Modrep => Ok(AnalysisPlan::Service(AnalysisRequest::Modrep)),
            AnalysisOptions::Vocd(o) => {
                let default = VocdConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Vocd(VocdConfig {
                    capitalization: o.capitalization,
                    // Already the effective preserve-case state: the `+k`
                    // polarity is resolved per command at the dispatch boundary
                    // (AnalysisCommandName::effective_case_sensitive).
                    case_sensitive: o.case_sensitive,
                    ..default
                })))
            }
            AnalysisOptions::Codes(o) => {
                let default = CodesConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Codes(CodesConfig {
                    max_depth: o.max_depth.unwrap_or(default.max_depth),
                })))
            }
            AnalysisOptions::Chains(o) => {
                let default = ChainsConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Chains(
                    ChainsConfig {
                        tier: o.tier.unwrap_or(default.tier),
                    },
                )))
            }
            AnalysisOptions::Complexity => Ok(AnalysisPlan::Service(AnalysisRequest::Complexity)),
            AnalysisOptions::Corelex(o) => {
                let default = CorelexConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Corelex(
                    CorelexConfig {
                        min_frequency: o.threshold.unwrap_or(default.min_frequency),
                    },
                )))
            }
            AnalysisOptions::Dss(o) => {
                let default = DssConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Dss(DssConfig {
                    rules_path: o.rules_path,
                    max_utterances: o.max_utterances.unwrap_or(default.max_utterances),
                })))
            }
            AnalysisOptions::Eval(o) => {
                let default = EvalConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Eval(EvalConfig {
                    database_path: o.database_path,
                    database_filter: o.database_filter,
                    ..default
                })))
            }
            AnalysisOptions::EvalDialect(o) => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Eval(EvalConfig {
                    database_path: o.database_path,
                    database_filter: o.database_filter,
                    variant: EvalVariant::Dialect,
                })))
            }
            AnalysisOptions::Flucalc(o) => Ok(AnalysisPlan::Service(AnalysisRequest::Flucalc(
                FlucalcConfig {
                    syllable_mode: o.syllable_mode,
                },
            ))),
            AnalysisOptions::Ipsyn(o) => {
                let default = IpsynConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Ipsyn(IpsynConfig {
                    rules_path: o.rules_path,
                    max_utterances: o.max_utterances.unwrap_or(default.max_utterances),
                })))
            }
            AnalysisOptions::Keymap(o) => {
                let tier = o.tier.unwrap_or_else(|| KeymapConfig::default().tier);
                Ok(AnalysisPlan::Service(AnalysisRequest::keymap(
                    o.keywords, tier,
                )?))
            }
            AnalysisOptions::Kideval(o) => {
                let default = KidevalConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Kideval(
                    KidevalConfig {
                        dss_rules_path: o.dss_rules_path,
                        ipsyn_rules_path: o.ipsyn_rules_path,
                        dss_max_utterances: o
                            .dss_max_utterances
                            .unwrap_or(default.dss_max_utterances),
                        ipsyn_max_utterances: o
                            .ipsyn_max_utterances
                            .unwrap_or(default.ipsyn_max_utterances),
                        database_path: o.database_path,
                        database_filter: o.database_filter,
                    },
                )))
            }
            AnalysisOptions::Mortable(o) => {
                let script_path = o.script_path.ok_or_else(|| {
                    AnalysisServiceError::InvalidRequest(
                        "mortable requires a scriptPath option".to_owned(),
                    )
                })?;
                Ok(AnalysisPlan::Service(AnalysisRequest::Mortable(
                    MortableConfig { script_path },
                )))
            }
            AnalysisOptions::Rely(o) => {
                let secondary_file = o.second_file.ok_or_else(|| {
                    AnalysisServiceError::InvalidRequest(
                        "rely requires a secondFile option".to_owned(),
                    )
                })?;
                let tier = o.tier.unwrap_or_else(|| RelyConfig::default().tier);
                Ok(AnalysisPlan::Rely(RelyRequest {
                    secondary_file,
                    config: RelyConfig { tier },
                }))
            }
            AnalysisOptions::Script(o) => {
                let template_path = o.template_path.ok_or_else(|| {
                    AnalysisServiceError::InvalidRequest(
                        "script requires a templatePath option".to_owned(),
                    )
                })?;
                Ok(AnalysisPlan::Service(AnalysisRequest::Script(
                    ScriptConfig { template_path },
                )))
            }
            AnalysisOptions::Sugar(o) => {
                let default = SugarConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Sugar(SugarConfig {
                    min_utterances: o.min_utterances.unwrap_or(default.min_utterances),
                })))
            }
            AnalysisOptions::Trnfix(o) => {
                let default = TrnfixConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Trnfix(
                    TrnfixConfig {
                        tier1: o.tier1.unwrap_or(default.tier1),
                        tier2: o.tier2.unwrap_or(default.tier2),
                    },
                )))
            }
            AnalysisOptions::Uniq(o) => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Uniq(UniqConfig {
                    sort_by_frequency: o.sort_by_frequency,
                })))
            }
        }
    }
}
