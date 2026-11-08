// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native refusals of OWL class-expression and data-range extraction.

use purrdf_core::TermValue;

/// A malformed or unrepresentable OWL expression, retaining its original operands.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OwlSyntaxError {
    /// An extraction left an unfinished native syntax capture.
    IncompleteCapture,
    /// An anonymous class-expression descent reached its native depth ceiling.
    ExpressionDepth {
        /// The original node the extractor refused to enter.
        node: TermValue,
        /// Number of class-expression nodes already active.
        active_depth: usize,
        /// Maximum number of active nodes.
        limit: usize,
    },
    /// A data-range node was revisited on its active descent.
    DataRangeCycle {
        /// The original repeated data-range node.
        node: TermValue,
    },
    /// A data-range descent reached its native depth ceiling.
    DataRangeDepth {
        /// The original node the extractor refused to enter.
        node: TermValue,
        /// Number of data-range nodes already active.
        active_depth: usize,
        /// Maximum number of active nodes.
        limit: usize,
    },
    /// An authored restriction has no property operand.
    MissingProperty {
        /// The original restriction node.
        node: TermValue,
    },
    /// A qualified cardinality has neither class nor data-range filler.
    MissingQualifiedFiller {
        /// The original restriction node.
        node: TermValue,
    },
    /// The original literal's collapsed lexical form is not a `u32`.
    CardinalityLexical {
        /// The complete authored literal, before whitespace collapse.
        literal: TermValue,
        /// The original integer parser refusal.
        cause: core::num::ParseIntError,
    },
    /// The parsed cardinality leaves no representation for its successor.
    CardinalityRange {
        /// The complete authored literal.
        literal: TermValue,
        /// The original parsed cardinality.
        value: u32,
        /// The largest cardinality this representation admits.
        maximum: u32,
    },
    /// A cardinality operand is not a literal.
    CardinalityNotLiteral {
        /// The complete original operand.
        value: TermValue,
    },
}

impl purrdf_hash::error::StructuredError for OwlSyntaxError {
    purrdf_hash::error_visit! {
        IncompleteCapture => "owl-syntax-incomplete-capture" {},
        ExpressionDepth { node, active_depth, limit } => "owl-syntax-expression-depth" {
            "node" => Term(node), "active_depth" => Unsigned(*active_depth as u128),
            "limit" => Unsigned(*limit as u128),
        },
        DataRangeCycle { node } => "owl-syntax-data-range-cycle" { "node" => Term(node) },
        DataRangeDepth { node, active_depth, limit } => "owl-syntax-data-range-depth" {
            "node" => Term(node), "active_depth" => Unsigned(*active_depth as u128),
            "limit" => Unsigned(*limit as u128),
        },
        MissingProperty { node } => "owl-syntax-missing-property" { "node" => Term(node) },
        MissingQualifiedFiller { node } => "owl-syntax-missing-qualified-filler" {
            "node" => Term(node),
        },
        CardinalityLexical { literal, cause } => "owl-syntax-cardinality-lexical" {
            "literal" => Term(literal), "cause" => Cause(cause),
        },
        CardinalityRange { literal, value, maximum } => "owl-syntax-cardinality-range" {
            "literal" => Term(literal), "value" => Unsigned(u128::from(*value)),
            "maximum" => Unsigned(u128::from(*maximum)),
        },
        CardinalityNotLiteral { value } => "owl-syntax-cardinality-not-literal" {
            "value" => Term(value),
        },
    }
}

impl core::fmt::Display for OwlSyntaxError {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IncompleteCapture => output.write_str("native syntax capture did not complete"),
            Self::ExpressionDepth { limit, .. } => {
                write!(output, "OWL class expression nests deeper than {limit}")
            }
            Self::DataRangeCycle { .. } => output.write_str("cyclic OWL data range"),
            Self::DataRangeDepth { limit, .. } => {
                write!(output, "OWL data range nests deeper than {limit}")
            }
            Self::MissingProperty { .. } => {
                output.write_str("owl:Restriction without owl:onProperty")
            }
            Self::MissingQualifiedFiller { .. } => {
                output.write_str("qualified cardinality without owl:onClass or owl:onDataRange")
            }
            Self::CardinalityLexical { literal, .. } => {
                let original: &dyn core::fmt::Debug = match literal {
                    TermValue::Literal { lexical_form, .. } => lexical_form,
                    value => value,
                };
                write!(output, "non-integer cardinality literal: {original:?}")
            }
            Self::CardinalityRange { value, maximum, .. } => write!(
                output,
                "cardinality {value} exceeds this reasoner's representable bound \
                 ({maximum} is the largest supported cardinality): refusing rather \
                 than deciding wrongly",
            ),
            Self::CardinalityNotLiteral { value } => {
                write!(output, "cardinality value is not a literal: {value:?}")
            }
        }
    }
}

impl std::error::Error for OwlSyntaxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CardinalityLexical { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
