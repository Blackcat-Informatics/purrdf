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
//! grammar. A caller that wants that presentation asks for it explicitly with
//! [`canonical_case`], which is a separate, allocating rewrite and never
//! changes what [`parse`] or [`is_well_formed`] answer.
//!
//! # What a parsed tag gives you
//!
//! [`LanguageTag`] borrows the input and reports each section as a slice of it.
//! Beyond the flat sections it decomposes the two grouped ones the grammar
//! leaves joined — [`LanguageTag::extensions_by_singleton`] yields one
//! [`Extension`] per `singleton 1*("-" (2*8alphanum))` run, and
//! [`LanguageTag::private_use_subtags`] yields the subtags after the `x`
//! marker. [`LanguageTagBuf`] is the owning twin for callers that must outlive
//! the borrow, and carries the standard trait set ([`core::str::FromStr`],
//! [`AsRef`], [`core::borrow::Borrow`], [`Ord`], [`core::hash::Hash`]).
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

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::str::FromStr;

/// The closed `grandfathered` set of RFC 5646 §2.2.8, in the order the RFC
/// presents it: the seventeen `irregular` tags, then the nine `regular` ones.
///
/// Matched case-insensitively as whole tags, and the spelling here is the
/// **registered** one, which is what §2.1.1 canonical case preserves for these
/// tags rather than deriving. The set is closed by the specification — "no new
/// grandfathered tags will be created" — so this array is complete by
/// construction rather than by maintenance.
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

/// A `singleton`, and the `x`/`X` `privateuse` marker, are each exactly one
/// character. That is what makes a one-character subtag unambiguous: no other
/// production admits one.
const SINGLETON_LENGTH: usize = 1;

/// Bytes to skip to step past a leading one-character marker *and* the hyphen
/// that joins it to the subtags it introduces (`u-islamcal` -> `islamcal`).
const MARKER_PREFIX_WIDTH: usize = SINGLETON_LENGTH + 1;

/// The acceptance language a judgement is made against.
///
/// Every variant other than [`Self::Rfc5646`] is a *widening* of it: the set of
/// accepted tags only grows, and each variant documents the single bound it
/// lifts. Nothing here narrows the grammar, so a tag accepted under
/// [`Self::Rfc5646`] is accepted under every profile.
///
/// The variants are declared in acceptance order, so the derived [`Ord`] *is*
/// the ⊂ relation this module documents: a greater profile accepts every tag a
/// lesser one does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// The human-readable reason, as a `&'static str`.
    ///
    /// [`fmt::Display`] renders exactly this. It is exposed separately because
    /// a consumer whose own error type carries a `&'static str` payload — the
    /// embedding artifact validator is one — would otherwise have to collapse
    /// every refusal to a single generic sentence to fit, which is precisely
    /// how a typed diagnostic stops reaching the user.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
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
        }
    }
}

impl fmt::Display for LanguageTagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl core::error::Error for LanguageTagError {}

/// A half-open byte range into the tag a [`LanguageTag`] borrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// Where each section of the `langtag` production sat in the input.
///
/// Split out from [`LanguageTag`] so that the borrowing and the owning form can
/// share one decomposition rather than two copies of the same seven fields that
/// could drift apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Sections {
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

impl Sections {
    /// The decomposition of a form that names no sections.
    const NONE: Self = Self {
        language: None,
        extlang: None,
        script: None,
        region: None,
        variants: None,
        extensions: None,
        private_use: None,
    };
}

/// A language tag accepted by the profile it was parsed under, borrowing the
/// input verbatim.
///
/// Each section of the `langtag` production is recorded as the byte range it
/// occupied, so every accessor is a slice of the original input: nothing is
/// copied, re-encoded or case-normalized. A tag whose [`form`](Self::form) is
/// [`TagForm::ConcreteSyntaxOnly`] has no sections at all, that terminal naming
/// none.
///
/// # Identity
///
/// [`PartialEq`] is structural over the whole decomposition, but [`Ord`] and
/// [`Hash`] key on the tag string alone. Those agree, and the agreement is a
/// property of the parser rather than a convention: a `LanguageTag` can only be
/// produced by [`parse_with`], and for one input string every profile that
/// accepts it yields the *same* record (the wider profiles only ever lift a
/// bound, and [`Profile::ConcreteSyntaxLangtag`] returns the narrower profile's
/// reading verbatim whenever one exists). So equal strings imply equal records,
/// which is exactly what makes the [`Borrow<str>`] impl below lawful — a
/// `HashMap` or `BTreeMap` keyed by a tag can be probed with a plain `&str`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageTag<'a> {
    /// The input, exactly as supplied.
    tag: &'a str,
    /// Which top-level alternative matched.
    form: TagForm,
    /// Where each section of the production sat.
    sections: Sections,
}

impl<'a> LanguageTag<'a> {
    /// A tag that matched a whole-tag alternative (`privateuse` as the entire
    /// input, or `grandfathered`), which has no decomposable sections.
    const fn whole(tag: &'a str, form: TagForm, private_use: Option<Span>) -> Self {
        Self {
            tag,
            form,
            sections: Sections {
                private_use,
                ..Sections::NONE
            },
        }
    }

    /// The parse record for an accepted tag, given its decomposition.
    const fn decomposed(tag: &'a str, form: TagForm, sections: Sections) -> Self {
        Self {
            tag,
            form,
            sections,
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
        self.sections.language.map(|span| span.of(self.tag))
    }

    /// The extended-language subtags, hyphen-joined, when present.
    #[must_use]
    pub fn extended_language(&self) -> Option<&'a str> {
        self.sections.extlang.map(|span| span.of(self.tag))
    }

