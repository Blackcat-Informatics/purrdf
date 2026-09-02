// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The embedding kNN surface, end to end, from the vantage a host has.
//!
//! One query text carried through every stage the surface touches — a real PURREMB
//! artifact encoded and sealed by `purrdf_core`'s own writer, a space opened over it, a
//! relation registered under the host's own IRI, a query parsed, planned, governed and
//! answered — and then the charge ledger that execution left behind. Nothing here reaches
//! into the crate's internals: a surface whose stages only line up from inside is a
//! surface a host cannot use.

use std::sync::Arc;

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec,
    RdfDataset, RdfDatasetBuilder, RdfTermTarget, SparqlRequest, SparqlResult, StageImplementation,
    TargetId, TargetSet, TargetSetId, TermValue, VectorDtype, VectorSpaceId,
};
use purrdf_sparql_eval::{
    ChargePoint, EmbeddingKnnRelation, EmbeddingSpace, GovernedOutcome, KnnGuard,
    NativeSparqlEngine, NodeCharges, PropertyFunctionRegistry, QueryGovernors, QueryOptions,
    ResourceDimension,
};

/// The data namespace of the fixture terms.
const EX: &str = "https://example.org/d/";

/// The IRI **this host** chose for its space. PurRDF mints none: without a registration
/// under this exact IRI the predicate below is an ordinary triple pattern.
const SPACE_IRI: &str = "https://example.org/space/points";

/// The acceptance query: the three nearest neighbours of `d:a`, with their distances.
const QUERY: &str = "PREFIX knn: <https://example.org/space/>\n\
                     PREFIX d: <https://example.org/d/>\n\
                     SELECT ?neighbour ?distance WHERE {\n\
                       ?neighbour knn:points ( d:a 3 ?distance )\n\
                     }\n";

// ---------------------------------------------------------------------------
// The fixture artifact
// ---------------------------------------------------------------------------

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

/// Encode a sealed PURREMB artifact holding one `f64` row per `(local name, vector)`.
fn artifact(
    rows: &[(&str, Vec<f64>)],
) -> (
    Vec<u8>,
    TargetSetId,
    VectorSpaceId,
    Vec<(TargetId, TermValue)>,
) {
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
        metric: DistanceMetric::SquaredEuclidean,
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
    (encoded.bytes, set.id, vector_space, bindings)
}

/// The three points every test below searches: `a` at the origin, `b` at distance 25,
/// `c` at distance 2500.
fn points() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("a", vec![0.0, 0.0]),
        ("b", vec![3.0, 4.0]),
        ("c", vec![30.0, 40.0]),
    ]
}

/// Open `rows` as a space under a guard admitting ten candidates and five neighbours.
fn space(rows: &[(&str, Vec<f64>)]) -> EmbeddingSpace {
    let (bytes, target_set, vector_space, bindings) = artifact(rows);
    EmbeddingSpace::from_artifact(
        &bytes,
        target_set,
        vector_space,
        bindings,
        KnnGuard::new(10, 5).expect("positive bounds"),
    )
    .expect("the space opens")
}

/// The whole of what a host does: register a relation over a space under its own IRI.
fn registry(rows: &[(&str, Vec<f64>)]) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        SPACE_IRI,
        Arc::new(EmbeddingKnnRelation::new(Arc::new(space(rows)))),
    );
    registry
}

/// A dataset holding one unrelated triple: the answers come from the embedding space, and
/// this is what makes that observable rather than merely stated.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}unrelated"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let o = builder.intern_iri(&format!("{EX}o"));
    builder.push_quad(s, p, o, None);
    builder.freeze().expect("freeze fixture")
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn with_relations(relations: &PropertyFunctionRegistry) -> QueryOptions<'_> {
    QueryOptions {
        property_functions: relations,
        ..QueryOptions::EMPTY
    }
}

/// Render a solution result as `[[neighbour local name, distance lexical], ..]`.
fn rows_of(result: &SparqlResult) -> Vec<Vec<String>> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT returns solutions");
    };
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell.as_ref().expect("every cell is bound") {
                    TermValue::Iri(text) => text.strip_prefix(EX).expect("fixture IRI").to_owned(),
                    TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
                    other => panic!("unexpected cell {other:?}"),
                })
                .collect()
        })
        .collect()
}

