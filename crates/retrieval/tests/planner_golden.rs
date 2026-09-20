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
    DepthCause, DepthInputs, Iri, Metric, Plan, PlanError, RankFidelity, RegistryId,
    RejectionReason, RequestTerm, RetrievalRequest, Statistics, Term, UnservedReason, UnservedTerm,
    depth_cause, depth_from, plan,
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
        fidelity: RankFidelity::EXACT,
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
            fidelity: RankFidelity::EXACT,
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

/// Four strata whose statistics land in four different cells of the recording
/// law, plus the request's own predicate.
///
/// `mixed_registry` cannot express this: a selectivity bounds a stratum only
/// where that stratum's producer actually **receives** the term, so the two
/// strata a selectivity is asserted over here declare placing patterns, while
/// the two it must not reach are catch-alls that receive nothing.
///
/// Declared row counts are distinct and are chosen so the four derived depths
/// are distinct too (25, 30, 100, 50). A test that mis-maps one stratum's record
/// onto another therefore fails on the number rather than passing on a
/// coincidence.
fn transcript_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let literal_pattern = TermPattern {
        kind: TermKind::Literal,
        datatype: None,
        language: Some("en".to_owned()),
        predicate: Some(ex("body")),
    };
    for (name, stratum, patterns, rows) in [
        // Receives the lexical term, so a selectivity reported for it applies.
        (
            ex("pf/t-selective"),
            ex("transcript/selectivity-only"),
            vec![literal_pattern],
            100_u64,
        ),
        // Receives the seed term, so a selectivity reported for it applies.
        (
            ex("pf/t-both"),
            ex("transcript/both"),
            vec![TermPattern::of_kind(TermKind::Iri)],
            200,
        ),
        // Catch-alls: matched by everything, receiving nothing, so no
        // selectivity can bound them and only a cardinality can.
        (
            ex("pf/t-counted"),
            ex("transcript/cardinality-only"),
            vec![TermPattern::of_kind(TermKind::Any)],
            500,
        ),
        (
            ex("pf/t-silent"),
            ex("transcript/silent"),
            vec![TermPattern::of_kind(TermKind::Any)],
            50,
        ),
    ] {
        registry.register_ranked(name, relation(rows), ranked(&stratum, patterns, false));
    }
    registry
}

/// The provider for [`transcript_registry`], reporting a different combination
/// for each stratum — and one cardinality for a subject nothing consults.
///
/// That last row is the control. `never/consulted` is neither a stratum nor a
/// request predicate, so the provider is *willing* to speak about it and the
/// plan must still not name it. Without a subject the provider answers for, a
/// test asserting absence cannot tell "correctly not consulted" from "recorded
/// nothing at all".
fn transcript_statistics() -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("transcript/cardinality-only")), 100);
    cardinalities.insert(iri(&ex("transcript/both")), 100);
    cardinalities.insert(iri(&ex("never/consulted")), 9_999);
    MockStatistics {
        source: "transcript-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities,
        selectivities: vec![
            (
                iri(&ex("transcript/selectivity-only")),
                lexical_term(),
                250_000,
            ),
            (iri(&ex("transcript/both")), seed_term(), 300_000),
        ],
    }
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
    "a208c3b5fa55e6a374a2530c1368a652aa1d64edadedbb95c2caab8d9e37312b";

/// The identity of the plan the lexical-request golden records, pinned for the
/// reason [`MIXED_REQUEST_PLAN_ID`] is.
const LEXICAL_REQUEST_PLAN_ID: &str =
    "ae44005737dd7d7cbfc1c70b4c8df081d04e4247c9442f2078515c3f12e9b7ac";

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

/// One statistics-snapshot row as plain data: the subject, the reported
/// cardinality, the aggregate selectivity in parts per million, and the request
/// terms that aggregate came from.
type SnapshotRow = (String, Option<u64>, Option<u64>, Vec<u32>);