    /// The script subtag, when present.
    #[must_use]
    pub fn script(&self) -> Option<&'a str> {
        self.sections.script.map(|span| span.of(self.tag))
    }

    /// The region subtag, when present.
    #[must_use]
    pub fn region(&self) -> Option<&'a str> {
        self.sections.region.map(|span| span.of(self.tag))
    }

    /// The variant subtags in order of appearance.
    pub fn variants(&self) -> impl Iterator<Item = &'a str> {
        subtags_in(self.sections.variants.map(|span| span.of(self.tag)))
    }

    /// The raw extension section (`a-myext-b-another`), hyphen-joined, when
    /// present.
    ///
    /// This is the run exactly as written. Use
    /// [`extensions_by_singleton`](Self::extensions_by_singleton) to walk it as
    /// the `singleton 1*("-" (2*8alphanum))` groups the ABNF actually names.
    #[must_use]
    pub fn extensions(&self) -> Option<&'a str> {
        self.sections.extensions.map(|span| span.of(self.tag))
    }

    /// The extension sections, one [`Extension`] per singleton, in the order
    /// they appear.
    ///
    /// The grouping is recovered from the run rather than recorded during the
    /// parse, and it is unambiguous: `extension = singleton 1*("-"
    /// (2*8alphanum))` gives every extension subtag at least two characters, so
    /// the only one-character subtag inside the run is the next singleton.
    ///
    /// Well-formedness does not *interpret* extensions and does not deduplicate
    /// them: RFC 5646 §2.2.9 puts a repeated singleton on the invalid-but-well-
    /// formed side, so `ar-a-aaa-b-bbb-a-ccc` yields three extensions, two of
    /// which are keyed `a`. A caller that needs one section per singleton must
    /// decide for itself which repeat wins.
    pub fn extensions_by_singleton(&self) -> impl Iterator<Item = Extension<'a>> {
        split_extensions(self.extensions())
    }

    /// The first extension section keyed by `singleton`, matched
    /// case-insensitively because `singleton` is an ABNF character range over
    /// both cases.
    ///
    /// "First" rather than "the": see
    /// [`extensions_by_singleton`](Self::extensions_by_singleton) for why a
    /// well-formed tag may carry a singleton twice.
    #[must_use]
    pub fn extension(&self, singleton: char) -> Option<Extension<'a>> {
        self.extensions_by_singleton()
            .find(|extension| extension.singleton().eq_ignore_ascii_case(&singleton))
    }

    /// The private-use section including its `x`/`X` marker (`x-phonebk`), or
    /// the whole tag for the whole-tag private-use form.
    #[must_use]
    pub fn private_use(&self) -> Option<&'a str> {
        self.sections.private_use.map(|span| span.of(self.tag))
    }

    /// The private-use subtags after the `x`/`X` marker, in order.
    ///
    /// Empty when the tag has no private-use section. The marker itself is
    /// never yielded, and `privateuse = "x" 1*("-" (1*8alphanum))` guarantees at
    /// least one subtag whenever the section exists.
    pub fn private_use_subtags(&self) -> impl Iterator<Item = &'a str> {
        subtags_in(
            self.private_use()
                .and_then(|section| section.get(MARKER_PREFIX_WIDTH..)),
        )
    }

    /// This tag rewritten in the RFC 5646 §2.1.1 canonical case convention.
    ///
    /// See [`canonical_case`] for the rule and its two exceptions. Allocating
    /// and idempotent; the result parses to an equal tag under the same profile,
    /// case being insignificant to the grammar.
    #[must_use]
    pub fn canonical_case(&self) -> String {
        let mut canonical = String::with_capacity(self.tag.len());
        write_canonical_case(self.tag, &mut canonical);
        canonical
    }

    /// `true` when this tag is already spelled in §2.1.1 canonical case.
    ///
    /// Decides the same question as `tag.as_str() == tag.canonical_case()`
    /// without allocating.
    #[must_use]
    pub fn is_canonical_case(&self) -> bool {
        canonical_case_holds(self.tag)
    }

    /// This tag as an owning [`LanguageTagBuf`], copying the input once.
    #[must_use]
    pub fn to_owned_tag(&self) -> LanguageTagBuf {
        LanguageTagBuf {
            tag: self.tag.to_owned(),
            form: self.form,
            sections: self.sections,
        }
    }
}

impl fmt::Display for LanguageTag<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag)
    }
}

impl AsRef<str> for LanguageTag<'_> {
    fn as_ref(&self) -> &str {
        self.tag
    }
}

impl Borrow<str> for LanguageTag<'_> {
    fn borrow(&self) -> &str {
        self.tag
    }
}

impl Hash for LanguageTag<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
    }
}

impl Ord for LanguageTag<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tag.cmp(other.tag)
    }
}

impl PartialOrd for LanguageTag<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// One `extension = singleton 1*("-" (2*8alphanum))` section of a tag.
///
/// Yielded by [`LanguageTag::extensions_by_singleton`]. The section is a slice
/// of the original input, so its case is whatever the author wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Extension<'a> {
    /// The `singleton` that keys this extension.
    singleton: char,
    /// The whole section, singleton included (`u-islamcal`).
    section: &'a str,
}

