// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The embedding kNN surface, from the vantage a host has: a real PURREMB artifact built
//! by `purrdf_core`'s own writer, read back through its own reader, and searched.
//!
//! Nothing here fabricates a vector table in memory and calls it an embedding space. The
//! fixtures below encode, seal and fully verify an artifact for every case, because half
//! of what this surface has to get right is *reading the artifact correctly* — which
//! metric it declares, which rows its target set numbers, which matrix its guards bind —
//! and a hand-built table would test none of it.

use std::sync::Arc;

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DerivedIndex, DimensionalityPolicy, DistanceMetric,
    EmbeddingBuilder, EmbeddingFamilyContract, IndexBuildDeterminism, IndexCoordinates,
    IndexGuardContract, IndexLossContract, IndexPayloadStorage, IndexUseRole, MatrixInput,
    MatrixRow, PrefixPostprocessing, ProjectionSpec, RdfDatasetBuilder, RdfTermTarget,
    StageImplementation, TargetSet, VectorDtype,
};

use super::*;

/// The fixture namespace. PurRDF mints no IRIs; these are the example vocabulary the
/// repository's fixtures use.
const EX: &str = "https://example.org/d/";

/// A built artifact together with everything a host needs to open it as a space.
struct Fixture {
    /// The encoded PURREMB bytes.
    artifact: Vec<u8>,
    /// The target set the vectors are indexed by.
    target_set: TargetSetId,
    /// The vector space the matrix projects into.
    vector_space: VectorSpaceId,
    /// Each target's RDF term, in the order the caller supplied its rows.
    bindings: Vec<(TargetId, TermValue)>,
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

/// An IRI term in the fixture namespace.
fn iri(local: &str) -> TermValue {
    TermValue::iri(format!("{EX}{local}"))
}

/// Build an `f64` PURREMB artifact holding one row per `(local name, vector)` pair.
///
/// The rows are handed to the builder in the order given; the builder sorts them by
/// `TargetId`, which is exactly the canonical numbering this surface's tie-break relies
/// on. Several tests below feed the same rows in two different orders on purpose.
fn fixture(metric: &DistanceMetric, rows: &[(&str, Vec<f64>)]) -> Fixture {
    fixture_with_indexes(metric, rows, Vec::new())
}

/// [`fixture`], with `indexes` declared as derived-index guards over the artifact.
fn fixture_with_indexes(
    metric: &DistanceMetric,
    rows: &[(&str, Vec<f64>)],
    indexes: Vec<DerivedIndex>,
) -> Fixture {
    let dimension = u32::try_from(rows.first().map_or(0, |(_, v)| v.len())).expect("small");
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");

    let mut targets = Vec::with_capacity(rows.len());
    let mut bindings = Vec::with_capacity(rows.len());
    for (local, _) in rows {
        let term = iri(local);
        let TermValue::Iri(text) = &term else {
            unreachable!("fixture terms are IRIs")
        };
        let target = RdfTermTarget::Iri(text.clone())
            .into_target(true, None)
            .expect("term target");
        bindings.push((target.id, term));
        targets.push(target);
    }
    let set = TargetSet::new(targets.iter().map(|t| t.id).collect()).expect("target set");

    // The source pack's own dataset target is declared too — PURREMB requires the
    // artifact to name the dataset it is bound to — but it is deliberately NOT a row of
    // the searched target set, which is what makes the coverage check below a check of
    // the SET rather than of every target the artifact happens to hold.
    let mut declared = targets;
    declared.push(source.dataset_target(true).expect("dataset target"));
    // The writer looks targets up by binary search, so the declaration list is sorted by
    // id — the same canonical order `TargetSet` itself imposes on its rows.
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
        metric: metric.clone(),
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
        indexes,
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f64_matrix(matrix);
    let encoded = builder.build().expect("encoded artifact");

    Fixture {
        artifact: encoded.bytes,
        target_set: set.id,
        vector_space,
        bindings,
    }
}

impl Fixture {
    /// Open this fixture as a space under `guard`.
    fn open(&self, guard: KnnGuard) -> Result<EmbeddingSpace, EvalError> {
        EmbeddingSpace::from_artifact(
            &self.artifact,
            self.target_set,
            self.vector_space,
            self.bindings.clone(),
            guard,
        )
    }
}

/// A guard admitting a hundred candidates and ten neighbours — comfortably above every
/// fixture below, so a test that is not about the guard is never about the guard.
fn roomy() -> KnnGuard {
    KnnGuard::new(100, 10).expect("positive bounds")
}

/// The three-point fixture the ranking tests share: `a` at the origin-ish, `b` near it,
/// `c` far.
fn points() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("a", vec![0.0, 0.0]),
        ("b", vec![3.0, 4.0]),
        ("c", vec![30.0, 40.0]),
    ]
}

/// Open a space over `rows` under `metric`, with a roomy guard.
fn space(metric: &DistanceMetric, rows: &[(&str, Vec<f64>)]) -> EmbeddingSpace {
    fixture(metric, rows).open(roomy()).expect("space opens")
}

/// Invoke `relation` with the given per-position bindings and drain the cursor.
fn invoke(
    relation: &EmbeddingKnnRelation,
    bound: &[Option<TermValue>],
    ceiling: Option<u64>,
) -> Result<Vec<PfRow>, EvalError> {
    let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
    let (subject, object) = refs.split_at(relation.arity().subject);
    let args = PfArgs::new(subject, object);
    let mut cursor = relation.open(&args, ceiling)?;
    let mut out = Vec::new();
    while let Some(row) = cursor.next()? {
        out.push(row);
    }
    Ok(out)
}

/// An `xsd:integer` literal.
fn count(k: i64) -> TermValue {
    TermValue::typed_literal(k.to_string(), "http://www.w3.org/2001/XMLSchema#integer")
}

/// The `(neighbour local name, distance lexical)` pairs a result carries.
fn named(rows: &[PfRow]) -> Vec<(String, String)> {
    rows.iter()
        .map(|row| {
            let TermValue::Iri(neighbour) = &row[KNN_NEIGHBOUR] else {
                panic!("the neighbour position carries an IRI, got {:?}", row[0])
            };
            let TermValue::Literal { lexical_form, .. } = &row[KNN_DISTANCE] else {
                panic!("the distance position carries a literal")
            };
            (
                neighbour.strip_prefix(EX).expect("fixture IRI").to_owned(),
                lexical_form.clone(),
            )
        })
        .collect()
}

