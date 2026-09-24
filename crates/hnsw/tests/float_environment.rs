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
//! On `x86_64` the refusal names the MXCSR, which is read before anything else. On 32-bit
//! x86 no register is read, so the same flushed thread is refused on the behavioural
//! probe's evidence alone, and the default environment still answers: that is the
//! target class the probe exists for. The refusal itself is unit-tested for `aarch64`'s
//! FPCR and RISC-V's `frm` in `purrdf_core::distance`, and wasm has no such register.

#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

#[path = "support/purremb.rs"]
mod purremb;

use std::sync::Arc;

use purrdf_core::DistanceMetric;
use purrdf_core::distance::{
    Arithmetic, Bound, Bounded, Exact, FloatEnvironmentError, FloatEnvironmentEvidence, Measure,
    Reassociated, RecordedPathError, Resolved, RowsRef,
};
use purrdf_hnsw::{HnswError, HnswIndex, Kernel, Params, VectorMatrix, guard, relation::HnswSpace};
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

/// Whether `refusal` is the flush-to-zero refusal this target reports for an FTZ thread:
/// the MXCSR by name where it is read, and the probe's flushed-result row where not.
fn is_ftz_refusal(refusal: &FloatEnvironmentError) -> bool {
    let FloatEnvironmentError::FlushToZero { evidence } = refusal else {
        return false;
    };
    if cfg!(target_arch = "x86_64") {
        matches!(
            evidence,
            FloatEnvironmentEvidence::Register { name: "MXCSR", .. }
        )
    } else {
        matches!(
            evidence,
            FloatEnvironmentEvidence::Probe {
                operation: "f64::MIN_POSITIVE * 0.5",
                observed: 0,
                ..
            }
        )
    }
}

fn is_ftz(error: &HnswError) -> bool {
    matches!(error, HnswError::FloatEnvironment(refusal) if is_ftz_refusal(refusal))
}

