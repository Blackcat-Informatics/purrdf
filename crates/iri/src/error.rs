// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed parse/resolution failures.
//!
//! Per the repo `no-optionality / hard-fail` doctrine, every malformed input is a
//! typed [`IriError`] — never a degraded fallback, never a silent default. The
//! variants are deliberately specific so callers (and conformance vectors) can
//! assert *why* a string was rejected, not merely that it was.

use core::fmt;

/// The remedy for [`IriError::NoBase`].
///
/// Each `REMEDY_*` constant is the **single owner** of its wording: both
/// [`IriError::remedy_hint`] and the [`Display`](fmt::Display) rendering read it
/// from here, so the accessor and the message cannot drift apart the way they did
/// when each spelled its own prose.
const REMEDY_NO_BASE: &str = "add a base to the document (`@base`/`BASE` in \
     Turtle-family syntaxes, `xml:base` in RDF/XML) or pass a base IRI to the API";

/// The remedy for [`IriError::NotAbsoluteByGrammar`].
const REMEDY_NOT_ABSOLUTE_BY_GRAMMAR: &str = "write the IRI in absolute form; this \
     syntax admits no relative IRI reference, so supplying a base will not help";

/// The remedy for [`IriError::NonAbsoluteBase`].
const REMEDY_NON_ABSOLUTE_BASE: &str =
    "supply a base IRI that has a scheme, e.g. `http://example.org/dir/`";

/// The remedy for [`IriError::MissingScheme`].
const REMEDY_MISSING_SCHEME: &str = "write the IRI in absolute form, with a scheme";

/// Why an IRI/URI string (or a reference-resolution / CURIE operation) failed.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IriError {
    /// The string is empty where a non-empty IRI/URI was required.
    Empty,
    /// A scheme was required (e.g. resolving against a base that has no scheme,
    /// or validating an absolute URI) but none was present.
    MissingScheme,
    /// The scheme is present but malformed: it must match `ALPHA *( ALPHA / DIGIT
    /// / "+" / "-" / "." )` (RFC-3986 §3.1). Carries the offending scheme text.
    BadScheme(String),
    /// A percent-encoding triplet (`%` `HEXDIG` `HEXDIG`) is truncated or contains
    /// a non-hex digit. Carries the byte offset of the offending `%`.
    BadPercentEncoding(usize),
    /// A character outside the permitted grammar appeared in a component. Carries
    /// the offending `char` and its byte offset.
    DisallowedChar(char, usize),
    /// The authority/host component is malformed (e.g. an unterminated IPv6
    /// literal `[...]`). Carries a short reason.
    BadAuthority(String),
    /// Reference resolution was asked to produce an absolute IRI from a base that
    /// is itself not absolute (has no scheme) — RFC-3986 §5.1 requires an absolute
    /// base. Carries the base text.
    NonAbsoluteBase(String),
    /// A **relative** IRI reference was encountered in a grammar that permits one,
    /// but no base IRI was in scope to resolve it against. Carries the offending
    /// reference verbatim.
    ///
    /// This is fixable by supplying a base (an in-document `@base`/`BASE`/`xml:base`
    /// directive, or a base passed to the API). PurRDF never invents one: deriving a
    /// base from a retrieval IRI would break byte determinism, diverge across
    /// surfaces that have no retrieval IRI (stdin, wasm, the C ABI), and leak local
    /// filesystem paths into published RDF.
    NoBase {
        /// The relative reference that could not be resolved.
        reference: String,
    },
    /// A **relative** IRI reference was encountered in a grammar whose syntax admits
    /// only absolute IRIs (N-Triples, N-Quads, TriX, `HexTuples`). Carries the
    /// offending reference verbatim.
    ///
    /// Unlike [`NoBase`](Self::NoBase), this is **not** fixable by supplying a base:
    /// the document is invalid for its own grammar, and a base is never applied.
    NotAbsoluteByGrammar {
        /// The relative reference the grammar does not admit.
        reference: String,
    },
}

impl IriError {
    /// The byte offset the failure was reported at, for the offset-bearing
    /// variants ([`BadPercentEncoding`](Self::BadPercentEncoding)/
    /// [`DisallowedChar`](Self::DisallowedChar)). `None` for the whole-string
    /// variants that are not tied to a single byte position.
    #[must_use]
    pub fn byte_offset(&self) -> Option<usize> {
        match self {
            Self::BadPercentEncoding(at) | Self::DisallowedChar(_, at) => Some(*at),
            Self::Empty
            | Self::MissingScheme
            | Self::BadScheme(_)
            | Self::BadAuthority(_)
            | Self::NonAbsoluteBase(_)
            | Self::NoBase { .. }
            | Self::NotAbsoluteByGrammar { .. } => None,
        }
    }

