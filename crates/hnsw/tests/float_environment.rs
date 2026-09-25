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
use purrdf_hnsw::{
    HnswError, HnswIndex, Kernel, Params, Ranked, VectorMatrix, guard, relation::HnswSpace,
};
use purrdf_sparql_eval::{
    EmbeddingKnnRelation, EmbeddingSpace, EvalError, KnnGuard, PfArgs, PfRow, PropertyFunction,
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

/// Every row of one kNN invocation seeded at `seed`: the ranked read of depth `k` when
/// `neighbour` is free, the membership lookup of `neighbour` when it is bound.
fn knn_invoke<A: Arithmetic>(
    relation: &EmbeddingKnnRelation<A>,
    seed: &purrdf_core::TermValue,
    neighbour: Option<&purrdf_core::TermValue>,
) -> Result<Vec<PfRow>, EvalError> {
    let count =
        purrdf_core::TermValue::typed_literal("3", "http://www.w3.org/2001/XMLSchema#integer");
    let subject = [neighbour];
    let object = [Some(seed), neighbour.is_none().then_some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None)?;
    let mut rows = Vec::new();
    while let Some(row) = cursor.next()? {
        rows.push(row);
    }
    Ok(rows)
}

/// A batch answer as its rows and distance bits, so equality is bit-identity.
fn bits(batch: &[Vec<Ranked>]) -> Vec<Vec<(usize, u64)>> {
    batch
        .iter()
        .map(|ranked| {
            ranked
                .iter()
                .map(|scored| (scored.row, scored.distance.to_bits()))
                .collect()
        })
        .collect()
}

/// What one thread's calls answered, every compute entry that runs on a shared index or
/// relation after it was built on another thread.
struct WorkerAnswers {
    exact_batch: Result<Vec<Vec<Ranked>>, HnswError>,
    fast_batch: Result<Vec<Vec<Ranked>>, HnswError>,
    exact_pair: Result<f64, HnswError>,
    fast_pair: Result<f64, HnswError>,
    exact_scan: Result<Vec<PfRow>, EvalError>,
    fast_scan: Result<Vec<PfRow>, EvalError>,
    exact_lookup: Result<Vec<PfRow>, EvalError>,
    fast_lookup: Result<Vec<PfRow>, EvalError>,
}

/// **The float-environment check belongs to the thread that computes, not the one that
/// built.**
///
/// Every index and relation here is built on the test thread, in the default
/// environment, and then shared with two worker threads through plain references —
/// which is exactly what a stored, thread-free selection allows and a stored handle no
/// longer can (`Resolved` is neither `Send` nor `Sync`). One worker sets flush-to-zero
/// on itself and makes the calls; the other makes the same calls in the default
/// environment.
///
/// The treatment: every call from the flushing worker — `search_batch` and `row_distance`
/// on the exact and reassociated indexes, the ranked scan and the membership lookup on
/// the exact and reassociated kNN relations — is refused by name. A structure that
/// carried the builder's proof across would have computed there and answered.
///
/// The valid neighbour: every call from the clean worker answers, bit for bit, what the
/// same calls answer on the test thread, so the refusal is the worker's environment and
/// not a structure that cannot be searched off the thread that built it.
#[test]
fn a_worker_thread_is_checked_where_it_computes_not_where_the_index_was_built() {
    let fixture = purremb::Fixture::new(40, 20, params());
    let matrix = fixture.matrix.clone();
    let exact = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
        .expect("builds in the default environment");
    let fast =
        HnswIndex::build_reassociated(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds in the default environment");
    let space = Arc::new(
        EmbeddingSpace::from_artifact(
            &fixture.without_index,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(64, 8).expect("valid"),
        )
        .expect("the space opens in the default environment"),
    );
    let exact_knn = EmbeddingKnnRelation::new(Arc::clone(&space));
    let fast_knn = EmbeddingKnnRelation::new_reassociated(Arc::clone(&space))
        .expect("the reassociated relation constructs in the default environment");
    let seed = fixture.terms[0].clone();
    let held = fixture.terms[7].clone();
    let queries: Vec<usize> = (0..matrix.rows()).collect();

    let answers = || WorkerAnswers {
        exact_batch: exact.search_batch(&queries, 4),
        fast_batch: fast.search_batch(&queries, 4),
        exact_pair: exact.row_distance(0, 7),
        fast_pair: fast.row_distance(0, 7),
        exact_scan: knn_invoke(&exact_knn, &seed, None),
        fast_scan: knn_invoke(&fast_knn, &seed, None),
        exact_lookup: knn_invoke(&exact_knn, &seed, Some(&held)),
        fast_lookup: knn_invoke(&fast_knn, &seed, Some(&held)),
    };

    // The single-thread oracle, on the thread that built everything.
    let single_exact: Vec<Vec<Ranked>> = queries
        .iter()
        .map(|&row| exact.search_rows(row, 4).expect("searches"))
        .collect();
    let single_fast: Vec<Vec<Ranked>> = queries
        .iter()
        .map(|&row| fast.search_rows(row, 4).expect("searches"))
        .collect();
    let here = answers();

    let (flushed, clean) = std::thread::scope(|scope| {
        let flushed = scope.spawn(|| {
            let _flushed = Flushed::new();
            answers()
        });
        let clean = scope.spawn(answers);
        (
            flushed.join().expect("the flushing worker returns"),
            clean.join().expect("the clean worker returns"),
        )
    });

    assert!(
        is_ftz(&flushed.exact_batch.expect_err("refused")),
        "exact search_batch"
    );
    assert!(
        is_ftz(&flushed.fast_batch.expect_err("refused")),
        "reassociated search_batch"
    );
    assert!(
        is_ftz(&flushed.exact_pair.expect_err("refused")),
        "exact row_distance"
    );
    assert!(
        is_ftz(&flushed.fast_pair.expect_err("refused")),
        "reassociated row_distance"
    );
    assert!(
        is_ftz_eval(&flushed.exact_scan.expect_err("refused")),
        "exact kNN scan"
    );
    assert!(
        is_ftz_eval(&flushed.fast_scan.expect_err("refused")),
        "reassociated kNN scan, whose relation selected its path on the building thread"
    );
    assert!(
        is_ftz_eval(&flushed.exact_lookup.expect_err("refused")),
        "exact kNN membership lookup"
    );
    assert!(
        is_ftz_eval(&flushed.fast_lookup.expect_err("refused")),
        "reassociated kNN membership lookup"
    );

    let clean_exact = clean.exact_batch.expect("the clean worker searches");
    let clean_fast = clean.fast_batch.expect("the clean worker searches");
    assert_eq!(bits(&clean_exact), bits(&single_exact), "exact batch");
    assert_eq!(bits(&clean_fast), bits(&single_fast), "reassociated batch");
    assert_eq!(
        bits(&here.exact_batch.expect("searches")),
        bits(&single_exact)
    );
    assert_eq!(
        clean.exact_pair.expect("answers").to_bits(),
        here.exact_pair.expect("answers").to_bits()
    );
    assert_eq!(
        clean.fast_pair.expect("answers").to_bits(),
        here.fast_pair.expect("answers").to_bits()
    );
    let exact_scan = here.exact_scan.expect("answers");
    assert_eq!(exact_scan.len(), 3);
    assert_eq!(clean.exact_scan.expect("answers"), exact_scan);
    assert_eq!(
        clean.fast_scan.expect("answers"),
        here.fast_scan.expect("answers")
    );
    let lookup = here.exact_lookup.expect("answers");
    assert_eq!(lookup.len(), 1, "the held term is answered by its row");
    assert_eq!(clean.exact_lookup.expect("answers"), lookup);
    assert_eq!(
        clean.fast_lookup.expect("answers"),
        here.fast_lookup.expect("answers")
    );
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

/// A row whose L2 norm is subnormal: the 3-4-5 triangle scaled into the subnormal range,
/// so its norm is `5e-310`. The fold's own ratios stay normal (`0.75`, `0.5625`, `1.5625`);
/// only the final product `scale · √ssq` is subnormal, and a thread that flushes subnormal
/// results returns `+0` for it.
const SUBNORMAL_ROW: [f64; 2] = [3e-310, 4e-310];

/// PURREMB §13.2's scaled L2 fold, transcribed from the specification's written order:
/// the reference every public norm entry point is held to, and the arithmetic a flushed
/// thread is shown to run differently. No handle, so it can be run where none can be
/// obtained; `black_box` keeps the compiler from folding it at compile time, under the
/// default environment, instead of on the thread under test.
fn reference_norm(values: &[f64]) -> f64 {
    let mut scale = 0.0_f64;
    let mut ssq = 1.0_f64;
    for &value in core::hint::black_box(values) {
        let value = value.abs();
        if value == 0.0 {
            continue;
        }
        if scale < value {
            let ratio = scale / value;
            let square = ratio * ratio;
            let product = ssq * square;
            ssq = 1.0 + product;
            scale = value;
        } else {
            let ratio = value / scale;
            let square = ratio * ratio;
            ssq += square;
        }
    }
    let root = ssq.sqrt();
    core::hint::black_box(scale) * root
}

#[test]
fn every_norm_entry_point_needs_the_exact_handle_a_flushing_thread_cannot_obtain() {
    // The public norm entry points -- `Resolved::<Exact>::norm` (re-exported by the kNN
    // module), reached directly or from a reassociated handle through `Resolved::exact`,
    // and `VectorMatrix::norm_of_row` -- all take the exact handle, so the only way to
    // reach one is through `resolve`/`resolve_recorded`, each of which refuses a flushing
    // thread by name. So no norm, and no cosine kernel dividing by one, is computed there.
    let matrix = VectorMatrix::new(1, 2, SUBNORMAL_ROW.to_vec()).expect("finite row");
    let (flushed_norm, exact, fast, recorded) = {
        let flushed = Flushed::new();
        let norm = reference_norm(&SUBNORMAL_ROW);
        let exact = Exact::resolve();
        let fast = Reassociated::resolve();
        let recorded = Exact::resolve_recorded(Exact::IMAGE_CODE);
        drop(flushed);
        (norm, exact, fast, recorded)
    };
    // The observation that makes the refusal worth something: on that thread the normative
    // fold itself returns +0 for this row, a norm a cosine kernel would divide by (or a
    // zero-norm check would reject a row that has a direction).
    assert_eq!(
        flushed_norm.to_bits(),
        0,
        "the flushed thread folds the row's norm to +0"
    );
    assert!(
        is_ftz_refusal(&exact.expect_err("refused")),
        "Exact::resolve"
    );
    assert!(
        is_ftz_refusal(&fast.expect_err("refused")),
        "Reassociated::resolve, the only source of a handle `exact` converts"
    );
    assert!(
        matches!(
            recorded.expect_err("refused"),
            RecordedPathError::FloatEnvironment(ref refusal) if is_ftz_refusal(refusal)
        ),
        "Exact::resolve_recorded"
    );

    // The valid neighbour: the same thread, register restored. The reference fold now
    // keeps the subnormal norm, and every public entry point returns exactly its bits --
    // the nonzero value the flushed thread lost.
    let expected = reference_norm(&SUBNORMAL_ROW);
    assert!(
        expected.is_subnormal(),
        "the default environment keeps the subnormal norm, got {expected:e}"
    );
    let exact = Exact::resolve().expect("the default environment resolves");
    let fast = Reassociated::resolve().expect("the default environment resolves");
    let recorded = Exact::resolve_recorded(Exact::IMAGE_CODE).expect("resolves");
    for (entry, value) in [
        ("Resolved::<Exact>::norm", exact.norm(&SUBNORMAL_ROW)),
        (
            "Resolved::<Exact>::norm on a recorded handle",
            recorded.norm(&SUBNORMAL_ROW),
        ),
        (
            "Resolved::<Reassociated>::exact().norm",
            fast.exact().norm(&SUBNORMAL_ROW),
        ),
        ("VectorMatrix::norm_of_row", matrix.norm_of_row(exact, 0)),
        (
            "VectorMatrix::norm_of_row from a reassociated handle",
            matrix.norm_of_row(fast.exact(), 0),
        ),
    ] {
        assert_eq!(
            value.to_bits(),
            expected.to_bits(),
            "{entry}: {value:e} against the reference {expected:e}"
        );
    }
}
