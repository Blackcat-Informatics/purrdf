// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed ShEx parse failures.
//!
//! Per the repo `no-optionality / hard-fail` doctrine, every malformed schema
//! is a typed [`ShexError`] — never a degraded fallback, never a panic. The
//! parsers are fuzz-safe: arbitrary input yields `Err`, not `panic!`.

use core::fmt;

mod member;
pub use member::{
    JsonContext, JsonExpectation, JsonMember, JsonMemberReason, JsonOffered, JsonType,
};
mod facet;
pub use facet::{
    AdmittedNumericFacet, FacetKind, FacetReason, NumericFacetOperand, UnsignedFacetOperand,
};
mod grammar;
pub use grammar::{GrammarExpectation, GrammarReason};
mod language;
pub use language::{LanguageMember, SHEXJ_LANGUAGE_POLICY, SHEXJ_LANGUAGE_PROFILE};
mod shape_map;
pub use shape_map::{
    MapExpectation, ShapeMapContext, ShapeMapLanguageRefusal, ShapeMapPosition, ShapeMapReason,
};
mod lexical;
pub use lexical::{LexicalContext, LexicalReason, RepeatOperation};

/// Why a ShExC or ShExJ document failed to parse into a
/// [`crate::ast::Schema`].
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShexError {
    /// Parsing cannot enter the named construct without consuming the shared
    /// physical stack margin. This is a resource refusal, not invalid syntax.
    StackMargin {
        /// The recursive parser construct whose next frame was refused.
        construct: &'static str,
    },
    /// The ShExC byte stream could not be tokenized. Carries its native reason
    /// and the byte offset at which the lexer gave up.
    Lex {
        /// What the lexer objected to.
        reason: LexicalReason,
        /// Byte offset of the offending input.
        at: usize,
    },
    /// A token-stream grammar admission refusal with original native operands.
    Grammar {
        /// The grammar requirement and original encountered tokens.
        reason: Box<GrammarReason>,
        /// The historical parser byte coordinate.
        at: usize,
    },
    /// A facet conflict or integer/datatype admission refusal.
    Facet {
        /// The original native evidence and cause.
        reason: Box<FacetReason>,
        /// The historical parser byte coordinate.
        at: usize,
    },
    /// A contextual, semantic, resource or exact-schema alignment refusal from
    /// ShExC or ShExJ. Carries a human reason
    /// and the byte offset of the offending token (best-effort).
    Syntax {
        /// What the parser objected to.
        reason: String,
        /// Byte offset of the offending token.
        at: usize,
    },
    /// A shape-map grammar refusal with original scalar and UTF-8 byte positions.
    ShapeMapSyntax {
        /// The original native reason and lexical operands.
        reason: ShapeMapReason,
        /// The true UTF-8 byte position in the original input.
        at: usize,
        /// The original decoded-scalar cursor, retained independently.
        scalar_offset: usize,
    },
    /// An IRI could not be resolved against the schema base (delegated to
    /// `purrdf-iri`).
    Iri {
        /// The rejected lexical form.
        lexical: String,
        /// The underlying resolution failure.
        reason: purrdf_iri::IriError,
    },
    /// A native refusal from ShExJ schema materialization.
    JsonMember(Box<JsonMemberReason>),
    /// The shared JSON reader's original ShExJ ingress refusal, including its
    /// native kind, byte offset and nested lexical or storage fields.
    JsonRead(purrdf_lex::json::Error),
    /// A language tag or stem failed the shared concrete-syntax profile.
    LanguageTag {
        /// The original member position, distinguishing tags from stems.
        member: LanguageMember,
        /// The owned original value, without normalization or case folding.
        value: String,
        /// The shared language parser's original cause.
        cause: purrdf_iri::langtag::LanguageTagError,
    },
    /// A parsed JSON value does not match the ShExJ object model.
    Shexj(String),
    /// An `IMPORT`ed schema could not be resolved by the caller-supplied
    /// import resolver (no ambient I/O — resolution is injected). Carries the
    /// import IRI and the concrete cause the resolver reported, so a read or
    /// parse failure behind an import surfaces its real reason rather than a
    /// vague "unresolved import".
    Import {
        /// The import IRI that failed to resolve.
        iri: String,
        /// The concrete failure the resolver surfaced for `iri`.
        cause: Box<Self>,
    },
    /// Two schemas in the same import closure declare the same shape label
    /// with conflicting definitions.
    ImportConflict(String),
    /// A shape map names a shape the schema does not declare. Carries the
    /// selector as the ShapeMap grammar writes it (`START` or `<label>`).
    ///
    /// Neither the ShapeMap specification nor ShEx 2.1 defines this case: §5.7's
    /// Shape Expression Reference Requirement binds a `shapeExprRef` written
    /// *inside* a schema (which [`crate::check_structure`] enforces), and
    /// `satisfies` is defined only where "se2 is the shape expression having se
    /// as id" — with no such expression the relation is undefined, not false.
    /// The repository's no-optionality/hard-fail doctrine therefore refuses it
    /// rather than answering `nonconformant`, which would state a finding about
    /// the DATA that the data had no part in.
    UnknownShape(String),
}

