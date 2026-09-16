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
//! # Profiles
//!
//! [`Profile`] names the acceptance language a judgement is made against.
//! [`Profile::Rfc5646`] is the ABNF verbatim and is what the profile-free
//! entry points ([`parse`], [`is_well_formed`]) use. The alternatives are
//! [`Profile::Rfc5646PrivateUseRelaxed`], which lifts exactly one bound, and
//! [`Profile::ConcreteSyntaxLangtag`], which swaps the whole production for the
//! `LANGTAG` terminal that the RDF concrete syntaxes actually specify — see
//! each variant for what it widens and why. Adding a profile is adding a
//! documented widening in *this* module, which is the point: a caller that
//! needs "RFC 5646 but…" names a profile here rather than growing a fourth
//! private copy of the grammar somewhere else in the workspace.
//!
//! The variants are totally ordered by acceptance —
//! [`Profile::Rfc5646`] ⊂ [`Profile::Rfc5646PrivateUseRelaxed`] ⊂
//! [`Profile::ConcreteSyntaxLangtag`] — and a tag a narrower profile accepts
//! [`parse_with`]s to the *same* [`LanguageTag`] under every wider one, so
//! widening never costs a caller its decomposition.
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

/// The acceptance language a judgement is made against.
///
/// Every variant other than [`Self::Rfc5646`] is a *widening* of it: the set of
/// accepted tags only grows, and each variant documents the single bound it
/// lifts. Nothing here narrows the grammar, so a tag accepted under
/// [`Self::Rfc5646`] is accepted under every profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Profile {
    /// The RFC 5646 §2.1 ABNF verbatim, with nothing added or removed. This is
    /// what [`parse`] and [`is_well_formed`] apply.
    #[default]
    Rfc5646,
    /// RFC 5646 §2.1 with the eight-character subtag ceiling lifted for
    /// `privateuse` subtags only.
    ///
    /// # What it widens
    ///
    /// Precisely one bound: the `1*8alphanum` in
    /// `privateuse = "x" 1*("-" (1*8alphanum))` becomes `1*alphanum`, for the
    /// subtags that follow an `x`/`X` marker and for those only. Every other
    /// production keeps its exact §2.1 shape, so `abcdefghi-x-a` is still
    /// refused (the over-long subtag is a `language`, not a private-use one)
    /// and `de-419-DE` is still refused for the reason it always was. The
    /// widening is unambiguous because a standalone one-character `x` subtag
    /// can be nothing but the `privateuse` marker: `singleton` explicitly
    /// carves `x` and `X` out, and no other production admits a one-character
    /// subtag at all.
    ///
    /// # Why it exists
    ///
    /// PurRDF's internal artifacts carry private-use tags of the form
    /// `x-purrdf-<name>`, where `<name>` is a spelled-out English language name
    /// rather than a subtag. Several of those names exceed eight characters —
    /// `x-purrdf-afrikaans` (9) and `x-purrdf-norwegiannynorsk` (16) are both
    /// live in the workspace's fixtures and corpora — so the §2.1 ceiling
    /// refuses them. The tags are confined to the private-use space the RFC
    /// sets aside for exactly this ("subtags ... available for private
    /// definition"), and the only thing standing in the way is a length bound
    /// whose whole purpose is registry discipline, which does not apply inside
    /// `x-`. Callers that must round-trip those artifacts select this profile;
    /// callers that judge externally-supplied tags should not.
    Rfc5646PrivateUseRelaxed,
    /// The `LANGTAG` **terminal** of the RDF concrete syntaxes, rather than the
    /// RFC 5646 `Language-Tag` production.
    ///
    /// # What it accepts
    ///
    /// Exactly the terminal the Turtle, TriG, N-Triples and N-Quads grammars
    /// spell, minus its leading `@`:
    ///
    /// ```text
    /// LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*
    /// ```
    ///
    /// A non-empty all-ALPHA first subtag, then zero or more non-empty
    /// alphanumeric subtags. There is no length cap on any subtag and no
    /// production structure at all: `script`, `region`, `variant`, `extension`
    /// and `privateuse` simply do not exist at this level, so `en-fr-jura` and
    /// `fr-be-fbcl` — neither of which is well-formed RFC 5646, `variant`
    /// admitting a four-character subtag only when the first character is a
    /// DIGIT — are both accepted.
    ///
    /// # What it still refuses
    ///
    /// The genuine garbage the terminal itself excludes, which is what makes
    /// this a grammar and not a rubber stamp: a first subtag that is not all
    /// letters (`1`, `9-9`, `123-456`), an empty subtag anywhere (the empty
    /// string, `-`, `en-`, `en--US`), and any non-alphanumeric character
    /// (`en-ü`, `en-a!`).
    ///
    /// # Why it exists
    ///
    /// The concrete syntaxes' terminal is deliberately looser than RFC 5646,
    /// and the W3C's own approved corpora rely on that looseness: the ShEx
    /// validation vectors carry `"ab"@en-fr-jura` and `"septante"@fr-be-fbcl`
    /// in `text/turtle` documents, so a parser that held Turtle to §2.1 would
    /// fail to read approved tests. Tags published by downstream projects push
    /// the same way — a private-use family of the form `x-<project>-<language
    /// name>` routinely exceeds the eight-character private-use cap
    /// (`x-gmeow-norwegiannynorsk` is sixteen), and those literals are numerous
    /// enough that refusing them would break a real consumer.
    ///
    /// So this profile is what an *ingesting parser* holds a language tag to:
    /// the contract the document's own grammar states. A caller deciding
    /// whether a tag is fit to publish, index or match wants
    /// [`Self::Rfc5646`] instead — this one deliberately does not answer that
    /// question.
    ///
    /// # Decomposition
    ///
    /// Because the terminal is a strict superset of
    /// [`Self::Rfc5646PrivateUseRelaxed`], a tag with an RFC 5646 reading keeps
    /// it: [`parse_with`] reports the same sections it would under the narrower
    /// profile. Only a tag that has no such reading comes back as
    /// [`TagForm::ConcreteSyntaxOnly`], with no sections, because the terminal
    /// genuinely does not name any.
    ConcreteSyntaxLangtag,
}

