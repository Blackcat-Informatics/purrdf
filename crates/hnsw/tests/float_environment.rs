// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A flushing float environment is refused by name at every entry point that computes a
//! distance, and the default environment is not.
//!
//! The exact arithmetic defines its bits under IEEE-754 round-to-nearest with subnormals
//! preserved. A thread with flush-to-zero set would build a different graph and rank by
//! different distances with nothing in the result to say so, so every entry point
//! resolves the arithmetic on its own thread first. This suite sets FTZ on the test
//! thread with `ldmxcsr`, calls each entry point, restores the register, and asserts the
//! named refusal; then it runs the same calls in the default environment and asserts
//! they answer. The register is per-thread, so no other test observes it.
//!
//! `x86_64` only: the refusal itself is unit-tested for `aarch64`'s FPCR in
//! `purrdf_core::distance`, and wasm has no such register.

#![cfg(target_arch = "x86_64")]

#[path = "support/purremb.rs"]
mod purremb;

use std::sync::Arc;

use purrdf_core::DistanceMetric;
use purrdf_core::distance::FloatEnvironmentError;
use purrdf_hnsw::{HnswError, HnswIndex, Params, guard, relation::HnswSpace};
use purrdf_sparql_eval::{
    EmbeddingKnnRelation, EmbeddingSpace, EvalError, KnnGuard, PfArgs, PropertyFunction,
};

/// MXCSR flush-to-zero.
const FTZ: u32 = 1 << 15;

fn read_mxcsr() -> u32 {
    let mut value: u32 = 0;
    // SAFETY: `stmxcsr` stores the 32-bit MXCSR to a live, aligned, writable `u32`.
    unsafe {
        core::arch::asm!(
            "stmxcsr [{ptr}]",
            ptr = in(reg) &raw mut value,
            options(nostack, preserves_flags),
        );
    }
    value
}

fn write_mxcsr(value: u32) {
    // SAFETY: `ldmxcsr` loads MXCSR from a live, aligned `u32`. Every value written here is
    // the saved register or the saved register with FTZ added, and `Flushed` restores the
    // saved one on every exit, including a panic.
    unsafe {
        core::arch::asm!(
            "ldmxcsr [{ptr}]",
            ptr = in(reg) &raw const value,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// FTZ set on this thread for as long as the guard lives.
struct Flushed(u32);

impl Flushed {
    fn new() -> Self {
        let saved = read_mxcsr();
        write_mxcsr(saved | FTZ);
        Self(saved)
    }
}

impl Drop for Flushed {
    fn drop(&mut self) {
        write_mxcsr(self.0);
    }
}

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("valid")
}

fn is_ftz(error: &HnswError) -> bool {
    matches!(
        error,
        HnswError::FloatEnvironment(FloatEnvironmentError::FlushToZero {
            register: "MXCSR",
            ..
        })
    )
}

fn is_ftz_eval(error: &EvalError) -> bool {
    matches!(
        error,
        EvalError::FloatEnvironment(FloatEnvironmentError::FlushToZero {
            register: "MXCSR",
            ..
        })
    )
}

/// Pull every row out of the kNN relation over `space` seeded at row 0.
fn knn_rows(
    relation: &EmbeddingKnnRelation,
    seed: &purrdf_core::TermValue,
) -> Result<usize, EvalError> {
    let count =
        purrdf_core::TermValue::typed_literal("3", "http://www.w3.org/2001/XMLSchema#integer");
    let subject = [None];
    let object = [Some(seed), Some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None)?;
    let mut rows = 0;
    while cursor.next()?.is_some() {
        rows += 1;
    }
    Ok(rows)
}

#[test]
fn every_distance_entry_point_refuses_a_flushing_environment_and_answers_the_default() {
    // Everything built before FTZ is set, in the default environment.
    let fixture = purremb::Fixture::new(40, 20, params());
    let matrix = fixture.matrix.clone();
    let index = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
        .expect("builds in the default environment");
    let image = index.canonical_image();
    let query = matrix.row_to_vec(3);
    let exact_space = Arc::new(
        EmbeddingSpace::from_artifact(
            &fixture.without_index,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(64, 8).expect("valid"),
        )
        .expect("the exact space opens in the default environment"),
    );
    let relation = EmbeddingKnnRelation::new(Arc::clone(&exact_space));
    let seed = fixture.terms[0].clone();
    let mut view = purrdf_core::EmbeddingView::from_bytes(&fixture.bytes).expect("opens");
    purrdf_core::verify_embedding(&mut view).expect("verifies");
    let selected = guard::select(&view).expect("one HNSW guard");

    {
        let flushed = Flushed::new();
        let build = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params());
        let decode = HnswIndex::decode(matrix.clone(), &image);
        let verify = index.verify_rebuild();
        let rows = index.search_rows(3, 4);
        let rows_work = index.search_rows_work(3, 4);
        let vector = index.search_vector(&query, 4);
        let vector_work = index.search_vector_work(&query, 4);
        let guard_verify = guard::verify_rebuild(&selected, &matrix, &params());
        let hnsw_space = HnswSpace::from_artifact(
            &fixture.bytes,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(64, 8).expect("valid"),
        );
        let exact_open = EmbeddingSpace::from_artifact(
            &fixture.without_index,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(64, 8).expect("valid"),
        );
        let exact_search = knn_rows(&relation, &seed);
        drop(flushed);

        assert!(is_ftz(&build.expect_err("build refuses")), "build");
        assert!(is_ftz(&decode.expect_err("decode refuses")), "decode");
        assert!(
            is_ftz(&verify.expect_err("rebuild refuses")),
            "verify_rebuild"
        );
        assert!(is_ftz(&rows.expect_err("search refuses")), "search_rows");
        assert!(
            is_ftz(&rows_work.expect_err("search refuses")),
            "search_rows_work"
        );
        assert!(
            is_ftz(&vector.expect_err("search refuses")),
            "search_vector"
        );
        assert!(
            is_ftz(&vector_work.expect_err("search refuses")),
            "search_vector_work"
        );
        assert!(
            is_ftz(&guard_verify.expect_err("refuses")),
            "guard::verify_rebuild"
        );
        assert!(
            is_ftz_eval(&hnsw_space.expect_err("refuses")),
            "HnswSpace::from_artifact keeps the named variant"
        );
        assert!(
            is_ftz_eval(&exact_open.expect_err("refuses")),
            "EmbeddingSpace::from_artifact"
        );
        assert!(
            is_ftz_eval(&exact_search.expect_err("refuses")),
            "the exact kNN search"
        );
    }

    // The valid neighbour: the same calls, on the same thread, after the register was
    // restored, all answer.
    assert!(HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params()).is_ok());
    let decoded = HnswIndex::decode(matrix.clone(), &image).expect("decodes");
    assert_eq!(decoded.canonical_image(), image);
    assert!(index.verify_rebuild().expect("rebuilds"));
    assert_eq!(index.search_rows(3, 4).expect("searches").len(), 4);
    assert_eq!(index.search_vector(&query, 4).expect("searches").len(), 4);
    assert!(guard::verify_rebuild(&selected, &matrix, &params()).expect("verifies"));
    assert!(
        HnswSpace::from_artifact(
            &fixture.bytes,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(64, 8).expect("valid"),
        )
        .is_ok()
    );
    assert_eq!(
        knn_rows(&relation, &seed).expect("the exact search answers"),
        3
    );
}

