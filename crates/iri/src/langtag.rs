// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! BCP 47 / RFC 5646 language-tag **well-formedness** (the `Language-Tag`
//! production of [RFC 5646 §2.1]).
//!
//! # Scope
//!
//! [`parse`] decides **well-formedness** and nothing else: the purely syntactic
//! judgement of RFC 5646 §2.2.9, taken against the §2.1 ABNF and the closed
//! §2.2.8 grandfathered list. It does **not** decide *validity* — whether each
//! subtag is registered in the IANA Language Subtag Registry — and it is not an
//! RFC 4647 language-range matcher. Neither of those is available anywhere in
//! this crate; a caller that needs them must obtain them elsewhere.
//!
//! Two consequences of where §2.2.9 draws that line are worth stating outright,
//! because both look like bugs to a reader who expects validity:
//!
//! * A tag with a **duplicate extension singleton** (`ar-a-aaa-b-bbb-a-ccc`) is
//!   well-formed and *not* valid. It is accepted here.
//! * Subtags are never looked up, so `qq-Zzzz-QQ` is well-formed even though no
//!   such language, script or region is registered.
//!
//! Case is insignificant to the judgement, and the input is never re-encoded or
//! case-normalized: RDF keeps language tags lexical-verbatim, and the §2.1.1
//! case conventions are a *presentation* recommendation, not part of the
//! grammar.
//!
//! # How the grammar is decomposed
//!
//! The parser is recursive descent with one function per line of the `langtag`
//! production, driven by a shared subtag `Cursor`. See [`parse`] for the
//! production-to-function map. There is no parser state variable: the sequence
//! of calls *is* the production, and each optional or repeated section is a
//! `take_if`/`while let` over a predicate that is itself one ABNF alternative.
//! No backtracking is needed, because at every position the admissible
//! productions are disjoint on subtag shape (a 3-letter subtag can only be an
//! `extlang`, a 4-letter one only a `script`, and so on).
//!
//! # Failure
//!
//! Every rejection is a typed [`LanguageTagError`] naming the production that
//! refused, and carries a stable [`LanguageTagError::diagnostic_code`]. This
//! module is the single owner of the `langtag-*` code family.
//!
//! # How the accepted language is pinned
//!
//! `tests/langtag_differential.rs` holds a frozen accept/reject table of 3935
//! candidate tags, generated from the ABNF and labelled by an external oracle,
//! and asserts [`is_well_formed`] against every row. `tests/PROVENANCE.md`
//! records where the inputs and the verdicts each came from. A disagreement
//! there is a defect in this module, never in the table.
//!
//! [RFC 5646 §2.1]: https://www.rfc-editor.org/rfc/rfc5646#section-2.1

use core::fmt;

/// The closed `grandfathered` set of RFC 5646 §2.2.8, in the order the RFC
/// presents it: the seventeen `irregular` tags, then the nine `regular` ones.
///
/// Matched case-insensitively as whole tags. The set is closed by the
/// specification — "no new grandfathered tags will be created" — so this array
/// is complete by construction rather than by maintenance.
const GRANDFATHERED: [&str; 26] = [
    // irregular
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
    "sgn-BE-FR",
    "sgn-BE-NL",
    "sgn-CH-DE",
    // regular
    "art-lojban",
    "cel-gaulish",
    "no-bok",
    "no-nyn",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
];

/// The longest subtag any RFC 5646 production admits.
///
/// Read off the ABNF rather than assumed: `language` tops out at `5*8ALPHA`,
/// `variant` at `5*8alphanum`, an `extension` subtag at `2*8alphanum` and a
/// `privateuse` subtag at `1*8alphanum`; the fixed-width productions
/// (`extlang`, `script`, `region`, `singleton`) are all shorter. Eight is
/// therefore a bound every subtag position shares, which is what makes the
/// lexical pre-pass in [`check_subtag_envelope`] legitimate.
const SUBTAG_CEILING: usize = 8;

