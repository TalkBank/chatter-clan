use serde::Serialize;

use crate::framework::{AnalysisResult, CommandOutput, OutputFormat, Section, TableRow};

/// Per-code chain statistics.
#[derive(Debug, Clone, Serialize)]
pub struct CodeChainStats {
    /// Code token.
    pub code: String,
    /// Number of separate chains.
    pub num_chains: u64,
    /// Average chain length.
    pub avg_length: f64,
    /// Standard deviation of chain lengths.
    pub std_dev: f64,
    /// Minimum chain length observed.
    pub min_length: u64,
    /// Maximum chain length observed.
    pub max_length: u64,
}

/// Per-speaker chain data.
#[derive(Debug, Clone, Serialize)]
pub struct SpeakerChains {
    /// Speaker identifier.
    pub speaker: String,
    /// Per-code chain statistics.
    pub codes: Vec<CodeChainStats>,
}

/// Typed output for the CHAINS command.
#[derive(Debug, Clone, Serialize)]
pub struct ChainsResult {
    /// Per-speaker chain data.
    pub speakers: Vec<SpeakerChains>,
}

impl ChainsResult {
    pub(crate) fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("chains");
        for speaker in &self.speakers {
            let rows: Vec<TableRow> = speaker
                .codes
                .iter()
                .map(|c| TableRow {
                    values: vec![
                        c.code.clone(),
                        c.num_chains.to_string(),
                        format!("{:.2}", c.avg_length),
                        format!("{:.2}", c.std_dev),
                        c.min_length.to_string(),
                        c.max_length.to_string(),
                    ],
                })
                .collect();
            let section = Section::with_table(
                format!("Speaker: {}", speaker.speaker),
                vec![
                    "Code".to_owned(),
                    "# Chains".to_owned(),
                    "Avg Length".to_owned(),
                    "Std Dev".to_owned(),
                    "Min".to_owned(),
                    "Max".to_owned(),
                ],
                rows,
            );
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for ChainsResult {
    /// Render chain statistics as a human-readable text table.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// Render chain statistics in CLAN-compatible format.
    fn render_clan(&self) -> String {
        let mut out = String::new();
        for speaker in &self.speakers {
            // CLAN skips speakers with no chains
            if speaker.codes.is_empty() {
                continue;
            }
            out.push_str(&format!("Speaker: {}\n", speaker.speaker));
            for c in &speaker.codes {
                out.push_str(&format!(
                    "  {:>10}  chains:{:>3}  avg:{:.2}  sd:{:.2}  min:{}  max:{}\n",
                    c.code, c.num_chains, c.avg_length, c.std_dev, c.min_length, c.max_length
                ));
            }
            out.push('\n');
        }
        out
    }
}
