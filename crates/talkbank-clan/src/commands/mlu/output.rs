//! Typed MLU results and rendering logic.

use std::fmt;
use std::path::Path;

use serde::Serialize;
use talkbank_model::IDHeader;
use talkbank_model::model::content::word::UntranscribedStatus;

use crate::framework::id_columns::{ID_COLUMN_HEADERS, speaker_id_to_cells};
use crate::framework::spreadsheet::{Cell, ColumnWidth, Row, SheetName, Workbook, Worksheet};
use crate::framework::{CommandOutput, MorphemeCount, UtteranceCount};

/// Typed output from the MLU command.
///
/// Contains per-speaker MLU statistics with strongly-typed numeric fields,
/// replacing the stringly-typed `AnalysisResult` for programmatic access.
#[derive(Debug, Clone, Serialize)]
pub struct MluResult {
    /// Per-speaker MLU statistics, in encounter order.
    pub speakers: Vec<MluSpeakerResult>,
    /// CLAN `+o3`: the single `speakers` entry is the pooled result, rendered
    /// `*COMBINED*` instead of `*<code>:`. Internal render flag, not serialized,
    /// so default-mode JSON is unchanged.
    #[serde(skip)]
    pub combine_speakers: bool,
    /// CLAN `+sxxx`/`+syyy` re-included untranscribed statuses, drives which of
    /// the three CLAN header variants (mlu.cpp:246-253) is printed. Internal
    /// render flag, not serialized.
    #[serde(skip)]
    pub re_included_untranscribed: Vec<UntranscribedStatus>,
    /// CLAN `+d`/`+d1` speaker rows, when an ID-oriented display mode is selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_rows: Option<Vec<MluIdRow>>,
}

/// Unit of the MLU value and count column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MluCountUnit {
    /// Morphemes from `%mor`.
    Morphemes,
    /// Words from the main tier (`-bw`).
    Words,
}

/// One tab-separated CLAN MLU `+d1` row.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MluIdRow {
    /// A speaker's row. Missing ID headers are rendered with placeholder fields.
    Speaker {
        /// Source CHAT input path, as supplied to CLAN.
        file: std::path::PathBuf,
        /// Parsed speaker identity, when the file contains an `@ID` for it.
        id_header: Option<Box<IDHeader>>,
        /// Speaker MLU statistics for this file.
        result: MluSpeakerResult,
    },
    /// The pooled `+o3` row.
    Combined {
        /// First source CHAT input path, as supplied to CLAN.
        file: std::path::PathBuf,
        /// Pooled MLU statistics.
        result: MluSpeakerResult,
    },
}

impl MluResult {
    /// Build CLAN's `+d` SpreadsheetML workbook from the per-file speaker rows.
    pub fn to_spreadsheet(
        &self,
        worksheet_name: &str,
        count_unit: MluCountUnit,
    ) -> Option<Workbook> {
        let rows = self.id_rows.as_deref()?;
        let count_name = match count_unit {
            MluCountUnit::Morphemes => "morphemes",
            MluCountUnit::Words => "words",
        };
        let mut header_cells = vec![Cell::text("File")];
        header_cells.extend(ID_COLUMN_HEADERS.iter().map(|header| {
            if *header == "Custom field" {
                Cell::text("Custom_field")
            } else {
                Cell::text(*header)
            }
        }));
        header_cells.extend([
            Cell::text("#utterances"),
            Cell::text(format!("#{count_name}")),
            Cell::text(format!("{count_name}/utterances")),
            Cell::text("Standard deviation"),
        ]);
        let mut workbook_rows = vec![Row::new(header_cells)];
        workbook_rows.extend(rows.iter().map(spreadsheet_row));
        Some(Workbook::new(vec![Worksheet::new(
            SheetName::new(worksheet_name),
            ColumnWidth(95),
            workbook_rows,
        )]))
    }

