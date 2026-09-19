// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The pure planner's contract: declaration-only term matching, deterministic
//! golden output, statistics consumption, and purity.
//!
//! The fixtures are `example.org` throughout. The registry is a mock of the
//! property-function seam; the statistics provider is a plain caller-owned
//! lookup. Neither touches a store, a file, a network or a clock.

use std::collections::BTreeMap;
use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    Iri, Metric, Plan, PlanError, RegistryId, RejectionReason, RequestTerm, RetrievalRequest,
    Statistics, Term, UnservedReason, UnservedTerm, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, CandidateDomains, DuplicatePolicy, EvalError, PfArgs, PfArity,
    PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement, Volatility,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn kernel_iri(text: &str) -> purrdf_core::Iri {
    purrdf_core::parse_iri(text).expect("fixture IRIs are valid")
}

fn lexical_term() -> RequestTerm {
    RequestTerm::Lexical {
        text: "quick brown fox".to_owned(),
        language: Some("en".to_owned()),
        predicate: Some(iri(&ex("body"))),
    }
}

fn vector_term() -> RequestTerm {
    RequestTerm::Vector {
        embedding: vec![0.25, -1.5, 3.0],
        metric: Metric::Cosine,
        index_hint: Some("hint".to_owned()),
    }
}

fn seed_term() -> RequestTerm {
    RequestTerm::EntitySeed {
        entity: Term::new("<http://example.org/seed>"),
    }
}

fn mixed_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term(), vector_term(), seed_term()])
}

fn lexical_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term()])
}

fn vector_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![vector_term()])
}

/// Each accepted pattern, with the request term's value rendered into the
/// object-side position. The mocks are arity (1,1) and project `?c0`, so the
/// candidate is position 0 and a rendered facet binds at position 1.
///
/// An unconstrained `TermKind::Any` pattern is the exception: it declares **no**
/// placement at all, so it is matched by every request term and receives none of
/// them. Its argument stays free, and the plan reports every term that reached
/// only this producer — matching a shape is not the same as receiving it.
fn accepted(patterns: Vec<TermPattern>) -> Vec<AcceptedTerm> {
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
fn ranked(stratum: &str, patterns: Vec<TermPattern>, mandatory: bool) -> RankedDeclaration {
    RankedDeclaration {
        stratum: kernel_iri(stratum),
        accepted_terms: accepted(patterns),
        depth_placement: None,
        candidate_position: 0,
        duplicates: DuplicatePolicy::Unique,
        domains: CandidateDomains::Unrestricted,
        block_position: None,
        mandatory,
    }
}

/// A mock relation of arity (1,1) declaring `rows` rows per invocation.
fn relation(rows: u64) -> Arc<dyn PropertyFunction> {
    relation_of_arity(rows, 1)
}

/// A mock relation of arity `(1, object)` declaring `rows` rows per invocation.
fn relation_of_arity(rows: u64, object: usize) -> Arc<dyn PropertyFunction> {
    let arity = PfArity::new(1, object);
    Arc::new(MockProducer {
        arity,
        mode: arity.all_free_mode(),
        rows,
    })
}

/// `pf/pair`: one producer that genuinely **receives** two request terms, by
/// accepting two distinguishable shapes and placing each in its own argument
/// position.
///
/// Needed wherever a claim is about the terms a producer holds rather than the
/// ones it matches. Two terms of the same shape could not both be received — a
/// placement is declared per shape, so the second would contend for the first's
/// position — so a multi-term producer is a multi-shape one.
fn pair_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/pair"),
        relation_of_arity(200, 2),
        RankedDeclaration {
            stratum: kernel_iri(&ex("stratum/pair")),
            accepted_terms: vec![
                AcceptedTerm {
                    pattern: TermPattern::of_kind(TermKind::Literal),
                    placements: vec![TermPlacement {
                        facet: RequestFacet::Value,
                        position: 1,
                        datatype: None,
                    }],
                },
                AcceptedTerm {
                    pattern: TermPattern::of_kind(TermKind::Iri),
                    placements: vec![TermPlacement {
                        facet: RequestFacet::Value,
                        position: 2,
                        datatype: None,
                    }],
                },
            ],
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            mandatory: false,
        },
    );
    registry
}

/// A request of two shapes one producer can hold at once.
fn lexical_and_seed_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term(), seed_term()])
}

/// The mixed registry: one catch-all (`Any`) producer, one
/// literal-only producer, one IRI-only producer, one quoted-triple producer
/// that no request term can reach, and one unranked producer.
fn mixed_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let literal_pattern = TermPattern {
        kind: TermKind::Literal,
        datatype: None,
        language: Some("en".to_owned()),
        predicate: Some(ex("body")),
    };
    for (name, declaration, rows) in [
        (
            ex("pf/any"),
            Some(ranked(
                &ex("stratum/universal"),
                vec![TermPattern::of_kind(TermKind::Any)],
                true,
            )),
            200,
        ),
        (
            ex("pf/literal"),
            Some(ranked(&ex("stratum/text"), vec![literal_pattern], false)),
            100,
        ),
        (
            ex("pf/iri"),
            Some(ranked(
                &ex("stratum/graph"),
                vec![TermPattern::of_kind(TermKind::Iri)],
                false,
            )),
            50,
        ),
        (
            ex("pf/quoted"),
            Some(ranked(
                &ex("stratum/quoted"),
                vec![TermPattern::of_kind(TermKind::Triple)],
                false,
            )),
            25,
        ),
        // Registered with no declaration at all: that is the whole of "does not
        // participate in ranked retrieval".
        (ex("pf/not-ranked"), None, 0),
    ] {
        match declaration {
            Some(declaration) => registry.register_ranked(name, relation(rows), declaration),
            None => registry.register(name, relation(rows)),
        }
    }
    registry
}

