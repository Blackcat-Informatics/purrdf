// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original term owners and shallow dataset evidence at predicate refusal.

use std::fmt;

use purrdf_core::TermId;
use purrdf_hash::error::StructuredError;

use super::term_key;
use crate::{RdfDataset, TermRef, TermValue};

/// A native visualization predicate or term-kind refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VizPredicateError {
    /// An annotation predicate is not an IRI term.
    DatasetAnnotation {
        /// Original predicate ID, meaningful only in its source dataset.
        predicate_id: TermId,
        /// Original semantic term, independent of the source dataset.
        actual: TermValue,
    },
    /// A quad predicate is not an IRI term.
    DatasetQuad {
        /// Original predicate ID, meaningful only in its source dataset.
        predicate_id: TermId,
        /// Original semantic term and shallow native ID evidence.
        actual: VizResolvedPredicate,
    },
    /// A quoted statement's predicate is not an IRI term.
    ExpectedIri {
        /// Original owned predicate term.
        actual: TermValue,
    },
    /// An `rdf:reifies` object is not a triple term.
    ReifiesObjectNotTriple {
        /// Original owned object term.
        actual: TermValue,
    },
    /// A triple term reached the ordinary-term conversion boundary.
    TripleAsOrdinaryTerm {
        /// Original owned triple term.
        actual: TermValue,
    },
}

impl fmt::Display for VizPredicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatasetAnnotation { actual, .. } | Self::ExpectedIri { actual } => {
                write_refusal(f, &term_key(actual))
            }
            Self::DatasetQuad { actual, .. } => {
                write_refusal(f, &format!("{:?}", actual.native_ref()))
            }
            Self::ReifiesObjectNotTriple { .. } => {
                write_refusal(f, "rdf:reifies object must be a triple term")
            }
            Self::TripleAsOrdinaryTerm { .. } => write_refusal(
                f,
                "triple terms are represented as statements, not ordinary terms",
            ),
        }
    }
}

/// The original quoted-predicate wrapper, shared by all five production sites.
fn write_refusal(f: &mut fmt::Formatter<'_>, spelling: &str) -> fmt::Result {
    write!(f, "visualization predicate {spelling:?} is not an IRI")
}

impl std::error::Error for VizPredicateError {}

impl StructuredError for VizPredicateError {
    purrdf_hash::error_visit! {
        DatasetAnnotation { predicate_id, actual } => "rdf-viz-annotation-predicate" {
            "predicate_index" => Unsigned(predicate_id.index() as u128),
            "actual" => Term(actual),
        },
        DatasetQuad { predicate_id, actual } => "rdf-viz-quad-predicate" {
            "predicate_index" => Unsigned(predicate_id.index() as u128),
            "actual" => Record(actual),
        },
        ExpectedIri { actual } => "rdf-viz-predicate-not-iri" { "actual" => Term(actual) },
        ReifiesObjectNotTriple { actual } => "rdf-viz-reifies-object-not-triple" {
            "actual" => Term(actual),
        },
        TripleAsOrdinaryTerm { actual } => "rdf-viz-triple-as-ordinary-term" {
            "actual" => Term(actual),
        },
    }
}

/// A predicate's semantic owner and original shallow dataset IDs.
///
/// The private fields are captured together from one immutable dataset term.
/// The shared visitor reports the semantic value and native IDs separately;
/// IDs are dataset-local dense indices, never cross-dataset value identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizResolvedPredicate {
    /// Native home of all semantic ownership, clone, equality and drop behavior.
    actual: TermValue,
    /// Only the native IDs that the dataset-independent value does not retain.
    ids: NativeIds,
}

impl VizResolvedPredicate {
    pub(super) fn capture(dataset: &RdfDataset, id: TermId) -> Self {
        let ids = match dataset.resolve(id) {
            TermRef::Iri(_) | TermRef::Blank { .. } => NativeIds::Leaf,
            TermRef::Literal { datatype, .. } => NativeIds::Literal { datatype },
            TermRef::Triple { s, p, o } => NativeIds::Triple { s, p, o },
        };
        Self {
            actual: dataset.term_value(id),
            ids,
        }
    }

    /// The original term by value, retained after the source dataset is dropped.
    #[must_use]
    pub fn actual(&self) -> &TermValue {
        &self.actual
    }

    /// Borrow the exact native presentation, without another semantic term model.
    fn native_ref(&self) -> TermRef<'_> {
        match (&self.actual, &self.ids) {
            (TermValue::Iri(iri), NativeIds::Leaf) => TermRef::Iri(iri),
            (TermValue::Blank { label, scope }, NativeIds::Leaf) => TermRef::Blank {
                label,
                scope: *scope,
            },
            (
                TermValue::Literal {
                    lexical_form,
                    language,
                    direction,
                    ..
                },
                NativeIds::Literal { datatype },
            ) => TermRef::Literal {
                lexical: lexical_form,
                datatype: *datatype,
                language: language.as_deref(),
                direction: *direction,
            },
            (TermValue::Triple { .. }, NativeIds::Triple { s, p, o }) => TermRef::Triple {
                s: *s,
                p: *p,
                o: *o,
            },
            // The only constructor reads both fields from the same immutable
            // dataset term; callers can neither construct nor mutate the fields.
            _ => unreachable!("INVARIANT: sealed predicate snapshot retains its native term kind"),
        }
    }
}

impl StructuredError for VizResolvedPredicate {
    purrdf_hash::error_visit! {
        @record self { actual, ids } => "rdf-viz-resolved-predicate" {
            "actual" => Term(&self.actual), "ids" => Record(&self.ids),
        }
    }
}

/// Shallow native ID metadata, not a term value or another identity scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeIds {
    /// An IRI or blank node carries no dataset-local component IDs.
    Leaf,
    /// A literal refers to its datatype by native ID.
    Literal { datatype: TermId },
    /// A quoted triple refers to its components by native ID.
    Triple { s: TermId, p: TermId, o: TermId },
}

impl StructuredError for NativeIds {
    purrdf_hash::error_visit! {
        Leaf => "rdf-viz-term-without-component-ids" {},
        Literal { datatype } => "rdf-viz-literal-native-ids" {
            "datatype_index" => Unsigned(datatype.index() as u128),
        },
        Triple { s, p, o } => "rdf-viz-triple-native-ids" {
            "subject_index" => Unsigned(s.index() as u128),
            "predicate_index" => Unsigned(p.index() as u128),
            "object_index" => Unsigned(o.index() as u128),
        },
    }
}
