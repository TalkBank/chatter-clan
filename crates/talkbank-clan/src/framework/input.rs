//! Shared models for discovering CHAT files before analysis runs.
//!
//! The CLI and LSP both need to turn user-selected files or directories into the
//! flat list of CHAT files consumed by [`AnalysisRunner`](super::AnalysisRunner).
//! Keep that discovery behavior in the library so outer wrappers do not duplicate
//! directory walking or ad hoc skipped-path tracking.

use std::fmt;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// One or more positional arguments looked like CLAN flags (a leading `+` or
/// `-`) but no command consumed them.
///
/// clap does not treat a `+`-prefixed token as an option, so such a token is
/// collected as a positional *file* argument. Treating it as a missing file
/// (warn-and-skip) is fail-open: a CLAN user running `freq +d8` would get
/// silent default output instead of an error. CLAN's own `getflag` rejects
/// unknown options, and a token beginning with a flag sigil is never a
/// filename, so this is a hard error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecognizedClanFlagArgs {
    /// The offending tokens, in the order the user supplied them.
    flags: Vec<PathBuf>,
}

impl UnrecognizedClanFlagArgs {
    /// Borrow the flag-shaped tokens that triggered the error.
    pub fn flags(&self) -> &[PathBuf] {
        &self.flags
    }
}

impl fmt::Display for UnrecognizedClanFlagArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = self
            .flags
            .iter()
            .map(|flag| format!("'{}'", flag.display()))
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "unrecognized option(s): {rendered} (a '+'/'-' argument is a CLAN flag, not a file)"
        )
    }
}

impl std::error::Error for UnrecognizedClanFlagArgs {}

/// Result of discovering CHAT files from one or more user-selected paths.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoveredChatFiles {
    files: Vec<PathBuf>,
    skipped_paths: Vec<PathBuf>,
    /// Tokens that begin with a CLAN flag sigil (`+` / `-`) yet reached file
    /// discovery as positionals, i.e. flags no command consumed. Kept distinct
    /// from `skipped_paths` (genuinely-missing files, which are warn-and-skip)
    /// because an unconsumed flag is a hard error, not a missing file.
    unrecognized_flags: Vec<PathBuf>,
}

impl DiscoveredChatFiles {
    /// Discover CHAT files from a single file or directory path.
    pub fn from_path(path: &Path) -> Self {
        let mut discovered = Self::default();
        discovered.extend_from_path(path);
        discovered
    }

    /// Discover CHAT files from a list of file or directory paths.
    pub fn from_paths(paths: &[PathBuf]) -> Self {
        let mut discovered = Self::default();
        for path in paths {
            discovered.extend_from_path(path);
        }
        discovered
    }

    /// Borrow the discovered files in traversal order.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Consume the discovery result and return the discovered files, failing
    /// closed if any positional argument was a flag-shaped token no command
    /// consumed.
    ///
    /// This is the sole way to extract the file list, so every caller is forced
    /// by the type to handle the unrecognized-flag case rather than silently
    /// proceeding with default output (the fail-open bug this guards against).
    pub fn into_files(self) -> Result<Vec<PathBuf>, UnrecognizedClanFlagArgs> {
        if self.unrecognized_flags.is_empty() {
            Ok(self.files)
        } else {
            Err(UnrecognizedClanFlagArgs {
                flags: self.unrecognized_flags,
            })
        }
    }

    /// Borrow any user-provided paths that could not be resolved.
    pub fn skipped_paths(&self) -> &[PathBuf] {
        &self.skipped_paths
    }

    /// Borrow any positional arguments that were flag-shaped tokens (`+`/`-`)
    /// no command consumed.
    pub fn unrecognized_flags(&self) -> &[PathBuf] {
        &self.unrecognized_flags
    }

    /// Whether discovery produced zero files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn extend_from_path(&mut self, path: &Path) {
        // A token beginning with a CLAN flag sigil ('+' or '-') is never a
        // filename: it is a flag no rewriter arm consumed. Reject it fail-closed
        // here rather than letting it fall through to the missing-file warn-skip
        // below. Checked before `is_file()` so that even a pathological on-disk
        // file literally named `+d8` is treated as the flag CLAN syntax says it
        // is. A bare "-" (the conventional stdin marker) is exempt.
        if looks_like_clan_flag(path) {
            self.unrecognized_flags.push(path.to_path_buf());
            return;
        }

