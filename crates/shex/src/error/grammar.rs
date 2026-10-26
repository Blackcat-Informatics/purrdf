// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native ShExC token-stream grammar admission operands.
use crate::lexer::Spanned;
use core::fmt;
use purrdf_hash::error::{ErrorValue, StructuredError};

/// The grammar requirement at the original parser cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarExpectation {
    /// Requires `'=' after start`.
    StartEquals,
    /// Requires `')'`.
    RightParen,
    /// Requires `'['`.
    LeftBracket,
    /// Requires `'~' after '@' in a value set`.
    LanguageTilde,
    /// Requires `'{'`.
    LeftBrace,
    /// Requires `'}'`.
    RightBrace,
    /// Requires `'('`.
    LeftParen,
    /// Requires `predicate`.
    Predicate,
    /// Requires `shape label`.
    ShapeLabel,
    /// Requires `datatype IRI`.
    Datatype,
    /// Requires `value-set IRI`.
    ValueIri,
    /// Requires `IRI exclusion`.
    IriExclusion,
    /// Requires `annotation object`.
    AnnotationObject,
    /// Requires `prefix name after PREFIX`.
    PrefixName,
    /// Requires `IRI after BASE`.
    BaseIri,
    /// Requires `IRI after IMPORT`.
    ImportIri,
    /// Requires `shape expression`.
    ShapeExpression,
    /// Requires `non-negative INTEGER as string facet value`.
    StringFacet,
    /// Requires `non-negative INTEGER as digit-count facet value`.
    DigitFacet,
    /// Requires `numeric literal as facet value`.
    NumericFacet,
    /// Requires `'-' exclusion after '.' in a value set`.
    WildcardExclusion,
    /// Requires `IRI, literal or language tag exclusion`.
    Exclusion,
    /// Requires `language tag exclusion`.
    LanguageExclusion,
    /// Requires `literal`.
    Literal,
}
impl StructuredError for GrammarExpectation {
    purrdf_hash::error_visit! {
        StartEquals => "shexc-expect-start-equals" {},
        RightParen => "shexc-expect-right-paren" {},
        LeftBracket => "shexc-expect-left-bracket" {},
        LanguageTilde => "shexc-expect-language-tilde" {},
        LeftBrace => "shexc-expect-left-brace" {},
        RightBrace => "shexc-expect-right-brace" {},
        LeftParen => "shexc-expect-left-paren" {},
        Predicate => "shexc-expect-predicate" {},
        ShapeLabel => "shexc-expect-shape-label" {},
        Datatype => "shexc-expect-datatype" {},
        ValueIri => "shexc-expect-value-iri" {},
        IriExclusion => "shexc-expect-iri-exclusion" {},
        AnnotationObject => "shexc-expect-annotation-object" {},
        PrefixName => "shexc-expect-prefix-name" {},
        BaseIri => "shexc-expect-base-iri" {},
        ImportIri => "shexc-expect-import-iri" {},
        ShapeExpression => "shexc-expect-shape-expression" {},
        StringFacet => "shexc-expect-string-facet" {},
        DigitFacet => "shexc-expect-digit-facet" {},
        NumericFacet => "shexc-expect-numeric-facet" {},
        WildcardExclusion => "shexc-expect-wildcard-exclusion" {},
        Exclusion => "shexc-expect-exclusion" {},
        LanguageExclusion => "shexc-expect-language-exclusion" {},
        Literal => "shexc-expect-literal" {},
    }
}
impl fmt::Display for GrammarExpectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::StartEquals => "'=' after start",
            Self::RightParen => "')'",
            Self::LeftBracket => "'['",
            Self::LanguageTilde => "'~' after '@' in a value set",
            Self::LeftBrace => "'{'",
            Self::RightBrace => "'}'",
            Self::LeftParen => "'('",
            Self::Predicate => "predicate",
            Self::ShapeLabel => "shape label",
            Self::Datatype => "datatype IRI",
            Self::ValueIri => "value-set IRI",
            Self::IriExclusion => "IRI exclusion",
            Self::AnnotationObject => "annotation object",
            Self::PrefixName => "prefix name after PREFIX",
            Self::BaseIri => "IRI after BASE",
            Self::ImportIri => "IRI after IMPORT",
            Self::ShapeExpression => "shape expression",
            Self::StringFacet => "non-negative INTEGER as string facet value",
            Self::DigitFacet => "non-negative INTEGER as digit-count facet value",
            Self::NumericFacet => "numeric literal as facet value",
            Self::WildcardExclusion => "'-' exclusion after '.' in a value set",
            Self::Exclusion => "IRI, literal or language tag exclusion",
            Self::LanguageExclusion => "language tag exclusion",
            Self::Literal => "literal",
        })
    }
}
impl GrammarExpectation {
    fn shows_encountered(self) -> bool {
        matches!(
            self,
            Self::StartEquals
                | Self::RightParen
                | Self::LeftBracket
                | Self::LanguageTilde
                | Self::LeftBrace
                | Self::RightBrace
                | Self::LeftParen
                | Self::Predicate
                | Self::ShapeLabel
                | Self::Datatype
                | Self::ValueIri
                | Self::IriExclusion
                | Self::AnnotationObject
                | Self::ShapeExpression
                | Self::NumericFacet
                | Self::Exclusion
                | Self::Literal
        )
    }
}

