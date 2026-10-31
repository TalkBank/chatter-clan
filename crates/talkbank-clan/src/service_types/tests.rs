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
        AnalysisOptions::Chip(ChipOptions::default()).command_name(),
        AnalysisCommandName::Chip
    );
    assert_eq!(
        AnalysisOptions::Complexity.command_name(),
        AnalysisCommandName::Complexity
    );
}

#[test]
fn builder_validates_chip_speaker_roles_and_window() {
    let options = ChipOptions {
        adults: vec!["MOT".to_owned()],
        children: vec!["CHI".to_owned()],
        window: Some(4),
        case_mode: crate::commands::chip::ChipCaseMode::Fold,
        ..ChipOptions::default()
    };
    let plan = AnalysisRequestBuilder::new(AnalysisOptions::Chip(options))
        .build()
        .expect("valid CHIP options should build");

    match plan {
        AnalysisPlan::Service(AnalysisRequest::Chip(config)) => {
            assert_eq!(config.adults()[0].as_str(), "MOT");
            assert_eq!(config.children()[0].as_str(), "CHI");
            assert_eq!(config.window.capacity(), 4);
            assert_eq!(config.case_mode, crate::commands::chip::ChipCaseMode::Fold);
        }
        other => panic!("unexpected plan: {other:?}"),
    }
}

#[test]
fn builder_preserves_mlu_id_row_mode() {
    let plan = AnalysisRequestBuilder::new(AnalysisOptions::Mlu(MluOptions {
        output_mode: crate::commands::mlu::MluOutputMode::IdRowsCombinedFiles,
        ..MluOptions::default()
    }))
    .build()
    .expect("valid MLU options should build");

    match plan {
        AnalysisPlan::Service(AnalysisRequest::Mlu(config)) => {
            assert_eq!(
                config.output_mode,
                crate::commands::mlu::MluOutputMode::IdRowsCombinedFiles
            );
        }
        other => panic!("unexpected plan: {other:?}"),
    }
}

#[test]
fn builder_rejects_unsupported_chip_exclusion_syntax() {
    let options = ChipOptions {
        exclusion_patterns: vec![crate::framework::WordPattern::from("n|go")],
        ..ChipOptions::default()
    };
    let result = AnalysisRequestBuilder::new(AnalysisOptions::Chip(options)).build();

    assert!(matches!(
        result,
        Err(AnalysisServiceError::InvalidRequest(message))
            if message.contains("unsupported CHIP -h pattern")
    ));
}

#[test]
fn chip_plus_k_toggles_the_preserve_case_default() {
    assert!(AnalysisCommandName::Chip.preserves_case_by_default());
    assert!(AnalysisCommandName::Chip.effective_case_sensitive(false));
    assert!(!AnalysisCommandName::Chip.effective_case_sensitive(true));
}

#[test]
fn builder_rejects_conflicting_chip_speaker_roles() {
    let options = ChipOptions {
        adults: vec!["CHI".to_owned()],
        children: vec!["CHI".to_owned()],
        ..ChipOptions::default()
    };
    let result = AnalysisRequestBuilder::new(AnalysisOptions::Chip(options)).build();
    assert!(matches!(
        result,
        Err(AnalysisServiceError::InvalidRequest(_))
    ));
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
