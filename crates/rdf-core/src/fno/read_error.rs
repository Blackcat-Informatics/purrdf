// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native data at the strict FnO read boundary.

use super::{FnoAdmissionError, TermValue};
use crate::{ListError, QuadValues, TermLookupError};

/// The required shape of one FnO field.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FnoFieldKind {
    /// A required scalar has exactly one distinct object.
    ExactlyOne,
    /// An optional scalar has at most one distinct object.
    AtMostOne,
    /// An absolute IRI term.
    Iri,
    /// A native blank node, including its document scope.
    Blank,
    /// A literal with all its original facets.
    Literal,
    /// A plain `xsd:string` without language or direction.
    PlainString,
    /// The writer's canonical `xsd:boolean` spelling, `true` or `false`.
    Boolean,
    /// The subject has the structural type required by its referencing role.
    Role {
        /// Original required class IRI.
        class: String,
    },
}

impl purrdf_hash::error::StructuredError for FnoFieldKind {
    purrdf_hash::error_visit! {
        ExactlyOne => "fno-field-exactly-one" {},
        AtMostOne => "fno-field-at-most-one" {},
        Iri => "fno-field-iri" {},
        Blank => "fno-field-blank" {},
        Literal => "fno-field-literal" {},
        PlainString => "fno-field-plain-string" {},
        Boolean => "fno-field-boolean" {},
        Role { class } => "fno-field-role" { "class" => Iri(class) },
    }
}

/// A statement cannot be reproduced by the FnO writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FnoStructureKind {
    /// Content in a named graph.
    NamedGraph,
    /// An RDF 1.2 reifier declaration.
    Reifier,
    /// An RDF 1.2 annotation row.
    Annotation,
    /// A predicate, type or disconnected record outside the admitted model.
    Unclaimed,
    /// A list cell is an IRI or another nonblank term.
    NonblankCell,
    /// Two lists share a cell, or a list cell is also a modeled record.
    SharedCell,
}

impl purrdf_hash::error::StructuredError for FnoStructureKind {
    purrdf_hash::error_visit! {
        NamedGraph => "fno-structure-named-graph" {},
        Reifier => "fno-structure-reifier" {},
        Annotation => "fno-structure-annotation" {},
        Unclaimed => "fno-structure-unclaimed" {},
        NonblankCell => "fno-structure-nonblank-cell" {},
        SharedCell => "fno-structure-shared-cell" {},
    }
}

/// A strict FnO read refuses without publishing a partial catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FnoReadError<E = std::convert::Infallible> {
    /// The original operational read failure, before or during the read.
    Source(E),
    /// The shared native term resolver refused an input identifier.
    Term(TermLookupError<E>),
    /// The explicit ontology selector has no unique document.
    Document {
        /// Original caller selector.
        ontology_iri: String,
        /// Every candidate document in canonical native term order.
        candidates: Vec<TermValue>,
    },
    /// A field has the wrong cardinality, value kind or structural role.
    Field {
        /// Original subject.
        subject: TermValue,
        /// Original field predicate.
        predicate: String,
        /// Original distinct objects in canonical native term order.
        objects: Vec<TermValue>,
        /// The violated field obligation.
        expected: FnoFieldKind,
    },
    /// The shared strict list walker refused a chain.
    List {
        /// Original subject of the list-valued property.
        subject: TermValue,
        /// Original list-valued predicate.
        predicate: String,
        /// Original strict fault, node and ordered member prefix.
        cause: ListError<TermValue>,
    },
    /// A well-formed `fno:returns` list does not contain exactly one output.
    Returns {
        /// Original function subject.
        subject: TermValue,
        /// Original list-valued predicate.
        predicate: String,
        /// Original object of the list-valued property.
        head: TermValue,
        /// Original members in list order.
        members: Vec<TermValue>,
    },
    /// A complete original row cannot be represented by the writer.
    Statement {
        /// Original subject, predicate, object and graph.
        row: Box<QuadValues>,
        /// The structural boundary that refused it.
        reason: FnoStructureKind,
    },
    /// Even an empty named graph would be lost on emission.
    Graph {
        /// Original graph identity.
        graph: TermValue,
    },
    /// The shared catalog admission path refused reconstructed fields.
    Admission(Box<FnoAdmissionError>),
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for FnoReadError<E>
{
    purrdf_hash::error_visit! {
        Source(cause) => "fno-read-source" { "cause" => Cause(cause) },
        Term(cause) => "fno-read-term" { "cause" => Cause(cause) },
        Document { ontology_iri, candidates } => "fno-read-document" {
            "ontology_iri" => Iri(ontology_iri),
            "candidates" => List(candidates.iter().map(|value| purrdf_hash::error::ErrorValue::Term(value))),
        },
        Field { subject, predicate, objects, expected } => "fno-read-field" {
            "subject" => Term(subject), "predicate" => Iri(predicate),
            "objects" => List(objects.iter().map(|value| purrdf_hash::error::ErrorValue::Term(value))),
            "expected" => Record(expected),
        },
        List { subject, predicate, cause } => "fno-read-list" {
            "subject" => Term(subject), "predicate" => Iri(predicate), "cause" => Cause(cause),
        },
        Returns { subject, predicate, head, members } => "fno-read-returns" {
            "subject" => Term(subject), "predicate" => Iri(predicate), "head" => Term(head),
            "members" => List(members.iter().map(|value| purrdf_hash::error::ErrorValue::Term(value))),
        },
        Statement { row, reason } => "fno-read-statement" {
            "row" => Record(&**row), "reason" => Record(reason),
        },
        Graph { graph } => "fno-read-graph" { "graph" => Term(graph) },
        Admission(cause) => "fno-read-admission" { "cause" => Cause(&**cause) },
    }
}

impl<E: std::fmt::Display> std::fmt::Display for FnoReadError<E> {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(cause) => write!(out, "FnO source read: {cause}"),
            Self::Term(cause) => write!(out, "FnO term read: {cause}"),
            Self::Document {
                ontology_iri,
                candidates,
            } => write!(
                out,
                "FnO ontology {ontology_iri:?} requires one document, found {}",
                candidates.len()
            ),
            Self::Field {
                subject,
                predicate,
                expected,
                ..
            } => write!(out, "FnO {subject:?} {predicate} requires {expected:?}"),
            Self::List {
                subject,
                predicate,
                cause,
            } => write!(out, "FnO {subject:?} {predicate}: {cause}"),
            Self::Returns {
                subject,
                predicate,
                members,
                ..
            } => write!(
                out,
                "FnO {subject:?} {predicate} requires one output, found {}",
                members.len()
            ),
            Self::Statement { row, reason } => write!(out, "FnO unsupported {reason:?}: {row:?}"),
            Self::Graph { graph } => write!(out, "FnO cannot preserve named graph {graph:?}"),
            Self::Admission(cause) => std::fmt::Display::fmt(cause, out),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for FnoReadError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(cause) => Some(cause),
            Self::Term(cause) => Some(cause),
            Self::List { cause, .. } => Some(cause),
            Self::Admission(cause) => Some(cause.as_ref()),
            _ => None,
        }
    }
}