/// A plan's statistics snapshot as plain tuples, in the recorded order.
///
/// Compared whole rather than key by key, for the reason [`depths_of`] is: an
/// *extra* row is a subject the plan claims to have consulted and did not, and a
/// per-key assertion would never see it. The tuple carries every field a row has,
/// so a value mapped onto the wrong subject fails on the number.
fn entries_of(plan: &Plan) -> Vec<SnapshotRow> {
    plan.statistics_snapshot
        .entries
        .iter()
        .map(|entry| {
            (
                entry.subject.clone(),
                entry.cardinality,
                entry.selectivity_ppm,
                entry.selectivity_terms.clone(),
            )
        })
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
            fidelity: RankFidelity::EXACT,
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
            fidelity: RankFidelity::EXACT,
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
            fidelity: RankFidelity::EXACT,
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
        Some(Some(500)),
        "the predicate is named, and the provider's cardinality for it is recorded"
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
            plan.stratum_derivations
                .get(&iri(&stratum))
                .map(|inputs| inputs.cardinality),
            Some(Some(cardinality)),
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
    assert_eq!(body.cardinality, Some(500));
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

    // The record explains the depth rather than reporting a second number: the
    // value that narrowed the stratum is the value recorded beside it, and the
    // depth is recomputable from it.
    let inputs = narrowed
        .stratum_derivations
        .get(&iri(&ex("stratum/text")))
        .expect("the narrowed stratum records its derivation");
    assert_eq!(inputs.selectivity_ppm, Some(250_000));
    assert_eq!(
        inputs.selectivity_terms,
        vec![0],
        "the aggregate names the one term it came from"
    );
    narrowed.certify().expect("the plan derives its own depths");

    // The stratum the provider said nothing about records the silence rather
    // than a number, and still certifies.
    let untouched_inputs = narrowed
        .stratum_derivations
        .get(&iri(&ex("stratum/universal")))
        .expect("the untouched stratum records its derivation too");
    assert_eq!(untouched_inputs.selectivity_ppm, None);
    assert_eq!(untouched_inputs.selectivity_terms, [] as [u32; 0]);
    untouched
        .certify()
        .expect("the plan derives its own depths");
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

    // The record still holds what the provider actually said. The floor is the
    // planner's decision about the depth, not an edit of the measurement — so a
    // recorded zero beside a depth of one is the honest pair, and `certify`
    // agrees because the floor is part of the one arithmetic path.
    let floored = plan(&lexical_request(), &mixed_registry(), &none).expect("plans");
    assert_eq!(
        floored
            .stratum_derivations
            .get(&iri(&ex("stratum/text")))
            .and_then(|inputs| inputs.selectivity_ppm),
        Some(0),
        "the provider reported zero and the plan says so"
    );
    floored
        .certify()
        .expect("a floored depth is still derivable from its own inputs");
    assert_eq!(
        floored.explain_depth(&iri(&ex("stratum/text"))),
        Some(DepthCause::Floor),
        "a depth of one that came from the floor says so, rather than looking like \
         a declaration of one row"
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
    let inputs = planned
        .stratum_derivations
        .get(&iri(&ex("stratum/pair")))
        .expect("the summed stratum records its derivation");
    assert_eq!(
        inputs.selectivity_ppm,
        Some(700_000),
        "the recorded aggregate is the one the depth was derived from"
    );
    // A sum does not say which terms it came from, so the domain is recorded
    // beside it: without this, a provider that moved the same total onto other
    // terms would leave a byte-identical plan describing a different measurement.
    assert_eq!(
        inputs.selectivity_terms,
        vec![0, 1],
        "both terms contributed, and the record names both"
    );
    planned.certify().expect("the plan derives its own depths");
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

// ---------------------------------------------------------------------------
// 9. The derivation record: every consulted statistic, and only those
// ---------------------------------------------------------------------------

/// The request both transcript tests plan: one lexical term and one seed, so
/// the two placing producers each receive exactly one of them.
fn transcript_request() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term(), seed_term()])
}

/// A provider that reports a selectivity and no cardinality narrows the depth,
/// so the plan must record the statistic it was narrowed by.
///
/// This is the whole of the defect: the depth moved and the evidence did not.
/// Both halves are asserted together, because either alone passes over a
/// different bug — the depth alone is what a plan already got right, and the
/// record alone would pass for a build that recorded a statistic it never
/// applied.
#[test]
fn a_selectivity_only_stratum_is_recorded_and_narrows_the_depth() {
    let stratum = iri(&ex("transcript/selectivity-only"));
    let planned = plan(
        &transcript_request(),
        &transcript_registry(),
        &transcript_statistics(),
    )
    .expect("plans");

    assert_eq!(
        planned.stratum_depths[&stratum], 25,
        "a quarter of the declared 100 rows; the selectivity narrowed the read"
    );
    let inputs = planned
        .stratum_derivations
        .get(&stratum)
        .expect("a consulted stratum records its derivation");
    assert_eq!(
        inputs.cardinality, None,
        "the provider reported no cardinality, and absent is not zero"
    );
    assert_eq!(
        inputs.selectivity_ppm,
        Some(250_000),
        "the statistic that narrowed the depth is the statistic recorded"
    );
    assert_eq!(
        inputs.selectivity_terms,
        vec![0],
        "the lexical term is term 0"
    );
    assert_eq!(inputs.declared, 100);
    planned
        .certify()
        .expect("the depth follows from the inputs recorded beside it");
    assert_eq!(
        planned.explain_depth(&stratum),
        Some(DepthCause::Selectivity)
    );

    // The same stratum is named on the plan's own snapshot, with the cardinality
    // absent rather than zero. The neighbour in the same assertion is a stratum
    // the provider DID count, so "everything is None" cannot pass for this.
    let row = |subject: &str| {
        entries_of(&planned)
            .into_iter()
            .find(|(recorded, ..)| recorded == subject)
    };
    assert_eq!(
        row(&ex("transcript/selectivity-only")),
        Some((
            ex("transcript/selectivity-only"),
            None,
            Some(250_000),
            vec![0]
        )),
        "a stratum consulted for a selectivity alone is named, with the \
         cardinality recorded as the absence it was"
    );
    assert_eq!(
        row(&ex("transcript/cardinality-only")),
        Some((ex("transcript/cardinality-only"), Some(100), None, vec![])),
        "and the stratum the provider counted carries its count, so `None` above \
         is a measurement of silence rather than a snapshot that records nothing"
    );
}