/// The mock statistics: cardinalities for the strata and the request predicate,
/// and one reported selectivity.
fn fixture_statistics() -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("stratum/universal")), 1000);
    cardinalities.insert(iri(&ex("stratum/text")), 100);
    cardinalities.insert(iri(&ex("stratum/graph")), 50);
    cardinalities.insert(iri(&ex("stratum/pair")), 1000);
    cardinalities.insert(iri(&ex("body")), 500);
    MockStatistics {
        source: "example-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities,
        selectivities: vec![(iri(&ex("body")), lexical_term(), 500_000)],
    }
}

/// A statistics provider that reports only the request predicate, so a stratum
/// with a genuinely unbounded declared row count has no cardinality to bound it.
fn no_stratum_cardinality() -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("body")), 500);
    MockStatistics {
        source: "example-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities,
        selectivities: vec![],
    }
}

// ---------------------------------------------------------------------------
// Mock property-function seam
// ---------------------------------------------------------------------------

struct MockProducer {
    arity: PfArity,
    mode: BindingPattern,
    rows: u64,
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
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // Planning never opens a cursor; the mock still satisfies the trait.
        Ok(Box::new(EmptyCursor))
    }
}

struct EmptyCursor;

impl PfCursor for EmptyCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }
}

struct MockStatistics {
    source: String,
    revision: String,
    cardinalities: BTreeMap<Iri, u64>,
    /// Reported selectivities as `(subject, term, parts per million)`. A `Vec`
    /// rather than a map because the key carries a [`RequestTerm`], which is
    /// deliberately not `Ord`.
    selectivities: Vec<(Iri, RequestTerm, u64)>,
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

    fn selectivity_ppm(&self, subject: &Iri, term: &RequestTerm) -> Option<u64> {
        self.selectivities
            .iter()
            .find(|(declared, candidate, _)| declared == subject && candidate == term)
            .map(|(_, _, value)| *value)
    }
}

// ---------------------------------------------------------------------------
// Golden support
// ---------------------------------------------------------------------------

const GOLDEN_DIR: &str = "tests/fixtures/planner";

fn golden_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(GOLDEN_DIR)
        .join(name)
}

fn read_golden(name: &str) -> String {
    std::fs::read_to_string(golden_path(name))
        .unwrap_or_else(|error| panic!("golden {name} is checked in: {error}"))
}

/// A deterministic rendering of a plan as pretty JSON.
///
/// `serde_json`'s default (non-`preserve_order`) `Value` is sorted-key, so the
/// `HashMap` stratum maps serialize in canonical order rather than in
/// iteration order. The registry instance id is a per-process counter
/// (`registry_id.rs`), so it is pinned here and asserted separately; every
/// durable field is captured verbatim.
fn canonical_json(plan: &Plan) -> String {
    let mut value = serde_json::to_value(plan).expect("a plan serializes");
    value
        .as_object_mut()
        .expect("a plan serializes to an object")
        .insert(
            "registry_instance_id".to_owned(),
            serde_json::Value::from(0_u64),
        );
    let mut rendered = serde_json::to_string_pretty(&value).expect("the value renders");
    rendered.push('\n');
    rendered
}

// ---------------------------------------------------------------------------
// 1. Golden tests
// ---------------------------------------------------------------------------

/// The identity of the plan the mixed-request golden records.
///
/// Pinned as a literal beside the golden document rather than read back out of
/// it. A golden compared only as rendered text answers "did the text move"; the
/// identity answers "did the plan move", over the canonical bytes every later
/// stage keys on, and a change that altered the identity while leaving the
/// rendering alone would otherwise pass unnoticed.
const MIXED_REQUEST_PLAN_ID: &str =
    "812a811b57d65ec25b35f210d1830a0c053baa5de5fc5e72fbd4f80a39f02d15";

/// The identity of the plan the lexical-request golden records, pinned for the
/// reason [`MIXED_REQUEST_PLAN_ID`] is.
const LEXICAL_REQUEST_PLAN_ID: &str =
    "d4949c9af9995f02c8768881961d410287165cee67cd02091f42f1f2d9e4f981";

/// A plan's content identity, with the per-process registry instance counter
/// pinned exactly as [`canonical_json`] pins it.
///
/// [`Plan::id`] digests the canonical bytes, and those bytes carry
/// [`Plan::registry_instance_id`] — a counter minted per registry per process,
/// so the live value differs from run to run and from test order to test order.
/// Pinning it to zero is what makes the identity a property of the *plan* rather
/// than of the process that happened to build it, and leaves every durable field
/// covered.
fn pinned_id(plan: &Plan) -> String {
    let mut pinned = plan.clone();
    pinned.registry_instance_id = RegistryId::from_raw(0);
    pinned.id().to_hex()
}

/// A plan's per-stratum depths, keyed by stratum IRI in a deterministic order.
///
/// The recorded map is a `HashMap`, so it is collected into an ordered one here
/// to be compared whole: asserting the map rather than one key at a time is what
/// makes an *extra* stratum depth a failure, and an extra depth licenses reading
/// rows in a stratum the assertion never mentioned.
fn depths_of(plan: &Plan) -> BTreeMap<String, u32> {
    plan.stratum_depths
        .iter()
        .map(|(stratum, depth)| (stratum.as_str().to_owned(), *depth))
        .collect()
}

