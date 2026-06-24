use talkbank_model::alignment::helpers::{WordItem, walk_words};
use talkbank_model::dependent_tier::TimTier;
use talkbank_model::{
    BracketedItem, BulletContent, BulletContentSegment, ContentAnnotation, DependentTier, GraTier,
    MainTier, MorTier, Utterance, UtteranceContent, WriteChat,
};

use crate::framework::{TierKind, is_countable_word};

/// Render the spoken lexical surface text of a main tier by walking the AST.
///
/// This intentionally ignores CHAT syntax wrappers such as terminators, postcodes,
/// bullets, events, pauses, and separators. Replaced-word nodes contribute the
/// original spoken word rather than the replacement text.
pub fn spoken_main_text(main: &MainTier) -> String {
    spoken_content_text(&main.content.content)
}

/// Render spoken lexical text from utterance content by walking the AST.
pub fn spoken_content_text(content: &[UtteranceContent]) -> String {
    let mut words = Vec::new();
    walk_words(content, None, &mut |leaf| match leaf {
        WordItem::Word(word) => {
            if is_countable_word(word) {
                words.push(word.raw_text().to_owned());
            }
        }
        WordItem::ReplacedWord(replaced) => {
            if is_countable_word(&replaced.word) {
                words.push(replaced.word.raw_text().to_owned());
            }
        }
        WordItem::Separator(_) => {}
    });
    words.join(" ")
}

/// Count scoped error annotations (`[*]`, `[* code]`) across main-tier content.
pub fn count_main_scoped_errors(content: &[UtteranceContent]) -> u64 {
    count_content_scoped_errors(content)
}

/// Render the payload text of a dependent tier without its `%tag:\t` prefix.
pub fn dependent_tier_content_text(tier: &DependentTier) -> String {
    match tier {
        DependentTier::Mor(t) => t.to_content(),
        DependentTier::Gra(t) => t.to_content(),
        DependentTier::Pho(t) | DependentTier::Mod(t) => t.to_content(),
        DependentTier::Act(t) => bullet_content_text(&t.content),
        DependentTier::Cod(t) => bullet_content_text(&t.content),
        DependentTier::Add(t) => bullet_content_text(&t.content),
        DependentTier::Com(t) => bullet_content_text(&t.content),
        DependentTier::Exp(t) => bullet_content_text(&t.content),
        DependentTier::Gpx(t) => bullet_content_text(&t.content),
        DependentTier::Int(t) => bullet_content_text(&t.content),
        DependentTier::Sit(t) => bullet_content_text(&t.content),
        DependentTier::Spa(t) => bullet_content_text(&t.content),
        DependentTier::Alt(t) => t.as_str().to_owned(),
        DependentTier::Coh(t) => t.as_str().to_owned(),
        DependentTier::Def(t) => t.as_str().to_owned(),
        DependentTier::Eng(t) => t.as_str().to_owned(),
        DependentTier::Err(t) => t.as_str().to_owned(),
        DependentTier::Fac(t) => t.as_str().to_owned(),
        DependentTier::Flo(t) => t.as_str().to_owned(),
        DependentTier::Modsyl(t) => t.to_string(),
        DependentTier::Phosyl(t) => t.to_string(),
        DependentTier::Phoaln(t) => t.to_string(),
        DependentTier::Xphoint(t) => t.to_string(),
        DependentTier::Gls(t) => t.as_str().to_owned(),
        DependentTier::Ort(t) => t.as_str().to_owned(),
        DependentTier::Par(t) => t.as_str().to_owned(),
        DependentTier::Tim(t) => tim_tier_text(t),
        DependentTier::Wor(t) => wor_tier_text(t),
        DependentTier::UserDefined(t) => t.content.as_str().to_owned(),
        DependentTier::Unsupported(t) => t.content.as_str().to_owned(),
        DependentTier::Sin(t) => {
            let mut out = String::new();
            for (i, item) in t.items.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                let _ = item.write_chat(&mut out);
            }
            out
        }
    }
}

