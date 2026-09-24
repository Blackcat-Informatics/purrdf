// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The reassociated kNN kernels, executed on the host and on
//! `wasm32-unknown-unknown`, each held to the error bound of the exact answer.**
//!
//! The reassociated arithmetic promises no bits: its sums may be reassociated and
//! contracted differently on every target, so there is no golden to pin and a lexical
//! comparison like `knn_wasm_determinism`'s would be the wrong test. What it does
//! promise holds on every target, and this file executes that promise on each one it
//! runs on: the entry points exist and resolve there, the path they resolve is the one
//! the build was made for, each distance is within `2·n·ε·Σ|tᵢ|` of the exact one, the
//! bounded form agrees bit for bit with the full one, and an overflow is refused. The
//! same holds one level up: `EmbeddingKnnRelation::new_reassociated` constructs over a
//! real PURREMB space there, and every neighbour it ranks sits within that bound of the
//! exact relation's distance for the same neighbour.
//!
//! `make wasm-test` runs it twice on wasm32: on the baseline build, whose path is
//! `wasm-scalar`, and on a `+simd128` build, whose path is `wasm-simd128`. Natively it
//! is an ordinary `#[test]`, on `x86_64` and `aarch64` along their named paths and on
//! every other target along the portable one.
//!
//! ```text
//! cargo test -p purrdf-sparql-eval --target wasm32-unknown-unknown --test knn_wasm_reassociated
//! ```

#![allow(clippy::doc_markdown, reason = "prose names targets, not items")]

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::distance::{Arithmetic, Path};
use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec,
    RdfDatasetBuilder, RdfTermTarget, StageImplementation, TargetSet, TermValue, VectorDtype,
};
use purrdf_sparql_eval::knn::{Bound, Bounded, Exact, Reassociated, Resolved};
use purrdf_sparql_eval::{
    EmbeddingKnnRelation, EmbeddingSpace, Kernel, KnnGuard, PfArgs, PropertyFunction, knn::norm,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// A seeded splitmix64 stream of values in `[-1, 1)`, none of them exactly
/// representable as short decimals, so every product and partial sum rounds.
fn stream(len: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
        })
        .collect()
}