/// NOT part of the normal test run (`#[ignore]`): (re)writes both committed
/// planner goldens from the planner's current output.
///
/// A golden here is a **measurement**, not a preference: it pins the planner's
/// rendering *and* `registry_content_fingerprint`, which is a digest of the
/// registry's declarations. Neither can be reasoned out by hand, so whenever an
/// intentional, reviewed change alters what the planner emits or what the
/// fingerprint covers, produce the new goldens by running the planner:
///
/// ```text
/// cargo test -p purrdf-retrieval --test planner_golden regenerate_planner_goldens -- --ignored
/// ```
///
/// then commit the updated fixtures alongside the change that caused them to
/// differ. Editing a fingerprint by hand — or resolving a merge by picking one
/// side of one — records a value no run ever produced.
#[test]
#[ignore = "regenerates the committed golden fixtures; run explicitly with -- --ignored"]
fn regenerate_planner_goldens() {
    let registry = mixed_registry();
    let statistics = fixture_statistics();
    for (name, request) in [
        ("mixed_request.json", mixed_request()),
        ("lexical_request.json", lexical_request()),
    ] {
        let plan = plan(&request, &registry, &statistics).expect("plans");
        std::fs::write(golden_path(name), canonical_json(&plan)).expect("writes the golden");
    }
}

#[test]
fn golden_mixed_request_matches() {
    let registry = mixed_registry();
    let plan = plan(&mixed_request(), &registry, &fixture_statistics()).expect("plans");
    assert_eq!(
        plan.registry_instance_id,
        registry.instance_id(),
        "the plan records the live registry instance identity"
    );
    assert_eq!(
        canonical_json(&plan),
        read_golden("mixed_request.json"),
        "planner output drifted from the golden; update {} if intended",
        golden_path("mixed_request.json").display()
    );
    assert_eq!(
        depths_of(&plan),
        BTreeMap::from([
            (ex("stratum/graph"), 50),
            (ex("stratum/text"), 100),
            (ex("stratum/universal"), 200),
        ]),
        "the depths this fixture derives, named rather than eyeballed out of the golden"
    );
    assert_eq!(pinned_id(&plan), MIXED_REQUEST_PLAN_ID);
}

#[test]
fn golden_lexical_request_matches() {
    let registry = mixed_registry();
    let plan = plan(&lexical_request(), &registry, &fixture_statistics()).expect("plans");
    assert_eq!(
        canonical_json(&plan),
        read_golden("lexical_request.json"),
        "planner output drifted from the golden; update {} if intended",
        golden_path("lexical_request.json").display()
    );
    assert_eq!(
        depths_of(&plan),
        BTreeMap::from([(ex("stratum/text"), 100), (ex("stratum/universal"), 200),]),
        "the depths this fixture derives, named rather than eyeballed out of the golden"
    );
    assert_eq!(pinned_id(&plan), LEXICAL_REQUEST_PLAN_ID);
}

#[test]
fn planning_is_a_pure_replay() {
    let request = mixed_request();
    let registry = mixed_registry();
    let statistics = fixture_statistics();
    let first = plan(&request, &registry, &statistics).expect("plans");
    let second = plan(&request, &registry, &statistics).expect("plans");
    assert_eq!(first, second, "same inputs, same plan");
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    assert_eq!(first.id(), second.id());
    assert_eq!(canonical_json(&first), canonical_json(&second));
}

// ---------------------------------------------------------------------------
// 2. Term matching
// ---------------------------------------------------------------------------

fn binding<'a>(plan: &'a Plan, producer: &str) -> Option<&'a purrdf_retrieval::ProducerBinding> {
    plan.producer_bindings
        .iter()
        .find(|binding| binding.producer == producer)
}

fn reason(plan: &Plan, producer: &str) -> Option<RejectionReason> {
    plan.producer_decisions
        .iter()
        .find_map(|decision| match decision {
            purrdf_retrieval::ProducerDecision::Rejected {
                producer: name,
                reason,
            } if name == producer => Some(*reason),
            _ => None,
        })
}

#[test]
fn lexical_term_matches_a_literal_producer() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    let literal = binding(&plan, &ex("pf/literal")).expect("literal producer selected");
    assert_eq!(literal.request_terms, vec![0]);
    assert_eq!(literal.stratum, iri(&ex("stratum/text")));
}

#[test]
fn a_vector_term_the_only_acceptor_of_which_places_nothing_is_reported_not_bound() {
    // The registry's only acceptor of a vector term is the unconstrained
    // catch-all, and it declares no placement. So the producer is selected — its
    // pattern does match — and it is bound to NOTHING: an embedding that reached
    // it reached none of its arguments, and a binding would claim otherwise.
    //
    // Asserting the binding alone could never see the difference, because a
    // producer bound to a term it does not place and one bound to no term emit
    // the identical text. What separates them is the per-term evidence, which is
    // what this asserts.
    let unplaced =
        plan(&vector_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    let any =
        binding(&unplaced, &ex("pf/any")).expect("the catch-all's pattern matches the vector");
    assert_eq!(
        any.request_terms,
        Vec::<u32>::new(),
        "it writes no part of the embedding, so it serves no term"
    );
    assert_eq!(
        unplaced.unserved_terms,
        vec![UnservedTerm {
            request_term: 0,
            reason: UnservedReason::AcceptedWithoutPlacement,
        }],
        "and the request says so by index rather than looking answered"
    );
    assert_eq!(unplaced.unserved_evidence(), unplaced.unserved_terms);
    assert_eq!(
        reason(&unplaced, &ex("pf/literal")),
        Some(RejectionReason::NoAcceptedTerm),
        "the text producer constrains a language a vector cannot carry"
    );
    assert_eq!(
        reason(&unplaced, &ex("pf/iri")),
        Some(RejectionReason::NoAcceptedTerm)
    );

    // The neighbouring valid case, and the one that proves the arm is not a
    // dead end: a producer declaring the datatype its own space reads an
    // embedding under accepts the vector AND is bound to it. The emitted
    // constant is pinned in `compile_request.rs`.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/embedding"),
        relation(100),
        RankedDeclaration {
            stratum: kernel_iri(&ex("stratum/embedding")),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern::of_kind(TermKind::Literal),
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    // Named by the host, never minted here.
                    datatype: Some(ex("embedding")),
                }],
            }],
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            mandatory: false,
        },
    );
    let served = plan(&vector_request(), &registry, &fixture_statistics())
        .expect("the vector term reaches a producer that declares its datatype");
    assert_eq!(
        binding(&served, &ex("pf/embedding"))
            .expect("the embedding producer is bound")
            .request_terms,
        vec![0]
    );
    assert_eq!(
        served.unserved_terms,
        Vec::new(),
        "nothing is unserved: the embedding reached a producer with its content"
    );
}

