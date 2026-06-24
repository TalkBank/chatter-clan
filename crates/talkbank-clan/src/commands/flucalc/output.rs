//! Typed FLUCALC results and rendering logic.

use serde::Serialize;

use crate::framework::{
    AnalysisResult, CommandOutput, OutputFormat, Section, TableRow, UtteranceCount, WordCount,
};

/// Per-speaker fluency metrics.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SpeakerFluency {
    /// Speaker identifier.
    pub speaker: String,
    /// Total utterances analyzed.
    pub utterances: UtteranceCount,
    /// Total words produced.
    pub total_words: WordCount,

    // Stuttering-Like Disfluencies (SLD)
    /// Prolongations (`:` in word).
    pub prolongations: u64,
    /// Broken words (`^` notation).
    pub broken_words: u64,
    /// Blocks (`≠` notation).
    pub blocks: u64,
    /// Part-word repetitions (PWR).
    pub part_word_reps: u64,
    /// Whole-word repetitions (WWR).
    pub whole_word_reps: u64,

    // Typical Disfluencies (TD)
    /// Phrase repetitions `[/]`.
    pub phrase_reps: u64,
    /// Word/phrase revisions `[//]`.
    pub revisions: u64,
    /// Filled pauses (`&-uh`, `&-um`, etc.).
    pub filled_pauses: u64,
    /// Phonological fragments (`&+`).
    pub phon_fragments: u64,
}

impl SpeakerFluency {
    /// Total stuttering-like disfluencies.
    pub fn total_sld(&self) -> u64 {
        self.prolongations
            + self.broken_words
            + self.blocks
            + self.part_word_reps
            + self.whole_word_reps
    }

    /// Total typical disfluencies.
    pub fn total_td(&self) -> u64 {
        self.phrase_reps + self.revisions + self.filled_pauses + self.phon_fragments
    }

    /// Total disfluencies.
    pub fn total_disfluencies(&self) -> u64 {
        self.total_sld() + self.total_td()
    }

    /// SLD percentage (per 100 words).
    pub fn sld_pct(&self) -> f64 {
        if self.total_words > 0 {
            self.total_sld() as f64 / self.total_words as f64 * 100.0
        } else {
            0.0
        }
    }

    /// TD percentage (per 100 words).
    pub fn td_pct(&self) -> f64 {
        if self.total_words > 0 {
            self.total_td() as f64 / self.total_words as f64 * 100.0
        } else {
            0.0
        }
    }
}

/// Typed output for the FLUCALC command.
#[derive(Debug, Clone, Serialize)]
pub struct FlucalcResult {
    /// Per-speaker fluency data.
    pub speakers: Vec<SpeakerFluency>,
}

impl FlucalcResult {
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("flucalc");
        for sp in &self.speakers {
            let rows = vec![
                TableRow {
                    values: vec!["Prolongations".to_owned(), sp.prolongations.to_string()],
                },
                TableRow {
                    values: vec!["Broken words".to_owned(), sp.broken_words.to_string()],
                },
                TableRow {
                    values: vec!["Blocks".to_owned(), sp.blocks.to_string()],
                },
                TableRow {
                    values: vec!["Part-word reps".to_owned(), sp.part_word_reps.to_string()],
                },
                TableRow {
                    values: vec!["Whole-word reps".to_owned(), sp.whole_word_reps.to_string()],
                },
                TableRow {
                    values: vec!["Total SLD".to_owned(), sp.total_sld().to_string()],
                },
                TableRow {
                    values: vec!["Phrase reps".to_owned(), sp.phrase_reps.to_string()],
                },
                TableRow {
                    values: vec!["Revisions".to_owned(), sp.revisions.to_string()],
                },
                TableRow {
                    values: vec!["Filled pauses".to_owned(), sp.filled_pauses.to_string()],
                },
                TableRow {
                    values: vec!["Phon fragments".to_owned(), sp.phon_fragments.to_string()],
                },
                TableRow {
                    values: vec!["Total TD".to_owned(), sp.total_td().to_string()],
                },
                TableRow {
                    values: vec!["Total words".to_owned(), sp.total_words.to_string()],
                },
                TableRow {
                    values: vec!["SLD %".to_owned(), format!("{:.1}%", sp.sld_pct())],
                },
                TableRow {
                    values: vec!["TD %".to_owned(), format!("{:.1}%", sp.td_pct())],
                },
            ];
            let section = Section::with_table(
                format!("Speaker: {}", sp.speaker),
                vec!["Metric".to_owned(), "Value".to_owned()],
                rows,
            );
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for FlucalcResult {
    /// Render fluency metrics as a human-readable text table.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render fluency metrics in CLAN-compatible format.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for sp in &self.speakers {
            out.push_str(&format!("Speaker: {}\n", sp.speaker));
            out.push_str(&format!("  Utterances:      {}\n", sp.utterances));
            out.push_str(&format!("  Total words:     {}\n", sp.total_words));
            out.push_str("  --- SLD ---\n");
            out.push_str(&format!("  Prolongations:   {}\n", sp.prolongations));
            out.push_str(&format!("  Broken words:    {}\n", sp.broken_words));
            out.push_str(&format!("  Blocks:          {}\n", sp.blocks));
            out.push_str(&format!("  Part-word reps:  {}\n", sp.part_word_reps));
            out.push_str(&format!("  Whole-word reps: {}\n", sp.whole_word_reps));
            out.push_str(&format!(
                "  SLD total:       {} ({:.1}%)\n",
                sp.total_sld(),
                sp.sld_pct()
            ));
            out.push_str("  --- TD ---\n");
            out.push_str(&format!("  Phrase reps:     {}\n", sp.phrase_reps));
            out.push_str(&format!("  Revisions:       {}\n", sp.revisions));
            out.push_str(&format!("  Filled pauses:   {}\n", sp.filled_pauses));
            out.push_str(&format!("  Phon fragments:  {}\n", sp.phon_fragments));
            out.push_str(&format!(
                "  TD total:        {} ({:.1}%)\n",
                sp.total_td(),
                sp.td_pct()
            ));
            out.push('\n');
        }
        out
    }
}
