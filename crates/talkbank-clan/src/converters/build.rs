//! Typed constructors shared by the `*2chat` converters.
//!
//! Converters synthesize CHAT from foreign formats, so nothing they build has
//! a source position in a CHAT file: every location is `Location::Unlocated`,
//! stated here once instead of at each call site. Constructors that Chatter
//! makes fallible (language codes, word text) surface that as
//! [`TransformError`] rather than substituting a default.

use talkbank_model::{
    Bullet, Header, IDHeader, LanguageCode, LanguageCodes, Line, Location, MainTier,
    MainTierLocations, ParticipantEntries, ParticipantEntry, ParticipantRole, SpeakerCode,
    Terminator, UtteranceContent, Word, WordText,
};

use crate::framework::TransformError;

/// A header line with no source position.
pub(crate) fn header(header: Header) -> Line {
    Line::header(Location::Unlocated, header)
}

/// Parse a language code from converter options.
pub(crate) fn language(code: &str) -> Result<LanguageCode, TransformError> {
    LanguageCode::new(code)
        .map_err(|e| TransformError::Transform(format!("Invalid language code {code:?}: {e}")))
}

/// A participant with no name and the given role.
pub(crate) fn participant(speaker: SpeakerCode, role: ParticipantRole) -> ParticipantEntry {
    ParticipantEntry::new(speaker, None, role)
}

/// `@UTF8`, `@Begin`, `@Languages`, `@Participants` and one `@ID` per
/// participant: the preamble every converter emits.
pub(crate) fn preamble(
    language: &LanguageCode,
    participants: &[(SpeakerCode, ParticipantRole)],
    corpus: &str,
) -> Vec<Line> {
    let mut lines = vec![
        header(Header::Utf8),
        header(Header::Begin),
        header(Header::Languages {
            codes: LanguageCodes::new(vec![language.clone()]),
        }),
        header(Header::Participants {
            entries: ParticipantEntries::new(
                participants
                    .iter()
                    .map(|(speaker, role)| participant(speaker.clone(), role.clone()))
                    .collect(),
            ),
        }),
    ];
    lines.extend(participants.iter().map(|(speaker, role)| {
        header(Header::ID(
            IDHeader::new(language.clone(), speaker.clone(), role.clone()).with_corpus(corpus),
        ))
    }));
    lines
}

/// A word whose text is exactly `text`; empty text is an error.
pub(crate) fn word(text: &str) -> Result<UtteranceContent, TransformError> {
    let text = WordText::new(text)
        .map_err(|e| TransformError::Transform(format!("Cannot build word {text:?}: {e}")))?;
    Ok(UtteranceContent::Word(Box::new(Word::new(
        Location::Unlocated,
        text,
    ))))
}

/// A main tier with no source positions.
pub(crate) fn main_tier(
    speaker: SpeakerCode,
    content: Vec<UtteranceContent>,
    terminator: Terminator,
) -> MainTier {
    MainTier::new(MainTierLocations::UNLOCATED, speaker, content, terminator)
}

/// `.` with no source position.
pub(crate) fn period() -> Terminator {
    Terminator::Period {
        location: Location::Unlocated,
    }
}

/// `?` with no source position.
pub(crate) fn question() -> Terminator {
    Terminator::Question {
        location: Location::Unlocated,
    }
}

/// `!` with no source position.
pub(crate) fn exclamation() -> Terminator {
    Terminator::Exclamation {
        location: Location::Unlocated,
    }
}

/// An authoritative timing bullet with no source position.
pub(crate) fn bullet(start_ms: u64, end_ms: u64) -> Bullet {
    Bullet::new(Location::Unlocated, start_ms, end_ms)
}
