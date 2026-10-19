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
fn polynomial_refusals_keep_original_scales_and_sorted_coefficients() {
    use super::{Polynomial2d, PolynomialTerm};
    use crate::PolynomialInputError;
    use purrdf_testkit::error_capture::Capture;

    for (scale, axis) in [
        ([Rat::zero(), Rat::zero()], 0),
        ([Rat::one(), Rat::zero()], 1),
    ] {
        let origin = [Rat::from_i64(17), Rat::from_i64(-23)];
        let error = Polynomial2d::new(3, origin.clone(), scale.clone(), Vec::new()).unwrap_err();
        assert!(matches!(error.original(), GeoError::PolynomialInput(cause)
            if **cause == PolynomialInputError::Normalization { degree: 3, origin, scale, axis, term_count: 0 }));
        assert_eq!(
            error.detail(),
            "polynomial normalization scales must be positive"
        );
    }
    let first = PolynomialTerm {
        x_power: 0,
        y_power: 0,
        x_coefficient: Rat::from_int(Int::one().shl(4096)),
        y_coefficient: Rat::from_i64(-7),
    };
    let second = PolynomialTerm {
        x_coefficient: Rat::from_i64(19),
        y_coefficient: Rat::parse_decimal(
            "0.0000000000000000000000000000000000000000000000000000001",
        )
        .unwrap(),
        ..first
    };
    let excessive = PolynomialTerm {
        x_power: u16::MAX,
        y_power: u16::MAX,
        x_coefficient: Rat::one(),
        y_coefficient: Rat::zero(),
    };
    let error = Polynomial2d::new(
        1,
        [Rat::zero(), Rat::zero()],
        [Rat::one(), Rat::one()],
        vec![excessive.clone(), first.clone(), second.clone()],
    )
    .unwrap_err();
    assert!(matches!(error.original(), GeoError::PolynomialInput(cause)
        if **cause == PolynomialInputError::Degree { degree: 1, index: 2, term_count: 3, term: excessive }));
    let error = Polynomial2d::new(
        1,
        [Rat::zero(), Rat::zero()],
        [Rat::one(), Rat::one()],
        vec![second.clone(), first.clone()],
    )
    .unwrap_err();
    // Equal keys retain their original order in the existing stable sorter.
    assert!(matches!(error.original(), GeoError::PolynomialInput(cause)
        if **cause == PolynomialInputError::Duplicate { degree: 1, index: 0, term_count: 2, first: second, second: first }));
    assert_eq!(
        Capture::read(error.original())
            .child("cause")
            .child("first")
            .code,
        "polynomial-term"
    );
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let last = error.clone();
    assert_eq!(window.close().allocations, 0);
    drop(error);
    assert_eq!(
        Capture::read(last.original()).child("cause").code,
        "polynomial-duplicate"
    );
}

