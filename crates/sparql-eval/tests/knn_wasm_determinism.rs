// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **The same nearest-neighbour answer, executed on x86-64 and on
//! `wasm32-unknown-unknown`, compared against the same pinned bytes.**
//!
//! The kNN kernels rank by binary64 arithmetic. Every other test in this crate proves the
//! kernel is a pure function *of one target*: run it twice on this machine and it agrees
//! with itself. That is not the claim the surface makes. The claim is that a host and a
//! browser answering the same query over the same artifact return the same rows in the
//! same order with the same distance lexicals — and two runs on one target cannot
//! distinguish a kernel that is target-independent from one that merely happens to be
//! self-consistent wherever it was last compiled.
//!
//! Floating-point ranking is exactly where that distinction bites. Reassociating a sum,
//! or contracting a multiply and an add into a fused multiply-add, changes the last bit;
//! two nearly-tied neighbours then swap, and the two engines disagree about the *answer*
//! rather than about a rounding detail. The kernels are written to make both impossible
//! (see `knn::metric`), and the WebAssembly core specification requires the same
//! correctly-rounded IEEE-754 results this target does. This file is where that stops
//! being an argument and becomes an executed test.
//!
//! # How it runs on both
//!
//! One test body, two attributes. Natively it is an ordinary `#[test]` picked up by
//! `cargo test --workspace`. On `wasm32-unknown-unknown` it is a `#[wasm_bindgen_test]`,
//! compiled to wasm and executed in Node by `make wasm-test` (and by CI's wasm job):
//!
//! ```text
//! cargo test -p purrdf-sparql-eval --target wasm32-unknown-unknown --test knn_wasm_determinism
//! ```
//!
//! Both runs assert the *same* literal expectations — the ones written below. A target
//! that computes a different last bit renders a different `xsd:double` lexical and fails
//! here rather than surfacing as a mystery reordering in production.
//!
//! # Why the fixture looks like that
//!
//! The vectors are deliberately **not** exactly representable in binary64, and the metric
//! is cosine — the one kernel that adds a division and a square root to the dot-product
//! fold. Every component rounds, every product rounds, every partial sum rounds, and the
//! norms run through the scaled fold. A fixture of small integers would produce exact
//! arithmetic, agree on every target for free, and prove nothing about the hazard this
//! test exists for.
//!
//! The expected lexicals are `xsd:double` canonical forms, which round-trip the exact
//! bits: a rounded decimal rendering would make two adjacent doubles print alike and hide
//! precisely the divergence being watched for.

#![allow(clippy::doc_markdown, reason = "prose names targets, not items")]

use std::sync::Arc;

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec,
    RdfDatasetBuilder, RdfTermTarget, SparqlRequest, SparqlResult, StageImplementation, TargetSet,
    TermValue, VectorDtype,
};
use purrdf_sparql_eval::{
    EmbeddingKnnRelation, EmbeddingSpace, KnnGuard, NativeSparqlEngine, PropertyFunctionRegistry,
    QueryOptions,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// The fixture's data namespace.
const EX: &str = "https://example.org/d/";

/// The IRI this fixture host registers its space under. PurRDF mints none.
const SPACE_IRI: &str = "https://example.org/space/points";

/// The query: the five nearest neighbours of `d:v0`, with their distances.
const QUERY: &str = "PREFIX knn: <https://example.org/space/>\n\
                     PREFIX d: <https://example.org/d/>\n\
                     SELECT ?neighbour ?distance WHERE {\n\
                       ?neighbour knn:points ( d:v0 5 ?distance )\n\
                     }\n";

/// Six six-dimensional vectors whose every component is a binary64 *approximation* of the
/// decimal written, so no partial product or partial sum in the fold is exact.
///
/// `v5` carries a component eleven orders of magnitude away from its neighbours, which is
/// what puts the scaled norm fold — not the naive `sum(x²).sqrt()` — on the tested path.
fn vectors() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("v0", vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6]),
        ("v1", vec![0.7, 0.1, 0.9, 0.3, 0.11, 0.13]),
        ("v2", vec![-0.31, 0.62, -0.17, 0.41, 0.29, -0.53]),
        ("v3", vec![1e16, 1.0, -1e16, 0.7, 0.21, 3.3]),
        ("v4", vec![0.101, 0.199, 0.302, 0.397, 0.511, 0.607]),
        ("v5", vec![1e-11, 7.7, 0.037, -2.9, 1e11, 0.4]),
    ]
}

