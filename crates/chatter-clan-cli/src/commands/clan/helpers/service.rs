use std::path::{Path, PathBuf};

use crate::cli::{ClanOutputFormat, CommonAnalysisArgs};
use talkbank_clan::commands::freq::FreqSpreadsheetMode;
use talkbank_clan::framework::{DiscoveredChatFiles, OutputFormat, format_clan_banner};
use talkbank_clan::service::AnalysisService;
use talkbank_clan::service_types::{
    AnalysisCommandName, AnalysisOptions, AnalysisPlan, AnalysisRequest, AnalysisRequestBuilder,
    ClanScopeMode,
};

use super::{
    banner::{
        CLAN_BANNER_VERSION, build_main_scope, clan_invocation_echo, clan_source_for,
        clan_timestamp_now,
    },
    filter::build_filter,
    io::exit_with_error,
};

pub(in super::super) fn run_analysis_and_print(
    options: AnalysisOptions,
    paths: &[PathBuf],
    common: &CommonAnalysisArgs,
) {
    let command_name = options.command_name();
    let plan = build_analysis_plan_or_exit(options);
    let AnalysisPlan::Service(request) = plan else {
        exit_with_error(format!(
            "Error: {command_name} requires paired-file execution"
        ));
    };

    // FREQ `+d2`/`+d3` write an aggregate SpreadsheetML file (full mirror of
    // CLAN's file-argument behaviour: a `stat.frq*.xls` in the working
    // directory) instead of rendering text to stdout.
    if let AnalysisRequest::Freq(config) = &request
        && let Some(mode) = config.spreadsheet
    {
        run_freq_spreadsheet_and_write(request, paths, common, mode);
        return;
    }

    run_request_and_print(command_name, request, paths, common);
}

/// File CLAN writes the FREQ spreadsheet to: `+d2` (per-word) -> `stat.frq.xls`,
/// `+d3` (summary) -> `stat.frq0.xls`, `+d20` (one-per-row, also `onlydata = 3`)
/// -> `stat.frq.xls`, `+dCN` (percent-of-speakers, `onlydata = 4`) ->
/// `words.frq.xls` (`freq.cpp` `StatName` / `WordsName` + `.xls`).
fn freq_spreadsheet_filename(mode: FreqSpreadsheetMode) -> &'static str {
    match mode {
        FreqSpreadsheetMode::PerWord | FreqSpreadsheetMode::PerSpeakerWord => "stat.frq.xls",
        FreqSpreadsheetMode::TypesTokens => "stat.frq0.xls",
        FreqSpreadsheetMode::PercentOfSpeakers(_) => "words.frq.xls",
    }
}

/// Build the FREQ `+d2`/`+d3` spreadsheet across the input files and write it to
/// the working directory, mirroring CLAN's file-argument behaviour.
fn run_freq_spreadsheet_and_write(
    request: AnalysisRequest,
    paths: &[PathBuf],
    common: &CommonAnalysisArgs,
    mode: FreqSpreadsheetMode,
) {
    let (files, service) = discover_files_and_service(paths, common);
    let workbook = service
        .execute_spreadsheet(request, &files)
        .unwrap_or_else(|err| exit_with_error(format!("Error: {err}")));
    let xml = workbook
        .write_xml()
        .unwrap_or_else(|err| exit_with_error(format!("Error: {err}")));

    let filename = freq_spreadsheet_filename(mode);
    if let Err(err) = std::fs::write(filename, &xml) {
        exit_with_error(format!("Error writing {filename}: {err}"));
    }
    eprintln!("Output file \"{filename}\"");
}

pub(in super::super) fn run_paired_analysis_and_print(
    options: AnalysisOptions,
    primary_file: &Path,
    format: ClanOutputFormat,
) {
    let command_name = options.command_name();
    let plan = build_analysis_plan_or_exit(options);
    let AnalysisPlan::Rely(request) = plan else {
        exit_with_error(format!(
            "Error: {command_name} does not support paired-file execution"
        ));
    };

    let service = AnalysisService::new();
    match service.execute_rely_rendered(request, primary_file, convert_format(format)) {
        Ok(result) => print!("{result}"),
        Err(error) => exit_with_error(format!("Error: {error}")),
    }
}

fn build_analysis_plan_or_exit(options: AnalysisOptions) -> AnalysisPlan {
    AnalysisRequestBuilder::new(options)
        .build()
        .unwrap_or_else(|error| exit_with_error(format!("Error: {error}")))
}