/// The reassociated arithmetic on this target.
fn resolved() -> Resolved<Reassociated> {
    Reassociated::resolve().expect("the default float environment is the IEEE one")
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_reassociated_path_is_the_one_this_build_was_made_for() {
    let fast = resolved();
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    assert_eq!(fast.path(), Path::WasmSimd128);
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    assert_eq!(fast.path(), Path::WasmScalar);
    #[cfg(target_arch = "aarch64")]
    assert_eq!(fast.path(), Path::Neon);
    #[cfg(target_arch = "x86_64")]
    assert!(matches!(
        fast.path(),
        Path::Sse2 | Path::Avx2Fma | Path::Avx512f
    ));
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "wasm64"
    )))]
    {
        assert_eq!(fast.path(), Path::Portable);
        assert_eq!(
            fast.image_code(),
            8,
            "the portable path records its own code"
        );
    }
    assert!(
        fast.evidence()
            .is_some_and(|text| text.contains(fast.path().name())),
        "the handle names its divergence along its own path"
    );
    assert!(Reassociated::IMAGE_CODES.contains(&fast.image_code()));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_reassociated_distance_is_within_the_error_bound_of_the_exact_one() {
    let fast = resolved();
    let exact_handle = Exact::resolve().expect("the default float environment");
    for (len, seed) in [(6_usize, 1_u64), (64, 2), (70, 3), (200, 4), (1_024, 5)] {
        let a = stream(len, seed);
        let b = stream(len, seed ^ 0xA5A5);
        let a32: Vec<f32> = a.iter().map(|&value| value as f32).collect();
        let (na, nb) = (norm(&a), norm(&b));
        // Components in [-1, 1) make every term at most 4, so `Σ|tᵢ| ≤ 4·n` and the
        // contract's `2·n·ε·Σ|tᵢ|` is at most `8·n²·ε`.
        let bound = 8.0 * (len * len) as f64 * f64::EPSILON;
        for kernel in [
            Kernel::SquaredEuclidean,
            Kernel::NegativeDot,
            Kernel::Cosine,
        ] {
            let exact = kernel
                .distance(exact_handle, &a, na, &b, nb)
                .expect("finite");
            let reassociated = kernel
                .distance_reassociated(fast, &a, na, &b, nb)
                .expect("finite");
            assert!(
                (exact - reassociated).abs() <= bound,
                "{kernel:?} at len {len} on {}: exact {exact}, reassociated {reassociated}",
                fast.path()
            );
            assert_eq!(
                kernel.distance_bounded_reassociated(
                    fast,
                    &a,
                    na,
                    &b,
                    nb,
                    Bound::Above(f64::INFINITY)
                ),
                Bounded::Below(reassociated),
                "{kernel:?} at len {len}: the bounded form is the same compilation"
            );
            // A narrow operand runs its own compilation; it is held to the same bound
            // against the exact answer for the same (widened) values.
            let na32 = norm(&a32);
            let exact32 = kernel
                .distance(exact_handle, &a32, na32, &b, nb)
                .expect("finite");
            let fast32 = kernel
                .distance_reassociated(fast, &a32, na32, &b, nb)
                .expect("finite");
            assert!(
                (exact32 - fast32).abs() <= bound,
                "{kernel:?} f32 at len {len}"
            );
        }
        let full = Kernel::SquaredEuclidean
            .distance_reassociated(fast, &a, na, &b, nb)
            .expect("finite");
        assert_eq!(
            Kernel::SquaredEuclidean.distance_bounded_reassociated(
                fast,
                &a,
                na,
                &b,
                nb,
                Bound::AtOrAbove(full.next_up())
            ),
            Bounded::Below(full),
            "len {len}: a bound one ulp above the distance is not met"
        );
        assert_eq!(
            Kernel::SquaredEuclidean.distance_bounded_reassociated(
                fast,
                &a,
                na,
                &b,
                nb,
                Bound::AtOrAbove(full)
            ),
            Bounded::Beyond,
            "len {len}: the distance itself meets `AtOrAbove`"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_reassociated_distance_refuses_an_overflow() {
    let fast = resolved();
    let huge = vec![1e300_f64; 70];
    let negative = vec![-1e300_f64; 70];
    assert_eq!(
        Kernel::NegativeDot.distance_reassociated(fast, &huge, 0.0, &huge, 0.0),
        None
    );
    assert_eq!(
        Kernel::SquaredEuclidean.distance_bounded_reassociated(
            fast,
            &huge,
            0.0,
            &negative,
            0.0,
            Bound::AtOrAbove(1.0)
        ),
        Bounded::NonFinite
    );
    let large = vec![1e150_f64; 2];
    assert!(
        Kernel::SquaredEuclidean
            .distance_reassociated(fast, &large, 0.0, &[0.0_f64, 0.0], 0.0)
            .is_some_and(f64::is_finite),
        "the valid neighbour ranks"
    );
}

/// The fixture's data namespace.
const EX: &str = "https://example.org/d/";

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

/// Encode `rows` as a sealed PURREMB artifact under `metric` and open it as a queryable
/// space.
fn space(rows: &[(String, Vec<f64>)], metric: DistanceMetric) -> EmbeddingSpace {
    let dimension = u32::try_from(rows[0].1.len()).expect("small");
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");

    let mut targets = Vec::with_capacity(rows.len());
    let mut bindings = Vec::with_capacity(rows.len());
    for (local, _) in rows {
        let text = format!("{EX}{local}");
        let target = RdfTermTarget::Iri(text.clone())
            .into_target(true, None)
            .expect("term target");
        bindings.push((target.id, TermValue::iri(text)));
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
        rows: rows
            .iter()
            .zip(&bindings)
            .map(|((_, values), (target, _))| MatrixRow::new(*target, values.clone()))
            .collect(),
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
        KnnGuard::new(64, 8).expect("positive bounds"),
    )
    .expect("the space opens")
}

/// Every neighbour `relation` ranks for the seed `query`, by local name, with its
/// distance.
fn neighbours(relation: &dyn PropertyFunction, query: &str, k: usize) -> BTreeMap<String, f64> {
    let seed = TermValue::iri(format!("{EX}{query}"));
    let count = TermValue::typed_literal(k.to_string(), "http://www.w3.org/2001/XMLSchema#integer");
    let subject = [None];
    let object = [Some(&seed), Some(&count), None];
    let mut cursor = relation
        .open(&PfArgs::new(&subject, &object), None)
        .expect("the invocation opens");
    let mut out = BTreeMap::new();
    while let Some(row) = cursor.next().expect("the search runs") {
        let TermValue::Iri(text) = &row[0] else {
            panic!("the neighbour is an IRI")
        };
        let TermValue::Literal { lexical_form, .. } = &row[3] else {
            panic!("the distance is a literal")
        };
        out.insert(
            text.strip_prefix(EX).expect("fixture IRI").to_owned(),
            lexical_form.parse::<f64>().expect("an xsd:double lexical"),
        );
    }
    assert_eq!(out.len(), k, "every requested neighbour is ranked");
    out
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_reassociated_relation_constructs_and_ranks_within_the_error_bound_of_the_exact_one() {
    const DIMS: usize = 70;
    const ROWS: usize = 8;
    let rows: Vec<(String, Vec<f64>)> = (0..ROWS)
        .map(|row| (format!("r{row}"), stream(DIMS, 0x51 + row as u64)))
        .collect();
    // Components in [-1, 1) make every term at most 4, so the contract's
    // `2·n·ε·Σ|tᵢ|` is at most `8·n²·ε`, as in the kernel-level test above.
    let bound = 8.0 * (DIMS * DIMS) as f64 * f64::EPSILON;
    for metric in [
        DistanceMetric::SquaredEuclidean,
        DistanceMetric::NegativeDot,
        DistanceMetric::Cosine,
    ] {
        let space = Arc::new(space(&rows, metric.clone()));
        let exact = EmbeddingKnnRelation::new(Arc::clone(&space));
        let fast = EmbeddingKnnRelation::new_reassociated(Arc::clone(&space));
        assert!(
            fast.is_ok(),
            "{metric:?}: the reassociated relation constructs on this target: {:?}",
            fast.as_ref().err()
        );
        let fast = fast.expect("checked above");
        assert_eq!(
            fast.resolved().map(Resolved::path),
            Some(resolved().path()),
            "the relation runs the path this build resolves"
        );
        for (query, _) in &rows {
            let exact_rows = neighbours(&exact, query, ROWS);
            let fast_rows = neighbours(&fast, query, ROWS);
            assert_eq!(
                exact_rows.keys().collect::<Vec<_>>(),
                fast_rows.keys().collect::<Vec<_>>(),
                "{metric:?} from {query}: every row is ranked by both"
            );
            for (neighbour, exact_distance) in &exact_rows {
                let fast_distance = fast_rows[neighbour];
                assert!(
                    (exact_distance - fast_distance).abs() <= bound,
                    "{metric:?} {query} -> {neighbour} on {}: exact {exact_distance}, \
                     reassociated {fast_distance}",
                    resolved().path()
                );
            }
        }
    }
}
