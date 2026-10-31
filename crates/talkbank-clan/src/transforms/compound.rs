//! COMPOUND -- normalize compound word formatting.
//!
//! Reimplements CLAN's COMPOUND command, which normalizes compound word
//! notation in CHAT files. In CHAT, compound words are joined with `+`
//! (e.g., `ice+cream`). This command converts dash-joined compounds to the
//! canonical plus notation.
//!
//! # Operations
//!
//! - Normalize dash-joined compounds to plus notation: `ice-cream` --> `ice+cream`
//! - Preserves filler prefixes (`&-uh`) and omission prefixes (`0word`)
//! - Only converts when all parts are purely alphabetic
//!
//! # Differences from CLAN
//!
//! - Operates on the typed AST rather than raw text line scanning.
//! - Uses the framework transform pipeline (parse → transform → serialize).
//! - Modifies `Word` surface forms in the AST, preserving surrounding
//!   structure (annotations, brackets) that raw text substitution could break.

use talkbank_model::{ChatFile, Line, UtteranceContent, Word};

use crate::framework::{TransformCommand, TransformError, replace_word_text};

/// Configuration for the COMPOUND command.
pub struct CompoundConfig {
    /// Convert dashes to plus signs in compound words.
    pub dash_to_plus: bool,
}

/// COMPOUND transform: normalize compound word formatting.
pub struct CompoundCommand {
    config: CompoundConfig,
}

impl CompoundCommand {
    /// Create a new COMPOUND command.
    pub fn new(config: CompoundConfig) -> Self {
        Self { config }
    }
}

impl TransformCommand for CompoundCommand {
    type Config = CompoundConfig;

    /// Convert dash-joined compounds to plus notation across all main tiers.
    fn transform(&self, file: &mut ChatFile) -> Result<(), TransformError> {
        if !self.config.dash_to_plus {
            return Ok(());
        }

        for line in file.lines.as_mut_slice().iter_mut() {
            if let Line::Utterance(utt) = line {
                normalize_compounds(utt.main.content.content.as_mut_slice())?;
            }
        }
        Ok(())
    }
}

/// Normalize compound words in utterance content.
fn normalize_compounds(items: &mut [UtteranceContent]) -> Result<(), TransformError> {
    for item in items.iter_mut() {
        match item {
            UtteranceContent::Word(word) => normalize_compound_word(word)?,
            UtteranceContent::AnnotatedWord(annotated) => {
                normalize_compound_word(&mut annotated.inner)?;
            }
            UtteranceContent::ReplacedWord(replaced) => {
                normalize_compound_word(&mut replaced.word)?;
                for rep in replaced.replacement.words.as_mut_slice().iter_mut() {
                    normalize_compound_word(rep)?;
                }
            }
            UtteranceContent::Group(group) => {
                for bi in group.content.content.as_mut_slice().iter_mut() {
                    if let talkbank_model::BracketedItem::Word(w) = bi {
                        normalize_compound_word(w)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Normalize a single compound word: convert dashes to plus signs.
fn normalize_compound_word(word: &mut Word) -> Result<(), TransformError> {
    let raw = word.raw_text().to_owned();
    if raw.contains('-') && !raw.starts_with('&') && !raw.starts_with('0') {
        // Only convert dashes that look like compound joins
        // (not morphological markers like "-PL" or prefixed words)
        let parts: Vec<&str> = raw.split('-').collect();
        if parts.len() >= 2
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_alphabetic()))
        {
            let normalized = parts.join("+");
            replace_word_text(word, &normalized)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_dash_compound() {
        let mut word = crate::test_support::word("ice-cream");
        normalize_compound_word(&mut word).expect("valid replacement");
        assert_eq!(word.raw_text(), "ice+cream");
    }

    #[test]
    fn preserve_filler_prefix() {
        let mut word = crate::test_support::word("&-uh");
        normalize_compound_word(&mut word).expect("valid replacement");
        assert_eq!(word.raw_text(), "&-uh");
    }

    #[test]
    fn preserve_omission_prefix() {
        let mut word = crate::test_support::word("0word");
        let original = word.raw_text().to_owned();
        normalize_compound_word(&mut word).expect("valid replacement");
        assert_eq!(word.raw_text(), original);
    }
}
