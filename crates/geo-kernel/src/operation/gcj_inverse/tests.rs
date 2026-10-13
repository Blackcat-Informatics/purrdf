// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use crate::{
    CoordinateOperation, ExecutionLimits, ExecutionPolicy, GeoError, GeographicReference,
    MetricContext, OperationPoint,
    operation::{CoordinateUnit, OperationModel, OperationReference},
};
use purrdf_hash::hex::Digest32;

fn rat(text: &str) -> Rat {
    Rat::parse_decimal(text).expect("exact decimal")
}
fn bounds(west: &str, east: &str, south: &str, north: &str) -> Applicability {
    Applicability::new(rat(west), rat(east), rat(south), rat(north)).expect("explicit bounds")
}
fn forward(bounds: Applicability) -> CoordinateOperation {
    let reference = |id| OperationReference {
        realization: Digest32::new([id; 32]),
        unit: CoordinateUnit::Degrees,
        swapped_axes: false,
    };
    CoordinateOperation::compile(
        reference(1),
        reference(2),
        OperationModel::GcjRationalHarmonicV1(bounds),
    )
    .expect("compile")
}
fn point(longitude: &str, latitude: &str) -> OperationPoint {
    OperationPoint {
        x: rat(longitude),
        y: rat(latitude),
        z: None,
        epoch: None,
    }
}
fn raised_context() -> MetricContext {
    MetricContext::new(
        GeographicReference::wgs84(),
        ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 8_000_000,
            max_iterations: 256,
            max_subdivision_levels: 128,
            ..ExecutionLimits::GEOMETRY
        })
        .expect("policy"),
    )
    .expect("context")
}

#[test]
fn ordinary_inverse_is_global_and_certifies_its_quantized_forward_residual() {
    let operation = forward(bounds("72.004", "137.8347", "0.8293", "55.8271"));
    let source = point("116.397", "39.908");
    let mut context = MetricContext::wgs84().expect("context");
    let target = operation.apply(&source, &mut context).expect("forward");
    let inverse = operation.inverse().expect("explicit inverse");
    let restored = inverse
        .apply(target.point(), &mut context)
        .expect("global inverse");
    let tolerance = rat("0.000000000001");
    assert!(restored.point().x.sub(&source.x).abs() <= tolerance);
    assert!(restored.point().y.sub(&source.y).abs() <= tolerance);
    let replay = operation
        .apply(restored.point(), &mut context)
        .expect("forward replay");
    assert!(replay.point().x.sub(&target.point().x).abs() <= tolerance);
    assert!(replay.point().y.sub(&target.point().y).abs() <= tolerance);
    assert_ne!(inverse.law_id(), operation.law_id());
    assert_eq!(inverse.source(), operation.target());
    assert_eq!(inverse.target(), operation.source());
}

#[test]
fn original_cusp_turns_produce_ambiguity_despite_equal_rounded_candidates() {
    let operation = forward(bounds("104.9", "105.1", "34.9", "35.1"));
    // u=2e-7 lies before the left cusp chart's tiny positive turn. Its image
    // has two distinct left-chart preimages and an additional right preimage.
    let mut context = raised_context();
    let target = operation
        .apply(&point("104.99999999999996", "35"), &mut context)
        .expect("forward near cusp");
    let inverse = operation.inverse().expect("inverse");
    assert!(
        matches!(inverse.apply(target.point(), &mut context), Err(GeoError::AmbiguousTransform { roots }) if roots == 3)
    );
}

#[test]
fn caller_restriction_proves_uniqueness_and_outside_always_refuses() {
    let operation = forward(bounds("116.396", "116.398", "39.907", "39.909"));
    let mut context = MetricContext::wgs84().expect("context");
    let target = operation
        .apply(&point("116.397", "39.908"), &mut context)
        .expect("forward");
    let inverse = operation.inverse().expect("inverse");
    assert!(inverse.apply(target.point(), &mut context).is_ok());
    assert!(matches!(
        inverse.apply(&point("120", "40"), &mut context),
        Err(GeoError::Domain(_))
    ));
    assert!(operation.apply(&point("120", "40"), &mut context).is_err());
}
