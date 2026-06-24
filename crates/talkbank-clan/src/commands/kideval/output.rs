use serde::Serialize;

use crate::framework::{
    AnalysisResult, AnalysisScore, CommandOutput, OutputFormat, POSCount, ScorePoints, Section,
    TableRow, TypeCount, UtteranceCount, WordCount,
};

/// Per-speaker combined evaluation metrics.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SpeakerKideval {
    /// Speaker identifier.
    pub speaker: String,
    /// Number of utterances analyzed.
    pub utterances: UtteranceCount,
    /// Total words (tokens).
    pub total_words: WordCount,
    /// Number of different words (NDW).
    pub ndw: TypeCount,
    /// Type-token ratio (TTR).
    pub ttr: AnalysisScore,
    /// MLU in words.
    pub mlu_words: AnalysisScore,
    /// MLU in morphemes.
    pub mlu_morphemes: AnalysisScore,

    // Morphological category counts
    /// Nouns.
    pub nouns: POSCount,
    /// Verbs.
    pub verbs: POSCount,
    /// Auxiliaries.
    pub auxiliaries: POSCount,
    /// Modals.
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

    // Combined scores
    /// DSS score (from developmental sentence scoring).
    pub dss_score: AnalysisScore,
    /// VOCD score (from vocabulary diversity, uses existing vocd command).
    pub vocd_score: AnalysisScore,
    /// IPSYN score (from productive syntax).
    pub ipsyn_score: ScorePoints,

    /// Word-level errors.
    pub word_errors: POSCount,
}

/// Typed output for the KIDEVAL command.
#[derive(Debug, Clone, Serialize)]
pub struct KidevalResult {
    /// Per-speaker combined results.
    pub speakers: Vec<SpeakerKideval>,
    /// Per-speaker normative comparisons (parallel to `speakers`).
    /// Present only when a database was provided in config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparisons: Option<Vec<Vec<crate::commands::kideval_columns::KidevalMeasureComparison>>>,
}

impl KidevalResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("kideval");
        for (i, sp) in self.speakers.iter().enumerate() {
            let has_comparison = self
                .comparisons
                .as_ref()
                .is_some_and(|c| !c.get(i).is_some_and(|v| v.is_empty()));

            let (columns, rows) = if has_comparison {
                // Control-flow invariant: `has_comparison` is true
                // only when `self.comparisons.as_ref().is_some_and(...)`
                // holds, which proves `self.comparisons` is `Some`.
                #[allow(clippy::unwrap_used)]
                let comps = &self.comparisons.as_ref().unwrap()[i];
                let cols = vec![
                    "Metric".to_owned(),
                    "Score".to_owned(),
                    "DB Mean".to_owned(),
                    "DB SD".to_owned(),
                    "Z-Score".to_owned(),
                    "N".to_owned(),
                ];
                let rows: Vec<TableRow> = comps
                    .iter()
                    .map(|c| TableRow {
                        values: vec![
                            c.label.to_owned(),
                            format!("{:.2}", c.score),
                            format!("{:.2}", c.db_mean),
                            format!("{:.2}", c.db_sd),
                            c.z_score
                                .map(|z| format!("{z:+.2}"))
                                .unwrap_or_else(|| "N/A".to_owned()),
                            c.db_n.to_string(),
                        ],
                    })
                    .collect();
                (cols, rows)
            } else {
                let cols = vec!["Metric".to_owned(), "Value".to_owned()];
                let rows = vec![
                    TableRow {
                        values: vec!["Utterances".to_owned(), sp.utterances.to_string()],
                    },
                    TableRow {
                        values: vec!["Total words".to_owned(), sp.total_words.to_string()],
                    },
                    TableRow {
                        values: vec!["NDW".to_owned(), sp.ndw.to_string()],
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
                        values: vec!["DSS score".to_owned(), format!("{:.2}", sp.dss_score)],
                    },
                    TableRow {
                        values: vec!["IPSYN score".to_owned(), sp.ipsyn_score.to_string()],
                    },
                    TableRow {
                        values: vec!["Nouns".to_owned(), sp.nouns.to_string()],
                    },
                    TableRow {
                        values: vec!["Verbs".to_owned(), sp.verbs.to_string()],
                    },
                    TableRow {
                        values: vec!["Pronouns".to_owned(), sp.pronouns.to_string()],
                    },
                    TableRow {
                        values: vec!["Word errors".to_owned(), sp.word_errors.to_string()],
                    },
                ];
                (cols, rows)
            };

            let section = Section::with_table(format!("Speaker: {}", sp.speaker), columns, rows);
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for KidevalResult {
    /// Render per-speaker metric/value table.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible fixed-width summary per speaker.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for sp in &self.speakers {
            out.push_str(&format!("Speaker: {}\n", sp.speaker));
            out.push_str(&format!("  Utterances:      {}\n", sp.utterances));
            out.push_str(&format!("  Total words:     {}\n", sp.total_words));
            out.push_str(&format!("  NDW:             {}\n", sp.ndw));
            out.push_str(&format!("  TTR:             {:.3}\n", sp.ttr));
            out.push_str(&format!("  MLU (words):     {:.2}\n", sp.mlu_words));
            out.push_str(&format!("  MLU (morphemes): {:.2}\n", sp.mlu_morphemes));
            out.push_str(&format!("  DSS:             {:.2}\n", sp.dss_score));
            out.push_str(&format!("  IPSYN:           {}\n", sp.ipsyn_score));
            out.push_str(&format!("  Nouns:           {}\n", sp.nouns));
            out.push_str(&format!("  Verbs:           {}\n", sp.verbs));
            out.push_str(&format!("  Pronouns:        {}\n", sp.pronouns));
            out.push_str(&format!("  Word errors:     {}\n\n", sp.word_errors));
        }
        out
    }
}