impl<'a> Extension<'a> {
    /// The singleton this extension is keyed by (`'u'` in `u-islamcal`), as
    /// written.
    #[must_use]
    pub const fn singleton(self) -> char {
        self.singleton
    }

    /// The whole section including its singleton (`u-islamcal`).
    #[must_use]
    pub const fn as_str(self) -> &'a str {
        self.section
    }

    /// The subtags after the singleton, in order (`co`, `phonebk` for
    /// `u-co-phonebk`).
    ///
    /// Never empty: the `1*` in the production requires at least one.
    pub fn subtags(self) -> impl Iterator<Item = &'a str> {
        self.section
            .get(MARKER_PREFIX_WIDTH..)
            .into_iter()
            .flat_map(|subtags| subtags.split('-'))
    }
}

impl fmt::Display for Extension<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.section)
    }
}

impl AsRef<str> for Extension<'_> {
    fn as_ref(&self) -> &str {
        self.section
    }
}

/// An owning [`LanguageTag`].
///
/// The borrowing form is the working one — it copies nothing — but a caller
/// that stores a tag past the lifetime of the bytes it was read from needs the
/// tag to own them. This is that form: the same decomposition over a `String`,
/// with the trait set a map key or a sort key needs.
///
/// Its identity rules are [`LanguageTag`]'s, for the same reason: structural
/// equality, string-keyed [`Ord`] and [`Hash`], and therefore a lawful
/// [`Borrow<str>`] that lets `HashMap<LanguageTagBuf, _>` be probed with `&str`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageTagBuf {
    /// The tag exactly as supplied.
    tag: String,
    /// Which top-level alternative matched.
    form: TagForm,
    /// Where each section of the production sat.
    sections: Sections,
}

impl LanguageTagBuf {
    /// Parses `tag` against [`Profile::Rfc5646`] into an owning tag.
    ///
    /// # Errors
    ///
    /// The typed [`LanguageTagError`] [`parse`] would have returned.
    pub fn parse(tag: &str) -> Result<Self, LanguageTagError> {
        Self::parse_with(tag, Profile::Rfc5646)
    }

    /// Parses `tag` against `profile` into an owning tag.
    ///
    /// # Errors
    ///
    /// The typed [`LanguageTagError`] [`parse_with`] would have returned.
    pub fn parse_with(tag: &str, profile: Profile) -> Result<Self, LanguageTagError> {
        parse_with(tag, profile).map(|parsed| parsed.to_owned_tag())
    }

    /// The borrowing view, which is where every section accessor lives.
    #[must_use]
    pub fn as_language_tag(&self) -> LanguageTag<'_> {
        LanguageTag {
            tag: &self.tag,
            form: self.form,
            sections: self.sections,
        }
    }

    /// The tag exactly as supplied.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.tag
    }

    /// Consumes the tag, returning the string it owns.
    #[must_use]
    pub fn into_string(self) -> String {
        self.tag
    }

    /// Which top-level alternative matched.
    #[must_use]
    pub const fn form(&self) -> TagForm {
        self.form
    }

    /// The primary language subtag, when the form has one.
    #[must_use]
    pub fn primary_language(&self) -> Option<&str> {
        self.as_language_tag().primary_language()
    }

    /// The extended-language subtags, hyphen-joined, when present.
    #[must_use]
    pub fn extended_language(&self) -> Option<&str> {
        self.as_language_tag().extended_language()
    }

    /// The script subtag, when present.
    #[must_use]
    pub fn script(&self) -> Option<&str> {
        self.as_language_tag().script()
    }

    /// The region subtag, when present.
    #[must_use]
    pub fn region(&self) -> Option<&str> {
        self.as_language_tag().region()
    }

    /// The variant subtags in order of appearance.
    pub fn variants(&self) -> impl Iterator<Item = &str> {
        subtags_in(self.sections.variants.map(|span| span.of(&self.tag)))
    }

    /// The raw extension section, hyphen-joined, when present.
    #[must_use]
    pub fn extensions(&self) -> Option<&str> {
        self.as_language_tag().extensions()
    }

    /// The extension sections, one per singleton, in order.
    pub fn extensions_by_singleton(&self) -> impl Iterator<Item = Extension<'_>> {
        split_extensions(self.extensions())
    }

    /// The first extension section keyed by `singleton`.
    #[must_use]
    pub fn extension(&self, singleton: char) -> Option<Extension<'_>> {
        self.as_language_tag().extension(singleton)
    }

    /// The private-use section including its `x`/`X` marker.
    #[must_use]
    pub fn private_use(&self) -> Option<&str> {
        self.as_language_tag().private_use()
    }

    /// The private-use subtags after the `x`/`X` marker, in order.
    pub fn private_use_subtags(&self) -> impl Iterator<Item = &str> {
        subtags_in(
            self.private_use()
                .and_then(|section| section.get(MARKER_PREFIX_WIDTH..)),
        )
    }

    /// This tag rewritten in RFC 5646 §2.1.1 canonical case.
    #[must_use]
    pub fn canonical_case(&self) -> String {
        self.as_language_tag().canonical_case()
    }

    /// `true` when this tag is already spelled in §2.1.1 canonical case.
    #[must_use]
    pub fn is_canonical_case(&self) -> bool {
        self.as_language_tag().is_canonical_case()
    }

    /// This tag re-spelled in §2.1.1 canonical case, keeping the decomposition.
    ///
    /// Case is insignificant to every production, so the spans are unchanged by
    /// construction — the rewrite replaces bytes, never subtag boundaries.
    #[must_use]
    pub fn into_canonical_case(self) -> Self {
        let mut canonical = String::with_capacity(self.tag.len());
        write_canonical_case(&self.tag, &mut canonical);
        Self {
            tag: canonical,
            ..self
        }
    }
}

