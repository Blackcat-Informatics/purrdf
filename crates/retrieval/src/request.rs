// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The typed retrieval request: what is being asked for, and how much of it.
//!
//! A request carries two things. The terms are what is being looked for; the
//! [`ReadBound`] is how much of the answer is wanted, and it is here rather than
//! at the last stage because it is the one input that decides how deep the read
//! has to be. A bound known only at `fuse` is a bound that arrives after every
//! stratum's depth has already been recorded and emitted.
//!
//! The producers this layer composes consume different modalities: a needle for
//! a BM25 relation, a seed term for a nearest-neighbour relation, a geometry for
//! a spatial one. A request is therefore a list of typed terms, and a producer
//! declares which shapes it accepts as serializable data
//! (`purrdf_sparql_eval::TermPattern`). Matching is a lookup over those
//! declarations, never inference.
//!
//! A term shape being expressible here is not a claim that some producer
//! accepts it. [`RequestTerm::Vector`] is the standing example and says so on
//! its own documentation: it renders, and a producer that declares the datatype
//! receives it, but no relation in this workspace declares that shape yet.
//! [`RequestTerm::Spatial`] is a second: the spatial
//! relation this workspace ships computes a **set** — it sorts and deduplicates
//! its pairs and carries neither a score nor a rank — so it composes as a
//! constraint on candidates rather than as a stratum of a fused ranking, and it
//! is not declared ranked here. [`RequestTerm::Temporal`] and
//! [`RequestTerm::NumericRange`] are a third and fourth: no relation in this
//! workspace accepts either shape today.
//!
//! # The lattice is closed, deliberately
//!
//! [`RequestTerm`] is a closed enum, and it stays closed. The layer's doctrine
//! is that producers are caller-supplied configuration, so the modalities a
//! caller can ask for are not bounded by the producers that happen to exist
//! in-tree — which is precisely why the interval modalities above are here
//! before any producer takes them. Carrying a modality ahead of its producer is
//! honest only when an unanswered term says so, and one does: a term no
//! declaration accepts is reported per term as
//! [`UnservedReason::NoProducerAccepts`](crate::UnservedReason::NoProducerAccepts).
//!
//! Closed rather than `#[non_exhaustive]` is the deliberate choice, and the
//! reason is what a caller gets when a modality is added later: an exhaustive
//! `match` over this enum stops compiling, which is exactly the signal a caller
//! routing terms to its own producers wants. `#[non_exhaustive]` would replace
//! that compile error with a wildcard arm that silently swallows the new
//! modality, and forcing a wildcard arm on every caller is the cost it charges
//! for the semver relief. The relief is worth less than the signal here,
//! because adding variants before first publication is free.

use purrdf_text::Fixed;
use serde::{Deserialize, Serialize};