#[test]
fn grid_refusals_preserve_dimensions_holes_source_bounds_and_trial_refinement() {
    use super::{
        BilinearGrid, BilinearGridInverse, GridLocationRefusal, GridNode, MetricSourceDomain,
    };
    use crate::{GridConfigurationReason, GridDomainSide, ModelInputError};
    use purrdf_xsd::math::{CoordinateMath, MathError, MathLimits};

    for (columns, rows, spacing, reason) in [
        (
            1,
            1,
            [Rat::zero(), Rat::zero()],
            GridConfigurationReason::Columns,
        ),
        (
            2,
            1,
            [Rat::zero(), Rat::zero()],
            GridConfigurationReason::Rows,
        ),
        (
            2,
            2,
            [Rat::zero(), Rat::zero()],
            GridConfigurationReason::Spacing { axis: 0 },
        ),
        (
            2,
            2,
            [Rat::one(), Rat::zero()],
            GridConfigurationReason::Spacing { axis: 1 },
        ),
    ] {
        let origin = [Rat::from_i64(17), Rat::from_i64(-23)];
        let error = BilinearGrid::new(origin.clone(), spacing.clone(), columns, rows, Vec::new())
            .unwrap_err();
        assert!(matches!(error.original(), GeoError::ModelInput(cause)
            if **cause == ModelInputError::GridConfiguration { reason, origin, spacing, columns, rows, node_count: 0 }));
    }
    for (lower, upper, axis) in [
        ([Rat::one(), Rat::one()], [Rat::zero(), Rat::zero()], 0),
        ([Rat::zero(), Rat::one()], [Rat::one(), Rat::zero()], 1),
    ] {
        let error = MetricSourceDomain::new(lower.clone(), upper.clone()).unwrap_err();
        let GeoError::ModelInput(cause) = error.original() else {
            panic!("native source rectangle")
        };
        let ModelInputError::SourceRectangle {
            domain,
            axis: original,
        } = &**cause
        else {
            panic!("original bounds")
        };
        assert_eq!(
            (*original, domain.lower(), domain.upper()),
            (axis, &lower, &upper)
        );
    }
    let node = GridNode {
        x: Rat::zero(),
        y: Rat::zero(),
    };
    let grid = BilinearGrid::new(
        [Rat::zero(), Rat::zero()],
        [Rat::one(), Rat::one()],
        2,
        2,
        vec![Some(node.clone()), None, Some(node.clone()), Some(node)],
    )
    .unwrap();
    let missing = grid.corners(0, 0).unwrap_err();
    assert_eq!(
        (
            missing.column,
            missing.row,
            missing.node_column,
            missing.node_row,
            missing.index
        ),
        (0, 0, 1, 0, 1)
    );
    let point = OperationPoint {
        x: Rat::parse_decimal("0.5").unwrap(),
        y: Rat::parse_decimal("0.5").unwrap(),
        z: None,
        epoch: None,
    };
    let error = grid
        .validate_point(&point, &mut WorkProgress::integer(None))
        .unwrap_err();
    assert!(matches!(error.original(), GeoError::ModelInput(cause)
        if matches!(&**cause, ModelInputError::Location { grid: original, x, y, cause: GridLocationRefusal::NoCompletePatch { column: 0, row: 0, first_column: 0, first_row: 0 } }
            if **original == grid && **x == point.x && **y == point.y)));
    let mut progress = WorkProgress::integer(None);
    let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
    assert_eq!(
        grid.location_math(
            &Rat::from_i64(-1),
            &Rat::zero(),
            false,
            &mut math,
            &mut progress
        ),
        Err(MathError::PrecisionExhausted)
    );
    assert!(progress.latched_error().is_none());
    let outside = OperationPoint {
        x: Rat::from_i64(-1),
        ..point
    };
    let error = grid
        .validate_point(&outside, &mut WorkProgress::integer(None))
        .unwrap_err();
    assert!(matches!(error.original(), GeoError::ModelInput(cause)
        if matches!(&**cause, ModelInputError::Location { cause: GridLocationRefusal::OutsideExtent { axis: 0, relative, last_node: 1 }, .. } if relative == &Rat::from_i64(-1))));
    let inverse = BilinearGridInverse {
        grid: grid.clone(),
        source_domain: MetricSourceDomain::new(
            [Rat::from_i64(-1), Rat::zero()],
            [Rat::from_i64(2), Rat::from_i64(2)],
        )
        .unwrap(),
    };
    let error = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Metres),
        reference(2, CoordinateUnit::Metres),
        OperationModel::BilinearGridInverse(Box::new(inverse.clone())),
    )
    .unwrap_err();
    assert!(matches!(error.original(), GeoError::ModelInput(cause)
        if matches!(&**cause, ModelInputError::InverseRectangle { grid: original, domain, upper, axis: 0, side: GridDomainSide::Lower }
            if **original == grid && domain == &inverse.source_domain && *upper[0] == Rat::one() && *upper[1] == Rat::one())));
}

