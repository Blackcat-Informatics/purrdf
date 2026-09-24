// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only latency harness for the embedding kNN relation's exhaustive search, under
//! both arithmetics.
//!
//! One deterministic PURREMB space per `(metric, dimension)` is sealed, verified and
//! opened once through the public [`EmbeddingSpace::from_artifact`], then searched by
//! [`EmbeddingKnnRelation::new`] (the `Exact` arithmetic) and
//! [`EmbeddingKnnRelation::new_reassociated`] (the `Reassociated` one) over the same
//! space. One iteration is one whole invocation as the engine runs it: `open` with a
//! bound seed and `k`, then drain the cursor -- the full-space scan, the top-`k`
//! selection and the canonical `xsd:double` rendering of every emitted distance.
//!
//! Benchmark ids are `knn_relation/<arithmetic>/<metric>/d<dims>`. The space stores
//! binary64 rows, so every scan here is the `f64xf64` batch kernel: the dot fold
//! (`distance.exact.dot` / `distance.reassociated.dot`) under `dot` and `cosine`, the
//! squared-Euclidean fold (`distance.exact.sqeuclid` / `distance.reassociated.sqeuclid`)
//! under `sqeuclid`, with the norms of `knn.norm` computed once at construction.
//!
//! All inputs come from a fixed splitmix64 stream. No timing or speedup is asserted, and
//! the two arithmetics are reported side by side rather than divided into each other.

use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec,
    RdfDatasetBuilder, RdfTermTarget, StageImplementation, TargetSet, TermValue, VectorDtype,
};
use purrdf_sparql_eval::{
    EmbeddingKnnRelation, EmbeddingSpace, KnnGuard, PfArgs, PropertyFunction,
};

/// The rows in every space.
const ROWS: usize = 4_096;

/// The neighbours each invocation asks for.
const K: usize = 10;

/// The production embedding widths measured.
const DIMS: [usize; 3] = [384, 768, 1_536];

/// The seed of the fixture stream.
const SEED: u64 = 0x4b4e_4e5f_5245_4c4e;

/// The fixture's data namespace.
const EX: &str = "https://example.org/d/";

/// One step of splitmix64.
const fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// `len` values in `[-1, 1)` from the fixed stream at `seed`, none exactly zero.
fn stream(len: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = splitmix64(state);
            let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
            let value = unit.mul_add(2.0, -1.0);
            if value == 0.0 { 0.25 } else { value }
        })
        .collect()
}

/// A fixture artifact identity, distinct per `name`.
fn identity(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("artifact identity")
}

/// A fixture applied stage, distinct per `name`.
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

/// Seal `ROWS` rows of `dims` components as a PURREMB artifact under `metric` and open
/// it as a queryable space.
fn space(dims: usize, metric: DistanceMetric) -> EmbeddingSpace {
    let dimension = u32::try_from(dims).expect("a production width fits u32");
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");

    let mut values = stream(ROWS * dims, SEED ^ dims as u64).into_iter();
    let mut targets = Vec::with_capacity(ROWS);
    let mut bindings = Vec::with_capacity(ROWS);
    let mut rows = Vec::with_capacity(ROWS);
    for row in 0..ROWS {
        let text = format!("{EX}r{row}");
        let target = RdfTermTarget::Iri(text.clone())
            .into_target(true, None)
            .expect("term target");
        bindings.push((target.id, TermValue::iri(text)));
        rows.push(MatrixRow::new(
            target.id,
            values.by_ref().take(dims).collect(),
        ));
        targets.push(target);
    }
    let set = TargetSet::new(targets.iter().map(|t| t.id).collect()).expect("target set");
    let mut declared = targets;
    declared.push(source.dataset_target(true).expect("dataset target"));
    declared.sort_unstable_by_key(|target| target.id);

    let contract = EmbeddingFamilyContract {
        model: identity("model"),
        engine: identity("engine"),
        tokenizer: identity("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F64,
        metric,
        dimensionality: DimensionalityPolicy::fixed(dimension, PrefixPostprocessing::None)
            .expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family");
    let projection = ProjectionSpec::derive(family.id, dimension, PrefixPostprocessing::None);
    let vector_space = projection.vector_space_id;

    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: set.id,
        stored_dimension: dimension,
        rows,
        projections: vec![projection],
    };
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets: declared,
        target_sets: vec![set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f64_matrix(matrix);
    let encoded = builder.build().expect("encoded artifact");

    EmbeddingSpace::from_artifact(
        &encoded.bytes,
        set.id,
        vector_space,
        bindings,
        KnnGuard::new(ROWS as u64, K as u64).expect("positive bounds"),
    )
    .expect("the space opens")
}

/// One whole invocation: open for `seed` and `k`, drain the cursor, return the row count.
fn invoke(relation: &dyn PropertyFunction, seed: &TermValue, count: &TermValue) -> usize {
    let subject = [None];
    let object = [Some(seed), Some(count), None];
    let mut cursor = relation
        .open(&PfArgs::new(&subject, &object), None)
        .expect("the invocation opens");
    let mut emitted = 0;
    while let Some(row) = cursor.next().expect("the search runs") {
        black_box(&row);
        emitted += 1;
    }
    emitted
}

fn bench_knn_relation(c: &mut Criterion) {
    let seed = TermValue::iri(format!("{EX}r{}", ROWS / 3));
    let count = TermValue::typed_literal(K.to_string(), "http://www.w3.org/2001/XMLSchema#integer");

    let mut group = c.benchmark_group("knn_relation");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(1));
    for dims in DIMS {
        group.throughput(Throughput::Elements((ROWS * dims) as u64));
        for (label, metric) in [
            ("dot", DistanceMetric::NegativeDot),
            ("sqeuclid", DistanceMetric::SquaredEuclidean),
            ("cosine", DistanceMetric::Cosine),
        ] {
            let space = Arc::new(space(dims, metric));
            let exact = EmbeddingKnnRelation::new(Arc::clone(&space));
            let reassociated = EmbeddingKnnRelation::new_reassociated(Arc::clone(&space))
                .expect("the reassociated relation constructs on this target");
            // Both relations answer the full `k` before either is timed, so neither id
            // can be measuring an empty or truncated invocation.
            assert_eq!(invoke(&exact, &seed, &count), K);
            assert_eq!(invoke(&reassociated, &seed, &count), K);

            group.bench_function(format!("exact/{label}/d{dims}"), |b| {
                b.iter(|| black_box(invoke(&exact, black_box(&seed), &count)));
            });
            group.bench_function(format!("reassociated/{label}/d{dims}"), |b| {
                b.iter(|| black_box(invoke(&reassociated, black_box(&seed), &count)));
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_knn_relation);
criterion_main!(benches);