/// Answer `query` against `relations`, ungoverned.
fn answer(query: &str, relations: &PropertyFunctionRegistry) -> SparqlResult {
    NativeSparqlEngine::new()
        .query_with_options_view(&*dataset(), request(query), with_relations(relations))
        .expect("the call resolves and evaluates")
}

// ---------------------------------------------------------------------------
// The acceptance path
// ---------------------------------------------------------------------------

#[test]
fn a_registered_space_answers_a_knn_query_in_rank_order() {
    let result = answer(QUERY, &registry(&points()));
    let SparqlResult::Solutions { variables, .. } = &result else {
        panic!("a SELECT returns solutions");
    };
    assert_eq!(
        variables,
        &vec!["neighbour".to_owned(), "distance".to_owned()]
    );
    assert_eq!(
        rows_of(&result),
        vec![
            vec!["a".to_owned(), "0.0E0".to_owned()],
            vec!["b".to_owned(), "2.5E1".to_owned()],
            vec!["c".to_owned(), "2.5E3".to_owned()],
        ],
        "exact rows in rank order: over-generation is as much a bug as under-generation, \
         so this is an equality rather than a containment"
    );
}

#[test]
fn an_iri_that_is_not_registered_is_an_ordinary_triple_pattern() {
    // The over-refusal control for the vocabulary itself. PurRDF reserves no namespace, so
    // a predicate under `knn:` that the host did NOT register must read the graph exactly
    // as any other predicate does — no error, no interception, and here no rows, because
    // the fixture graph holds no such triple.
    let query = "PREFIX knn: <https://example.org/space/>\n\
                 PREFIX d: <https://example.org/d/>\n\
                 SELECT ?x WHERE { d:a knn:unregistered ?x }\n";
    let result = answer(query, &registry(&points()));
    assert!(
        rows_of(&result).is_empty(),
        "an unregistered predicate is data, not a relation"
    );
}

#[test]
fn a_neighbour_joins_back_to_the_graph_by_basic_graph_pattern() {
    // The retrieved term is a real RDF term, not an opaque handle: it unifies with the
    // dataset. This is the whole reason the surface returns terms rather than row numbers.
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri(&format!("{EX}b"));
    let predicate = builder.intern_iri(&format!("{EX}label"));
    let object = builder.intern_iri(&format!("{EX}beta"));
    builder.push_quad(subject, predicate, object, None);
    let graph = builder.freeze().expect("freeze");

    let query = "PREFIX knn: <https://example.org/space/>\n\
                 PREFIX d: <https://example.org/d/>\n\
                 SELECT ?label WHERE {\n\
                   ?neighbour knn:points ( d:a 3 ?distance ) .\n\
                   ?neighbour d:label ?label\n\
                 }\n";
    let relations = registry(&points());
    let result = NativeSparqlEngine::new()
        .query_with_options_view(&*graph, request(query), with_relations(&relations))
        .expect("evaluate");
    assert_eq!(rows_of(&result), vec![vec!["beta".to_owned()]]);
}

#[test]
fn a_limit_yields_the_prefix_of_the_unlimited_answer_at_every_k() {
    // The ceiling pushdown, observed where a caller can see it. `LIMIT n` must return the
    // FIRST n rows of the unlimited answer — row for row, not merely n rows of it. A
    // relation that stopped early against a ceiling it could not honour would show up
    // here as a different row set, not merely a shorter one.
    let relations = registry(&points());
    let full = rows_of(&answer(QUERY, &relations));
    assert_eq!(full.len(), 3);

    for limit in 0..=full.len() + 1 {
        let limited = format!("{QUERY}LIMIT {limit}\n");
        assert_eq!(
            rows_of(&answer(&limited, &relations)),
            full[..limit.min(full.len())].to_vec(),
            "LIMIT {limit} must be the first {limit} rows of the unlimited answer"
        );
    }
}