/// A stratum the planner consulted and the provider said nothing about is named
/// with both inputs absent.
///
/// Absent from the record and "recorded as absent" are different facts: the
/// first cannot distinguish a subject never asked about from one that answered
/// nothing, and a replay against a provider that later speaks would not notice.
#[test]
fn a_consulted_but_silent_stratum_records_both_statistics_absent() {
    let planned = plan(
        &transcript_request(),
        &transcript_registry(),
        &transcript_statistics(),
    )
    .expect("plans");
    let inputs = planned
        .stratum_derivations
        .get(&iri(&ex("transcript/silent")))
        .expect("a consulted stratum is recorded even when the provider is silent");

    assert_eq!(inputs.cardinality, None);
    assert_eq!(inputs.selectivity_ppm, None);
    assert_eq!(inputs.selectivity_terms, [] as [u32; 0]);
    assert_eq!(
        inputs.declared, 50,
        "the declaration is recorded whether or not the provider spoke"
    );
    assert_eq!(
        planned.stratum_depths[&iri(&ex("transcript/silent"))],
        50,
        "a provider that measured nothing narrows nothing"
    );
    assert_eq!(
        planned.explain_depth(&iri(&ex("transcript/silent"))),
        Some(DepthCause::Declaration)
    );
}

/// A subject the provider reports and the planner never consults stays out of
/// the plan.
///
/// The control has a real oracle: `never/consulted` carries a cardinality of
/// 9999, so the provider is willing to speak about it. A build that recorded
/// every subject it could reach rather than every subject it asked about would
/// name it here. The other four are asserted present in the same test, so this
/// cannot pass by recording nothing.
#[test]
fn a_subject_the_provider_reports_but_nothing_consults_is_absent() {
    let planned = plan(
        &transcript_request(),
        &transcript_registry(),
        &transcript_statistics(),
    )
    .expect("plans");

    let never = ex("never/consulted");
    assert!(
        !planned
            .stratum_derivations
            .keys()
            .any(|stratum| stratum.as_str() == never),
        "an unconsulted subject is not a derivation"
    );
    assert!(
        !planned
            .statistics_snapshot
            .entries
            .iter()
            .any(|entry| entry.subject == never),
        "nor an ancillary entry"
    );

    for stratum in [
        "transcript/selectivity-only",
        "transcript/both",
        "transcript/cardinality-only",
        "transcript/silent",
    ] {
        assert!(
            planned.stratum_derivations.contains_key(&iri(&ex(stratum))),
            "{stratum} was consulted, so it is recorded; \
             without this the absence above would pass for a plan recording nothing"
        );
    }
}

/// The recorded subjects are exactly the ones planning consulted: every stratum
/// it derived a depth for, and every predicate the request named.
///
/// Both expectations are built from the request and the registry rather than
/// read back out of the plan, so the test states the law instead of restating
/// the output.
#[test]
fn the_record_names_exactly_the_consulted_subjects() {
    let planned = plan(
        &transcript_request(),
        &transcript_registry(),
        &transcript_statistics(),
    )
    .expect("plans");

    let derived: Vec<&str> = planned
        .stratum_derivations
        .keys()
        .map(Iri::as_str)
        .collect();
    let depths: BTreeMap<&str, u32> = planned
        .stratum_depths
        .iter()
        .map(|(stratum, depth)| (stratum.as_str(), *depth))
        .collect();
    assert_eq!(
        derived,
        depths.keys().copied().collect::<Vec<&str>>(),
        "a derivation for every depth and a depth for every derivation"
    );

    // The snapshot names every subject either statistic was consulted for: the
    // four strata, and the one predicate the request named — the lexical term
    // names `body`, the seed term names none.
    let mut expected: Vec<String> = derived
        .iter()
        .map(|stratum| (*stratum).to_owned())
        .collect();
    expected.push(ex("body"));
    expected.sort();
    assert_eq!(
        planned
            .statistics_snapshot
            .entries
            .iter()
            .map(|entry| entry.subject.clone())
            .collect::<Vec<String>>(),
        expected,
        "every stratum a depth was derived for, and every request predicate"
    );
    planned.certify().expect("every depth derives");
}

/// Every stratum a depth was derived for is named on the plan's own snapshot,
/// carrying the statistics its derivation was built from.
///
/// The whole list is asserted, and the five rows differ in every field that can
/// differ: a cardinality with no selectivity, a selectivity with no cardinality,
/// both, neither, and the request predicate the provider is silent about. A
/// build that mapped one stratum's statistics onto another, or dropped a row,
/// fails on the number rather than on a shape.
#[test]
fn the_snapshot_names_every_stratum_a_depth_was_derived_for() {
    let planned = plan(
        &transcript_request(),
        &transcript_registry(),
        &transcript_statistics(),
    )
    .expect("plans");

    assert_eq!(
        entries_of(&planned),
        vec![
            // The request's predicate. This provider says nothing about it.
            (ex("body"), None, None, vec![]),
            // A stratum the provider answered both questions for.
            (ex("transcript/both"), Some(100), Some(300_000), vec![1]),
            // A catch-all: it receives no term, so no selectivity can bound it
            // and none was consulted.
            (ex("transcript/cardinality-only"), Some(100), None, vec![]),
            // The defect's own case: a selectivity narrowed the depth and the
            // provider reported no cardinality at all.
            (
                ex("transcript/selectivity-only"),
                None,
                Some(250_000),
                vec![0]
            ),
            (ex("transcript/silent"), None, None, vec![]),
        ],
        "the snapshot names every consulted subject, with what was consulted for it"
    );
}

