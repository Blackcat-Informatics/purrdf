// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The membership question an approximate index really can answer, and the two facts
//! that decide what it may declare about it.
//!
//! This relation's completeness axis is [`Completeness::Lossy`] on every request, over
//! every space, unconditionally — a beam offers the candidates it reached and never
//! certifies that nothing else matched. That is exactly the producer a *search*-based
//! exclusion basis must be refused from, because "my beam did not find it" is not "I do
//! not hold it".
//!
//! It is also exactly the producer a *membership* basis must be admitted from. A term the
//! vector matrix holds no row for is a term no traversal can reach — at any `ef`, in any
//! layer, from any entry point — because every row a beam can name is a row of the matrix.
//! The verdict is a fact about the matrix, not about what the search reached, so it is
//! exact however lossy the search is, and refusing it on the completeness axis would throw
//! away a provably exact answer. Both directions are asserted here.
//!
//! The relation declares that basis, and the other fact this file pins is what makes the
//! declaration honest: the membership question is a **different access pattern** — the
//! count left free — and an exclusion lookup arrives in it rather than in the ranked one.
//! A lookup admitted with the count bound would be answered by `is this candidate among
//! your best n`, whose absences are not exclusions, so the two patterns are kept apart
//! here: the count-bound call still cuts at `k`, the count-free call is the point lookup,
//! and neither is a second reading of the other.
//!
//! Fixtures use `example.org` throughout; every IRI below is fixture configuration, never
//! a minted vocabulary.

use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{DistanceMetric, TermValue};
use purrdf_hnsw::relation::{HnswObservations, HnswRelation, HnswSpace};
use purrdf_hnsw::{HnswIndex, Params, VectorMatrix, level::splitmix64};
use purrdf_sparql_eval::{
    CandidateDomains, Completeness, ExclusionBasis, KnnGuard, OrderFidelity, PfArgs, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankedDeclaration, TermKind,
};

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const PREDICATE: &str = "https://example.org/pf/nearest";
const STRATUM: &str = "https://example.org/stratum/vector";

/// How many rows the fixture space holds.
const ROWS: usize = 32;

/// How many terms beyond those rows the fixture's universe names.
///
/// The space is built over the first [`ROWS`] of them, so the walk below has a genuinely
/// non-empty answer in **both** directions — a term with a row and a term without — and
/// neither half of the agreement is vacuous.
const STRANGERS: usize = 24;

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("the fixture parameters are valid")
}

/// A deterministic fixture matrix. Nothing here reads a clock or an RNG.
fn matrix(rows: usize, dims: usize) -> VectorMatrix {
    let mut state = 0x51DE_0000_1234_ABCD_u64;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        state = splitmix64(state);
        let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
        let value = unit.mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.25 } else { value });
    }
    VectorMatrix::new(rows, dims, data).expect("the fixture matrix is valid")
}

/// The fixture's whole term universe: the terms the space holds, then the strangers.
fn universe() -> Vec<TermValue> {
    (0..ROWS + STRANGERS)
        .map(|at| TermValue::iri(format!("https://example.org/doc/{at}")))
        .collect()
}

/// A space over the first [`ROWS`] terms of the universe, at beam width `ef_search`.
fn space_at(ef_search: usize) -> (VectorMatrix, Arc<HnswSpace>) {
    let vectors = matrix(ROWS, 4);
    let index = HnswIndex::build(
        vectors.clone(),
        &DistanceMetric::SquaredEuclidean,
        Params::new(4, 8, 16, ef_search).expect("the fixture beam is valid"),
    )
    .expect("the fixture graph builds");
    let terms: Vec<TermValue> = universe().into_iter().take(ROWS).collect();
    let guard = KnnGuard::new(ROWS as u64, ROWS as u64).expect("the fixture guard is valid");
    (
        vectors,
        Arc::new(HnswSpace::from_index(index, terms, guard).expect("the fixture space is valid")),
    )
}

