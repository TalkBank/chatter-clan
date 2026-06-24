use serde::Serialize;

use crate::framework::CommandOutput;

/// Typed output from the PHONFREQ command.
#[derive(Debug, Clone, Serialize)]
pub struct PhonfreqResult {
    /// Per-phone frequency entries, sorted alphabetically.
    pub entries: Vec<PhonfreqEntry>,
}

/// A single phone frequency entry.
#[derive(Debug, Clone, Serialize)]
pub struct PhonfreqEntry {
    /// The phone character
    pub phone: String,
    /// Total occurrences
    pub total: u64,
    /// Occurrences as first character of a pho word
    pub initial: u64,
    /// Occurrences as last character of a pho word
    pub final_pos: u64,
    /// Occurrences in middle positions
    pub other: u64,
}

impl CommandOutput for PhonfreqResult {
    /// Render per-phone totals and positional counts in CLAN-style columns.
    ///
    /// CLAN's phonfreq pads the phone column to 4 **bytes**, not 4
    /// characters, so multi-byte UTF-8 phones (æ, ð, ɑ, ə, ɛ, ɪ) get
    /// fewer trailing spaces than ASCII phones do. Rust's `{:<4}`
    /// pads by character count, which over-pads multi-byte chars by
    /// one position. We pad by byte length to match CLAN exactly.
    fn render_text(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        for entry in &self.entries {
            let pad = 4usize.saturating_sub(entry.phone.len());
            writeln!(
                out,
                "{:>3}  {}{} initial = {:>3}, final = {:>3}, other = {:>3}",
                entry.total,
                entry.phone,
                " ".repeat(pad),
                entry.initial,
                entry.final_pos,
                entry.other,
            )
            .ok();
        }

        out
    }

    /// CLAN output currently matches `render_text()` exactly for this command.
    fn render_clan(&self) -> String {
        self.render_text()
    }
}