        if path.is_file() {
            self.files.push(path.to_path_buf());
            return;
        }

        if path.is_dir() {
            for entry in WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(Result::ok)
            {
                let candidate = entry.path();
                if candidate.is_file() && candidate.extension().is_some_and(|ext| ext == "cha") {
                    self.files.push(candidate.to_path_buf());
                }
            }
            return;
        }

        self.skipped_paths.push(path.to_path_buf());
    }
}

/// Whether a positional argument is a CLAN flag sigil rather than a path.
///
/// A token starting with `+` or `-` is a flag in CLAN invocation syntax; a bare
/// `-` is the conventional stdin marker and is deliberately exempt. Non-UTF-8
/// paths cannot be flags and are treated as ordinary paths.
fn looks_like_clan_flag(path: &Path) -> bool {
    match path.to_str() {
        Some("-") => false,
        Some(token) => token.starts_with('+') || token.starts_with('-'),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::DiscoveredChatFiles;

    /// Direct files should be preserved, and directories should contribute nested CHAT files.
    #[test]
    fn discovers_files_from_direct_paths_and_directories() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        let direct_file = root.join("direct.txt");
        let chat_file = root.join("nested").join("sample.cha");
        let ignored_file = root.join("nested").join("sample.txt");

        fs::create_dir_all(chat_file.parent().expect("parent")).expect("create nested dir");
        fs::write(&direct_file, "direct").expect("write direct file");
        fs::write(&chat_file, "@Begin\n@End\n").expect("write chat file");
        fs::write(&ignored_file, "ignore").expect("write ignored file");

        let discovered =
            DiscoveredChatFiles::from_paths(&[direct_file.clone(), root.join("nested")]);

        assert!(discovered.files().contains(&direct_file));
        assert!(discovered.files().contains(&chat_file));
        assert!(!discovered.files().contains(&ignored_file));
        assert!(discovered.skipped_paths().is_empty());
    }

    /// Invalid paths should be tracked so outer wrappers can surface warnings consistently.
    #[test]
    fn tracks_skipped_paths() {
        let missing = PathBuf::from("/definitely/not/a/real/chat/path");
        let discovered = DiscoveredChatFiles::from_paths(std::slice::from_ref(&missing));

        assert!(discovered.files().is_empty());
        assert_eq!(discovered.skipped_paths(), std::slice::from_ref(&missing));
        // A genuinely-missing FILE is not a flag: it stays warn-and-skip, so
        // `into_files` still succeeds (with an empty list), preserving CLAN's
        // "warn on a missing file, continue" behaviour.
        assert!(discovered.unrecognized_flags().is_empty());
        assert_eq!(
            DiscoveredChatFiles::from_paths(&[missing]).into_files(),
            Ok(Vec::new())
        );
    }

    /// A `+`-prefixed token is a flag no command consumed, not a file. It must
    /// route to `unrecognized_flags` (not `skipped_paths`) and make `into_files`
    /// fail closed, even when a real file is also present.
    #[test]
    fn flag_shaped_token_fails_closed() {
        let temp = tempdir().expect("tempdir");
        let chat_file = temp.path().join("real.cha");
        fs::write(&chat_file, "@Begin\n@End\n").expect("write chat file");

        let flag = PathBuf::from("+d8");
        let discovered = DiscoveredChatFiles::from_paths(&[flag.clone(), chat_file]);

        assert_eq!(discovered.unrecognized_flags(), std::slice::from_ref(&flag));
        assert!(discovered.skipped_paths().is_empty());
        let err = discovered
            .into_files()
            .expect_err("a flag-shaped token must fail closed");
        assert_eq!(err.flags(), &[flag]);
    }

    /// A bare `-` (the conventional stdin marker) is deliberately exempt from
    /// flag detection.
    #[test]
    fn bare_dash_is_not_a_flag() {
        let dash = PathBuf::from("-");
        let discovered = DiscoveredChatFiles::from_paths(std::slice::from_ref(&dash));

        assert!(discovered.unrecognized_flags().is_empty());
        // It is not a real file, so it stays in the warn-and-skip bucket.
        assert_eq!(discovered.skipped_paths(), &[dash]);
    }
}
