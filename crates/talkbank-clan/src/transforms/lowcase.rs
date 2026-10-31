//! LOWCASE -- lowercase all words on main tiers.
//!
//! Reimplements CLAN's `lowcase` command, which converts all words on main
//! tiers to lowercase. Speaker codes, headers, and dependent tiers are
//! preserved unchanged. The transformation recurses into annotated words,
//! replaced words, groups, and annotated groups.
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409329)
//! for the original command documentation.
//!
//! # Differences from CLAN
//!
//! - Operates on the typed AST rather than raw text line scanning.
//! - Uses the framework transform pipeline (parse → transform → serialize).
//! - Recurses into all AST word-bearing nodes (annotated words, replaced
//!   words, groups, annotated groups) rather than lowercasing raw line text,
//!   ensuring structural markers and speaker codes are never affected.

use talkbank_model::{BracketedItem, ChatFile, Line, UtteranceContent, Word};

use crate::framework::{TransformCommand, TransformError, replace_word_text};

/// LOWCASE transform: lowercase all words on main tiers.
pub struct LowcaseCommand;

impl TransformCommand for LowcaseCommand {
    type Config = ();

    /// Lowercase all main-tier word surfaces while preserving structure.
    fn transform(&self, file: &mut ChatFile) -> Result<(), TransformError> {
        for line in file.lines.as_mut_slice().iter_mut() {
            if let Line::Utterance(utterance) = line {
                lowercase_content(utterance.main.content.content.as_mut_slice())?;
            }
        }
        Ok(())
    }
}

/// Lowercase all words in utterance content items.
fn lowercase_content(items: &mut [UtteranceContent]) -> Result<(), TransformError> {
    for item in items.iter_mut() {
        match item {
            UtteranceContent::Word(word) => {
                lowercase_word(word)?;
            }
            UtteranceContent::AnnotatedWord(annotated) => {
                lowercase_word(&mut annotated.inner)?;
            }
            UtteranceContent::ReplacedWord(replaced) => {
                lowercase_word(&mut replaced.word)?;
                for rep in replaced.replacement.words.as_mut_slice().iter_mut() {
                    lowercase_word(rep)?;
                }
            }
            UtteranceContent::Group(group) => {
                lowercase_bracketed_items(group.content.content.as_mut_slice())?;
            }
            UtteranceContent::AnnotatedGroup(annotated) => {
                lowercase_bracketed_items(annotated.inner.content.content.as_mut_slice())?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Lowercase a single word's raw_text (which drives serialization).
///
/// The English first-person pronoun "I" is preserved as uppercase, matching
/// CLAN's behavior.
fn lowercase_word(word: &mut Word) -> Result<(), TransformError> {
    // Preserve the pronoun "I", CLAN keeps it uppercase.
    if word.cleaned_text() == "I" {
        return Ok(());
    }
    let raw = word.raw_text().to_owned();
    let lowered = raw.to_lowercase();
    if lowered != raw {
        replace_word_text(word, &lowered)?;
    }
    Ok(())
}

/// Lowercase words inside bracketed items (groups).
fn lowercase_bracketed_items(items: &mut [BracketedItem]) -> Result<(), TransformError> {
    for item in items.iter_mut() {
        match item {
            BracketedItem::Word(word) => {
                lowercase_word(word)?;
            }
            BracketedItem::AnnotatedWord(annotated) => {
                lowercase_word(&mut annotated.inner)?;
            }
            BracketedItem::ReplacedWord(replaced) => {
                lowercase_word(&mut replaced.word)?;
                for rep in replaced.replacement.words.as_mut_slice().iter_mut() {
                    lowercase_word(rep)?;
                }
            }
            BracketedItem::AnnotatedGroup(annotated) => {
                lowercase_bracketed_items(annotated.inner.content.content.as_mut_slice())?;
            }
            _ => {}
        }
    }
    Ok(())
}