#[test]
fn native_projection_refusals_keep_original_model_fields_and_order() {
    use super::Applicability;
    use crate::{GcjInputReason, MercatorRequirement, ProjectionInputError};
    use purrdf_testkit::error_capture::Capture;

    for (model, radius, eccentricity, square_domain, requirement) in [
        (
            OperationModel::WebMercator {
                radius: Rat::zero(),
            },
            Rat::zero(),
            None,
            None,
            MercatorRequirement::Radius,
        ),
        (
            OperationModel::EllipsoidalMercator {
                semimajor: Rat::zero(),
                eccentricity_squared: Rat::from_i64(-1),
            },
            Rat::zero(),
            Some(Rat::from_i64(-1)),
            None,
            MercatorRequirement::Radius,
        ),
        (
            OperationModel::EllipsoidalMercator {
                semimajor: Rat::one(),
                eccentricity_squared: Rat::from_i64(-1),
            },
            Rat::one(),
            Some(Rat::from_i64(-1)),
            None,
            MercatorRequirement::EccentricityLower,
        ),
        (
            OperationModel::EllipsoidalMercator {
                semimajor: Rat::one(),
                eccentricity_squared: Rat::one(),
            },
            Rat::one(),
            Some(Rat::one()),
            None,
            MercatorRequirement::EccentricityUpper,
        ),
        (
            OperationModel::MercatorToGeographic {
                radius: Rat::one(),
                eccentricity_squared: Rat::one(),
                square_domain: true,
            },
            Rat::one(),
            Some(Rat::one()),
            Some(true),
            MercatorRequirement::EccentricityUpper,
        ),
    ] {
        let (input, output) = model.units();
        let law = model.descriptor();
        let error = CoordinateOperation::compile(reference(1, input), reference(2, output), model)
            .unwrap_err();
        let GeoError::ProjectionInput(cause) = error.original() else {
            panic!("native Mercator configuration")
        };
        let ProjectionInputError::MercatorConfiguration {
            requirement: original,
            law: original_law,
            square_domain: original_square,
            radius: original_radius,
            eccentricity_squared,
        } = &**cause
        else {
            panic!("original parameter fields")
        };
        assert_eq!(
            (*original, *original_law, *original_square),
            (requirement, law, square_domain)
        );
        assert_eq!(&**original_radius, &radius);
        assert_eq!(eccentricity_squared.as_deref(), eccentricity.as_ref());
        assert!(!error.is_expression_error());
        assert_eq!(
            Capture::read(error.original()).child("cause").code,
            "projection-mercator-configuration"
        );
    }
    let point = OperationPoint {
        x: Rat::from_i64(17),
        y: Rat::from_i64(90),
        z: Some(Rat::from_i64(7)),
        epoch: Some(Rat::from_i64(2026)),
    };
    for (model, radius, eccentricity) in [
        (OperationModel::BaiduMercatorAnalyticV1, None, None),
        (
            OperationModel::WebMercator {
                radius: Rat::from_i64(6_378_137),
            },
            Some(Rat::from_i64(6_378_137)),
            None,
        ),
        (
            OperationModel::EllipsoidalMercator {
                semimajor: Rat::from_i64(6_378_137),
                eccentricity_squared: Rat::parse_decimal("0.0067").unwrap(),
            },
            Some(Rat::from_i64(6_378_137)),
            Some(Rat::parse_decimal("0.0067").unwrap()),
        ),
    ] {
        let error = model
            .validate_point(&point, &mut WorkProgress::integer(None))
            .unwrap_err();
        let GeoError::ProjectionInput(cause) = error.original() else {
            panic!("native Mercator pole")
        };
        let ProjectionInputError::MercatorPole {
            law,
            point: original,
            radius: original_radius,
            eccentricity_squared,
        } = &**cause
        else {
            panic!("original pole fields")
        };
        assert_eq!((*law, original), (model.descriptor(), &point));
        assert_eq!(original_radius.as_deref(), radius.as_ref());
        assert_eq!(eccentricity_squared.as_deref(), eccentricity.as_ref());
        assert!(error.is_expression_error());
        assert_eq!(error.detail(), "Mercator is undefined at a pole");
    }
    let poles = Applicability::new(
        Rat::from_i64(-10),
        Rat::from_i64(10),
        Rat::from_i64(-90),
        Rat::from_i64(90),
    )
    .unwrap();
    let error = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Degrees),
        reference(2, CoordinateUnit::Degrees),
        OperationModel::GcjRationalHarmonicV1(poles.clone()),
    )
    .unwrap_err();
    assert!(matches!(error.original(), GeoError::ProjectionInput(cause)
        if matches!(&**cause, ProjectionInputError::Gcj { reason: GcjInputReason::Poles, domain, point: None } if **domain == poles)));
    let domain =
        Applicability::new(Rat::from_i64(-1), Rat::one(), Rat::from_i64(-1), Rat::one()).unwrap();
    let outside = OperationPoint {
        x: Rat::from_i64(2),
        y: Rat::zero(),
        ..point
    };
    let error = OperationModel::GcjRationalHarmonicV1(domain.clone())
        .validate_point(&outside, &mut WorkProgress::integer(None))
        .unwrap_err();
    assert!(matches!(error.original(), GeoError::ProjectionInput(cause)
        if matches!(&**cause, ProjectionInputError::Gcj { reason: GcjInputReason::Outside, domain: original, point: Some(point) } if **original == domain && *point == outside)));
}