/// CLAN `+t%X` token stream (`freq.cpp:914-938`, the `case 't'` `+t%` arm): the
/// whitespace-delimited, CLAN-countable tokens of the named dependent tier on
/// `utterance`, in document order. Returns an empty `Vec` when the tier is
/// absent.
///
/// CLAN selects a dependent tier with `+t%X`, sets `nomain=TRUE`, and counts its
/// raw line split on whitespace, so a clitic `v|go~aux|be` is ONE token (unlike
/// chatter's structural `%mor`, which splits post-clitics into separate items).
/// CLAN's `getword` skips standalone CHAT terminator punctuation, so the bare
/// `.` terminator that `%mor` content carries is dropped (on the reference
/// `%mor` line `... noun|top .`, CLAN counts 15 tokens, not 16); `%gra` content
/// carries no terminator token, so nothing is dropped there.
///
/// This is the reusable dependent-tier token primitive: KWAL and COMBO `+t%X`
/// reuse it, and the byte-identical open-coded form in `rely`/`keymap`/`chains`
/// (`dependent_tier_content_text(...).split_whitespace().filter(|t| !t.is_empty()
/// && *t != ".")`, each behind a `%cod` special-case) should migrate onto it.
pub fn dependent_tier_tokens(utterance: &Utterance, tier: &TierKind) -> Vec<String> {
    let mut tokens = Vec::new();
    for dep in &utterance.dependent_tiers {
        // `DependentTier::kind()` is the generic tier-label accessor; compare it
        // to the requested tier's wire label (`%gra` -> "gra", `%grt` aliases to
        // "gra" via TierKind, matching CLAN's case-insensitive tier match).
        if dep.kind() == tier.as_str() {
            tokens.extend(
                dependent_tier_content_text(dep)
                    .split_whitespace()
                    .filter(|token| !token.is_empty() && *token != ".")
                    .map(str::to_owned),
            );
        }
    }
    tokens
}

/// Serialize `%mor` items as token strings, preserving per-item
/// boundaries. The output always includes one trailing terminator
/// element.
pub fn mor_item_texts(tier: &MorTier) -> Vec<String> {
    let mut out = Vec::with_capacity(tier.items().len() + 1);
    for item in tier.items() {
        out.push(WriteChat::to_chat_string(item));
    }
    out.push(tier.terminator.to_string());
    out
}

/// Serialize `%gra` relations as token strings, preserving relation boundaries.
pub fn gra_relation_texts(tier: &GraTier) -> Vec<String> {
    tier.relations()
        .iter()
        .map(WriteChat::to_chat_string)
        .collect()
}

/// Count morphemes in one typed `%mor` item, including post-clitics.
///
/// This counts EVERY feature as a bound morpheme (1 + `features.len()`). It is
/// the maximal/feature-level count used by SUGAR and EVAL; it is NOT the
/// MLU-style §7.21 traced count (see [`mor_item_traced_morpheme_count`]). The
/// two are deliberately different morpheme notions.
pub fn mor_item_morpheme_count(item: &talkbank_model::dependent_tier::mor::Mor) -> u64 {
    mor_word_morpheme_count(&item.main)
        + item
            .post_clitics
            .iter()
            .map(mor_word_morpheme_count)
            .sum::<u64>()
}

/// Feature names that count as a §7.21-traced bound morpheme. The CLAN manual
/// (CLAN.html section 7.21, point 6) traces, for English, "Plural, Past,
/// Possessive, Plural-Possessive, Present Participle, clitic negative, and
/// clitic auxiliary". CLAN's `countMorphs` (OSX-CLAN/src/clan/cutt.cpp:10752)
/// matches both the UD name (`-Plur`) and the legacy CLAN-mor name (`-PL`);
/// chatter operates on UD `%mor`, so the UD names are primary and the legacy
/// all-caps names are retained for any legacy-tagged input. A word matching ANY
/// of these adds at most +1, matching `countMorphs`.
///
/// **`Ger` (UD present participle) is a DELIBERATE DIVERGENCE from CLAN**
/// (CLAN-DIV, the `-Ger` gap): CLAN lists the legacy `-PRESP` but never added the
/// UD spelling `-Ger`, so the binary under-counts UD present participles
/// (`verb|go-Ger` "going" counts 1, not 2). chatter adds `-Ger` here, the
/// like-for-like UD completion of a morpheme section 7.21 already traces (exactly
/// as `-Plur` joined `-PL`). The other §7.21-relevant UD labels our corpus uses
/// (`-Gen` possessive, `-Part`/`-Prog`) are deliberately NOT added: their §7.21
/// mapping needs grounding and is deferred to a separate audit, not inferred here.
pub const TRACED_MORPHEME_SUFFIXES: &[&str] = &[
    "Plur", "PL", "PAST", "Past", "POSS", "PASTP", "Pastp", "PRESP", "Ger",
];

