// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use purrdf_core::distance::{Arithmetic, BuildShape, Path, Reassociated};
use purrdf_core::{EmbeddingView, IndexUseRole, verify_embedding};
use purrdf_hnsw::{HnswError, HnswIndex, Params, guard, profile, relation::HnswSpace};
use purrdf_sparql_eval::{Completeness, KnnGuard, OrderFidelity, PropertyFunction};

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("valid")
}

/// The refusal `guard::validate_guard` raises for a foreign evidence revision, verbatim.
///
/// Asserted as a whole sentence rather than matched by error variant alone: `GuardProfile`
/// carries a dozen different refusals, and a check that started rejecting these artifacts
/// for an unrelated reason would still be a `GuardProfile`.
const FOREIGN_REVISION_REFUSAL: &str = "the implementation evidence revision is not the profile's approximation statement; an \
     HNSW guard must publish what it does not promise";

/// The refusal `guard::validate_guard` raises for a loss contract that transforms vectors.
const TRANSFORMING_LOSS_REFUSAL: &str = "the loss contract claims an HNSW payload transforms vectors; this profile stores no \
     vectors, so the claim is false";

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
    assert_eq!(space.evidence(), profile::LOSS_EVIDENCE);

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

#[test]
fn the_declared_use_role_is_read_off_the_projection() {
    // PURREMB defines role 2 as "coarse retrieval over a shorter effective prefix". An index
    // over a truncated projection IS that, whatever the profile would prefer to call itself,
    // and declaring a general-purpose role over a prefix understates the artifact's loss to
    // every consumer that reads the role to decide whether a rerank is owed.
    let full = purremb::Fixture::with_prefix(48, 8, 8, params());
    assert_eq!(
        full.use_role,
        IndexUseRole::Generic,
        "an index over the whole stored width is general purpose"
    );

    let coarse = purremb::Fixture::with_prefix(48, 8, 4, params());
    assert_eq!(
        coarse.use_role,
        IndexUseRole::CoarsePrefixRetrieval,
        "an index over half the stored width is coarse-prefix retrieval"
    );

    // Both artifacts must still verify, select and load: the role is a description, not a
    // restriction, and a prefix artifact is an ordinary artifact.
    for fixture in [&full, &coarse] {
        let mut view = EmbeddingView::from_bytes(&fixture.bytes).expect("the artifact opens");
        verify_embedding(&mut view).expect("the artifact verifies");
        let selected = guard::select(&view).expect("exactly one HNSW guard");
        guard::validate_guard(&selected).expect("the profile validates");
        let effective = view
            .effective_matrix(fixture.target_set, fixture.vector_space)
            .expect("readable")
            .expect("present");
        guard::check_coordinates(
            &selected,
            fixture.target_set,
            fixture.vector_space,
            &effective,
        )
        .expect("the coordinates, prefix and role all agree with the artifact");
    }
}

#[test]
fn a_search_over_a_coarse_prefix_still_answers() {
    // The role says the answer was decided on a truncation. It does not say the index stops
    // working, and an artifact that declared the honest role but could no longer be searched
    // would have traded one false statement for a worse one.
    let fixture = purremb::Fixture::with_prefix(48, 8, 4, params());
    let guard_value = KnnGuard::new(48, 48).expect("valid");
    let space = HnswSpace::from_artifact(
        &fixture.bytes,
        fixture.target_set,
        fixture.vector_space,
        fixture.bindings(),
        guard_value,
    )
    .expect("a coarse-prefix artifact yields a space");
    assert_eq!(
        space.dimension(),
        4,
        "the space searches the declared prefix, not the stored width"
    );
}

// --- What `HnswRelation::fidelity` is allowed to assume --------------------
//
// `HnswRelation::fidelity` reports `profile::LOSS_EVIDENCE` as its completeness evidence and
// computes its order axis from this build's compiled-in `profile::loss_contract()` -- neither
// is decoded out of the artifact the space was bound from. That is equivalent to reading the
// artifact only because the two refusals below fire: a guard publishing a different evidence
// revision, or a different loss contract, never becomes a space at all, so an artifact this
// space could have been built from cannot disagree with the constants. Loosening either check
// loosens what `HnswRelation::fidelity` may assume, and this is where that shows up.

#[test]
fn a_foreign_evidence_revision_is_refused_at_bind_time() {
    let fixture = purremb::Fixture::new(24, 4, params());
    let foreign = fixture.foreign_evidence_revision();

    // The container is intact: real builder, real sections, a guard that still names this
    // profile. The only thing wrong with the artifact is the sentence it publishes about
    // what it does not promise.
    let mut view = EmbeddingView::from_bytes(&foreign).expect("the artifact opens");
    verify_embedding(&mut view).expect("the artifact verifies");
    let selected = guard::select(&view).expect("the guard still names the profile");
    let error = guard::validate_guard(&selected).expect_err("a foreign revision must be refused");
    assert!(
        matches!(&error, HnswError::GuardProfile { description } if description == FOREIGN_REVISION_REFUSAL),
        "the refusal must name the evidence revision, got {error}"
    );

    // And the outermost entry point a host actually calls refuses for the same reason, not
    // merely somewhere deeper for an incidental one.
    let refusal = HnswSpace::from_artifact(
        &foreign,
        fixture.target_set,
        fixture.vector_space,
        fixture.bindings(),
        KnnGuard::new(24, 24).expect("valid"),
    )
    .expect_err("no space binds over a foreign evidence revision");
    assert!(
        refusal.to_string().contains(FOREIGN_REVISION_REFUSAL),
        "the host-facing refusal must name the evidence revision, got {refusal}"
    );
}