/// Discover the analysis input files from `paths` (warning on entries that are
/// neither a file nor a directory, exiting if none remain) and build the
/// filtered [`AnalysisService`]. Shared by the stdout-render and the
/// spreadsheet-file output paths.
fn discover_files_and_service(
    paths: &[PathBuf],
    common: &CommonAnalysisArgs,
) -> (Vec<PathBuf>, AnalysisService) {
    let discovered_files = DiscoveredChatFiles::from_paths(paths);
    for skipped_path in discovered_files.skipped_paths() {
        eprintln!(
            "Warning: {:?} is not a file or directory, skipping",
            skipped_path
        );
    }
    // Fail closed: a flag-shaped token (`+d8`, `+QQQ`, ...) that no command
    // consumed is a user error, not a missing file to skip silently.
    let files = discovered_files
        .into_files()
        .unwrap_or_else(|err| exit_with_error(format!("Error: {err}")));
    if files.is_empty() {
        exit_with_error("Error: no .cha files found".to_owned());
    }
    let filter = build_filter(common).unwrap_or_else(|err| {
        exit_with_error(format!("Error: {err}"));
    });
    (files, AnalysisService::with_filter(filter))
}

fn run_request_and_print(
    command_name: AnalysisCommandName,
    request: AnalysisRequest,
    paths: &[PathBuf],
    common: &CommonAnalysisArgs,
) {
    let (files, service) = discover_files_and_service(paths, common);
    let format = convert_format(common.format);
    let want_clan_banner = matches!(format, OutputFormat::Clan);
    let scope = clan_scope_for(command_name, common);

    // CLAN's banner line 1 echoes the user's argv verbatim
    // (e.g. `freq +scat <path>`); we compute it once per call.
    let invocation = clan_invocation_echo();

    if common.per_file {
        match service.execute_rendered_per_file(request, &files, format) {
            Ok(results) => {
                for (path, result) in results {
                    if want_clan_banner {
                        print!(
                            "{}",
                            format_clan_banner(
                                &invocation,
                                &command_name.to_string(),
                                CLAN_BANNER_VERSION,
                                &scope,
                                &clan_source_for(&path),
                                &clan_timestamp_now(),
                            )
                        );
                    } else {
                        println!("From file: {}", path.display());
                    }
                    print!("{result}");
                    if !want_clan_banner {
                        println!();
                    }
                }
            }
            Err(e) => exit_with_error(format!("Error: {e}")),
        }
    } else {
        match service.execute_rendered(request, &files, format) {
            Ok(result) => {
                if want_clan_banner {
                    // Aggregated mode: CLAN emits the banner once. Use the
                    // first input path as the `From file …` reference.
                    let source = files
                        .first()
                        .map(|p| clan_source_for(p))
                        .unwrap_or_else(|| "From pipe input".to_owned());
                    print!(
                        "{}",
                        format_clan_banner(
                            &invocation,
                            &command_name.to_string(),
                            CLAN_BANNER_VERSION,
                            &scope,
                            &source,
                            &clan_timestamp_now(),
                        )
                    );
                }
                print!("{result}");
            }
            Err(e) => exit_with_error(format!("Error: {e}")),
        }
    }
}

/// Describe the analysis scope the way CLAN's mainloop does. The
/// three banner shapes, main-only, dep-only, and combined, match
/// CLAN's `cutt.cpp` mainloop (near line 12100) branching on `nomain`
/// and `tct`. Per-command selection lives in
/// [`AnalysisCommandName::clan_scope_mode`].
fn clan_scope_for(command_name: AnalysisCommandName, common: &CommonAnalysisArgs) -> String {
    let main_scope = build_main_scope(
        &common.speaker,
        &common.exclude_speaker,
        &common.role,
        common.id_filter.as_ref(),
    );
    match command_name.clan_scope_mode() {
        ClanScopeMode::MainOnly => main_scope,
        ClanScopeMode::DependentOnly(tier) => {
            format!("ONLY dependent tiers matching: %{};", tier.to_uppercase())
        }
        ClanScopeMode::MainAndDependent(tier) => format!(
            "{main_scope}\n\tand those speakers' ONLY dependent tiers matching: %{};",
            tier.to_uppercase()
        ),
    }
}

fn convert_format(format: ClanOutputFormat) -> OutputFormat {
    match format {
        ClanOutputFormat::Text => OutputFormat::Text,
        ClanOutputFormat::Json => OutputFormat::Json,
        ClanOutputFormat::Csv => OutputFormat::Csv,
        ClanOutputFormat::Clan => OutputFormat::Clan,
    }
}
