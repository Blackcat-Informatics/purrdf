// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! BCP 47 / RFC 5646 language-tag **well-formedness** (the `Language-Tag`
//! production of [RFC 5646 §2.1]).
//!
//! This is the first-party replacement for the `oxilangtag` dependency: a
//! zero-dependency, wasm-clean checker for the exact ABNF, written from the
//! specification. It decides **well-formedness only** — the purely syntactic
//! judgement of §2.2.9. Registry-based *validity* (subtags checked against the
//! IANA registry) and RFC 4647 *matching* are deliberately not here; each
//! lands together with its first consumer, so no unexercised surface ships.
//!
//! The accepted language is intentionally identical to the predecessor's so
//! that replacing the dependency changes **no acceptance decision** anywhere
//! in the workspace. Two properties of that language deserve calling out:
//!
//! * A tag with a **duplicate extension singleton** (`ar-a-aaa-b-bbb-a-ccc`)
//!   is *well-formed* but not *valid* (RFC 5646 §2.2.9 draws exactly this
//!   line). It is accepted here.
//! * The 26 **grandfathered** tags of §2.2.8 are matched case-insensitively
//!   as whole units.
//!
//! Per the repo `no-optionality / hard-fail` doctrine every rejection is a
//! typed [`LanguageTagError`] naming *why* the string was refused, and every
//! failure carries a stable [`LanguageTagError::diagnostic_code`]. This module
//! is the single owner of the `langtag-*` code family.
//!
//! [RFC 5646 §2.1]: https://www.rfc-editor.org/rfc/rfc5646#section-2.1

use core::fmt;

/// The 26 grandfathered tags of RFC 5646 §2.2.8 (17 irregular + 9 regular),
/// matched case-insensitively as complete tags. The list is closed by the
/// specification: "no new grandfathered tags will be created".
const GRANDFATHERED: [&str; 26] = [
    "art-lojban",
    "cel-gaulish",
    "en-GB-oed",
    "i-ami",
    "i-bnn",
    "i-default",
    "i-enochian",
    "i-hak",
    "i-klingon",
    "i-lux",
    "i-mingo",
    "i-navajo",
    "i-pwn",
    "i-tao",
    "i-tay",
    "i-tsu",
    "no-bok",
    "no-nyn",
    "sgn-BE-FR",
    "sgn-BE-NL",
    "sgn-CH-DE",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
];

/// Which of the three top-level `Language-Tag` alternatives matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagForm {
    /// The ordinary `langtag` production (`language ["-" script] ...`).
    Langtag,
    /// A whole-tag `privateuse` production (`"x" 1*("-" (1*8alphanum))`).
    PrivateUse,
    /// One of the 26 closed `grandfathered` tags, matched as a unit.
    Grandfathered,
}

/// Why a string failed the `Language-Tag` production.
///
/// The variants are deliberately specific so callers (and fixtures) can assert
/// *why* a tag was rejected, not merely that it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LanguageTagError {
    /// Two adjacent hyphens, a leading/trailing hyphen, or an empty input
    /// produced a zero-length subtag.
    EmptySubtag,
    /// A subtag exceeds the 8-character ceiling every alternative shares.
    SubtagTooLong,
    /// The primary language subtag is not 2–8 ASCII letters.
    InvalidLanguage,
    /// A subtag fits no production admissible in its position.
    InvalidSubtag,
    /// More than three `extlang` subtags (`extlang = 3ALPHA *2("-" 3ALPHA)`).
    TooManyExtlangs,
    /// An extension singleton with no following subtag (`en-a`).
    EmptyExtension,
    /// A `x`/`X` singleton with no following private-use subtag (`en-x`, `x-`).
    EmptyPrivateUse,
}

impl LanguageTagError {
    /// The stable machine-readable code for this failure.
    ///
    /// This module is the single owner of the `langtag-*` family; the strings
    /// are a contract and never change meaning.
    #[must_use]
    pub const fn diagnostic_code(self) -> &'static str {
        match self {
            Self::EmptySubtag => "langtag-empty-subtag",
            Self::SubtagTooLong => "langtag-subtag-too-long",
            Self::InvalidLanguage => "langtag-invalid-language",
            Self::InvalidSubtag => "langtag-invalid-subtag",
            Self::TooManyExtlangs => "langtag-too-many-extlangs",
            Self::EmptyExtension => "langtag-empty-extension",
            Self::EmptyPrivateUse => "langtag-empty-private-use",
        }
    }
}

