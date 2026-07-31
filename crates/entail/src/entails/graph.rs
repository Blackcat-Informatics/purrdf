// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The owned, dataset-independent triple view both sides of a match are read through.
//!
//! # Why an owned view at all
//!
//! [`TermRef`](purrdf_core::TermRef) borrows into ONE dataset's term table and carries a
//! literal's datatype as a [`TermId`](purrdf_core::TermId) local to it, so two `TermRef`s
//! from two independently-parsed datasets cannot be compared: the same IRI is a different
//! id in each. A conclusion-directed service compares exactly that — a conclusion graph
//! the caller parsed against a closure this crate built — so both sides are resolved into
//! [`TermValue`], whose every coordinate is by value and therefore means the same thing in
//! every dataset.
//!
//! # Why only the default graph
//!
//! [`crate::materialize`] closes the default graph against itself and each named graph
//! against the union of itself and the default graph, landing each conclusion in the graph
//! that produced it. An entailment question asked of a DATASET therefore has to name which
//! graph answers it, and this service names the default graph: it is the graph SPARQL's
//! entailment regimes call the active graph by default, it is where an RDF/XML or Turtle
//! document's whole content lands, and it is where the chase's own conclusions about that
//! content land. Reading a named graph as part of the answer would let a conclusion be
//! "entailed" by a graph the question never mentioned.

use purrdf_core::{RdfDataset, TermValue};

/// A `(subject, predicate, object)` triple of owned, dataset-independent terms.
pub(crate) type Triple = [TermValue; 3];

/// Every default-graph triple of `ds`, as owned terms, in the dataset's frozen quad order.
///
/// Frozen order is a function of the dataset alone, so two runs over one dataset produce
/// the same vector — which is what keeps the index below, and every diagnostic built from
/// it, reproducible.
pub(crate) fn default_graph_triples(ds: &RdfDataset) -> Vec<Triple> {
    ds.quads()
        .filter(|quad| quad.g.is_none())
        .map(|quad| {
            [
                ds.term_value(quad.s),
                ds.term_value(quad.p),
                ds.term_value(quad.o),
            ]
        })
        .collect()
}

/// Render a term the way a diagnostic prints it.
///
/// Not a serialization: it is deliberately lossy (a literal's datatype is dropped unless it
/// is the only thing that distinguishes it) because its only consumer is a human reading a
/// [`MissReason`](super::homomorphism::MissReason). Anything that must round-trip goes
/// through a codec, not through this.
pub(crate) fn show(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => format!("<{iri}>"),
        TermValue::Blank { label, scope } => format!("_:{label}#{}", scope.ordinal()),
        TermValue::Literal {
            lexical_form,
            language,
            ..
        } => language.as_ref().map_or_else(
            || format!("{lexical_form:?}"),
            |lang| format!("{lexical_form:?}@{lang}"),
        ),
        TermValue::Triple { s, p, o } => {
            format!("<<{} {} {}>>", show(s), show(p), show(o))
        }
    }
}
