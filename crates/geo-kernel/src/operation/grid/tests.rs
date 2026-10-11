// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::operation::{
    CoordinateOperation, CoordinateUnit, OperationChain, OperationModel, OperationPoint,
    RotationConvention, Similarity2d, TransformOutputGrid, tests::reference,
};
use crate::{GeoError, MetricContext, MetricWorkObserver, Rat};

fn identity(unit: CoordinateUnit) -> CoordinateOperation {
    let model = if unit == CoordinateUnit::Metres {
        OperationModel::Similarity2d(Similarity2d {
            translation: [Rat::zero(), Rat::zero()],
            scale: Rat::one(),
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        })
    } else {
        OperationModel::Bd09LlV1
    };
    CoordinateOperation::compile(reference(1, unit), reference(2, unit), model).unwrap()
}
fn point(x: &str, y: &str) -> OperationPoint {
    OperationPoint {
        x: Rat::parse_decimal(x).unwrap(),
        y: Rat::parse_decimal(y).unwrap(),
        z: None,
        epoch: None,
    }
}

#[test]
fn alternate_scalar_grids_bind_only_active_quanta_and_preserve_default_bytes() {
    let operation = identity(CoordinateUnit::Metres);
    let input = point("1.234567891234567891", "-2.987654321987654321");
    let mut context = MetricContext::wgs84().unwrap();
    let default = operation.apply(&input, &mut context).unwrap();
    assert_eq!(default.output_grid(), TransformOutputGrid::DEFAULT);
    let same = operation
        .apply_with_grid(&input, TransformOutputGrid::new(u32::MAX, 6), &mut context)
        .unwrap();
    assert_eq!(default, same);
    assert_eq!(default.certificate_bytes(), same.certificate_bytes());
    for places in [9, 18] {
        let grid = TransformOutputGrid::new(15, places);
        let answer = operation
            .apply_with_grid(&input, grid, &mut context)
            .unwrap();
        assert_eq!(answer.output_grid(), grid);
        assert_eq!(answer.operation_id(), default.operation_id());
        assert_ne!(answer.law_id(), default.law_id());
        assert_eq!(
            answer.point().x,
            Rat::from_decimal(input.x.round_to_scale(places), places)
        );
        assert_eq!(
            answer.point().y,
            Rat::from_decimal(input.y.round_to_scale(places), places)
        );
    }
    for grid in [
        TransformOutputGrid::new(15, 5),
        TransformOutputGrid::new(15, u32::MAX),
    ] {
        assert!(matches!(
            operation.apply_with_grid(&input, grid, &mut context),
            Err(GeoError::PrecisionExhausted { .. })
        ));
    }
}

#[test]
fn angular_grids_reuse_the_original_inverse_and_quantize_only_final_chain_images() {
    let operation = identity(CoordinateUnit::Degrees);
    let input = point("100.123456789123456789", "20.345678912345678912");
    let mut context = MetricContext::wgs84().unwrap();
    let default = operation.apply(&input, &mut context).unwrap();
    for places in [12, 18] {
        let grid = TransformOutputGrid::new(places, 6);
        let answer = operation
            .apply_with_grid(&input, grid, &mut context)
            .unwrap();
        assert_eq!(answer.operation_id(), default.operation_id());
        assert_ne!(answer.law_id(), default.law_id());
        let inverse = operation.inverse().unwrap();
        let restored = inverse
            .apply_with_grid(answer.point(), grid, &mut context)
            .unwrap();
        assert!(
            restored.point().x.sub(&input.x).abs() <= Rat::parse_decimal("0.000000000001").unwrap()
        );
        assert!(
            restored.point().y.sub(&input.y).abs() <= Rat::parse_decimal("0.000000000001").unwrap()
        );
    }
    assert!(matches!(
        operation.apply_with_grid(&input, TransformOutputGrid::new(10, 6), &mut context),
        Err(GeoError::PrecisionExhausted { .. })
    ));
    let first = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Metres),
        reference(2, CoordinateUnit::Metres),
        OperationModel::Similarity2d(Similarity2d {
            translation: [Rat::parse_decimal("0.00000049").unwrap(), Rat::zero()],
            scale: Rat::one(),
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        }),
    )
    .unwrap();
    let second = CoordinateOperation::compile(
        reference(2, CoordinateUnit::Metres),
        reference(3, CoordinateUnit::Metres),
        OperationModel::Similarity2d(Similarity2d {
            translation: [Rat::zero(), Rat::zero()],
            scale: Rat::from_i64(2),
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        }),
    )
    .unwrap();
    let chain = OperationChain::compile(vec![first, second]).unwrap();
    let answer = chain
        .apply_with_grid(
            &point("0", "0"),
            TransformOutputGrid::new(15, 9),
            &mut context,
        )
        .unwrap();
    assert_eq!(answer.point().x, Rat::parse_decimal("0.00000098").unwrap());
}

#[test]
fn explicit_grid_batches_observe_actual_work_and_clear_every_refused_slot() {
    #[derive(Default)]
    struct Receipt {
        work: u64,
        peak: u64,
        calls: usize,
        stop_after: Option<usize>,
    }
    impl MetricWorkObserver for Receipt {
        fn charge_chunk(&mut self, work: u64, peak: u64) -> Result<(), GeoError> {
            self.work += work;
            self.peak += peak;
            self.calls += 1;
            if self.stop_after == Some(self.calls) {
                Err(GeoError::Cancelled)
            } else {
                Ok(())
            }
        }
    }
    let operation = identity(CoordinateUnit::Metres);
    let points = [point("1.2345678912", "0"), point("0", "2.3456789123")];
    let worker = |warm| {
        let mut context = MetricContext::wgs84().unwrap();
        if warm {
            operation
                .apply_batch_with_grid(
                    &points,
                    &mut [None, None],
                    TransformOutputGrid::new(15, 9),
                    &mut context,
                )
                .unwrap();
        }
        context
    };
    for warm in [false, true] {
        let mut output = [None, None];
        let mut context = worker(warm);
        let mut receipt = Receipt::default();
        operation
            .apply_batch_with_grid_metered(
                &points,
                &mut output,
                TransformOutputGrid::new(15, 9),
                &mut context,
                &mut receipt,
            )
            .unwrap();
        assert!(output.iter().all(Option::is_some));
        assert_eq!(receipt.work, context.work_items());
        assert_eq!(receipt.peak, context.workspace_peak());
        let calls = receipt.calls;
        for stop in 1..=calls {
            let mut context = worker(warm);
            let mut receipt = Receipt {
                stop_after: Some(stop),
                ..Receipt::default()
            };
            assert_eq!(
                operation.apply_batch_with_grid_metered(
                    &points,
                    &mut output,
                    TransformOutputGrid::new(15, 9),
                    &mut context,
                    &mut receipt,
                ),
                Err(GeoError::Cancelled)
            );
            assert_eq!(receipt.calls, stop);
            assert_eq!(output, [None, None]);
            // Reuse only after every copied source, completed prefix and refused
            // finalization reservation has been released, in both cache states.
            operation.apply(&points[0], &mut context).unwrap();
        }
    }
}