fn space() -> Arc<HnswSpace> {
    space_at(params().ef_search()).1
}

fn stratum() -> purrdf_core::Iri {
    purrdf_core::parse_iri(STRATUM).expect("the fixture stratum IRI is valid")
}

/// The relation's own declaration, with nothing edited.
fn declaration(relation: &HnswRelation) -> RankedDeclaration {
    relation.ranked_declaration(
        stratum(),
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        OrderFidelity::Faithful,
        CandidateDomains::Unrestricted,
    )
}

/// Open the relation and drain it, returning the rows and the work reported.
///
/// `k` is an `Option` because the count is what decides which of the relation's two
/// questions an invocation asks: `Some` is the ranked read, `None` is the membership
/// lookup. Every call site below says which one it means.
fn drain(
    relation: &HnswRelation,
    seed: Option<&TermValue>,
    k: Option<&str>,
    neighbour: Option<&TermValue>,
) -> (Vec<PfRow>, u64) {
    let count = k.map(|k| TermValue::typed_literal(k, XSD_INTEGER));
    let subject = [neighbour];
    let object = [seed, count.as_ref(), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None).expect("the invocation opens");
    let mut rows = Vec::new();
    while let Some(row) = cursor.next().expect("the invocation yields") {
        rows.push(row);
    }
    (rows, cursor.take_work())
}

/// The four counters, read together so a failure prints the whole row of the table.
fn report(observed: &HnswObservations) -> String {
    format!(
        "membership_lookups={} membership_distances={} searches={} graph_candidates={}",
        observed.membership_lookups(),
        observed.membership_distances(),
        observed.searches(),
        observed.graph_candidates(),
    )
}

// ---------------------------------------------------------------------------
// What this producer may and may not declare
// ---------------------------------------------------------------------------

/// **A lossy producer is admitted a `Membership` basis, and declares exactly that.**
///
/// The registration is executed, because the interesting fact is that it succeeds.
/// Keying a refusal on the completeness axis would reject it, and that would reject a
/// provably exact answer: a term the matrix holds no row for is a term no beam reaches at
/// any `ef`. The registry does not make that mistake.
///
/// The relation's own declaration is read off the declaration rather than assumed, and it
/// is `Membership` — the same verdict the registry gives, arrived at independently. The
/// two are asserted side by side because they are two facts, not one: what a producer may
/// declare and what it does declare are different questions, and a test that checked only
/// the registry would pass over a relation that declared nothing at all.
#[test]
fn a_membership_basis_is_declared_and_admitted_from_a_lossy_producer() {
    let relation = HnswRelation::new(space());
    let declared = declaration(&relation);

    // The premise, read off the declaration rather than assumed: this producer declares
    // loss on the completeness axis, and it does so unconditionally.
    assert!(
        matches!(declared.fidelity.completeness, Completeness::Lossy { .. }),
        "an HNSW search offers candidates and never certifies absence: {:?}",
        declared.fidelity.completeness
    );

    // What it actually declares, and why — see this test's own docs.
    assert_eq!(
        declared.exclusion,
        ExclusionBasis::Membership,
        "an absence in this producer's term universe is exact however lossy its beam is, \
         and a lookup reaches the mode that answers it"
    );

    // The valid neighbour, executed: the lossy producer's OWN membership basis is what
    // the registry admits. This is the case a completeness-keyed refusal would have
    // rejected, and the declaration is the relation's rather than one this test wrote.
    let membership = declaration(&relation);
    assert_eq!(membership.exclusion, ExclusionBasis::Membership);
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(PREDICATE, Arc::new(HnswRelation::new(space())), membership);
    assert!(registry.resolve(PREDICATE).is_some());
    assert_eq!(
        registry
            .ranked_declaration(PREDICATE)
            .expect("the producer registered as a ranked one")
            .exclusion,
        ExclusionBasis::Membership,
        "the registry admits membership from a producer that is lossy on every request, \
         which is the refusal that must NOT exist"
    );
}

