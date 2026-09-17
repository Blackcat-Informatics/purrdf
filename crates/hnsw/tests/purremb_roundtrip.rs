// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The HNSW guard adapter round-trips through the **real** PURREMB sections.
//!
//! Nothing here is a mock: each test builds a certified `.purrpck` source, a typed
//! embedding family and matrix, an HNSW index folded through `guard::derived_index`, and a
//! second artifact that carries the guard and payload in the genuine `INDEX_GUARDS` and
//! `INDEX_PAYLOAD` sections. It then reopens the bytes, verifies the container, selects the
//! one HNSW guard, reloads the payload from `IndexGuardView::payload_bytes`, and searches.
//!
//! The negative cases are as important as the positive one: a payload whose bytes were
//! substituted after the guard was sealed, and an artifact with zero or two HNSW guards,
//! are all construction-time failures, not searches that return a wrong or arbitrary
//! answer.

#[path = "support/purremb.rs"]
mod purremb;

use purrdf_core::{EmbeddingView, verify_embedding};
use purrdf_hnsw::{HnswError, HnswIndex, Params, guard, relation::HnswSpace};
use purrdf_sparql_eval::{KnnGuard, PropertyFunction};

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("valid")
}

#[test]
fn the_payload_round_trips_through_the_real_sections() {
    let fixture = purremb::Fixture::new(48, 8, params());

    let mut view = EmbeddingView::from_bytes(&fixture.bytes).expect("the artifact opens");
    verify_embedding(&mut view).expect("the artifact verifies");

    let selected = guard::select(&view).expect("exactly one HNSW guard");
    let declared = guard::validate_guard(&selected).expect("the profile validates");
    assert_eq!(
        declared, fixture.params,
        "the guard names the built parameters"
    );

    let payload = guard::payload_bytes(&selected).expect("inline bytes");
    guard::verify_payload_commitment(&selected, payload).expect("the commitment holds");
    assert_eq!(
        payload, fixture.image,
        "the section carries the canonical image"
    );

    let index = guard::load(&selected, fixture.matrix.clone()).expect("the payload decodes");
    assert_eq!(index.canonical_image(), fixture.image);
    assert!(guard::verify_rebuild(&selected, &fixture.matrix, &fixture.params).expect("rebuilds"));

    // Reload directly from the borrowed view bytes and prove the search is the same.
    let direct = HnswIndex::decode(
        fixture.matrix.clone(),
        selected.payload_bytes().expect("bytes"),
    )
    .expect("decodes from the view");
    assert_eq!(
        direct.search_rows(7, 3).expect("searches"),
        index.search_rows(7, 3).expect("searches"),
        "the reloaded index answers exactly as the built one"
    );
}

#[test]
fn a_reloaded_space_searches_over_the_target_row_order() {
    let fixture = purremb::Fixture::new(32, 4, params());
    let space = std::sync::Arc::new(
        HnswSpace::from_artifact(
            &fixture.bytes,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(32, 32).expect("valid"),
        )
        .expect("the space loads"),
    );

    assert_eq!(space.row_count(), 32);
    // The row index is the target-set row order: the term bound to row `r` is found at `r`.
    for row in 0..space.row_count() {
        let term = space.term(row).expect("a term").clone();
        assert_eq!(space.row_of(&term), Some(row));
    }
    assert_eq!(space.evidence(), purrdf_hnsw::profile::LOSS_EVIDENCE);

    let relation = space.relation();
    let seed = space.term(7).expect("a term").clone();
    let count =
        purrdf_core::TermValue::typed_literal("3", "http://www.w3.org/2001/XMLSchema#integer");
    let subject = [None];
    let object = [Some(&seed), Some(&count), None];
    let args = purrdf_sparql_eval::PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None).expect("opens");
    let mut rows = 0;
    while let Some(row) = cursor.next().expect("no error") {
        rows += 1;
        assert_eq!(row.len(), 4, "every row is the four-position shape");
        assert_eq!(row[1], seed, "the seed is echoed");
    }
    assert!(rows <= 3);
    assert!(cursor.take_work() > 0);
}

#[test]
fn a_substituted_payload_is_refused_before_any_search() {
    let fixture = purremb::Fixture::new(24, 4, params());
    let tampered = fixture.tampered_payload();

    let view = EmbeddingView::from_bytes(&tampered).expect("the framing still opens");
    let selected = guard::select(&view).expect("the guard still names the profile");
    let error = guard::load(&selected, fixture.matrix.clone())
        .expect_err("the substituted payload must not decode");
    assert!(
        matches!(error, HnswError::PayloadCommitment { .. }),
        "expected a commitment failure, got {error}"
    );
    assert!(
        !guard::verify_rebuild(&selected, &fixture.matrix, &fixture.params).expect("boolean"),
        "a substituted payload is not a rebuild"
    );

    // The whole construction refuses too, before any search can be issued.
    assert!(
        HnswSpace::from_artifact(
            &tampered,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(24, 24).expect("valid"),
        )
        .is_err()
    );
}

#[test]
fn zero_matching_guards_is_a_construction_failure() {
    let fixture = purremb::Fixture::new(8, 4, params());
    let view = EmbeddingView::from_bytes(&fixture.without_index).expect("the artifact opens");
    assert!(matches!(
        guard::select(&view),
        Err(HnswError::MissingIndexGuard { .. })
    ));
}

#[test]
fn multiple_matching_guards_is_a_construction_failure() {
    let fixture = purremb::Fixture::new(8, 4, params());
    let duplicated = fixture.with_second_index(Params::new(2, 2, 2, 1).expect("valid"));
    let view = EmbeddingView::from_bytes(&duplicated).expect("the artifact opens");
    assert!(matches!(
        guard::select(&view),
        Err(HnswError::AmbiguousIndexGuard { count: 2 })
    ));
}

#[test]
fn verify_rebuild_is_true_for_fresh_and_false_for_tampered() {
    let fixture = purremb::Fixture::new(24, 4, params());
    let view = EmbeddingView::from_bytes(&fixture.bytes).expect("opens");
    let selected = guard::select(&view).expect("selects");
    assert!(
        guard::verify_rebuild(&selected, &fixture.matrix, &fixture.params).expect("boolean"),
        "a freshly built artifact rebuilds exactly"
    );

    // A tampered payload is not a rebuild of the source matrix either.
    let tampered_bytes = fixture.tampered_payload();
    let tampered = EmbeddingView::from_bytes(&tampered_bytes).expect("opens");
    let tampered_guard = guard::select(&tampered).expect("selects");
    assert!(
        !guard::verify_rebuild(&tampered_guard, &fixture.matrix, &fixture.params).expect("boolean"),
        "a substituted payload is never the rebuild"
    );

    // A different parameter set does not describe this payload.
    assert!(
        !guard::verify_rebuild(
            &selected,
            &fixture.matrix,
            &Params::new(8, 16, 32, 8).expect("valid")
        )
        .expect("boolean"),
        "the same bytes under other parameters are not the rebuild"
    );
}
