// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original ShExC facet conflict and admission evidence.
use crate::ast::NumericLiteral;
use crate::exact::ExactBound;
use crate::lexer::Spanned;
use core::fmt;
use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_xsd::XsdDatatype;

/// The identity of a facet, independent of its original keyword spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetKind {
    /// A pattern and its flags.
    Pattern,
    /// Exact string length.
    Length,
    /// Minimum string length.
    MinLength,
    /// Maximum string length.
    MaxLength,
    /// Inclusive lower numeric bound.
    MinInclusive,
    /// Exclusive lower numeric bound.
    MinExclusive,
    /// Inclusive upper numeric bound.
    MaxInclusive,
    /// Exclusive upper numeric bound.
    MaxExclusive,
    /// Total decimal digits.
    TotalDigits,
    /// Fractional decimal digits.
    FractionDigits,
}
impl FacetKind {
    pub(crate) const RANGE: [Self; 4] = [
        Self::MinInclusive,
        Self::MinExclusive,
        Self::MaxInclusive,
        Self::MaxExclusive,
    ];
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}
impl fmt::Display for FacetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pattern => "pattern",
            Self::Length => "length",
            Self::MinLength => "minlength",
            Self::MaxLength => "maxlength",
            Self::MinInclusive => "mininclusive",
            Self::MinExclusive => "minexclusive",
            Self::MaxInclusive => "maxinclusive",
            Self::MaxExclusive => "maxexclusive",
            Self::TotalDigits => "totaldigits",
            Self::FractionDigits => "fractiondigits",
        })
    }
}
impl StructuredError for FacetKind {
    purrdf_hash::error_visit! {
        Pattern => "shex-facet-pattern" {}, Length => "shex-facet-length" {}, MinLength => "shex-facet-minlength" {}, MaxLength => "shex-facet-maxlength" {},
        MinInclusive => "shex-facet-mininclusive" {}, MinExclusive => "shex-facet-minexclusive" {}, MaxInclusive => "shex-facet-maxinclusive" {}, MaxExclusive => "shex-facet-maxexclusive" {},
        TotalDigits => "shex-facet-totaldigits" {}, FractionDigits => "shex-facet-fractiondigits" {},
    }
}

/// One original keyword/value occurrence and its admitted unsigned value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsignedFacetOperand {
    keyword: Spanned,
    token: Spanned,
    value: u64,
}
impl UnsignedFacetOperand {
    pub(crate) fn new(keyword: Spanned, token: Spanned, value: u64) -> Self {
        Self {
            keyword,
            token,
            value,
        }
    }
    /// The original facet keyword, including its spelling and byte span.
    pub fn keyword(&self) -> &Spanned {
        &self.keyword
    }
    /// The original integer token and byte span.
    pub fn token(&self) -> &Spanned {
        &self.token
    }
    /// The admitted unsigned value.
    pub fn value(&self) -> u64 {
        self.value
    }
}
impl StructuredError for UnsignedFacetOperand {
    purrdf_hash::error_visit! {
        @record self { keyword, token, value } => "shex-facet-unsigned-operand" {
            "keyword" => Record(&self.keyword), "token" => Record(&self.token), "value" => Unsigned(u128::from(self.value))
        }
    }
}

/// An original numeric occurrence, including the AST value and exact bound.
/// Equality compares the AST floating value's bits without changing AST equality.
#[derive(Debug, Clone)]
pub struct NumericFacetOperand {
    keyword: Spanned,
    token: Spanned,
    value: NumericLiteral,
    exact: ExactBound,
}
impl NumericFacetOperand {
    pub(crate) fn new(
        keyword: Spanned,
        token: Spanned,
        value: NumericLiteral,
        exact: ExactBound,
    ) -> Self {
        Self {
            keyword,
            token,
            value,
            exact,
        }
    }
    /// The original facet keyword, including case and byte span.
    pub fn keyword(&self) -> &Spanned {
        &self.keyword
    }
    /// The original numeric token and byte span.
    pub fn token(&self) -> &Spanned {
        &self.token
    }
    /// The actual numeric value admitted to the AST, including saturation.
    pub fn value(&self) -> &NumericLiteral {
        &self.value
    }
    /// The exact original bound spelling.
    pub fn exact_lexical(&self) -> &str {
        self.exact.lexical()
    }
    /// The original bound's syntax-derived XSD datatype.
    pub fn exact_datatype(&self) -> XsdDatatype {
        self.exact.datatype()
    }
}
impl PartialEq for NumericFacetOperand {
    fn eq(&self, other: &Self) -> bool {
        self.keyword == other.keyword
            && self.token == other.token
            && self.exact == other.exact
            && match (self.value, other.value) {
                (NumericLiteral::Integer(a), NumericLiteral::Integer(b)) => a == b,
                (NumericLiteral::Fractional(a), NumericLiteral::Fractional(b)) => {
                    a.to_bits() == b.to_bits()
                }
                _ => false,
            }
    }
}
impl Eq for NumericFacetOperand {}
impl StructuredError for NumericFacetOperand {
    purrdf_hash::error_visit! {
        @record self { keyword, token, value, exact } => "shex-facet-numeric-operand" {
            "keyword" => Record(&self.keyword), "token" => Record(&self.token), "value" => Record(&self.value), "exact" => Record(&self.exact)
        }
    }
}