/// Whether a `%mor` word carries any §7.21-traced bound-morpheme feature.
fn mor_word_has_traced_morpheme(word: &talkbank_model::MorWord) -> bool {
    word.features
        .iter()
        .any(|f| TRACED_MORPHEME_SUFFIXES.contains(&f.value()))
}

/// Count §7.21-traced morphemes in one `%mor` item: 1 for the stem plus at most
/// +1 for a traced bound morpheme, and the same for each post-clitic. This is
/// the MLU morpheme count, and the unit FREQ `+x…m` filters on. Distinct from
/// [`mor_item_morpheme_count`], which counts every feature.
pub fn mor_item_traced_morpheme_count(item: &talkbank_model::dependent_tier::mor::Mor) -> u64 {
    let main = 1 + u64::from(mor_word_has_traced_morpheme(&item.main));
    let clitics: u64 = item
        .post_clitics
        .iter()
        .map(|c| 1 + u64::from(mor_word_has_traced_morpheme(c)))
        .sum();
    main + clitics
}

/// Sum the §7.21-traced morphemes across an utterance's `%mor` tier. Returns
/// `None` when the utterance has no `%mor` tier (CLAN counts such an utterance as
/// zero morphemes rather than falling back to main-tier word counting).
pub fn count_traced_morphemes_in_utterance(utterance: &Utterance) -> Option<u64> {
    let mor_tier = utterance.mor_tier()?;
    Some(
        mor_tier
            .items()
            .iter()
            .map(mor_item_traced_morpheme_count)
            .sum(),
    )
}

/// Return the main POS tag string for each `%mor` item.
pub fn mor_item_pos_tags(tier: &MorTier) -> Vec<String> {
    tier.items()
        .iter()
        .map(|item| item.main.pos.to_string())
        .collect()
}

/// Return whether a `%mor` item contains any verb-like chunk.
pub fn mor_item_has_verb(
    item: &talkbank_model::dependent_tier::mor::Mor,
    is_verb_pos: impl Fn(&str) -> bool,
) -> bool {
    is_verb_pos(item.main.pos.as_ref())
        || item
            .post_clitics
            .iter()
            .any(|clitic| is_verb_pos(clitic.pos.as_ref()))
}

fn bullet_content_text(content: &BulletContent) -> String {
    let mut out = String::new();
    for segment in &content.segments {
        match segment {
            BulletContentSegment::Text(text) => out.push_str(&text.text),
            BulletContentSegment::Continuation => out.push_str("\n\t"),
            BulletContentSegment::Bullet(_) | BulletContentSegment::Picture(_) => {}
        }
    }
    out
}

fn mor_word_morpheme_count(word: &talkbank_model::dependent_tier::mor::word::MorWord) -> u64 {
    1 + word.features.len() as u64
}

fn count_content_scoped_errors(content: &[UtteranceContent]) -> u64 {
    let mut total = 0u64;
    for item in content {
        match item {
            UtteranceContent::AnnotatedWord(annotated) => {
                total += count_scoped_errors(&annotated.scoped_annotations.0);
            }
            UtteranceContent::ReplacedWord(replaced) => {
                total += count_scoped_errors(&replaced.scoped_annotations.0);
            }
            UtteranceContent::AnnotatedEvent(annotated) => {
                total += count_scoped_errors(&annotated.scoped_annotations.0);
            }
            UtteranceContent::AnnotatedGroup(annotated) => {
                total += count_scoped_errors(&annotated.scoped_annotations.0);
                total += count_bracketed_scoped_errors(&annotated.inner.content.content);
            }
            UtteranceContent::PhoGroup(group) => {
                total += count_bracketed_scoped_errors(&group.content.content);
            }
            UtteranceContent::SinGroup(group) => {
                total += count_bracketed_scoped_errors(&group.content.content);
            }
            UtteranceContent::Quotation(quotation) => {
                total += count_bracketed_scoped_errors(&quotation.content.content);
            }
            UtteranceContent::AnnotatedAction(annotated) => {
                total += count_scoped_errors(&annotated.scoped_annotations.0);
            }
            UtteranceContent::Retrace(retrace) => {
                total += count_bracketed_scoped_errors(&retrace.content.content);
            }
            UtteranceContent::Word(_)
            | UtteranceContent::Event(_)
            | UtteranceContent::Pause(_)
            | UtteranceContent::Group(_)
            | UtteranceContent::Freecode(_)
            | UtteranceContent::Separator(_)
            | UtteranceContent::OverlapPoint(_)
            | UtteranceContent::InternalBullet(_)
            | UtteranceContent::LongFeatureBegin(_)
            | UtteranceContent::LongFeatureEnd(_)
            | UtteranceContent::UnderlineBegin(_)
            | UtteranceContent::UnderlineEnd(_)
            | UtteranceContent::NonvocalBegin(_)
            | UtteranceContent::NonvocalEnd(_)
            | UtteranceContent::NonvocalSimple(_)
            | UtteranceContent::OtherSpokenEvent(_) => {}
        }
    }
    total
}