/// `extlang = 3ALPHA *2("-" 3ALPHA)` — the repetition is bounded at two, so at
/// most three `extlang` subtags in total.
const EXTLANG_REPEAT_LIMIT: usize = 2;

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
/// One variant per way the decomposition in [`parse`] can refuse: two from the
/// lexical envelope every production shares, four from a named production that
/// could not be satisfied, and one for input left over after the whole
/// production ran. Callers (and fixtures) can therefore assert *which* rule
/// refused, not merely that something did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LanguageTagError {
    /// A subtag position held no characters at all: an empty input, a leading
    /// or trailing `-`, or two adjacent hyphens. Every production requires at
    /// least one character per subtag.
    SubtagLengthZero,
    /// A subtag ran past the eight-character bound shared by every production
    /// (see `SUBTAG_CEILING`).
    SubtagLengthOverEight,
    /// The first subtag satisfies no alternative of
    /// `language = 2*3ALPHA ["-" extlang] / 4ALPHA / 5*8ALPHA` — it is shorter
    /// than two characters, longer than eight, or not entirely ASCII letters.
    LanguageProductionUnmatched,
    /// A fourth `extlang` subtag followed a full one. `extlang` is
    /// `3ALPHA *2("-" 3ALPHA)`, so its repetition stops at three.
    ExtlangRepetitionExceeded,
    /// An `extension` singleton was not followed by a subtag, but
    /// `extension = singleton 1*("-" (2*8alphanum))` requires at least one
    /// (`en-a`, `en-a-b-cc`).
    SingletonWithoutSubtag,
    /// An `x`/`X` marker was not followed by a subtag, but
    /// `privateuse = "x" 1*("-" (1*8alphanum))` requires at least one (`x`,
    /// `en-x`).
    PrivateUseWithoutSubtag,
    /// Subtags remained after every section of the production had its turn, so
    /// the leftover fits nowhere the grammar still allows (`de-419-DE`,
    /// `en-Lat1`, `en-ü`).
    UnconsumedSubtag,
}

impl LanguageTagError {
    /// The stable machine-readable code for this failure.
    ///
    /// This module is the single owner of the `langtag-*` family; the strings
    /// are a contract and never change meaning.
    #[must_use]
    pub const fn diagnostic_code(self) -> &'static str {
        match self {
            Self::SubtagLengthZero => "langtag-subtag-length-zero",
            Self::SubtagLengthOverEight => "langtag-subtag-length-over-eight",
            Self::LanguageProductionUnmatched => "langtag-language-production-unmatched",
            Self::ExtlangRepetitionExceeded => "langtag-extlang-repetition-exceeded",
            Self::SingletonWithoutSubtag => "langtag-singleton-without-subtag",
            Self::PrivateUseWithoutSubtag => "langtag-private-use-without-subtag",
            Self::UnconsumedSubtag => "langtag-unconsumed-subtag",
        }
    }
}

impl fmt::Display for LanguageTagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::SubtagLengthZero => "language tag has a zero-length subtag",
            Self::SubtagLengthOverEight => "language-tag subtag longer than 8 characters",
            Self::LanguageProductionUnmatched => {
                "first subtag matches no alternative of the RFC 5646 `language` production"
            }
            Self::ExtlangRepetitionExceeded => {
                "more than three `extlang` subtags (the repetition is bounded at two)"
            }
            Self::SingletonWithoutSubtag => "extension singleton with no following subtag",
            Self::PrivateUseWithoutSubtag => "private-use marker with no following subtag",
            Self::UnconsumedSubtag => "subtag left over after the RFC 5646 `langtag` production",
        };
        f.write_str(message)
    }
}

impl core::error::Error for LanguageTagError {}

/// A half-open byte range into the tag a [`LanguageTag`] borrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    /// Byte offset of the first character of the section.
    start: usize,
    /// Byte offset one past its last character.
    end: usize,
}