use crate::fuse::TopK;
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
/// that accepts none of them is not reached by that term. `PartialEq` (and
/// therefore `Eq`) is implemented by hand because the vector arm carries
/// `f32`: two embeddings are equal iff their bit patterns are equal (compared
/// via [`f32::to_bits`]), which is the identity a plan's canonical encoding
/// ([`Plan::canonical_bytes`](crate::Plan::canonical_bytes)) writes them under,
/// so two terms are equal exactly when their encodings are. Bit-pattern
/// equality means `0.0f32` and `-0.0f32` are
/// distinct (their bits differ) and a `NaN` embedding equals itself
/// (`to_bits()` is a total, reflexive function even though `f32`'s `PartialOrd`
/// is not), so this type's `PartialEq` is a genuine equivalence relation and
/// `Eq` holds for every value, including deserialized or otherwise untrusted
/// ones.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    ///
    /// # It renders, and a producer that declares the datatype receives it
    ///
    /// A query embedding has a constant form: the components' exact bit
    /// patterns, written as one literal by
    /// [`encode_embedding`](crate::encode_embedding) and read back by
    /// [`decode_embedding`](crate::decode_embedding). Like a geometry, it is
    /// written under a datatype the *producer* declares — required, never
    /// fabricated, because PurRDF mints no vocabulary — and the producer that
    /// declares it owns the parse. A producer whose value placement declares no
    /// datatype is refused at placement rather than handed an embedding under a
    /// datatype this layer invented.
    ///
    /// A producer accepting [`TermKind::Literal`](purrdf_sparql_eval::TermKind)
    /// is therefore reachable by this arm, exactly as it is by a needle, a
    /// geometry or an interval endpoint: what all of those have in common is the
    /// RDF term kind they are written as.
    ///
    /// # No producer in this workspace accepts this shape yet
    ///
    /// The nearest-neighbour producer this repository ships
    /// (`purrdf_sparql_eval::EmbeddingKnnRelation`) searches *from a term it
    /// already holds a vector for*: it takes a seed term and looks its row up in
    /// its own space. Its accepted request shape is therefore [`Self::EntitySeed`],
    /// not this arm, and this arm waits for a producer that takes the components
    /// themselves.
    ///
    /// It is deliberately **not** aliased to `EntitySeed` in the meantime. A
    /// seed lookup and a literal embedding query are different modalities: one
    /// names an RDF term the space must already hold and answers "what is near
    /// this thing", the other carries the coordinates and answers "what is near
    /// this point", and a space can serve either without serving both. They need
    /// different capability declarations — different accepted patterns,
    /// different placements, different failure when the space cannot answer — so
    /// folding them together would let a request for one be silently answered by
    /// the other, which is precisely the class of wrong answer the declaration
    /// machinery exists to make impossible.
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
    /// A temporal term: a closed interval on the caller's own time line.
    ///
    /// Each endpoint is carried in the caller's own temporal lexical form, the
    /// way [`Self::Spatial`] carries a geometry in the caller's own geometry
    /// encoding, and for the same reason: PurRDF mints no calendar datatype and
    /// parses none, so the layer carries the lexical verbatim and the producer's
    /// own [`TermPlacement`](purrdf_sparql_eval::TermPlacement) declares which
    /// datatype it is written under. A producer that declares no datatype for
    /// the endpoint is refused at placement rather than handed a plain string
    /// that would compare as one.
    ///
    /// # No producer in this workspace accepts this shape yet
    ///
    /// Both endpoints nonetheless render, because both have a SPARQL constant
    /// form: they bind through
    /// [`RequestFacet::LowerBound`](purrdf_sparql_eval::RequestFacet::LowerBound)
    /// and [`UpperBound`](purrdf_sparql_eval::RequestFacet::UpperBound).
    ///
    /// # Both endpoints are inclusive, and that is the whole interval language
    ///
    /// An argument position carries a value, not a comparison operator, so
    /// expressing a strict endpoint would mean carrying an operator the seam
    /// declares no facet for — vocabulary this layer does not own and does not
    /// mint. A caller that wants a strict endpoint narrows the endpoint itself,
    /// which its own temporal encoding can always express.
    ///
    /// An absent endpoint is a half-open interval, and an interval with neither
    /// endpoint constrains nothing and is refused when the request is planned.
    Temporal {
        /// The predicate IRI the interval constrains.
        predicate: Iri,
        /// The inclusive lower endpoint, in the caller's temporal lexical form,
        /// or `None` for an interval unbounded below.
        lower: Option<String>,
        /// The inclusive upper endpoint, in the caller's temporal lexical form,
        /// or `None` for an interval unbounded above.
        upper: Option<String>,
    },
    /// A numeric range term: a closed interval, exact.
    ///
    /// The endpoints are [`Fixed`] — the same exact base-10 type a stratum
    /// weight and a [`Self::Spatial`] maximum distance are carried in —
    /// never `f64`. Two reasons, and either alone would settle it. A binary
    /// float cannot represent most decimal endpoints a caller writes, so the
    /// range that reaches the producer would not be the range the caller asked
    /// for; and a request term is part of a plan's canonical identity, where a
    /// type with two spellings of one value (`0.0` and `-0.0`) and a value that
    /// is not equal to itself (`NaN`) has no place. `Fixed` renders losslessly
    /// through `to_decimal_lexical`, so the emitted constant is exactly the
    /// endpoint.
    ///
    /// # No producer in this workspace accepts this shape yet
    ///
    /// Both endpoints render, through the same
    /// [`LowerBound`](purrdf_sparql_eval::RequestFacet::LowerBound) and
    /// [`UpperBound`](purrdf_sparql_eval::RequestFacet::UpperBound) facets
    /// [`Self::Temporal`] uses; the producer declares the numeric datatype its
    /// own relation compares under.
    ///
    /// # Both endpoints are inclusive
    ///
    /// For the reason given on [`Self::Temporal`]. Here the caller's workaround
    /// is exact rather than merely available: at the declared scale the value
    /// immediately below an endpoint is `Fixed::from_raw(raw - 1)`, so a strict
    /// bound is representable without approximation.
    ///
    /// An interval with neither endpoint constrains nothing, and one whose
    /// lower endpoint exceeds its upper can match nothing; both are refused when
    /// the request is planned. A degenerate interval whose endpoints are equal
    /// is a single point and is admitted.
    NumericRange {
        /// The predicate IRI the interval constrains.
        predicate: Iri,
        /// The inclusive lower endpoint, or `None` for a range unbounded below.
        #[serde(with = "crate::iri::fixed_option")]
        lower: Option<Fixed>,
        /// The inclusive upper endpoint, or `None` for a range unbounded above.
        #[serde(with = "crate::iri::fixed_option")]
        upper: Option<Fixed>,
    },
    /// An entity seed: retrieve from a term the caller already knows.
    EntitySeed {
        /// The seed term.
        entity: Term,
    },
}

