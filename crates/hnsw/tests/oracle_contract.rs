// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The approximation contract and the no-completeness claim, asserted.
//!
//! HNSW is an approximate index. This suite makes that contractual rather than
//! aspirational by testing the three governed channels and the negative behaviour the
//! contract forbids:
//!
//! 1. the artifact-level loss contract says *approximate*, the implementation identity
//!    carries the evidence sentence, and the adapter verifies both at bind time;
//! 2. the relation is registered under the caller's predicate IRI, with no fabricated
//!    namespace;
//! 3. an empty or incomplete result is an **offer of candidates**, never a certification
//!    that a nearer row does not exist — even when the exact oracle proves one does.
//!
//! The work-accounting contract lives here too, because it is the same boundary: a search
//! that examined a million candidates to return five must report the million, or the
//! engine's budget bounds the answer set rather than the execution.

#[path = "support/purremb.rs"]
mod purremb;

use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{EmbeddingView, TermValue, verify_embedding};
use purrdf_hnsw::relation::{HnswRelation, HnswSpace, register_hnsw_relation};
use purrdf_hnsw::{HnswIndex, Params, VectorMatrix, guard, level::splitmix64, profile};
use purrdf_sparql_eval::knn::{Kernel, Ranked, best, norm};
use purrdf_sparql_eval::{
    EvalError, KnnGuard, PfArgs, PfRow, PropertyFunction, PropertyFunctionRegistry,
};

use purrdf_core::DistanceMetric;

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// A deterministic fixture matrix.
fn matrix(rows: usize, dims: usize, seed: u64) -> VectorMatrix {
    let mut state = seed;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        state = splitmix64(state);
        let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
        let value = unit.mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.25 } else { value });
    }
    VectorMatrix::new(rows, dims, data).expect("valid fixture")
}

/// A space over a fresh fixture, plus the matrix its exact oracle ranks.
fn fixture_space(rows: usize, dims: usize, params: Params) -> (VectorMatrix, Arc<HnswSpace>) {
    let matrix = matrix(rows, dims, 0xabcd_ef01_2345_6789);
    let index = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params)
        .expect("builds");
    let terms = (0..rows)
        .map(|row| TermValue::iri(format!("https://example.org/oracle/{row}")))
        .collect();
    let guard = KnnGuard::new(rows as u64, rows as u64).expect("valid");
    let space = Arc::new(HnswSpace::from_index(index, terms, guard).expect("space"));
    (matrix, space)
}

/// The exact top-`k` rows, by the shared kernel — this suite's oracle.
fn exact_top_k(matrix: &VectorMatrix, query_row: usize, k: usize) -> Vec<usize> {
    let kernel = Kernel::SquaredEuclidean;
    let query = matrix.row(query_row);
    let query_norm = norm(query);
    let scored: Vec<Ranked> = (0..matrix.rows())
        .map(|row| Ranked {
            distance: kernel
                .distance(query, query_norm, matrix.row(row), norm(matrix.row(row)))
                .expect("finite"),
            row,
        })
        .collect();
    best(k, scored)
        .into_iter()
        .map(|ranked| ranked.row)
        .collect()
}

/// The row index a term occupies in a space.
fn row_of(space: &HnswSpace, row: usize) -> TermValue {
    space.term(row).expect("a term").clone()
}

/// Open the relation and drain it, returning the rows and the work reported.
fn drain(
    relation: &HnswRelation,
    seed: Option<&TermValue>,
    k: &str,
    neighbour: Option<&TermValue>,
    ceiling: Option<u64>,
) -> (Vec<PfRow>, u64) {
    let count = TermValue::typed_literal(k, XSD_INTEGER);
    let subject = [neighbour];
    let object = [seed, Some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, ceiling).expect("opens");
    let mut rows = Vec::new();
    while let Some(row) = cursor.next().expect("no error") {
        rows.push(row);
    }
    (rows, cursor.take_work())
}

// ---------------------------------------------------------------------------
// The approximation contract
// ---------------------------------------------------------------------------