/// Which of the three top-level `Language-Tag` alternatives matched, or that
/// none did and only a looser profile's grammar was satisfied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagForm {
    /// The ordinary `langtag` production (`language ["-" script] ...`).
    Langtag,
    /// A whole-tag `privateuse` production (`"x" 1*("-" (1*8alphanum))`).
    PrivateUse,
    /// One of the 26 closed `grandfathered` tags, matched as a unit.
    Grandfathered,
    /// No RFC 5646 alternative matched, but the concrete syntaxes' `LANGTAG`
    /// terminal did. Reachable only under [`Profile::ConcreteSyntaxLangtag`],
    /// and the one form with no decomposable sections to report — the terminal
    /// names none. `en-fr-jura` is this.
    ConcreteSyntaxOnly,
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
    /// The first subtag of the `LANGTAG` terminal was not one or more ASCII
    /// letters (`1`, `9-9`, `123-456`). Reachable only under
    /// [`Profile::ConcreteSyntaxLangtag`]; the RFC 5646 profiles refuse the
    /// same inputs under [`Self::LanguageProductionUnmatched`], whose message
    /// would name a production this profile does not apply.
    TerminalPrimaryNotAlpha,
    /// A `LANGTAG` subtag after the first held a character outside
    /// `[a-zA-Z0-9]` (`en-ü`, `en-a!`). Reachable only under
    /// [`Profile::ConcreteSyntaxLangtag`], for the same reason as
    /// [`Self::TerminalPrimaryNotAlpha`].
    TerminalSubtagNotAlphanum,
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
            Self::TerminalPrimaryNotAlpha => "langtag-terminal-primary-not-alpha",
            Self::TerminalSubtagNotAlphanum => "langtag-terminal-subtag-not-alphanum",
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
            Self::TerminalPrimaryNotAlpha => {
                "first subtag of the `LANGTAG` terminal is not one or more ASCII letters"
            }
            Self::TerminalSubtagNotAlphanum => {
                "`LANGTAG` subtag after the first holds a character outside [a-zA-Z0-9]"
            }
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

/// A language tag accepted by the profile it was parsed under, borrowing the
/// input verbatim.
///
/// Each section of the `langtag` production is recorded as the byte range it
/// occupied, so every accessor is a slice of the original input: nothing is
/// copied, re-encoded or case-normalized. A tag whose [`form`](Self::form) is
/// [`TagForm::ConcreteSyntaxOnly`] has no sections at all, that terminal naming
/// none.
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