/// A complete token-stream grammar refusal, retaining original lexical operands.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GrammarReason {
    /// A required production was absent; `None` means end of input.
    Expected {
        /// The required grammar production.
        expected: GrammarExpectation,
        /// The original token at the probe, or end of input.
        encountered: Option<Spanned>,
    },
    /// A facet keyword lacked a value of the required lexical kind.
    FacetValue {
        /// The required kind of facet value.
        expected: GrammarExpectation,
        /// The original consumed facet keyword, retaining case and span.
        facet: Spanned,
        /// The original value token, or end of input.
        encountered: Option<Spanned>,
    },
    /// A token cannot begin a directive or declaration.
    Declaration {
        /// The original unexpected token.
        encountered: Spanned,
    },
    /// A keyword could not form a node constraint.
    Keyword {
        /// The original keyword spelling.
        word: String,
        /// Start byte offset of the original keyword.
        start: usize,
        /// End byte offset of the original keyword.
        end: usize,
    },
    /// A token cannot be a value-set member; `None` means end of input.
    ValueSet {
        /// The original member token, or end of input.
        encountered: Option<Spanned>,
    },
    /// A PREFIX declaration lacked its namespace IRI after the original name.
    PrefixIri {
        /// The original consumed prefix name.
        prefix: Spanned,
        /// The original namespace token, or end of input.
        encountered: Option<Spanned>,
    },
    /// A PREFIX declaration supplied a nonempty local part.
    PrefixLocal {
        /// The original prefixed name and its byte span.
        encountered: Spanned,
    },
    /// A literal carried both language and datatype suffixes.
    LiteralSuffix {
        /// The original lexical literal token.
        literal: Spanned,
        /// The original language suffix.
        language: Spanned,
        /// The conflicting datatype marker, before any datatype is parsed.
        marker: Spanned,
    },
    /// A dollar-labelled expression resolved to an inclusion.
    LabelledInclusion {
        /// The resolved IRI or blank-node label.
        label: String,
        /// The resolved IRI or blank-node inclusion reference.
        reference: String,
        /// Original tokens spelling the dollar label.
        label_tokens: Vec<Spanned>,
        /// Original tokens spelling the bracketed inclusion.
        reference_tokens: Vec<Spanned>,
    },
}

impl StructuredError for GrammarReason {
    purrdf_hash::error_visit! {
        Expected { expected, encountered } => "shexc-grammar-expected" {
            "expected" => Record(expected), "encountered" => Value(encountered.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value)))
        },
        FacetValue { expected, facet, encountered } => "shexc-grammar-facet-value" {
            "expected" => Record(expected), "facet" => Record(facet), "encountered" => Value(encountered.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value)))
        },
        Declaration { encountered } => "shexc-grammar-declaration" { "encountered" => Record(encountered) },
        Keyword { word, start, end } => "shexc-grammar-keyword" { "word" => Text(word), "start" => ByteCount(*start as u128), "end" => ByteCount(*end as u128) },
        ValueSet { encountered } => "shexc-grammar-value-set" { "encountered" => Value(encountered.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))) },
        PrefixIri { prefix, encountered } => "shexc-grammar-prefix-iri" { "prefix" => Record(prefix), "encountered" => Value(encountered.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))) },
        PrefixLocal { encountered } => "shexc-grammar-prefix-local" { "encountered" => Record(encountered) },
        LiteralSuffix { literal, language, marker } => "shexc-grammar-literal-suffix" { "literal" => Record(literal), "language" => Record(language), "marker" => Record(marker) },
        LabelledInclusion { label, reference, label_tokens, reference_tokens } => "shexc-grammar-labelled-inclusion" {
            "label" => Text(label), "reference" => Text(reference),
            "label_tokens" => List(label_tokens.iter().map(|value| ErrorValue::Record(value))),
            "reference_tokens" => List(reference_tokens.iter().map(|value| ErrorValue::Record(value)))
        },
    }
}
impl fmt::Display for GrammarReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected {
                expected,
                encountered,
            }
            | Self::FacetValue {
                expected,
                encountered,
                ..
            } => {
                write!(f, "expected {expected}")?;
                if expected.shows_encountered() {
                    write!(
                        f,
                        ", found {:?}",
                        encountered.as_ref().map(|value| &value.token)
                    )?;
                }
                Ok(())
            }
            Self::Declaration { encountered } => write!(
                f,
                "expected directive or shape declaration, found {:?}",
                encountered.token
            ),
            Self::Keyword { word, .. } => write!(f, "unexpected keyword {word}"),
            Self::ValueSet { encountered } => write!(
                f,
                "unexpected {:?} in value set",
                encountered.as_ref().map(|value| &value.token)
            ),
            Self::PrefixIri { .. } => f.write_str("expected IRI after PREFIX name"),
            Self::PrefixLocal { .. } => f.write_str("PREFIX name must end with ':'"),
            Self::LiteralSuffix { .. } => {
                f.write_str("literal cannot carry both language tag and datatype")
            }
            Self::LabelledInclusion { .. } => f.write_str("'$' label cannot name an inclusion"),
        }
    }
}
impl std::error::Error for GrammarReason {}
