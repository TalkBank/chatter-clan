use serde::Serialize;

use crate::framework::{
    AnalysisResult, AnalysisScore, CommandOutput, MorphemeCount, OutputFormat, POSCount, Section,
    TableRow, TypeCount, UtteranceCount, WordCount,
};

/// Per-speaker evaluation metrics.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SpeakerEval {
    /// Speaker identifier.
    pub speaker: String,
    /// Number of utterances.
    pub utterances: UtteranceCount,
    /// Total words (tokens).
    pub total_words: WordCount,
    /// Number of different words (types).
    pub ndw: TypeCount,
    /// Type-token ratio.
    pub ttr: AnalysisScore,

    // Morphological category counts
    /// Nouns.
    pub nouns: POSCount,
    /// Verbs (all types).
    pub verbs: POSCount,
    /// Auxiliary verbs.
    pub auxiliaries: POSCount,
    /// Modal verbs.
    pub modals: POSCount,
    /// Prepositions.
    pub prepositions: POSCount,
    /// Adjectives.
    pub adjectives: POSCount,
    /// Adverbs.
    pub adverbs: POSCount,
    /// Conjunctions.
    pub conjunctions: POSCount,
    /// Determiners.
    pub determiners: POSCount,
    /// Pronouns.
    pub pronouns: POSCount,
    /// Plurals.
    pub plurals: POSCount,
    /// Past tense.
    pub past_tense: POSCount,
    /// Present participle (-ing).
    pub present_participle: POSCount,
    /// Past participle.
    pub past_participle: POSCount,

    // Error counts
    /// Word-level errors `[*]`.
    pub word_errors: POSCount,
    /// Utterance-level errors (from postcodes).
    pub utterance_errors: UtteranceCount,

    // Derived metrics
    /// Mean length of utterance (words).
    pub mlu_words: AnalysisScore,
    /// Mean length of utterance (morphemes).
    pub mlu_morphemes: AnalysisScore,
    /// Total morphemes.
    pub total_morphemes: MorphemeCount,
    /// Open-closed ratio (content words / function words).
    pub open_closed_ratio: AnalysisScore,
}

/// Typed output for the EVAL command.
#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    /// Per-speaker evaluation data.
    pub speakers: Vec<SpeakerEval>,
    /// Per-speaker normative comparisons (parallel to `speakers`).
    /// Present only when a database was provided in config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparisons: Option<Vec<Vec<super::super::eval_columns::EvalMeasureComparison>>>,
}

impl EvalResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("eval");
        for (idx, sp) in self.speakers.iter().enumerate() {
            let comparison = self.comparisons.as_ref().and_then(|c| c.get(idx));

            if let Some(cmp) = comparison {
                let headers = vec![
                    "Measure".to_owned(),
                    "Score".to_owned(),
                    "DB Mean".to_owned(),
                    "DB SD".to_owned(),
                    "Z-Score".to_owned(),
                    "N".to_owned(),
                ];
                let rows = cmp
                    .iter()
                    .map(|m| TableRow {
                        values: vec![
                            m.label.to_owned(),
                            format!("{:.2}", m.score),
                            format!("{:.2}", m.db_mean),
                            format!("{:.2}", m.db_sd),
                            m.z_score
                                .map(|z| format!("{z:+.2}"))
                                .unwrap_or_else(|| "-".to_owned()),
                            m.db_n.to_string(),
                        ],
                    })
                    .collect();
                let section =
                    Section::with_table(format!("Speaker: {}", sp.speaker), headers, rows);
                result.add_section(section);
            } else {
                let rows = vec![
                    TableRow {
                        values: vec!["Utterances".to_owned(), sp.utterances.to_string()],
                    },
                    TableRow {
                        values: vec!["Total words".to_owned(), sp.total_words.to_string()],
                    },
                    TableRow {
                        values: vec!["NDW (types)".to_owned(), sp.ndw.to_string()],
                    },
                    TableRow {
                        values: vec!["TTR".to_owned(), format!("{:.3}", sp.ttr)],
                    },
                    TableRow {
                        values: vec!["MLU (words)".to_owned(), format!("{:.2}", sp.mlu_words)],
                    },
                    TableRow {
                        values: vec![
                            "MLU (morphemes)".to_owned(),
                            format!("{:.2}", sp.mlu_morphemes),
                        ],
                    },
                    TableRow {
                        values: vec!["Nouns".to_owned(), sp.nouns.to_string()],
                    },
                    TableRow {
                        values: vec!["Verbs".to_owned(), sp.verbs.to_string()],
                    },
                    TableRow {
                        values: vec!["Auxiliaries".to_owned(), sp.auxiliaries.to_string()],
                    },
                    TableRow {
                        values: vec!["Modals".to_owned(), sp.modals.to_string()],
                    },
                    TableRow {
                        values: vec!["Prepositions".to_owned(), sp.prepositions.to_string()],
                    },
                    TableRow {
                        values: vec!["Adjectives".to_owned(), sp.adjectives.to_string()],
                    },
                    TableRow {
                        values: vec!["Adverbs".to_owned(), sp.adverbs.to_string()],
                    },
                    TableRow {
                        values: vec!["Conjunctions".to_owned(), sp.conjunctions.to_string()],
                    },
                    TableRow {
                        values: vec!["Determiners".to_owned(), sp.determiners.to_string()],
                    },
                    TableRow {
                        values: vec!["Pronouns".to_owned(), sp.pronouns.to_string()],
                    },
                    TableRow {
                        values: vec!["Word errors".to_owned(), sp.word_errors.to_string()],
                    },
                ];
                let section = Section::with_table(
                    format!("Speaker: {}", sp.speaker),
                    vec!["Metric".to_owned(), "Value".to_owned()],
                    rows,
                );
                result.add_section(section);
            }
        }
        result
    }
}

impl CommandOutput for EvalResult {
    /// Render evaluation metrics as a human-readable text table.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render evaluation metrics in CLAN-compatible format.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for sp in &self.speakers {
            out.push_str(&format!("Speaker: {}\n", sp.speaker));
            out.push_str(&format!("  Utterances:       {}\n", sp.utterances));
            out.push_str(&format!("  Total words:      {}\n", sp.total_words));
            out.push_str(&format!("  NDW:              {}\n", sp.ndw));
            out.push_str(&format!("  TTR:              {:.3}\n", sp.ttr));
            out.push_str(&format!("  MLU (words):      {:.2}\n", sp.mlu_words));
            out.push_str(&format!("  MLU (morphemes):  {:.2}\n", sp.mlu_morphemes));
            out.push_str(&format!("  Nouns:            {}\n", sp.nouns));
            out.push_str(&format!("  Verbs:            {}\n", sp.verbs));
            out.push_str(&format!("  Auxiliaries:      {}\n", sp.auxiliaries));
            out.push_str(&format!("  Modals:           {}\n", sp.modals));
            out.push_str(&format!("  Prepositions:     {}\n", sp.prepositions));
            out.push_str(&format!("  Adjectives:       {}\n", sp.adjectives));
            out.push_str(&format!("  Adverbs:          {}\n", sp.adverbs));
            out.push_str(&format!("  Conjunctions:     {}\n", sp.conjunctions));
            out.push_str(&format!("  Determiners:      {}\n", sp.determiners));
            out.push_str(&format!("  Pronouns:         {}\n", sp.pronouns));
            out.push_str(&format!("  Word errors:      {}\n\n", sp.word_errors));
        }
        out
    }
}