/// Just the neighbour names, in emission order.
fn order(rows: &[PfRow]) -> Vec<String> {
    named(rows).into_iter().map(|(name, _)| name).collect()
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

#[test]
fn a_space_reads_the_metric_dimension_and_rows_the_artifact_declares() {
    let space = space(&DistanceMetric::SquaredEuclidean, &points());
    assert_eq!(space.metric(), &DistanceMetric::SquaredEuclidean);
    assert_eq!(space.dimension(), 2);
    assert_eq!(space.row_count(), 3);
    assert_eq!(space.guard(), roomy());
    for local in ["a", "b", "c"] {
        assert!(
            space.row_of(&iri(local)).is_some(),
            "{local} must be reachable by term"
        );
    }
    assert_eq!(
        space.row_of(&iri("absent")),
        None,
        "a term the space does not hold has no row"
    );
}

#[test]
fn rows_are_numbered_by_canonical_target_order_not_by_the_order_they_were_supplied() {
    // The tie-break's foundation. A PURREMB target set is sorted and deduplicated by
    // `TargetId` — a digest of canonical identity — so two hosts that supply the same
    // rows in different orders get the SAME numbering, and "break ties by row" is
    // therefore a statement about content rather than about insertion order.
    let forward = points();
    let mut backward = points();
    backward.reverse();

    let a = space(&DistanceMetric::SquaredEuclidean, &forward);
    let b = space(&DistanceMetric::SquaredEuclidean, &backward);

    let numbering = |s: &EmbeddingSpace| -> Vec<TermValue> {
        (0..s.row_count())
            .map(|row| s.term(row).expect("row is named").clone())
            .collect()
    };
    assert_eq!(
        numbering(&a),
        numbering(&b),
        "the two spaces must number their rows identically"
    );

    // And the answers agree, which is the property the numbering exists to give.
    let relation_a = EmbeddingKnnRelation::new(Arc::new(a));
    let relation_b = EmbeddingKnnRelation::new(Arc::new(b));
    let ask = |r: &EmbeddingKnnRelation| {
        invoke(r, &[None, Some(iri("a")), Some(count(3)), None], None).expect("search")
    };
    assert_eq!(named(&ask(&relation_a)), named(&ask(&relation_b)));
}

#[test]
fn an_extension_metric_is_refused_and_every_built_in_metric_is_admitted() {
    let extension = DistanceMetric::Extension {
        identifier: "https://example.org/my-metric".to_owned(),
        parameter_encoding: "application/cbor".to_owned(),
        parameters: vec![7],
    };
    let error = fixture(&extension, &points())
        .open(roomy())
        .expect_err("an opaque metric names a rule this engine cannot evaluate");
    assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
    assert!(
        error.to_string().contains("caller-defined distance metric"),
        "got {error}"
    );

    // The neighbouring VALID cases: all three built-ins open. A refusal that also
    // rejected these would be an over-refusal nothing else in this file would catch.
    for metric in [
        DistanceMetric::Cosine,
        DistanceMetric::NegativeDot,
        DistanceMetric::SquaredEuclidean,
    ] {
        // Cosine cannot rank a zero-norm vector, so the shared `points()` fixture (whose
        // first row is the origin) is replaced for it by an all-nonzero one.
        let rows = vec![
            ("a", vec![1.0, 1.0]),
            ("b", vec![3.0, 4.0]),
            ("c", vec![30.0, 40.0]),
        ];
        let opened = fixture(&metric, &rows).open(roomy());
        assert!(
            opened.is_ok(),
            "{metric:?} is a built-in PURREMB metric and must open: {opened:?}"
        );
    }
}

#[test]
fn a_space_larger_than_the_candidate_bound_is_refused_and_one_exactly_at_it_is_not() {
    let rows = points();
    let exactly = KnnGuard::new(3, 10).expect("positive");
    assert!(
        fixture(&DistanceMetric::SquaredEuclidean, &rows)
            .open(exactly)
            .is_ok(),
        "a space of exactly the admitted size must open — the bound is inclusive"
    );

    let one_short = KnnGuard::new(2, 10).expect("positive");
    let error = fixture(&DistanceMetric::SquaredEuclidean, &rows)
        .open(one_short)
        .expect_err("three rows exceed a two-candidate bound");
    assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
    assert!(error.to_string().contains("3 row(s)"), "got {error}");
    assert!(error.to_string().contains("2 candidate(s)"), "got {error}");
}

#[test]
fn a_zero_guard_bound_is_refused_in_either_position() {
    for (candidates, neighbours) in [(0, 5), (5, 0), (0, 0)] {
        let error = KnnGuard::new(candidates, neighbours)
            .expect_err("a bound of zero admits nothing at all");
        assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
    }
    // The neighbouring VALID case: one of each is a legitimate, if tiny, configuration.
    assert!(KnnGuard::new(1, 1).is_ok());
}

#[test]
fn a_row_with_no_bound_term_is_refused_and_a_full_cover_is_not() {
    let built = fixture(&DistanceMetric::SquaredEuclidean, &points());
    assert!(built.open(roomy()).is_ok(), "the full cover opens");

    let mut short = built.bindings.clone();
    short.pop();
    let error = EmbeddingSpace::from_artifact(
        &built.artifact,
        built.target_set,
        built.vector_space,
        short,
        roomy(),
    )
    .expect_err("an unnamed row would search silently and report partially");
    assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
    assert!(
        error.to_string().contains("has no bound RDF term"),
        "got {error}"
    );
}

#[test]
fn a_duplicate_term_and_a_duplicate_target_are_both_refused() {
    let built = fixture(&DistanceMetric::SquaredEuclidean, &points());

    // Two rows claiming one term: the query seed would be ambiguous.
    let mut duplicated_term = built.bindings.clone();
    duplicated_term[1].1 = duplicated_term[0].1.clone();
    let error = EmbeddingSpace::from_artifact(
        &built.artifact,
        built.target_set,
        built.vector_space,
        duplicated_term,
        roomy(),
    )
    .expect_err("one term cannot name two rows");
    assert!(
        error.to_string().contains("bound to two different rows"),
        "got {error}"
    );

    // One target bound twice: the second binding would silently overwrite the first, and
    // some other row would be left uncovered.
    let mut duplicated_target = built.bindings.clone();
    duplicated_target[1].0 = duplicated_target[0].0;
    let error = EmbeddingSpace::from_artifact(
        &built.artifact,
        built.target_set,
        built.vector_space,
        duplicated_target,
        roomy(),
    )
    .expect_err("one row stands for exactly one term");
    assert!(error.to_string().contains("is bound twice"), "got {error}");
}

#[test]
fn a_binding_for_a_target_the_space_does_not_hold_is_refused() {
    let built = fixture(&DistanceMetric::SquaredEuclidean, &points());
    let stranger = RdfTermTarget::Iri(format!("{EX}stranger"))
        .into_target(true, None)
        .expect("term target");
    let mut extra = built.bindings.clone();
    extra.push((stranger.id, iri("stranger")));
    let error = EmbeddingSpace::from_artifact(
        &built.artifact,
        built.target_set,
        built.vector_space,
        extra,
        roomy(),
    )
    .expect_err("a binding for an absent target names nothing");
    assert!(
        error.to_string().contains("does not\nhold") || error.to_string().contains("does not"),
        "got {error}"
    );
}

#[test]
fn an_unknown_target_set_or_vector_space_is_refused() {
    let built = fixture(&DistanceMetric::SquaredEuclidean, &points());
    let nowhere_set = TargetSetId::from_raw([9_u8; 32]);
    let nowhere_space = VectorSpaceId::from_raw([9_u8; 32]);

    let error = EmbeddingSpace::from_artifact(
        &built.artifact,
        nowhere_set,
        built.vector_space,
        built.bindings.clone(),
        roomy(),
    )
    .expect_err("a target set the artifact does not declare");
    assert!(matches!(error, EvalError::Config(_)), "got {error:?}");

    let error = EmbeddingSpace::from_artifact(
        &built.artifact,
        built.target_set,
        nowhere_space,
        built.bindings.clone(),
        roomy(),
    )
    .expect_err("a vector space the artifact does not declare");
    assert!(matches!(error, EvalError::Config(_)), "got {error:?}");

    // The neighbouring VALID case, so the two refusals above are about the identifiers
    // and not about the artifact being unopenable for some unrelated reason.
    assert!(built.open(roomy()).is_ok());
}

#[test]
fn a_zero_norm_vector_is_refused_under_cosine_and_admitted_under_the_metrics_that_do_not_divide() {
    // PURREMB v1: "Cosine distance is undefined for a zero-norm operand and hard-fails
    // rather than inventing a score." The refusal therefore belongs to the METRIC, not to
    // the vector — and this is the control that proves it, because the SAME artifact
    // shape opens fine under the two kernels that never divide by a norm.
    let with_origin = vec![
        ("a", vec![0.0, 0.0]),
        ("b", vec![3.0, 4.0]),
        ("c", vec![30.0, 40.0]),
    ];

    let error = fixture(&DistanceMetric::Cosine, &with_origin)
        .open(roomy())
        .expect_err("a zero-norm vector has no direction");
    assert!(matches!(error, EvalError::Data(_)), "got {error:?}");
    assert!(error.to_string().contains("zero L2 norm"), "got {error}");

    for metric in [
        DistanceMetric::SquaredEuclidean,
        DistanceMetric::NegativeDot,
    ] {
        assert!(
            fixture(&metric, &with_origin).open(roomy()).is_ok(),
            "{metric:?} never divides by a norm, so the origin is an ordinary point"
        );
    }

    // And the neighbouring valid case for cosine itself: a vector that is very small but
    // NOT zero still has a direction, and must not be caught by the same check.
    let tiny = vec![
        ("a", vec![f64::MIN_POSITIVE, 0.0]),
        ("b", vec![3.0, 4.0]),
        ("c", vec![30.0, 40.0]),
    ];
    assert!(
        fixture(&DistanceMetric::Cosine, &tiny)
            .open(roomy())
            .is_ok(),
        "a subnormal component still gives a non-zero norm through the scaled fold"
    );
}

#[test]
fn a_corrupted_artifact_is_a_data_error_rather_than_a_panic() {
    let mut built = fixture(&DistanceMetric::SquaredEuclidean, &points());
    let at = built.artifact.len() / 2;
    built.artifact[at] ^= 0xFF;
    let error = built
        .open(roomy())
        .expect_err("a flipped byte must be caught by PURREMB's own sealing");
    assert!(matches!(error, EvalError::Data(_)), "got {error:?}");
}

// ---------------------------------------------------------------------------
// Ranking
// ---------------------------------------------------------------------------

#[test]
fn neighbours_come_back_in_rank_order_with_exact_rows() {
    // `a` = (0,0), `b` = (3,4), `c` = (30,40). Squared Euclidean from `a`: 0, 25, 2500.
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let rows = invoke(
        &relation,
        &[None, Some(iri("a")), Some(count(3)), None],
        None,
    )
    .expect("search");
    assert_eq!(
        named(&rows),
        vec![
            ("a".to_owned(), "0.0E0".to_owned()),
            ("b".to_owned(), "2.5E1".to_owned()),
            ("c".to_owned(), "2.5E3".to_owned()),
        ],
        "exact rows: over-generation is as much a bug as under-generation"
    );

    // Every row echoes its two input positions, which is what lets the engine's equality
    // filter and a repeated variable at the call site work at all.
    for row in &rows {
        assert_eq!(row.len(), 4);
        assert_eq!(row[KNN_QUERY], iri("a"));
        assert_eq!(row[KNN_COUNT], count(3));
    }
}

#[test]
fn cosine_and_squared_euclidean_rank_the_same_vectors_in_opposite_orders() {
    // THE test that the declared metric is respected rather than assumed. The two rows
    // are chosen so the two metrics genuinely disagree:
    //
    //   query `q` = (1, 0)
    //   `far`     = (2, 0)      — same direction, cosine distance 0; L2 distance^2 = 1
    //   `near`    = (0.9, 0.9)  — 45 degrees off, cosine distance ~0.293; L2^2 = 0.82
    //
    // so squared Euclidean puts `near` first and cosine puts `far` first. A surface that
    // ignored the declaration and always used one kernel would fail exactly one of these.
    let rows = vec![
        ("q", vec![1.0, 0.0]),
        ("far", vec![2.0, 0.0]),
        ("near", vec![0.9, 0.9]),
    ];
    let euclidean =
        EmbeddingKnnRelation::new(Arc::new(space(&DistanceMetric::SquaredEuclidean, &rows)));
    let cosine = EmbeddingKnnRelation::new(Arc::new(space(&DistanceMetric::Cosine, &rows)));

    let ask = |r: &EmbeddingKnnRelation| {
        order(&invoke(r, &[None, Some(iri("q")), Some(count(3)), None], None).expect("search"))
    };
    let by_position = ask(&euclidean);
    let by_direction = ask(&cosine);
    assert_eq!(
        by_position,
        vec!["q", "near", "far"],
        "squared Euclidean ranks by position"
    );
    // Under cosine, `far` is at distance EXACTLY zero from `q` (same direction), as is
    // `q` itself, so those two tie and the canonical row order separates them. What the
    // metric decides — and the whole point of the fixture — is the relative order of
    // `near` and `far`, which is the reverse of the Euclidean one.
    let position_of =
        |rank: &[String], name: &str| rank.iter().position(|n| n == name).expect("present");
    assert!(
        position_of(&by_position, "near") < position_of(&by_position, "far"),
        "squared Euclidean puts the closer point first: {by_position:?}"
    );
    assert!(
        position_of(&by_direction, "far") < position_of(&by_direction, "near"),
        "cosine puts the aligned point first — the OPPOSITE order: {by_direction:?}"
    );
}

#[test]
fn negative_dot_ranks_by_descending_similarity() {
    // `-dot` is smallest for the largest dot product, so the largest projection onto the
    // query ranks first. `q`=(1,0): dot(q,big)=10, dot(q,q)=1, dot(q,away)=-5.
    let rows = vec![
        ("q", vec![1.0, 0.0]),
        ("big", vec![10.0, 0.0]),
        ("away", vec![-5.0, 0.0]),
    ];
    let relation = EmbeddingKnnRelation::new(Arc::new(space(&DistanceMetric::NegativeDot, &rows)));
    let rows = invoke(
        &relation,
        &[None, Some(iri("q")), Some(count(3)), None],
        None,
    )
    .expect("search");
    assert_eq!(
        named(&rows),
        vec![
            ("big".to_owned(), "-1.0E1".to_owned()),
            ("q".to_owned(), "-1.0E0".to_owned()),
            ("away".to_owned(), "5.0E0".to_owned()),
        ]
    );
}

#[test]
fn equal_distances_break_by_canonical_row_order_and_never_by_chance() {
    // Two rows with IDENTICAL vectors are genuinely equidistant from everything. The
    // order between them is therefore decided entirely by the tie-break, and it must be
    // the same whichever order the artifact's rows were supplied in.
    let forward = vec![
        ("q", vec![1.0, 0.0]),
        ("twin1", vec![5.0, 0.0]),
        ("twin2", vec![5.0, 0.0]),
    ];
    let mut backward = forward.clone();
    backward.reverse();

    let ask = |rows: &[(&str, Vec<f64>)]| {
        let relation =
            EmbeddingKnnRelation::new(Arc::new(space(&DistanceMetric::SquaredEuclidean, rows)));
        order(
            &invoke(
                &relation,
                &[None, Some(iri("q")), Some(count(3)), None],
                None,
            )
            .expect("search"),
        )
    };
    let one = ask(&forward);
    let other = ask(&backward);
    assert_eq!(
        one, other,
        "the tie-break is content order, not input order"
    );
    assert_eq!(one[0], "q", "the seed is its own nearest neighbour");
    assert_eq!(
        one.len(),
        3,
        "both twins are returned; a tie is not a reason to drop one"
    );
}

#[test]
fn a_term_is_its_own_nearest_neighbour_under_the_two_metrics_that_promise_it_and_not_the_third() {
    // A distance metric puts a point at its own minimum; a SIMILARITY score does not have
    // to. `-dot` is unnormalized, so a vector pointing the same way but ten times longer
    // has ten times the dot product and therefore a *smaller* `-dot` than the seed's own.
    //
    // That asymmetry is exactly what a surface which quietly used one kernel for
    // everything would get wrong, so it is asserted in both directions rather than
    // averaged into a claim that holds for two metrics out of three.
    let rows = vec![
        ("q", vec![1.0, 2.0]),
        ("b", vec![3.0, 4.0]),
        ("c", vec![30.0, 40.0]),
    ];
    for metric in [DistanceMetric::SquaredEuclidean, DistanceMetric::Cosine] {
        let relation = EmbeddingKnnRelation::new(Arc::new(space(&metric, &rows)));
        let found = order(
            &invoke(
                &relation,
                &[None, Some(iri("q")), Some(count(1)), None],
                None,
            )
            .expect("search"),
        );
        assert_eq!(found, vec!["q"], "under {metric:?}");
    }

    let relation = EmbeddingKnnRelation::new(Arc::new(space(&DistanceMetric::NegativeDot, &rows)));
    assert_eq!(
        order(
            &invoke(
                &relation,
                &[None, Some(iri("q")), Some(count(1)), None],
                None
            )
            .expect("search")
        ),
        vec!["c"],
        "`-dot` is a similarity, not a distance: the longest vector in the seed's own \
         direction wins, and the seed does not"
    );
}

#[test]
fn k_rows_come_back_exactly_and_every_prefix_is_the_prefix_of_the_full_ranking() {
    // The short-bag guard. `>= k` is satisfied by returning everything and `<= k` by
    // returning nothing, so the count is asserted EXACTLY, and the rows are compared
    // against the full ranking's prefix rather than merely counted.
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let full = order(
        &invoke(
            &relation,
            &[None, Some(iri("a")), Some(count(3)), None],
            None,
        )
        .expect("search"),
    );
    assert_eq!(full.len(), 3);

    for k in 0..=3_i64 {
        let rows = invoke(
            &relation,
            &[None, Some(iri("a")), Some(count(k)), None],
            None,
        )
        .expect("search");
        assert_eq!(
            rows.len(),
            usize::try_from(k).expect("small"),
            "k = {k} must yield EXACTLY k rows"
        );
        assert_eq!(
            order(&rows),
            full[..usize::try_from(k).expect("small")].to_vec(),
            "and they must be the FIRST k of the full ranking, in that order"
        );
    }

    // Asking for more neighbours than the space holds yields every row it holds — not a
    // padded bag, and not an error.
    let all = invoke(
        &relation,
        &[None, Some(iri("a")), Some(count(10)), None],
        None,
    )
    .expect("search");
    assert_eq!(order(&all), full);
}

// ---------------------------------------------------------------------------
// Refusals, each with its neighbouring valid case
// ---------------------------------------------------------------------------

#[test]
fn a_free_query_or_count_is_refused_and_the_bound_forms_are_not() {
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));

    let error = invoke(&relation, &[None, None, Some(count(2)), None], None)
        .expect_err("this relation cannot enumerate seeds");
    assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
    assert!(error.to_string().contains("is free"), "got {error}");

    let error = invoke(&relation, &[None, Some(iri("a")), None, None], None)
        .expect_err("this relation cannot invent how many neighbours to return");
    assert!(matches!(error, EvalError::Function(_)), "got {error:?}");

    // The neighbouring VALID case.
    assert_eq!(
        invoke(
            &relation,
            &[None, Some(iri("a")), Some(count(2)), None],
            None
        )
        .expect("search")
        .len(),
        2
    );
}