#[test]
fn native_affine_refusals_keep_rates_epoch_and_adjusted_scale() {
    use super::HelmertRates;
    use crate::{AffineInputError, HelmertInputReason, OperationValidationContext};
    use purrdf_testkit::error_capture::Capture;
    use purrdf_xsd::math::{CoordinateMath, MathError, MathLimits};

    let parameters = HelmertParameters {
        translation: [
            Rat::parse_decimal("123456789012345678901234567890123456789012345678901234567890.125")
                .unwrap(),
            Rat::zero(),
            Rat::one(),
        ],
        rotation_arcseconds: [Rat::zero(), Rat::one(), Rat::from_i64(2)],
        scale_ppm: Rat::zero(),
        pivot: [Rat::one(), Rat::from_i64(2), Rat::from_i64(3)],
        law: RotationLaw::HelmertEulerRzRyRxV1,
        convention: RotationConvention::CoordinateFrame,
        rates: Some(HelmertRates {
            epoch: Rat::from_i64(2000),
            translation: [Rat::one(), Rat::zero(), Rat::zero()],
            rotation_arcseconds: [Rat::zero(), Rat::one(), Rat::zero()],
            scale_ppm: Rat::from_i64(-1_000_000),
        }),
        inverse: true,
    };
    let point = OperationPoint {
        x: Rat::one(),
        y: Rat::from_i64(2),
        z: Some(Rat::from_i64(3)),
        epoch: None,
    };
    let declared = HelmertParameters {
        scale_ppm: Rat::from_i64(-1_000_000),
        ..parameters.clone()
    };
    let declared_error = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Metres),
        reference(2, CoordinateUnit::Metres),
        OperationModel::Helmert(Box::new(declared.clone())),
    )
    .unwrap_err();
    let observed = OperationPoint {
        epoch: Some(Rat::parse_decimal("2002.5").unwrap()),
        ..point.clone()
    };
    for (error, expected_parameters, expected_point, expected_scale, reason) in [
        (
            declared_error,
            &declared,
            None,
            None,
            HelmertInputReason::DeclaredScale,
        ),
        (
            parameters
                .validate_point(&point, &mut WorkProgress::integer(None))
                .unwrap_err(),
            &parameters,
            Some(&point),
            None,
            HelmertInputReason::ObservationEpoch,
        ),
        (
            parameters
                .validate_point(&observed, &mut WorkProgress::integer(None))
                .unwrap_err(),
            &parameters,
            Some(&observed),
            Some(Rat::from_i64(-2_500_000)),
            HelmertInputReason::AdjustedScale,
        ),
    ] {
        let GeoError::AffineInput(cause) = error.original() else {
            panic!("original affine input")
        };
        let AffineInputError::Helmert {
            reason: original_reason,
            parameters: original,
            point,
            height,
            adjusted_scale,
        } = &**cause
        else {
            panic!("original Helmert fields")
        };
        assert_eq!(
            (*original_reason, &**original),
            (reason, expected_parameters)
        );
        assert_eq!(point.as_ref(), expected_point);
        assert!(height.is_none());
        assert_eq!(adjusted_scale.as_deref(), expected_scale.as_ref());
        assert_eq!(
            error.is_expression_error(),
            reason != HelmertInputReason::DeclaredScale
        );
        assert_eq!(
            Capture::read(error.original())
                .child("cause")
                .child("parameters")
                .child("rates")
                .code,
            "helmert-rates"
        );
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let last = error.clone();
        assert_eq!(window.close().allocations, 0);
        drop(error);
        assert_eq!(
            Capture::read(last.original()).child("cause").code,
            "affine-helmert-input"
        );
    }
    let model = OperationModel::Helmert(Box::new(parameters.clone()));
    let mut progress = WorkProgress::integer(None);
    assert_eq!(
        model.validate_presence(None, None, &mut progress),
        Err(GeoError::MissingHeight)
    );
    let error = model
        .validate_presence(point.z.as_ref(), None, &mut progress)
        .unwrap_err();
    assert!(matches!(error.original(), GeoError::AffineInput(cause)
        if matches!(&**cause, AffineInputError::Helmert { point: None, height: Some(original), reason: HelmertInputReason::ObservationEpoch, .. }
            if Some(&**original) == point.z.as_ref())));

    let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
    let mut progress = WorkProgress::integer(None);
    assert_eq!(
        parameters.evaluate_with_matrix(&point, &[], &mut math, &mut progress),
        Err(MathError::Cancelled)
    );
    let error = progress.latched_error().unwrap();
    let GeoError::OperationValidation { context, cause } = &error else {
        panic!("original math seam context")
    };
    assert_eq!(*context, OperationValidationContext::AffineEpoch);
    let original = std::error::Error::source(cause)
        .unwrap()
        .downcast_ref::<GeoError>()
        .unwrap();
    assert!(matches!(original, GeoError::AffineInput(cause)
        if matches!(&**cause, AffineInputError::Helmert { reason: HelmertInputReason::AdjustmentEpoch, .. })));
    assert_eq!(error.detail(), "missing epoch");
    assert_eq!(math.work_used(), 0);

    let mut progress = WorkProgress::integer(None);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    assert_eq!(
        GeoError::math_validation_failure(
            GeoError::MemoryExhausted { limit: u64::MAX },
            OperationValidationContext::SourceMetadata,
            &mut progress
        ),
        MathError::Cancelled
    );
    assert_eq!(
        progress.latched_error(),
        Some(GeoError::MemoryExhausted { limit: u64::MAX })
    );
    assert_eq!(window.close().allocations, 0);

    let similarity = Similarity2d {
        translation: [Rat::zero(), Rat::one()],
        scale: Rat::zero(),
        rotation_degrees: Rat::from_i64(17),
        convention: RotationConvention::PositionVector,
        inverse: false,
    };
    let error = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Metres),
        reference(2, CoordinateUnit::Metres),
        OperationModel::Similarity2d(similarity.clone()),
    )
    .unwrap_err();
    assert!(matches!(error.original(), GeoError::AffineInput(cause)
        if **cause == AffineInputError::SimilarityScale { parameters: similarity }));
}