/// An unbounded stratum never reaches the selectivity step, so a selectivity
/// reported for it must not be recorded as though it had applied.
///
/// A fraction of an unmeasured total is not a measurement, which is why
/// `depth_from` returns before consulting it. Recording one anyway would be a
/// plan stating a derivation that did not happen — the same defect this work
/// closes, pointing the other way.
#[test]
fn an_unbounded_stratum_records_no_applied_selectivity() {
    let mut registry = PropertyFunctionRegistry::new();
    let literal_pattern = TermPattern {
        kind: TermKind::Literal,
        datatype: None,
        language: Some("en".to_owned()),
        predicate: Some(ex("body")),
    };
    registry.register_ranked(
        ex("pf/unbounded"),
        relation(u64::MAX),
        ranked(&ex("transcript/unbounded"), vec![literal_pattern], false),
    );

    let stratum = iri(&ex("transcript/unbounded"));
    let statistics = MockStatistics {
        source: "transcript-statistics".to_owned(),
        revision: "r1".to_owned(),
        // No cardinality, so the bound stays the unbounded declaration.
        cardinalities: BTreeMap::new(),
        // A selectivity the provider is perfectly willing to report.
        selectivities: vec![(stratum.clone(), lexical_term(), 250_000)],
    };

    let planned = plan(&lexical_request(), &registry, &statistics).expect("plans");
    let inputs = planned
        .stratum_derivations
        .get(&stratum)
        .expect("the stratum is consulted");

    assert_eq!(inputs.declared, u64::MAX);
    assert_eq!(inputs.cardinality, None);
    assert_eq!(
        inputs.selectivity_ppm, None,
        "the derivation returned before the selectivity step, so none was applied"
    );
    assert_eq!(
        inputs.selectivity_terms,
        [] as [u32; 0],
        "and no term contributed to a value that was never taken"
    );
    planned
        .certify()
        .expect("the recorded depth follows from the recorded inputs");
    assert_eq!(
        planned.explain_depth(&stratum),
        Some(DepthCause::Unbounded),
        "an unbounded declaration with no licensed prefix reads at the ceiling"
    );

    // And the snapshot row for that stratum is a PROJECTION of the derivation,
    // not a second consultation. The oracle is real: this provider answers
    // 250000 for exactly this `(stratum, term)` pair, so a build that re-asked
    // it would record that number here and the plan would state a derivation
    // that did not happen.
    assert_eq!(
        entries_of(&planned),
        vec![
            // The request predicate, which this provider says nothing about.
            (ex("body"), None, None, vec![]),
            (ex("transcript/unbounded"), None, None, vec![]),
        ],
        "the entry records what was consulted, which here is no selectivity at all"
    );
}

/// A subject that is both a stratum and a request-term predicate is recorded
/// once, as the derivation's own consultation.
///
/// The stratum's row wins because that is the consultation that actually bound a
/// depth: a reader checking the depth needs the selectivity the depth came from,
/// not a wider aggregate nothing was derived from.
///
/// The two candidates are made to differ so the assertion can tell which was
/// recorded. `pf/shared` receives only the lexical term, so its derivation
/// aggregates 200000 over term 0; the request-predicate rule aggregates over the
/// whole request, which under this provider is 500000 over terms 0 and 1. The
/// control stratum beside it carries a third, different pair.
#[test]
fn a_subject_that_is_both_a_stratum_and_a_request_predicate_records_the_derivation() {
    let shared = ex("body");
    let mut registry = PropertyFunctionRegistry::new();
    // Its stratum IRI *is* the lexical term's predicate IRI.
    registry.register_ranked(
        ex("pf/shared"),
        relation(1_000),
        ranked(
            &shared,
            vec![TermPattern {
                kind: TermKind::Literal,
                datatype: None,
                language: Some("en".to_owned()),
                predicate: Some(ex("body")),
            }],
            false,
        ),
    );
    // The control: a stratum that is nobody's predicate, with its own distinct
    // numbers, so a row copied onto the wrong subject fails on the value.
    registry.register_ranked(
        ex("pf/other"),
        relation(50),
        ranked(
            &ex("stratum/other"),
            vec![TermPattern::of_kind(TermKind::Iri)],
            false,
        ),
    );

    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&shared), 400);
    let statistics = MockStatistics {
        source: "shared-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities,
        selectivities: vec![
            // Reached by the stratum, because `pf/shared` receives term 0.
            (iri(&shared), lexical_term(), 200_000),
            // Reached only by the request-predicate rule, which aggregates over
            // every term: `pf/shared` does not receive the seed.
            (iri(&shared), seed_term(), 300_000),
            (iri(&ex("stratum/other")), seed_term(), 600_000),
        ],
    };

    let planned =
        plan(&lexical_and_seed_request(), &registry, &statistics).expect("the request plans");

    assert_eq!(
        entries_of(&planned),
        vec![
            (shared.clone(), Some(400), Some(200_000), vec![0]),
            (ex("stratum/other"), None, Some(600_000), vec![1]),
        ],
        "one row for the shared subject, and it is the derivation's"
    );
    assert_ne!(
        entries_of(&planned)[0],
        (shared.clone(), Some(400), Some(500_000), vec![0, 1]),
        "the request-predicate aggregate over the whole request is the value that \
         must NOT have been recorded"
    );

    // The row is the derivation's because the derivation says the same thing,
    // and the depths differ per stratum so neither row can be the other's.
    let derivation = planned
        .stratum_derivations
        .get(&iri(&shared))
        .expect("the shared subject is a stratum");
    assert_eq!(derivation.cardinality, Some(400));
    assert_eq!(derivation.selectivity_ppm, Some(200_000));
    assert_eq!(derivation.selectivity_terms, vec![0]);
    assert_eq!(
        depths_of(&planned),
        BTreeMap::from([(shared, 80), (ex("stratum/other"), 30)]),
        "a fifth of 400 rows, and three fifths of 50"
    );
    planned.certify().expect("the two records agree");
}