impl Span {
    /// The section this span covers. Both offsets are subtag boundaries, which
    /// are ASCII `-` positions or the ends of the string, so they always fall
    /// on a character boundary.
    fn of(self, tag: &str) -> &str {
        &tag[self.start..self.end]
    }

    /// How many bytes the span covers.
    const fn len(self) -> usize {
        self.end - self.start
    }
}

/// A well-formed RFC 5646 language tag, borrowing the input verbatim.
///
/// Each section of the `langtag` production is recorded as the byte range it
/// occupied, so every accessor is a slice of the original input: nothing is
/// copied, re-encoded or case-normalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageTag<'a> {
    /// The input, exactly as supplied.
    tag: &'a str,
    /// Which top-level alternative matched.
    form: TagForm,
    /// `language`, minus any `extlang`; `None` for the whole-tag forms.
    language: Option<Span>,
    /// The `extlang` subtags as one hyphen-joined run.
    extlang: Option<Span>,
    /// `script = 4ALPHA`.
    script: Option<Span>,
    /// `region = 2ALPHA / 3DIGIT`.
    region: Option<Span>,
    /// Every `variant` subtag as one hyphen-joined run.
    variants: Option<Span>,
    /// Every `extension` including its singleton, as one hyphen-joined run.
    extensions: Option<Span>,
    /// `privateuse` including its `x`/`X` marker; the whole tag for the
    /// whole-tag form.
    private_use: Option<Span>,
}

impl<'a> LanguageTag<'a> {
    /// A tag that matched a whole-tag alternative (`privateuse` as the entire
    /// input, or `grandfathered`), which has no decomposable sections.
    const fn whole(tag: &'a str, form: TagForm, private_use: Option<Span>) -> Self {
        Self {
            tag,
            form,
            language: None,
            extlang: None,
            script: None,
            region: None,
            variants: None,
            extensions: None,
            private_use,
        }
    }

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
        self.language.map(|span| span.of(self.tag))
    }

    /// The extended-language subtags, hyphen-joined, when present.
    #[must_use]
    pub fn extended_language(&self) -> Option<&'a str> {
        self.extlang.map(|span| span.of(self.tag))
    }

    /// The script subtag, when present.
    #[must_use]
    pub fn script(&self) -> Option<&'a str> {
        self.script.map(|span| span.of(self.tag))
    }

    /// The region subtag, when present.
    #[must_use]
    pub fn region(&self) -> Option<&'a str> {
        self.region.map(|span| span.of(self.tag))
    }

    /// The variant subtags in order of appearance.
    pub fn variants(&self) -> impl Iterator<Item = &'a str> {
        self.variants
            .map(|span| span.of(self.tag))
            .into_iter()
            .flat_map(|section| section.split('-'))
    }

    /// The raw extension section (`a-myext-b-another`), hyphen-joined, when
    /// present. Grouping the subtags under their singletons is the caller's
    /// concern: well-formedness does not interpret extensions.
    #[must_use]
    pub fn extensions(&self) -> Option<&'a str> {
        self.extensions.map(|span| span.of(self.tag))
    }

    /// The private-use section including its `x`/`X` marker (`x-phonebk`), or
    /// the whole tag for the whole-tag private-use form.
    #[must_use]
    pub fn private_use(&self) -> Option<&'a str> {
        self.private_use.map(|span| span.of(self.tag))
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