#[test]
fn a_non_integer_count_is_refused_and_a_derived_integer_type_is_not() {
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));

    for bad in [
        TermValue::typed_literal("2.5", "http://www.w3.org/2001/XMLSchema#decimal"),
        TermValue::typed_literal("two", "http://www.w3.org/2001/XMLSchema#string"),
        iri("a"),
    ] {
        let error = invoke(
            &relation,
            &[None, Some(iri("a")), Some(bad.clone()), None],
            None,
        )
        .expect_err("there is no number of neighbours that names");
        assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
    }

    // The over-refusal control. `xsd:int` and `xsd:unsignedByte` are integer-family
    // datatypes; a check that only admitted the literal string `xsd:integer` would refuse
    // a query that is perfectly well formed, and every test above would still pass.
    for good in [
        TermValue::typed_literal("2", "http://www.w3.org/2001/XMLSchema#int"),
        TermValue::typed_literal("2", "http://www.w3.org/2001/XMLSchema#long"),
        TermValue::typed_literal("2", "http://www.w3.org/2001/XMLSchema#unsignedByte"),
        TermValue::typed_literal("2", "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"),
    ] {
        let rows = invoke(
            &relation,
            &[None, Some(iri("a")), Some(good.clone()), None],
            None,
        )
        .unwrap_or_else(|e| panic!("{good:?} is an integer literal and must be accepted: {e}"));
        assert_eq!(rows.len(), 2);
    }
}

