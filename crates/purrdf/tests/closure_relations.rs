// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`purrdf::ClosureRelations`] — the materialize-then-register order, at the public API.
//!
//! A property-function registry is built by the caller, before the call. For an ordinary
//! query that is unremarkable, because the caller holds the dataset the query runs over.
//! For an entailment-regime query it is not: the dataset the query is evaluated over is
//! the CLOSURE, which the entry point materializes internally and the caller never holds,
//! so a relation the caller snapshotted answers about the pre-closure data while every
//! other pattern in the same query reads the closure.
//!
//! That divergence was real, silent, and complete-looking: over the fixture below the
//! walk answered `{b}` and `ex:p+` in the same query answered `{b, c}`, both at success.
//! These tests hold the fix to three separate claims — the rebuilt registry sees the
//! derived edge, [`ClosureRelations::NONE`] leaves the pre-existing behaviour byte for
//! byte, and the one pairing that IS refused refuses on witnesses actually minted rather
//! than on a regime's name.

use std::sync::Arc;

use purrdf::sparql::{
    NativeSparqlEngine, PathDirection, PathGraph, PathLimits, PathStep, PathWitnessRelation,
    PropertyFunctionRegistry, QueryGovernors, QueryOptions,
};
use purrdf::{
    ClosureRelations, GovernedEntailment, GraphMatch, QueryEntailment, RdfDataset,
    RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue, parse_dataset,
    query_with_entailment_governed,
};

const NS: &str = "http://example.org/";
const RDFS_SUBPROPERTY: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const WALK: &str = "http://example.org/pf#walk";

/// `ex:sub rdfs:subPropertyOf ex:p . ex:a ex:p ex:b . ex:b ex:sub ex:c .`
///
/// The RDFS closure DERIVES `ex:b ex:p ex:c`, which the assertion does not carry — so a
/// step over `ex:p` has exactly one edge before materialization and two after. That gap is
/// the whole experiment.
fn subproperty_chain() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let sub_property_of = b.intern_iri(RDFS_SUBPROPERTY);
    let sub = b.intern_iri(&format!("{NS}sub"));
    let p = b.intern_iri(&format!("{NS}p"));
    let a = b.intern_iri(&format!("{NS}a"));
    let b_node = b.intern_iri(&format!("{NS}b"));
    let c = b.intern_iri(&format!("{NS}c"));
    b.push_quad(sub, sub_property_of, p, None);
    b.push_quad(a, p, b_node, None);
    b.push_quad(b_node, sub, c, None);
    b.freeze().expect("the fixture freezes")
}

/// `A ⊑ ∃r.B`, `a : A` — the ontology whose combined-approach answer needs a chase-minted
/// existential witness.
fn some_values_from() -> Arc<RdfDataset> {
    let turtle = concat!(
        "@prefix ex: <http://example.org/> .\n",
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
        "ex:A a owl:Class .\n",
        "ex:B a owl:Class .\n",
        "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; \
         owl:someValuesFrom ex:B ] .\n",
        "ex:a a ex:A .\n",
    );
    parse_dataset(turtle.as_bytes(), "text/turtle", None).expect("the ontology parses")
}

/// `ex:A owl:equivalentClass ex:B` puts the TBox OUTSIDE the combined approach's Horn
/// fragment, so that lane declines and mints no witness at all.
fn equivalent_class() -> Arc<RdfDataset> {
    let turtle = concat!(
        "@prefix ex: <http://example.org/> .\n",
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
        "ex:A a owl:Class .\n",
        "ex:B a owl:Class .\n",
        "ex:A owl:equivalentClass ex:B .\n",
        "ex:a a ex:A .\n",
        "ex:a ex:p ex:b .\n",
    );
    parse_dataset(turtle.as_bytes(), "text/turtle", None).expect("the ontology parses")
}

/// A one-alternative `ex:p`-forward walk registered at [`WALK`], snapshotted over
/// `dataset`.
fn walk_registry(dataset: &RdfDataset) -> PropertyFunctionRegistry {
    let step = PathStep::new(vec![(
        TermValue::iri(format!("{NS}p")),
        PathDirection::Forward,
    )])
    .expect("one forward alternative is a step");
    let graph = Arc::new(
        PathGraph::from_dataset(dataset, &step, GraphMatch::Default).expect("the step snapshots"),
    );
    let limits = PathLimits::new(1, 4, 1024, 100_000).expect("the envelope is buildable");
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        WALK.to_owned(),
        Arc::new(PathWitnessRelation::new(graph, limits)),
    );
    registry
}

