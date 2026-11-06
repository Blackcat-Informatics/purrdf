// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed operands of SPARQL namespace dispatch and rendering refusals.

use super::SparqlCallForm;

/// Why a SPARQL vocabulary name has no scalar node-expression reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SparqlUnsupported {
    /// An aggregate needs an aggregate node expression.
    Aggregate,
    /// EXISTS forms consume graph patterns rather than scalar arguments.
    GraphPattern,
    /// The namespace defines no callable scalar with this local name.
    Unknown,
    /// A registered builtin belongs to neither supported namespace.
    Namespace,
}

/// Why a SPARQL node-expression call cannot be lowered or rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SparqlCallError {
    /// The original namespace spelling has no scalar call form.
    Unsupported {
        /// Original full IRI.
        iri: String,
        /// Original local name, when a SPARQL namespace was present.
        local: Option<String>,
        /// The unsupported semantic category.
        reason: SparqlUnsupported,
    },
    /// The actual argument count is outside the form's admitted range.
    Arity {
        /// Original function IRI.
        iri: String,
        /// Selected syntax form, including its original keyword/operator.
        form: SparqlCallForm,
        /// Actual supplied count.
        actual: usize,
        /// Minimum admitted count.
        minimum: usize,
        /// Maximum admitted count, absent for an unbounded grammar.
        maximum: Option<usize>,
    },
    /// An authored node contains this original call refusal.
    AtNode {
        /// Original authored node identity.
        node: Box<crate::term::Term>,
        /// Original call refusal.
        cause: Box<Self>,
    },
    /// The shared SPARQL parser refused the exact rendered probe.
    RenderedSyntax {
        /// Original function IRI.
        iri: String,
        /// Original authored node identity.
        node: Box<crate::term::Term>,
        /// Exact rendered expression.
        expression: String,
        /// Exact SELECT carrier offered to the parser.
        probe: String,
        /// Original native parser refusal.
        cause: Box<purrdf_sparql_algebra::ParseError>,
    },
}

impl SparqlCallError {
    pub(crate) fn at_node(self, node: &crate::term::Term) -> Self {
        Self::AtNode {
            node: Box::new(node.clone()),
            cause: Box::new(self),
        }
    }
}

impl std::fmt::Display for SparqlCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { iri, reason, .. } => match reason {
                SparqlUnsupported::Aggregate => write!(
                    f,
                    "<{iri}> names a SPARQL AGGREGATE, not a scalar function, and cannot be called from a \
                     node expression; the SHACL aggregate node expressions are shnex:count, shnex:min, \
                     shnex:max and shnex:sum"
                ),
                SparqlUnsupported::GraphPattern => write!(
                    f,
                    "<{iri}> is a SPARQL functional form over a GRAPH PATTERN, not over argument values, \
                     and cannot be called from a node expression; the SHACL existence node expression is \
                     shnex:exists"
                ),
                SparqlUnsupported::Unknown => {
                    write!(f, "<{iri}> is not a callable SPARQL 1.2 function name")
                }
                SparqlUnsupported::Namespace => write!(
                    f,
                    "<{iri}> is registered as a built-in list-parameter function but is neither \
                     shnex:conformsToShape nor a sparql: function"
                ),
            },
            Self::Arity {
                iri,
                form,
                actual,
                minimum,
                maximum,
            } => match form {
                SparqlCallForm::Call(keyword) => {
                    write!(f, "<{iri}> is the SPARQL `{keyword}` function and takes ")?;
                    match maximum {
                        Some(maximum) if maximum == minimum => write!(f, "exactly {minimum}"),
                        Some(maximum) => write!(f, "{minimum} to {maximum}"),
                        None => write!(f, "at least {minimum}"),
                    }?;
                    write!(f, " argument(s), got {actual}")
                }
                SparqlCallForm::Infix(op) => write!(
                    f,
                    "<{iri}> is the SPARQL `{op}` operator and takes exactly 2 arguments, got {actual}"
                ),
                SparqlCallForm::Prefix(op) => write!(
                    f,
                    "<{iri}> is the SPARQL unary `{op}` operator and takes exactly 1 argument, got {actual}"
                ),
                SparqlCallForm::Membership(keyword) => write!(
                    f,
                    "<{iri}> is the SPARQL `{keyword}` form and takes at least 1 argument, got {actual}"
                ),
                SparqlCallForm::Ebv => write!(
                    f,
                    "<{iri}> is the SPARQL effective-boolean-value coercion and takes exactly 1 argument, got {actual}"
                ),
            },
            Self::AtNode { node, cause } => {
                write!(f, "SPARQL function node expression on {node}: {cause}")
            }
            Self::RenderedSyntax {
                iri,
                node,
                expression,
                cause,
                ..
            } => write!(
                f,
                "SPARQL function node expression <{iri}> on {node} does not render a valid SPARQL expression ({expression}): {cause}"
            ),
        }
    }
}

impl std::error::Error for SparqlCallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AtNode { cause, .. } => Some(cause.as_ref()),
            Self::RenderedSyntax { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for SparqlUnsupported {
    purrdf_hash::error_visit! {
        Aggregate => "sparql-call-aggregate" {}, GraphPattern => "sparql-call-graph-pattern" {},
        Unknown => "sparql-call-unknown" {}, Namespace => "sparql-call-namespace" {},
    }
}

impl purrdf_hash::error::StructuredError for SparqlCallForm {
    purrdf_hash::error_visit! {
        Call(keyword) => "sparql-call-form" { "keyword" => Text(keyword) },
        Infix(operator) => "sparql-infix-form" { "operator" => Text(operator) },
        Prefix(operator) => "sparql-prefix-form" { "operator" => Text(operator) },
        Membership(keyword) => "sparql-membership-form" { "keyword" => Text(keyword) },
        Ebv => "sparql-ebv-form" {},
    }
}

impl purrdf_hash::error::StructuredError for SparqlCallError {
    purrdf_hash::error_visit! {
        Unsupported { iri, local, reason } => "shacl-sparql-call-unsupported" {
            "iri" => Iri(iri), "local" => Value(local.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent,
                purrdf_hash::error::ErrorValue::Text)), "reason" => Record(reason),
        },
        Arity { iri, form, actual, minimum, maximum } => "shacl-sparql-call-arity" {
            "iri" => Iri(iri), "form" => Record(form), "actual" => Unsigned(*actual as u128),
            "minimum" => Unsigned(*minimum as u128),
            "maximum" => Value(maximum.map_or(purrdf_hash::error::ErrorValue::Absent,
                |value| purrdf_hash::error::ErrorValue::Unsigned(value as u128))),
        },
        AtNode { node, cause } => "shacl-sparql-call-at-node" { "node" => Term(node.as_ref()), "cause" => Cause(cause.as_ref()) },
        RenderedSyntax { iri, node, expression, probe, cause } => "shacl-sparql-call-syntax" {
            "iri" => Iri(iri), "node" => Term(node.as_ref()), "expression" => Text(expression),
            "probe" => Text(probe), "cause" => Cause(cause.as_ref()),
        },
    }
}