#[test]
fn a_negative_count_is_refused_and_zero_is_an_honest_empty_answer() {
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));

    let error = invoke(
        &relation,
        &[None, Some(iri("a")), Some(count(-1)), None],
        None,
    )
    .expect_err("a search cannot return a negative number of neighbours");
    assert!(matches!(error, EvalError::Function(_)), "got {error:?}");

    // Zero is the boundary a clamp-or-refuse rule gets wrong in both directions: it is a
    // well-formed request for nothing, answered with nothing.
    assert_eq!(
        invoke(
            &relation,
            &[None, Some(iri("a")), Some(count(0)), None],
            None
        )
        .expect("k = 0 is a valid request"),
        [] as [Vec<TermValue>; 0]
    );
}

#[test]
fn a_count_above_the_guard_is_refused_and_one_exactly_at_it_is_not() {
    let built = fixture(&DistanceMetric::SquaredEuclidean, &points());
    let guard = KnnGuard::new(100, 2).expect("positive");
    let relation = EmbeddingKnnRelation::new(Arc::new(built.open(guard).expect("space")));

    let rows = invoke(
        &relation,
        &[None, Some(iri("a")), Some(count(2)), None],
        None,
    )
    .expect("k exactly at the guard is admitted — the bound is inclusive");
    assert_eq!(rows.len(), 2);

    let error = invoke(
        &relation,
        &[None, Some(iri("a")), Some(count(3)), None],
        None,
    )
    .expect_err("k one past the guard is refused, not clamped");
    assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
    assert!(error.to_string().contains("asks for 3"), "got {error}");
    assert!(error.to_string().contains("at most 2"), "got {error}");
}