    /// The CLAN exclusion-header line(s) under the `MLU for Speaker:` row.
    ///
    /// CLAN picks one of three forms (mlu.cpp:246-253), keyed only on whether
    /// `xxx`/`yyy` were re-included (`www` is never includable). Note CLAN has
    /// NO `yyy`-only branch: `+syyy` alone still prints the default header even
    /// though it re-includes the `yyy` utterances in the count. We reproduce
    /// that exactly (the binary is the `chatter-clan` parity oracle); see the
    /// MLU audit's documented-divergence note on the `+syyy` manual conflict.
    fn exclusion_header(&self) -> &'static str {
        let xxx = self
            .re_included_untranscribed
            .contains(&UntranscribedStatus::Unintelligible);
        let yyy = self
            .re_included_untranscribed
            .contains(&UntranscribedStatus::Phonetic);
        match (xxx, yyy) {
            (true, true) => concat!(
                "  MLU (xxx and yyy are EXCLUDED from the morpheme counts, but are INCLUDED in utterance counts):\n",
                "  MLU (www is EXCLUDED from the utterance and morpheme counts):\n",
            ),
            (true, false) => concat!(
                "  MLU (xxx is EXCLUDED from the morpheme counts, but is INCLUDED in utterance counts):\n",
                "  MLU (yyy and www are EXCLUDED from the utterance and morpheme counts):\n",
            ),
            // `(false, true)` (yyy-only) falls here too: CLAN has no yyy-only
            // header branch, so it prints the default line.
            _ => "  MLU (xxx, yyy and www are EXCLUDED from the utterance and morpheme counts):\n",
        }
    }
}

fn spreadsheet_row(row: &MluIdRow) -> Row {
    let (file, id_header, speaker, result) = match row {
        MluIdRow::Speaker {
            file,
            id_header,
            result,
        } => (
            file.as_path(),
            id_header.as_deref(),
            result.speaker.as_str(),
            result,
        ),
        MluIdRow::Combined { file, result } => (file.as_path(), None, "*COMBINED*", result),
    };
    let filename = file
        .file_name()
        .unwrap_or(file.as_os_str())
        .to_string_lossy()
        .into_owned();
    let mut cells = vec![Cell::text(filename)];
    cells.extend(speaker_id_to_cells(id_header, speaker));
    cells.push(Cell::count(result.utterances));
    cells.push(Cell::count(result.morphemes));
    cells.push(if result.utterances == 0 {
        Cell::text("NA")
    } else {
        Cell::ratio(result.mlu)
    });
    cells.push(if result.utterances <= 1 || !result.sd.is_finite() {
        Cell::text("NA")
    } else {
        Cell::ratio(result.sd)
    });
    Row::new(cells)
}

/// MLU statistics for a single speaker.
#[derive(Debug, Clone, Serialize)]
pub struct MluSpeakerResult {
    /// Speaker code (e.g., "CHI", "MOT")
    pub speaker: String,
    /// Number of utterances included in the calculation
    pub utterances: UtteranceCount,
    /// Total morphemes (or words, if `--words` mode) across all utterances
    pub morphemes: MorphemeCount,
    /// Mean length of utterance (morphemes / utterances)
    pub mlu: f64,
    /// Population standard deviation of utterance lengths (/ n denominator).
    /// NAN when n=1 (rendered as "NA" in CLAN format).
    pub sd: f64,
    /// Minimum utterance length
    pub min: MorphemeCount,
    /// Maximum utterance length
    pub max: MorphemeCount,
}