/// Every plan the golden corpus pins derives its own depths, and those depths
/// are the ones version 3 recorded.
///
/// The second half is the no-semantic-drift check. The literals are the depths
/// transcribed from the committed version-3 fixtures before they were
/// regenerated, so this compares the new arithmetic against the old behaviour
/// rather than against its own output.
#[test]
fn every_golden_plan_certifies_at_the_depths_version_three_recorded() {
    for (request, registry, expected) in [
        (
            mixed_request(),
            mixed_registry(),
            vec![
                (ex("stratum/graph"), 50_u32),
                (ex("stratum/text"), 100),
                (ex("stratum/universal"), 200),
            ],
        ),
        (
            lexical_request(),
            mixed_registry(),
            vec![(ex("stratum/text"), 100), (ex("stratum/universal"), 200)],
        ),
    ] {
        let planned = plan(&request, &registry, &fixture_statistics()).expect("plans");
        planned
            .certify()
            .expect("a golden plan derives its own depths");
        let recorded: Vec<(String, u32)> = expected
            .iter()
            .map(|(stratum, _)| (stratum.clone(), planned.stratum_depths[&iri(stratum)]))
            .collect();
        assert_eq!(
            recorded, expected,
            "the depths are the ones version 3 recorded; the record grew, the plan did not move"
        );
    }
}

/// Every value a version-3 plan recorded is still recorded, in the place
/// version 3 recorded it.
///
/// The expected literals are transcribed from the committed version-3 golden:
/// `body` 500/500000 as a request predicate, and the three strata's
/// cardinalities on the snapshot. The derivation record was *added* beside them;
/// nothing moved out of the snapshot to make room for it, because a reader who
/// asks a plan which subjects it consulted must still be told all of them.
#[test]
fn a_cardinality_carrying_plan_records_every_value_version_three_did() {
    let planned = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");

    assert_eq!(
        entries_of(&planned),
        vec![
            (ex("body"), Some(500), Some(500_000), vec![0]),
            (ex("stratum/graph"), Some(50), None, vec![]),
            (ex("stratum/text"), Some(100), None, vec![]),
            (ex("stratum/universal"), Some(1000), None, vec![]),
        ],
        "the four rows version 3's snapshot carried, with the four values it carried"
    );

    for (stratum, cardinality) in [
        (ex("stratum/graph"), 50_u64),
        (ex("stratum/text"), 100),
        (ex("stratum/universal"), 1000),
    ] {
        let inputs = planned
            .stratum_derivations
            .get(&iri(&stratum))
            .expect("every stratum records its derivation");
        assert_eq!(
            inputs.cardinality,
            Some(cardinality),
            "{stratum}'s cardinality is the one version 3 recorded"
        );
        assert_eq!(
            inputs.selectivity_ppm, None,
            "{stratum} carried no selectivity in version 3 either"
        );
    }
}

/// A missing row declaration is never defaulted into a measurement.
///
/// `place` admits an invocation only where some declared mode subsumes it, and
/// `declared_row_bound` reads the declaration by filtering on that same
/// predicate — so a placement that succeeded has already proved the bound
/// exists, and `PlanError::UndeclaredRowBound` is unreachable against a registry
/// that did not move. This executes that guard rather than assuming it.
///
/// The value matters because it is an input every depth is derived from. A
/// default of zero floors the read to one probing row — a plan quietly asking a
/// producer for a single row — while `u64::MAX` declares an unbounded relation.
/// Neither is a thing the registry said.
///
/// Both halves run, because a refusal is a claim too: the producer whose
/// declared mode subsumes nothing is refused, and its neighbour — the same
/// registry with a mode that does subsume the invocation — plans, and records
/// the declaration it really made rather than `declared: 0` at `depth: 1`.
#[test]
fn an_undeclared_row_bound_is_not_a_declaration_of_zero_rows() {
    let stratum = ex("transcript/undeclared");
    let plan_with = |mode: BindingPattern| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            ex("pf/mode"),
            Arc::new(MockProducer {
                arity: PfArity::new(1, 1),
                mode,
                rows: 10,
            }),
            ranked(&stratum, vec![TermPattern::of_kind(TermKind::Any)], false),
        );
        plan(&lexical_request(), &registry, &transcript_statistics())
    };

    // The `Any` pattern declares no placement, so the invocation leaves every
    // argument free. A producer declaring only the fully-bound mode subsumes it
    // nowhere.
    let refused = plan_with(BindingPattern::from_bools([true, true]));
    assert!(
        matches!(refused, Err(PlanError::NoApplicableProducers)),
        "placement refuses before a row bound is ever read, which is what makes \
         an absent declaration unreachable at the derivation: {refused:?}"
    );

    // The neighbour: a producer whose all-free mode does subsume that same
    // invocation plans, and its declaration reaches the record intact.
    let planned = plan_with(PfArity::new(1, 1).all_free_mode()).expect("the neighbour plans");
    let inputs = planned
        .stratum_derivations
        .get(&iri(&stratum))
        .expect("the stratum records its derivation");
    assert_eq!(
        inputs.declared, 10,
        "the declaration the registry made is the one recorded"
    );
    assert_ne!(
        (inputs.declared, planned.stratum_depths[&iri(&stratum)]),
        (0, 1),
        "a missing declaration defaulted to zero would floor this read to one row"
    );
    assert_eq!(planned.stratum_depths[&iri(&stratum)], 10);
    planned
        .certify()
        .expect("the depth derives from the declaration");
}

