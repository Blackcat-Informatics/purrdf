// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-free checks of private distance failures and frozen generation laws.

use super::*;

#[test]
fn computed_distance_refusals_retain_rows_metric_and_original_bits() {
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    let workspace = crate::WorkspaceCapability::resident();
    let guard = KnnGuard::new(100, 10).expect("positive bounds");
    let space = EmbeddingSpace::from_vectors(
        &DistanceMetric::SquaredEuclidean,
        1,
        vec![
            (TermValue::iri("https://example.org/d/a"), vec![1e300]),
            (TermValue::iri("https://example.org/d/b"), vec![-1e300]),
        ],
        guard,
    )
    .unwrap();
    let errors = [
        space
            .row_distance::<Exact>(None, 0, 1, &workspace)
            .unwrap_err(),
        space
            .search_admitted::<Exact>(None, 0, 2, &workspace)
            .unwrap_err(),
    ];
    drop(space);
    drop(workspace);
    for error in errors {
        let copy = error.clone();
        drop(error);
        let EvalError::RetainedNativeFailure(failure) = &copy else {
            panic!("the data refusal must retain its native context");
        };
        assert_eq!(failure.kind(), crate::NativeDiagnosticKind::Data);
        let fields = Capture::read(failure.cause());
        assert_eq!(fields.code, "knn-non-finite-distance");
        assert!(matches!(fields.field("query_row"), ErrorValue::Unsigned(0)));
        assert!(matches!(fields.field("row"), ErrorValue::Unsigned(1)));
        let native = std::error::Error::source(failure)
            .unwrap()
            .downcast_ref::<DistanceRefusal>()
            .unwrap();
        assert_eq!(native.metric, DistanceMetric::SquaredEuclidean);
        let cause = fields.child("cause");
        assert_eq!(cause.code, "distance-non-finite");
        assert!(matches!(cause.field("value"), ErrorValue::Float64Bits(bits)
            if bits == f64::INFINITY.to_bits()));
        assert!(std::error::Error::source(native).unwrap().is::<NonFinite>());
        assert_eq!(
            copy.to_string(),
            "malformed RDF input: the distance from row 0 to row 1 left the finite range under SquaredEuclidean; the artifact's magnitudes cannot be ranked under this metric"
        );
    }
}

#[test]
fn both_space_generations_are_frozen() {
    let terms = [
        TermValue::iri("https://example.org/d/a"),
        TermValue::iri("https://example.org/d/b"),
    ];
    let artifact = space_generation(
        ProjectionContentDigest::from_raw([0x11; 32]),
        FamilyContractDigest::from_raw([0x22; 32]),
        &terms,
    );
    assert_eq!(
        &*artifact,
        "f5023470280500dc109e1a2c1030bdd4847e77fdb00259a00e2f847e0def0159"
    );
    let vectors = vectors_generation(
        &DistanceMetric::SquaredEuclidean,
        2,
        &[0.5, -1.0, 2.0, 0.25],
        &terms,
    );
    assert_eq!(
        &*vectors,
        "f30b674454f5457a2b749cae0e92a9aa5b62d12f3e0b100cd9a8b45c17f8dd66"
    );
}
