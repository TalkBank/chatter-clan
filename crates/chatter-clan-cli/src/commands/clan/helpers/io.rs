use std::path::Path;

use talkbank_clan::framework::{TransformCommand, run_transform};

pub(in super::super) fn run_normalize_alias(path: &Path, output: Option<&Path>) {
    let content = read_file_or_exit(path);
    let options = talkbank_model::ParseValidateOptions::default();
    match talkbank_transform::normalize_chat(&content, options) {
        Ok(normalized) => write_output_or_exit(&normalized, output),
        Err(e) => exit_with_error(format!("Error: {e}")),
    }
}

pub(in super::super) fn read_file_or_exit(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| {
        exit_with_error(format!("Error reading {}: {e}", path.display()));
    })
}

pub(in super::super) fn parse_chat_or_exit(path: &Path) -> talkbank_model::ChatFile {
    let content = read_file_or_exit(path);
    talkbank_transform::parse_and_validate(
        &content,
        talkbank_model::ParseValidateOptions::default(),
    )
    .unwrap_or_else(|e| exit_with_error(format!("Error parsing {}: {e}", path.display())))
}

pub(in super::super) fn write_output_or_exit(content: &str, output: Option<&Path>) {
    if let Some(path) = output {
        if let Err(e) = std::fs::write(path, content) {
            exit_with_error(format!("Error writing {}: {e}", path.display()));
        }
    } else {
        print!("{content}");
    }
}

pub(in super::super) fn run_converter(
    result: Result<talkbank_model::ChatFile, talkbank_clan::framework::TransformError>,
    output: Option<&Path>,
) {
    match result {
        Ok(chat) => write_output_or_exit(&chat.to_string(), output),
        Err(e) => exit_with_error(format!("Error: {e}")),
    }
}

pub(in super::super) fn run_transform_or_exit<T: TransformCommand>(
    cmd: &T,
    path: &Path,
    output: Option<&Path>,
) {
    if let Err(e) = run_transform(cmd, path, output) {
        exit_with_error(format!("Error: {e}"));
    }
}

pub(in super::super) fn exit_with_error(message: String) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

/// Emit a CLAN-style refusal message to stderr and exit non-zero,
/// matching CLAN's behavior when a required flag is missing.
///
/// CLAN's pre-banner refusals (`Please specify a code tier with
/// "+t" option.`, `Please specify ipsyn rules file name with "+l"
/// option.`, …) are deliberately byte-level reproduced, researchers'
/// scripts may grep stderr for these. Pass the exact message CLAN
/// emits; do not paraphrase.
pub(in super::super) fn exit_with_clan_refusal(message: &str) -> ! {
    exit_with_error(message.to_owned())
}
