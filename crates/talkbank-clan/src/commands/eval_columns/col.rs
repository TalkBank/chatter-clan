/// Named column indices in the Eval `.cut` database.
///
/// These match the `retrieveTS()` read order in CLAN's `eval.cpp`.
pub const TIME: usize = 0;
/// NDW (number of different words / speaker word count).
pub const NDW: usize = 1;
/// Frequency tokens (total words).
pub const FREQ_TOKENS: usize = 2;
// 3: CUR (content units ratio)
// 4: nounsNV (nouns for N/V ratio)
// 5: verbsNV (verbs for N/V ratio)
/// MLU words (sum, not average, divide by mluUtt for MLU).
pub const MLU_WORDS_SUM: usize = 6;
/// MLU morphemes (sum, not average).
pub const MLU_MORF_SUM: usize = 7;
/// Total utterances.
pub const TOTAL_UTTS: usize = 8;
/// MLU utterance count.
pub const MLU_UTTS: usize = 9;
/// Word errors.
pub const WORD_ERRORS: usize = 10;
/// Utterance errors.
pub const UTT_ERRORS: usize = 11;
/// Total morphemes on %mor tier.
pub const MOR_TOTAL: usize = 12;
// 13: density (lexical density)
/// Nouns.
pub const NOUNS: usize = 14;
/// Verbs.
pub const VERBS: usize = 15;
/// Auxiliaries.
pub const AUX: usize = 16;
/// Modals.
pub const MODALS: usize = 17;
/// Prepositions.
pub const PREP: usize = 18;
/// Adjectives.
pub const ADJ: usize = 19;
/// Adverbs.
pub const ADV: usize = 20;
/// Conjunctions.
pub const CONJ: usize = 21;
/// Pronouns.
pub const PRON: usize = 22;
/// Determiners.
pub const DET: usize = 23;
// 24: thrS (3rd person -s)
// 25: thrnS (3rd person non-s)
/// Past tense.
pub const PAST: usize = 26;
/// Past participle.
pub const PAST_PARTICIPLE: usize = 27;
/// Plurals.
pub const PLURALS: usize = 28;
/// Present participle.
pub const PRESENT_PARTICIPLE: usize = 29;
/// Open class words count.
pub const OPEN_CLASS: usize = 30;
/// Closed class words count.
pub const CLOSED_CLASS: usize = 31;
// 32: retracings
// 33: repetitions