impl fmt::Display for LanguageTagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptySubtag => "empty subtag in language tag",
            Self::SubtagTooLong => "language-tag subtag longer than 8 characters",
            Self::InvalidLanguage => "primary language subtag is not 2-8 ASCII letters",
            Self::InvalidSubtag => "subtag fits no RFC 5646 production at its position",
            Self::TooManyExtlangs => "more than three extended-language subtags",
            Self::EmptyExtension => "extension singleton with no following subtag",
            Self::EmptyPrivateUse => "private-use marker with no following subtag",
        };
        f.write_str(message)
    }
}

impl core::error::Error for LanguageTagError {}

/// A well-formed RFC 5646 language tag, borrowing the input verbatim.
///
/// The tag text is never re-encoded or case-normalized: RDF keeps language
/// tags lexical-verbatim, and case formatting (§2.1.1) is a *presentation*
/// convention, not part of well-formedness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageTag<'a> {
    tag: &'a str,
    form: TagForm,
    language_end: usize,
    extlang_end: usize,
    script_end: usize,
    region_end: usize,
    variant_end: usize,
    extension_end: usize,
    private_use_start: Option<usize>,
}

impl<'a> LanguageTag<'a> {
    /// The tag exactly as supplied.
    #[must_use]
    pub const fn as_str(&self) -> &'a str {
        self.tag
    }

    /// Which top-level alternative matched.
    #[must_use]
    pub const fn form(&self) -> TagForm {
        self.form
    }

    /// The primary language subtag (`langtag` form only; a grandfathered or
    /// whole-tag private-use tag has no decomposable components).
    #[must_use]
    pub fn primary_language(&self) -> Option<&'a str> {
        matches!(self.form, TagForm::Langtag).then(|| &self.tag[..self.language_end])
    }

    /// The extended-language subtags, hyphen-joined, when present.
    #[must_use]
    pub fn extended_language(&self) -> Option<&'a str> {
        self.component(self.language_end, self.extlang_end)
    }

    /// The script subtag, when present.
    #[must_use]
    pub fn script(&self) -> Option<&'a str> {
        self.component(self.extlang_end, self.script_end)
    }

    /// The region subtag, when present.
    #[must_use]
    pub fn region(&self) -> Option<&'a str> {
        self.component(self.script_end, self.region_end)
    }

    /// The variant subtags in order of appearance.
    pub fn variants(&self) -> impl Iterator<Item = &'a str> {
        self.component(self.region_end, self.variant_end)
            .into_iter()
            .flat_map(|section| section.split('-'))
    }

    /// The raw extension section (`a-myext-b-another`), hyphen-joined, when
    /// present. Singleton grouping is the caller's concern until an extension
    /// consumer exists.
    #[must_use]
    pub fn extensions(&self) -> Option<&'a str> {
        self.component(self.variant_end, self.extension_end)
    }

    /// The private-use section including its `x`/`X` marker (`x-phonebk`), or
    /// the whole tag for the whole-tag private-use form.
    #[must_use]
    pub fn private_use(&self) -> Option<&'a str> {
        self.private_use_start.map(|start| &self.tag[start..])
    }

    fn component(&self, before: usize, end: usize) -> Option<&'a str> {
        (matches!(self.form, TagForm::Langtag) && end > before).then(|| &self.tag[before + 1..end])
    }
}

impl fmt::Display for LanguageTag<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag)
    }
}

/// `true` when `tag` matches the RFC 5646 `Language-Tag` production.
#[must_use]
pub fn is_well_formed(tag: &str) -> bool {
    parse(tag).is_ok()
}