fn is_ftz_eval(error: &EvalError) -> bool {
    matches!(error, EvalError::FloatEnvironment(refusal) if is_ftz_refusal(refusal))
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

/// The smallest-magnitude row this suite scores: its square, and its product with itself,
/// are subnormal, so a thread that flushes subnormal results computes zero for them.
const TINY: f64 = 1e-160;

/// Three two-component rows: `[TINY, 0]`, `[0, 0]`, `[TINY, 0]`. Row 0 to row 1 is a
/// squared Euclidean distance of `TINY²`, and row 0 against row 2 a dot product of `TINY²`,
/// both subnormal.
fn subnormal_rows() -> Vec<f64> {
    vec![TINY, 0.0, 0.0, 0.0, TINY, 0.0]
}

/// The batch kernel's value for `(query, row)` under `resolved`: the oracle every pair
/// entry point is held to.
fn batch<A: Arithmetic>(
    resolved: Resolved<A>,
    measure: Measure,
    data: &[f64],
    query: usize,
    row: usize,
) -> Option<f64> {
    let rows = RowsRef::new(data, 3, 2, &[]).expect("a 3x2 matrix");
    let mut out = [None; 3];
    resolved.distances(measure, rows.row(query), 0.0, rows, &mut out);
    out[row]
}

/// Every public pair entry point on the handle `resolved`, for the pair `(query, row)` of
/// `matrix`, alongside the batch kernel's value for the same pair.
fn pairs_equal_batch<A: Arithmetic>(
    resolved: Resolved<A>,
    matrix: &VectorMatrix,
    kernel: Kernel,
    query: usize,
    row: usize,
    pair: Option<f64>,
) -> f64 {
    let data = subnormal_rows();
    let expected = batch(resolved, kernel.measure(), &data, query, row).expect("finite");
    let bits = Some(expected.to_bits());
    let law = A::ID;
    assert_eq!(
        pair.map(f64::to_bits),
        bits,
        "{law} {kernel:?}: Kernel pair"
    );
    assert_eq!(
        matrix
            .distance(resolved, kernel, query, 0.0, row, 0.0)
            .map(f64::to_bits),
        bits,
        "{law} {kernel:?}: VectorMatrix::distance"
    );
    assert_eq!(
        matrix
            .distance_from_query(
                resolved,
                kernel,
                &data[query * 2..query * 2 + 2],
                0.0,
                row,
                0.0
            )
            .map(f64::to_bits),
        bits,
        "{law} {kernel:?}: VectorMatrix::distance_from_query"
    );
    assert_eq!(
        matrix.distance_bounded(
            resolved,
            kernel,
            query,
            0.0,
            row,
            0.0,
            Bound::Above(f64::INFINITY)
        ),
        Bounded::Below(expected),
        "{law} {kernel:?}: VectorMatrix::distance_bounded"
    );
    expected
}

#[test]
fn every_pair_entry_point_needs_a_handle_a_flushing_thread_cannot_obtain() {
    // The pair entry points -- `Kernel::distance` and its bounded and reassociated
    // siblings, and `VectorMatrix::distance`, `distance_from_query` and
    // `distance_bounded` -- all take a `Resolved` handle, so the only way to reach one is
    // through `resolve`/`resolve_recorded`. On a flushing thread every one of those is
    // refused by name, so no pair distance can be computed there.
    let matrix = VectorMatrix::new(3, 2, subnormal_rows()).expect("finite rows");
    let (flushed_square, exact, fast, recorded, fast_recorded) = {
        let flushed = Flushed::new();
        let square = core::hint::black_box(TINY) * core::hint::black_box(TINY);
        let exact = Exact::resolve();
        let fast = Reassociated::resolve();
        let recorded = Exact::resolve_recorded(Exact::IMAGE_CODE);
        let fast_recorded = Reassociated::resolve_recorded(8);
        drop(flushed);
        (square, exact, fast, recorded, fast_recorded)
    };
    // The observation that makes the refusal worth something: on that thread the very term
    // these pairs are made of flushes to zero, so a pair computed there would have
    // returned different bits from the default environment's.
    assert_eq!(
        flushed_square.to_bits(),
        0,
        "the flushed thread computes TINY² as +0"
    );
    assert!(
        is_ftz_refusal(&exact.expect_err("refused")),
        "Exact::resolve"
    );
    assert!(
        is_ftz_refusal(&fast.expect_err("refused")),
        "Reassociated::resolve"
    );
    assert!(
        matches!(
            recorded.expect_err("refused"),
            RecordedPathError::FloatEnvironment(ref refusal) if is_ftz_refusal(refusal)
        ),
        "Exact::resolve_recorded"
    );
    assert!(
        matches!(
            fast_recorded.expect_err("refused"),
            RecordedPathError::FloatEnvironment(ref refusal) if is_ftz_refusal(refusal)
        ),
        "Reassociated::resolve_recorded checks the environment before the code"
    );

    // The valid neighbour: the same thread, register restored. Every pair entry point
    // answers, bit for bit what the batch kernel answers for the same pair, and the answer
    // is the subnormal the flushed thread would have lost.
    let square = core::hint::black_box(TINY) * core::hint::black_box(TINY);
    assert!(
        square.is_subnormal(),
        "the default environment keeps TINY² = {square:e}"
    );
    let exact = Exact::resolve().expect("the default environment resolves");
    let data = subnormal_rows();
    let (a, zero, b) = (&data[0..2], &data[2..4], &data[4..6]);
    let euclid = pairs_equal_batch(
        exact,
        &matrix,
        Kernel::SquaredEuclidean,
        0,
        1,
        Kernel::SquaredEuclidean.distance(exact, a, 0.0, zero, 0.0),
    );
    assert_eq!(euclid.to_bits(), square.to_bits(), "the exact TINY²");
    assert_eq!(
        Kernel::SquaredEuclidean.distance_bounded(
            exact,
            a,
            0.0,
            zero,
            0.0,
            Bound::Above(f64::INFINITY)
        ),
        Bounded::Below(euclid),
        "Kernel::distance_bounded"
    );
    let dot = pairs_equal_batch(
        exact,
        &matrix,
        Kernel::NegativeDot,
        0,
        2,
        Kernel::NegativeDot.distance(exact, a, 0.0, b, 0.0),
    );
    assert_eq!(dot.to_bits(), (-square).to_bits(), "the exact -TINY²");

    let fast = Reassociated::resolve().expect("the default environment resolves");
    for (kernel, row, other) in [
        (Kernel::SquaredEuclidean, 1, zero),
        (Kernel::NegativeDot, 2, b),
    ] {
        let value = pairs_equal_batch(
            fast,
            &matrix,
            kernel,
            0,
            row,
            kernel.distance_reassociated(fast, a, 0.0, other, 0.0),
        );
        assert!(
            value.is_subnormal(),
            "{kernel:?} reassociated keeps the subnormal: {value:e}"
        );
        assert_eq!(
            kernel.distance_bounded_reassociated(
                fast,
                a,
                0.0,
                other,
                0.0,
                Bound::Above(f64::INFINITY)
            ),
            Bounded::Below(value),
            "{kernel:?}: Kernel::distance_bounded_reassociated"
        );
    }
}