#[test]
fn a_seed_the_space_does_not_hold_is_empty_rather_than_an_error() {
    // The over-refusal control that matters most in practice: a query ranging a seed over
    // terms only some of which are embedded must not abort. This is a well-formed
    // question the data does not answer, exactly as an unmatched triple pattern is.
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    assert_eq!(
        invoke(
            &relation,
            &[None, Some(iri("stranger")), Some(count(3)), None],
            None
        )
        .expect("an unembedded seed is a question, not a mistake"),
        [] as [Vec<TermValue>; 0]
    );
    // And the neighbouring case still answers, so the emptiness above is about the seed.
    assert_eq!(
        invoke(
            &relation,
            &[None, Some(iri("a")), Some(count(3)), None],
            None
        )
        .expect("search")
        .len(),
        3
    );
}

#[test]
fn a_wrong_argument_count_is_refused_before_the_search() {
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let subject: [Option<&TermValue>; 0] = [];
    let query = iri("a");
    let k = count(1);
    let object = [Some(&query), Some(&k), None];
    let args = PfArgs::new(&subject, &object);
    let Err(error) = relation.open(&args, None) else {
        panic!("a zero-argument subject side does not match the declaration")
    };
    assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
    assert!(error.to_string().contains("expects"), "got {error}");
}

// ---------------------------------------------------------------------------
// The declared shape and its row bound
// ---------------------------------------------------------------------------

#[test]
fn the_declared_shape_is_the_documented_one() {
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    assert_eq!(relation.arity(), PfArity::new(1, 3));
    assert_eq!(relation.volatility(), Volatility::Stable);
    assert_eq!(
        relation
            .modes()
            .iter()
            .map(|mode| mode.code())
            .collect::<Vec<_>>(),
        vec![KNN_MODE.to_owned()]
    );

    // Feasibility: everything binding the seed and the count is admitted, nothing else.
    for admitted in ["fbbf", "bbbf", "fbbb", "bbbb"] {
        assert!(
            relation.admits(BindingPattern::from_code(admitted)),
            "{admitted} binds both inputs and must be admitted"
        );
    }
    for refused in ["ffff", "bfbf", "fbff", "bbff", "ffbf", "fbfb"] {
        assert!(
            !relation.admits(BindingPattern::from_code(refused)),
            "{refused} leaves an input free and cannot be served"
        );
    }
}

#[test]
fn the_row_bound_table_is_the_documented_one() {
    // Three rows, a guard admitting two neighbours: the two branches of the table are
    // distinguishable, and neither is accidentally equal to the row count.
    let built = fixture(&DistanceMetric::SquaredEuclidean, &points());
    let relation = EmbeddingKnnRelation::new(Arc::new(
        built
            .open(KnnGuard::new(100, 2).expect("positive"))
            .expect("space"),
    ));
    let declared = |code: &str| relation.rows_per_invocation(BindingPattern::from_code(code));

    assert_eq!(declared("fbbf"), 2, "min(max_neighbours = 2, rows = 3)");
    assert_eq!(declared("fbbb"), 2, "a bound distance restricts nothing");
    assert_eq!(
        declared("bbbf"),
        1,
        "a bound neighbour names at most one row"
    );
    assert_eq!(declared("bbbb"), 1);

    // Measured, not assumed. The same relation over a ONE-row space declares 1 for the
    // unbound-neighbour mode even though its guard admits ten, because the bound is the
    // minimum of two independently valid limits and here the space is the tighter one.
    // A table that hard-coded `max_neighbours` would say 10 and be wrong.
    let single = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &[("only", vec![1.0, 1.0])],
    )));
    assert_eq!(single.space().guard().max_neighbours(), 10);
    assert_eq!(
        single.rows_per_invocation(BindingPattern::from_code("fbbf")),
        1,
        "min(max_neighbours = 10, rows = 1) is the ROW count"
    );
    assert_eq!(
        single.rows_per_invocation(BindingPattern::from_code("bbbf")),
        1
    );
}

#[test]
fn rows_per_invocation_is_an_upper_bound_that_is_respected_and_attained() {
    // Two properties, and the second is the one an `<=` assertion cannot check:
    //
    // (a) every admitted invocation emits at most the declared number of rows;
    // (b) where the table declares a number, some invocation actually EMITS that many.
    //     A bound nothing attains is not a bound, it is a guess — and `u64::MAX` would
    //     satisfy (a) for every relation ever written.
    // Two candidates deliberately EQUIDISTANT from the seed, so that binding `?distance`
    // — the position the table says restricts nothing — can genuinely retain more than
    // one row. On a fixture whose distances were all distinct, `fbbb` would emit one row
    // for the accidental reason that the fixture has no ties, and the table's claim would
    // go unexercised while the assertion still passed.
    let rows = vec![
        ("seed", vec![0.0, 0.0]),
        ("left", vec![-1.0, 0.0]),
        ("right", vec![1.0, 0.0]),
    ];
    let relation = EmbeddingKnnRelation::new(Arc::new(
        fixture(&DistanceMetric::SquaredEuclidean, &rows)
            .open(KnnGuard::new(100, 3).expect("positive"))
            .expect("space"),
    ));

    // The rows the unbounded answer produces, so bound values are drawn from reality
    // rather than from what this test hopes reality looks like.
    let answer = invoke(
        &relation,
        &[None, Some(iri("seed")), Some(count(3)), None],
        None,
    )
    .expect("search");
    assert_eq!(answer.len(), 3);
    let tied = answer[1].clone();
    assert_eq!(
        answer[2][KNN_DISTANCE], tied[KNN_DISTANCE],
        "the fixture's two outer points must genuinely tie, or `fbbb` proves nothing"
    );

    let cases: [(&str, [Option<TermValue>; 4], u64); 4] = [
        ("fbbf", [None, Some(iri("seed")), Some(count(3)), None], 3),
        (
            "fbbb",
            [
                None,
                Some(iri("seed")),
                Some(count(3)),
                Some(tied[KNN_DISTANCE].clone()),
            ],
            2,
        ),
        (
            "bbbf",
            [
                Some(tied[KNN_NEIGHBOUR].clone()),
                Some(iri("seed")),
                Some(count(3)),
                None,
            ],
            1,
        ),
        (
            "bbbb",
            [
                Some(tied[KNN_NEIGHBOUR].clone()),
                Some(iri("seed")),
                Some(count(3)),
                Some(tied[KNN_DISTANCE].clone()),
            ],
            1,
        ),
    ];

    for (code, bound, expected) in cases {
        let declared = relation.rows_per_invocation(BindingPattern::from_code(code));
        let emitted =
            u64::try_from(invoke(&relation, &bound, None).expect("search").len()).expect("small");
        assert!(
            emitted <= declared,
            "mode {code} emitted {emitted} rows against a declared bound of {declared}"
        );
        assert_eq!(
            emitted, expected,
            "mode {code} must emit exactly {expected} rows on this fixture"
        );
        if expected == declared {
            continue;
        }
        // The only mode whose declared bound this fixture does not attain in one call is
        // `fbbb`: a bound distance restricts nothing IN GENERAL (arbitrarily many rows
        // can tie), so the honest declaration is the unbound one, and the two rows above
        // are what show the declaration is not merely `1` in disguise.
        assert_eq!(code, "fbbb");
        assert!(emitted > 1, "and it must exceed the trivial single row");
    }

    // Attainment for the mode that claims EXACTLY one: a bound neighbour, emitting one.
    // `<=` alone is satisfied by declaring `u64::MAX`, so a declared `1` that no
    // invocation reaches is a guess rather than a bound.
    assert_eq!(
        relation.rows_per_invocation(BindingPattern::from_code("bbbf")),
        1
    );
}

