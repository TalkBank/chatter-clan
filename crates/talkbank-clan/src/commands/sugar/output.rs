use serde::Serialize;

use crate::framework::{
    AnalysisResult, CommandOutput, MorphemeCount, OutputFormat, Section, UtteranceCount, WordCount,
};

/// Per-speaker SUGAR metrics.
#[derive(Debug, Clone, Serialize)]
pub struct SpeakerSugar {
    /// Speaker identifier.
    pub speaker: String,
    /// Mean Length of Utterance in morphemes.
    pub mlu_s: Option<f64>,
    /// Total Number of Words.
    pub tnw: WordCount,
    /// Words Per clause (utterances with verbs).
    pub wps: Option<f64>,
    /// Clauses Per utterance with verbs.
    pub cps: Option<f64>,
    /// Total utterances counted.
    pub utterance_count: UtteranceCount,
    /// Total morphemes counted.
    pub morpheme_count: MorphemeCount,
}

/// Typed output for the SUGAR command.
#[derive(Debug, Clone, Serialize)]
pub struct SugarResult {
    /// Per-speaker metrics.
    pub speakers: Vec<SpeakerSugar>,
}

impl SugarResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("sugar");
        for speaker in &self.speakers {
            let mut section = Section::with_fields(
                format!("Speaker: {}", speaker.speaker),
                indexmap::IndexMap::new(),
            );
            section.fields.insert(
                "MLU-S".to_owned(),
                speaker
                    .mlu_s
                    .map_or("N/A".to_owned(), |v| format!("{v:.3}")),
            );
            section
                .fields
                .insert("TNW".to_owned(), speaker.tnw.to_string());
            section.fields.insert(
                "WPS".to_owned(),
                speaker.wps.map_or("N/A".to_owned(), |v| format!("{v:.3}")),
            );
            section.fields.insert(
                "CPS".to_owned(),
                speaker.cps.map_or("N/A".to_owned(), |v| format!("{v:.3}")),
            );
            section
                .fields
                .insert("Utterances".to_owned(), speaker.utterance_count.to_string());
            section
                .fields
                .insert("Morphemes".to_owned(), speaker.morpheme_count.to_string());
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for SugarResult {
    /// Render per-speaker SUGAR metrics (MLU-S, TNW, WPS, CPS).
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render CLAN-compatible per-speaker summary.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for speaker in &self.speakers {
            out.push_str(&format!("Speaker: {}\n", speaker.speaker));
            out.push_str(&format!(
                "  MLU-S: {}\n",
                speaker
                    .mlu_s
                    .map_or("N/A".to_owned(), |v| format!("{v:.3}"))
            ));
            out.push_str(&format!("  TNW: {}\n", speaker.tnw));
            out.push_str(&format!(
                "  WPS: {}\n",
                speaker.wps.map_or("N/A".to_owned(), |v| format!("{v:.3}"))
            ));
            out.push_str(&format!(
                "  CPS: {}\n",
                speaker.cps.map_or("N/A".to_owned(), |v| format!("{v:.3}"))
            ));
            out.push('\n');
        }
        out
    }
}