// ---------------------------------------------------------------------------
// The read ceiling names itself
// ---------------------------------------------------------------------------

/// The deepest depth a read can be taken to.
///
/// `MAX_READ_DEPTH` is crate-private, so this restates it — which is the point:
/// a test that imported the planner's own constant could not observe the
/// planner and the ceiling disagreeing about where it is. It is one row
/// shallower than a `u32` holds, because the compiler reads one row PAST the
/// depth to tell an exhausted stratum from a truncated one, and at `u32::MAX`
/// that probe row is not a number a `LIMIT` can express.
const CEILING: u32 = u32::MAX - 1;

/// Inputs with nothing set: each case below turns on exactly the legs it names.
fn depth_inputs(declared: u64) -> DepthInputs {
    DepthInputs {
        declared,
        cardinality: None,
        selectivity_ppm: None,
        selectivity_terms: Vec::new(),
        licensed_prefix: None,
    }
}

/// A depth the read ceiling cut is explained as the ceiling, not as the input
/// that named the uncut number.
///
/// The ceiling sits at `u32::MAX - 1`, so a cause decided by whether the derived
/// bound *fits a `u32`* reads it one row too wide: at exactly `u32::MAX` the
/// conversion succeeds, the ceiling arm is skipped, and the explanation names
/// the declaration — or the cardinality, or the selectivity, or the prefix —
/// while the depth beside it has been clamped with nothing saying so. That is a
/// second derivation disagreeing with the first, which is exactly what a
/// recorded derivation exists to rule out.
///
/// Both halves run for every finite leg, because over-attribution is the mirror
/// of the silent drop. The row ABOVE the ceiling must say `ReadCeiling`; its
/// neighbour AT the ceiling must still say which input really bound it, and a
/// fix that answered `ReadCeiling` for the neighbour would be a failed fix. The
/// oracle is the pair of numbers, not the variant alone: the over-ceiling row's
/// derived bound is strictly deeper than the depth recorded for it, and the
/// neighbour's is that depth exactly, so a case that mapped one input onto the
/// other's would fail on arithmetic before it reached the cause.
#[test]
fn a_depth_the_ceiling_cut_is_explained_as_the_ceiling() {
    // The declaration alone. `u32::MAX` rows declared is one row deeper than a
    // read can go; `u32::MAX - 1` is a declaration served whole.
    let over = depth_inputs(u64::from(u32::MAX));
    assert_eq!(depth_from(&over), CEILING);
    assert_eq!(
        depth_cause(&over),
        DepthCause::ReadCeiling,
        "the declaration named {} rows and the read stops one short of it, so the \
         ceiling is what fixed this depth",
        u32::MAX
    );
    let at = depth_inputs(u64::from(CEILING));
    assert_eq!(
        depth_from(&at),
        CEILING,
        "the deepest declaration a read can serve whole is served whole"
    );
    assert_eq!(
        depth_cause(&at),
        DepthCause::Declaration,
        "and nothing cut it, so the declaration is still the cause"
    );

    // The cardinality leg, under an unbounded declaration so the measurement is
    // unambiguously the narrower input.
    let over = DepthInputs {
        cardinality: Some(u64::from(u32::MAX)),
        ..depth_inputs(u64::MAX)
    };
    assert_eq!(depth_from(&over), CEILING);
    assert_eq!(depth_cause(&over), DepthCause::ReadCeiling);
    let at = DepthInputs {
        cardinality: Some(u64::from(CEILING)),
        ..depth_inputs(u64::MAX)
    };
    assert_eq!(depth_from(&at), CEILING);
    assert_eq!(
        depth_cause(&at),
        DepthCause::Cardinality,
        "a measurement exactly at the ceiling is served whole, and it is the \
         measurement that bound the read"
    );

    // The selectivity leg: half of twice the boundary lands on the boundary.
    let over = DepthInputs {
        selectivity_ppm: Some(500_000),
        selectivity_terms: vec![0],
        ..depth_inputs(2 * u64::from(u32::MAX))
    };
    assert_eq!(depth_from(&over), CEILING);
    assert_eq!(depth_cause(&over), DepthCause::ReadCeiling);
    let at = DepthInputs {
        selectivity_ppm: Some(500_000),
        selectivity_terms: vec![0],
        ..depth_inputs(2 * u64::from(CEILING))
    };
    assert_eq!(depth_from(&at), CEILING);
    assert_eq!(
        depth_cause(&at),
        DepthCause::Selectivity,
        "the ratio halved the declaration onto the boundary, so the ratio is the cause"
    );

    // The licensed prefix, against a finite declaration deeper than any read.
    let over = DepthInputs {
        licensed_prefix: Some(u64::from(u32::MAX)),
        ..depth_inputs(u64::MAX - 1)
    };
    assert_eq!(depth_from(&over), CEILING);
    assert_eq!(depth_cause(&over), DepthCause::ReadCeiling);
    let at = DepthInputs {
        licensed_prefix: Some(u64::from(CEILING)),
        ..depth_inputs(u64::MAX - 1)
    };
    assert_eq!(depth_from(&at), CEILING);
    assert_eq!(
        depth_cause(&at),
        DepthCause::LicensedPrefix,
        "a prefix exactly at the ceiling is the number recorded, so it is the cause"
    );

    // And the control, nowhere near the boundary: a small declaration is served
    // at its own number and explained as itself. Its depth differs from every
    // row above, so a rendering that answered `ReadCeiling` for everything would
    // fail here on the variant and everywhere above on nothing.
    let small = depth_inputs(10);
    assert_eq!(depth_from(&small), 10);
    assert_eq!(depth_cause(&small), DepthCause::Declaration);
}

