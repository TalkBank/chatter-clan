use std::fmt::Write;

use serde::Serialize;

use crate::framework::CommandOutput;

/// A single model→replica mapping entry.
#[derive(Debug, Clone, Serialize)]
pub struct ReplicaEntry {
    /// The replica word form.
    pub word: String,
    /// Number of times this model→replica pairing was observed.
    pub count: u64,
}

/// A single model word with its replica variants.
#[derive(Debug, Clone, Serialize)]
pub struct ModelEntry {
    /// The model (target) word form.
    pub model: String,
    /// Total occurrences of this model word.
    pub total: u64,
    /// All replica variants observed for this model word.
    pub replicas: Vec<ReplicaEntry>,
}

/// Per-speaker MODREP result.
#[derive(Debug, Clone, Serialize)]
pub struct ModrepSpeakerResult {
    /// Speaker code.
    pub speaker: String,
    /// Model word entries with replica variants, sorted alphabetically.
    pub entries: Vec<ModelEntry>,
}

/// Typed output from the MODREP command.
#[derive(Debug, Clone, Serialize)]
pub struct ModrepResult {
    /// Per-speaker results in encounter order.
    pub speakers: Vec<ModrepSpeakerResult>,
}

impl CommandOutput for ModrepResult {
    /// Render per-speaker model/replica mappings in CLAN-like tabular text.
    fn render_text(&self) -> String {
        let mut out = String::new();

        for speaker in &self.speakers {
            writeln!(out, "Speaker *{}:", speaker.speaker).ok();
            for entry in &speaker.entries {
                writeln!(out, "  {:>3} {}", entry.total, entry.model).ok();
                for replica in &entry.replicas {
                    writeln!(out, "      {:>3} {}", replica.count, replica.word).ok();
                }
            }
            writeln!(out).ok();
        }

        out
    }

    /// CLAN output is currently identical to `render_text()` for this command.
    fn render_clan(&self) -> String {
        // CLAN format matches our text format for this command
        self.render_text()
    }
}