    /// The stable, machine-readable diagnostic code for this failure.
    ///
    /// This function is the **single owner** of these strings for the whole
    /// workspace: every crate that reports an IRI failure must route through it
    /// rather than spelling a code inline, so the codes cannot drift apart per
    /// codec. The mapping is total — adding an [`IriError`] variant without a code
    /// is a compile error here, not a silently missing code at a call site.
    ///
    /// The two base-related codes are deliberately distinct because their remedies
    /// are: [`NoBase`](Self::NoBase) is fixed by supplying a base, and
    /// [`NotAbsoluteByGrammar`](Self::NotAbsoluteByGrammar) is not.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_iri::IriError;
    ///
    /// assert_eq!(
    ///     IriError::NoBase { reference: "foo".to_owned() }.diagnostic_code(),
    ///     "iri-relative-no-base"
    /// );
    /// assert_eq!(
    ///     IriError::NotAbsoluteByGrammar { reference: "foo".to_owned() }.diagnostic_code(),
    ///     "iri-not-absolute-by-grammar"
    /// );
    /// ```
    #[must_use]
    pub fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::Empty => "iri-empty",
            Self::MissingScheme => "iri-missing-scheme",
            Self::BadScheme(_) => "iri-bad-scheme",
            Self::BadPercentEncoding(_) => "iri-bad-percent-encoding",
            Self::DisallowedChar(_, _) => "iri-disallowed-char",
            Self::BadAuthority(_) => "iri-bad-authority",
            Self::NonAbsoluteBase(_) => "iri-non-absolute-base",
            Self::NoBase { .. } => "iri-relative-no-base",
            Self::NotAbsoluteByGrammar { .. } => "iri-not-absolute-by-grammar",
        }
    }

    /// Actionable guidance for the failures a user can actually act on, or `None`
    /// where the only remedy is "fix the malformed string" and the message already
    /// pinpoints the offending byte.
    ///
    /// The [`Display`](core::fmt::Display) rendering **appends exactly this string**
    /// (after `"; "`) for every variant that has one, reading it from the same
    /// `REMEDY_*` constant this accessor returns — so `{err}` is self-sufficient at
    /// every consumer, and the two can never drift. Use this accessor only when a
    /// diagnostic renders the remedy in its own field; concatenating it onto `{err}`
    /// as well would print it twice.
    #[must_use]
    pub fn remedy_hint(&self) -> Option<&'static str> {
        match self {
            Self::NoBase { .. } => Some(REMEDY_NO_BASE),
            Self::NotAbsoluteByGrammar { .. } => Some(REMEDY_NOT_ABSOLUTE_BY_GRAMMAR),
            Self::NonAbsoluteBase(_) => Some(REMEDY_NON_ABSOLUTE_BASE),
            Self::MissingScheme => Some(REMEDY_MISSING_SCHEME),
            Self::Empty
            | Self::BadScheme(_)
            | Self::BadPercentEncoding(_)
            | Self::DisallowedChar(_, _)
            | Self::BadAuthority(_) => None,
        }
    }

    /// Write the *condition* — what went wrong — without the remedy.
    ///
    /// [`Display`](fmt::Display) is exactly this followed by
    /// [`remedy_hint`](Self::remedy_hint); splitting them is what keeps the message
    /// and the hint derived from one string per variant.
    fn write_condition(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty IRI/URI string"),
            Self::MissingScheme => f.write_str("missing scheme"),
            Self::BadScheme(s) => write!(f, "malformed scheme: {s:?}"),
            Self::BadPercentEncoding(at) => {
                write!(f, "malformed percent-encoding at byte {at}")
            }
            Self::DisallowedChar(c, at) => {
                write!(f, "disallowed character {c:?} at byte {at}")
            }
            Self::BadAuthority(why) => write!(f, "malformed authority: {why}"),
            Self::NonAbsoluteBase(b) => {
                write!(f, "base IRI is not absolute (no scheme): {b:?}")
            }
            Self::NoBase { reference } => {
                write!(
                    f,
                    "relative IRI reference {reference:?} has no base IRI in scope"
                )
            }
            Self::NotAbsoluteByGrammar { reference } => {
                write!(
                    f,
                    "relative IRI reference {reference:?} is not permitted by this syntax"
                )
            }
        }
    }

    /// Resolve this error's byte offset to a 1-based source [`Position`] against
    /// the lexical form the error came from. `None` for variants without a
    /// single offset.
    ///
    /// [`Position`]: crate::Position
    #[must_use]
    pub fn locate(&self, src: &str) -> Option<crate::Position> {
        self.byte_offset()
            .map(|at| crate::LineIndex::new(src).locate(src, at))
    }
}

impl fmt::Display for IriError {
    /// The condition, then the remedy — `"<what went wrong>; <what to do>"`.
    ///
    /// The remedy is part of the message rather than a field a consumer has to
    /// remember to fetch: `crates/shex`, `crates/sparql-algebra` and the text codecs
    /// all render `{code}: {err}`, so anything not in `{err}` reaches no user.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_condition(f)?;
        if let Some(remedy) = self.remedy_hint() {
            write!(f, "; {remedy}")?;
        }
        Ok(())
    }
}

// `Debug` mirrors `Display` so test failures print the human-readable reason
// rather than a struct dump (matches the `purrdf-xsd` `XsdError` convention).
impl fmt::Debug for IriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for IriError {}

