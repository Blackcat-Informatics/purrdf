// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native source work refusal and required metadata precede coordinate copies.

use super::{
    CoordinateOperation, CoordinateUnit, HelmertParameters, OperationChain, OperationModel,
    OperationPoint, RotationConvention, RotationLaw, Similarity2d, epoch_fields_admitted,
    tests::reference,
};
use crate::context::WorkProgress;
use crate::{
    ExecutionLimits, ExecutionPolicy, GeoError, GeographicReference, Int, MetricContext, Rat,
};

#[test]
fn streamed_parameter_fields_keep_the_original_canonical_binding_preimage() {
    let source = reference(1, CoordinateUnit::Metres);
    let target = reference(2, CoordinateUnit::Metres);
    let operation = CoordinateOperation::compile(
        source,
        target,
        OperationModel::Similarity2d(Similarity2d {
            translation: [
                Rat::new(Int::from_i64(5), Int::from_i64(2)).unwrap(),
                Rat::new(Int::from_i64(7), Int::from_i64(3)).unwrap(),
            ],
            scale: Rat::new(Int::from_i64(11), Int::from_i64(2)).unwrap(),
            rotation_degrees: Rat::new(Int::from_i64(45), Int::from_i64(2)).unwrap(),
            convention: RotationConvention::CoordinateFrame,
            inverse: true,
        }),
    )
    .unwrap();
    let law = operation.law_id().digest();
    let expected = crate::profile::hash_fields(
        super::OPERATION_BINDING,
        [
            law.as_bytes().as_slice(),
            source.realization.as_bytes().as_slice(),
            target.realization.as_bytes().as_slice(),
            &[0, 0],
            &[1, 1],
            b"5",
            b"2",
            b"7",
            b"3",
            b"11",
            b"2",
            b"45",
            b"2",
        ],
    );
    assert_eq!(operation.id(), expected);
}

#[test]
fn original_large_rational_refuses_before_clone_and_later_height_precedes_it() {
    let similarity = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Metres),
        reference(2, CoordinateUnit::Metres),
        OperationModel::Similarity2d(Similarity2d {
            translation: [Rat::zero(), Rat::zero()],
            scale: Rat::one(),
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        }),
    )
    .unwrap();
    let mut limits = ExecutionLimits::GEOMETRY;
    limits.max_work_items = 64;
    let policy = ExecutionPolicy::new(limits).unwrap();
    let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
    let magnitude = Int::one().shl(32_768);
    let point = OperationPoint {
        x: Rat::new(magnitude.clone(), magnitude.add(&Int::one())).unwrap(),
        y: Rat::zero(),
        z: None,
        epoch: None,
    };
    assert_eq!(
        similarity.apply(&point, &mut context),
        Err(GeoError::WorkExhausted { limit: 64 })
    );
    assert_eq!(context.work_items(), 0);
    let helmert = CoordinateOperation::compile(
        reference(2, CoordinateUnit::Metres),
        reference(3, CoordinateUnit::Metres),
        OperationModel::Helmert(Box::new(HelmertParameters {
            translation: [Rat::zero(), Rat::zero(), Rat::zero()],
            rotation_arcseconds: [Rat::zero(), Rat::zero(), Rat::zero()],
            scale_ppm: Rat::zero(),
            pivot: [Rat::zero(), Rat::zero(), Rat::zero()],
            law: RotationLaw::HelmertSmallAngleV1,
            convention: RotationConvention::PositionVector,
            rates: None,
            inverse: false,
        })),
    )
    .unwrap();
    let chain = OperationChain::compile(vec![similarity, helmert]).unwrap();
    assert_eq!(
        chain.apply(&point, &mut context),
        Err(GeoError::MissingHeight)
    );
    assert_eq!(context.work_items(), 0);
}

#[test]
fn admitted_epoch_fields_keep_normalized_source_bytes_and_actual_receipt() {
    let mut context =
        MetricContext::new(GeographicReference::wgs84(), ExecutionPolicy::geometry()).unwrap();
    context.begin(1).unwrap();
    let epoch = Rat::parse_decimal("2026.10").unwrap();
    let (fields, retained) =
        epoch_fields_admitted(Some(&epoch), &mut context, &mut WorkProgress::new(None)).unwrap();
    let fields = fields.unwrap();
    assert_eq!(fields[0], "20261");
    assert_eq!(fields[1], "10");
    assert!(
        retained
            >= 7 + crate::PreparedValue::<[String; 2]>::required_bytes(&WorkProgress::new(None))
                .unwrap()
    );
    assert!(context.work_items() > 0);
    assert_eq!(
        epoch_fields_admitted(None, &mut context, &mut WorkProgress::new(None)).unwrap(),
        (None, 0)
    );
}

#[test]
fn scalar_workers_retain_parameter_sized_destinations_between_completed_calls() {
    let scale = Rat::new(Int::one(), Int::one().shl(1024)).unwrap();
    let operation = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Metres),
        reference(2, CoordinateUnit::Metres),
        OperationModel::Similarity2d(Similarity2d {
            translation: [Rat::zero(), Rat::zero()],
            scale,
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        }),
    )
    .unwrap();
    let input = OperationPoint {
        x: Rat::one(),
        y: Rat::one(),
        z: None,
        epoch: None,
    };
    let mut context = MetricContext::wgs84().unwrap();
    assert!(context.integer_scratch().is_none());
    let first = operation.apply(&input, &mut context).unwrap();
    assert_eq!(first.point().x, Rat::zero());
    assert_eq!(first.point().y, Rat::zero());
    let scratch = context.integer_scratch().unwrap();
    let required = (u64::from(context.policy().limits().max_precision_bits) * 4
        + 2048
        + 2 * operation.max_original_operand_bits())
    .div_ceil(64) as usize;
    assert!(scratch.limb_capacity() >= required);
    let retained = context.retained_workspace_bytes();
    assert!(retained > 0);
    let second = operation.apply(&input, &mut context).unwrap();
    assert_eq!(first, second);
    assert_eq!(context.retained_workspace_bytes(), retained);
}