/// Parses `tag` against `Language-Tag = langtag / privateuse / grandfathered`.
///
/// The three alternatives are tried in the order below, and the `langtag` body
/// is one call per line of the production:
///
/// | ABNF | implemented by |
/// |------|----------------|
/// | `grandfathered` | `is_grandfathered` |
/// | `privateuse` (whole tag) | `take_private_use` |
/// | `language = 2*3ALPHA ["-" extlang] / 4ALPHA / 5*8ALPHA` | `take_language` |
/// | `extlang = 3ALPHA *2("-" 3ALPHA)` | `take_extlang` |
/// | `["-" script]` | `Cursor::take_if(is_script)` |
/// | `["-" region]` | `Cursor::take_if(is_region)` |
/// | `*("-" variant)` | `take_variants` |
/// | `*("-" extension)` | `take_extensions` |
/// | `["-" privateuse]` | `take_private_use` |
///
/// # Errors
///
/// A typed [`LanguageTagError`] naming the production that refused; see that
/// type for the failure map.
pub fn parse(tag: &str) -> Result<LanguageTag<'_>, LanguageTagError> {
    // `grandfathered` first: it is a closed set of literals, most of which the
    // `langtag` production cannot analyse at all (`i-ami` has a one-letter
    // primary language) and some of which it would analyse into *different*
    // components (`art-lojban` would read as a language plus a variant). The
    // set is disjoint from `privateuse`, so only `langtag` is shadowed, and
    // only for the 26 tags the RFC says are grandfathered.
    if is_grandfathered(tag) {
        return Ok(LanguageTag::whole(tag, TagForm::Grandfathered, None));
    }

    // Every remaining alternative is a hyphen-separated list of 1..=8-character
    // subtags. Checking that envelope once, up front, keeps the two length
    // failures out of the production functions below, which then only ever ask
    // "does this subtag have the shape my production names?".
    check_subtag_envelope(tag)?;

    let mut cursor = Cursor::new(tag);

    // `privateuse` as the whole tag. It is the only alternative that may begin
    // with the reserved `x` singleton, so a leading `x` commits to it: there is
    // no `langtag` reading to fall back to.
    if let Some(span) = take_private_use(&mut cursor)? {
        require_exhausted(&cursor)?;
        return Ok(LanguageTag::whole(tag, TagForm::PrivateUse, Some(span)));
    }

    // `langtag = language ["-" script] ["-" region] *("-" variant)
    //            *("-" extension) ["-" privateuse]`
    let (language, extlang) = take_language(&mut cursor)?;
    let script = cursor.take_if(is_script);
    let region = cursor.take_if(is_region);
    let variants = take_variants(&mut cursor);
    let extensions = take_extensions(&mut cursor)?;
    let private_use = take_private_use(&mut cursor)?;
    require_exhausted(&cursor)?;

    Ok(LanguageTag {
        tag,
        form: TagForm::Langtag,
        language: Some(language),
        extlang,
        script,
        region,
        variants,
        extensions,
        private_use,
    })
}

/// `true` when `tag` is one of the 26 closed `grandfathered` tags of §2.2.8.
///
/// Whole-tag comparison, so `zh-min` and `zh-min-nan` cannot shadow each other,
/// and case-insensitive, because RFC 5234 literals are.
fn is_grandfathered(tag: &str) -> bool {
    GRANDFATHERED
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(tag))
}

/// Enforces the subtag-length envelope every production shares.
///
/// Byte lengths are used rather than character counts. That is exact for the
/// grammar, whose every terminal is ASCII: a subtag that would need the
/// distinction is one containing a multi-byte character, which no production
/// admits anyway.
fn check_subtag_envelope(tag: &str) -> Result<(), LanguageTagError> {
    for subtag in tag.split('-') {
        if subtag.is_empty() {
            return Err(LanguageTagError::SubtagLengthZero);
        }
        if subtag.len() > SUBTAG_CEILING {
            return Err(LanguageTagError::SubtagLengthOverEight);
        }
    }
    Ok(())
}

/// A forward cursor over a tag's hyphen-separated subtags.
///
/// Holds a byte offset rather than an iterator so that each accepted subtag can
/// report the [`Span`] it occupied, which is what lets the accessors on
/// [`LanguageTag`] slice the original input.
#[derive(Debug)]
struct Cursor<'a> {
    /// The tag being walked.
    tag: &'a str,
    /// Byte offset at which the pending subtag starts.
    at: usize,
    /// Set once the final subtag has been consumed. Needed because a cursor
    /// sitting at `tag.len()` is otherwise indistinguishable from one pointing
    /// at a trailing empty subtag.
    exhausted: bool,
}