impl purrdf_hash::error::StructuredError for ShexError {
    purrdf_hash::error_visit! {
        StackMargin { construct } => "shex-stack-margin" { "construct" => Code(construct) },
        Lex { reason, at } => "shex-lex" { "reason" => Cause(reason), "at" => Unsigned(*at as u128) },
        Facet { reason, at } => "shex-syntax" { "reason" => Cause(&**reason), "at" => Unsigned(*at as u128) },
        Grammar { reason, at } => "shex-syntax" { "reason" => Cause(&**reason), "at" => Unsigned(*at as u128) },
        Syntax { reason, at } => "shex-syntax" { "reason" => Text(reason), "at" => Unsigned(*at as u128) },
        ShapeMapSyntax { reason, at, scalar_offset } => "shex-shape-map-syntax" {
            "reason" => Cause(reason), "at" => ByteCount(*at as u128), "scalar_offset" => Unsigned(*scalar_offset as u128)
        },
        Iri { lexical, reason } => "shex-iri" { "lexical" => Text(lexical), "cause" => Cause(reason) },
        JsonRead(cause) => "shex-json-read" { "cause" => Cause(cause) },
        LanguageTag { member, value, cause } => "shex-json-language-tag" {
            "member" => Record(member), "value" => Text(value),
            "profile" => Code(SHEXJ_LANGUAGE_POLICY), "cause" => Cause(cause)
        },
        JsonMember(reason) => "shex-json" { "reason" => Cause(&**reason) },
        Shexj(reason) => "shex-json" { "reason" => Text(reason) },
        Import { iri, cause } => "shex-import" { "iri" => Iri(iri), "cause" => Cause(&**cause) },
        ImportConflict(label) => "shex-import-conflict" { "label" => Text(label) },
        UnknownShape(selector) => "shex-unknown-shape" { "selector" => Text(selector) },
    }
}

purrdf_lex::constructors! {
    impl ShexError {
        /// Construct a [`ShexError::Syntax`] at a byte offset.
        pub fn syntax(reason, at: usize) -> Self::Syntax { .. };

        /// Construct a [`ShexError::Shexj`] from any displayable reason.
        pub fn shexj(reason) -> Self::Shexj;

        /// Construct a [`ShexError::ImportConflict`] for a conflicting shape label.
        pub fn import_conflict(label) -> Self::ImportConflict;

        /// Construct a [`ShexError::UnknownShape`] from a rendered shape selector.
        pub fn unknown_shape(selector) -> Self::UnknownShape;
    }
}

impl ShexError {
    /// Construct a native lexical refusal at its original byte offset.
    #[must_use]
    pub fn lex(reason: LexicalReason, at: usize) -> Self {
        Self::Lex { reason, at }
    }