#[test]
fn every_reassociated_entry_point_refuses_a_flushing_environment_and_answers_the_default() {
    // The reassociated index hard-fails exactly as its exact sibling does: the same named
    // refusal from every entry point, and the same answers once the register is restored.
    let fixture = purremb::Fixture::new_reassociated(40, 20, params());
    let matrix = fixture.matrix.clone();
    let index =
        HnswIndex::build_reassociated(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds in the default environment");
    let image = index.canonical_image();
    let query = matrix.row_to_vec(3);
    let mut view = purrdf_core::EmbeddingView::from_bytes(&fixture.bytes).expect("opens");
    purrdf_core::verify_embedding(&mut view).expect("verifies");
    let selected = guard::select(&view).expect("one HNSW guard");
    let open = || {
        HnswSpace::from_artifact_reassociated(
            &fixture.bytes,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(64, 8).expect("valid"),
        )
    };

    {
        let flushed = Flushed::new();
        let build = HnswIndex::build_reassociated(
            matrix.clone(),
            &DistanceMetric::SquaredEuclidean,
            params(),
        );
        let decode = HnswIndex::decode_reassociated(matrix.clone(), &image);
        let verify = index.verify_rebuild();
        let rows = index.search_rows(3, 4);
        let vector = index.search_vector(&query, 4);
        let guard_verify = guard::verify_rebuild(&selected, &matrix, &params());
        let space = open();
        drop(flushed);

        assert!(is_ftz(&build.expect_err("build refuses")), "build");
        assert!(is_ftz(&decode.expect_err("decode refuses")), "decode");
        assert!(
            is_ftz(&verify.expect_err("rebuild refuses")),
            "verify_rebuild"
        );
        assert!(is_ftz(&rows.expect_err("search refuses")), "search_rows");
        assert!(
            is_ftz(&vector.expect_err("search refuses")),
            "search_vector"
        );
        assert!(
            is_ftz(&guard_verify.expect_err("refuses")),
            "guard::verify_rebuild"
        );
        assert!(
            is_ftz_eval(&space.expect_err("refuses")),
            "HnswSpace::from_artifact_reassociated keeps the named variant"
        );
    }

    assert!(
        HnswIndex::build_reassociated(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
            .is_ok()
    );
    let decoded = HnswIndex::decode_reassociated(matrix.clone(), &image).expect("decodes");
    assert_eq!(decoded.canonical_image(), image);
    assert!(index.verify_rebuild().expect("rebuilds"));
    assert_eq!(index.search_rows(3, 4).expect("searches").len(), 4);
    assert_eq!(index.search_vector(&query, 4).expect("searches").len(), 4);
    assert!(guard::verify_rebuild(&selected, &matrix, &params()).expect("verifies"));
    assert!(open().is_ok());
}