impl fmt::Display for LanguageTagBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.tag)
    }
}

impl AsRef<str> for LanguageTagBuf {
    fn as_ref(&self) -> &str {
        &self.tag
    }
}

impl Borrow<str> for LanguageTagBuf {
    fn borrow(&self) -> &str {
        &self.tag
    }
}

impl Hash for LanguageTagBuf {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
    }
}

impl Ord for LanguageTagBuf {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tag.cmp(&other.tag)
    }
}

impl PartialOrd for LanguageTagBuf {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for LanguageTagBuf {
    type Err = LanguageTagError;

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        Self::parse(tag)
    }
}

impl From<LanguageTag<'_>> for LanguageTagBuf {
    fn from(tag: LanguageTag<'_>) -> Self {
        tag.to_owned_tag()
    }
}

/// The subtags of one hyphen-joined run, or nothing when the run is absent.
///
/// A free function rather than a method because both [`LanguageTag`] and
/// [`LanguageTagBuf`] need it at *different* lifetimes: the borrowing form
/// yields slices of the input it was parsed from, the owning form slices of the
/// `String` it holds.
fn subtags_in(run: Option<&str>) -> impl Iterator<Item = &str> {
    run.into_iter().flat_map(|section| section.split('-'))
}

/// The extension sections of one hyphen-joined extension run, keyed by
/// singleton. The twin of [`subtags_in`], and a free function for the same
/// reason.
fn split_extensions(run: Option<&str>) -> impl Iterator<Item = Extension<'_>> {
    let mut rest = run.unwrap_or_default();
    core::iter::from_fn(move || next_extension(&mut rest))
}

/// Splits the leading `extension` section off a hyphen-joined extension run,
/// advancing `rest` past it.
///
/// The section ends immediately before the next one-character subtag, which can
/// only be the following singleton: every `extension` subtag is `2*8alphanum`.
fn next_extension<'a>(rest: &mut &'a str) -> Option<Extension<'a>> {
    if rest.is_empty() {
        return None;
    }
    let mut end = rest.len();
    let mut offset = 0usize;
    for (index, subtag) in rest.split('-').enumerate() {
        if index > 0 && subtag.len() == SINGLETON_LENGTH {
            // Back up over the hyphen that joined this singleton to the section
            // being split off, so neither side keeps a dangling separator.
            end = offset - 1;
            break;
        }
        offset += subtag.len() + 1;
    }
    let (section, tail) = rest.split_at(end);
    *rest = tail.strip_prefix('-').unwrap_or_default();
    let singleton = section.chars().next()?;
    Some(Extension { singleton, section })
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

/// Rewrites `tag` in the **RFC 5646 §2.1.1** canonical case convention.
///
/// # The rule
///
/// §2.1.1 is a recommendation about *presentation*, not part of the grammar:
/// "all subtags, including extension and private use subtags, use lowercase
/// letters with two exceptions: two-letter and four-letter subtags that neither
/// appear at the start of the tag nor occur after singletons. Such two-letter
/// subtags are all uppercase … and four-letter subtags are titlecase". So
/// `de-de` becomes `de-DE`, `zh-hant` becomes `zh-Hant`, and `EN-us` becomes
/// `en-US`.
///
/// # The two forms with their own rule
///
/// * A **grandfathered** tag keeps its *registered* spelling, which the §2.2.8
///   list holds verbatim: `I-ENOCHIAN` becomes `i-enochian`, `en-gb-oed`
///   becomes `en-GB-oed`, `SGN-be-fr` becomes `sgn-BE-FR`. These are canonical
///   by registration rather than by rule, so they are looked up, not derived.
/// * A **whole-tag private-use** form (`x-…`) lowercases entirely. It falls out
///   of the positional rule rather than needing a special case — the `x` marker
///   *is* a one-character subtag, so every subtag after it is "after a
///   singleton" — but it is worth stating, because the naive reading of "title-
///   case the four-letter subtag" would corrupt `x-gmeow-chinese-latn` into a
///   `Latn` that is not a script and never was. Private-use subtags are
///   case-insensitive and carry no title-case rule at any length.
///
/// The rewrite touches bytes only, never subtag boundaries, so the result is
/// well-formed under the same profile and re-parses to an equal tag. It is
/// idempotent.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::langtag::{Profile, canonical_case, canonical_case_with};
///
/// // The three conventions: language lower, region upper, script title.
/// assert_eq!(canonical_case("de-de")?, "de-DE");
/// assert_eq!(canonical_case("zh-hant")?, "zh-Hant");
/// assert_eq!(canonical_case("EN-us")?, "en-US");
///
/// // A grandfathered tag keeps its registered spelling.
/// assert_eq!(canonical_case("EN-GB-OED")?, "en-GB-oed");
/// assert_eq!(canonical_case("I-ENOCHIAN")?, "i-enochian");
///
/// // A whole-tag private-use form lowercases, and nothing in it is a script.
/// assert_eq!(canonical_case("X-GMEOW-CHINESE-LATN")?, "x-gmeow-chinese-latn");
///
/// // Long private-use subtags need the profile that admits them, and then
/// // come back byte-identical.
/// assert_eq!(
///     canonical_case_with("x-gmeow-norwegiannynorsk", Profile::Rfc5646PrivateUseRelaxed)?,
///     "x-gmeow-norwegiannynorsk"
/// );
/// # Ok::<(), purrdf_iri::langtag::LanguageTagError>(())
/// ```
///
/// # Errors
///
/// A typed [`LanguageTagError`]: there is no canonical case for a string that
/// is not a language tag, and inventing one would be the silent-drop bug in its
/// normalizing disguise.
pub fn canonical_case(tag: &str) -> Result<String, LanguageTagError> {
    canonical_case_with(tag, Profile::Rfc5646)
}

