use super::*;
use crate::framework::CommandOutput;
use talkbank_model::Span;
use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

/// Build a minimal utterance with plain words for command tests.
fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
    let content: Vec<UtteranceContent> = words
        .iter()
        .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
        .collect();
    let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
    Utterance::new(main)
}

/// CLAN's `+c1` mode (`CapitalizationFilter::MidUpper`) drops
/// words without an uppercase letter past position 0, so
/// `McDonald` and `iPhone` survive, plain `Cookie` (initial-
/// only) does not.
#[test]
fn freq_mid_upper_filters_initial_only_words() {
    let command = FreqCommand::new(FreqConfig {
        count_source: CountSource::MainTier,
        capitalization: CapitalizationFilter::MidUpper,
        sort: FreqSort::Alphabetical,
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
        parenthesis_mode: ParenthesisMode::default(),
        prosody_mode: ProsodyMode::default(),
        include_retracings: false,
        replacement_mode: ReplacementChoice::default(),
        word_delimiters: Default::default(),
    });
    let mut state = FreqState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // `McDonald` and `iPhone` pass; `I`, `Cookie`, `want`, `a`
    // all fail (either no uppercase at all, or only initial).
    let u = make_utterance(
        "CHI",
        &[
            "I", "want", "a", "Cookie", "from", "McDonald", "on", "iPhone",
        ],
    );
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 1);
    let chi = &result.speakers[0];
    assert_eq!(chi.total_tokens, 2);
    let words: Vec<&str> = chi.entries.iter().map(|e| e.word.as_str()).collect();
    assert!(words.contains(&"mcdonald"));
    assert!(words.contains(&"iphone"));
    assert!(!words.contains(&"cookie"));
    assert!(!words.contains(&"i"));
}