/// **The membership mode is a new point of the lattice, and it takes nothing from the
/// one beside it.**
///
/// The two facts have to be asserted together, because each without the other is the
/// defect this shape exists to prevent. If `fbbf` subsumed the membership pattern, the
/// membership pattern would be a second reading of a call that already had one, and
/// declaring it would silently change what that call answers. If `fbbf` did *not* still
/// subsume the count-bound candidate call, the ranked question would have moved.
#[test]
fn the_membership_mode_is_new_and_the_ranked_one_is_untouched() {
    let relation = HnswRelation::new(space());
    let general = BindingPattern::from_code("fbbf");
    let membership = BindingPattern::from_code("bbff");
    let candidate_and_count = BindingPattern::from_code("bbbf");

    let declared: Vec<String> = relation.modes().iter().map(|mode| mode.code()).collect();
    assert_eq!(declared, vec!["fbbf".to_owned(), "bbff".to_owned()]);

    // Subsumption is `bound(declared) ⊆ bound(invocation)`. `fbbf` binds position 2, the
    // count, which the membership pattern leaves free — so it does NOT subsume it, and
    // the membership pattern was infeasible until it was declared. That is what makes
    // the two genuinely different questions rather than two readings of one.
    assert!(
        !general.subsumes(membership),
        "`fbbf` binds the count that `bbff` leaves free, so it cannot subsume it; if it \
         did, declaring `bbff` would be re-reading a call that already had a meaning"
    );

    // And the call the ranked question is asked by is still subsumed by `fbbf` — it was
    // feasible before and is feasible now, through the same mode, with the same meaning.
    assert!(
        general.subsumes(candidate_and_count),
        "a bound candidate beside a bound count is the ranked question and always was"
    );

    // The bound the registry reads beside the membership mode is the point bound it
    // really is, and the ranked mode's is not a point bound at all.
    assert_eq!(relation.rows_per_invocation(membership), 1);
    assert!(
        relation.rows_per_invocation(general) > 1,
        "over this fixture"
    );
}

/// **The ranked question still cuts at `k`, and the membership question still does not
/// exist without one.**
///
/// The control this whole change is measured against. A term the space holds but the
/// offer of one leaves out must still be an empty answer, or a query nobody edited
/// started returning different rows.
#[test]
fn a_bound_count_keeps_its_cut_and_a_free_one_without_a_candidate_is_refused() {
    let space = space();
    let relation = HnswRelation::new(Arc::clone(&space));
    let seed = space.term(0).expect("the fixture holds row zero").clone();

    // The offer of one names one row. Every other held term is outside it.
    let (offer, _) = drain(&relation, Some(&seed), Some("1"), None);
    assert_eq!(offer.len(), 1);
    let named = offer[0][HnswRelation::NEIGHBOUR].clone();
    let outside = (0..space.row_count())
        .map(|row| space.term(row).expect("a row of the space").clone())
        .find(|term| *term != named)
        .expect("the fixture holds more than one row");

    // Bound count: the cut applies, and the held-but-unoffered term is an empty answer.
    let (cut, _) = drain(&relation, Some(&seed), Some("1"), Some(&outside));
    assert!(
        cut.is_empty(),
        "a bound count asks `is this term among the k you offer`, and this term is not"
    );
    // The neighbouring case that must still succeed: the term the offer DID name.
    let (kept, _) = drain(&relation, Some(&seed), Some("1"), Some(&named));
    assert_eq!(kept.len(), 1, "and the term it did offer is still named");

    // Free count: the membership question, and the same held-but-unoffered term is now
    // named — because a different question was asked, through a different mode.
    let (held, _) = drain(&relation, Some(&seed), None, Some(&outside));
    assert_eq!(held.len(), 1);
    assert_eq!(held[0][HnswRelation::NEIGHBOUR], outside);

    // A free count with no candidate either is no question at all, and is refused rather
    // than answered with some invented depth.
    let subject = [None];
    let object = [Some(&seed), None, None];
    let args = PfArgs::new(&subject, &object);
    assert!(
        relation.open(&args, None).is_err(),
        "how many neighbours to retrieve is a question this relation is asked, not one it \
         answers"
    );
}