#[test]
fn the_answer_is_byte_identical_across_two_independently_built_artifacts() {
    // The determinism claim, asserted on the bytes a consumer actually receives, over two
    // artifacts built from the same content in OPPOSITE row orders. Distances included:
    // an `xsd:double` lexical carries the exact bits, so an accumulation-order or
    // fused-multiply-add divergence would show up as a different string here.
    let forward = registry(&points());
    let mut reversed_rows = points();
    reversed_rows.reverse();
    let reversed = registry(&reversed_rows);

    let render = |relations: &PropertyFunctionRegistry| {
        purrdf_sparql_results::to_json(
            &answer(QUERY, relations),
            &purrdf_sparql_results::ResultProvenance::default(),
            None,
        )
        .expect("serialize")
        .bytes
    };
    assert_eq!(
        render(&forward),
        render(&reversed),
        "two independently built artifacts over the same content must answer byte for byte"
    );

    // And across repeated runs on ONE engine, so the plan cache is exercised rather than
    // avoided.
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let once = engine
        .query_with_options_view(&*data, request(QUERY), with_relations(&forward))
        .expect("evaluate");
    let baseline = purrdf_sparql_results::to_json(
        &once,
        &purrdf_sparql_results::ResultProvenance::default(),
        None,
    )
    .expect("serialize")
    .bytes;
    for _ in 0..16 {
        let again = engine
            .query_with_options_view(&*data, request(QUERY), with_relations(&forward))
            .expect("evaluate");
        assert_eq!(
            purrdf_sparql_results::to_json(
                &again,
                &purrdf_sparql_results::ResultProvenance::default(),
                None,
            )
            .expect("serialize")
            .bytes,
            baseline
        );
    }
}

// ---------------------------------------------------------------------------
// The governor
// ---------------------------------------------------------------------------

#[test]
fn the_governor_charges_the_search_in_proportion_to_the_candidates_it_examined() {
    // The acceptance criterion, stated as the inequality that motivates it: the query
    // returns THREE rows and the search examined THREE candidates here, so the interesting
    // number is not the equality but the fact that the two are independent — the next test
    // holds the row count fixed at one and watches the work stay at three.
    let relations = registry(&points());
    let engine = NativeSparqlEngine::new();

    let outcome = engine
        .query_governed(
            &dataset(),
            request(QUERY),
            with_relations(&relations),
            &QueryGovernors::METERED,
        )
        .expect("the call resolves and evaluates under governors");
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = outcome
    else {
        panic!("METERED bounds nothing, so this must complete");
    };
    assert!(evidence.is_complete());
    assert_eq!(rows_of(&result).len(), 3);

    let explanation = engine
        .explain_query_with_options(&dataset(), QUERY, None, with_relations(&relations))
        .expect("explain");
    let at = |point: ChargePoint| -> u64 {
        explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(point))
            .sum()
    };
    assert_eq!(at(ChargePoint::PropertyFunctionInvocation), 1);
    assert_eq!(at(ChargePoint::PropertyFunctionRow), 3);
    assert_eq!(
        at(ChargePoint::PropertyFunctionWork),
        3,
        "one distance computation per row of the space, charged once per unit"
    );

    // The decomposition prices the same execution the governed run spent, so the work
    // point is live on the governed lane and not merely on the explain one.
    assert_eq!(
        evidence.consumed.get(ResourceDimension::Fuel),
        explanation
            .ledger()
            .iter()
            .map(NodeCharges::fuel_total)
            .sum::<u64>(),
        "the governed entry and the explanation must price the same run"
    );
}

#[test]
fn the_search_charge_follows_the_space_size_rather_than_the_rows_returned() {
    // THE test for the acceptance criterion, and the one a flat per-row charge cannot
    // pass. Two queries return exactly ONE row each; the spaces they search differ in
    // size. Priced by rows they are identical. Priced by the work they actually did they
    // are not, and the difference is exactly the difference in candidates.
    let small = registry(&points());
    let mut larger_rows = points();
    for index in 0..5 {
        larger_rows.push((
            ["p", "q", "r", "s", "t"][index],
            vec![100.0 + index as f64, 0.0],
        ));
    }
    let large = registry(&larger_rows);

    let one_row = "PREFIX knn: <https://example.org/space/>\n\
                   PREFIX d: <https://example.org/d/>\n\
                   SELECT ?neighbour WHERE { ?neighbour knn:points ( d:a 1 ?distance ) }\n";

    let engine = NativeSparqlEngine::new();
    let measure = |relations: &PropertyFunctionRegistry| {
        let explanation = engine
            .explain_query_with_options(&dataset(), one_row, None, with_relations(relations))
            .expect("explain");
        let at = |point: ChargePoint| -> u64 {
            explanation
                .ledger()
                .iter()
                .map(|node| node.fuel_at(point))
                .sum()
        };
        (
            at(ChargePoint::PropertyFunctionRow),
            at(ChargePoint::PropertyFunctionWork),
        )
    };

    let (small_rows, small_work) = measure(&small);
    let (large_rows, large_work) = measure(&large);

    assert_eq!(
        small_rows, large_rows,
        "both queries return one row, so the row point cannot tell the two searches apart"
    );
    assert_eq!(small_rows, 1);
    assert_eq!(small_work, 3, "three candidates examined");
    assert_eq!(large_work, 8, "eight candidates examined");
    assert!(
        large_work > small_work,
        "and the work point does tell them apart, which is the whole acceptance criterion"
    );
}