impl PartialEq for RequestTerm {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Lexical {
                    text: left_text,
                    language: left_language,
                    predicate: left_predicate,
                },
                Self::Lexical {
                    text: right_text,
                    language: right_language,
                    predicate: right_predicate,
                },
            ) => {
                left_text == right_text
                    && left_language == right_language
                    && left_predicate == right_predicate
            }
            (
                Self::Vector {
                    embedding: left_embedding,
                    metric: left_metric,
                    index_hint: left_index_hint,
                },
                Self::Vector {
                    embedding: right_embedding,
                    metric: right_metric,
                    index_hint: right_index_hint,
                },
            ) => {
                left_metric == right_metric
                    && left_index_hint == right_index_hint
                    && left_embedding.len() == right_embedding.len()
                    && left_embedding
                        .iter()
                        .zip(right_embedding)
                        .all(|(left, right)| left.to_bits() == right.to_bits())
            }
            (
                Self::Spatial {
                    geometry: left_geometry,
                    predicate: left_predicate,
                    max_distance: left_max_distance,
                },
                Self::Spatial {
                    geometry: right_geometry,
                    predicate: right_predicate,
                    max_distance: right_max_distance,
                },
            ) => {
                left_geometry == right_geometry
                    && left_predicate == right_predicate
                    && left_max_distance == right_max_distance
            }
            (
                Self::Temporal {
                    predicate: left_predicate,
                    lower: left_lower,
                    upper: left_upper,
                },
                Self::Temporal {
                    predicate: right_predicate,
                    lower: right_lower,
                    upper: right_upper,
                },
            ) => {
                left_predicate == right_predicate
                    && left_lower == right_lower
                    && left_upper == right_upper
            }
            (
                Self::NumericRange {
                    predicate: left_predicate,
                    lower: left_lower,
                    upper: left_upper,
                },
                Self::NumericRange {
                    predicate: right_predicate,
                    lower: right_lower,
                    upper: right_upper,
                },
            ) => {
                left_predicate == right_predicate
                    && left_lower == right_lower
                    && left_upper == right_upper
            }
            (
                Self::EntitySeed {
                    entity: left_entity,
                },
                Self::EntitySeed {
                    entity: right_entity,
                },
            ) => left_entity == right_entity,
            (
                Self::Lexical { .. }
                | Self::Vector { .. }
                | Self::Spatial { .. }
                | Self::Temporal { .. }
                | Self::NumericRange { .. }
                | Self::EntitySeed { .. },
                _,
            ) => false,
        }
    }
}