/// `true` when `tag` matches the `Language-Tag` production as `profile` draws
/// it. [`is_well_formed`] is this with [`Profile::Rfc5646`].
#[must_use]
pub fn is_well_formed_with(tag: &str, profile: Profile) -> bool {
    parse_with(tag, profile).is_ok()
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
    parse_with(tag, Profile::Rfc5646)
}

/// Parses `tag` against the grammar `profile` names.
///
/// For the two RFC 5646 profiles the decomposition is the one [`parse`]
/// documents, and `profile` only selects which bounds the `privateuse` subtags
/// are held to. [`Profile::ConcreteSyntaxLangtag`] replaces the production with
/// the concrete syntaxes' `LANGTAG` terminal; see [`Profile`] for what each
/// variant widens.
///
/// # Errors
///
/// A typed [`LanguageTagError`] naming the production that refused; see that
/// type for the failure map.
pub fn parse_with(tag: &str, profile: Profile) -> Result<LanguageTag<'_>, LanguageTagError> {
    if matches!(profile, Profile::ConcreteSyntaxLangtag) {
        return parse_concrete_syntax_langtag(tag);
    }

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
    check_subtag_envelope(tag, profile)?;

    let mut cursor = Cursor::new(tag);

    // `privateuse` as the whole tag. It is the only alternative that may begin
    // with the reserved `x` singleton, so a leading `x` commits to it: there is
    // no `langtag` reading to fall back to.
    if let Some(span) = take_private_use(&mut cursor, profile)? {
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
    let private_use = take_private_use(&mut cursor, profile)?;
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

/// [`Profile::ConcreteSyntaxLangtag`]: the `LANGTAG` terminal rather than the
/// RFC 5646 production.
///
/// The RFC 5646 reading is tried FIRST, and not as an optimisation: the
/// terminal is a strict superset of [`Profile::Rfc5646PrivateUseRelaxed`] — the
/// production's every subtag is non-empty and alphanumeric, and its every first
/// subtag is all-ALPHA (`language` is ALPHA, `privateuse` opens with `x`, and
/// all 26 grandfathered tags begin with letters) — so whenever that reading
/// exists it is a reading of the *same* accepted tag, and reporting it keeps
/// the decomposition a caller would have got from the narrower profile. Only a
/// tag with no such reading falls through to the terminal, where there is
/// nothing to decompose.
fn parse_concrete_syntax_langtag(tag: &str) -> Result<LanguageTag<'_>, LanguageTagError> {
    if let Ok(parsed) = parse_with(tag, Profile::Rfc5646PrivateUseRelaxed) {
        return Ok(parsed);
    }
    check_langtag_terminal(tag)?;
    Ok(LanguageTag::whole(tag, TagForm::ConcreteSyntaxOnly, None))
}