// ---------------------------------------------------------------------------
// The row ceiling and the reported work
// ---------------------------------------------------------------------------

/// Open `relation` for the roomy fixture and hand back its cursor.
fn cursor_for(
    relation: &EmbeddingKnnRelation,
    bound: &[Option<TermValue>],
    ceiling: Option<u64>,
) -> Box<dyn PfCursor> {
    let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
    let (subject, object) = refs.split_at(relation.arity().subject);
    let args = PfArgs::new(subject, object);
    relation.open(&args, ceiling).expect("open")
}

#[test]
fn a_ceiling_yields_the_prefix_of_the_unbounded_answer_at_every_k() {
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let bound = [None, Some(iri("a")), Some(count(3)), None];
    let full = order(&invoke(&relation, &bound, None).expect("search"));

    for ceiling in 0..=full.len() + 2 {
        let capped = order(
            &invoke(&relation, &bound, Some(ceiling as u64)).expect("search under a ceiling"),
        );
        assert_eq!(
            capped,
            full[..ceiling.min(full.len())].to_vec(),
            "a ceiling of {ceiling} must yield the FIRST {ceiling} rows, row for row"
        );
    }
}

#[test]
fn a_zero_ceiling_does_no_work_at_all() {
    // The reason the search is lazy rather than eager in `open`: a call whose ceiling is
    // already spent must not pay for a scan nobody will read.
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let mut cursor = cursor_for(
        &relation,
        &[None, Some(iri("a")), Some(count(3)), None],
        Some(0),
    );
    assert!(cursor.next().expect("no error").is_none());
    assert_eq!(
        cursor.take_work(),
        0,
        "no rows were wanted, so no candidate was examined"
    );
}

#[test]
fn the_reported_work_is_the_candidate_count_and_is_taken_exactly_once() {
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let mut cursor = cursor_for(
        &relation,
        &[None, Some(iri("a")), Some(count(1)), None],
        None,
    );

    assert_eq!(
        cursor.take_work(),
        0,
        "nothing has been searched before the first pull"
    );
    assert!(cursor.next().expect("no error").is_some());
    assert_eq!(
        cursor.take_work(),
        3,
        "one distance computation per row of the space — and note the invocation returns \
         ONE row, which is exactly the gap between rows and work this channel exists for"
    );
    assert_eq!(
        cursor.take_work(),
        0,
        "the work is TAKEN: reporting it twice would charge one search to the caller twice"
    );
    assert!(cursor.next().expect("no error").is_none());
    assert_eq!(cursor.take_work(), 0, "and the terminating pull adds none");
}

#[test]
fn an_unembedded_seed_reports_no_work() {
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let mut cursor = cursor_for(
        &relation,
        &[None, Some(iri("stranger")), Some(count(3)), None],
        None,
    );
    assert!(cursor.next().expect("no error").is_none());
    assert_eq!(cursor.take_work(), 0, "no seed, no distances");
}

#[test]
fn a_ceiling_counts_emitted_rows_not_skipped_ones() {
    // A bound `?neighbour` filters AFTER the ranking. A ceiling of one must therefore
    // still produce the one matching row, rather than spending the licence on the nearer
    // rows the filter discards and then reporting exhaustion.
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let rows = invoke(
        &relation,
        &[Some(iri("c")), Some(iri("a")), Some(count(3)), None],
        Some(1),
    )
    .expect("search");
    assert_eq!(
        order(&rows),
        vec!["c"],
        "the two nearer rows are filtered, not charged against the licence"
    );
}

#[test]
fn a_ceiling_is_withheld_from_the_selection_when_a_post_selection_position_is_bound() {
    // The subtle one. `?neighbour` and `?distance` are filtered after the ranking, so
    // shrinking the RANKING to the ceiling would let the cursor filter a prefix and
    // report exhaustion with fewer rows than the engine asked for — a short bag read as a
    // complete answer.
    //
    // Here the engine offers a ceiling of 1 and the call binds `?neighbour` to the row
    // that ranks THIRD. Pushing the ceiling into the selection would rank only `a` and
    // find nothing; withholding it ranks all three and finds `c`.
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::SquaredEuclidean,
        &points(),
    )));
    let rows = invoke(
        &relation,
        &[Some(iri("c")), Some(iri("a")), Some(count(3)), None],
        Some(1),
    )
    .expect("search");
    assert_eq!(order(&rows), vec!["c"]);

    // The same for a bound distance.
    let far = TermValue::typed_literal("2.5E3", "http://www.w3.org/2001/XMLSchema#double");
    let rows = invoke(
        &relation,
        &[None, Some(iri("a")), Some(count(3)), Some(far)],
        Some(1),
    )
    .expect("search");
    assert_eq!(order(&rows), vec!["c"]);
}

#[test]
fn emission_is_a_pure_function_of_the_invocation() {
    // Repeated invocations of one relation, and of two independently built spaces over
    // the same content, must agree exactly — the determinism claim, asserted on the rows
    // a caller actually sees.
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::Cosine,
        &[
            ("q", vec![1.0, 2.0, 3.0]),
            ("b", vec![3.0, 2.0, 1.0]),
            ("c", vec![-1.0, 0.5, 2.0]),
        ],
    )));
    let bound = [None, Some(iri("q")), Some(count(3)), None];
    let first = named(&invoke(&relation, &bound, None).expect("search"));
    for _ in 0..8 {
        assert_eq!(
            named(&invoke(&relation, &bound, None).expect("search")),
            first
        );
    }

    let rebuilt = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::Cosine,
        &[
            ("c", vec![-1.0, 0.5, 2.0]),
            ("q", vec![1.0, 2.0, 3.0]),
            ("b", vec![3.0, 2.0, 1.0]),
        ],
    )));
    assert_eq!(
        named(&invoke(&rebuilt, &bound, None).expect("search")),
        first,
        "an independently built artifact over the same content answers identically, \
         distance lexicals included"
    );
}