#[test]
fn a_guard_claiming_transformed_vectors_is_refused_at_bind_time() {
    // PURREMB writes the loss contract's `approximate` field itself and always writes it
    // true, so no encoder can produce a non-approximate HNSW guard; `transforms_vectors` is
    // the field a producer chooses, and it is the one a consumer's order axis turns on.
    let fixture = purremb::Fixture::new(24, 4, params());
    let transforming = fixture.transforming_loss_contract();

    let mut view = EmbeddingView::from_bytes(&transforming).expect("the artifact opens");
    verify_embedding(&mut view).expect("the artifact verifies");
    let selected = guard::select(&view).expect("the guard still names the profile");
    let error =
        guard::validate_guard(&selected).expect_err("a transforming loss contract is refused");
    assert!(
        matches!(&error, HnswError::GuardProfile { description } if description == TRANSFORMING_LOSS_REFUSAL),
        "the refusal must name the loss contract, got {error}"
    );

    let refusal = HnswSpace::from_artifact(
        &transforming,
        fixture.target_set,
        fixture.vector_space,
        fixture.bindings(),
        KnnGuard::new(24, 24).expect("valid"),
    )
    .expect_err("no space binds over a transforming loss contract");
    assert!(
        refusal.to_string().contains(TRANSFORMING_LOSS_REFUSAL),
        "the host-facing refusal must name the loss contract, got {refusal}"
    );
}

#[test]
fn the_profiles_own_evidence_and_loss_contract_still_bind_and_are_what_fidelity_reports() {
    // The neighbouring valid case for BOTH refusals above, built by the same fixture that
    // produced the two tampered artifacts and differing from each in exactly one guard field.
    // A guard check that had tightened into rejecting every artifact would satisfy the two
    // refusal tests and fail here, which is the only reason those tests mean anything.
    let fixture = purremb::Fixture::new(24, 4, params());
    let space = std::sync::Arc::new(
        HnswSpace::from_artifact(
            &fixture.bytes,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(24, 24).expect("valid"),
        )
        .expect("the untampered artifact binds"),
    );

    // A host that handed the build the vectors it meant, unquantized, has
    // nothing to disclose on the order axis and says so. What it says there
    // cannot reach the completeness axis, which is what the assertion below
    // rests on.
    let fidelity = space.relation().fidelity(OrderFidelity::Faithful);
    let Completeness::Lossy { evidence } = &fidelity.completeness else {
        panic!("an HNSW search offers candidates and never certifies absence");
    };
    assert_eq!(
        &**evidence,
        profile::LOSS_EVIDENCE,
        "the evidence a bound space reports is the profile's own sentence, byte for byte"
    );
    assert_eq!(
        fidelity.order,
        OrderFidelity::Faithful,
        "a graph over untransformed vectors compares exact distances for every candidate it \
         visits, so the rows it does return are in true relative order"
    );
}

/// The guard's profile, read back from `artifact`, or its refusal.
fn validated(artifact: &[u8]) -> purrdf_hnsw::Result<Params> {
    let mut view = EmbeddingView::from_bytes(artifact).expect("the artifact opens");
    verify_embedding(&mut view).expect("the artifact verifies");
    let selected = guard::select(&view).expect("the guard names the profile");
    guard::validate_guard(&selected)
}

/// The description of a `GuardProfile` refusal, or a panic naming what was raised instead.
fn profile_refusal(error: HnswError) -> String {
    match error {
        HnswError::GuardProfile { description } => description,
        other => panic!("expected a profile refusal, got {other}"),
    }
}