/// `LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*`, minus the `@` the lexer has
/// already taken.
///
/// One `+` per subtag is the whole grammar, so the emptiness test comes before
/// the character-class test at every position: `all_alpha` and `all_alphanum`
/// are vacuously true for the empty string, and without the ordering `en-`
/// would be accepted.
fn check_langtag_terminal(tag: &str) -> Result<(), LanguageTagError> {
    let mut subtags = tag.split('-');
    // `str::split` always yields at least one item, so the default is unreachable;
    // taking it as the empty string routes an impossible case to the same refusal
    // the empty input gets rather than to a panic.
    let primary = subtags.next().unwrap_or_default();
    if primary.is_empty() {
        return Err(LanguageTagError::SubtagLengthZero);
    }
    if !all_alpha(primary) {
        return Err(LanguageTagError::TerminalPrimaryNotAlpha);
    }
    for subtag in subtags {
        if subtag.is_empty() {
            return Err(LanguageTagError::SubtagLengthZero);
        }
        if !all_alphanum(subtag) {
            return Err(LanguageTagError::TerminalSubtagNotAlphanum);
        }
    }
    Ok(())
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
///
/// Under [`Profile::Rfc5646PrivateUseRelaxed`] the ceiling stops applying at
/// the first standalone `x`/`X` subtag, because every subtag from there on is
/// a `privateuse` subtag and nothing else can be: `singleton` carves `x` out,
/// and no other production admits a one-character subtag. The zero-length
/// refusal is unconditional — an empty subtag fits no production under any
/// profile. Lifting the ceiling *here* as well as in
/// [`is_private_use_subtag`] is what keeps this pre-pass and the productions
/// below in agreement, rather than having the pre-pass refuse a tag the
/// productions would have taken.
fn check_subtag_envelope(tag: &str, profile: Profile) -> Result<(), LanguageTagError> {
    let relaxed = matches!(
        profile,
        Profile::Rfc5646PrivateUseRelaxed | Profile::ConcreteSyntaxLangtag
    );
    let mut ceiling_applies = true;
    for subtag in tag.split('-') {
        if subtag.is_empty() {
            return Err(LanguageTagError::SubtagLengthZero);
        }
        if ceiling_applies && subtag.len() > SUBTAG_CEILING {
            return Err(LanguageTagError::SubtagLengthOverEight);
        }
        if relaxed && is_private_use_marker(subtag) {
            ceiling_applies = false;
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
///
/// This is the only production `profile` reaches: the `1*8alphanum` bound on
/// its subtags is the single thing
/// [`Profile::Rfc5646PrivateUseRelaxed`] widens.
fn take_private_use(
    cursor: &mut Cursor<'_>,
    profile: Profile,
) -> Result<Option<Span>, LanguageTagError> {
    let Some(marker) = cursor.take_if(is_private_use_marker) else {
        return Ok(None);
    };
    let mut run = marker;
    let mut subtags = 0usize;
    while let Some(subtag) = cursor.take_if(|text| is_private_use_subtag(text, profile)) {
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

/// One `1*8alphanum` subtag of `privateuse` — `1*alphanum` under
/// [`Profile::Rfc5646PrivateUseRelaxed`], which lifts the upper bound and
/// nothing else.
fn is_private_use_subtag(text: &str, profile: Profile) -> bool {
    let ceiling = match profile {
        Profile::Rfc5646 => SUBTAG_CEILING,
        // `ConcreteSyntaxLangtag` re-enters through the relaxed profile rather
        // than reaching here, but it caps no subtag either, so it answers with
        // the relaxed profile and stays correct if that routing ever changes.
        Profile::Rfc5646PrivateUseRelaxed | Profile::ConcreteSyntaxLangtag => usize::MAX,
    };
    (1..=ceiling).contains(&text.len()) && all_alphanum(text)
}

#[cfg(test)]
mod tests {
    use super::{
        GRANDFATHERED, LanguageTagError, Profile, TagForm, is_well_formed, is_well_formed_with,
        parse, parse_with,
    };

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
            LanguageTagError::TerminalPrimaryNotAlpha,
            LanguageTagError::TerminalSubtagNotAlphanum,
        ]
        .map(LanguageTagError::diagnostic_code);
        assert_eq!(codes[0], "langtag-subtag-length-zero");
        assert_eq!(codes[6], "langtag-unconsumed-subtag");
        assert_eq!(codes[8], "langtag-terminal-subtag-not-alphanum");
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
    fn the_profile_free_entry_points_are_exactly_rfc5646() {
        // The frozen differential table asserts `is_well_formed`, so the
        // default profile must never drift away from the unparameterized pair.
        assert_eq!(Profile::default(), Profile::Rfc5646);
        for tag in [
            "en",
            "x-purrdf-english",
            "x-purrdf-afrikaans",
            "abcdefghi",
            "de-CH-x-phonebk",
            "",
        ] {
            assert_eq!(
                parse(tag),
                parse_with(tag, Profile::Rfc5646),
                "{tag:?} must agree"
            );
            assert_eq!(
                is_well_formed(tag),
                is_well_formed_with(tag, Profile::Rfc5646),
                "{tag:?} must agree"
            );
        }
    }

    #[test]
    fn the_relaxed_profile_widens_private_use_subtags_and_only_those() {
        // What it widens: a `privateuse` subtag past the eight-character
        // ceiling, in both the whole-tag and the trailing-section position.
        for widened in [
            "x-purrdf-afrikaans",
            "x-purrdf-norwegiannynorsk",
            "de-CH-x-norwegiannynorsk",
            "en-x-abcdefghi",
        ] {
            assert_eq!(
                parse(widened),
                Err(LanguageTagError::SubtagLengthOverEight),
                "{widened:?} is over the RFC 5646 ceiling"
            );
            assert!(
                is_well_formed_with(widened, Profile::Rfc5646PrivateUseRelaxed),
                "{widened:?} must be accepted by the relaxed profile"
            );
        }

        // What it does NOT widen. Each of these is refused by BOTH profiles,
        // and is paired with a neighbour the relaxed profile must still take,
        // so a future "relax a bit more" cannot pass unnoticed.
        let unchanged: &[(&str, LanguageTagError, &str)] = &[
            // An over-long subtag BEFORE the marker is not a private-use one.
            (
                "abcdefghi-x-a",
                LanguageTagError::SubtagLengthOverEight,
                "abcdefgh-x-a",
            ),
            (
                "en-abcdefghi-x-a",
                LanguageTagError::SubtagLengthOverEight,
                "en-abcdefgh-x-a",
            ),
            // Non-alphanumerics stay out of the private-use section too.
            (
                "x-purrdf-afri!",
                LanguageTagError::UnconsumedSubtag,
                "x-purrdf-afri",
            ),
            // Every other production keeps its exact §2.1 shape.
            ("de-419-DE", LanguageTagError::UnconsumedSubtag, "de-DE"),
            ("e", LanguageTagError::LanguageProductionUnmatched, "en"),
            ("en--US", LanguageTagError::SubtagLengthZero, "en-US"),
            ("en-x", LanguageTagError::PrivateUseWithoutSubtag, "en-x-a"),
            ("x", LanguageTagError::PrivateUseWithoutSubtag, "x-a"),
        ];
        for (refused, expected, accepted) in unchanged {
            assert_eq!(parse(refused), Err(*expected), "{refused:?} under RFC 5646");
            assert_eq!(
                parse_with(refused, Profile::Rfc5646PrivateUseRelaxed),
                Err(*expected),
                "{refused:?} must stay refused under the relaxed profile"
            );
            assert!(
                is_well_formed_with(accepted, Profile::Rfc5646PrivateUseRelaxed),
                "{accepted:?} must stay accepted under the relaxed profile"
            );
        }
    }

    #[test]
    fn the_relaxed_profile_still_decomposes_into_the_same_sections() {
        let tag = parse_with(
            "de-CH-x-norwegiannynorsk",
            Profile::Rfc5646PrivateUseRelaxed,
        )
        .expect("relaxed profile accepts an over-long private-use subtag");
        assert_eq!(tag.form(), TagForm::Langtag);
        assert_eq!(tag.primary_language(), Some("de"));
        assert_eq!(tag.region(), Some("CH"));
        assert_eq!(tag.private_use(), Some("x-norwegiannynorsk"));

        let whole = parse_with("x-purrdf-afrikaans", Profile::Rfc5646PrivateUseRelaxed)
            .expect("whole-tag private use");
        assert_eq!(whole.form(), TagForm::PrivateUse);
        assert_eq!(whole.private_use(), Some("x-purrdf-afrikaans"));
    }

    /// The terminal profile's whole contract, both halves. The accept column is
    /// not decoration: this profile exists BECAUSE tags that are not well-formed
    /// RFC 5646 are lawful in a Turtle document, so a regression that quietly
    /// tightened it would look exactly like correct strictness.
    #[test]
    fn the_terminal_profile_accepts_the_langtag_terminal_and_refuses_the_rest() {
        // `LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*`, minus the `@`.
        for accepted in [
            // Ordinary tags, well-formed under every profile.
            "en",
            "en-US",
            "zh-Hans-CN",
            "i-enochian",
            "de-CH-x-phonebk",
            // Not well-formed RFC 5646, but lawful terminals — and both are
            // carried by approved W3C ShEx vectors read as `text/turtle`.
            // `jura` and `fbcl` are four ALPHA, and `variant` admits four
            // characters only when the first is a DIGIT.
            "en-fr-jura",
            "fr-be-fbcl",
            // Private-use families this workspace and a downstream project
            // publish. The last two are over the §2.1 eight-character
            // private-use cap, which is why the strict profile cannot be the
            // ingestion contract.
            "x-purrdf-english",
            "x-purrdf-afrikaans",
            "x-purrdf-norwegiannynorsk",
            "x-gmeow-english",
            "x-gmeow-chinese-latn",
            "x-gmeow-norwegiannynorsk",
            // No length cap applies at any position under this profile.
            "abcdefghijklmnop",
            "en-abcdefghijklmnop",
        ] {
            assert!(
                is_well_formed_with(accepted, Profile::ConcreteSyntaxLangtag),
                "{accepted:?} is a lawful `LANGTAG` terminal and must be accepted"
            );
        }

        // (refused input, the error it must name, a neighbour that must accept)
        let refused: &[(&str, LanguageTagError, &str)] = &[
            ("", LanguageTagError::SubtagLengthZero, "en"),
            ("-", LanguageTagError::SubtagLengthZero, "en"),
            ("en-", LanguageTagError::SubtagLengthZero, "en"),
            ("-en", LanguageTagError::SubtagLengthZero, "en"),
            ("en--US", LanguageTagError::SubtagLengthZero, "en-US"),
            ("1", LanguageTagError::TerminalPrimaryNotAlpha, "en"),
            ("9-9", LanguageTagError::TerminalPrimaryNotAlpha, "en-9"),
            (
                "123-456",
                LanguageTagError::TerminalPrimaryNotAlpha,
                "abc-456",
            ),
            (
                "en-ü",
                LanguageTagError::TerminalSubtagNotAlphanum,
                "en-u-uu",
            ),
            (
                "en-a!",
                LanguageTagError::TerminalSubtagNotAlphanum,
                "en-a-bb",
            ),
        ];
        for (input, expected, neighbour) in refused {
            assert_eq!(
                parse_with(input, Profile::ConcreteSyntaxLangtag),
                Err(*expected),
                "{input:?} must refuse"
            );
            assert!(
                is_well_formed_with(neighbour, Profile::ConcreteSyntaxLangtag),
                "{neighbour:?} must stay accepted"
            );
        }
    }

    /// The profiles are totally ordered by acceptance, and widening never costs
    /// a caller the decomposition the narrower profile gave it.
    #[test]
    fn the_terminal_profile_is_a_superset_that_preserves_decomposition() {
        for tag in [
            "en",
            "en-US",
            "zh-cmn-Hans-CN-1901-u-islamcal-x-priv",
            "i-enochian",
            "x-whatever",
            "x-purrdf-afrikaans",
            "ar-a-aaa-b-bbb-a-ccc",
        ] {
            // Whatever the relaxed profile accepts, the terminal accepts, and
            // parses to the very same record.
            let relaxed = parse_with(tag, Profile::Rfc5646PrivateUseRelaxed)
                .expect("accepted by the relaxed profile");
            assert_eq!(
                parse_with(tag, Profile::ConcreteSyntaxLangtag),
                Ok(relaxed),
                "{tag:?} must keep its RFC 5646 decomposition under the terminal"
            );
            assert_ne!(
                relaxed.form(),
                TagForm::ConcreteSyntaxOnly,
                "{tag:?} has an RFC 5646 reading"
            );
        }

        // A tag with no RFC 5646 reading comes back with no sections, because
        // the terminal names none.
        let terminal_only = parse_with("en-fr-jura", Profile::ConcreteSyntaxLangtag)
            .expect("a lawful `LANGTAG` terminal");
        assert_eq!(terminal_only.form(), TagForm::ConcreteSyntaxOnly);
        assert_eq!(terminal_only.as_str(), "en-fr-jura");
        assert_eq!(terminal_only.primary_language(), None);
        assert_eq!(terminal_only.script(), None);
        assert_eq!(terminal_only.region(), None);
        assert_eq!(terminal_only.variants().count(), 0);
        assert_eq!(terminal_only.extensions(), None);
        assert_eq!(terminal_only.private_use(), None);
        assert_eq!(
            parse(terminal_only.as_str()),
            Err(LanguageTagError::UnconsumedSubtag),
            "…and RFC 5646 still refuses it, which is the whole point"
        );
    }

    #[test]
    fn non_ascii_and_control_input_is_refused() {
        for tag in ["en-ü", "ünn", "en\u{9}US", "en-US\u{0}"] {
            assert!(!is_well_formed(tag), "{tag:?} must refuse");
        }
        assert!(is_well_formed("en-US"), "the ASCII neighbour still accepts");
    }
}