/// Parses `tag` against the RFC 5646 `Language-Tag` production.
///
/// # Errors
///
/// A typed [`LanguageTagError`] naming the first production the input failed;
/// never a degraded fallback.
pub fn parse(tag: &str) -> Result<LanguageTag<'_>, LanguageTagError> {
    if GRANDFATHERED
        .iter()
        .any(|entry| entry.eq_ignore_ascii_case(tag))
    {
        return Ok(LanguageTag {
            tag,
            form: TagForm::Grandfathered,
            language_end: 0,
            extlang_end: 0,
            script_end: 0,
            region_end: 0,
            variant_end: 0,
            extension_end: 0,
            private_use_start: None,
        });
    }
    if let Some(rest) = strip_private_use_marker(tag) {
        parse_private_use_subtags(rest)?;
        return Ok(LanguageTag {
            tag,
            form: TagForm::PrivateUse,
            language_end: 0,
            extlang_end: 0,
            script_end: 0,
            region_end: 0,
            variant_end: 0,
            extension_end: 0,
            private_use_start: Some(0),
        });
    }
    parse_langtag(tag)
}

/// Strips a whole-tag `privateuse` marker: `x-...` / `X-...`.
fn strip_private_use_marker(tag: &str) -> Option<&str> {
    let mut bytes = tag.bytes();
    (matches!(bytes.next(), Some(b'x' | b'X')) && bytes.next() == Some(b'-')).then(|| &tag[2..])
}

/// `privateuse = "x" 1*("-" (1*8alphanum))` — the part after `x-`.
fn parse_private_use_subtags(rest: &str) -> Result<(), LanguageTagError> {
    if rest.is_empty() {
        return Err(LanguageTagError::EmptyPrivateUse);
    }
    for subtag in rest.split('-') {
        if subtag.is_empty() {
            return Err(LanguageTagError::EmptySubtag);
        }
        if subtag.len() > 8 {
            return Err(LanguageTagError::SubtagTooLong);
        }
        if !is_alphanumeric(subtag) {
            return Err(LanguageTagError::InvalidSubtag);
        }
    }
    Ok(())
}

/// Position in the `langtag` production. Each state names what the *previous*
/// subtag established; the admissible productions for the next subtag follow
/// RFC 5646's ordering (`language ["-" script] ["-" region] *("-" variant)
/// *("-" extension) ["-" privateuse]`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Start,
    /// After a 2–3 letter primary language: `extlang` is still admissible.
    AfterShortLanguage,
    /// After a 4+ letter primary language or an `extlang` run: script next.
    AfterLanguage,
    AfterScript,
    AfterRegion,
    /// Inside an extension; `expects_subtag` is `true` until the singleton has
    /// at least one 2–8 alphanumeric subtag.
    InExtension {
        expects_subtag: bool,
    },
    /// Inside the trailing private-use section; `expects_subtag` is `true`
    /// until the `x` marker has at least one subtag.
    InPrivateUse {
        expects_subtag: bool,
    },
}

