// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native shape-map grammar refusals and original lexical operands.

use core::fmt;
use purrdf_core::RdfTextDirection;
use purrdf_hash::error::ErrorValue;
use purrdf_iri::{langtag::LanguageTagError, terminals::UcharError};

/// The grammar requirement at the original map cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapExpectation {
    /// The association separator.
    Association,
    /// A triple pattern's focus position.
    Focus,
    /// An RDF term.
    Term,
    /// The three-scalar triple-term closing delimiter.
    TripleClose,
    /// An IRI or the `a` predicate keyword.
    Predicate,
    /// An IRI or the `START` shape keyword.
    Shape,
    /// An angle-delimited IRI reference.
    Iri,
    /// The colon after a blank-node sigil.
    BlankColon,
    /// The first scalar of a blank-node label.
    BlankStart,
    /// An original required delimiter.
    Character(char),
}

impl purrdf_hash::error::StructuredError for MapExpectation {
    purrdf_hash::error_visit! {
        Association => "shape-map-expect-association" {}, Focus => "shape-map-expect-focus" {},
        Term => "shape-map-expect-term" {}, TripleClose => "shape-map-expect-triple-close" {},
        Predicate => "shape-map-expect-predicate" {}, Shape => "shape-map-expect-shape" {},
        Iri => "shape-map-expect-iri" {}, BlankColon => "shape-map-expect-blank-colon" {},
        BlankStart => "shape-map-expect-blank-start" {},
        Character(scalar) => "shape-map-expect-character" { "scalar" => Character(*scalar) },
    }
}

impl fmt::Display for MapExpectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Association => "expected '@' between node selector and shape",
            Self::Focus => "expected FOCUS in triple pattern",
            Self::Term => "expected a term (<iri>, _:blank, \"literal\" or <<( s p o )>>)",
            Self::TripleClose => "expected ')>>' to close a triple term opened with '<<('",
            Self::Predicate => "expected a predicate (<iri> or 'a')",
            Self::Shape => "expected a shape label (START or <iri>)",
            Self::Iri => "expected an IRI in angle brackets",
            Self::BlankColon => "expected ':' after '_' in blank node",
            Self::BlankStart => {
                "a blank-node label must begin with PN_CHARS_U or a digit after `_:`"
            }
            Self::Character(scalar) => return write!(f, "expected '{scalar}'"),
        })
    }
}

/// The original lexical construct surrounding a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeMapContext {
    /// An angle-delimited IRI reference.
    Iri,
    /// A quoted literal's lexical form.
    String,
}
impl purrdf_hash::error::StructuredError for ShapeMapContext {
    purrdf_hash::error_visit! {
        Iri => "shape-map-context-iri" {}, String => "shape-map-context-string" {},
    }
}
impl fmt::Display for ShapeMapContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Iri => "IRI",
            Self::String => "string",
        })
    }
}

/// The original role in which a prefixed name was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeMapPosition {
    /// A concrete node selector.
    Node,
    /// A triple-pattern predicate.
    Predicate,
    /// A shape selector.
    Shape,
    /// An IRI reference, including a literal datatype.
    Iri,
}
impl purrdf_hash::error::StructuredError for ShapeMapPosition {
    purrdf_hash::error_visit! {
        Node => "shape-map-position-node" {}, Predicate => "shape-map-position-predicate" {},
        Shape => "shape-map-position-shape" {}, Iri => "shape-map-position-iri" {},
    }
}
impl fmt::Display for ShapeMapPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Node => "a node must be an IRI",
            Self::Predicate => "a predicate must be `a` or an IRI",
            Self::Shape => "a shape label must be `START` or an IRI",
            Self::Iri => "an IRI must be written",
        })
    }
}

/// An original language token and its separately interpreted base direction.
/// The private prefix boundary is established by the parser's suffix split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeMapLanguageRefusal {
    raw: String,
    tag_bytes: usize,
    direction: Option<RdfTextDirection>,
    cause: LanguageTagError,
}
impl ShapeMapLanguageRefusal {
    pub(crate) fn new(
        raw: String,
        tag_bytes: usize,
        direction: Option<RdfTextDirection>,
        cause: LanguageTagError,
    ) -> Self {
        Self {
            raw,
            tag_bytes,
            direction,
            cause,
        }
    }
    /// The full original token, including any recognized direction suffix.
    pub fn raw(&self) -> &str {
        &self.raw
    }
    /// The original prefix actually judged by the language parser.
    pub fn tag(&self) -> &str {
        &self.raw[..self.tag_bytes]
    }
    /// The original interpreted base direction.
    pub fn direction(&self) -> Option<RdfTextDirection> {
        self.direction
    }
    /// The shared parser's original native cause.
    pub fn cause(&self) -> &LanguageTagError {
        &self.cause
    }
}
impl purrdf_hash::error::StructuredError for ShapeMapLanguageRefusal {
    purrdf_hash::error_visit! {
        @record self { raw, tag_bytes, direction, cause } => "shape-map-language" {
            "raw" => Text(&self.raw), "tag" => Text(self.tag()), "tag_bytes" => ByteCount(self.tag_bytes as u128),
            "direction" => Value(self.direction.map_or(ErrorValue::Absent, |value| ErrorValue::Code(value.as_str()))),
            "profile" => Code(super::SHEXJ_LANGUAGE_POLICY), "cause" => Cause(&self.cause)
        }
    }
}
impl fmt::Display for ShapeMapLanguageRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected a language tag: {} [{}]",
            self.cause,
            self.cause.diagnostic_code()
        )
    }
}
purrdf_lex::error_source!(ShapeMapLanguageRefusal => { Self { cause, .. } => cause, });