impl CommandOutput for MluResult {
    /// Our clean text format.
    fn render_text(&self) -> String {
        if let Some(rows) = &self.id_rows {
            return render_id_rows(rows);
        }

        let mut out = String::new();
        for (i, s) in self.speakers.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            fmt::write(
                &mut out,
                format_args!(
                    "Speaker: {}\n\
                     \x20 Utterances: {}\n\
                     \x20 Total morphemes: {}\n\
                     \x20 MLU: {:.3}\n\
                     \x20 SD: {:.3}\n\
                     \x20 Range: {}-{}\n",
                    s.speaker, s.utterances, s.morphemes, s.mlu, s.sd, s.min, s.max
                ),
            )
            .ok();
        }
        out
    }

    /// CLAN-compatible output matching legacy CLAN character-for-character.
    ///
    /// Format (from CLAN snapshot):
    /// ```text
    /// MLU for Speaker: *CHI:
    ///   MLU (xxx, yyy and www are EXCLUDED from the utterance and morpheme counts):
    /// \tNumber of: utterances = 2, morphemes = 3
    /// \tRatio of morphemes over utterances = 1.500
    /// \tStandard deviation = 0.500
    /// ```
    ///
    /// When utterances = 0, only header + counts are emitted (no Ratio/SD).
    /// When n = 1, SD is printed as "NA" (sample SD undefined).
    fn render_clan(&self) -> String {
        if let Some(rows) = &self.id_rows {
            return render_id_rows(rows);
        }

        let mut out = String::new();
        for (i, s) in self.speakers.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            // Header and counts are always present. CLAN `+o3` labels the pooled
            // result `*COMBINED*` (no `:`); per-speaker rows use `*<code>:`.
            let label = if self.combine_speakers {
                "*COMBINED*".to_owned()
            } else {
                format!("*{}:", s.speaker)
            };
            fmt::write(
                &mut out,
                format_args!(
                    "MLU for Speaker: {}\n{}\tNumber of: utterances = {}, morphemes = {}\n",
                    label,
                    self.exclusion_header(),
                    s.utterances,
                    s.morphemes
                ),
            )
            .ok();

            // When utterances = 0, CLAN omits Ratio and SD lines entirely
            if s.utterances > 0 {
                fmt::write(
                    &mut out,
                    format_args!("\tRatio of morphemes over utterances = {:.3}\n", s.mlu),
                )
                .ok();

                if s.sd.is_nan() {
                    // n=1: sample SD is undefined
                    fmt::write(&mut out, format_args!("\tStandard deviation = NA\n")).ok();
                } else {
                    fmt::write(
                        &mut out,
                        format_args!("\tStandard deviation = {:.3}\n", s.sd),
                    )
                    .ok();
                }
            }
        }
        // CLAN emits a trailing blank line after the last per-speaker
        // block; match that so a hex-level diff against legacy mlu
        // output ends cleanly.
        if !self.speakers.is_empty() {
            out.push('\n');
        }
        out
    }
}

fn render_id_rows(rows: &[MluIdRow]) -> String {
    let mut out = String::new();
    for row in rows {
        let (file, id_header, result, combined) = match row {
            MluIdRow::Speaker {
                file,
                id_header,
                result,
            } => (file.as_path(), id_header.as_deref(), result, false),
            MluIdRow::Combined { file, result } => (file.as_path(), None, result, true),
        };
        let mut columns = vec![file_path_for_output(file)];
        columns.extend(id_columns(
            id_header,
            if combined {
                "*COMBINED*"
            } else {
                result.speaker.as_str()
            },
        ));
        columns.push(format!("{:5}", result.utterances));
        columns.push(format!("{:5}", result.morphemes));
        columns.push(if result.utterances == 0 {
            "NA".to_owned()
        } else {
            format!("{:7.3}", result.mlu)
        });
        columns.push(if result.utterances == 0 || result.sd.is_nan() {
            "NA".to_owned()
        } else {
            format!("{:7.3}", result.sd)
        });
        out.push_str(&columns.join("\t"));
        out.push('\n');
    }
    out
}

fn id_columns(id_header: Option<&IDHeader>, speaker: &str) -> Vec<String> {
    let mut fields = vec![".".to_owned(); 10];
    if let Some(id) = id_header {
        let language = id
            .language
            .iter()
            .map(|code| code.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        fields[0] = field_or_dot(Some(&language));
        fields[1] = field_or_dot(Some(id.corpus.as_str()));
        fields[2] = field_or_dot(Some(id.speaker.as_str()));
        fields[3] = field_or_dot(id.age.as_ref().map(|value| value.as_str()));
        fields[4] = field_or_dot(id.sex.as_ref().map(|value| value.as_str()));
        fields[5] = field_or_dot(id.group.as_ref().map(|value| value.as_str()));
        let ses = id.ses.as_ref().map(|value| value.as_str());
        fields[6] = field_or_dot(ses.as_deref());
        fields[7] = field_or_dot(Some(id.role.as_str()));
        fields[8] = field_or_dot(id.education.as_ref().map(|value| value.as_str()));
        fields[9] = field_or_dot(id.custom_field.as_ref().map(|value| value.as_str()));
    } else {
        fields[2] = field_or_dot(Some(speaker));
    }
    fields
}

fn field_or_dot(value: Option<&str>) -> String {
    value
        .filter(|field| !field.is_empty())
        .unwrap_or(".")
        .to_owned()
}

fn file_path_for_output(file: &Path) -> String {
    file.to_string_lossy().into_owned()
}