#[test]
fn the_artifact_binds_the_approximate_loss_contract_and_evidence() {
    let fixture = purremb::Fixture::new(16, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let mut view = EmbeddingView::from_bytes(&fixture.bytes).expect("opens");
    verify_embedding(&mut view).expect("verifies");
    let selected = guard::select(&view).expect("selects");

    // The loss contract carries PURREMB's own `approximate = true` field and this profile's
    // `transforms_vectors = false`.
    let entries: Vec<_> = purrdf_core::canonical_tlv(selected.guard_bytes())
        .expect("the guard parses")
        .collect();
    let loss = entries
        .iter()
        .find(|entry| entry.tag == 5)
        .expect("a loss contract");
    let loss_entries: Vec<_> = purrdf_core::canonical_tlv(loss.value)
        .expect("the loss block parses")
        .collect();
    let flag = |tag: u16| {
        loss_entries
            .iter()
            .find(|entry| entry.tag == tag)
            .map(|entry| entry.value)
    };
    assert_eq!(flag(1), Some([1].as_slice()), "approximate");
    assert_eq!(flag(2), Some([0].as_slice()), "no vector transform");

    // The evidence sentence is bound into the implementation identity, so the guard digest
    // covers it, and the adapter verifies it at bind time.
    assert_eq!(
        profile::LOSS_EVIDENCE,
        "approximate: recall measured against the exact oracle on synthetic corpora up to \
         50,000 rows, and UNMEASURED at the 10^6 scale this index exists for; an offer of \
         candidates is never a proof of absence"
    );
    assert_eq!(
        profile::implementation().revision.as_deref(),
        Some(profile::LOSS_EVIDENCE.as_bytes())
    );
    assert!(guard::validate_guard(&selected).is_ok());
}

#[test]
fn the_relation_predicate_iri_is_callers_supplied() {
    let (_, space) = fixture_space(8, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let iri = "https://example.org/oracle/nearest";
    let mut registry = PropertyFunctionRegistry::new();
    register_hnsw_relation(&mut registry, iri, Arc::clone(&space));
    assert!(registry.resolve(iri).is_some());
    assert!(
        registry
            .resolve("https://example.org/oracle/other")
            .is_none(),
        "no namespace is fabricated: an unregistered IRI resolves to nothing"
    );
    let described = registry.describe().expect("describes");
    assert_eq!(described.len(), 1);
    assert_eq!(described[0].iri, iri);
}

// ---------------------------------------------------------------------------
// No completeness claim
// ---------------------------------------------------------------------------

#[test]
fn an_empty_candidate_offer_is_not_evidence_of_absence() {
    let (matrix, space) = fixture_space(64, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let relation = space.relation();

    let query_row = 0;
    let exact = exact_top_k(&matrix, query_row, 1)[0];
    let other = (0..space.row_count())
        .find(|&row| row != exact)
        .expect("a row that is not the exact nearest");

    let seed = row_of(&space, query_row);
    let bound = row_of(&space, other);

    // Binding `?neighbour` to a row that is not returned makes the cursor empty — but that
    // is a filter result, not a statement that the row is absent from the graph.
    let (hits, _) = drain(&relation, Some(&seed), "1", Some(&bound), None);
    assert!(hits.is_empty(), "the bound row is not among the top one");
    assert!(
        space.row_of(&bound).is_some(),
        "the row the cursor did not offer is still a row of the space"
    );

    // With the same seed and no neighbour bound, the relation offers a candidate: the
    // empty answer was about the filter, and the graph is non-empty.
    let (unbound, _) = drain(&relation, Some(&seed), "1", None, None);
    assert_eq!(unbound.len(), 1);
    assert!(space.row_count() > 0);
}

#[test]
fn an_approximate_miss_is_an_offer_never_a_proof_of_absence() {
    // A deliberately sparse graph (M = M0 = ef_construction = 2, ef_search = 1) so the
    // search genuinely misses the exact oracle on some queries — a test where it never
    // missed would be watching nothing.
    let params = Params::new(2, 2, 2, 1).expect("valid");
    let (matrix, space) = fixture_space(64, 4, params);
    let relation = space.relation();

    let mut misses = 0;
    for query_row in 0..matrix.rows() {
        let exact = exact_top_k(&matrix, query_row, 1)[0];
        let seed = row_of(&space, query_row);
        let (rows, work) = drain(&relation, Some(&seed), "1", None, None);
        assert!(work > 0, "a non-empty search reports its evaluations");
        let offered: Vec<usize> = rows
            .iter()
            .map(|row| {
                let term = row[0].clone();
                space.row_of(&term).expect("the offered term is a row")
            })
            .collect();
        if offered.first().copied() != Some(exact) {
            misses += 1;
            // The oracle knows a nearer row. The relation never says it does not exist —
            // it simply did not offer it — and that row is still a row of the space.
            assert!(
                space.term(exact).is_some(),
                "the missed row is a real row; the cursor's silence is not absence"
            );
            assert!(
                !offered.contains(&exact),
                "an approximate miss is exactly a row the cursor did not offer"
            );
        }
    }
    assert!(
        misses > 0,
        "the sparse fixture must actually miss the oracle somewhere, else this test proves \
         nothing; adjust the fixture rather than deleting the assertion"
    );
}

#[test]
fn an_absent_seed_is_an_empty_offer_not_a_missing_graph() {
    let (_, space) = fixture_space(32, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let relation = space.relation();
    let absent = TermValue::iri("https://example.org/oracle/not-a-row");
    assert!(space.row_of(&absent).is_none());

    let (rows, work) = drain(&relation, Some(&absent), "3", None, None);
    assert!(
        rows.is_empty(),
        "a seed the space does not hold answers nothing"
    );
    assert_eq!(work, 0, "and there was nothing to evaluate");
    assert!(
        space.row_count() > 0,
        "the graph is not empty: the empty answer is about the seed, never the data"
    );
}

// ---------------------------------------------------------------------------
// Work accounting
// ---------------------------------------------------------------------------

#[test]
fn the_search_is_lazy_and_its_work_is_charged_once() {
    let (_, space) = fixture_space(64, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let relation = space.relation();
    let seed = row_of(&space, 3);
    let count = TermValue::typed_literal("3", XSD_INTEGER);
    let subject = [None];
    let object = [Some(&seed), Some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None).expect("opens");

    assert_eq!(
        cursor.take_work(),
        0,
        "opening performs no search; the work is charged when the first row is pulled"
    );
    let mut rows = 0;
    while cursor.next().expect("no error").is_some() {
        rows += 1;
    }
    assert!(rows <= 3);
    let work = cursor.take_work();
    assert!(
        work > 0,
        "a drained search reports the candidates it examined"
    );
    assert!(work <= 64, "it can never examine more than the graph holds");
    assert_eq!(cursor.take_work(), 0, "work is spent, not re-reported");
}

#[test]
fn a_ceiling_that_prevents_the_first_pull_charges_nothing() {
    let (_, space) = fixture_space(64, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let relation = space.relation();
    let seed = row_of(&space, 0);
    let count = TermValue::typed_literal("5", XSD_INTEGER);
    let subject = [None];
    let object = [Some(&seed), Some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, Some(0)).expect("opens");
    assert!(cursor.next().expect("no error").is_none());
    assert_eq!(cursor.take_work(), 0);
}

#[test]
fn a_planner_ceiling_never_changes_which_rows_are_offered() {
    // `ef_search` is artifact identity. If the beam width were widened to fit `k`, then a
    // planner-supplied row ceiling — which shrinks the `k` handed to the search — would
    // shrink the beam too, and the same query would return a *different* candidate set
    // depending on a query-plan artifact. `HnswRelation` declares `Volatility::Stable`, so
    // that would be a false declaration rather than merely a surprise.
    //
    // The regime that exposes it is `k > ef_search`, which every other ceiling test in this
    // file avoids.
    let (_, space) = fixture_space(64, 4, Params::new(4, 8, 16, 4).expect("valid"));
    let relation = space.relation();
    let seed = row_of(&space, 0);

    let (unbounded, _) = drain(&relation, Some(&seed), "16", None, None);
    let (ceilinged, _) = drain(&relation, Some(&seed), "16", None, Some(3));

    assert!(
        !ceilinged.is_empty(),
        "the ceiling should shorten the offer, not empty it"
    );
    assert!(
        ceilinged.len() <= unbounded.len(),
        "a ceiling can only shorten the offer"
    );
    assert_eq!(
        ceilinged,
        unbounded[..ceilinged.len()],
        "the ceilinged offer must be a prefix of the unbounded one: the ceiling decides how \
         many rows are emitted, never which graph is searched"
    );
}

#[test]
fn a_request_wider_than_the_beam_is_answered_short_not_widened() {
    // An offer of fewer than `k` rows is legal — this index offers candidates and never
    // certifies absence. Silently searching a wider graph than the artifact declares is not.
    let params = Params::new(4, 8, 16, 4).expect("valid");
    let (_, space) = fixture_space(64, 4, params);
    let seed = row_of(&space, 0);
    let (rows, _) = drain(&space.relation(), Some(&seed), "40", None, None);
    assert_eq!(
        rows.len(),
        params.ef_search(),
        "a request for 40 rows against ef_search={} must be answered with exactly the beam \
         the artifact declares — neither widened to 40 nor short of the beam",
        params.ef_search()
    );
}

#[test]
fn a_zero_request_charges_nothing() {
    let (_, space) = fixture_space(32, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let seed = row_of(&space, 0);
    let (rows, work) = drain(&space.relation(), Some(&seed), "0", None, None);
    assert_eq!(rows, Vec::<PfRow>::new());
    assert_eq!(work, 0);
}

#[test]
fn budget_exhaustion_mid_traversal_still_reports_the_work_done() {
    let (_, space) = fixture_space(64, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let relation = space.relation();
    let seed = row_of(&space, 2);
    let count = TermValue::typed_literal("10", XSD_INTEGER);
    let subject = [None];
    let object = [Some(&seed), Some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, Some(1)).expect("opens");
    let first = cursor.next().expect("no error");
    assert!(first.is_some(), "the licence admits one row");
    let work = cursor.take_work();
    assert!(work > 0);
    assert!(cursor.next().expect("no error").is_none());
    assert_eq!(cursor.take_work(), 0);
}

#[test]
fn a_rejected_payload_is_a_construction_failure_not_a_search() {
    let fixture = purremb::Fixture::new(16, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let error = HnswSpace::from_artifact(
        &fixture.tampered_payload(),
        fixture.target_set,
        fixture.vector_space,
        fixture.bindings(),
        KnnGuard::new(16, 16).expect("valid"),
    )
    .expect_err("the tampered payload is refused");
    assert!(matches!(error, EvalError::Data { .. }));
}

#[test]
fn a_bound_position_filter_is_honoured_without_shrinking_the_offer() {
    let (_, space) = fixture_space(32, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let relation = space.relation();
    // Bind `?distance` to a value no row carries: the cursor filters every candidate and
    // answers empty, which is a filtered offer rather than a different relation.
    let seed = row_of(&space, 5);
    let count = TermValue::typed_literal("3", XSD_INTEGER);
    let distance =
        TermValue::typed_literal("123456.789", "http://www.w3.org/2001/XMLSchema#double");
    let subject = [None];
    let object = [Some(&seed), Some(&count), Some(&distance)];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None).expect("opens");
    let mut rows = 0;
    while cursor.next().expect("no error").is_some() {
        rows += 1;
    }
    assert_eq!(rows, 0);
    assert!(cursor.take_work() > 0, "the candidates were still examined");
}

#[test]
fn the_declared_mode_is_the_seed_and_count_contract() {
    let (_, space) = fixture_space(8, 4, Params::new(4, 8, 16, 8).expect("valid"));
    let relation = HnswRelation::new(Arc::clone(&space));
    assert_eq!(relation.arity().subject, 1);
    assert_eq!(relation.arity().object, 3);
    let mode = BindingPattern::from_code("fbbf");
    assert!(relation.admits(mode));
    assert_eq!(relation.rows_per_invocation(mode), 8);
}