#[test]
fn distances_are_emitted_as_exact_xsd_double_lexicals() {
    // The score is the metric's value, carried in a datatype that round-trips its bits —
    // never a rounded decimal, which would make two adjacent doubles print alike and hide
    // exactly the divergence this surface's determinism claim is about.
    let relation = EmbeddingKnnRelation::new(Arc::new(space(
        &DistanceMetric::Cosine,
        &[("q", vec![3.0, 4.0]), ("b", vec![0.0, 5.0])],
    )));
    let rows = invoke(
        &relation,
        &[None, Some(iri("q")), Some(count(2)), None],
        None,
    )
    .expect("search");
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = &rows[1][KNN_DISTANCE]
    else {
        panic!("the distance is a typed literal")
    };
    assert_eq!(datatype, XSD_DOUBLE);
    // 1 - 20/(5*5) = 1 - 0.8, one rounding per operation.
    assert_eq!(
        lexical_form,
        &purrdf_xsd::numeric::canonical_double(1.0_f64 - 0.8_f64)
    );
    assert_eq!(
        lexical_form
            .parse::<f64>()
            .expect("canonical double round-trips")
            .to_bits(),
        (1.0_f64 - 0.8_f64).to_bits(),
        "the lexical form must carry the exact bits, not a rounded rendering of them"
    );
}

// ---------------------------------------------------------------------------
// Derived-index guards
// ---------------------------------------------------------------------------

#[test]
fn a_derived_index_guard_naming_this_space_is_checked_and_a_matching_one_is_admitted() {
    // PURREMB stores an opaque third-party ANN payload behind a guard that binds it to
    // the exact coordinates it was built over. PurRDF cannot run the payload, but a
    // reader can check the binding — and this is the OVER-REFUSAL side of that check: a
    // guard that agrees with the matrix being scanned must not stop the space opening,
    // and must not change a single answer.
    //
    // The coordinates a guard needs (`MatrixId`, `ProjectionId`) are derived by the
    // writer, so the fixture is built twice: once to learn them, once to declare a guard
    // over them.
    let rows = points();
    let plain = fixture(&DistanceMetric::SquaredEuclidean, &rows);
    let view = EmbeddingView::from_bytes(&plain.artifact).expect("structural view");
    let effective = view
        .effective_matrix(plain.target_set, plain.vector_space)
        .expect("readable")
        .expect("the fixture joins its set to its space");
    let space_view = view.vector_space(plain.vector_space).expect("space");

    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let coordinates = IndexCoordinates {
        source_exact_digest: source.source_exact_digest(),
        family_id: space_view.family_id(),
        vector_space_id: plain.vector_space,
        matrix_id: effective.matrix().id(),
        projection_id: effective.projection().id(),
        target_set_id: plain.target_set,
        prefix_dimension: space_view.dimension(),
    };
    let guard_contract = IndexGuardContract {
        implementation: identity("ann-implementation"),
        parameter_encoding: "application/cbor".to_owned(),
        parameters: vec![0xA0],
        loss: IndexLossContract {
            transforms_vectors: false,
            loss_encoding: None,
            loss_parameters: None,
        },
        use_role: IndexUseRole::CoarsePrefixRetrieval,
        payload_media_type: "application/octet-stream".to_owned(),
        certified_metadata_binding: None,
    };
    let index = DerivedIndex::new(
        coordinates,
        IndexPayloadStorage::Inline(vec![1, 2, 3, 4]),
        IndexBuildDeterminism::Deterministic,
        &guard_contract,
    )
    .expect("an internally consistent derived index");

    let guarded = fixture_with_indexes(&DistanceMetric::SquaredEuclidean, &rows, vec![index]);
    let guarded_view = EmbeddingView::from_bytes(&guarded.artifact).expect("structural view");
    assert_eq!(
        guarded_view.index_guard_count(),
        1,
        "the fixture must genuinely carry a guard, or this test watches nothing"
    );

    let space = guarded
        .open(roomy())
        .expect("a guard that agrees with the scanned matrix must not stop the space opening");
    assert_eq!(space.row_count(), 3);

    // And the answers are byte-identical to the unguarded artifact's: a guard is metadata
    // about an index PurRDF does not run, so it cannot move a single row.
    let ask = |s: EmbeddingSpace| {
        let relation = EmbeddingKnnRelation::new(Arc::new(s));
        named(
            &invoke(
                &relation,
                &[None, Some(iri("a")), Some(count(3)), None],
                None,
            )
            .expect("search"),
        )
    };
    assert_eq!(ask(space), ask(plain.open(roomy()).expect("space")));
}

// ---------------------------------------------------------------------------
// The two artifact shapes the reader branches on
// ---------------------------------------------------------------------------

/// Build a PURREMB artifact over `rows` with a chosen scalar type and a chosen set of
/// effective prefixes, and open the prefix at `queried` as a space.
///
/// The fixtures above all take one path through the reader: `f64` scalars, one projection,
/// effective dimension equal to stored dimension. PURREMB permits three more combinations
/// and `EmbeddingSpace` reads all of them — a matrix stored as `f32` (which is what a real
/// embedding model emits), and a Matryoshka space whose rows are a *shorter, separately
/// postprocessed leading prefix* of the stored ones. Neither is a variation on the tested
/// path; each is a different branch, and an untested branch in a reader is a wrong answer
/// waiting for the first artifact that takes it.
fn prefixed_fixture(
    metric: &DistanceMetric,
    rows: &[(&str, Vec<f64>)],
    dtype: VectorDtype,
    prefixes: &[(u32, PrefixPostprocessing)],
    queried: usize,
) -> Fixture {
    let stored = u32::try_from(rows.first().map_or(0, |(_, v)| v.len())).expect("small");
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");

    let mut targets = Vec::with_capacity(rows.len());
    let mut bindings = Vec::with_capacity(rows.len());
    for (local, _) in rows {
        let term = iri(local);
        let TermValue::Iri(text) = &term else {
            unreachable!("fixture terms are IRIs")
        };
        let target = RdfTermTarget::Iri(text.clone())
            .into_target(true, None)
            .expect("term target");
        bindings.push((target.id, term));
        targets.push(target);
    }
    let set = TargetSet::new(targets.iter().map(|t| t.id).collect()).expect("target set");
    let mut declared = targets;
    declared.push(source.dataset_target(true).expect("dataset target"));
    declared.sort_unstable_by_key(|target| target.id);

    let effective: Vec<purrdf_core::EffectivePrefix> = prefixes
        .iter()
        .map(
            |&(dimension, postprocessing)| purrdf_core::EffectivePrefix {
                dimension,
                postprocessing,
            },
        )
        .collect();
    let dimensionality = if effective.len() == 1 {
        DimensionalityPolicy::fixed(effective[0].dimension, effective[0].postprocessing)
            .expect("fixed dimensions")
    } else {
        DimensionalityPolicy::matryoshka(effective.clone()).expect("matryoshka dimensions")
    };

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
        dtype,
        metric: metric.clone(),
        dimensionality,
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family");
    let projections: Vec<ProjectionSpec> = effective
        .iter()
        .map(|prefix| ProjectionSpec::derive(family.id, prefix.dimension, prefix.postprocessing))
        .collect();
    let vector_space = projections[queried].vector_space_id;

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
    match dtype {
        VectorDtype::F32 => builder.add_f32_matrix(MatrixInput {
            family_id: family.id,
            target_set_id: set.id,
            stored_dimension: stored,
            rows: rows
                .iter()
                .zip(&bindings)
                .map(|((_, values), (target, _))| {
                    // Every fixture value below is exactly representable in binary32, so
                    // this narrowing is exact and the f32 and f64 artifacts hold the same
                    // real numbers — which is what makes comparing their ANSWERS a test of
                    // the reader rather than of the cast.
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "the fixture values are exactly representable in binary32"
                    )]
                    MatrixRow::new(*target, values.iter().map(|&v| v as f32).collect())
                })
                .collect(),
            projections,
        }),
        VectorDtype::F64 => builder.add_f64_matrix(MatrixInput {
            family_id: family.id,
            target_set_id: set.id,
            stored_dimension: stored,
            rows: rows
                .iter()
                .zip(&bindings)
                .map(|((_, values), (target, _))| MatrixRow::new(*target, values.clone()))
                .collect(),
            projections,
        }),
    };
    let encoded = builder.build().expect("encoded artifact");

    Fixture {
        artifact: encoded.bytes,
        target_set: set.id,
        vector_space,
        bindings,
    }
}

