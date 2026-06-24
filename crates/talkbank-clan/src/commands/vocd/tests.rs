use rand::SeedableRng;
use rand::rngs::StdRng;
use talkbank_model::Span;
use talkbank_model::{MainTier, Terminator, Utterance, UtteranceContent, Word};

use super::stats::{average_ttr, d_from_ttr, find_min_d, ttr_equation};
use super::*;
use crate::framework::CommandOutput;

/// Typical D/N values should produce a mid-range TTR.
#[test]
fn ttr_equation_basic() {
    let ttr = ttr_equation(100.0, 50);
    assert!(ttr > 0.8 && ttr < 0.9, "TTR={ttr}");
}

/// Very high D should push the theoretical TTR close to 1.
#[test]
fn ttr_equation_high_d() {
    let ttr = ttr_equation(10000.0, 50);
    assert!(ttr > 0.99, "Expected TTR near 1.0, got {ttr}");
}

/// Very low D should push the theoretical TTR close to 0.
#[test]
fn ttr_equation_low_d() {
    let ttr = ttr_equation(0.1, 50);
    assert!(ttr < 0.1, "Expected TTR near 0, got {ttr}");
}

/// Inverse equation should approximately recover the original D.
#[test]
fn d_from_ttr_inverse() {
    let d_original = 50.0;
    let n = 40;
    let ttr = ttr_equation(d_original, n);
    let d_recovered = d_from_ttr(n, ttr);
    assert!(
        (d_original - d_recovered).abs() < 0.01,
        "Expected ~{d_original}, got {d_recovered}"
    );
}

/// Degenerate `TTR == 1.0` should map to the guarded zero-D branch.
#[test]
fn d_from_ttr_handles_ttr_one() {
    let d = d_from_ttr(50, 1.0);
    assert_eq!(d, 0.0);
}

/// Bootstrap averaging should keep TTR means in a valid numeric range.
#[test]
fn average_ttr_produces_valid_range() {
    let tokens: Vec<String> = (0..100).map(|i| format!("word{}", i % 30)).collect();
    let mut rng = StdRng::seed_from_u64(42);
    let (mean, std_dev) = average_ttr(&tokens, 35, 50, &mut rng);

    assert!(mean > 0.0 && mean <= 1.0, "Mean TTR={mean}");
    assert!(std_dev >= 0.0, "Std dev should be non-negative: {std_dev}");
}

/// Finds min d converges.
#[test]
fn find_min_d_converges() {
    let known_d = 60.0;
    let entries: Vec<NtEntry> = (35..=50)
        .map(|n| NtEntry {
            n,
            samples: 100,
            mean_ttr: ttr_equation(known_d, n),
            std_dev: 0.0,
            d_value: d_from_ttr(n, ttr_equation(known_d, n)),
        })
        .collect();

    let (d_opt, min_ls) = find_min_d(known_d, &entries);

    assert!(
        (d_opt - known_d).abs() < 0.1,
        "Expected ~{known_d}, got {d_opt}"
    );
    assert!(min_ls < 0.001, "Expected very small LS error, got {min_ls}");
}

/// Speakers below the sample ceiling should emit warnings, not results.
#[test]
fn vocd_insufficient_tokens_warning() {
    let cmd = VocdCommand::default();
    let mut state = VocdState::default();
    let file_ctx = test_file_context();

    let content: Vec<UtteranceContent> = ["hello", "world", "test"]
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
    let utt = Utterance::new(main);

    cmd.process_utterance(&utt, &file_ctx, &mut state);
    let result = cmd.finalize(state);

    assert!(
        result.speakers.is_empty(),
        "Should have no speakers with enough tokens"
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].speaker, "CHI");
    assert_eq!(result.warnings[0].token_count, 3);
}

/// Adequate token counts should produce full trial output and positive D.
#[test]
fn vocd_with_enough_tokens() {
    let cmd = VocdCommand::new(VocdConfig {
        sample_from: 5,
        sample_to: 10,
        num_samples: 20,
        capitalization: CapitalizationFilter::Any,
        case_sensitive: false,
    });
    let mut state = VocdState::default();
    let file_ctx = test_file_context();

    let word_pool = [
        "the", "dog", "cat", "ran", "big", "small", "house", "tree", "bird", "fish", "walk",
        "jump", "red", "blue", "green", "fast", "slow", "nice", "good", "bad",
    ];

    for _ in 0..3 {
        for chunk in word_pool.chunks(5) {
            let content: Vec<UtteranceContent> = chunk
                .iter()
                .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
                .collect();
            let main = MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY });
            let utt = Utterance::new(main);
            cmd.process_utterance(&utt, &file_ctx, &mut state);
        }
    }

    let result = cmd.finalize(state);

    assert_eq!(result.speakers.len(), 1);
    assert_eq!(result.warnings.len(), 0);

    let speaker = &result.speakers[0];
    assert_eq!(speaker.speaker, "CHI");
    assert_eq!(speaker.tokens, 60);
    assert_eq!(speaker.trials.len(), NUM_TRIALS);
    assert!(
        speaker.d_optimum_average > 0.0,
        "D should be positive, got {}",
        speaker.d_optimum_average
    );
}

/// Text rendering should include both per-trial tables and summary block.
#[test]
fn vocd_render_text_format() {
    let result = VocdResult {
        speakers: vec![VocdSpeakerResult {
            speaker: "CHI".to_string(),
            types: 100,
            tokens: 500,
            ttr: 0.2,
            trials: vec![VocdTrial {
                entries: vec![NtEntry {
                    n: 35,
                    samples: 100,
                    mean_ttr: 0.80,
                    std_dev: 0.05,
                    d_value: 50.0,
                }],
                d_average: 50.0,
                d_std_dev: 2.0,
                d_optimum: 49.5,
                min_least_sq: 0.001,
            }],
            d_optimum_values: vec![49.5],
            d_optimum_average: 49.5,
        }],
        warnings: vec![],
    };

    let text = result.render_text();
    assert!(text.contains("Speaker: *CHI:"));
    assert!(text.contains("D_optimum"));
    assert!(text.contains("VOCD RESULTS SUMMARY"));
    assert!(text.contains("Types,Tokens,TTR"));
    assert!(text.contains("49.50"));
}