#[test]
fn original_operation_extents_and_unit_refusals_keep_exact_fields() {
    use super::Applicability;
    use crate::{BindingConfigError, measure::Ordinate};
    use purrdf_testkit::error_capture::Capture;

    for (west, east, south, north, axis) in [
        (
            "123456789012345678901234567890123456789012345678901234567890.125",
            "0",
            "2",
            "1",
            Ordinate::X,
        ),
        (
            "0",
            "1",
            "0.0000000000000000000000000000000000000000000000000000001",
            "0",
            Ordinate::Y,
        ),
        ("0", "0", "0", "1", Ordinate::X),
    ] {
        let [west, east, south, north] =
            [west, east, south, north].map(|value| Rat::parse_decimal(value).unwrap());
        let error = Applicability::new(west.clone(), east.clone(), south.clone(), north.clone())
            .unwrap_err();
        let GeoError::BindingConfiguration(cause) = error.original() else {
            panic!("original applicability refusal")
        };
        assert!(
            matches!(&**cause, BindingConfigError::Applicability { axis: original, west: first, east: second, south: third, north: fourth }
            if *original == axis && **first == west && **second == east && **third == south && **fourth == north)
        );
        assert_eq!(
            Capture::read(error.original()).child("cause").code,
            "carrier-operation-applicability"
        );
        assert_eq!(
            error.detail(),
            "operation applicability must have positive extents"
        );
    }
    let source = reference(91, CoordinateUnit::Metres);
    let target = reference(92, CoordinateUnit::Metres);
    let model = OperationModel::WebMercator {
        radius: Rat::from_i64(6_378_137),
    };
    let law = model.descriptor();
    let error = CoordinateOperation::compile(source, target, model).unwrap_err();
    assert!(
        matches!(error.original(), GeoError::BindingConfiguration(cause)
        if **cause == BindingConfigError::OperationUnits { source, target, input: CoordinateUnit::Degrees, output: CoordinateUnit::Metres, law })
    );
    assert_eq!(
        Capture::read(error.original()).child("cause").code,
        "carrier-operation-units"
    );
}