// ---------------------------------------------------------------------------
// The walk: every term, both directions
// ---------------------------------------------------------------------------

#[test]
fn every_term_of_the_universe_agrees_with_its_row() {
    let space = space();
    let relation = HnswRelation::new(Arc::clone(&space));
    let seed = space
        .term(0)
        .expect("the fixture space holds row zero")
        .clone();

    let mut present = 0_u64;
    let mut absent = 0_u64;
    for term in universe() {
        let (rows, _) = drain(&relation, Some(&seed), None, Some(&term));
        match space.row_of(&term) {
            Some(_) => {
                present += 1;
                assert_eq!(
                    rows.len(),
                    1,
                    "{term:?} has a row, so the producer may still name it and the lookup \
                     must say so"
                );
                assert_eq!(rows[0][HnswRelation::NEIGHBOUR], term);
            }
            None => {
                absent += 1;
                assert!(
                    rows.is_empty(),
                    "{term:?} has no row, so this producer names it at no rank"
                );
            }
        }
    }

    // Neither direction is vacuous: the walk really saw both answers.
    assert_eq!(present, ROWS as u64);
    assert_eq!(absent, STRANGERS as u64);
    assert!(present > 0 && absent > 0);
}

// ---------------------------------------------------------------------------
// The oracle: the verdict is exact, not lucky
// ---------------------------------------------------------------------------

#[test]
fn an_excluded_term_is_named_by_no_beam_at_any_width_and_a_held_one_is() {
    // The treatment: a term the matrix holds no row for. The lookup excludes it, and no
    // search at any declared beam width, from any seed, ever names it.
    let stranger = universe()
        .into_iter()
        .nth(ROWS)
        .expect("the universe names more terms than the space holds");

    // The control: a term the matrix DOES hold. It must be named by some search, or the
    // walk below proves only that the walk names nothing.
    let held = TermValue::iri("https://example.org/doc/0");

    let mut named_the_held_one = 0_u64;
    for ef_search in [1_usize, 2, 4, 8, 16, 32] {
        let (_, space) = space_at(ef_search);
        let relation = HnswRelation::new(Arc::clone(&space));
        assert!(space.row_of(&stranger).is_none());
        assert!(space.row_of(&held).is_some());

        // The lookup's verdict, at this width: excluded, whatever the beam reached.
        let (excluded, _) = drain(&relation, Some(&held), None, Some(&stranger));
        assert!(excluded.is_empty(), "ef_search = {ef_search}");

        // The observing oracle: every seed, the deepest read the guard admits, at this
        // width. If any of them named the stranger the verdict above would have been a
        // false certainty rather than an exact one.
        for row in 0..space.row_count() {
            let seed = space.term(row).expect("a row of the space").clone();
            let (rows, _) = drain(&relation, Some(&seed), Some(&ROWS.to_string()), None);
            for emitted in &rows {
                assert_ne!(
                    emitted[HnswRelation::NEIGHBOUR],
                    stranger,
                    "ef_search = {ef_search}, seed row {row}: a term with no row was named"
                );
                if emitted[HnswRelation::NEIGHBOUR] == held {
                    named_the_held_one += 1;
                }
            }
        }
    }
    assert!(
        named_the_held_one > 0,
        "the control must be named by some search, or the walk above is a walk over \
         answers that name nothing"
    );
}

// ---------------------------------------------------------------------------
// The cost: no distance, no traversal
// ---------------------------------------------------------------------------

