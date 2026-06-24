use super::*;

#[test]
fn analysis_command_name_all_is_complete() {
    assert_eq!(AnalysisCommandName::ALL.len(), 34);
}

#[test]
fn analysis_command_name_all_round_trips_wire_names() {
    for name in AnalysisCommandName::ALL {
        let wire = name.to_string();
        assert_eq!(wire.parse::<AnalysisCommandName>(), Ok(*name));
    }
}

/// `AnalysisOptions::command_name` derives the `AnalysisCommandName`
/// from the variant. Spot-check several variants to pin the mapping.
#[test]
fn analysis_options_command_name_freq() {
    let opts = AnalysisOptions::Freq(FreqOptions::default());
    assert_eq!(opts.command_name(), AnalysisCommandName::Freq);
}

#[test]
fn analysis_options_command_name_combo() {
    let opts = AnalysisOptions::Combo(ComboOptions::default());
    assert_eq!(opts.command_name(), AnalysisCommandName::Combo);
}

#[test]
fn analysis_options_command_name_dist() {
    let opts = AnalysisOptions::Dist(DistOptions::default());
    assert_eq!(opts.command_name(), AnalysisCommandName::Dist);
}

#[test]
fn analysis_options_command_name_unit_variants() {
    assert_eq!(
        AnalysisOptions::Wdlen.command_name(),
        AnalysisCommandName::Wdlen
    );
    assert_eq!(
        AnalysisOptions::Cooccur(CooccurOptions::default()).command_name(),
        AnalysisCommandName::Cooccur,
    );
    assert_eq!(
        AnalysisOptions::Chip.command_name(),
        AnalysisCommandName::Chip
    );
    assert_eq!(
        AnalysisOptions::Complexity.command_name(),
        AnalysisCommandName::Complexity
    );
}

#[test]
fn analysis_options_command_name_distinguishes_eval_dialect() {
    let plain = AnalysisOptions::Eval(EvalOptions::default());
    let dialect = AnalysisOptions::EvalDialect(EvalOptions::default());
    assert_eq!(plain.command_name(), AnalysisCommandName::Eval);
    assert_eq!(dialect.command_name(), AnalysisCommandName::EvalDialect);
}

#[test]
fn builder_threads_dist_once_per_turn() {
    let plan = AnalysisRequestBuilder::new(AnalysisOptions::Dist(DistOptions {
        once_per_turn: true,
        case_sensitive: false,
    }))
    .build()
    .expect("dist should build");

    match plan {
        AnalysisPlan::Service(AnalysisRequest::Dist(config)) => {
            assert!(config.once_per_turn);
        }
        other => panic!("unexpected plan: {other:?}"),
    }
}

#[test]
fn builder_threads_freq_capitalization() {
    let plan = AnalysisRequestBuilder::new(AnalysisOptions::Freq(FreqOptions {
        count_source: crate::commands::freq::CountSource::MainTier,
        capitalization: crate::framework::CapitalizationFilter::MidUpper,
        sort: crate::commands::freq::FreqSort::Alphabetical,
        word_list_only: false,
        types_tokens_only: false,
        case_sensitive: false,
        word_filter: Default::default(),
        spreadsheet: None,
        frame_size: None,
        multiword_match: Default::default(),
        include_multiplicity: Default::default(),
        multiword_display: Default::default(),
        include_zero_frequency: false,
        combine_speakers: false,
        parenthesis_mode: crate::framework::ParenthesisMode::default(),
        prosody_mode: crate::framework::ProsodyMode::default(),
        include_retracings: false,
        replacement_mode: crate::framework::ReplacementChoice::default(),
        word_delimiters: crate::framework::WordDelimiters::default(),
    }))
    .build()
    .expect("freq should build");

    match plan {
        AnalysisPlan::Service(AnalysisRequest::Freq(config)) => {
            assert_eq!(
                config.capitalization,
                crate::framework::CapitalizationFilter::MidUpper
            );
        }
        other => panic!("unexpected plan: {other:?}"),
    }
}

#[test]
fn builder_threads_vocd_capitalization() {
    let plan = AnalysisRequestBuilder::new(AnalysisOptions::Vocd(VocdOptions {
        capitalization: crate::framework::CapitalizationFilter::InitialUpper,
        case_sensitive: false,
    }))
    .build()
    .expect("vocd should build");

    match plan {
        AnalysisPlan::Service(AnalysisRequest::Vocd(config)) => {
            assert_eq!(
                config.capitalization,
                crate::framework::CapitalizationFilter::InitialUpper
            );
        }
        other => panic!("unexpected plan: {other:?}"),
    }
}

#[test]
fn builder_threads_combo_flags() {
    let plan = AnalysisRequestBuilder::new(AnalysisOptions::Combo(ComboOptions {
        search: vec!["dog".to_owned()],
        exclude_search: vec!["cat".to_owned()],
        first_match_only: true,
        dedupe_matches: true,
        case_sensitive: false,
        context_before: 1,
        context_after: 2,
    }))
    .build()
    .expect("combo should build");

    match plan {
        AnalysisPlan::Service(AnalysisRequest::Combo(config)) => {
            assert!(config.first_match_only);
            assert!(config.dedupe_matches);
            assert_eq!(config.context_before, 1);
            assert_eq!(config.context_after, 2);
            assert_eq!(config.search.len(), 1);
            assert_eq!(config.exclude.len(), 1);
        }
        other => panic!("unexpected plan: {other:?}"),
    }
}