#[test]
fn guard_refuses_cross_paired_profile() {
    let exact = purremb::Fixture::new(24, 4, params());
    let fast = purremb::Fixture::new_reassociated(24, 4, params());
    let path = Reassociated::resolve()
        .expect("the test thread runs the default float environment")
        .path();

    // The two legal pairings: each implementation with its own arithmetic's evidence, and
    // a payload recording a code that evidence names. Both validate and both load.
    assert_eq!(validated(&exact.bytes).expect("validates"), params());
    assert_eq!(validated(&fast.bytes).expect("validates"), params());
    assert_eq!(
        fast.guard_contract.implementation.identifier,
        profile::IMPLEMENTATION_ID_REASSOCIATED
    );
    assert!(
        HnswSpace::from_artifact(
            &exact.bytes,
            exact.target_set,
            exact.vector_space,
            exact.bindings(),
            KnnGuard::new(24, 24).expect("valid"),
        )
        .is_ok()
    );
    let space = HnswSpace::from_artifact_reassociated(
        &fast.bytes,
        fast.target_set,
        fast.vector_space,
        fast.bindings(),
        KnnGuard::new(24, 24).expect("valid"),
    )
    .expect("the reassociated space binds");
    assert_eq!(space.evidence(), profile::loss_evidence_reassociated(path));

    // Illegal: the exact implementation carrying the reassociated evidence.
    let mut crossed = profile::implementation();
    crossed.revision = Some(profile::loss_evidence_reassociated(path).into_bytes());
    let refusal = profile_refusal(
        validated(&exact.with_implementation(crossed)).expect_err("a cross pairing is refused"),
    );
    assert!(
        refusal.contains("`hnsw-v2` carries the evidence revision `hnsw-reassociated-v2`"),
        "{refusal}"
    );

    // Illegal: the reassociated implementation carrying the exact evidence.
    let mut crossed = profile::implementation_for::<Reassociated>(path);
    crossed.revision = Some(profile::LOSS_EVIDENCE.as_bytes().to_vec());
    let refusal = profile_refusal(
        validated(&fast.with_implementation(crossed)).expect_err("a cross pairing is refused"),
    );
    assert!(
        refusal.contains("`hnsw-reassociated-v2` carries the evidence revision `hnsw-v2`"),
        "{refusal}"
    );

    // Illegal: a legal reassociated identity for ANOTHER path over this path's payload. The
    // guard alone is a published row, so it validates; the pairing of that row with the
    // payload's recorded code is what the load refuses.
    let other = [
        Path::Portable,
        Path::Sse2,
        Path::Avx2Fma,
        Path::Avx512f,
        Path::Neon,
        Path::WasmSimd128,
    ]
    .into_iter()
    .find(|candidate| *candidate != path)
    .expect("another reassociated path exists");
    let elsewhere = fast.with_implementation(profile::implementation_for::<Reassociated>(other));
    assert_eq!(
        validated(&elsewhere).expect("the row alone is legal"),
        params()
    );
    let mut view = EmbeddingView::from_bytes(&elsewhere).expect("the artifact opens");
    verify_embedding(&mut view).expect("the artifact verifies");
    let selected = guard::select(&view).expect("the guard names the profile");
    let refusal = profile_refusal(
        guard::load_reassociated(&selected, fast.matrix.clone())
            .expect_err("a guard naming another path than its payload is refused"),
    );
    assert!(refusal.contains("two different compilations"), "{refusal}");

    // And an implementation loaded as the other arithmetic is refused at the guard, before
    // any payload is decoded.
    let mut view = EmbeddingView::from_bytes(&fast.bytes).expect("the artifact opens");
    verify_embedding(&mut view).expect("the artifact verifies");
    let selected = guard::select(&view).expect("the guard names the profile");
    let refusal = profile_refusal(
        guard::load(&selected, fast.matrix.clone()).expect_err("loaded as the wrong law"),
    );
    assert!(refusal.contains("hnsw-reassociated-v2"), "{refusal}");
}

/// A reassociated payload recorded by a build of another shape is refused by name at the
/// guard, both by the load and by the rebuild verification; the payload commitment is
/// the artifact's own, so the refusal is the shape check and not a failed checksum. The
/// neighbour is the same artifact with the payload this build wrote.
#[test]
fn a_foreign_build_shape_is_refused_at_the_guard() {
    let fast = purremb::Fixture::new_reassociated(24, 4, params());
    let here = BuildShape::here();
    // The shape follows the code in a reassociated header.
    assert_eq!(fast.image[64..72], here.bits().to_le_bytes());
    let foreign = BuildShape::from_bits(here.bits() ^ 1 << 47);
    assert_ne!(foreign, here);
    let mut patched = fast.image.clone();
    patched[64..72].copy_from_slice(&foreign.bits().to_le_bytes());
    let refusal = HnswError::ArithmeticBuildMismatch {
        recorded: foreign,
        here,
    };

    for (artifact, refused) in [
        (fast.with_payload(fast.image.clone()), false),
        (fast.with_payload(patched), true),
    ] {
        let mut view = EmbeddingView::from_bytes(&artifact).expect("the artifact opens");
        verify_embedding(&mut view).expect("the payload commitment is the artifact's own");
        let selected = guard::select(&view).expect("the guard names the profile");
        guard::verify_payload_commitment(
            &selected,
            guard::payload_bytes(&selected).expect("inline"),
        )
        .expect("the guard commits these bytes");
        let loaded = guard::load_reassociated(&selected, fast.matrix.clone());
        let verified = guard::verify_rebuild(&selected, &fast.matrix, &fast.params);
        if refused {
            assert_eq!(loaded.expect_err("a foreign build is refused"), refusal);
            assert_eq!(
                verified.expect_err("refused by name, never `Ok(false)`"),
                refusal
            );
        } else {
            let index = loaded.expect("this build's payload loads");
            assert_eq!(index.canonical_image(), fast.image);
            assert!(verified.expect("this build's payload verifies"));
        }
    }
}