/// The ordinary `langtag` alternative, walked subtag-by-subtag with byte
/// offsets so the component accessors can slice the original text.
#[allow(clippy::too_many_lines)] // one production per arm; splitting would scatter the grammar
fn parse_langtag(tag: &str) -> Result<LanguageTag<'_>, LanguageTagError> {
    let mut position = Position::Start;
    let mut language_end = 0;
    let mut extlang_end = 0;
    let mut script_end = 0;
    let mut region_end = 0;
    let mut variant_end = 0;
    let mut extension_end = 0;
    let mut private_use_start = None;
    let mut extlang_count = 0u8;

    let mut offset = 0usize;
    for subtag in tag.split('-') {
        let end = offset + subtag.len();
        if subtag.is_empty() {
            return Err(LanguageTagError::EmptySubtag);
        }
        if subtag.len() > 8 {
            return Err(LanguageTagError::SubtagTooLong);
        }
        position = match position {
            Position::Start => {
                // language = 2*3ALPHA / 4ALPHA / 5*8ALPHA (the union is any
                // 2-8 letter run; which arm matched decides extlang admission).
                if subtag.len() < 2 || !is_alphabetic(subtag) {
                    return Err(LanguageTagError::InvalidLanguage);
                }
                language_end = end;
                if subtag.len() < 4 {
                    Position::AfterShortLanguage
                } else {
                    Position::AfterLanguage
                }
            }
            Position::InPrivateUse { .. } => {
                // privateuse subtags: 1*8alphanum, no further sections.
                if !is_alphanumeric(subtag) {
                    return Err(LanguageTagError::InvalidSubtag);
                }
                Position::InPrivateUse {
                    expects_subtag: false,
                }
            }
            _ if matches!(subtag, "x" | "X") => {
                if position
                    == (Position::InExtension {
                        expects_subtag: true,
                    })
                {
                    return Err(LanguageTagError::EmptyExtension);
                }
                private_use_start = Some(offset);
                Position::InPrivateUse {
                    expects_subtag: true,
                }
            }
            _ if subtag.len() == 1 && is_alphanumeric(subtag) => {
                // singleton = alphanum except x/X (handled above).
                if position
                    == (Position::InExtension {
                        expects_subtag: true,
                    })
                {
                    return Err(LanguageTagError::EmptyExtension);
                }
                Position::InExtension {
                    expects_subtag: true,
                }
            }
            Position::InExtension { .. } => {
                // extension subtags: 2*8alphanum (a 1-char subtag was taken by
                // the singleton arm above).
                if !is_alphanumeric(subtag) {
                    return Err(LanguageTagError::InvalidSubtag);
                }
                extension_end = end;
                Position::InExtension {
                    expects_subtag: false,
                }
            }
            Position::AfterShortLanguage if subtag.len() == 3 && is_alphabetic(subtag) => {
                // extlang = 3ALPHA *2("-" 3ALPHA): at most three segments.
                extlang_count += 1;
                if extlang_count > 3 {
                    return Err(LanguageTagError::TooManyExtlangs);
                }
                extlang_end = end;
                Position::AfterShortLanguage
            }
            Position::AfterShortLanguage | Position::AfterLanguage
                if subtag.len() == 4 && is_alphabetic(subtag) =>
            {
                // script = 4ALPHA
                script_end = end;
                Position::AfterScript
            }
            Position::AfterShortLanguage | Position::AfterLanguage | Position::AfterScript
                if subtag.len() == 2 && is_alphabetic(subtag)
                    || subtag.len() == 3 && is_numeric(subtag) =>
            {
                // region = 2ALPHA / 3DIGIT
                region_end = end;
                Position::AfterRegion
            }
            Position::AfterShortLanguage
            | Position::AfterLanguage
            | Position::AfterScript
            | Position::AfterRegion
                if is_alphanumeric(subtag)
                    && (subtag.len() >= 5 && subtag.as_bytes()[0].is_ascii_alphabetic()
                        || subtag.len() >= 4 && subtag.as_bytes()[0].is_ascii_digit()) =>
            {
                // variant = 5*8alphanum / (DIGIT 3alphanum)
                variant_end = end;
                Position::AfterRegion
            }
            Position::AfterShortLanguage
            | Position::AfterLanguage
            | Position::AfterScript
            | Position::AfterRegion => return Err(LanguageTagError::InvalidSubtag),
        };
        offset = end + 1;
    }

    match position {
        Position::InExtension {
            expects_subtag: true,
        } => return Err(LanguageTagError::EmptyExtension),
        Position::InPrivateUse {
            expects_subtag: true,
        } => return Err(LanguageTagError::EmptyPrivateUse),
        _ => {}
    }

    // Cascade the section ends so every accessor slices a well-defined range
    // even when intermediate sections are absent.
    extlang_end = extlang_end.max(language_end);
    script_end = script_end.max(extlang_end);
    region_end = region_end.max(script_end);
    variant_end = variant_end.max(region_end);
    extension_end = extension_end.max(variant_end);

    Ok(LanguageTag {
        tag,
        form: TagForm::Langtag,
        language_end,
        extlang_end,
        script_end,
        region_end,
        variant_end,
        extension_end,
        private_use_start,
    })
}

fn is_alphabetic(subtag: &str) -> bool {
    subtag.bytes().all(|byte| byte.is_ascii_alphabetic())
}