    /// Admit every recursive front-end at the workspace's one stack boundary.
    pub(crate) fn admit_stack(construct: &'static str) -> Result<()> {
        if purrdf_stack::is_low() {
            Err(Self::StackMargin { construct })
        } else {
            Ok(())
        }
    }

    /// Construct a [`ShexError::Iri`] from a `purrdf-iri` failure, prefixing the
    /// reason with that crate's [`diagnostic_code`].
    ///
    /// Every IRI failure ShEx reports comes through here, so "a relative IRI
    /// reference with no base in scope" is spelled `iri-relative-no-base` in a ShEx
    /// diagnostic exactly as it is in an RDF or SPARQL one — ShEx does not get its
    /// own vocabulary for a condition the whole workspace shares.
    ///
    /// [`diagnostic_code`]: purrdf_iri::IriError::diagnostic_code
    ///
    /// # Examples
    ///
    /// ```
    /// use purrdf_shex::ShexError;
    ///
    /// let cause = purrdf_iri::BaseScope::empty().resolve("knows").unwrap_err();
    /// let err = ShexError::iri("knows", &cause);
    /// assert!(err.to_string().contains("iri-relative-no-base"));
    /// ```
    pub fn iri(lexical: impl Into<String>, cause: &purrdf_iri::IriError) -> Self {
        Self::Iri {
            lexical: lexical.into(),
            reason: cause.clone(),
        }
    }

    /// Construct a [`ShexError::Import`] for an unresolvable import IRI,
    /// preserving the concrete `cause` the resolver reported.
    pub fn import(iri: impl Into<String>, cause: Self) -> Self {
        Self::Import {
            iri: iri.into(),
            cause: Box::new(cause),
        }
    }

    /// The byte offset the failure was reported at, for the position-bearing
    /// variants ([`Lex`](Self::Lex), [`Grammar`](Self::Grammar),
    /// [`Facet`](Self::Facet), [`Syntax`](Self::Syntax) and
    /// [`ShapeMapSyntax`](Self::ShapeMapSyntax)). `None` for the
    /// others — in particular [`Import`](Self::Import), whose offset (if any)
    /// belongs to a *different* source document than the one being parsed.
    #[must_use]
    pub fn byte_offset(&self) -> Option<usize> {
        match self {
            Self::Lex { at, .. }
            | Self::Grammar { at, .. }
            | Self::Facet { at, .. }
            | Self::Syntax { at, .. }
            | Self::ShapeMapSyntax { at, .. } => Some(*at),
            Self::Iri { .. }
            | Self::StackMargin { .. }
            | Self::JsonMember(_)
            | Self::JsonRead(_)
            | Self::LanguageTag { .. }
            | Self::Shexj(_)
            | Self::Import { .. }
            | Self::ImportConflict(_)
            | Self::UnknownShape(_) => None,
        }
    }

    /// Resolve this error's byte offset to a 1-based source [`Position`] against
    /// the original input text.
    ///
    /// The lexer keeps a cheap byte offset on the happy path; the line/column
    /// table is built here, on the error path only. `None` for variants without
    /// a byte offset in this document.
    ///
    /// [`Position`]: purrdf_iri::Position
    #[must_use]
    pub fn locate(&self, src: &str) -> Option<purrdf_iri::Position> {
        self.byte_offset()
            .map(|at| purrdf_iri::LineIndex::new(src).locate(src, at))
    }
}