#[test]
fn original_chain_and_image_target_failures_keep_compared_declarations() {
    use super::{ImageMetric, OperationImageCurve};
    use crate::{
        BindingConfigError, Coord, CoordDim, Crs, Geometry, GeometryBody, ImageTargetReason,
    };
    use purrdf_testkit::error_capture::Capture;

    let error = OperationChain::compile(Vec::new()).unwrap_err();
    assert!(
        matches!(error.original(), GeoError::BindingConfiguration(cause)
        if **cause == BindingConfigError::EmptyOperationChain { operation_count: 0, compiled: false })
    );
    let first = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Degrees),
        reference(2, CoordinateUnit::Metres),
        OperationModel::WebMercator {
            radius: Rat::from_i64(6_378_137),
        },
    )
    .unwrap();
    let second = CoordinateOperation::compile(
        first.target(),
        reference(3, CoordinateUnit::Degrees),
        OperationModel::MercatorToGeographic {
            radius: Rat::from_i64(6_378_137),
            eccentricity_squared: Rat::zero(),
            square_domain: false,
        },
    )
    .unwrap();
    let error =
        OperationChain::compile(vec![first.clone(), second.clone(), first.clone()]).unwrap_err();
    assert!(
        matches!(error.original(), GeoError::BindingConfiguration(cause)
        if **cause == BindingConfigError::ChainBoundary {
            index: 1, preceding_operation: second.id(), following_operation: first.id(),
            target: second.target(), source: first.source(),
        })
    );
    assert_eq!(
        Capture::read(error.original()).child("cause").code,
        "carrier-operation-chain-boundary"
    );

    let endpoint = second.target();
    let chain = OperationChain::compile(vec![first, second]).unwrap();
    let geographic = GeographicReference::wgs84();
    let source = Geometry::new(CoordDim::Xy, GeometryBody::Point(None)).unwrap();
    let mut context = MetricContext::wgs84().unwrap();
    for (metric, reason) in [
        (ImageMetric::CartesianMetres, ImageTargetReason::MetricUnits),
        (
            ImageMetric::Geographic(Box::new(geographic.clone())),
            ImageTargetReason::MetricReference,
        ),
    ] {
        let error = chain
            .materialize_geometry(
                &source,
                Crs::new("https://example.org/target").unwrap(),
                &metric,
                None,
                &mut context,
            )
            .unwrap_err();
        assert!(
            matches!(error.original(), GeoError::BindingConfiguration(cause)
            if **cause == BindingConfigError::ImageTarget { reason, target: endpoint, metric: metric.clone() })
        );
        assert_eq!(
            Capture::read(error.original())
                .child("cause")
                .child("target")
                .code,
            "operation-reference"
        );
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let last = error.clone();
        assert_eq!(window.close().allocations, 0);
        drop(error);
        drop(metric);
        assert_eq!(
            Capture::read(last.original()).child("cause").code,
            "carrier-image-target"
        );
    }
    let error = OperationImageCurve::new(
        Coord::xy(Rat::zero(), Rat::zero()),
        Coord::xy(Rat::one(), Rat::one()),
        &chain,
        geographic.clone(),
        None,
        ExecutionPolicy::geometry(),
    )
    .unwrap_err();
    assert!(
        matches!(error.original(), GeoError::BindingConfiguration(cause)
        if **cause == BindingConfigError::ImageTarget {
            reason: ImageTargetReason::CurveReference, target: endpoint,
            metric: ImageMetric::Geographic(Box::new(geographic)),
        })
    );
}

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