fn is_numeric(subtag: &str) -> bool {
    subtag.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_alphanumeric(subtag: &str) -> bool {
    subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::{LanguageTagError, TagForm, is_well_formed, parse};

    #[test]
    fn components_decompose() {
        let tag = parse("zh-yue-Hant-HK-1901-a-myext-x-private-use").expect("well-formed");
        assert_eq!(tag.form(), TagForm::Langtag);
        assert_eq!(tag.primary_language(), Some("zh"));
        assert_eq!(tag.extended_language(), Some("yue"));
        assert_eq!(tag.script(), Some("Hant"));
        assert_eq!(tag.region(), Some("HK"));
        assert_eq!(tag.variants().collect::<Vec<_>>(), ["1901"]);
        assert_eq!(tag.extensions(), Some("a-myext"));
        assert_eq!(tag.private_use(), Some("x-private-use"));
        assert_eq!(tag.as_str(), "zh-yue-Hant-HK-1901-a-myext-x-private-use");
    }

    #[test]
    fn absent_components_are_none() {
        let tag = parse("de").expect("well-formed");
        assert_eq!(tag.primary_language(), Some("de"));
        assert_eq!(tag.extended_language(), None);
        assert_eq!(tag.script(), None);
        assert_eq!(tag.region(), None);
        assert_eq!(tag.variants().count(), 0);
        assert_eq!(tag.extensions(), None);
        assert_eq!(tag.private_use(), None);
    }

    #[test]
    fn whole_tag_private_use_and_grandfathered_forms() {
        let private = parse("x-whatever").expect("well-formed");
        assert_eq!(private.form(), TagForm::PrivateUse);
        assert_eq!(private.private_use(), Some("x-whatever"));
        assert_eq!(private.primary_language(), None);

        let grandfathered = parse("i-Enochian").expect("well-formed");
        assert_eq!(grandfathered.form(), TagForm::Grandfathered);
        assert_eq!(grandfathered.primary_language(), None);
        assert_eq!(grandfathered.as_str(), "i-Enochian");
    }

    #[test]
    fn typed_refusals() {
        assert_eq!(parse(""), Err(LanguageTagError::EmptySubtag));
        assert_eq!(parse("en-"), Err(LanguageTagError::EmptySubtag));
        assert_eq!(parse("-en"), Err(LanguageTagError::EmptySubtag));
        assert_eq!(parse("e"), Err(LanguageTagError::InvalidLanguage));
        assert_eq!(parse("a1"), Err(LanguageTagError::InvalidLanguage));
        assert_eq!(
            parse("abcdefghi"),
            Err(LanguageTagError::SubtagTooLong),
            "nine-letter primary subtag"
        );
        assert_eq!(parse("en-a"), Err(LanguageTagError::EmptyExtension));
        assert_eq!(parse("en-a-b-cc"), Err(LanguageTagError::EmptyExtension));
        assert_eq!(parse("en-x"), Err(LanguageTagError::EmptyPrivateUse));
        assert_eq!(parse("x-"), Err(LanguageTagError::EmptyPrivateUse));
        assert_eq!(
            parse("ab-abc-abc-abc-abc"),
            Err(LanguageTagError::TooManyExtlangs)
        );
        assert_eq!(parse("de-419-DE"), Err(LanguageTagError::InvalidSubtag));
        assert_eq!(parse("en-ü"), Err(LanguageTagError::InvalidSubtag));
    }

    #[test]
    fn refused_inputs_have_accepted_neighbors() {
        // Refusal discipline: every refusal above sits beside an accepted
        // neighbor, so a tightened production cannot silently over-refuse.
        for (refused, accepted) in [
            ("e", "en"),
            ("en-", "en"),
            ("en-a", "en-a-bb"),
            ("en-x", "en-x-a"),
            ("x-", "x-a"),
            ("abcdefghi", "abcdefgh"),
            ("ab-abc-abc-abc-abc", "ab-abc-abc-abc"),
            ("de-419-DE", "de-DE"),
        ] {
            assert!(parse(refused).is_err(), "{refused:?} must refuse");
            assert!(is_well_formed(accepted), "{accepted:?} must accept");
        }
    }

    #[test]
    fn diagnostic_codes_are_stable() {
        assert_eq!(
            LanguageTagError::EmptySubtag.diagnostic_code(),
            "langtag-empty-subtag"
        );
        assert_eq!(
            LanguageTagError::EmptyPrivateUse.diagnostic_code(),
            "langtag-empty-private-use"
        );
    }
}
