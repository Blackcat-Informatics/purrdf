// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original namespace operands and IRI causes, independent of presentation.

use std::fmt;

/// The operation that interpreted a namespace operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NamespaceContext {
    /// A caller-declared CURIE expanded through its namespace.
    Curie,
    /// An offered absolute IRI without a declared prefix binding.
    IriValue,
    /// A class key expanded through the primary namespace.
    ClassKey,
}

impl purrdf_hash::error::StructuredError for NamespaceContext {
    purrdf_hash::error_visit! {
        Curie => "namespace-curie" {},
        IriValue => "namespace-iri-value" {},
        ClassKey => "namespace-class-key" {},
    }
}

/// Why namespace construction or expansion refused the caller's input.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NamespaceError {
    /// The primary prefix is absent from both declared and builtin bindings.
    UndeclaredPrimary {
        /// The caller's original prefix.
        prefix: String,
    },
    /// A declared CURIE has no local name.
    EmptyLocal {
        /// The original CURIE.
        input: String,
    },
    /// An empty schema definition key cannot name a class.
    EmptyClassKey,
    /// The offered or expanded spelling is syntactically invalid.
    InvalidIri {
        /// The operation that interpreted the input.
        context: NamespaceContext,
        /// The original offered spelling.
        input: String,
        /// The exact spelling sent to the IRI parser.
        expanded: String,
        /// The original native parser cause.
        cause: purrdf_iri::IriError,
    },
    /// The spelling parses but has no scheme.
    RelativeIri {
        /// The operation that interpreted the input.
        context: NamespaceContext,
        /// The original offered spelling.
        input: String,
        /// The parsed relative reference.
        expanded: String,
    },
}

impl purrdf_hash::error::StructuredError for NamespaceError {
    purrdf_hash::error_visit! {
        UndeclaredPrimary { prefix } => "namespace-undeclared-primary" { "prefix" => Text(prefix) },
        EmptyLocal { input } => "namespace-empty-local" { "input" => Text(input) },
        EmptyClassKey => "namespace-empty-class-key" {},
        InvalidIri { context, input, expanded, cause } => "namespace-invalid-iri" {
            "context" => Record(context), "input" => Text(input),
            "expanded" => Text(expanded), "cause" => Cause(cause),
        },
        RelativeIri { context, input, expanded } => "namespace-relative-iri" {
            "context" => Record(context), "input" => Text(input), "expanded" => Text(expanded),
        },
    }
}

impl fmt::Display for NamespaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndeclaredPrimary { prefix } => write!(
                f,
                "Namespaces: primary prefix {prefix:?} is not declared — pass it in doc_prefixes (the shapes document's @prefix declarations) or use a W3C builtin"
            ),
            Self::EmptyLocal { input } => {
                write!(f, "CURIE {input:?} must have a non-empty local part")
            }
            Self::EmptyClassKey => {
                f.write_str("JSON Schema $defs key cannot be empty when used as a class")
            }
            Self::InvalidIri {
                context,
                input,
                cause,
                ..
            } => match context {
                NamespaceContext::Curie => write!(
                    f,
                    "expanded CURIE {input:?} is not an absolute IRI: {cause}"
                ),
                NamespaceContext::IriValue => write!(
                    f,
                    "IRI value {input:?} is neither absolute nor a caller-declared CURIE: {cause}"
                ),
                NamespaceContext::ClassKey => {
                    write!(f, "class key {input:?} does not form a valid IRI: {cause}")
                }
            },
            Self::RelativeIri { context, input, .. } => match context {
                NamespaceContext::Curie => {
                    write!(f, "expanded CURIE {input:?} is not an absolute IRI")
                }
                NamespaceContext::IriValue => write!(
                    f,
                    "IRI value {input:?} is neither absolute nor a caller-declared CURIE"
                ),
                NamespaceContext::ClassKey => {
                    write!(f, "class key {input:?} does not form an absolute IRI")
                }
            },
        }
    }
}

impl std::error::Error for NamespaceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidIri { cause, .. } => Some(cause),
            _ => None,
        }
    }
}

pub(super) fn absolute(
    input: &str,
    expanded: String,
    context: NamespaceContext,
) -> Result<String, NamespaceError> {
    match purrdf_iri::parse(&expanded) {
        Err(cause) => Err(NamespaceError::InvalidIri {
            context,
            input: input.to_owned(),
            expanded,
            cause,
        }),
        Ok(parsed) if !parsed.has_scheme() => Err(NamespaceError::RelativeIri {
            context,
            input: input.to_owned(),
            expanded,
        }),
        Ok(_) => Ok(expanded),
    }
}