/// Convenience alias for fallible IRI operations.
pub type Result<T> = core::result::Result<T, IriError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// One sample of **every** [`IriError`] variant.
    ///
    /// The exhaustive `match` below is the enforcement: adding a variant stops this
    /// compiling until the new arm is written, and the arm cannot be reached unless
    /// the sample is in `all`. Without it, a new variant would silently escape every
    /// total-mapping test in this module.
    fn every_variant() -> Vec<IriError> {
        let all = vec![
            IriError::Empty,
            IriError::MissingScheme,
            IriError::BadScheme("1x".to_owned()),
            IriError::BadPercentEncoding(0),
            IriError::DisallowedChar(' ', 0),
            IriError::BadAuthority("why".to_owned()),
            IriError::NonAbsoluteBase("/a".to_owned()),
            IriError::NoBase {
                reference: "foo".to_owned(),
            },
            IriError::NotAbsoluteByGrammar {
                reference: "foo".to_owned(),
            },
        ];
        let mut seen = 0usize;
        for err in &all {
            match err {
                IriError::Empty
                | IriError::MissingScheme
                | IriError::BadScheme(_)
                | IriError::BadPercentEncoding(_)
                | IriError::DisallowedChar(_, _)
                | IriError::BadAuthority(_)
                | IriError::NonAbsoluteBase(_)
                | IriError::NoBase { .. }
                | IriError::NotAbsoluteByGrammar { .. } => seen += 1,
            }
        }
        assert_eq!(seen, all.len());
        all
    }

    #[test]
    fn byte_offset_only_for_offset_variants() {
        assert_eq!(IriError::BadPercentEncoding(4).byte_offset(), Some(4));
        assert_eq!(IriError::DisallowedChar(' ', 6).byte_offset(), Some(6));
        assert_eq!(IriError::Empty.byte_offset(), None);
        assert_eq!(IriError::MissingScheme.byte_offset(), None);
    }

    /// Every variant must have its OWN code: a duplicate would silently merge two
    /// distinct conditions at every consumer that switches on the code.
    #[test]
    fn diagnostic_codes_are_distinct_and_kebab_prefixed() {
        let all = every_variant();
        let mut codes: Vec<&str> = all.iter().map(IriError::diagnostic_code).collect();
        let total = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), total, "duplicate diagnostic code: {codes:?}");
        assert!(codes.iter().all(|c| c.starts_with("iri-")));
    }

    /// The two base conditions differ in whether supplying a base is the remedy, so
    /// their codes and hints must not be interchangeable.
    #[test]
    fn base_conditions_have_honest_codes_and_remedies() {
        let no_base = IriError::NoBase {
            reference: "foo".to_owned(),
        };
        let by_grammar = IriError::NotAbsoluteByGrammar {
            reference: "foo".to_owned(),
        };
        assert_eq!(no_base.diagnostic_code(), "iri-relative-no-base");
        assert_eq!(by_grammar.diagnostic_code(), "iri-not-absolute-by-grammar");

        // Both messages name the offending reference verbatim.
        assert!(format!("{no_base}").contains("\"foo\""));
        assert!(format!("{by_grammar}").contains("\"foo\""));

        // "add a base" is the remedy for exactly one of them.
        assert!(no_base.remedy_hint().expect("hint").contains("@base"));
        assert!(
            by_grammar
                .remedy_hint()
                .expect("hint")
                .contains("will not help")
        );
        assert_eq!(IriError::Empty.remedy_hint(), None);
    }

    /// The contract `remedy_hint`'s rustdoc states: `{err}` is self-sufficient.
    ///
    /// Every consumer in this workspace renders `{code}: {err}` and never reads
    /// `remedy_hint` — `crates/shex/src/error.rs`, `crates/sparql-algebra/src/parser.rs`
    /// and `crates/rdf/src/native_codecs/text_parse.rs` among them — so a hint that
    /// lives only in the accessor reaches no user at all.
    #[test]
    fn display_ends_with_the_remedy_for_every_hint_bearing_variant() {
        let mut with_hints = 0usize;
        for err in every_variant() {
            let rendered = format!("{err}");
            if let Some(hint) = err.remedy_hint() {
                with_hints += 1;
                assert!(
                    rendered.ends_with(hint),
                    "{}: {rendered:?} does not end with its remedy {hint:?}",
                    err.diagnostic_code()
                );
                // …and the condition survives in front of it, rather than the
                // message having been replaced by the hint.
                assert!(
                    rendered.len() > hint.len() + 2,
                    "{}: message is nothing but its remedy",
                    err.diagnostic_code()
                );
            } else {
                // A hintless variant must not grow a trailing "; …" clause that no
                // accessor owns.
                assert!(
                    !rendered.contains("; "),
                    "{}: {rendered:?} carries unowned guidance",
                    err.diagnostic_code()
                );
            }
        }
        assert_eq!(with_hints, 4, "the hint-bearing variant set changed");
    }

    #[test]
    fn locate_resolves_offset() {
        let src = "http://example.org/a b";
        let at = src.find(' ').unwrap();
        let pos = IriError::DisallowedChar(' ', at).locate(src).unwrap();
        assert_eq!((pos.line, pos.column), (1, at as u32 + 1));
        assert!(IriError::Empty.locate(src).is_none());
    }
}
