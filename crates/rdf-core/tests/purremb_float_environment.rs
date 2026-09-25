// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A deterministic-L2 projection is sealed, verified and read only on a thread whose
//! float environment is the IEEE one; a raw projection, which involves no arithmetic, is
//! handled on any thread.
//!
//! PURREMB §13.2 normalizes a projection by a binary64 norm fold and division, and folds
//! the normalized values into the projection digest. A thread with flush-to-zero set
//! computes different bits for a row whose norm is subnormal: the writer would seal a
//! digest no IEEE reader recomputes, and the verifier would report an honest artifact as
//! a digest mismatch. So each resolves the exact arithmetic before the first normalized
//! row, and a flushing thread is refused with `EmbeddingError::FloatEnvironment`. This
//! suite sets FTZ on the test thread with `ldmxcsr`, runs the writer, the verifier and the
//! resolve that `EffectiveMatrixView::f64_row` needs, restores the register, and asserts
//! the named refusal; the raw neighbour is built and verified under the same flushed
//! register, and the normalized artifact is built, verified and read once it is restored.
//! The register is per-thread, so no other test observes it.
//!
//! Only where MXCSR governs binary64: `x86_64`, and 32-bit `x86` with SSE2. Without SSE2
//! binary64 runs on the x87, which has no flush-to-zero mode for FTZ to stand in for; its
//! own departures (a directed rounding control, a guard that did not take hold) are
//! exercised in `distance::binary64_tests`.

#![cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]

use purrdf_core::distance::{Arithmetic as _, Exact, FloatEnvironmentError};
use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingError, EmbeddingFamilyContract, EmbeddingView, MatrixInput, MatrixRow,
    PrefixPostprocessing, ProjectionSpec, RdfDatasetBuilder, StageImplementation, TargetSet,
    VectorDtype, verify_embedding,
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

/// Whether `error` is the flush-to-zero refusal: named by the MXCSR where it is read
/// (`x86_64`), and by the behavioural probe where it is not (32-bit x86).
fn is_ftz(error: &EmbeddingError) -> bool {
    let EmbeddingError::FloatEnvironment(FloatEnvironmentError::FlushToZero { .. }) = error else {
        return false;
    };
    true
}

/// The one stored row: the 3-4-5 triangle scaled into the subnormal range, so its norm
/// is `5e-310`. The fold's ratios stay normal; only the final product is subnormal, which
/// a flushing thread returns as `+0` -- and the writer would then refuse a row that has a
/// direction as a zero norm.
const ROW: [f64; 2] = [3e-310, 4e-310];

/// PURREMB §13.2's scaled L2 fold, transcribed from the specification's written order.
/// `black_box` keeps it from being folded at compile time under the default environment.
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

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("artifact")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("https://example.org/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/octet-stream",
            vec![1],
        )
        .expect("stage"),
    )
}

/// Build the one-row binary64 artifact whose only projection has `postprocessing`.
fn build(postprocessing: PrefixPostprocessing) -> Result<Vec<u8>, EmbeddingError> {
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let target = source.dataset_target(true).expect("dataset target");
    let target_id = target.id;
    let set = TargetSet::new(vec![target_id]).expect("target set");
    let contract = EmbeddingFamilyContract {
        model: artifact("model-ftz"),
        engine: artifact("engine-ftz"),
        tokenizer: artifact("tokenizer-ftz"),
        execution: stage("execution-ftz"),
        subject_projection: stage("projection-ftz"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling-ftz"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F64,
        metric: DistanceMetric::Cosine,
        dimensionality: DimensionalityPolicy::fixed(2, postprocessing).expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family");
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets: vec![target],
        target_sets: vec![set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: set.id,
        stored_dimension: 2,
        rows: vec![MatrixRow::new(target_id, ROW.to_vec())],
        projections: vec![ProjectionSpec::derive(family.id, 2, postprocessing)],
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f64_matrix(matrix);
    builder.build().map(|encoded| encoded.bytes)
}

fn verify(bytes: &[u8]) -> Result<(), EmbeddingError> {
    let mut view = EmbeddingView::from_bytes(bytes)?;
    verify_embedding(&mut view).map(|_| ())
}

#[test]
fn a_normalized_projection_needs_the_ieee_environment_and_a_raw_one_does_not() {
    // Built in the default environment, before FTZ is set.
    let normalized = build(PrefixPostprocessing::DeterministicL2).expect("builds by default");
    let raw = build(PrefixPostprocessing::None).expect("builds by default");

    let (flushed_norm, sealed, verified, resolved, raw_sealed, raw_verified) = {
        let flushed = Flushed::new();
        let norm = reference_norm(&ROW);
        let sealed = build(PrefixPostprocessing::DeterministicL2);
        let verified = verify(&normalized);
        let resolved = Exact::resolve();
        let raw_sealed = build(PrefixPostprocessing::None);
        let raw_verified = verify(&raw);
        drop(flushed);
        (norm, sealed, verified, resolved, raw_sealed, raw_verified)
    };
    // What the refusal prevents: on that thread the normative fold gives this row a zero
    // norm, so the writer would have refused a row with a direction, or sealed other bits.
    assert_eq!(
        flushed_norm.to_bits(),
        0,
        "the flushed thread folds the row's norm to +0"
    );
    assert!(
        is_ftz(&sealed.expect_err("the writer refuses")),
        "the writer names the float environment rather than a zero norm"
    );
    assert!(
        is_ftz(&verified.expect_err("the verifier refuses")),
        "the verifier names the float environment rather than a digest mismatch"
    );
    assert!(
        matches!(resolved, Err(FloatEnvironmentError::FlushToZero { .. })),
        "no handle for `EffectiveMatrixView::f64_row` on a flushing thread"
    );
    // The valid neighbour under the SAME flushed register: a raw projection folds stored
    // bytes and no arithmetic, so it is sealed -- to the default environment's bytes --
    // and verified there.
    assert_eq!(
        raw_sealed.expect("a raw projection is sealed on a flushing thread"),
        raw,
        "the raw artifact's bytes do not depend on the environment"
    );
    raw_verified.expect("a raw projection verifies on a flushing thread");

    // The valid neighbour once the register is restored: the normalized artifact is sealed
    // to the same bytes, verifies, and reads back as each stored value divided by the
    // subnormal norm the flushed thread lost.
    let expected_norm = reference_norm(&ROW);
    assert!(
        expected_norm.is_subnormal(),
        "the default environment keeps the subnormal norm, got {expected_norm:e}"
    );
    assert_eq!(
        build(PrefixPostprocessing::DeterministicL2).expect("builds"),
        normalized
    );
    let mut view = EmbeddingView::from_bytes(&normalized).expect("opens");
    verify_embedding(&mut view).expect("verifies");
    let projection = view.projections().next().expect("one projection");
    let space = projection.vector_space_id();
    let set = view
        .matrix(projection.matrix_id())
        .expect("the projection's matrix")
        .target_set_id();
    let effective = view
        .effective_matrix(set, space)
        .expect("readable")
        .expect("present");
    let exact = Exact::resolve().expect("the default environment resolves");
    assert_eq!(
        exact.norm(&ROW).to_bits(),
        expected_norm.to_bits(),
        "the public norm is the reference fold"
    );
    let values = effective
        .f64_row(0, exact)
        .expect("a normalized row")
        .map(|value| value.expect("finite").to_bits())
        .collect::<Vec<_>>();
    let expected = ROW
        .iter()
        .map(|value| (value / expected_norm).to_bits())
        .collect::<Vec<_>>();
    assert_eq!(values, expected, "each value divided by the subnormal norm");
}
