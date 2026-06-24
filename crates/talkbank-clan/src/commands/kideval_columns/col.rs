/// Named column indices in the KidEval `.cut` database.
///
/// These match the `0all_norms_with_columns.csv` header (score columns only,
/// 0-indexed from the start of the numeric data line).
/// Total utterances (mWords in CLAN source).
pub const TOTAL_UTTS: usize = 0;
/// MLU utterances (morf in CLAN source).
pub const MLU_UTTS: usize = 1;
/// MLU in words.
pub const MLU_WORDS: usize = 2;
/// MLU in morphemes.
pub const MLU_MORPHEMES: usize = 3;
// 4-6: MLU50 variants (not computed by our command)
/// Frequency types (unique words).
pub const FREQ_TYPES: usize = 7;
/// Frequency tokens (total words).
pub const FREQ_TOKENS: usize = 8;
/// Number of different words (NDW, 100-word sample).
pub const NDW: usize = 9;
// 10: NDW total
/// VOCD-D optimum average.
pub const VOCD: usize = 11;
/// Verbs per utterance ratio.
pub const VERBS_UTT: usize = 12;
/// Word errors count.
pub const WORD_ERRORS: usize = 13;
// 14: Utterance errors
// 15-16: retracing, repetition
/// DSS utterance count.
pub const DSS_UTTS: usize = 17;
/// DSS score.
pub const DSS: usize = 18;
/// IPSyn utterance count.
pub const IPSYN_UTTS: usize = 19;
/// IPSyn total score.
pub const IPSYN_TOTAL: usize = 20;
/// Total morphemes on %mor tier.
pub const MOR_WORDS: usize = 21;