/// The endpoints a `SELECT ?end` answer bound, sorted and deduplicated.
fn endpoints(result: &SparqlResult) -> Vec<String> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected a solution sequence");
    };
    let mut ends: Vec<String> = rows
        .iter()
        .map(|row| match row[0].as_ref().expect("?end is bound") {
            TermValue::Iri(iri) => iri.clone(),
            other => panic!("?end must bind an IRI, got {other:?}"),
        })
        .collect();
    ends.sort();
    ends.dedup();
    ends
}

/// The `SELECT ?end` over the registered walk, seeded at `ex:a`.
const WALK_QUERY: &str = "SELECT ?end WHERE { <http://example.org/a> \
                          <http://example.org/pf#walk> \
                          ( ?end ?pathId ?len ?step ?node ?edge ) } ORDER BY ?end";

/// The same endpoint set, written with the core grammar instead of a relation.
const GRAMMAR_QUERY: &str =
    "SELECT ?end WHERE { <http://example.org/a> <http://example.org/p>+ ?end } ORDER BY ?end";

/// Answer `query` over `dataset` under `entailment`, with `relations` deciding whether the
/// registry is re-derived over the closure.
fn answer(
    dataset: &Arc<RdfDataset>,
    query: &str,
    entailment: QueryEntailment<'_>,
    registry: &PropertyFunctionRegistry,
    relations: &ClosureRelations<'_>,
) -> Result<SparqlResult, purrdf::ReasoningError> {
    let outcome = query_with_entailment_governed(
        &NativeSparqlEngine::new(),
        dataset,
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        entailment,
        QueryOptions {
            property_functions: registry,
            ..QueryOptions::EMPTY
        },
        relations,
        &QueryGovernors::UNBOUNDED,
    )?;
    let GovernedEntailment::Answered { outcome, .. } = outcome else {
        panic!("an unbounded run is not stopped");
    };
    let purrdf::sparql::GovernedOutcome::Complete { result, .. } = outcome else {
        panic!("an unbounded run completes");
    };
    Ok(result)
}

/// A REBUILT REGISTRY WALKS THE CLOSURE, AND AGREES WITH `p+` OVER IT.
///
/// The two halves of one query must read one dataset. Both are run here under the same
/// regime over the same fixture, and their endpoint sets are compared to each other rather
/// than to a constant, so the assertion is the invariant itself.
#[test]
fn a_rebuilt_registry_walks_the_closure_and_agrees_with_p_plus() {
    let dataset = subproperty_chain();
    let source_registry = walk_registry(&dataset);
    let rebuild = |closure: &RdfDataset| Ok(walk_registry(closure));

    let walked = answer(
        &dataset,
        WALK_QUERY,
        QueryEntailment::Rdfs,
        &source_registry,
        &ClosureRelations::rebuilt_by(&rebuild),
    )
    .expect("the rebuilt walk answers");
    let grammar = answer(
        &dataset,
        GRAMMAR_QUERY,
        QueryEntailment::Rdfs,
        &PropertyFunctionRegistry::EMPTY,
        &ClosureRelations::NONE,
    )
    .expect("the grammar answers");

    assert_eq!(
        endpoints(&walked),
        endpoints(&grammar),
        "the walk's endpoint projection must agree with p+ under the same regime"
    );
    assert_eq!(
        endpoints(&walked),
        vec![format!("{NS}b"), format!("{NS}c")],
        "and the set both reach is the closure's, not the assertion's"
    );
}

/// `NONE` IS THE OLD BEHAVIOUR, EXACTLY — WHICH IS WHY IT IS NOT THE DEFAULT FOR A
/// DATASET-DERIVED RELATION.
///
/// The same fixture, the same regime, the same registry, and only the parameter differs:
/// with `NONE` the caller's pre-closure snapshot is what answers, and it stops at `ex:b`.
/// This is the short bag the fix exists to prevent, pinned here so that "NONE preserves
/// the previous semantics" is an executed claim rather than a promise — and so that the
/// test above cannot pass by accident of the relation having been widened.
#[test]
fn none_leaves_the_callers_pre_closure_snapshot_answering() {
    let dataset = subproperty_chain();
    let registry = walk_registry(&dataset);
    let walked = answer(
        &dataset,
        WALK_QUERY,
        QueryEntailment::Rdfs,
        &registry,
        &ClosureRelations::NONE,
    )
    .expect("the un-rebuilt walk answers");
    assert_eq!(
        endpoints(&walked),
        vec![format!("{NS}b")],
        "NONE keeps the caller's snapshot, which has one edge"
    );
}

