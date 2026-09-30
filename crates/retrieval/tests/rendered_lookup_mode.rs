// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A unit this layer renders attaches its producer's exclusion basis only where
//! the lookup it would ask is one the producer declares a mode for.
//!
//! Registration admits a declared basis against the widest lookup any text can
//! ask — every position but the depth supplied — because a caller's own text can
//! fill a position no request facet reaches. The rendered lookup fills only the
//! candidate and the placed constants, so a producer whose only candidate-bound
//! point mode also binds a position no placement fills (`bbfb` below: the
//! candidate, the needle and a trailing argument bound, the depth free) is one a
//! rendered stratum's lookup cannot be asked of. Attaching the basis anyway made
//! every such stratum fail at search time, when the lookup would not prepare.
//! The basis was the producer's, attached without the caller asking for it, so
//! the rendered unit declares none instead: the stratum ranks, the fusion reads a
//! stream that answers no lookup, and the trailer says so.
//!
//! Beside it, executed: the neighbour whose point mode differs only in leaving
//! that trailing position free (`bbff`) keeps its basis and is looked up — the
//! lookup count, read from the producer's side, is the oracle that tells a basis
//! honoured from one dropped — and both answer exactly what the same producer
//! answers declaring no basis at all.
//!
//! Fixtures use `example.org` throughout; every IRI below is fixture
//! configuration, never a minted vocabulary.

use purrdf_core::term_fixture::empty_dataset;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, DomainTag, DuplicatePolicy, Iri, ProducerStatus,
    RankFidelity, RequestTerm, SearchResult, Statistics, compile, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, DepthPlacement, ExclusionBasis, PfArity, PropertyFunctionRegistry,
    RankArithmetic, RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement,
};

#[path = "support/lookup.rs"]
mod lookup;

use lookup::{
    PREDICATES, PREFIXES, Producer, ROWS, asked, ex, expected_entities, producer_iri, profile,
    request, strata,
};
use purrdf_retrieval::block_on;

/// The streaming mode every producer declares: the needle and the depth bound,
/// the candidate and the trailing argument free.
const RANKED: &str = "fbbf";

/// A point mode binding the candidate, the needle and the trailing argument — a
/// position no placement fills — with the depth free.
const EXTRA_BOUND: &str = "bbfb";

/// The neighbouring point mode: the candidate and the needle bound, everything
/// else free.
const EXTRA_FREE: &str = "bbff";

/// Register both strata's producers under [`RANKED`] and `point`, declaring
/// `exclusion`. The needle is placed at position 1, the depth at position 2, and
/// position 3 is placed by nothing.
///
/// Both declare the one shared domain, so neither stratum's absence of a candidate
/// is settled by a declaration: only a lookup can settle it early.
fn registry(
    point: &str,
    exclusion: ExclusionBasis,
) -> (PropertyFunctionRegistry, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let shared = DomainTag::parse(&ex("domain/shared")).expect("fixture domain tags are IRIs");
    let mut registry = PropertyFunctionRegistry::new();
    for ((predicate, stratum), prefix) in PREDICATES.into_iter().zip(strata()).zip(PREFIXES) {
        registry.register_ranked(
            producer_iri(predicate),
            Arc::new(Producer {
                arity: PfArity::new(1, 3),
                modes: vec![
                    BindingPattern::from_code(RANKED),
                    BindingPattern::from_code(point),
                ],
                prefix,
                asked: Arc::clone(&asked),
            }),
            RankedDeclaration {
                stratum: purrdf_core::parse_iri(stratum.as_str()).expect("fixture IRI"),
                accepted_terms: vec![AcceptedTerm {
                    pattern: TermPattern {
                        kind: TermKind::Literal,
                        datatype: None,
                        language: Some("en".to_owned()),
                        predicate: Some(ex(predicate)),
                    },
                    placements: vec![TermPlacement {
                        facet: RequestFacet::Value,
                        position: 1,
                        datatype: None,
                    }],
                }],
                depth_placement: Some(DepthPlacement {
                    position: 2,
                    datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                }),
                candidate_position: 0,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                arithmetic: RankArithmetic::FloatFree,
                domains: CandidateDomains::within([shared.clone()]),
                block_position: None,
                exclusion,
                mandatory: false,
            },
        );
    }
    (registry, asked)
}

struct FixtureStatistics;

