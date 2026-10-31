//! Shared fixtures for unit tests: unlocated, programmatically built CHAT nodes.

#![allow(clippy::expect_used)]

use talkbank_model::{
    Location, MainTier, MainTierLocations, Terminator, UtteranceContent, Word, WordText,
};

/// A plain word; the text must be non-empty.
pub(crate) fn word(text: &str) -> Word {
    Word::new(
        Location::Unlocated,
        WordText::new(text).expect("test word text is non-empty"),
    )
}

/// An unlocated period terminator.
pub(crate) fn period() -> Terminator {
    Terminator::Period {
        location: Location::Unlocated,
    }
}

/// A main tier ending in a period.
pub(crate) fn main_tier(speaker: &str, content: Vec<UtteranceContent>) -> MainTier {
    MainTier::new(MainTierLocations::UNLOCATED, speaker, content, period())
}

/// A `%cod` tier from plain text.
pub(crate) fn cod_tier(text: &str) -> talkbank_model::CodTier {
    talkbank_model::CodTier::from_text(Location::Unlocated, text)
}

/// A `%gra` relation `index|head|label`.
pub(crate) fn rel(index: usize, head: usize, label: &str) -> talkbank_model::GrammaticalRelation {
    talkbank_model::GrammaticalRelation::new(
        index,
        head,
        talkbank_model::GrammaticalRelationType::new(label).expect("test relation label is valid"),
    )
}

/// An unlocated media bullet.
pub(crate) fn bullet(start_ms: u64, end_ms: u64) -> talkbank_model::Bullet {
    talkbank_model::Bullet::new(Location::Unlocated, start_ms, end_ms)
}

/// An `@ID` header with an ISO 639-3 language code.
pub(crate) fn id_header(language: &str, speaker: &str, role: &str) -> talkbank_model::IDHeader {
    talkbank_model::IDHeader::new(
        talkbank_model::LanguageCode::new(language).expect("test language code is valid"),
        speaker,
        role,
    )
}