#[test]
fn an_f32_matrix_answers_identically_to_the_f64_matrix_holding_the_same_numbers() {
    // `read_vectors` branches on the matrix's declared scalar type, and every other
    // fixture in this file takes the `f64` arm — while a real embedding model emits `f32`,
    // so the untested arm is the one production uses. The two artifacts below hold the
    // same real numbers (each value is exactly representable in binary32) and are read
    // through different decoders, so an answer that differs is a decoder defect: a wrong
    // stride, a dropped component, a lossy widening.
    let rows = points();
    let one_over_two_pow_ten = 1.0_f64 / 1024.0;
    let mut rows = rows;
    // A fractional component with an exact binary32 representation, so the comparison
    // exercises a mantissa rather than only integers a byte-swap could not disturb.
    rows.push(("d", vec![one_over_two_pow_ten, 0.5]));

    let as_f64 = prefixed_fixture(
        &DistanceMetric::SquaredEuclidean,
        &rows,
        VectorDtype::F64,
        &[(2, PrefixPostprocessing::None)],
        0,
    )
    .open(roomy())
    .expect("the f64 space opens");
    let as_f32 = prefixed_fixture(
        &DistanceMetric::SquaredEuclidean,
        &rows,
        VectorDtype::F32,
        &[(2, PrefixPostprocessing::None)],
        0,
    )
    .open(roomy())
    .expect("the f32 space opens");

    assert_eq!(as_f32.dimension(), 2);
    assert_eq!(as_f32.row_count(), rows.len());

    let bound = [None, Some(iri("a")), Some(count(4)), None];
    let from_f64 =
        named(&invoke(&EmbeddingKnnRelation::new(Arc::new(as_f64)), &bound, None).expect("search"));
    let from_f32 =
        named(&invoke(&EmbeddingKnnRelation::new(Arc::new(as_f32)), &bound, None).expect("search"));
    assert_eq!(from_f32.len(), 4, "all four rows, not a short bag");
    assert_eq!(
        from_f32, from_f64,
        "the f32 decoder must yield the same neighbours AND the same distance lexicals: \
         widening a binary32 to binary64 is exact, so there is nothing here for the two \
         paths to legitimately disagree about"
    );
}

#[test]
fn a_matryoshka_space_is_searched_over_its_effective_prefix_not_its_stored_row() {
    // A Matryoshka artifact stores full rows and declares shorter, separately
    // postprocessed leading prefixes as their own vector spaces. `EmbeddingSpace` takes
    // its dimension from the space it was asked for and reads the EFFECTIVE row, so a
    // search over the 2-prefix must rank by the first two components after that prefix's
    // deterministic L2 normalization — not by the stored four.
    //
    // The fixture makes the two answers incompatible rather than merely different: `near`
    // is almost parallel to the seed in the first two components and enormous in the last
    // two, while `far` is orthogonal in the first two and small everywhere. Ranked over
    // the 2-prefix the order is (seed, near, far); ranked over the stored row it is
    // (seed, far, near). Only one of those can be emitted.
    let rows = vec![
        ("seed", vec![1.0, 0.0, 0.0, 0.0]),
        ("near", vec![1.0, 0.015_625, 50.0, 0.0]),
        ("far", vec![0.0, 1.0, 0.0, 0.0]),
    ];
    let fixture = prefixed_fixture(
        &DistanceMetric::SquaredEuclidean,
        &rows,
        VectorDtype::F64,
        &[
            (2, PrefixPostprocessing::DeterministicL2),
            (4, PrefixPostprocessing::None),
        ],
        0,
    );
    let space = fixture.open(roomy()).expect("the 2-prefix space opens");
    assert_eq!(
        space.dimension(),
        2,
        "the space's dimension is the EFFECTIVE one; a reader that took the stored \
         dimension would read four components per row and run off the end of the matrix"
    );

    let relation = EmbeddingKnnRelation::new(Arc::new(space));
    let emitted = order(
        &invoke(
            &relation,
            &[None, Some(iri("seed")), Some(count(3)), None],
            None,
        )
        .expect("search"),
    );
    assert_eq!(
        emitted,
        vec!["seed".to_owned(), "near".to_owned(), "far".to_owned()],
        "the ranking is over the normalized 2-prefix; over the stored four-component row \
         `far` would come second"
    );

    // The control, so the claim above is a claim about the PROJECTION rather than about
    // this fixture: the same artifact's full-dimension space ranks the same three rows the
    // other way round.
    let full = prefixed_fixture(
        &DistanceMetric::SquaredEuclidean,
        &rows,
        VectorDtype::F64,
        &[
            (2, PrefixPostprocessing::DeterministicL2),
            (4, PrefixPostprocessing::None),
        ],
        1,
    )
    .open(roomy())
    .expect("the full space opens");
    assert_eq!(full.dimension(), 4);
    let over_stored = order(
        &invoke(
            &EmbeddingKnnRelation::new(Arc::new(full)),
            &[None, Some(iri("seed")), Some(count(3)), None],
            None,
        )
        .expect("search"),
    );
    assert_eq!(
        over_stored,
        vec!["seed".to_owned(), "far".to_owned(), "near".to_owned()],
        "and the two spaces of ONE artifact genuinely disagree, which is what makes the \
         assertion above a test"
    );
}