impl fmt::Display for ShexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StackMargin { construct } => {
                write!(
                    f,
                    "ShEx parsing refused {construct}: physical stack margin exhausted"
                )
            }
            Self::Lex { reason, at } => write!(f, "ShExC lex error at byte {at}: {reason}"),
            Self::ShapeMapSyntax { reason, at, .. } => {
                write!(f, "ShExC syntax error at byte {at}: {reason}")
            }
            Self::Facet { reason, at } => write!(f, "ShExC syntax error at byte {at}: {reason}"),
            Self::Grammar { reason, at } => write!(f, "ShExC syntax error at byte {at}: {reason}"),
            Self::Syntax { reason, at } => {
                write!(f, "ShExC syntax error at byte {at}: {reason}")
            }
            Self::Iri { lexical, reason } => {
                write!(
                    f,
                    "invalid IRI {lexical:?}: {}: {reason}",
                    reason.diagnostic_code()
                )
            }
            Self::JsonRead(cause) => write!(f, "ShExJ error: invalid JSON: {cause}"),
            Self::LanguageTag {
                member,
                value,
                cause,
            } => write!(
                f,
                "ShExJ error: {member}: malformed language tag {value:?}: {cause} [{}]",
                cause.diagnostic_code()
            ),
            Self::JsonMember(reason) => write!(f, "ShExJ error: {reason}"),
            Self::Shexj(reason) => write!(f, "ShExJ error: {reason}"),
            Self::Import { iri, cause } => write!(f, "unresolved IMPORT <{iri}>: {cause}"),
            Self::ImportConflict(label) => {
                write!(f, "conflicting redefinition of shape {label}")
            }
            Self::UnknownShape(selector) => write!(
                f,
                "the shape map names the shape {selector}, which the schema does not declare"
            ),
        }
    }
}

// `Debug` mirrors `Display` so test failures print the human-readable reason
// rather than a struct dump (matches the `purrdf-iri`/`purrdf-xsd` convention).
impl fmt::Debug for ShexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

purrdf_lex::error_source!(ShexError => {
    Self::Facet { reason, .. } => &**reason,
    Self::Grammar { reason, .. } => &**reason,
    Self::ShapeMapSyntax { reason, .. } => reason,
    Self::JsonMember(reason) => &**reason,
    Self::JsonRead(cause) => cause,
    Self::LanguageTag { cause, .. } => cause,
    Self::Lex { reason, .. } => reason,
    Self::Iri { reason, .. } => reason,
    Self::Import { cause, .. } => &**cause,
});

/// Convenience alias for fallible ShEx parse operations.
pub type Result<T> = core::result::Result<T, ShexError>;

/// Why a textual shape-map validation under a native XPath law could not finish.
/// Pattern-language errors remain conformance findings, not variants here.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum XPathValidationError {
    /// Parsing or admitting the requested shape map failed.
    ShapeMap(ShexError),
    /// Native pattern execution or construction was operationally refused.
    Regex(purrdf_core::xsd_regex::xpath::Error),
}

impl purrdf_hash::error::StructuredError for XPathValidationError {
    purrdf_hash::error_visit! {
        ShapeMap(cause) => "shex-xpath-shape-map" { "cause" => Cause(cause) },
        Regex(cause) => "shex-xpath-regex" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(XPathValidationError {
    ShapeMap(ShexError), Regex(purrdf_core::xsd_regex::xpath::Error)
});

impl fmt::Display for XPathValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShapeMap(error) => error.fmt(f),
            Self::Regex(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for XPathValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ShapeMap(error) => Some(error),
            Self::Regex(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_offset_only_for_positional_variants() {
        assert_eq!(
            ShexError::lex(LexicalReason::EmptyRegex, 5).byte_offset(),
            Some(5)
        );
        assert_eq!(ShexError::syntax("y", 9).byte_offset(), Some(9));
        assert_eq!(ShexError::shexj("bad").byte_offset(), None);
        assert_eq!(
            ShexError::import("http://example.org/s", ShexError::syntax("z", 1)).byte_offset(),
            None
        );
    }

    #[test]
    fn locate_resolves_line_and_column() {
        let src = "PREFIX ex: <http://example.org/>\n<S> {\n  ex:p .\n}";
        let at = src.find("ex:p").unwrap();
        let pos = ShexError::syntax("unexpected", at).locate(src).unwrap();
        assert_eq!((pos.line, pos.column), (3, 3));
        assert!(ShexError::shexj("bad").locate(src).is_none());
    }
}