/// The rows the query must answer with, on every target: `(neighbour, distance lexical)`.
///
/// Pinned literally rather than recomputed, because a test that recomputes its expectation
/// with the same kernel it is testing agrees with any kernel at all.
const EXPECTED: [(&str, &str); 5] = [
    ("v0", "0.0E0"),
    ("v4", "5.170924590081061E-5"),
    ("v1", "4.6244406150896744E-1"),
    ("v5", "4.758575816324252E-1"),
    ("v2", "9.661190786522847E-1"),
];

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

/// Encode the fixture as a sealed PURREMB artifact and open it as a queryable space.
fn space() -> EmbeddingSpace {
    let rows = vectors();
    let dimension = u32::try_from(rows[0].1.len()).expect("small");
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");

    let mut targets = Vec::with_capacity(rows.len());
    let mut bindings = Vec::with_capacity(rows.len());
    for (local, _) in &rows {
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
        metric: DistanceMetric::Cosine,
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

/// Answer [`QUERY`] as `(neighbour local name, distance lexical)` pairs.
fn answer() -> Vec<(String, String)> {
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(
        SPACE_IRI,
        Arc::new(EmbeddingKnnRelation::new(Arc::new(space()))),
    );

    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}unrelated"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let o = builder.intern_iri(&format!("{EX}o"));
    builder.push_quad(s, p, o, None);
    let dataset = builder.freeze().expect("freeze fixture");

    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &dataset,
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: &relations,
                ..QueryOptions::EMPTY
            },
        )
        .expect("the call resolves and evaluates");

    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT returns solutions")
    };
    rows.iter()
        .map(|row| {
            let Some(TermValue::Iri(text)) = row[0].as_ref() else {
                panic!("the neighbour is an IRI")
            };
            let Some(TermValue::Literal {
                lexical_form,
                datatype,
                ..
            }) = row[1].as_ref()
            else {
                panic!("the distance is a typed literal")
            };
            assert_eq!(
                datatype, "http://www.w3.org/2001/XMLSchema#double",
                "the distance must stay an xsd:double, whose canonical form carries the \
                 exact bits"
            );
            (
                text.strip_prefix(EX).expect("fixture IRI").to_owned(),
                lexical_form.clone(),
            )
        })
        .collect()
}

/// The pinned answer, asserted row for row and bit for bit — on whichever target is
/// executing this.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_pinned_answer_is_reproduced_on_this_target() {
    let rows = answer();

    // The short-bag guard first: five neighbours were asked for and six exist, so five
    // must come back. `>=` would be satisfied by returning all six and `<=` by returning
    // none.
    assert_eq!(
        rows.len(),
        EXPECTED.len(),
        "the query asked for exactly {} neighbours",
        EXPECTED.len()
    );

    for (at, ((neighbour, distance), (want_neighbour, want_distance))) in
        rows.iter().zip(EXPECTED.iter()).enumerate()
    {
        assert_eq!(
            (neighbour.as_str(), distance.as_str()),
            (*want_neighbour, *want_distance),
            "rank {at} differs from the pinned cross-target answer; a divergence here is \
             the reassociation/FMA hazard the kernels are written to exclude, not a \
             rounding detail"
        );
    }

    // And the whole answer as one string, so a reordering that preserved every pair
    // individually would still be caught.
    let rendered: Vec<String> = rows
        .iter()
        .map(|(neighbour, distance)| format!("{neighbour}={distance}"))
        .collect();
    assert_eq!(
        rendered.join("|"),
        EXPECTED
            .iter()
            .map(|(neighbour, distance)| format!("{neighbour}={distance}"))
            .collect::<Vec<_>>()
            .join("|")
    );
}