#[test]
fn a_fuel_ceiling_one_unit_below_the_metered_spend_trips() {
    // The charge is SPENT, not merely recorded. Measured rather than typed in: a boundary
    // whose number was guessed tests whatever its author believed.
    let relations = registry(&points());
    let engine = NativeSparqlEngine::new();
    let measure = |governors: &QueryGovernors| {
        engine
            .query_governed(
                &dataset(),
                request(QUERY),
                with_relations(&relations),
                governors,
            )
            .expect("a governor trip is an outcome, never an error")
    };

    let GovernedOutcome::Complete { evidence, .. } = measure(&QueryGovernors::METERED) else {
        panic!("METERED bounds nothing");
    };
    let spend = evidence.consumed.get(ResourceDimension::Fuel);
    assert!(spend > 0, "a search is not free");

    assert!(
        matches!(
            measure(&QueryGovernors::UNBOUNDED.with_fuel(spend)),
            GovernedOutcome::Complete { .. }
        ),
        "the measured spend is exactly affordable — the neighbouring VALID case, without \
         which the refusal below would prove nothing"
    );
    assert!(
        matches!(
            measure(&QueryGovernors::UNBOUNDED.with_fuel(spend - 1)),
            GovernedOutcome::BudgetExhausted(_)
        ),
        "one unit less is not"
    );
}

#[test]
fn a_declared_row_bound_lets_a_cell_ceiling_admit_the_call_rather_than_refuse_it() {
    // The over-refusal control for the planner's admission check, which prices a
    // property-function node from its DECLARED row bound against the intermediate-cell
    // ceiling. This relation declares `min(max_neighbours, rows)` = 3 over 4 columns, so a
    // ceiling of 12 cells must ADMIT it. A relation that declared `u64::MAX` (the honest
    // declaration for a generator with no configured bound) would be refused here, and the
    // caller would see a budget-exhausted outcome for a query that produces three rows.
    let relations = registry(&points());
    let outcome = NativeSparqlEngine::new()
        .query_governed(
            &dataset(),
            request(QUERY),
            with_relations(&relations),
            &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(64),
        )
        .expect("evaluate");
    let GovernedOutcome::Complete { result, .. } = outcome else {
        panic!("a truthfully-declared bound must be admitted, not refused up front");
    };
    assert_eq!(rows_of(&result).len(), 3);
}

// ---------------------------------------------------------------------------
// The top-k is the TRUE top-k, at a size where it could quietly stop being one
// ---------------------------------------------------------------------------

/// 192 deterministic eight-dimensional points, generated by a fixed LCG so the fixture is
/// the same on every machine and every run.
///
/// The components are small integers, exactly representable in binary64, so the oracle
/// below computes the *same real numbers* the kernel does and the comparison is an
/// equality rather than a tolerance. The point of the fixture is scale and tie density,
/// not rounding — rounding is what `knn_wasm_determinism.rs` watches.
fn crowd() -> Vec<(String, Vec<f64>)> {
    let mut rows = Vec::with_capacity(192);
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    for i in 0..192_u32 {
        let mut vector = Vec::with_capacity(8);
        for _ in 0..8 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            vector.push(f64::from((state >> 33) as u32 % 32) - 16.0);
        }
        rows.push((format!("q{i:03}"), vector));
    }
    rows
}

