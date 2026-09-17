// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The typed retrieval request.
//!
//! The producers this layer composes consume different modalities: lexical
//! terms for a BM25 relation, embedding vectors for a nearest-neighbour
//! relation, geometries for a spatial one. A request is therefore a list of
//! typed terms, and a producer declares which shapes it accepts as serializable
//! data (`purrdf_sparql_eval::TermPattern`). Matching is a lookup over those
//! declarations, never inference.

use purrdf_text::Fixed;
use serde::{Deserialize, Serialize};

use crate::iri::{Iri, Term};

/// The distance metric a vector request is expressed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Metric {
    /// Cosine distance.
    Cosine,
    /// Dot-product similarity.
    Dot,
    /// Euclidean (L2) distance.
    Euclidean,
}

/// One term of a retrieval request.
///
/// The enum is closed: a request term is one of these shapes, and a producer
/// that accepts none of them is not reached by that term. `Eq` is implemented
/// by hand because the vector arm carries `f32`: two embeddings are equal iff
/// their bit patterns are equal, which is the identity a plan's canonical
/// encoding uses. Callers must not place a NaN in an embedding; the canonical
/// encoding preserves any bit pattern, so a NaN would only make `PartialEq`
/// disagree with reflexivity, never corrupt the encoding.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RequestTerm {
    /// A lexical (full-text) term.
    Lexical {
        /// The needle text.
        text: String,
        /// An optional BCP 47 language tag the needle is restricted to.
        language: Option<String>,
        /// An optional predicate IRI the term must be associated with.
        predicate: Option<Iri>,
    },
    /// A vector (embedding) term.
    Vector {
        /// The query embedding.
        embedding: Vec<f32>,
        /// The metric the embedding is expressed in.
        metric: Metric,
        /// An optional caller-supplied index hint.
        index_hint: Option<String>,
    },
    /// A spatial term.
    Spatial {
        /// The query geometry in the caller's geometry encoding.
        geometry: String,
        /// The predicate IRI the geometry is associated with.
        predicate: Iri,
        /// An optional maximum distance.
        #[serde(with = "crate::iri::fixed_option")]
        max_distance: Option<Fixed>,
    },
    /// An entity seed: retrieve from a term the caller already knows.
    EntitySeed {
        /// The seed term.
        entity: Term,
    },
}

impl Eq for RequestTerm {}

/// A request: the ordered list of terms a plan is built for.
///
/// The list order is identity-bearing. It is the order the caller wrote, and
/// the planner binds request-term indices against it (see
/// [`ProducerBinding`](crate::ProducerBinding)).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalRequest {
    /// The request's terms, in caller order.
    pub terms: Vec<RequestTerm>,
}

impl RetrievalRequest {
    /// An empty request.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a request from explicit terms.
    #[must_use]
    pub fn from_terms(terms: Vec<RequestTerm>) -> Self {
        Self { terms }
    }

    /// The number of terms in the request.
    #[must_use]
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// Whether the request carries no terms.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}