#[test]
fn entity_seed_matches_an_iri_producer() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    let iri_producer = binding(&plan, &ex("pf/iri")).expect("IRI producer selected");
    assert_eq!(iri_producer.request_terms, vec![2]);
}

#[test]
fn unmatched_and_unranked_producers_are_rejected() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    assert_eq!(
        reason(&plan, &ex("pf/quoted")),
        Some(RejectionReason::NoAcceptedTerm)
    );
    assert_eq!(
        reason(&plan, &ex("pf/not-ranked")),
        Some(RejectionReason::NotRanked)
    );
}

#[test]
fn language_and_predicate_constraints_are_enforced() {
    let request = RetrievalRequest::complete(vec![RequestTerm::Lexical {
        text: "different".to_owned(),
        language: Some("fr".to_owned()),
        predicate: Some(iri(&ex("other"))),
    }]);
    let plan = plan(&request, &mixed_registry(), &fixture_statistics()).expect("plans");
    // The literal producer constrains language `en` and predicate `body`, so it
    // rejects; only the unconstrained `Any` producer matches the term — and it
    // declares no placement, so it receives none of it.
    assert_eq!(
        reason(&plan, &ex("pf/literal")),
        Some(RejectionReason::NoAcceptedTerm)
    );
    assert_eq!(
        binding(&plan, &ex("pf/any"))
            .expect("Any selected")
            .request_terms,
        Vec::<u32>::new()
    );
    assert_eq!(
        plan.unserved_terms,
        vec![UnservedTerm {
            request_term: 0,
            reason: UnservedReason::AcceptedWithoutPlacement,
        }]
    );
}

// ---------------------------------------------------------------------------
// 2b. Per-term evidence: a term that reached nothing says so
//
// The decisions above are per producer. A caller asks per term, and the request
// lattice deliberately carries modalities ahead of the producers that answer
// them, so "this registry has nobody for that" has to be visible as exactly that
// rather than as a term that quietly produced no rows.
// ---------------------------------------------------------------------------

/// A registry that accepts a lexical `en`/`body` term and nothing else, so a
/// vector term in the same request reaches no declaration at all.
fn literal_only_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/literal"),
        relation(100),
        ranked(
            &ex("stratum/text"),
            vec![TermPattern {
                kind: TermKind::Literal,
                datatype: None,
                language: Some("en".to_owned()),
                predicate: Some(ex("body")),
            }],
            false,
        ),
    );
    registry
}

/// The same registry plus a catch-all whose declaration demands that every term
/// it accepts be rendered into an argument position. A query embedding has no
/// SPARQL constant form, so the catch-all cannot be invoked for this request and
/// is rejected at placement — leaving the vector term accepted by something and
/// served by nothing.
fn accepting_but_uninvocable_registry() -> PropertyFunctionRegistry {
    let mut registry = literal_only_registry();
    registry.register_ranked(
        ex("pf/any"),
        relation(200),
        RankedDeclaration {
            stratum: kernel_iri(&ex("stratum/universal")),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern::of_kind(TermKind::Any),
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }],
            }],
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            mandatory: false,
        },
    );
    registry
}

fn lexical_and_vector_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term(), vector_term()])
}

#[test]
fn a_term_no_declaration_accepts_is_recorded_as_unserved() {
    let plan = plan(
        &lexical_and_vector_request(),
        &literal_only_registry(),
        &fixture_statistics(),
    )
    .expect("the lexical term still reaches a producer, so the request plans");

    assert_eq!(
        plan.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::NoProducerAccepts,
        }],
        "the vector term reached no declaration, and the plan says so by index"
    );
    assert_eq!(
        plan.unserved_evidence(),
        plan.unserved_terms,
        "the evidence this plan supports is the record it made"
    );
    // The valid neighbour, in the same plan: the term that DID reach a producer
    // is served and is not reported.
    assert_eq!(
        binding(&plan, &ex("pf/literal"))
            .expect("the literal producer takes the lexical term")
            .request_terms,
        vec![0],
        "term 0 reached a producer"
    );
    assert!(
        plan.unserved_terms
            .iter()
            .all(|entry| entry.request_term != 0),
        "a term that reached a producer is never reported unserved"
    );
}

#[test]
fn a_term_every_acceptor_of_which_was_rejected_is_recorded_as_such() {
    let plan = plan(
        &lexical_and_vector_request(),
        &accepting_but_uninvocable_registry(),
        &fixture_statistics(),
    )
    .expect("the lexical term still reaches an invocable producer");

    assert_eq!(
        reason(&plan, &ex("pf/any")),
        Some(RejectionReason::UnsatisfiedConstraint),
        "the catch-all accepted the vector term but cannot be invoked for it"
    );
    assert_eq!(
        plan.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::EveryAcceptingProducerRejected,
        }],
        "something accepted the shape and was then rejected, which is not the \
         same fact as nothing accepting it"
    );
}