impl Statistics for FixtureStatistics {
    fn source(&self) -> &'static str {
        "example-statistics"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, _predicate: &Iri) -> Option<u64> {
        Some(ROWS)
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

/// The basis each compiled unit declares, by stratum, as [`compile`] emits it.
fn compiled_bases(point: &str, exclusion: ExclusionBasis) -> Vec<(String, ExclusionBasis)> {
    let (registry, _) = registry(point, exclusion);
    let statistics = FixtureStatistics;
    let profile = profile();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let plan = plan(&request(), &registry, &statistics).expect("the fixture request plans");
    let compiled = compile(&plan, &env).expect("the fixture plan compiles");
    compiled
        .units
        .iter()
        .map(|unit| (unit.stratum.as_str().to_owned(), unit.contract.exclusion))
        .collect()
}

/// What one search answered, and what its producers were asked.
#[derive(Debug, PartialEq, Eq)]
struct Searched {
    /// The fused entities, in answer order.
    entities: Vec<String>,
    /// How many invocations each mode was asked, by mode code, ascending.
    asked: Vec<(String, usize)>,
    /// The basis the trailer reports for each stratum, ascending.
    bases: Vec<(String, ExclusionBasis)>,
    /// Every stratum the search reported as failing to execute.
    failed: Vec<String>,
}

/// Register under `point` and `exclusion`, and run the fixture request.
fn searched(point: &str, exclusion: ExclusionBasis) -> Searched {
    let (registry, asked) = registry(point, exclusion);
    let statistics = FixtureStatistics;
    let profile = profile();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let dataset = empty_dataset();
    let result: SearchResult = block_on(search(
        &request(),
        &registry,
        &statistics,
        dataset.as_ref(),
        &env,
        &profile,
    ))
    .expect("the fixture search answers");
    let entities = result
        .rows
        .iter()
        .map(|row| row.entity.as_str().to_owned())
        .collect();
    let mut counts = BTreeMap::<String, usize>::new();
    for mode in asked
        .lock()
        .expect("the fixture's record is never poisoned")
        .iter()
    {
        *counts.entry(mode.clone()).or_default() += 1;
    }
    Searched {
        entities,
        asked: counts.into_iter().collect(),
        bases: result
            .trailer
            .exclusion_bases
            .iter()
            .map(|(stratum, basis)| (stratum.as_str().to_owned(), *basis))
            .collect(),
        failed: result
            .trailer
            .statuses
            .iter()
            .filter(|(_, status)| matches!(status, ProducerStatus::ExecutionFailed { .. }))
            .map(|(stratum, _)| stratum.as_str().to_owned())
            .collect(),
    }
}

/// Every stratum under `basis`, in the ascending order the trailer and the
/// compiled bundle both report them in.
fn bases(basis: ExclusionBasis) -> Vec<(String, ExclusionBasis)> {
    let mut all: Vec<(String, ExclusionBasis)> = strata()
        .into_iter()
        .map(|stratum| (stratum.as_str().to_owned(), basis))
        .collect();
    all.sort_by(|left, right| left.0.cmp(&right.0));
    all
}

/// **A rendered stratum whose producer's only point mode binds a position the
/// rendered lookup leaves free is compiled declaring no basis, and searches
/// exactly as the basis-less producer does — never failing at search time.**
#[test]
fn a_rendered_lookup_no_declared_mode_serves_attaches_no_basis() {
    // Registration admits the basis: a caller's own text can fill position 3, so
    // the widest lookup is served by `bbfb`.
    let (registry, _) = registry(EXTRA_BOUND, ExclusionBasis::Membership);
    assert_eq!(
        registry
            .ranked_declaration(&producer_iri("title"))
            .expect("the producer registered")
            .exclusion,
        ExclusionBasis::Membership,
        "the producer's declaration keeps the basis it registered"
    );

    assert_eq!(
        compiled_bases(EXTRA_BOUND, ExclusionBasis::Membership),
        bases(ExclusionBasis::Unavailable),
        "the rendered lookup would ask `bbff`, which `bbfb` does not serve, so each \
         rendered unit is compiled declaring no basis"
    );

    let unavailable = searched(EXTRA_BOUND, ExclusionBasis::Unavailable);
    assert_eq!(
        unavailable,
        Searched {
            entities: expected_entities(),
            asked: asked(&[(RANKED, 2)]),
            bases: bases(ExclusionBasis::Unavailable),
            failed: Vec::new(),
        },
        "without a basis each stratum is read once, ranked, and never looked up"
    );
    assert_eq!(
        searched(EXTRA_BOUND, ExclusionBasis::Membership),
        unavailable,
        "the unit that cannot ask its lookup ranks, answers and reports exactly as the \
         basis-less producer does: no stratum fails, and the trailer names no basis"
    );
}

/// **The neighbour whose point mode leaves that position free keeps its basis,
/// is looked up in exactly that mode, and answers what the basis-less producer
/// answers.**
#[test]
fn a_rendered_lookup_a_declared_mode_serves_keeps_its_basis_and_is_asked() {
    assert_eq!(
        compiled_bases(EXTRA_FREE, ExclusionBasis::Membership),
        bases(ExclusionBasis::Membership),
        "`bbff` serves the rendered lookup, so the basis is attached"
    );

    let unavailable = searched(EXTRA_FREE, ExclusionBasis::Unavailable);
    assert_eq!(
        unavailable,
        Searched {
            entities: expected_entities(),
            asked: asked(&[(RANKED, 2)]),
            bases: bases(ExclusionBasis::Unavailable),
            failed: Vec::new(),
        },
        "without a basis nothing is looked up"
    );
    let membership = searched(EXTRA_FREE, ExclusionBasis::Membership);
    assert_eq!(
        membership,
        Searched {
            entities: expected_entities(),
            asked: asked(&[(EXTRA_FREE, 130), (RANKED, 2)]),
            bases: bases(ExclusionBasis::Membership),
            failed: Vec::new(),
        },
        "with a served point mode the lookups run in exactly that mode"
    );
    assert_eq!(
        membership.entities, unavailable.entities,
        "and change no answer"
    );
}
