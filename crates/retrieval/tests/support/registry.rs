// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The mock property-function registry the admission, planner and search tests
//! plan against, and the request terms they ask it.
//!
//! The registry is a mock of the property-function seam and the statistics
//! provider is a plain caller-owned lookup; the fixtures are `example.org`
//! throughout.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::TermValue;
use purrdf_retrieval::fixture::iri;
use purrdf_retrieval::{
    Iri, Metric, Plan, RankFidelity, RankedStreamImpl, RejectionReason, RequestTerm,
    RetrievalRequest, SearchResult, Statistics, Term, block_on,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, CandidateDomains, DuplicatePolicy, EvalError, ExclusionBasis,
    PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, RankArithmetic,
    RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
};

pub fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

pub fn kernel_iri(text: &str) -> purrdf_core::Iri {
    purrdf_core::parse_iri(text).expect("fixture IRIs are valid")
}

/// Each accepted pattern, with the request term's value rendered into the
/// object-side position. The mocks are arity (1,1) and project `?c0`, so the
/// candidate is position 0 and a rendered facet binds at position 1.
///
/// An unconstrained `TermKind::Any` pattern is the exception: it declares **no**
/// placement at all, so it is matched by every request term and receives none of
/// them. Its argument stays free, and the plan reports every term that reached
/// only this producer — matching a shape is not the same as receiving it.
pub fn accepted(patterns: Vec<TermPattern>) -> Vec<AcceptedTerm> {
    patterns
        .into_iter()
        .map(|pattern| {
            let placements = if pattern == TermPattern::of_kind(TermKind::Any) {
                Vec::new()
            } else {
                vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }]
            };
            AcceptedTerm {
                pattern,
                placements,
            }
        })
        .collect()
}

/// A ranked declaration, supplied where a producer is registered. `mandatory`
/// is declared by the host rather than inferred: it states, explicitly, what an
/// unconstrained `TermKind::Any` pattern used to imply.
pub fn ranked(stratum: &str, patterns: Vec<TermPattern>, mandatory: bool) -> RankedDeclaration {
    RankedDeclaration {
        stratum: kernel_iri(stratum),
        accepted_terms: accepted(patterns),
        depth_placement: None,
        candidate_position: 0,
        duplicates: DuplicatePolicy::Unique,
        fidelity: RankFidelity::EXACT,
        arithmetic: RankArithmetic::FloatFree,
        domains: CandidateDomains::Unrestricted,
        block_position: None,
        exclusion: ExclusionBasis::Unavailable,
        mandatory,
    }
}

pub fn lexical_term() -> RequestTerm {
    RequestTerm::Lexical {
        text: "quick brown fox".to_owned(),
        language: Some("en".to_owned()),
        predicate: Some(iri(&ex("body"))),
    }
}

pub fn vector_term() -> RequestTerm {
    RequestTerm::Vector {
        embedding: vec![0.25, -1.5, 3.0],
        metric: Metric::Cosine,
        index_hint: None,
    }
}

pub fn seed_term() -> RequestTerm {
    RequestTerm::EntitySeed {
        entity: Term::new(format!("<{}>", ex("seed"))),
    }
}

pub fn mixed_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term(), vector_term(), seed_term()])
}

/// A lexical term beside an entity seed: the multimodal request a coverage-floor
/// host asks for, and the one the wider quantifier refused.
pub fn lexical_and_seed_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term(), seed_term()])
}

pub fn lexical_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term()])
}

/// A mock ranked producer that declares `rows` and emits `emitted`.
pub struct MockProducer {
    pub arity: PfArity,
    pub mode: BindingPattern,
    pub rows: u64,
    pub emitted: Vec<Vec<TermValue>>,
}

impl PropertyFunction for MockProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        std::slice::from_ref(&self.mode)
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        self.rows
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // A bound position is an input the call site supplied, and the engine
        // drops any row that disagrees with it there. Echoing the input back is
        // the cheapest correct behaviour, and it is what makes these fixtures
        // sensitive to the constants the compiler renders.
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        let mut rows: Vec<Vec<TermValue>> = Vec::with_capacity(self.emitted.len());
        for row in &self.emitted {
            let mut echoed = Vec::with_capacity(row.len());
            for (position, value) in row.iter().enumerate() {
                echoed.push(
                    bound
                        .get(position)
                        .and_then(Clone::clone)
                        .unwrap_or_else(|| value.clone()),
                );
            }
            rows.push(echoed);
        }
        Ok(Box::new(RowCursor {
            rows: rows.into_iter(),
        }))
    }
}