/// One admitted numeric facet at a refused datatype, in original source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmittedNumericFacet {
    /// A numeric range bound.
    Range {
        /// The exact facet identity.
        kind: FacetKind,
        /// The original admitted bound and occurrence.
        operand: NumericFacetOperand,
    },
    /// A digit-count constraint.
    Digits {
        /// TOTALDIGITS or FRACTIONDIGITS.
        kind: FacetKind,
        /// The original admitted unsigned operand.
        operand: UnsignedFacetOperand,
    },
}
impl AdmittedNumericFacet {
    pub(crate) fn start(&self) -> usize {
        match self {
            Self::Range { operand, .. } => operand.keyword.start,
            Self::Digits { operand, .. } => operand.keyword.start,
        }
    }
}
impl StructuredError for AdmittedNumericFacet {
    purrdf_hash::error_visit! {
        Range { kind, operand } => "shex-facet-admitted-range" { "kind" => Record(kind), "operand" => Record(operand) },
        Digits { kind, operand } => "shex-facet-admitted-digits" { "kind" => Record(kind), "operand" => Record(operand) },
    }
}

/// A complete native facet conflict or integer/datatype admission refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FacetReason {
    /// A second pattern was encountered, before consuming it.
    Pattern {
        /// The original admitted regex and its flags.
        previous: Spanned,
        /// The incoming regex and its flags.
        incoming: Spanned,
    },
    /// A string-length or digit-count facet was repeated.
    UnsignedDuplicate {
        /// The repeated facet identity.
        kind: FacetKind,
        /// The first admitted occurrence and value.
        previous: UnsignedFacetOperand,
        /// The incoming converted occurrence and value.
        incoming: UnsignedFacetOperand,
    },
    /// A numeric range facet was repeated.
    NumericDuplicate {
        /// The repeated facet identity.
        kind: FacetKind,
        /// The first admitted occurrence, AST value and exact bound.
        previous: NumericFacetOperand,
        /// The incoming converted occurrence, AST value and exact bound.
        incoming: NumericFacetOperand,
    },
    /// An integer token could not be admitted as a nonnegative u64.
    Unsigned {
        /// The actual string-length or digit-count facet identity.
        kind: FacetKind,
        /// The original facet keyword.
        keyword: Spanned,
        /// The refused integer token.
        token: Spanned,
        /// The original standard integer parser's cause.
        cause: core::num::ParseIntError,
    },
    /// A datatype cannot carry the admitted numeric constraints.
    Datatype {
        /// The original datatype IRI or prefixed name token.
        token: Spanned,
        /// The resolved datatype IRI.
        resolved: String,
        /// The recognized native datatype, or an unknown IRI.
        recognized: Option<XsdDatatype>,
        /// Every admitted numeric facet in source order.
        facets: Vec<AdmittedNumericFacet>,
    },
}
impl StructuredError for FacetReason {
    purrdf_hash::error_visit! {
        Pattern { previous, incoming } => "shex-facet-duplicate-pattern" { "previous" => Record(previous), "incoming" => Record(incoming) },
        UnsignedDuplicate { kind, previous, incoming } => "shex-facet-duplicate-unsigned" { "kind" => Record(kind), "previous" => Record(previous), "incoming" => Record(incoming) },
        NumericDuplicate { kind, previous, incoming } => "shex-facet-duplicate-numeric" { "kind" => Record(kind), "previous" => Record(previous), "incoming" => Record(incoming) },
        Unsigned { kind, keyword, token, cause } => "shex-facet-unsigned" { "kind" => Record(kind), "keyword" => Record(keyword), "token" => Record(token), "minimum" => Unsigned(0), "maximum" => Unsigned(u128::from(u64::MAX)), "cause" => Cause(cause) },
        Datatype { token, resolved, recognized, facets } => "shex-facet-datatype" { "token" => Record(token), "resolved" => Iri(resolved), "recognized" => Value(recognized.map_or(ErrorValue::Absent, |value| ErrorValue::Iri(value.iri()))), "facets" => List(facets.iter().map(|value| ErrorValue::Record(value))) },
    }
}
impl fmt::Display for FacetReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pattern { .. } => f.write_str("duplicate pattern facet"),
            Self::UnsignedDuplicate { kind, .. } | Self::NumericDuplicate { kind, .. } => {
                write!(f, "duplicate {kind} facet")
            }
            Self::Unsigned { kind, .. } => write!(
                f,
                "{} must be a non-negative integer",
                match kind {
                    FacetKind::TotalDigits | FacetKind::FractionDigits => "digit-count facet value",
                    _ => "string facet value",
                }
            ),
            Self::Datatype { resolved, .. } => {
                write!(f, "numeric facet on non-numeric datatype <{resolved}>")
            }
        }
    }
}
purrdf_lex::error_source!(FacetReason => { Self::Unsigned { cause, .. } => cause, });