/// A complete native reason from the shape-map parser, without message fallbacks.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShapeMapReason {
    /// The original expected grammar and up to three scalars at its cursor.
    Expected {
        /// The unmet grammar requirement.
        expected: MapExpectation,
        /// The original current scalar, or end of input.
        found: Option<char>,
        /// The second original delimiter scalar, or end of input.
        next: Option<char>,
        /// The third original delimiter scalar, or end of input.
        after_next: Option<char>,
    },
    /// Input remains after a complete map.
    Trailing {
        /// The original following scalar.
        found: Option<char>,
    },
    /// A reifier spelling was used where only a triple term is admitted.
    Reifier {
        /// The scalar following the two opening angle brackets.
        found: Option<char>,
    },
    /// An IRI reference or string reached a forbidden scalar or end of input.
    Unterminated {
        /// The original lexical construct.
        context: ShapeMapContext,
        /// The original stopping scalar, or end of input.
        found: Option<char>,
    },
    /// The shared Unicode decoder's original refusal.
    UnicodeEscape {
        /// The original lexical construct.
        context: ShapeMapContext,
        /// The shared decoder's original cause.
        cause: UcharError,
    },
    /// An original prefixed name in one of the grammar's four IRI roles.
    PrefixedName {
        /// The original owned name.
        name: String,
        /// The original role.
        position: ShapeMapPosition,
    },
    /// The original language token, direction and native grammar cause.
    Language(ShapeMapLanguageRefusal),
    /// Input ended immediately after a string backslash.
    DanglingEscape,
    /// The original scalar after a string backslash is not an ECHAR or UCHAR.
    InvalidEscape {
        /// The original offending scalar.
        found: char,
    },
}
impl purrdf_hash::error::StructuredError for ShapeMapReason {
    purrdf_hash::error_visit! {
        Expected { expected, found, next, after_next } => "shape-map-expected" {
            "expected" => Record(expected), "found" => Value(found.map_or(ErrorValue::Absent, ErrorValue::Character)),
            "next" => Value(next.map_or(ErrorValue::Absent, ErrorValue::Character)), "after_next" => Value(after_next.map_or(ErrorValue::Absent, ErrorValue::Character))
        },
        Trailing { found } => "shape-map-trailing" { "found" => Value(found.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        Reifier { found } => "shape-map-reifier" { "found" => Value(found.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        Unterminated { context, found } => "shape-map-unterminated" { "context" => Record(context), "found" => Value(found.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        UnicodeEscape { context, cause } => "shape-map-unicode-escape" { "context" => Record(context), "cause" => Cause(cause) },
        PrefixedName { name, position } => "shape-map-prefixed-name" { "name" => Text(name), "position" => Record(position) },
        Language(cause) => "shape-map-language-refusal" { "cause" => Cause(cause) },
        DanglingEscape => "shape-map-dangling-escape" {},
        InvalidEscape { found } => "shape-map-invalid-escape" { "found" => Character(*found) },
    }
}
impl fmt::Display for ShapeMapReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected { expected, .. } => expected.fmt(f),
            Self::Trailing { .. } => f.write_str("trailing input after shape map"),
            Self::Reifier { .. } => f.write_str("`<< s p o >>` is RDF 1.2 reifier syntax, not a triple term; write a triple term as `<<( s p o )>>`"),
            Self::Unterminated { context: ShapeMapContext::Iri, .. } => f.write_str("unterminated IRI"),
            Self::Unterminated { context: ShapeMapContext::String, .. } => f.write_str("unterminated string literal"),
            Self::UnicodeEscape { context, cause } => match cause {
                UcharError::NotAnEscape => write!(f, "a backslash in an {context} must open a \\u/\\U escape"),
                UcharError::BadHex => write!(f, "bad \\u/\\U escape in {context} (expected hex digits)"),
                UcharError::NotAScalar => write!(f, "\\u/\\U escape in {context} is not a Unicode scalar value"),
            },
            Self::PrefixedName { name, position } => write!(f,
                "{position} in angle brackets; `{name}` is a prefixed name, and the ShapeMap \
                 grammar admits none — its `iri` production is IRIREF only. The specification \
                 also declines to say where a prefix map would come from, recording that \
                 resolving shape references against the schema's prefixes and node references \
                 against the data's is \"common practice\" that \"this specification does not \
                 specify\"; two documents may bind one prefix differently, so PurRDF resolves \
                 none rather than silently choosing a side. Write the IRI in full \
                 (`<http://example.org/S1>`), or a relative `<S1>` against a base"),
            Self::Language(cause) => cause.fmt(f),
            Self::DanglingEscape => f.write_str("dangling escape"),
            Self::InvalidEscape { .. } => f.write_str("invalid string escape"),
        }
    }
}
purrdf_lex::error_source!(ShapeMapReason => {
    Self::UnicodeEscape { cause, .. } => cause,
    Self::Language(cause) => cause,
});