pub struct RowCursor {
    pub rows: std::vec::IntoIter<Vec<TermValue>>,
}

impl PfCursor for RowCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.rows.next())
    }
}

pub fn producer(rows: u64, prefix: &str, count: usize) -> Arc<dyn PropertyFunction> {
    let arity = PfArity::new(1, 1);
    let emitted = (0..count)
        .map(|index| {
            vec![
                TermValue::iri(format!("{}entity{index}", ex(prefix))),
                TermValue::iri(format!("{}score{index}", ex(prefix))),
            ]
        })
        .collect();
    Arc::new(MockProducer {
        arity,
        mode: arity.all_free_mode(),
        rows,
        emitted,
    })
}

/// The fixture registry: a catch-all producer the host declares mandatory, a
/// literal producer, an IRI-seed producer, and one unranked producer.
pub fn fixture_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let literal_pattern = TermPattern {
        kind: TermKind::Literal,
        datatype: None,
        language: Some("en".to_owned()),
        predicate: Some(ex("body")),
    };
    registry.register_ranked(
        ex("pf/any"),
        producer(200, "universal/", 3),
        ranked(
            &ex("stratum/universal"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    registry.register_ranked(
        ex("pf/literal"),
        producer(100, "text/", 2),
        ranked(&ex("stratum/text"), vec![literal_pattern], false),
    );
    registry.register_ranked(
        ex("pf/iri"),
        producer(50, "graph/", 1),
        ranked(
            &ex("stratum/graph"),
            vec![TermPattern::of_kind(TermKind::Iri)],
            false,
        ),
    );
    // Registered with no declaration at all: that is the whole of "does not
    // participate in ranked retrieval".
    registry.register(ex("pf/not-ranked"), producer(0, "unranked/", 0));
    registry
}

pub struct MockStatistics {
    pub source: String,
    pub revision: String,
    pub cardinalities: BTreeMap<Iri, u64>,
}

impl Statistics for MockStatistics {
    fn source(&self) -> &str {
        &self.source
    }

    fn revision(&self) -> &str {
        &self.revision
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        self.cardinalities.get(predicate).copied()
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

pub fn statistics(revision: &str) -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("stratum/universal")), 1000);
    cardinalities.insert(iri(&ex("stratum/text")), 100);
    cardinalities.insert(iri(&ex("stratum/graph")), 50);
    cardinalities.insert(iri(&ex("body")), 500);
    MockStatistics {
        source: "example-statistics".to_owned(),
        revision: revision.to_owned(),
        cardinalities,
    }
}

/// Read an executed stream the way a caller that stopped at `execute` reads it:
/// one row at a time through the ranked-stream protocol, to exhaustion.
pub fn drain(mut stream: RankedStreamImpl<'_>) -> Vec<(u64, Term)> {
    let mut rows = Vec::new();
    // The block each row names is not what these assertions are about — every
    // producer here declares `Unrestricted` and so names none — so it is dropped
    // by name rather than compared.
    while let Some((rank, candidate, _block)) =
        block_on(stream.next()).expect("a materialized stream obeys the protocol")
    {
        rows.push((rank, candidate));
    }
    rows
}

/// One `Value` placement at `position`, untyped.
pub fn value_at(position: usize) -> Vec<TermPlacement> {
    vec![TermPlacement {
        facet: RequestFacet::Value,
        position,
        datatype: None,
    }]
}

/// The `LIMIT` a unit's outer `SELECT` carries.
pub fn emitted_limit(sparql: &str) -> u32 {
    sparql
        .rsplit("LIMIT ")
        .next()
        .expect("an emitted unit carries a LIMIT")
        .trim()
        .parse()
        .expect("an emitted LIMIT is a number")
}

/// The rejection reason a plan recorded for `producer`, if it rejected it.
pub fn rejection(planned: &Plan, producer: &str) -> Option<RejectionReason> {
    planned
        .producer_decisions
        .iter()
        .find_map(|decision| match decision {
            purrdf_retrieval::ProducerDecision::Rejected {
                producer: name,
                reason,
            } if name == producer => Some(*reason),
            _ => None,
        })
}

/// The entities `result` returned, in order.
pub fn entities(result: &SearchResult) -> Vec<String> {
    result
        .rows
        .iter()
        .map(|row| row.entity.as_str().to_owned())
        .collect()
}