impl<'a> Cursor<'a> {
    /// A cursor positioned at the first subtag of `tag`.
    const fn new(tag: &'a str) -> Self {
        Self {
            tag,
            at: 0,
            exhausted: false,
        }
    }

    /// The pending subtag and the span it occupies, without consuming it.
    fn peek(&self) -> Option<(Span, &'a str)> {
        if self.exhausted {
            return None;
        }
        let rest: &'a str = &self.tag[self.at..];
        let length = rest.find('-').unwrap_or(rest.len());
        let (text, _) = rest.split_at(length);
        Some((
            Span {
                start: self.at,
                end: self.at + length,
            },
            text,
        ))
    }

    /// Consumes the pending subtag.
    fn bump(&mut self) {
        let rest = &self.tag[self.at..];
        match rest.find('-') {
            Some(separator) => self.at += separator + 1,
            None => {
                self.at = self.tag.len();
                self.exhausted = true;
            }
        }
    }

    /// Consumes the pending subtag when it satisfies `production`, and reports
    /// the span it occupied. This is the whole of an ABNF `[...]` option and
    /// one step of a `*(...)` repetition.
    fn take_if(&mut self, production: impl Fn(&str) -> bool) -> Option<Span> {
        let (span, text) = self.peek()?;
        if !production(text) {
            return None;
        }
        self.bump();
        Some(span)
    }

    /// `true` when the pending subtag satisfies `production`, without
    /// consuming anything.
    fn peek_is(&self, production: impl Fn(&str) -> bool) -> bool {
        self.peek().is_some_and(|(_, text)| production(text))
    }
}

/// Refuses a tag with subtags the production never reached.
fn require_exhausted(cursor: &Cursor<'_>) -> Result<(), LanguageTagError> {
    if cursor.peek().is_some() {
        return Err(LanguageTagError::UnconsumedSubtag);
    }
    Ok(())
}

/// `language = 2*3ALPHA ["-" extlang] / 4ALPHA / 5*8ALPHA`
///
/// Returns the primary language span and the `extlang` run, if any. The three
/// alternatives collapse to "2..=8 ASCII letters" for the primary subtag; what
/// distinguishes them is that only the `2*3ALPHA` one admits a following
/// `extlang`, which is why the length is re-tested before recursing.
fn take_language(cursor: &mut Cursor<'_>) -> Result<(Span, Option<Span>), LanguageTagError> {
    let Some(span) = cursor.take_if(is_language) else {
        return Err(LanguageTagError::LanguageProductionUnmatched);
    };
    if span.len() > 3 {
        // The `4ALPHA` and `5*8ALPHA` alternatives have no `["-" extlang]`.
        return Ok((span, None));
    }
    let extlang = take_extlang(cursor)?;
    Ok((span, extlang))
}

/// `extlang = 3ALPHA *2("-" 3ALPHA)`
fn take_extlang(cursor: &mut Cursor<'_>) -> Result<Option<Span>, LanguageTagError> {
    let Some(first) = cursor.take_if(is_extlang_subtag) else {
        return Ok(None);
    };
    let mut run = first;
    for _ in 0..EXTLANG_REPEAT_LIMIT {
        match cursor.take_if(is_extlang_subtag) {
            Some(repeat) => run.end = repeat.end,
            None => return Ok(Some(run)),
        }
    }
    // The repetition is spent. A further 3-letter subtag can satisfy no later
    // production — `script` is 4ALPHA, `region` is 2ALPHA or 3DIGIT, `variant`
    // is 5*8alphanum or DIGIT 3alphanum, `singleton` and the `privateuse`
    // marker are one character — so it is unambiguously one `extlang` too
    // many, and saying so beats a generic leftover-subtag complaint. Anything
    // that is *not* 3 letters still falls through to the sections below, so
    // `zh-cmn-yue-nan-Hant-CN` is unaffected.
    if cursor.peek_is(is_extlang_subtag) {
        return Err(LanguageTagError::ExtlangRepetitionExceeded);
    }
    Ok(Some(run))
}