#[test]
fn a_request_every_term_of_which_is_served_records_nothing() {
    // The neighbouring valid case, and the exact meaning of an empty list: every
    // term reached a producer WITH its content. The needle reaches the literal
    // producer, which places it; the seed reaches the IRI producer, which places
    // it. The catch-all matches both and places neither, which adds nothing to
    // the evidence either way, because the two terms are already carried.
    let plan = plan(
        &lexical_and_seed_request(),
        &mixed_registry(),
        &fixture_statistics(),
    )
    .expect("plans");
    assert!(
        plan.unserved_terms.is_empty(),
        "every term reached a producer that places it, got {:?}",
        plan.unserved_terms
    );
    assert!(
        plan.unserved_evidence().is_empty(),
        "and the evidence it supports is empty too, got {:?}",
        plan.unserved_evidence()
    );
    for (producer, terms) in [(ex("pf/literal"), vec![0_u32]), (ex("pf/iri"), vec![1])] {
        assert_eq!(
            binding(&plan, &producer)
                .expect("the placing producer is bound")
                .request_terms,
            terms,
            "{producer} holds the term whose content it renders"
        );
    }
}

#[test]
fn a_producer_that_reads_nothing_of_the_request_still_plans_and_serves_no_term() {
    // The configuration that makes "matched but not received" worth carrying
    // rather than refusing at registration: a stratum whose ranking is
    // request-independent — a quality prior, fused beside the rest — accepts
    // every shape and reads none of it. Such a producer takes part in the answer
    // and its stratum emits, and the plan is honest that the request itself
    // reached nothing.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/prior"),
        relation(100),
        ranked(
            &ex("stratum/prior"),
            vec![TermPattern::of_kind(TermKind::Any)],
            false,
        ),
    );
    let plan = plan(&lexical_request(), &registry, &fixture_statistics())
        .expect("a request-independent producer is still a producer");
    assert_eq!(
        binding(&plan, &ex("pf/prior"))
            .expect("it is selected and its stratum emits")
            .request_terms,
        Vec::<u32>::new()
    );
    assert_eq!(
        plan.unserved_terms,
        vec![UnservedTerm {
            request_term: 0,
            reason: UnservedReason::AcceptedWithoutPlacement,
        }],
        "and the needle is reported rather than counted as answered"
    );
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/prior"))], 100);
}

// ---------------------------------------------------------------------------
// 2c. The interval modalities: carried ahead of their producers, and reported
// as unserved rather than quietly dropped.
// ---------------------------------------------------------------------------

fn temporal_term() -> RequestTerm {
    RequestTerm::Temporal {
        predicate: iri(&ex("observed")),
        lower: Some("2026-01-01T00:00:00Z".to_owned()),
        upper: Some("2026-02-01T00:00:00Z".to_owned()),
    }
}

fn numeric_range_term() -> RequestTerm {
    RequestTerm::NumericRange {
        predicate: iri(&ex("price")),
        lower: Some(purrdf_retrieval::Fixed::ONE),
        upper: None,
    }
}

#[test]
fn an_interval_term_no_registry_producer_takes_is_reported_per_term() {
    // The registry's literal producer constrains predicate `body`, so neither
    // interval reaches it; nothing else in this registry accepts a literal. The
    // lexical term still reaches it, so the request plans and the answer is
    // honest about what it could not serve.
    let request =
        RetrievalRequest::complete(vec![lexical_term(), temporal_term(), numeric_range_term()]);
    let plan = plan(&request, &literal_only_registry(), &fixture_statistics())
        .expect("the lexical term still reaches a producer");

    assert_eq!(
        plan.unserved_terms,
        vec![
            UnservedTerm {
                request_term: 1,
                reason: UnservedReason::NoProducerAccepts,
            },
            UnservedTerm {
                request_term: 2,
                reason: UnservedReason::NoProducerAccepts,
            },
        ],
        "a modality the lattice carries ahead of its producers says so by index"
    );
    assert_eq!(plan.unserved_evidence(), plan.unserved_terms);
    // The neighbouring valid case, in the same plan: the term that did reach a
    // producer is served.
    assert_eq!(
        binding(&plan, &ex("pf/literal"))
            .expect("the literal producer takes the lexical term")
            .request_terms,
        vec![0]
    );
}

#[test]
fn an_interval_term_reaches_a_producer_that_declares_its_predicate() {
    // The other side of the same fact: nothing about the arm makes it
    // unreachable. A producer declaring a literal pattern on the interval's own
    // predicate, with a placement for each endpoint, is bound to it.
    let mut registry = PropertyFunctionRegistry::new();
    let date_time = "http://www.w3.org/2001/XMLSchema#dateTime";
    registry.register_ranked(
        ex("pf/temporal"),
        relation(100),
        RankedDeclaration {
            stratum: kernel_iri(&ex("stratum/time")),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern {
                    kind: TermKind::Literal,
                    datatype: None,
                    language: None,
                    predicate: Some(ex("observed")),
                },
                // The mock relation is arity (1,1), so it has exactly one
                // argument position beside its candidate and can take exactly
                // one endpoint; the request below is the half-open interval that
                // matches what this producer declared it can receive.
                placements: vec![TermPlacement {
                    facet: RequestFacet::LowerBound,
                    position: 1,
                    datatype: Some(date_time.to_owned()),
                }],
            }],
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            mandatory: false,
        },
    );
    let request = RetrievalRequest::complete(vec![RequestTerm::Temporal {
        predicate: iri(&ex("observed")),
        lower: Some("2026-01-01T00:00:00Z".to_owned()),
        upper: None,
    }]);
    let plan = plan(&request, &registry, &fixture_statistics()).expect("the interval term plans");
    assert_eq!(
        binding(&plan, &ex("pf/temporal"))
            .expect("the temporal producer takes the interval")
            .request_terms,
        vec![0]
    );
    assert_eq!(
        plan.unserved_terms,
        Vec::new(),
        "the interval reached a producer, so nothing is reported unserved"
    );
}

