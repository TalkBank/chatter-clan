//! Shared CLI helper modules for `chatter clan`.

mod arg_conversions;
mod banner;
mod filter;
mod io;
mod service;
#[cfg(test)]
mod tests;

pub(super) use arg_conversions::{
    capitalization_to_filter, multiword_args_to_match, multiword_display_to_display,
    option_if_not_default, parenthesis_mode_arg_to_mode, prosody_mode_arg_to_mode,
    replacement_mode_arg_to_choice, search_multiplicity_to_include, sort_arg_to_freq_sort,
    spreadsheet_arg_to_mode,
};
pub(super) use filter::{load_search_expr_files_or_exit, take_per_word_filter};
pub(super) use io::{
    exit_with_clan_refusal, exit_with_error, parse_chat_or_exit, read_file_or_exit, run_converter,
    run_normalize_alias, run_transform_or_exit, write_output_or_exit,
};
pub(super) use service::{run_analysis_and_print, run_paired_analysis_and_print};