/// `*("-" variant)`
fn take_variants(cursor: &mut Cursor<'_>) -> Option<Span> {
    let mut run: Option<Span> = None;
    while let Some(variant) = cursor.take_if(is_variant) {
        run = Some(extend(run, variant));
    }
    run
}

/// `*("-" extension)`, where `extension = singleton 1*("-" (2*8alphanum))`
///
/// The `1*` is the only thing that can fail: a one-character alphanumeric
/// subtag is never an extension subtag, so it ends the inner repetition and is
/// re-offered to the outer one as the next singleton.
fn take_extensions(cursor: &mut Cursor<'_>) -> Result<Option<Span>, LanguageTagError> {
    let mut run: Option<Span> = None;
    while let Some(singleton) = cursor.take_if(is_singleton) {
        let mut extension = singleton;
        let mut subtags = 0usize;
        while let Some(subtag) = cursor.take_if(is_extension_subtag) {
            extension.end = subtag.end;
            subtags += 1;
        }
        if subtags == 0 {
            return Err(LanguageTagError::SingletonWithoutSubtag);
        }
        run = Some(extend(run, extension));
    }
    Ok(run)
}

/// `privateuse = "x" 1*("-" (1*8alphanum))`
///
/// Used twice, because the ABNF uses it twice: as the whole `Language-Tag` and
/// as the trailing option of `langtag`.
fn take_private_use(cursor: &mut Cursor<'_>) -> Result<Option<Span>, LanguageTagError> {
    let Some(marker) = cursor.take_if(is_private_use_marker) else {
        return Ok(None);
    };
    let mut run = marker;
    let mut subtags = 0usize;
    while let Some(subtag) = cursor.take_if(is_private_use_subtag) {
        run.end = subtag.end;
        subtags += 1;
    }
    if subtags == 0 {
        return Err(LanguageTagError::PrivateUseWithoutSubtag);
    }
    Ok(Some(run))
}

/// Grows a hyphen-joined run to cover one more section.
fn extend(run: Option<Span>, next: Span) -> Span {
    Span {
        start: run.map_or(next.start, |run| run.start),
        end: next.end,
    }
}

/// `ALPHA` over a whole subtag. Vacuously true for the empty string, which
/// [`check_subtag_envelope`] has already excluded.
fn all_alpha(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_alphabetic())
}

/// `DIGIT` over a whole subtag.
fn all_digit(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_digit())
}