#[test]
fn an_interval_that_constrains_nothing_is_refused_but_a_half_open_one_plans() {
    for empty in [
        RequestTerm::Temporal {
            predicate: iri(&ex("observed")),
            lower: None,
            upper: None,
        },
        RequestTerm::NumericRange {
            predicate: iri(&ex("price")),
            lower: None,
            upper: None,
        },
    ] {
        let request = RetrievalRequest::complete(vec![empty]);
        let error = plan(&request, &mixed_registry(), &fixture_statistics())
            .expect_err("an interval with neither endpoint constrains nothing");
        match error {
            PlanError::InvalidRequestTerm { reason, .. } => {
                assert!(reason.contains("neither endpoint"), "{reason}");
            }
            other => panic!("expected InvalidRequestTerm, got {other:?}"),
        }
    }
    // The neighbouring valid cases: one endpoint is a half-open interval, and it
    // plans.
    for half_open in [temporal_term(), numeric_range_term()] {
        let request = RetrievalRequest::complete(vec![half_open]);
        assert!(
            plan(&request, &mixed_registry(), &fixture_statistics()).is_ok(),
            "an interval carrying an endpoint is a question"
        );
    }
}

#[test]
fn an_inverted_numeric_range_is_refused_but_a_degenerate_one_plans() {
    let inverted = RetrievalRequest::complete(vec![RequestTerm::NumericRange {
        predicate: iri(&ex("price")),
        lower: Some(purrdf_retrieval::Fixed::from_raw(2)),
        upper: Some(purrdf_retrieval::Fixed::from_raw(1)),
    }]);
    let error = plan(&inverted, &mixed_registry(), &fixture_statistics())
        .expect_err("a range whose lower endpoint is above its upper matches nothing");
    match error {
        PlanError::InvalidRequestTerm { reason, .. } => {
            assert!(reason.contains("exceeds"), "{reason}");
        }
        other => panic!("expected InvalidRequestTerm, got {other:?}"),
    }
    // The neighbouring valid case, one raw unit away: equal endpoints are a
    // single point, which is a perfectly good question.
    let degenerate = RetrievalRequest::complete(vec![RequestTerm::NumericRange {
        predicate: iri(&ex("price")),
        lower: Some(purrdf_retrieval::Fixed::from_raw(1)),
        upper: Some(purrdf_retrieval::Fixed::from_raw(1)),
    }]);
    assert!(
        plan(&degenerate, &mixed_registry(), &fixture_statistics()).is_ok(),
        "a degenerate range is a point, not an empty set"
    );
}

#[test]
fn an_interval_predicate_reaches_the_statistics_snapshot() {
    // The interval names a predicate, so planning consults the statistics for
    // it exactly as it does for a needle's predicate; an arm the planner did not
    // teach `term_predicate` about would silently consult nothing.
    let request = RetrievalRequest::complete(vec![RequestTerm::NumericRange {
        predicate: iri(&ex("body")),
        lower: Some(purrdf_retrieval::Fixed::ONE),
        upper: None,
    }]);
    let plan = plan(&request, &mixed_registry(), &fixture_statistics()).expect("plans");
    assert_eq!(
        plan.statistics_snapshot
            .entries
            .iter()
            .find(|entry| entry.subject == ex("body"))
            .map(|entry| entry.cardinality),
        Some(500)
    );
}

// ---------------------------------------------------------------------------
// 3. Producer selection
// ---------------------------------------------------------------------------

/// Breadth of a pattern decides what a producer *matches*; its placements decide
/// what it is *bound to*. Both halves are executed here, because a rule that
/// conflated them would bind a producer to terms it never receives.
///
/// Whether the producer may be dropped is a third, separate fact the registry
/// declares, and admission reads it from there rather than deriving it from
/// either of these.
#[test]
fn an_unconstrained_producer_matches_every_term_and_holds_the_ones_it_places() {
    // Unconstrained and placement-free: matched by all three, bound to none.
    let matched = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    assert_eq!(
        binding(&matched, &ex("pf/any"))
            .expect("Any producer selected")
            .request_terms,
        Vec::<u32>::new()
    );
    assert_eq!(
        reason(&matched, &ex("pf/any")),
        None,
        "matching is what selected it; placing nothing is not a rejection"
    );

    // The other half: a producer whose accepted shapes each carry a placement is
    // bound to every term it places, which here is the whole request.
    let held = plan(
        &lexical_and_seed_request(),
        &pair_registry(),
        &fixture_statistics(),
    )
    .expect("plans");
    assert_eq!(
        binding(&held, &ex("pf/pair"))
            .expect("the pair producer is selected")
            .request_terms,
        vec![0, 1]
    );
    assert_eq!(held.unserved_terms, Vec::new());
}

#[test]
fn a_request_that_reaches_nothing_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/quoted"),
        relation(25),
        ranked(
            &ex("stratum/quoted"),
            vec![TermPattern::of_kind(TermKind::Triple)],
            false,
        ),
    );
    let error = plan(&lexical_request(), &registry, &fixture_statistics())
        .expect_err("a request reaching nothing must be refused");
    assert!(
        matches!(error, PlanError::NoApplicableProducers),
        "got {error:?}"
    );
}

