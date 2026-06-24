use std::path::PathBuf;

use crate::cli::CommonAnalysisArgs;
use talkbank_clan::framework::{
    FilterConfig, GemFilter, GemLabel, LoadWordListError, SpeakerFilter, WordFilter,
    WordFilterMode, WordPattern, load_word_list_file,
};
use talkbank_model::SpeakerCode;

use super::exit_with_error;

pub(super) fn collect_word_patterns(
    cli: &[String],
    files: &[PathBuf],
) -> Result<Vec<WordPattern>, LoadWordListError> {
    let mut patterns: Vec<WordPattern> =
        cli.iter().map(|s| WordPattern::from(s.as_str())).collect();
    for path in files {
        patterns.extend(load_word_list_file(path)?);
    }
    Ok(patterns)
}

/// Extract the CLAN `+sWORD` / `-sWORD` patterns from `common`,
/// build a [`WordFilter`] with [`WordFilterMode::PerWordEmit`], and
/// clear the word-filter fields on `common` so the framework's
/// utterance-gate sees an empty include/exclude list.
///
/// Per-word commands (FREQ, …) call this at their CLI entry point;
/// it is the single source of truth for "which `common` fields
/// count as word-filter inputs" so a future field addition (e.g.
/// `--include-word-regex`) updates one place.
pub(in super::super) fn take_per_word_filter(
    common: &mut CommonAnalysisArgs,
) -> Result<WordFilter, LoadWordListError> {
    let include = collect_word_patterns(&common.include_word, &common.include_word_file)?;
    let exclude = collect_word_patterns(&common.exclude_word, &common.exclude_word_file)?;
    common.include_word.clear();
    common.include_word_file.clear();
    common.exclude_word.clear();
    common.exclude_word_file.clear();
    Ok(WordFilter {
        include,
        exclude,
        case_sensitive: common.case_sensitive,
        mode: WordFilterMode::PerWordEmit,
    })
}

/// Sibling of [`collect_word_patterns`] for COMBO's
/// `+s@FILE` / `-s@FILE`. Each surviving line is a search-
/// expression string (parsed downstream by `SearchExpr::parse`);
/// returns lines concatenated in argv order. Exits on the first
/// I/O failure because this loader runs at dispatch time, outside
/// the `build_filter` error-bubbling path.
pub(in super::super) fn load_search_expr_files_or_exit(files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .flat_map(|file| {
            talkbank_clan::framework::load_search_expr_file(file)
                .unwrap_or_else(|err| exit_with_error(format!("Error: {err}")))
        })
        .collect()
}

pub(super) fn build_filter(common: &CommonAnalysisArgs) -> Result<FilterConfig, LoadWordListError> {
    let speaker_filter = SpeakerFilter {
        include: common.speaker.iter().map(SpeakerCode::new).collect(),
        exclude: common
            .exclude_speaker
            .iter()
            .map(SpeakerCode::new)
            .collect(),
    };

    let gem_filter = GemFilter {
        include: common
            .gem
            .iter()
            .map(|s| GemLabel::from(s.as_str()))
            .collect(),
        exclude: common
            .exclude_gem
            .iter()
            .map(|s| GemLabel::from(s.as_str()))
            .collect(),
    };

    // `--include-word-file` / `--exclude-word-file` load patterns
    // from disk and append to whatever `--include-word` /
    // `--exclude-word` already accumulated. Order: CLI patterns
    // first, then file patterns in `--…-file` argv order, with
    // each file's lines in source order.
    // Utterance-gate filter. Per-word commands (FREQ, …) extract
    // their patterns via `take_per_word_filter` before reaching here,
    // leaving the include/exclude lists empty for those commands.
    let word_filter = WordFilter {
        include: collect_word_patterns(&common.include_word, &common.include_word_file)?,
        exclude: collect_word_patterns(&common.exclude_word, &common.exclude_word_file)?,
        case_sensitive: common.case_sensitive,
        mode: WordFilterMode::UtteranceContext,
    };

    let role_filter = talkbank_clan::framework::RoleFilter {
        include: common.role.clone(),
    };

    // CLAN `-xWORD` (literal) and `-x@FILE` (loaded) both feed the single `+x`
    // exclude-from-count list, via the same literals-then-files collection the
    // `+s`/`-s` filters use; the fallible file read bubbles via `?` before the
    // (infallible) filter construction.
    let utterance_length_exclude = collect_word_patterns(
        &common.utterance_length_exclude,
        &common.utterance_length_exclude_file,
    )?;

    Ok(FilterConfig {
        speakers: speaker_filter,
        gems: gem_filter,
        words: word_filter,
        utterance_range: common.range,
        // CLAN `-xS` / `-x@FILE` / `+xxxx` arrive as separate argv tokens
        // (`--utterance-length-exclude[-file]` / `--utterance-length-restore`);
        // fold them into the parsed count form. Inert without `--utterance-length`
        // (the word-list / marker set alone is a no-op, matching CLAN where
        // `+xxxx` without a `+x C N U` count form does nothing).
        utterance_length: common.utterance_length.clone().map(move |mut length| {
            length.exclude_from_count = utterance_length_exclude;
            length.restore = talkbank_clan::framework::RestoreMarkers::from_statuses(
                &common.utterance_length_restore,
            );
            length
        }),
        id_filter: common.id_filter.clone(),
        roles: role_filter,
        ..FilterConfig::default()
    })
}