#[test]
fn a_membership_lookup_computes_no_distance_and_visits_no_graph_node() {
    let space = space();
    let relation = HnswRelation::new(Arc::clone(&space));
    let observed = relation.observations();
    let seed = space.term(0).expect("the fixture holds row zero").clone();
    let stranger = universe()
        .into_iter()
        .nth(ROWS)
        .expect("the universe is wider than the space");

    // 1. The excluded candidate: one lookup, and NOTHING else. Not a distance, not a
    //    node, not a beam. A zero here is read off a counter rather than inferred from a
    //    timing, which a fixture this small could never distinguish.
    let (rows, work) = drain(&relation, Some(&seed), None, Some(&stranger));
    assert_eq!(rows, Vec::<PfRow>::new());
    assert_eq!(
        work, 0,
        "an excluded candidate examined no candidate at all"
    );
    assert_eq!(observed.membership_lookups(), 1, "{}", report(&observed));
    assert_eq!(observed.membership_distances(), 0, "{}", report(&observed));
    assert_eq!(observed.searches(), 0, "{}", report(&observed));
    assert_eq!(observed.graph_candidates(), 0, "{}", report(&observed));

    // 2. The held candidate: one more lookup, one pairwise distance for the `?distance`
    //    the emitted row carries, and STILL no traversal — over a space of thirty-two
    //    rows and a beam that never ran, which is the whole claim.
    let held = space.term(7).expect("the fixture holds row seven").clone();
    let (rows, work) = drain(&relation, Some(&seed), None, Some(&held));
    assert_eq!(rows.len(), 1);
    assert_eq!(work, 1, "one pair examined, charged once");
    assert_eq!(observed.membership_lookups(), 2, "{}", report(&observed));
    assert_eq!(observed.membership_distances(), 1, "{}", report(&observed));
    assert_eq!(
        observed.searches(),
        0,
        "a call that names its own candidate ranks nothing — {}",
        report(&observed)
    );
    assert_eq!(observed.graph_candidates(), 0, "{}", report(&observed));

    // 3. The control, so the zeroes above are not the zeroes of a relation that never
    //    traverses anything. The same relation, the same seed, a bound count and the
    //    candidate left free: the beam runs and the counters move.
    let (rows, work) = drain(&relation, Some(&seed), Some("8"), None);
    assert_ne!(rows, Vec::<PfRow>::new());
    assert!(work > 0);
    assert_eq!(observed.searches(), 1, "{}", report(&observed));
    assert!(
        observed.graph_candidates() > 0,
        "the ranked read really did visit nodes, so `visited zero` above is a measurement \
         and not a property of the fixture — {}",
        report(&observed)
    );
    assert_eq!(
        observed.membership_lookups(),
        2,
        "and the ranked read performed no lookup — {}",
        report(&observed)
    );
}

#[test]
fn the_distance_a_lookup_reports_is_the_one_the_beam_would_have() {
    // The lookup fills `?distance` from one pairwise evaluation rather than from a
    // search. That value must be the search's value, bit for bit, or two readings of the
    // same pair would disagree depending on which question was asked.
    let (_, space) = space_at(ROWS);
    let relation = HnswRelation::new(Arc::clone(&space));
    let seed = space.term(0).expect("the fixture holds row zero").clone();

    let (ranked, _) = drain(&relation, Some(&seed), Some(&ROWS.to_string()), None);
    assert!(ranked.len() > 1, "the fixture must rank more than one row");
    let mut compared = 0_u64;
    for row in &ranked {
        let neighbour = row[HnswRelation::NEIGHBOUR].clone();
        let (looked_up, _) = drain(&relation, Some(&seed), None, Some(&neighbour));
        assert_eq!(looked_up.len(), 1);
        assert_eq!(
            looked_up[0][HnswRelation::DISTANCE],
            row[HnswRelation::DISTANCE],
            "the lookup's distance for {neighbour:?} must be the search's own"
        );
        compared += 1;
    }
    assert_eq!(compared, ranked.len() as u64);
}