/// Rewrites `tag` in §2.1.1 canonical case, accepting whatever `profile` does.
///
/// [`canonical_case`] is this with [`Profile::Rfc5646`], and describes the rule.
/// A tag accepted only by [`Profile::ConcreteSyntaxLangtag`] has no RFC 5646
/// reading and therefore no sections; §2.1.1's rule is positional, so it still
/// applies, and it is applied exactly as written.
///
/// # Errors
///
/// A typed [`LanguageTagError`] naming the production that refused.
pub fn canonical_case_with(tag: &str, profile: Profile) -> Result<String, LanguageTagError> {
    Ok(parse_with(tag, profile)?.canonical_case())
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

    Ok(LanguageTag::decomposed(
        tag,
        TagForm::Langtag,
        Sections {
            language: Some(language),
            extlang,
            script,
            region,
            variants,
            extensions,
            private_use,
        },
    ))
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

/// The **registered** spelling of `tag`, when it is one of the 26 closed
/// `grandfathered` tags of §2.2.8.
///
/// Whole-tag comparison, so `zh-min` and `zh-min-nan` cannot shadow each other,
/// and case-insensitive, because RFC 5234 literals are. Returning the registry's
/// own spelling rather than a bool is what lets §2.1.1 canonical case *preserve*
/// these tags instead of deriving them: the registered forms are the canonical
/// ones by fiat, not by rule.
fn registered_grandfathered(tag: &str) -> Option<&'static str> {
    GRANDFATHERED
        .iter()
        .copied()
        .find(|candidate| candidate.eq_ignore_ascii_case(tag))
}

/// `true` when `tag` is one of the 26 closed `grandfathered` tags of §2.2.8.
fn is_grandfathered(tag: &str) -> bool {
    registered_grandfathered(tag).is_some()
}

/// The case RFC 5646 §2.1.1 asks of one subtag position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SubtagCase {
    /// Every character lowercase — the default, and the whole rule for the
    /// first subtag and for everything after a singleton.
    Lower,
    /// Every character uppercase — a two-character `region`.
    Upper,
    /// First character uppercase, rest lowercase — a four-character `script`.
    Title,
}

/// The §2.1.1 case convention for the subtag at `index`, given whether a
/// `singleton` (or the `x` private-use marker) has already been seen.
///
/// §2.1.1 states the rule *positionally* rather than by production — everything
/// is lowercase except "two-letter and four-letter subtags that neither appear
/// at the start of the tag nor occur after singletons", which are uppercase and
/// title-case respectively. That is exactly this function, and it is why the
/// rule applies to the raw subtag sequence with no parse tree in hand. The
/// carve-outs are not decoration: they are what keeps `x-gmeow-chinese-latn`
/// from acquiring a `Latn` that was never a script subtag, and what keeps the
/// four-character subtags of an extension (`de-DE-u-co-phonebk`) lowercase.
const fn subtag_case(index: usize, after_singleton: bool, length: usize) -> SubtagCase {
    if index == 0 || after_singleton {
        return SubtagCase::Lower;
    }
    match length {
        2 => SubtagCase::Upper,
        4 => SubtagCase::Title,
        _ => SubtagCase::Lower,
    }
}

/// One byte of a subtag, cased as `case` asks. `first` selects the title-case
/// pivot and is ignored by the other two conventions.
const fn cased_byte(byte: u8, case: SubtagCase, first: bool) -> u8 {
    match case {
        SubtagCase::Lower => byte.to_ascii_lowercase(),
        SubtagCase::Upper => byte.to_ascii_uppercase(),
        SubtagCase::Title if first => byte.to_ascii_uppercase(),
        SubtagCase::Title => byte.to_ascii_lowercase(),
    }
}

/// Appends the §2.1.1 canonical spelling of `tag` to `out`.
///
/// Only bytes change: the subtag boundaries are copied across untouched, which
/// is what makes the rewrite safe to apply to an already-parsed tag without
/// re-deriving its spans.
fn write_canonical_case(tag: &str, out: &mut String) {
    if let Some(registered) = registered_grandfathered(tag) {
        out.push_str(registered);
        return;
    }
    let mut after_singleton = false;
    for (index, subtag) in tag.split('-').enumerate() {
        if index > 0 {
            out.push('-');
        }
        let case = subtag_case(index, after_singleton, subtag.len());
        for (position, byte) in subtag.bytes().enumerate() {
            out.push(char::from(cased_byte(byte, case, position == 0)));
        }
        after_singleton = after_singleton || subtag.len() == SINGLETON_LENGTH;
    }
}