fn count_bracketed_scoped_errors(items: &[BracketedItem]) -> u64 {
    let mut total = 0u64;
    for item in items {
        match item {
            BracketedItem::AnnotatedWord(annotated) => {
                total += count_scoped_errors(&annotated.scoped_annotations.0);
            }
            BracketedItem::ReplacedWord(replaced) => {
                total += count_scoped_errors(&replaced.scoped_annotations.0);
            }
            BracketedItem::AnnotatedEvent(annotated) => {
                total += count_scoped_errors(&annotated.scoped_annotations.0);
            }
            BracketedItem::AnnotatedAction(annotated) => {
                total += count_scoped_errors(&annotated.scoped_annotations.0);
            }
            BracketedItem::AnnotatedGroup(annotated) => {
                total += count_scoped_errors(&annotated.scoped_annotations.0);
                total += count_bracketed_scoped_errors(&annotated.inner.content.content);
            }
            BracketedItem::Retrace(retrace) => {
                total += count_bracketed_scoped_errors(&retrace.content.content);
            }
            BracketedItem::PhoGroup(group) => {
                total += count_bracketed_scoped_errors(&group.content.content);
            }
            BracketedItem::SinGroup(group) => {
                total += count_bracketed_scoped_errors(&group.content.content);
            }
            BracketedItem::Quotation(quotation) => {
                total += count_bracketed_scoped_errors(&quotation.content.content);
            }
            BracketedItem::Word(_)
            | BracketedItem::Event(_)
            | BracketedItem::Pause(_)
            | BracketedItem::Action(_)
            | BracketedItem::OverlapPoint(_)
            | BracketedItem::Separator(_)
            | BracketedItem::InternalBullet(_)
            | BracketedItem::Freecode(_)
            | BracketedItem::LongFeatureBegin(_)
            | BracketedItem::LongFeatureEnd(_)
            | BracketedItem::UnderlineBegin(_)
            | BracketedItem::UnderlineEnd(_)
            | BracketedItem::NonvocalBegin(_)
            | BracketedItem::NonvocalEnd(_)
            | BracketedItem::NonvocalSimple(_)
            | BracketedItem::OtherSpokenEvent(_) => {}
        }
    }
    total
}

fn count_scoped_errors(annotations: &[ContentAnnotation]) -> u64 {
    annotations
        .iter()
        .filter(|annotation| matches!(annotation, ContentAnnotation::Error(_)))
        .count() as u64
}

fn tim_tier_text(tier: &TimTier) -> String {
    tier.as_str().to_owned()
}

fn wor_tier_text(tier: &talkbank_model::dependent_tier::WorTier) -> String {
    let mut out = String::new();
    if let Some(language_code) = &tier.language_code {
        out.push_str("[- ");
        out.push_str(language_code.as_str());
        out.push_str("] ");
    }
    for (i, item) in tier.items.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        match item {
            talkbank_model::dependent_tier::WorItem::Word(word) => {
                out.push_str(word.cleaned_text());
                if let Some(bullet) = &word.inline_bullet {
                    out.push(' ');
                    let _ = bullet.write_chat(&mut out);
                }
            }
            talkbank_model::dependent_tier::WorItem::Separator { text, .. } => out.push_str(text),
        }
    }
    if let Some(terminator) = &tier.terminator {
        if !tier.items.is_empty() || tier.language_code.is_some() {
            out.push(' ');
        }
        let _ = terminator.write_chat(&mut out);
    }
    if let Some(bullet) = &tier.bullet {
        out.push(' ');
        let _ = bullet.write_chat(&mut out);
    }
    out
}