impl Eq for RequestTerm {}

/// How much of the answer a request is for.
///
/// This is a **read** bound before it is a row bound, and that is why it belongs
/// on the request rather than on the last stage. A caller that wants five rows
/// out of a stratum a producer declares a thousand rows for has told the planner
/// something the planner cannot otherwise learn: the depth it is about to record
/// does not have to be the declaration. So the bound arrives with the terms, the
/// planner derives each stratum's depth from it (see
/// [`plan`](crate::plan)), and the depth a plan records stays the depth that is
/// actually read — rather than a number narrowed later, at emission, while the
/// plan went on recording a read nobody took.
///
/// # Why an enum over a bound and a complete case, rather than an optional bound
///
/// Both arms are requests. "Give me the top five" and "give me everything these
/// strata hold" are two things a caller asks for, and neither is the absence of
/// the other: a caller that stops at the unfused rung — walking one stratum's
/// rows itself, with no fusion law in the path — is asking for the second, in
/// full, on purpose. An `Option` would spell that as a missing value and leave
/// every reader to decide what a missing bound licenses, which is exactly the
/// decision this type exists to take once.
///
/// It is also what keeps the derivation total. [`Self::Complete`] is not "no
/// narrowing applies"; it is "the narrowing this request licenses is none", and
/// the planner reads it as a value like any other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReadBound {
    /// At most this many fused rows, and therefore no deeper a read than those
    /// rows can come from.
    ///
    /// What the planner may do with it depends on what the producers declared
    /// about their own candidates, and on nothing else — see [`plan`](crate::plan)
    /// for the rule and its proof. A bound is never a licence to read *more*: it
    /// narrows a depth the registry and the statistics already set, or it changes
    /// nothing.
    Bounded(TopK),
    /// Every row the request's strata can yield, to the depth their own
    /// declarations and the statistics allow.
    ///
    /// The honest request of a caller that means to consume a whole stratum: the
    /// unfused rung, an export, a re-ranker that wants the candidate set rather
    /// than a prefix of it. It licenses no narrowing, so the depths are exactly
    /// the ones the registry declared and the statistics bounded.
    Complete,
}

/// A request: the ordered list of terms a plan is built for, and how much of the
/// answer it is for.
///
/// The list order is identity-bearing. It is the order the caller wrote, and
/// the planner binds request-term indices against it (see
/// [`ProducerBinding`](crate::ProducerBinding)).
///
/// There is deliberately no `Default`. An empty term list is a coherent value,
/// but a default [`ReadBound`] is not: both arms are things a caller asks for,
/// and picking one on the caller's behalf would decide how deep its read goes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalRequest {
    /// The request's terms, in caller order.
    pub terms: Vec<RequestTerm>,
    /// How much of the answer is wanted, and therefore how much of each stratum
    /// may have to be read to assemble it.
    pub bound: ReadBound,
}

impl RetrievalRequest {
    /// Build a request from explicit terms and an explicit bound.
    #[must_use]
    pub const fn from_terms(terms: Vec<RequestTerm>, bound: ReadBound) -> Self {
        Self { terms, bound }
    }

    /// Build a request for at most `top_k` fused rows.
    #[must_use]
    pub const fn bounded(terms: Vec<RequestTerm>, top_k: TopK) -> Self {
        Self::from_terms(terms, ReadBound::Bounded(top_k))
    }

    /// Build a request for everything the request's strata can yield.
    #[must_use]
    pub const fn complete(terms: Vec<RequestTerm>) -> Self {
        Self::from_terms(terms, ReadBound::Complete)
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