/// `+o1` (`reverse_concordance`) sorts entries by their
/// reversed character sequence, grouping words by suffix.
/// Input `cat`, `bat`, `dog`, `log`: by reverse-concordance
/// the keys become `tac`, `tab`, `god`, `gol`; sorted →
/// `god` (gol), `log` (gol), `bat` (tab), `cat` (tac).
/// Words sharing a suffix cluster together.
#[test]
fn freq_reverse_concordance_groups_by_suffix() {
    let command = FreqCommand::new(FreqConfig {
        count_source: CountSource::MainTier,
        capitalization: CapitalizationFilter::Any,
        sort: FreqSort::ReverseConcordance,
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
        parenthesis_mode: ParenthesisMode::default(),
        prosody_mode: ProsodyMode::default(),
        include_retracings: false,
        replacement_mode: ReplacementChoice::default(),
        word_delimiters: Default::default(),
    });
    let mut state = FreqState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["cat", "bat", "dog", "log"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let words: Vec<&str> = result.speakers[0]
        .entries
        .iter()
        .map(|e| e.word.as_str())
        .collect();
    // Sorted by reversed string: god, gol, tab, tac
    //                  original: dog, log, bat, cat
    assert_eq!(words, vec!["dog", "log", "bat", "cat"]);
}

/// Default sort (frequency descending, alphabetical tiebreak)
/// is unchanged when `reverse_concordance: false`. Companion
/// to the +o1 test for an obvious diff on the same input.
#[test]
fn freq_default_sort_is_alphabetical_when_freqs_equal() {
    let command = FreqCommand::default();
    let mut state = FreqState::default();
    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["cat", "bat", "dog", "log"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let words: Vec<&str> = result.speakers[0]
        .entries
        .iter()
        .map(|e| e.word.as_str())
        .collect();
    // All freqs are 1, so alphabetical tiebreak applies.
    assert_eq!(words, vec!["bat", "cat", "dog", "log"]);
}

/// CLAN's `+c` / `+c0` mode drops words whose first character
/// isn't uppercase. Two capitalized tokens in a mixed utterance
/// should be counted once each; the lower-case tokens disappear
/// from both the token total and the per-type table.
#[test]
fn freq_capitalized_only_filters_lowercase_words() {
    let command = FreqCommand::new(FreqConfig {
        count_source: CountSource::MainTier,
        capitalization: CapitalizationFilter::InitialUpper,
        sort: FreqSort::Alphabetical,
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
        parenthesis_mode: ParenthesisMode::default(),
        prosody_mode: ProsodyMode::default(),
        include_retracings: false,
        replacement_mode: ReplacementChoice::default(),
        word_delimiters: Default::default(),
    });
    let mut state = FreqState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // "I" and "Cookie" pass the filter; "want", "a", "and"
    // do not (lowercase initial); "123" has no leading letter.
    let u = make_utterance("CHI", &["I", "want", "a", "Cookie", "and", "123"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 1);
    let chi = &result.speakers[0];
    assert_eq!(chi.total_tokens, 2);
    assert_eq!(chi.total_types, 2);
    let words: Vec<&str> = chi.entries.iter().map(|e| e.word.as_str()).collect();
    assert!(words.contains(&"i"));
    assert!(words.contains(&"cookie"));
    assert!(!words.contains(&"want"));
    assert!(!words.contains(&"a"));
}

/// Counts should remain isolated per speaker key.
#[test]
fn freq_counts_words_per_speaker() {
    let command = FreqCommand::default();
    let mut state = FreqState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
    let u2 = make_utterance("CHI", &["I", "want", "more"]);
    let u3 = make_utterance("MOT", &["here", "you", "go"]);

    command.process_utterance(&u1, &file_ctx, &mut state);
    command.process_utterance(&u2, &file_ctx, &mut state);
    command.process_utterance(&u3, &file_ctx, &mut state);

    let result = command.finalize(state);
    assert_eq!(result.speakers.len(), 2);

    // CHI section
    let chi = &result.speakers[0];
    assert_eq!(chi.speaker, "CHI");
    assert_eq!(chi.total_tokens, 6);
    assert_eq!(chi.total_types, 4); // i, want, cookie, more

    // MOT section
    let mot = &result.speakers[1];
    assert_eq!(mot.speaker, "MOT");
    assert_eq!(mot.total_tokens, 3);
    assert_eq!(mot.total_types, 3);
}

/// TTR should be computed as `types / tokens`.
#[test]
fn freq_ttr_calculation() {
    let command = FreqCommand::default();
    let mut state = FreqState::default();

    let chat_file = talkbank_model::ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // All same word → TTR = 1/5 = 0.200
    let u = make_utterance("CHI", &["the", "the", "the", "the", "the"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = &result.speakers[0];
    assert!((chi.ttr - 0.2).abs() < 1e-10);
}

/// Text rendering should include speaker summary and token rows.
#[test]
fn freq_render_text_format() {
    let result = FreqResult {
        file_speaker_rows: Vec::new(),
        mattr_enabled: false,
        combine_speakers: false,
        mor_based: false,
        word_list_only: false,
        types_tokens_only: false,
        sort: FreqSort::Alphabetical,
        speakers: vec![FreqSpeakerResult {
            speaker: "CHI".to_owned(),
            entries: vec![
                FreqEntry {
                    word: "want".to_owned(),
                    display_form: None,
                    count: 2,
                },
                FreqEntry {
                    word: "cookie".to_owned(),
                    display_form: None,
                    count: 1,
                },
            ],
            total_types: 2,
            total_tokens: 3,
            ttr: 0.667,
            mattr: None,
        }],
    };

    let text = result.render_text();
    assert!(text.contains("Speaker: CHI"));
    assert!(text.contains("Total types: 2"));
    assert!(text.contains("want"));
    assert!(text.contains("cookie"));
}

/// CLAN rendering should expose legacy-style summary labels.
#[test]
fn freq_render_clan_format() {
    let result = FreqResult {
        file_speaker_rows: Vec::new(),
        mattr_enabled: false,
        combine_speakers: false,
        mor_based: false,
        word_list_only: false,
        types_tokens_only: false,
        sort: FreqSort::Alphabetical,
        speakers: vec![FreqSpeakerResult {
            speaker: "CHI".to_owned(),
            entries: vec![
                FreqEntry {
                    word: "want".to_owned(),
                    display_form: None,
                    count: 2,
                },
                FreqEntry {
                    word: "cookie".to_owned(),
                    display_form: None,
                    count: 1,
                },
            ],
            total_types: 2,
            total_tokens: 3,
            ttr: 0.667,
            mattr: None,
        }],
    };

    let clan = result.render_clan();
    assert!(clan.contains("Speaker: *CHI:"));
    assert!(clan.contains("2 want"));
    assert!(clan.contains("1 cookie"));
    assert!(clan.contains("Total number of different item types used"));
    assert!(clan.contains("Total number of items (tokens)"));
    assert!(clan.contains("Type/Token ratio"));
}

/// FREQ `+d1` / `--word-list-only`: emit one word per line, no
/// frequencies, no per-speaker banners, no totals. Output is
/// meant to be usable as input to `kwal +s@FILE`. Words are
/// alphabetized and deduped across the result.
///
/// CLAN manual §7.10.15 (+d1):
/// > "Outputs each of the words found in the input data file(s)
/// > one word per line with no further information about
/// > frequency. Later this output could be used as a word list
/// > file for kwal or combo programs."
#[test]
fn freq_word_list_only_strips_everything_but_words() {
    let result = FreqResult {
        file_speaker_rows: Vec::new(),
        mattr_enabled: false,
        combine_speakers: false,
        mor_based: false,
        word_list_only: true,
        types_tokens_only: false,
        sort: FreqSort::Alphabetical,
        speakers: vec![FreqSpeakerResult {
            speaker: "CHI".to_owned(),
            entries: vec![
                FreqEntry {
                    word: "want".to_owned(),
                    display_form: None,
                    count: 2,
                },
                FreqEntry {
                    word: "cookie".to_owned(),
                    display_form: None,
                    count: 1,
                },
            ],
            total_types: 2,
            total_tokens: 3,
            ttr: 0.667,
            mattr: None,
        }],
    };
    let clan = result.render_clan();
    let lines: Vec<&str> = clan.lines().filter(|l| !l.is_empty()).collect();
    // CLAN `+d1` combines speakers, so it prefixes the list with the
    // `;%* Combined Speakers output:` header (freq.cpp:1468-1471), then
    // one alphabetized word per line, nothing else.
    assert_eq!(
        lines,
        vec![";%* Combined Speakers output:", "cookie", "want"]
    );
    // No counts, banners, separators, or TTR matter for downstream
    // `kwal +s@FILE` consumption.
    assert!(
        !clan.contains("Speaker:"),
        "word-list-only must not emit Speaker banners"
    );
    assert!(
        !clan.contains("Total"),
        "word-list-only must not emit totals"
    );
    assert!(
        !clan.contains("Type/Token"),
        "word-list-only must not emit TTR"
    );
    assert!(
        !clan.contains("---"),
        "word-list-only must not emit separators"
    );
}

/// FREQ `+d4` / `--types-tokens-only`: emit only the
/// per-speaker type/token/TTR summary, dropping all per-word
/// frequency entries. The CLAN-format banner shape (Speaker
/// header + separator + totals + TTR note) is preserved.
///
/// CLAN manual §7.10.15 (+d4): "Allows you to output just the
/// type-token information."
#[test]
fn freq_types_tokens_only_drops_per_word_entries() {
    let result = FreqResult {
        file_speaker_rows: Vec::new(),
        mattr_enabled: false,
        combine_speakers: false,
        mor_based: false,
        word_list_only: false,
        types_tokens_only: true,
        sort: FreqSort::Alphabetical,
        speakers: vec![FreqSpeakerResult {
            speaker: "CHI".to_owned(),
            entries: vec![
                FreqEntry {
                    word: "want".to_owned(),
                    display_form: None,
                    count: 2,
                },
                FreqEntry {
                    word: "cookie".to_owned(),
                    display_form: None,
                    count: 1,
                },
            ],
            total_types: 2,
            total_tokens: 3,
            ttr: 0.667,
            mattr: None,
        }],
    };
    let clan = result.render_clan();
    // Summary lines, speaker banner, separator, TTR all kept.
    assert!(clan.contains("Speaker: *CHI:"));
    assert!(clan.contains("Total number of different item types used"));
    assert!(clan.contains("Total number of items (tokens)"));
    assert!(clan.contains("Type/Token ratio"));
    assert!(clan.contains("------------------------------"));
    // Per-word entry lines (shape: ` <count> <word>`) are dropped.
    // Note: "want" also appears in the static TTR-note boilerplate
    // ("If you want a TTR based on lemmas"), so we cannot bare-
    // substring-check for the word, match the entry-line shape.
    assert!(
        !clan.contains("  2 want\n"),
        "types-tokens-only must not emit `<count> <word>` entry lines: {clan:?}"
    );
    assert!(
        !clan.contains("  1 cookie\n"),
        "types-tokens-only must not emit `<count> <word>` entry lines: {clan:?}"
    );
}

/// CSV companion to `+d4`: `+d3` is the same content in
/// spreadsheet form. `render_csv` must honor `types_tokens_only`
/// the same way `render_clan` does, keep the `Speaker,X` /
/// `Total types,N` / `Total tokens,N` / `TTR,X` rows but drop
/// the per-word `Count,Word` header and `<count>,<word>` rows.
#[test]
fn freq_types_tokens_only_csv_drops_per_word_rows() {
    let result = FreqResult {
        file_speaker_rows: Vec::new(),
        mattr_enabled: false,
        combine_speakers: false,
        mor_based: false,
        word_list_only: false,
        types_tokens_only: true,
        sort: FreqSort::Alphabetical,
        speakers: vec![FreqSpeakerResult {
            speaker: "CHI".to_owned(),
            entries: vec![
                FreqEntry {
                    word: "want".to_owned(),
                    display_form: None,
                    count: 2,
                },
                FreqEntry {
                    word: "cookie".to_owned(),
                    display_form: None,
                    count: 1,
                },
            ],
            total_types: 2,
            total_tokens: 3,
            ttr: 0.667,
            mattr: None,
        }],
    };
    let csv = result.render_csv();
    // Speaker row + summary rows kept.
    assert!(csv.contains("Speaker,CHI"));
    assert!(csv.contains("Total types,2"));
    assert!(csv.contains("Total tokens,3"));
    assert!(csv.contains("TTR,0.667"));
    // Per-word header and rows dropped.
    assert!(
        !csv.contains("Count,Word"),
        "types-tokens-only CSV must not emit Count,Word header: {csv:?}"
    );
    assert!(
        !csv.contains("2,want"),
        "types-tokens-only CSV must not emit per-word rows: {csv:?}"
    );
    assert!(
        !csv.contains("1,cookie"),
        "types-tokens-only CSV must not emit per-word rows: {csv:?}"
    );
}

/// CLAN FREQ `+k` FOLDS case to lowercase: it toggles `nomap` off
/// (cutt.cpp:13816), so `Want`/`want`/`WANT` collapse to a single entry
/// with count 3. chatter represents the `+k`/fold state as
/// `case_sensitive: false` (the keying preserve-state is off).
#[test]
fn freq_plus_k_folds_case_variants() {
    use talkbank_model::ChatFile;
    let command = FreqCommand::new(FreqConfig {
        case_sensitive: false,
        ..FreqConfig::default()
    });
    let mut state = FreqState::default();
    let chat_file = ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    // Three case variants of "want" in a single utterance.
    let u = make_utterance("CHI", &["Want", "want", "WANT"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = result
        .speakers
        .iter()
        .find(|s| s.speaker == "CHI")
        .expect("CHI speaker should be present");
    assert_eq!(chi.total_tokens, 3, "all three tokens are counted");
    assert_eq!(
        chi.total_types, 1,
        "+k folds case, so the three variants collapse to 1 type"
    );
}

/// Companion to `freq_plus_k_folds_case_variants`: CLAN FREQ's DEFAULT
/// preserves case (nomap=TRUE, cutt.cpp:7845), so without `+k` the three
/// case variants stay distinct. `FreqConfig::default()` is preserve.
#[test]
fn freq_default_preserves_case_variants() {
    use talkbank_model::ChatFile;
    let command = FreqCommand::new(FreqConfig::default());
    let mut state = FreqState::default();
    let chat_file = ChatFile::new(vec![]);
    let file_ctx = FileContext {
        path: std::path::Path::new("test.cha"),
        chat_file: &chat_file,
        filename: "test",
        line_map: None,
    };

    let u = make_utterance("CHI", &["Want", "want", "WANT"]);
    command.process_utterance(&u, &file_ctx, &mut state);

    let result = command.finalize(state);
    let chi = result
        .speakers
        .iter()
        .find(|s| s.speaker == "CHI")
        .expect("CHI speaker should be present");
    assert_eq!(chi.total_tokens, 3);
    assert_eq!(
        chi.total_types, 3,
        "default preserves case, so the three variants stay distinct"
    );
}

/// Multi-speaker case for `+d1`: words from all speakers
/// merge into one alphabetized deduped list.
#[test]
fn freq_word_list_only_dedupes_across_speakers() {
    let result = FreqResult {
        file_speaker_rows: Vec::new(),
        mattr_enabled: false,
        combine_speakers: false,
        mor_based: false,
        word_list_only: true,
        types_tokens_only: false,
        sort: FreqSort::Alphabetical,
        speakers: vec![
            FreqSpeakerResult {
                speaker: "CHI".to_owned(),
                entries: vec![
                    FreqEntry {
                        word: "want".to_owned(),
                        display_form: None,
                        count: 2,
                    },
                    FreqEntry {
                        word: "cookie".to_owned(),
                        display_form: None,
                        count: 1,
                    },
                ],
                total_types: 2,
                total_tokens: 3,
                ttr: 0.667,
                mattr: None,
            },
            FreqSpeakerResult {
                speaker: "MOT".to_owned(),
                entries: vec![
                    FreqEntry {
                        word: "want".to_owned(),
                        display_form: None,
                        count: 1,
                    },
                    FreqEntry {
                        word: "apple".to_owned(),
                        display_form: None,
                        count: 1,
                    },
                ],
                total_types: 2,
                total_tokens: 2,
                ttr: 1.0,
                mattr: None,
            },
        ],
    };
    let clan = result.render_clan();
    let lines: Vec<&str> = clan.lines().filter(|l| !l.is_empty()).collect();
    // Combined across speakers, deduped, alphabetized, under CLAN's
    // `;%* Combined Speakers output:` header (freq.cpp:1468-1471).
    assert_eq!(
        lines,
        vec![";%* Combined Speakers output:", "apple", "cookie", "want"]
    );
}