/// An unbounded declaration reaches the ceiling by three roads, and the
/// explanation tells them apart.
///
/// The prefix branch is the one that used to lie: a request bound deeper than a
/// read can go was reported as `LicensedPrefix`, crediting the request with a
/// narrowing to a number the plan does not record. Its valid neighbour is a
/// prefix exactly at the ceiling, which really is the recorded depth and really
/// is the cause — the two differ by one row, and the pair is here so a fix that
/// collapsed them would show.
#[test]
fn an_unbounded_declaration_names_which_road_reached_the_ceiling() {
    // No prefix: the declaration promised more rows than a read can reach and
    // nothing narrowed it.
    let unbounded = depth_inputs(u64::MAX);
    assert_eq!(depth_from(&unbounded), CEILING);
    assert_eq!(depth_cause(&unbounded), DepthCause::Unbounded);

    // A prefix that is itself the unbounded sentinel narrows nothing, so the
    // fact is still "more rows than a read can reach" rather than a request
    // bound this plan was cut to.
    let vacuous = DepthInputs {
        licensed_prefix: Some(u64::MAX),
        ..depth_inputs(u64::MAX)
    };
    assert_eq!(depth_from(&vacuous), CEILING);
    assert_eq!(depth_cause(&vacuous), DepthCause::Unbounded);

    // A prefix one row past the ceiling: the depth recorded is NOT the prefix,
    // so the prefix is not the cause.
    let over = DepthInputs {
        licensed_prefix: Some(u64::from(u32::MAX)),
        ..depth_inputs(u64::MAX)
    };
    assert_eq!(depth_from(&over), CEILING);
    assert_eq!(depth_cause(&over), DepthCause::ReadCeiling);

    // The neighbour: a prefix exactly at the ceiling IS the depth recorded.
    let at = DepthInputs {
        licensed_prefix: Some(u64::from(CEILING)),
        ..depth_inputs(u64::MAX)
    };
    assert_eq!(depth_from(&at), CEILING);
    assert_eq!(depth_cause(&at), DepthCause::LicensedPrefix);

    // And a prefix far below it, so the control's depth differs from every row
    // above: a case that read the prefix and a case that ignored it cannot both
    // pass here.
    let narrow = DepthInputs {
        licensed_prefix: Some(7),
        ..depth_inputs(u64::MAX)
    };
    assert_eq!(depth_from(&narrow), 7);
    assert_eq!(depth_cause(&narrow), DepthCause::LicensedPrefix);
}

/// A licensed prefix of zero rows under an unbounded declaration is the floor,
/// not the prefix.
///
/// Both cases record a depth of one, which is exactly why the cause has to tell
/// them apart: a request that licensed one row got the row it asked for, and a
/// request that licensed none got a probing row the plan added over its head so
/// the producer — not the plan — is what reports the stratum empty. Crediting
/// the second to the prefix would say the caller asked for the row it is about
/// to be handed.
#[test]
fn a_prefix_of_no_rows_is_the_floor_rather_than_the_prefix() {
    let none = DepthInputs {
        licensed_prefix: Some(0),
        ..depth_inputs(u64::MAX)
    };
    assert_eq!(depth_from(&none), 1, "the floor lifts it to a probing row");
    assert_eq!(depth_cause(&none), DepthCause::Floor);

    // The neighbour that must NOT move: one licensed row is one row the caller
    // really asked for.
    let one = DepthInputs {
        licensed_prefix: Some(1),
        ..depth_inputs(u64::MAX)
    };
    assert_eq!(depth_from(&one), 1);
    assert_eq!(depth_cause(&one), DepthCause::LicensedPrefix);

    // And the finite branch already agreed, which is the consistency at issue:
    // the same zero against a declaration of ten reads the same way.
    let finite = DepthInputs {
        licensed_prefix: Some(0),
        ..depth_inputs(10)
    };
    assert_eq!(depth_from(&finite), 1);
    assert_eq!(depth_cause(&finite), DepthCause::Floor);
}