#[test]
fn malformed_terms_are_refused_with_a_name() {
    let request = RetrievalRequest::complete(vec![RequestTerm::Lexical {
        text: "   ".to_owned(),
        language: None,
        predicate: None,
    }]);
    let error = plan(&request, &mixed_registry(), &fixture_statistics())
        .expect_err("an empty needle is malformed");
    match error {
        PlanError::InvalidRequestTerm { term, reason } => {
            assert!(matches!(*term, RequestTerm::Lexical { .. }));
            assert!(reason.contains("empty"));
        }
        other => panic!("expected InvalidRequestTerm, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Statistics consumption
// ---------------------------------------------------------------------------

#[test]
fn depths_come_from_the_registry_row_bound_capped_by_statistics() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    // Declared 200, stat 1000 -> 200; declared 100, stat 100 -> 100; declared 50, stat 50 -> 50.
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/universal"))], 200);
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/text"))], 100);
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/graph"))], 50);
    for (stratum, cardinality) in [
        (ex("stratum/universal"), 1000_u64),
        (ex("stratum/text"), 100),
        (ex("stratum/graph"), 50),
    ] {
        assert_eq!(
            plan.statistics_snapshot
                .entries
                .iter()
                .find(|entry| entry.subject == stratum)
                .map(|entry| entry.cardinality),
            Some(cardinality),
            "the recorded cardinality is the statistic planning consulted"
        );
    }
}

#[test]
fn the_planner_reads_the_registry_row_bound_not_a_parallel_field() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/any"),
        // 40, not 200: only a planner that reads the seam's declaration can
        // see this change.
        relation(40),
        ranked(
            &ex("stratum/universal"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    let plan = plan(&mixed_request(), &registry, &fixture_statistics()).expect("plans");
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/universal"))], 40);
}

#[test]
fn statistics_lower_but_never_raise_the_declared_bound() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/any"),
        relation(5_000),
        ranked(
            &ex("stratum/universal"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    let plan = plan(&mixed_request(), &registry, &fixture_statistics()).expect("plans");
    // Declared 5_000, stat 1_000 -> 1_000, never the other direction.
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/universal"))], 1_000);
}

/// An unbounded declaration with no statistic to narrow it reads to the ceiling,
/// and its finite neighbours read exactly where they always did.
///
/// This used to be `PlanError::StatisticsUnavailable`. What made the refusal
/// obsolete is that the same planner now records *any* declaration larger than a
/// read can reach at the deepest readable depth, with the probe row one past it: a
/// declaration of `u64::MAX` and one of `u64::MAX - 1` produce the same depth, the
/// same emitted bound and the same ending, so refusing the first while serving the
/// second was a refusal one declared row wide. Nothing is claimed about the rows
/// below the ceiling — the read ends `DepthReached`, which names the planned depth
/// as the stopper.
#[test]
fn an_unbounded_stratum_without_statistics_reads_to_the_ceiling() {
    let endless = |declared: u64| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            ex("pf/unbounded"),
            relation(declared),
            ranked(
                &ex("stratum/endless"),
                vec![TermPattern::of_kind(TermKind::Any)],
                true,
            ),
        );
        registry
    };
    // The deepest depth a read can be taken to: one shallower than the deepest a
    // rank can express, because the read is emitted one row past the depth.
    let deepest = u32::MAX - 1;

    for declared in [u64::MAX, u64::MAX - 1, u64::from(u32::MAX)] {
        let planned = plan(
            &lexical_request(),
            &endless(declared),
            &no_stratum_cardinality(),
        )
        .expect("a declaration larger than a read can reach is planned, not refused");
        assert_eq!(
            planned.stratum_depths[&iri(&ex("stratum/endless"))],
            deepest,
            "a declaration of {declared} rows records the deepest readable depth"
        );
    }

    // The valid neighbours a ceiling must not disturb: a declaration a read can
    // reach, with no statistic in sight, is still exactly its own number.
    for declared in [1, 40, u64::from(deepest)] {
        let planned = plan(
            &lexical_request(),
            &endless(declared),
            &no_stratum_cardinality(),
        )
        .expect("a reachable declaration plans");
        let depth = u32::try_from(declared).expect("the fixture declarations fit a rank");
        assert_eq!(
            planned.stratum_depths[&iri(&ex("stratum/endless"))],
            depth,
            "a declaration of {declared} rows is the depth, untouched by the ceiling"
        );
    }
}

#[test]
fn the_snapshot_records_the_consulted_selectivity() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    let body = plan
        .statistics_snapshot
        .entries
        .iter()
        .find(|entry| entry.subject == ex("body"))
        .expect("the request predicate is recorded");
    assert_eq!(body.cardinality, 500);
    assert_eq!(body.selectivity_ppm, Some(500_000));
    assert_eq!(plan.statistics_snapshot.source, "example-statistics");
    assert_eq!(plan.statistics_snapshot.revision, "r1");
}

/// The fixture statistics with exactly the reported selectivities given, so a
/// test states the whole of what the provider knows about selectivity.
fn selectivity_statistics(reported: Vec<(Iri, RequestTerm, u64)>) -> MockStatistics {
    MockStatistics {
        selectivities: reported,
        ..fixture_statistics()
    }
}

/// A selectivity is a statement about the rows a term can match, so it bounds
/// the depth the way a cardinality does. Both halves are executed: the stratum
/// the provider spoke about is narrowed, and — the case that would be silently
/// over-refused if the rule leaked — the stratum it said nothing about, and the
/// identical plan under a provider that reports no selectivity at all, keep
/// exactly the depth the registry declared.
#[test]
fn a_reported_stratum_selectivity_bounds_that_stratum_and_only_that_stratum() {
    let reported = selectivity_statistics(vec![(
        iri(&ex("stratum/text")),
        lexical_term(),
        250_000, // a quarter of the stratum's rows can match
    )]);
    let narrowed = plan(&lexical_request(), &mixed_registry(), &reported).expect("plans");
    assert_eq!(
        narrowed.stratum_depths[&iri(&ex("stratum/text"))],
        25,
        "a quarter of 100 rows is 25, so a depth of 100 would license reading rows no term fills"
    );
    assert_eq!(
        narrowed.stratum_depths[&iri(&ex("stratum/universal"))],
        200,
        "the stratum the provider said nothing about keeps its declared depth"
    );

    let silent = selectivity_statistics(Vec::new());
    let untouched = plan(&lexical_request(), &mixed_registry(), &silent).expect("plans");
    assert_eq!(
        untouched.stratum_depths[&iri(&ex("stratum/text"))],
        100,
        "a provider that measured no selectivity narrows nothing"
    );

    // The snapshot explains the depth rather than reporting a second number:
    // the value that narrowed the stratum is the value recorded beside it.
    assert_eq!(
        narrowed
            .statistics_snapshot
            .entries
            .iter()
            .find(|entry| entry.subject == ex("stratum/text"))
            .and_then(|entry| entry.selectivity_ppm),
        Some(250_000)
    );
}

/// The bound is rounded up and clamped at unity, because the failure mode of
/// getting either wrong is a ranked list that is quietly shorter than the
/// stratum's real answer.
#[test]
fn a_selectivity_rounds_up_and_never_raises_a_declared_depth() {
    // One part per million of 100 rows is a ten-thousandth of a row. Rounding
    // down would record a depth of zero for a stratum that can still answer.
    let sparse = selectivity_statistics(vec![(iri(&ex("stratum/text")), lexical_term(), 1)]);
    assert_eq!(
        plan(&lexical_request(), &mixed_registry(), &sparse)
            .expect("plans")
            .stratum_depths[&iri(&ex("stratum/text"))],
        1
    );

    // Unity, and the two provider faults above it, all leave the bound exactly
    // where the registry's declaration and the cardinality put it. A ratio
    // cannot widen a depth.
    for ppm in [1_000_000_u64, 5_000_000, u64::MAX] {
        let full = selectivity_statistics(vec![(iri(&ex("stratum/text")), lexical_term(), ppm)]);
        assert_eq!(
            plan(&lexical_request(), &mixed_registry(), &full)
                .expect("plans")
                .stratum_depths[&iri(&ex("stratum/text"))],
            100,
            "{ppm} ppm must not raise the declared bound"
        );
    }

    // Zero is a measurement, not an absence, and it is also not a verdict. The
    // provider says no row under this stratum matches; the planner narrows the
    // read to one row and lets the producer be the thing that reports the
    // emptiness, because a depth of zero never lets the relation answer and would
    // still be reported as an exhausted stratum. An estimate narrows a read; it
    // never eliminates one.
    let none = selectivity_statistics(vec![(iri(&ex("stratum/text")), lexical_term(), 0)]);
    assert_eq!(
        plan(&lexical_request(), &mixed_registry(), &none)
            .expect("plans")
            .stratum_depths[&iri(&ex("stratum/text"))],
        1
    );

    // The snapshot still records what the provider actually said. The floor is
    // the planner's decision about the depth, not an edit of the measurement.
    assert_eq!(
        plan(&lexical_request(), &mixed_registry(), &none)
            .expect("plans")
            .statistics_snapshot
            .entries
            .iter()
            .find(|entry| entry.subject == ex("stratum/text"))
            .and_then(|entry| entry.selectivity_ppm),
        Some(0),
        "the provider reported zero and the plan says so"
    );
}

/// Several terms reaching one stratum are summed, not minimised. A producer
/// that answers the union of the terms it was handed returns more rows than the
/// most selective of them describes, and a bound below that would truncate its
/// ranked list with nothing saying so.
#[test]
fn selectivities_across_the_terms_one_stratum_receives_are_summed() {
    // Over `pf/pair`, which genuinely receives both terms. A selectivity is a
    // statement about the rows a term matches, so only a term the stratum's
    // producer actually holds can bound its depth — which is why this is not
    // asserted over a placement-free catch-all, whose rows no request term
    // filters at all.
    let reported = selectivity_statistics(vec![
        (iri(&ex("stratum/pair")), lexical_term(), 300_000),
        (iri(&ex("stratum/pair")), seed_term(), 400_000),
    ]);
    let planned = plan(&lexical_and_seed_request(), &pair_registry(), &reported).expect("plans");
    assert_eq!(
        planned.stratum_depths[&iri(&ex("stratum/pair"))],
        140,
        "seven tenths of the 200-row bound, not the three tenths the most selective term names"
    );
    assert_eq!(
        planned
            .statistics_snapshot
            .entries
            .iter()
            .find(|entry| entry.subject == ex("stratum/pair"))
            .and_then(|entry| entry.selectivity_ppm),
        Some(700_000),
        "the recorded aggregate is the one the depth was derived from"
    );
}

#[test]
fn a_selectivity_cannot_narrow_a_stratum_whose_producer_receives_no_term() {
    // The mirror of the test above, and the reason the terms read are the
    // carried ones. `pf/any` places nothing, so no request term filters its
    // rows; a provider that reports a selectivity against its stratum is
    // describing a filter that is not in the emitted query, and applying it
    // would bound the stratum below the rows it really answers.
    let reported = selectivity_statistics(vec![(
        iri(&ex("stratum/universal")),
        lexical_term(),
        250_000,
    )]);
    assert_eq!(
        plan(&lexical_request(), &mixed_registry(), &reported)
            .expect("plans")
            .stratum_depths[&iri(&ex("stratum/universal"))],
        200,
        "the declared bound, undisturbed"
    );
}

#[test]
fn the_planner_redeclares_no_seam_declaration_and_reads_no_ambient_state() {
    let source = include_str!("../src/planner.rs");
    // No parallel copy of the seam's declaration surface.
    for field in ["volatility:", "arity:", "modes:", "rows_per_invocation:"] {
        assert!(
            !source.contains(field),
            "planner.rs declares a parallel `{field}`; the seam is the single source"
        );
    }
    // Consumed through the registry's own describe machinery.
    assert!(source.contains("describe()"));
    assert!(source.contains("rows_per_invocation"));
    // Pure: no store, filesystem, network, clock or RNG.
    for ambient in [
        "std::fs",
        "std::net",
        "SystemTime",
        "Instant",
        "rand",
        "File::",
    ] {
        assert!(
            !source.contains(ambient),
            "planner.rs reaches for ambient state (`{ambient}`); planning must be pure"
        );
    }
}