#[test]
#[allow(
    clippy::suboptimal_flops,
    reason = "the oracle must mirror PURREMB's arithmetic — a separate rounded multiply \
              and add, never a fused multiply-add. Contracting it here would make the \
              oracle agree with a kernel that fused, which is the divergence the surface \
              is written to exclude"
)]
fn the_k_returned_are_the_true_k_nearest_of_a_crowded_space() {
    // Everything else in this file searches three points, where "the k nearest" and "the
    // k I happened to keep" are hard to tell apart: with three candidates a heap that
    // dropped the wrong one, or an ordering that was only accidentally right, still looks
    // plausible. This searches 192, against a brute-force oracle written out longhand
    // here rather than borrowed from the implementation — so the two agree because the
    // answer is right, not because they share a bug.
    //
    // Both halves are asserted: the COUNT is exact (a short bag of k-1 neighbours is
    // indistinguishable from success at a glance), and the CONTENT is compared row for
    // row against the oracle's prefix.
    let rows = crowd();
    let borrowed: Vec<(&str, Vec<f64>)> = rows
        .iter()
        .map(|(name, vector)| (name.as_str(), vector.clone()))
        .collect();

    let mut relations = PropertyFunctionRegistry::new();
    let (bytes, target_set, vector_space, bindings) = artifact(&borrowed);
    relations.register(
        SPACE_IRI,
        Arc::new(EmbeddingKnnRelation::new(Arc::new(
            EmbeddingSpace::from_artifact(
                &bytes,
                target_set,
                vector_space,
                bindings,
                KnnGuard::new(256, 32).expect("positive bounds"),
            )
            .expect("the space opens"),
        ))),
    );

    // The seed, and the oracle: every candidate scored by hand under the declared
    // squared-Euclidean metric, then fully sorted by (distance, name). Names order the
    // same way row numbers do here — the fixture's names are distinct and the tie-break
    // this asserts is over DISTANCE, which is what the metric decides.
    let (seed_name, seed) = &rows[7];
    let mut oracle: Vec<(f64, &str)> = rows
        .iter()
        .map(|(name, vector)| {
            let mut sum = 0.0_f64;
            for (x, y) in seed.iter().zip(vector.iter()) {
                let delta = x - y;
                sum += delta * delta;
            }
            (sum, name.as_str())
        })
        .collect();
    oracle.sort_by(|left, right| left.0.total_cmp(&right.0).then_with(|| left.1.cmp(right.1)));

    for k in [1_usize, 2, 7, 31, 32] {
        let query = format!(
            "PREFIX knn: <https://example.org/space/>\n\
             PREFIX d: <https://example.org/d/>\n\
             SELECT ?neighbour ?distance WHERE {{\n\
               ?neighbour knn:points ( d:{seed_name} {k} ?distance )\n\
             }}\n"
        );
        let got = rows_of(&answer(&query, &relations));
        assert_eq!(
            got.len(),
            k,
            "k = {k} of 192 candidates must yield EXACTLY k rows"
        );

        // The distances, in emission order, must be the oracle's first k distances — in
        // that order. This is the ranking claim: a heap that retained k good-but-not-best
        // candidates, or emitted them out of order, disagrees here.
        let emitted: Vec<f64> = got
            .iter()
            .map(|row| row[1].parse::<f64>().expect("an xsd:double lexical parses"))
            .collect();
        let expected: Vec<f64> = oracle[..k].iter().map(|(distance, _)| *distance).collect();
        assert_eq!(
            emitted, expected,
            "the {k} emitted distances must be the {k} smallest, in ascending order"
        );

        // And the names: equal distances are broken by row number, which is canonical
        // TargetId order rather than name order, so compare the SETS at the k-th
        // distance boundary. Every emitted neighbour must be one the oracle also places
        // within the tie-extended prefix — a neighbour from outside it is a wrong answer
        // even when its distance happens to match.
        let boundary = expected.last().copied().expect("k >= 1");
        let admissible: std::collections::BTreeSet<&str> = oracle
            .iter()
            .filter(|(distance, _)| *distance <= boundary)
            .map(|(_, name)| *name)
            .collect();
        for row in &got {
            assert!(
                admissible.contains(row[0].as_str()),
                "k = {k} emitted {} , which is not within the true k nearest",
                row[0]
            );
        }
        let distinct: std::collections::BTreeSet<&str> =
            got.iter().map(|row| row[0].as_str()).collect();
        assert_eq!(
            distinct.len(),
            k,
            "k = {k} must emit k DISTINCT neighbours, not one row repeated"
        );
    }
}