/// A cause that names an input names an input whose value IS the recorded depth.
///
/// This is the property the ceiling bug broke, stated without restating the
/// arithmetic: a plan declaring `u32::MAX` rows records a depth of
/// `u32::MAX - 1`, so "the declaration bound it" is a claim the two numbers
/// falsify. The check needs no second derivation — it reads the depth the
/// planner recorded and the input the planner blamed and asks whether they are
/// the same number — which is why it can sit over a sweep of every boundary the
/// derivation has on every leg without becoming the reimplementation it exists
/// to rule out.
///
/// The three causes that name no input are pinned to the number they mean
/// instead: the floor is one row, and the ceiling and the unbounded declaration
/// are the deepest a read can be taken to.
#[test]
fn a_named_cause_names_the_number_recorded() {
    let boundaries = [
        0_u64,
        1,
        2,
        u64::from(CEILING) - 1,
        u64::from(CEILING),
        u64::from(u32::MAX),
        u64::from(u32::MAX) + 1,
        u64::MAX - 1,
        u64::MAX,
    ];
    let cardinalities = [
        None,
        Some(0),
        Some(1),
        Some(u64::from(CEILING)),
        Some(u64::from(u32::MAX)),
        Some(u64::MAX),
    ];
    let selectivities = [None, Some(0), Some(1), Some(500_000), Some(1_000_000)];
    let prefixes = [
        None,
        Some(0),
        Some(1),
        Some(u64::from(CEILING)),
        Some(u64::from(u32::MAX)),
        Some(u64::MAX),
    ];
    let mut seen: Vec<DepthCause> = Vec::new();
    for declared in boundaries {
        for cardinality in cardinalities {
            for selectivity_ppm in selectivities {
                for licensed_prefix in prefixes {
                    let inputs = DepthInputs {
                        declared,
                        cardinality,
                        selectivity_ppm,
                        selectivity_terms: if selectivity_ppm.is_some() {
                            vec![0]
                        } else {
                            Vec::new()
                        },
                        licensed_prefix,
                    };
                    let depth = u64::from(depth_from(&inputs));
                    let cause = depth_cause(&inputs);
                    if !seen.contains(&cause) {
                        seen.push(cause);
                    }
                    // `min` is not the derivation under test: it is the
                    // definition of "the narrower of the two declarations", and
                    // the selectivity leg is checked against it rather than
                    // recomputed.
                    let measured = cardinality.map_or(declared, |rows| declared.min(rows));
                    // Independent checks rather than one `match`: every cause
                    // the classification has is pinned below, and the tally at
                    // the end refuses a run that reached an eighth.
                    if cause == DepthCause::Declaration {
                        assert_eq!(
                            declared, depth,
                            "the declaration was blamed for a depth it does not equal: {inputs:?}"
                        );
                    }
                    if cause == DepthCause::Cardinality {
                        assert_eq!(
                            cardinality,
                            Some(depth),
                            "the measurement was blamed for a depth it does not equal: {inputs:?}"
                        );
                    }
                    if cause == DepthCause::LicensedPrefix {
                        assert_eq!(
                            licensed_prefix,
                            Some(depth),
                            "the request's bound was blamed for a depth it does not equal: \
                             {inputs:?}"
                        );
                    }
                    if cause == DepthCause::Selectivity {
                        assert!(
                            depth < measured,
                            "a ratio that narrowed nothing was blamed: {inputs:?}"
                        );
                        assert!(
                            licensed_prefix.is_none_or(|rows| rows >= depth),
                            "the request's bound was at least as narrow, so the ratio is not \
                             what bound this: {inputs:?}"
                        );
                    }
                    if cause == DepthCause::Floor {
                        assert_eq!(
                            depth, 1,
                            "the floor means one probing row and nothing else: {inputs:?}"
                        );
                    }
                    if cause == DepthCause::ReadCeiling || cause == DepthCause::Unbounded {
                        assert_eq!(
                            depth,
                            u64::from(CEILING),
                            "a read that went as deep as a read can go is the only thing either \
                             of these means: {inputs:?}"
                        );
                    }
                }
            }
        }
    }
    // The sweep is worth what it covers: every cause the classification has must
    // have been reached, or the assertions above ran over a hole.
    seen.sort_by_key(|cause| format!("{cause:?}"));
    assert_eq!(
        seen,
        vec![
            DepthCause::Cardinality,
            DepthCause::Declaration,
            DepthCause::Floor,
            DepthCause::LicensedPrefix,
            DepthCause::ReadCeiling,
            DepthCause::Selectivity,
            DepthCause::Unbounded,
        ],
        "the sweep reaches every cause"
    );
}

/// Every stratum's derivation carries the row bound ITS OWN producer declared.
///
/// The declaration reaches `DepthInputs.declared` from a map keyed by stratum,
/// and this is the witness that the key and the value travel together. Four
/// surviving strata declare four different row counts, the provider narrows none
/// of them, and no request bound licenses a prefix — so each depth IS that
/// stratum's declaration and no two of them are the same number. A build that
/// read one stratum's bound under another's key lands on the wrong depth for at
/// least two of the four, and a build that defaulted a bound it could not find
/// records `declared: 0` and a floored depth of one, which is neither of the
/// four numbers below.
#[test]
fn every_stratum_records_the_row_bound_its_own_producer_declared() {
    let planned = plan(
        &mixed_request(),
        &mixed_registry(),
        // Reports the request predicate and nothing about any stratum, so
        // nothing lowers a declaration and the depth is the declaration.
        &no_stratum_cardinality(),
    )
    .expect("plans");

    for (stratum, declared) in [
        (ex("stratum/universal"), 200_u64),
        (ex("stratum/text"), 100),
        (ex("stratum/graph"), 50),
    ] {
        let inputs = planned
            .stratum_derivations
            .get(&iri(&stratum))
            .unwrap_or_else(|| panic!("{stratum} records a derivation"));
        assert_eq!(
            inputs.declared, declared,
            "{stratum} records the bound its own producer declared"
        );
        assert_eq!(
            inputs.cardinality, None,
            "{stratum} was not measured, so nothing narrowed the declaration"
        );
        assert_eq!(
            planned.stratum_depths[&iri(&stratum)],
            u32::try_from(declared).expect("the fixture bounds fit a read depth"),
            "{stratum} reads exactly as deep as it declared"
        );
    }
    assert_eq!(
        planned.stratum_derivations.len(),
        3,
        "three strata survived — the quoted one's producer accepts no term this \
         request carries — and a fourth would mean a bound arrived from nowhere"
    );
    planned
        .certify()
        .expect("each depth follows from the inputs recorded beside it");
}