/// `alphanum` (RFC 5646's `ALPHA / DIGIT`) over a whole subtag.
fn all_alphanum(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

/// The primary subtag of `language = 2*3ALPHA ... / 4ALPHA / 5*8ALPHA`: the
/// union of the three alternatives' first subtags is 2..=8 letters.
fn is_language(text: &str) -> bool {
    (2..=SUBTAG_CEILING).contains(&text.len()) && all_alpha(text)
}

/// One `3ALPHA` subtag of `extlang`.
fn is_extlang_subtag(text: &str) -> bool {
    text.len() == 3 && all_alpha(text)
}

/// `script = 4ALPHA`
fn is_script(text: &str) -> bool {
    text.len() == 4 && all_alpha(text)
}

/// `region = 2ALPHA / 3DIGIT`
fn is_region(text: &str) -> bool {
    (text.len() == 2 && all_alpha(text)) || (text.len() == 3 && all_digit(text))
}

/// `variant = 5*8alphanum / (DIGIT 3alphanum)`
///
/// The two alternatives do not overlap: the second is exactly four characters,
/// the first at least five.
fn is_variant(text: &str) -> bool {
    match text.len() {
        4 => text.starts_with(|first: char| first.is_ascii_digit()) && all_alphanum(text),
        5..=SUBTAG_CEILING => all_alphanum(text),
        _ => false,
    }
}

/// `singleton = DIGIT / %x41-57 / %x59-5A / %x61-77 / %x79-7A`
///
/// That is one alphanumeric character with `x` and `X` carved out, the RFC
/// having reserved them for `privateuse`.
fn is_singleton(text: &str) -> bool {
    text.len() == 1 && all_alphanum(text) && !is_private_use_marker(text)
}

/// One `2*8alphanum` subtag of `extension`.
fn is_extension_subtag(text: &str) -> bool {
    (2..=SUBTAG_CEILING).contains(&text.len()) && all_alphanum(text)
}

/// The `"x"` literal that opens `privateuse`. ABNF literals are
/// case-insensitive (RFC 5234 §2.3), so `X` opens it too.
fn is_private_use_marker(text: &str) -> bool {
    text.eq_ignore_ascii_case("x")
}

/// One `1*8alphanum` subtag of `privateuse`.
fn is_private_use_subtag(text: &str) -> bool {
    (1..=SUBTAG_CEILING).contains(&text.len()) && all_alphanum(text)
}

#[cfg(test)]
mod tests {
    use super::{GRANDFATHERED, LanguageTagError, TagForm, is_well_formed, parse};

    #[test]
    fn langtag_sections_are_reported_as_slices_of_the_input() {
        let tag = parse("zh-cmn-Hans-CN-1901-u-islamcal-x-priv").expect("well-formed");
        assert_eq!(tag.form(), TagForm::Langtag);
        assert_eq!(tag.as_str(), "zh-cmn-Hans-CN-1901-u-islamcal-x-priv");
        assert_eq!(tag.primary_language(), Some("zh"));
        assert_eq!(tag.extended_language(), Some("cmn"));
        assert_eq!(tag.script(), Some("Hans"));
        assert_eq!(tag.region(), Some("CN"));
        assert_eq!(tag.variants().collect::<Vec<_>>(), ["1901"]);
        assert_eq!(tag.extensions(), Some("u-islamcal"));
        assert_eq!(tag.private_use(), Some("x-priv"));
    }

    #[test]
    fn absent_sections_are_none() {
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
    fn repeated_sections_join_with_their_hyphens() {
        let tag = parse("sl-rozaj-biske-1994").expect("well-formed");
        assert_eq!(
            tag.variants().collect::<Vec<_>>(),
            ["rozaj", "biske", "1994"]
        );
        let extensions = parse("en-a-myext-b-another").expect("well-formed");
        assert_eq!(extensions.extensions(), Some("a-myext-b-another"));
    }

    #[test]
    fn whole_tag_forms_have_no_decomposable_sections() {
        let private_use = parse("x-whatever").expect("well-formed");
        assert_eq!(private_use.form(), TagForm::PrivateUse);
        assert_eq!(private_use.primary_language(), None);
        assert_eq!(private_use.private_use(), Some("x-whatever"));

        let grandfathered = parse("i-enochian").expect("well-formed");
        assert_eq!(grandfathered.form(), TagForm::Grandfathered);
        assert_eq!(grandfathered.primary_language(), None);
        assert_eq!(grandfathered.private_use(), None);
    }

    #[test]
    fn grandfathered_list_is_the_closed_set_of_twenty_six() {
        assert_eq!(GRANDFATHERED.len(), 26);
        for (index, tag) in GRANDFATHERED.iter().enumerate() {
            assert!(
                !GRANDFATHERED[..index].contains(tag),
                "{tag:?} appears twice"
            );
            assert_eq!(
                parse(tag).expect("grandfathered").form(),
                TagForm::Grandfathered
            );
        }
    }

    #[test]
    fn every_error_variant_is_reachable_with_an_accepted_neighbour() {
        // (refused input, the error it must name, a neighbour that must accept)
        let cases: &[(&str, LanguageTagError, &str)] = &[
            ("en--US", LanguageTagError::SubtagLengthZero, "en-US"),
            (
                "abcdefghi",
                LanguageTagError::SubtagLengthOverEight,
                "abcdefgh",
            ),
            ("e", LanguageTagError::LanguageProductionUnmatched, "en"),
            (
                "zh-cmn-yue-nan-hak",
                LanguageTagError::ExtlangRepetitionExceeded,
                "zh-cmn-yue-nan",
            ),
            ("en-a", LanguageTagError::SingletonWithoutSubtag, "en-a-bb"),
            ("en-x", LanguageTagError::PrivateUseWithoutSubtag, "en-x-a"),
            ("de-419-DE", LanguageTagError::UnconsumedSubtag, "de-DE"),
        ];
        for (refused, expected, accepted) in cases {
            assert_eq!(parse(refused), Err(*expected), "{refused:?}");
            assert!(is_well_formed(accepted), "{accepted:?} must stay accepted");
        }
    }

    #[test]
    fn the_extlang_bound_refuses_only_a_fourth_extlang() {
        // `extlang = 3ALPHA *2("-" 3ALPHA)`: one subtag plus at most two more.
        assert!(is_well_formed("zh-cmn"));
        assert!(is_well_formed("zh-cmn-yue"));
        assert!(is_well_formed("zh-cmn-yue-nan"));
        assert_eq!(
            parse("zh-cmn-yue-nan-hak"),
            Err(LanguageTagError::ExtlangRepetitionExceeded)
        );
        // A spent repetition must not poison the sections that follow it.
        assert!(is_well_formed("zh-cmn-yue-nan-Hant"));
        assert!(is_well_formed("zh-cmn-yue-nan-Hant-CN"));
        assert!(is_well_formed("zh-cmn-yue-nan-CN"));
        assert!(is_well_formed("zh-cmn-yue-nan-x-priv"));
        // `extlang` only follows the `2*3ALPHA` alternative of `language`.
        assert!(is_well_formed("abcd-Latn"));
        assert_eq!(
            parse("abcd-efg"),
            Err(LanguageTagError::UnconsumedSubtag),
            "a 4ALPHA primary language admits no extlang"
        );
    }

    #[test]
    fn diagnostic_codes_are_stable_and_distinct() {
        let codes = [
            LanguageTagError::SubtagLengthZero,
            LanguageTagError::SubtagLengthOverEight,
            LanguageTagError::LanguageProductionUnmatched,
            LanguageTagError::ExtlangRepetitionExceeded,
            LanguageTagError::SingletonWithoutSubtag,
            LanguageTagError::PrivateUseWithoutSubtag,
            LanguageTagError::UnconsumedSubtag,
        ]
        .map(LanguageTagError::diagnostic_code);
        assert_eq!(codes[0], "langtag-subtag-length-zero");
        assert_eq!(codes[6], "langtag-unconsumed-subtag");
        for (index, code) in codes.iter().enumerate() {
            assert!(code.starts_with("langtag-"), "{code:?}");
            assert!(!codes[..index].contains(code), "{code:?} appears twice");
        }
    }

    #[test]
    fn well_formedness_ignores_registry_validity() {
        // §2.2.9 puts a duplicate singleton on the invalid-but-well-formed side.
        assert!(is_well_formed("ar-a-aaa-b-bbb-a-ccc"));
        // Unregistered subtags of the right shape are likewise well-formed.
        assert!(is_well_formed("qq-Zzzz-QQ"));
    }

    #[test]
    fn case_is_insignificant() {
        for tag in ["en-US", "EN-us", "eN-Us", "ZH-HANT", "X-FOO", "I-ENOCHIAN"] {
            assert!(is_well_formed(tag), "{tag:?}");
        }
    }

    #[test]
    fn non_ascii_and_control_input_is_refused() {
        for tag in ["en-ü", "ünn", "en\u{9}US", "en-US\u{0}"] {
            assert!(!is_well_formed(tag), "{tag:?} must refuse");
        }
        assert!(is_well_formed("en-US"), "the ASCII neighbour still accepts");
    }
}