/// THE REFUSAL IS KEYED ON A MINTED WITNESS, NOT ON THE REGIME'S NAME.
///
/// A refusal is a claim, so the neighbour that must still succeed is executed beside it:
/// an `owl-direct` run over a TBox the combined approach declines mints nothing, and its
/// relation is rebuilt over the closure like any other regime's.
#[test]
fn the_witness_refusal_fires_only_when_the_chase_minted_one() {
    let minting = some_values_from();
    let registry = walk_registry(&minting);
    let rebuild = |closure: &RdfDataset| Ok(walk_registry(closure));

    let refused = answer(
        &minting,
        WALK_QUERY,
        QueryEntailment::OwlDirect,
        &registry,
        &ClosureRelations::rebuilt_by(&rebuild),
    )
    .expect_err("a chase-minted witness refuses the rebuild");
    let purrdf::ReasoningError::Query(diagnostic) = refused else {
        panic!("the refusal is a query-side diagnostic: {refused:?}");
    };
    assert_eq!(
        diagnostic.code,
        ClosureRelations::WITNESS_REFUSAL_CODE,
        "the refusal carries the constant the hosts classify it by"
    );
    assert!(
        diagnostic
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("rdfs")),
        "the remedy must name a regime that accepts the pairing: {diagnostic}"
    );

    // The neighbour: same regime, a TBox outside the Horn fragment, nothing minted.
    let mintless = equivalent_class();
    let walked = answer(
        &mintless,
        WALK_QUERY,
        QueryEntailment::OwlDirect,
        &walk_registry(&mintless),
        &ClosureRelations::rebuilt_by(&rebuild),
    )
    .expect("an owl-direct run that mints nothing still answers");
    assert_eq!(
        endpoints(&walked),
        vec![format!("{NS}b")],
        "and it answers over the closure's own ex:p edges"
    );

    // And the second neighbour: the SAME minting ontology, same regime, with no rebuilder
    // — the refusal is about the pairing, so the regime on its own is untouched.
    let answered = answer(
        &minting,
        "SELECT ?x WHERE { ?x <http://example.org/r> ?y . ?y a <http://example.org/B> }",
        QueryEntailment::OwlDirect,
        &PropertyFunctionRegistry::EMPTY,
        &ClosureRelations::NONE,
    )
    .expect("owl-direct without a rebuilder is untouched");
    let SparqlResult::Solutions { rows, .. } = &answered else {
        panic!("expected a solution sequence");
    };
    assert_eq!(
        rows.len(),
        1,
        "the combined approach's certain answer must still be returned"
    );
}

/// A REBUILDER'S OWN FAILURE IS REPORTED, NOT SWALLOWED INTO AN EMPTY ANSWER.
///
/// The host's snapshot of the closure can fail on its own account, and the one shape that
/// must never happen is the failure becoming zero rows at a success exit — the silent
/// short bag this whole parameter exists to remove. It comes back as a diagnostic carrying
/// its own code and the host's message verbatim.
#[test]
fn a_failing_rebuilder_surfaces_its_own_diagnostic() {
    let dataset = subproperty_chain();
    let registry = walk_registry(&dataset);
    let rebuild = |_: &RdfDataset| {
        Err(purrdf::sparql::EvalError::data(
            "the host could not read the closure",
        ))
    };
    let error = answer(
        &dataset,
        WALK_QUERY,
        QueryEntailment::Rdfs,
        &registry,
        &ClosureRelations::rebuilt_by(&rebuild),
    )
    .expect_err("a failing rebuilder is a failure");
    let purrdf::ReasoningError::Query(diagnostic) = error else {
        panic!("the failure is a query-side diagnostic: {error:?}");
    };
    assert_eq!(diagnostic.code, ClosureRelations::REBUILD_FAILURE_CODE);
    assert!(
        diagnostic
            .message
            .contains("the host could not read the closure"),
        "the host's own message must survive: {diagnostic}"
    );
}

/// THE FIX IS NOT `rdf:type`-SHAPED: IT REACHES EVERY MATERIALIZING REGIME.
///
/// `QueryEntailment::Simple` copies the source, so the rebuilt registry must answer
/// exactly what the caller's does — a regime that derives nothing changes no edge, and a
/// rebuild that widened an answer there would be widening it out of nowhere.
#[test]
fn a_regime_that_derives_nothing_answers_identically_either_way() {
    let dataset = subproperty_chain();
    let registry = walk_registry(&dataset);
    let rebuild = |closure: &RdfDataset| Ok(walk_registry(closure));
    let rebuilt = answer(
        &dataset,
        WALK_QUERY,
        QueryEntailment::Simple,
        &registry,
        &ClosureRelations::rebuilt_by(&rebuild),
    )
    .expect("simple answers");
    let plain = answer(
        &dataset,
        WALK_QUERY,
        QueryEntailment::Simple,
        &registry,
        &ClosureRelations::NONE,
    )
    .expect("simple answers");
    assert_eq!(
        endpoints(&rebuilt),
        endpoints(&plain),
        "an identity closure has the same edges, so both orders answer the same"
    );
    assert_eq!(endpoints(&rebuilt), vec![format!("{NS}b")]);
}
