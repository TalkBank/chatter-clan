//! CLAN `+s` search-pattern matching (the `patmat` wildcard semantics).
//!
//! Extracted verbatim from `word_filter.rs`; the parent re-exports
//! [`word_pattern_matches`] so `word_filter::word_pattern_matches` (and the
//! `framework` re-export) continue to resolve.

/// Match a word against a CLAN `+s` search pattern (both should be lowercased).
///
/// CLAN uses exact word matching by default. Wildcards (`*`) match
/// zero or more characters:
/// - `cookie` matches only "cookie" (exact)
/// - `cook*` matches "cookie", "cookies", "cook" (prefix)
/// - `*ing` matches "going", "running" (suffix)
/// - `*ook*` matches "cookie", "book" (contains)
pub fn word_pattern_matches(word: &str, pattern: &str) -> bool {
    if !pattern.contains('*') {
        return word == pattern;
    }

    let parts: Vec<&str> = pattern.split('*').collect();

    if parts.len() == 2 {
        let (prefix, suffix) = (parts[0], parts[1]);
        if prefix.is_empty() && suffix.is_empty() {
            return true; // "*" matches everything
        }
        if prefix.is_empty() {
            return word.ends_with(suffix);
        }
        if suffix.is_empty() {
            return word.starts_with(prefix);
        }
        return word.starts_with(prefix)
            && word.ends_with(suffix)
            && word.len() >= prefix.len() + suffix.len();
    }

    // General multi-wildcard: segments must appear in order
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !word[pos..].starts_with(part) {
                return false;
            }
            pos += part.len();
        } else if i == parts.len() - 1 {
            if !word[pos..].ends_with(part) {
                return false;
            }
        } else {
            match word[pos..].find(part) {
                Some(found) => pos += found + part.len(),
                None => return false,
            }
        }
    }
    true
}
