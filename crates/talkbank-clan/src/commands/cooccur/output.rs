//! Typed COOCCUR results and rendering logic.

use serde::Serialize;

use crate::framework::{
    AnalysisResult, CommandOutput, OutputFormat, Section, TableRow, UtteranceCount,
};

/// A co-occurring adjacent word cluster (N-gram) with its frequency count.
/// `words.len() == displays.len() == cluster_size`.
#[derive(Debug, Clone, Serialize)]
pub struct CooccurCluster {
    /// Words in the cluster, in utterance order. Lowercased/normalized.
    pub words: Vec<String>,
    /// CLAN display forms (preserve `+` in compounds), one per word.
    pub displays: Vec<String>,
    /// Number of times this cluster occurs.
    pub count: u64,
}

/// Typed output for the COOCCUR command.
#[derive(Debug, Clone, Serialize)]
pub struct CooccurResult {
    /// Word clusters sorted by co-occurrence count descending.
    pub clusters: Vec<CooccurCluster>,
    /// Number of unique clusters observed.
    pub unique_clusters: usize,
    /// Sum of all cluster counts.
    pub total_cluster_instances: u64,
    /// Total utterances examined.
    pub total_utterances: UtteranceCount,
    /// CLAN `+d`: whether the CLAN-format renderer should omit
    /// the leading frequency-count column.
    pub no_frequency_counts: bool,
}

impl CooccurResult {
    /// Convert typed co-occurrence data into the shared section/table render model.
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("cooccur");
        if self.clusters.is_empty() {
            return result;
        }

        // Header columns: "Word 1", "Word 2", ..., "Word N", "Count".
        // Length comes from the first cluster (all clusters in a run
        // share the same `cluster_size` from `CooccurConfig`).
        let n = self.clusters[0].words.len();
        let mut headers: Vec<String> = (1..=n).map(|i| format!("Word {i}")).collect();
        headers.push("Count".to_owned());

        let rows: Vec<TableRow> = self
            .clusters
            .iter()
            .map(|c| {
                let mut values = c.words.clone();
                values.push(c.count.to_string());
                TableRow { values }
            })
            .collect();

        let mut section = Section::with_table("Co-occurrences".to_owned(), headers, rows);
        section.fields.insert(
            "Unique clusters".to_owned(),
            self.unique_clusters.to_string(),
        );
        section.fields.insert(
            "Total cluster instances".to_owned(),
            self.total_cluster_instances.to_string(),
        );
        section.fields.insert(
            "Total utterances".to_owned(),
            self.total_utterances.to_string(),
        );

        result.add_section(section);
        result
    }
}

impl CommandOutput for CooccurResult {
    /// Render via the shared tabular text formatter.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// Format:
    /// ```text
    ///   1  gonna put
    ///   1  more cookie
    ///   1  the choo+choo's
    /// ```
    ///
    /// With `+nN` (cluster_size > 2) each row carries N space-
    /// separated display words instead of 2.
    fn render_clan(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        // CLAN sorts alphabetically by display-form sequence.
        let mut sorted: Vec<&CooccurCluster> = self.clusters.iter().collect();
        sorted.sort_by(|a, b| a.displays.cmp(&b.displays));

        for cluster in &sorted {
            let words = cluster.displays.join(" ");
            if self.no_frequency_counts {
                // CLAN `+d`: word-only row, no count column.
                writeln!(out, "{words}").ok();
            } else {
                writeln!(out, "{:>3}  {words}", cluster.count).ok();
            }
        }

        out
    }
}