/// `true` when `tag` is already spelled in §2.1.1 canonical case. The same walk
/// as [`write_canonical_case`], comparing instead of appending.
fn canonical_case_holds(tag: &str) -> bool {
    if let Some(registered) = registered_grandfathered(tag) {
        return tag == registered;
    }
    let mut after_singleton = false;
    for (index, subtag) in tag.split('-').enumerate() {
        let case = subtag_case(index, after_singleton, subtag.len());
        let cased = subtag
            .bytes()
            .enumerate()
            .all(|(position, byte)| byte == cased_byte(byte, case, position == 0));
        if !cased {
            return false;
        }
        after_singleton = after_singleton || subtag.len() == SINGLETON_LENGTH;
    }
    true
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
        Extension, GRANDFATHERED, LanguageTagBuf, LanguageTagError, Profile, TagForm,
        canonical_case, canonical_case_with, is_well_formed, is_well_formed_with, parse,
        parse_with,
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

    /// §2.1.1's three conventions, each with a neighbour that must come back
    /// byte-identical. Normalization rewrites strings, so "it changed nothing"
    /// is as much a claim to prove as "it changed this".
    #[test]
    fn canonical_case_applies_the_section_2_1_1_conventions() {
        // (input, canonical spelling)
        let rewritten: &[(&str, &str)] = &[
            // region: two characters, uppercase.
            ("de-de", "de-DE"),
            ("EN-us", "en-US"),
            ("sr-latn-rs", "sr-Latn-RS"),
            // script: four characters, title case.
            ("zh-hant", "zh-Hant"),
            ("ZH-HANS-CN", "zh-Hans-CN"),
            // language: always lowercase, whatever its length.
            ("DE", "de"),
            ("ABCDEFGH", "abcdefgh"),
            // variants and extensions: lowercase, and a four-character variant
            // is not a script even though it is four characters.
            ("SL-ROZAJ-BISKE-1994", "sl-rozaj-biske-1994"),
            ("HY-LATN-IT-AREVELA", "hy-Latn-IT-arevela"),
            ("DE-DE-U-CO-PHONEBK", "de-DE-u-co-phonebk"),
            ("EN-US-U-ISLAMCAL", "en-US-u-islamcal"),
            // a three-digit region stays as written; digits have no case.
            ("ES-419", "es-419"),
        ];
        for (input, canonical) in rewritten {
            assert_eq!(
                canonical_case(input).as_deref(),
                Ok(*canonical),
                "{input:?}"
            );
            assert!(!parse(input).expect("well-formed").is_canonical_case());
        }

        // Already canonical: the rewrite must be the identity on these.
        for unchanged in [
            "en",
            "en-US",
            "zh-Hans-CN",
            "de-CH-x-phonebk",
            "i-enochian",
            "es-419",
            "sl-rozaj-biske-1994",
            "zh-cmn-Hans-CN-1901-u-islamcal-x-priv",
        ] {
            assert_eq!(
                canonical_case(unchanged).as_deref(),
                Ok(unchanged),
                "{unchanged:?} is already canonical"
            );
            assert!(
                parse(unchanged).expect("well-formed").is_canonical_case(),
                "{unchanged:?}"
            );
        }
    }

    /// Grandfathered tags are canonical by *registration*, so the rewrite looks
    /// them up rather than deriving them.
    #[test]
    fn canonical_case_restores_the_registered_grandfathered_spelling() {
        for (input, registered) in [
            ("I-ENOCHIAN", "i-enochian"),
            ("i-enochian", "i-enochian"),
            ("en-gb-oed", "en-GB-oed"),
            ("EN-GB-OED", "en-GB-oed"),
            ("SGN-be-fr", "sgn-BE-FR"),
            ("ART-LOJBAN", "art-lojban"),
            ("ZH-MIN-NAN", "zh-min-nan"),
        ] {
            assert_eq!(
                canonical_case(input).as_deref(),
                Ok(registered),
                "{input:?}"
            );
        }
        // Every registered spelling is its own canonical form, which is what
        // makes the lookup a fixed point rather than a second convention.
        for tag in GRANDFATHERED {
            assert_eq!(canonical_case(tag).as_deref(), Ok(tag), "{tag:?}");
            assert!(parse(tag).expect("grandfathered").is_canonical_case());
        }
    }

    /// The private-use space has no title-case rule at any subtag length, and
    /// the families below are published downstream in volume. A normalizer that
    /// "helpfully" title-cased a four-character private-use subtag would mangle
    /// every one of them while every other test stayed green.
    #[test]
    fn canonical_case_lowercases_private_use_and_title_cases_nothing_in_it() {
        for unchanged in [
            "x-gmeow-english",
            "x-gmeow-chinese-latn",
            "x-purrdf-english",
            "de-CH-x-phonebk",
            "en-x-ab",
            "en-x-abcd",
            "x-ab-cd",
        ] {
            assert_eq!(
                canonical_case(unchanged).as_deref(),
                Ok(unchanged),
                "{unchanged:?} must survive normalization byte-identical"
            );
        }
        // Mixed case inside the private-use space folds down, and only down.
        for (input, canonical) in [
            ("X-GMEOW-CHINESE-LATN", "x-gmeow-chinese-latn"),
            ("az-Arab-x-AZE-derbend", "az-Arab-x-aze-derbend"),
            ("DE-ch-X-PhoneBk", "de-CH-x-phonebk"),
        ] {
            assert_eq!(canonical_case(input).as_deref(), Ok(canonical), "{input:?}");
        }
        // The over-long families need the profile that admits them; §2.1.1 then
        // leaves them exactly as written, sixteen-character subtag and all.
        for over_ceiling in [
            "x-gmeow-norwegiannynorsk",
            "x-gmeow-westernfrisian",
            "x-purrdf-afrikaans",
            "x-purrdf-norwegiannynorsk",
        ] {
            assert_eq!(
                canonical_case_with(over_ceiling, Profile::Rfc5646PrivateUseRelaxed).as_deref(),
                Ok(over_ceiling),
                "{over_ceiling:?} must survive normalization byte-identical"
            );
        }
    }

    /// Normalization is a rewrite of an *accepted* tag: it never invents a
    /// canonical form for something the grammar refused, and never changes what
    /// the grammar accepts.
    #[test]
    fn canonical_case_refuses_exactly_what_the_grammar_refuses() {
        // (refused input, the error it must name, a neighbour that must
        // normalize)
        let cases: &[(&str, LanguageTagError, &str)] = &[
            ("", LanguageTagError::SubtagLengthZero, "en"),
            ("en--US", LanguageTagError::SubtagLengthZero, "en-US"),
            ("e", LanguageTagError::LanguageProductionUnmatched, "en"),
            ("de-419-DE", LanguageTagError::UnconsumedSubtag, "de-DE"),
            ("en-x", LanguageTagError::PrivateUseWithoutSubtag, "en-x-a"),
            (
                "x-purrdf-afrikaans",
                LanguageTagError::SubtagLengthOverEight,
                "x-purrdf-english",
            ),
        ];
        for (refused, expected, accepted) in cases {
            assert_eq!(canonical_case(refused), Err(*expected), "{refused:?}");
            assert!(canonical_case(accepted).is_ok(), "{accepted:?}");
        }
    }

    /// Idempotent, and a fixed point of the grammar: the canonical spelling is
    /// accepted by the same profile and decomposes into the same sections,
    /// because case is insignificant to every production.
    #[test]
    fn canonical_case_is_idempotent_and_stays_well_formed() {
        for tag in [
            "DE-de",
            "ZH-HANT",
            "en-us",
            "SL-IT-NEDIS",
            "zh-CMN-hans-CN",
            "AR-a-aaa-b-bbb-a-ccc",
            "X-WHATEVER",
            "I-ENOCHIAN",
        ] {
            let once = canonical_case(tag).expect("well-formed");
            let twice = canonical_case(&once).expect("canonical form stays well-formed");
            assert_eq!(once, twice, "{tag:?} must reach a fixed point");
            assert!(parse(&once).expect("well-formed").is_canonical_case());
            let original = parse(tag).expect("well-formed");
            let normalized = parse(&once).expect("well-formed");
            assert_eq!(original.form(), normalized.form(), "{tag:?}");
            assert_eq!(
                original.primary_language().map(str::to_ascii_lowercase),
                normalized.primary_language().map(str::to_ascii_lowercase),
                "{tag:?} must keep its decomposition"
            );
        }
    }

    #[test]
    fn extensions_group_under_their_singletons() {
        let tag = parse("en-US-u-islamcal-a-myext-t-en").expect("well-formed");
        let grouped = tag
            .extensions_by_singleton()
            .map(|extension| (extension.singleton(), extension.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            grouped,
            [('u', "u-islamcal"), ('a', "a-myext"), ('t', "t-en")]
        );
        assert_eq!(
            tag.extension('u')
                .map(|extension| extension.subtags().collect::<Vec<_>>()),
            Some(vec!["islamcal"])
        );
        assert_eq!(tag.extension('z'), None);

        // Multi-subtag extensions, and the singleton lookup that finds them.
        let multi = parse("de-DE-u-co-phonebk-nu-latn").expect("well-formed");
        let unicode = multi.extension('u').expect("a `u` extension");
        assert_eq!(unicode.as_str(), "u-co-phonebk-nu-latn");
        assert_eq!(
            unicode.subtags().collect::<Vec<_>>(),
            ["co", "phonebk", "nu", "latn"]
        );
        // `singleton` is an ABNF character range over both cases.
        assert_eq!(multi.extension('U'), Some(unicode));

        // No extensions at all is an empty iterator, not a one-item one.
        assert_eq!(
            parse("en-US")
                .expect("well-formed")
                .extensions_by_singleton()
                .count(),
            0
        );
        assert_eq!(
            parse("x-whatever")
                .expect("well-formed")
                .extensions_by_singleton()
                .count(),
            0
        );

        // §2.2.9 admits a repeated singleton, so the grouping must report it
        // rather than silently fold the two together.
        let duplicated = parse("ar-a-aaa-b-bbb-a-ccc").expect("well-formed");
        assert_eq!(
            duplicated
                .extensions_by_singleton()
                .map(Extension::as_str)
                .collect::<Vec<_>>(),
            ["a-aaa", "b-bbb", "a-ccc"]
        );
        assert_eq!(
            duplicated.extension('a').map(Extension::as_str),
            Some("a-aaa"),
            "the lookup reports the first, and says so"
        );
    }

    #[test]
    fn private_use_subtags_are_reported_one_by_one() {
        let trailing = parse("de-CH-x-phonebk").expect("well-formed");
        assert_eq!(trailing.private_use(), Some("x-phonebk"));
        assert_eq!(
            trailing.private_use_subtags().collect::<Vec<_>>(),
            ["phonebk"]
        );

        let whole = parse("x-gmeow-chinese-latn").expect("well-formed");
        assert_eq!(
            whole.private_use_subtags().collect::<Vec<_>>(),
            ["gmeow", "chinese", "latn"]
        );

        let relaxed = parse_with(
            "x-gmeow-norwegiannynorsk",
            Profile::Rfc5646PrivateUseRelaxed,
        )
        .expect("the relaxed profile admits the long subtag");
        assert_eq!(
            relaxed.private_use_subtags().collect::<Vec<_>>(),
            ["gmeow", "norwegiannynorsk"]
        );

        // The marker is never yielded, and a tag without the section yields
        // nothing rather than an empty subtag.
        assert_eq!(
            parse("en-US")
                .expect("well-formed")
                .private_use_subtags()
                .count(),
            0
        );
        assert_eq!(
            parse("en-a-bb")
                .expect("well-formed")
                .private_use_subtags()
                .count(),
            0
        );
    }

    #[test]
    fn the_owning_form_carries_the_borrowing_form_and_its_traits() {
        use std::collections::{BTreeMap, HashMap};

        let owned: LanguageTagBuf = "zh-Hans-CN-x-priv".parse().expect("well-formed");
        assert_eq!(owned.as_str(), "zh-Hans-CN-x-priv");
        assert_eq!(owned.form(), TagForm::Langtag);
        assert_eq!(owned.primary_language(), Some("zh"));
        assert_eq!(owned.script(), Some("Hans"));
        assert_eq!(owned.region(), Some("CN"));
        assert_eq!(owned.private_use_subtags().collect::<Vec<_>>(), ["priv"]);
        assert_eq!(owned.to_string(), "zh-Hans-CN-x-priv");
        assert_eq!(AsRef::<str>::as_ref(&owned), "zh-Hans-CN-x-priv");

        // The two forms agree section for section.
        let borrowed = parse("zh-Hans-CN-x-priv").expect("well-formed");
        assert_eq!(owned.as_language_tag(), borrowed);
        assert_eq!(borrowed.to_owned_tag(), owned);
        assert_eq!(LanguageTagBuf::from(borrowed), owned);

        // `Borrow<str>` is what makes a tag usable as a map key probed by a
        // plain string, which is the whole reason the owning form exists.
        let mut hashed = HashMap::new();
        hashed.insert(owned.clone(), 1_u8);
        assert_eq!(hashed.get("zh-Hans-CN-x-priv"), Some(&1));
        let mut ordered = BTreeMap::new();
        ordered.insert(owned, 1_u8);
        assert_eq!(ordered.get("zh-Hans-CN-x-priv"), Some(&1));
        let owned = LanguageTagBuf::parse("zh-Hans-CN-x-priv").expect("well-formed");
        assert_eq!(ordered.get(&owned), Some(&1));

        // `Ord` is the string order, so a sort is the spelling's sort.
        let mut tags = ["en-US", "de-DE", "zh-Hant"]
            .map(|tag| LanguageTagBuf::parse(tag).expect("well-formed"))
            .to_vec();
        tags.sort();
        assert_eq!(
            tags.iter().map(LanguageTagBuf::as_str).collect::<Vec<_>>(),
            ["de-DE", "en-US", "zh-Hant"]
        );

        // Normalization on the owning form keeps the decomposition.
        let folded = LanguageTagBuf::parse("ZH-hant-cn").expect("well-formed");
        assert!(!folded.is_canonical_case());
        assert_eq!(folded.canonical_case(), "zh-Hant-CN");
        let canonical = folded.into_canonical_case();
        assert_eq!(canonical.as_str(), "zh-Hant-CN");
        assert_eq!(canonical.script(), Some("Hant"));
        assert_eq!(canonical.region(), Some("CN"));
        assert_eq!(
            canonical,
            LanguageTagBuf::parse("zh-Hant-CN").expect("well-formed"),
            "normalizing must land on the record a fresh parse gives"
        );

        // And it refuses what the borrowing form refuses, by the same error.
        assert_eq!(
            "de-419-DE".parse::<LanguageTagBuf>(),
            Err(LanguageTagError::UnconsumedSubtag)
        );
        assert!("de-DE".parse::<LanguageTagBuf>().is_ok());
        assert_eq!(
            LanguageTagBuf::parse("x-purrdf-afrikaans"),
            Err(LanguageTagError::SubtagLengthOverEight)
        );
        assert!(
            LanguageTagBuf::parse_with("x-purrdf-afrikaans", Profile::Rfc5646PrivateUseRelaxed)
                .is_ok()
        );
    }

    /// Every error renders the same text through both doors, so a consumer that
    /// can only carry a `&'static str` reports the same reason as one that can
    /// format.
    #[test]
    fn the_error_message_is_what_display_renders() {
        for error in [
            LanguageTagError::SubtagLengthZero,
            LanguageTagError::SubtagLengthOverEight,
            LanguageTagError::LanguageProductionUnmatched,
            LanguageTagError::ExtlangRepetitionExceeded,
            LanguageTagError::SingletonWithoutSubtag,
            LanguageTagError::PrivateUseWithoutSubtag,
            LanguageTagError::UnconsumedSubtag,
            LanguageTagError::TerminalPrimaryNotAlpha,
            LanguageTagError::TerminalSubtagNotAlphanum,
        ] {
            assert_eq!(error.to_string(), error.message(), "{error:?}");
            assert!(!error.message().is_empty(), "{error:?}");
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
