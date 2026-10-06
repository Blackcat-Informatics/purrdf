// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use crate::{
    CoordinateOperation, MetricContext,
    operation::{CoordinateUnit, GridNode, OperationModel, OperationReference, PolynomialTerm},
};
use purrdf_hash::hex::Digest32;
fn rat(text: &str) -> Rat {
    Rat::parse_decimal(text).expect("decimal")
}
fn point(x: &str, y: &str) -> OperationPoint {
    OperationPoint {
        x: rat(x),
        y: rat(y),
        z: None,
        epoch: None,
    }
}
fn operation(model: OperationModel) -> CoordinateOperation {
    let reference = |id| OperationReference {
        realization: Digest32::new([id; 32]),
        unit: CoordinateUnit::Metres,
        swapped_axes: false,
    };
    CoordinateOperation::compile(reference(1), reference(2), model).expect("compile")
}
fn domain(xmin: &str, xmax: &str, ymin: &str, ymax: &str) -> MetricSourceDomain {
    MetricSourceDomain::new([rat(xmin), rat(ymin)], [rat(xmax), rat(ymax)]).expect("domain")
}
fn term(x: u16, y: u16, cx: &str, cy: &str) -> PolynomialTerm {
    PolynomialTerm {
        x_power: x,
        y_power: y,
        x_coefficient: rat(cx),
        y_coefficient: rat(cy),
    }
}
#[test]
fn exact_affine_inverse_preserves_decimal_domain_boundaries() {
    let polynomial = Polynomial2d::new(
        1,
        [rat("0.1"), rat("0.2")],
        [rat("2"), rat("3")],
        vec![
            term(0, 0, "7", "-2"),
            term(1, 0, "2", "1"),
            term(0, 1, "1", "4"),
        ],
    )
    .expect("polynomial");
    let source = point("0.1", "0.2");
    let forward = operation(OperationModel::Polynomial2d(polynomial.clone()));
    let inverse = operation(OperationModel::Polynomial2dInverse(Box::new(
        Polynomial2dInverse {
            polynomial,
            source_domain: domain("0.1", "10", "0.2", "20"),
        },
    )));
    let mut context = MetricContext::wgs84().expect("context");
    let target = forward.apply(&source, &mut context).expect("forward");
    let restored = inverse
        .apply(target.point(), &mut context)
        .expect("inverse at boundary");
    assert_eq!(restored.point(), &source);
    assert!(matches!(
        inverse.apply(&point("-100", "-100"), &mut context),
        Err(GeoError::Domain(_))
    ));
}
#[test]
fn polynomial_fold_returns_every_root_as_ambiguity() {
    let polynomial = Polynomial2d::new(
        2,
        [Rat::zero(), Rat::zero()],
        [Rat::one(), Rat::one()],
        vec![term(2, 0, "1", "0"), term(0, 1, "0", "1")],
    )
    .expect("polynomial");
    let inverse = operation(OperationModel::Polynomial2dInverse(Box::new(
        Polynomial2dInverse {
            polynomial,
            source_domain: domain("-2", "2", "-1", "1"),
        },
    )));
    let mut context = MetricContext::wgs84().expect("context");
    assert!(matches!(
        inverse.apply(&point("1", "0"), &mut context),
        Err(GeoError::AmbiguousTransform { roots: 2 })
    ));
    assert!(matches!(
        inverse.apply(&point("-1", "0"), &mut context),
        Err(GeoError::Domain(_))
    ));
}
#[test]
fn closed_grid_boundary_survives_neighbor_hole_and_inverts() {
    let node = || {
        Some(GridNode {
            x: rat("2"),
            y: rat("-1"),
        })
    };
    let grid = BilinearGrid::new(
        [rat("0.1"), rat("0.2")],
        [Rat::one(), Rat::one()],
        3,
        2,
        vec![node(), node(), None, node(), node(), None],
    )
    .expect("grid");
    let source = point("1.1", "0.7");
    let forward = operation(OperationModel::BilinearGrid(grid.clone()));
    let inverse = operation(OperationModel::BilinearGridInverse(Box::new(
        BilinearGridInverse {
            grid,
            source_domain: domain("0.1", "2.1", "0.2", "1.2"),
        },
    )));
    let mut context = MetricContext::wgs84().expect("context");
    let target = forward
        .apply(&source, &mut context)
        .expect("closed valid edge");
    let restored = inverse
        .apply(target.point(), &mut context)
        .expect("inverse closed edge");
    assert_eq!(restored.point(), &source);
    assert!(matches!(
        forward.apply(&point("1.2", "0.7"), &mut context),
        Err(GeoError::Domain(_))
    ));
    assert!(matches!(
        inverse.apply(&point("3.2", "-0.3"), &mut context),
        Err(GeoError::Domain(_))
    ));
}

#[test]
fn nonlinear_bilinear_inverse_isolates_the_complete_patch_root() {
    let node = |x| {
        Some(GridNode {
            x: Rat::from_i64(x),
            y: Rat::zero(),
        })
    };
    let grid = BilinearGrid::new(
        [Rat::zero(), Rat::zero()],
        [Rat::one(), Rat::one()],
        2,
        2,
        vec![node(0), node(0), node(0), node(1)],
    )
    .expect("xy displacement");
    let inverse = operation(OperationModel::BilinearGridInverse(Box::new(
        BilinearGridInverse {
            grid,
            source_domain: domain("0", "1", "0", "1"),
        },
    )));
    let restored = inverse
        .apply(
            &point("0.75", "0.5"),
            &mut MetricContext::wgs84().expect("context"),
        )
        .expect("nonlinear root");
    assert_eq!(restored.point(), &point("0.5", "0.5"));
}
#[test]
fn grid_fold_has_two_distinct_roots_and_shared_node_aliases_once() {
    let node = |x| {
        Some(GridNode {
            x: Rat::from_i64(x),
            y: Rat::zero(),
        })
    };
    let grid = BilinearGrid::new(
        [Rat::zero(), Rat::zero()],
        [Rat::one(), Rat::one()],
        3,
        2,
        vec![node(0), node(0), node(-2), node(0), node(0), node(-2)],
    )
    .expect("grid");
    let inverse = operation(OperationModel::BilinearGridInverse(Box::new(
        BilinearGridInverse {
            grid,
            source_domain: domain("0", "2", "0", "1"),
        },
    )));
    let mut context = MetricContext::wgs84().expect("context");
    assert!(matches!(
        inverse.apply(&point("0.5", "0.5"), &mut context),
        Err(GeoError::AmbiguousTransform { roots: 2 })
    ));
    assert_eq!(
        inverse
            .apply(&point("1", "0.5"), &mut context)
            .expect("shared node unique")
            .point(),
        &point("1", "0.5")
    );
}

#[test]
fn inverse_chain_certifies_final_quantization_against_unrounded_intermediate() {
    let model = Polynomial2d::new(
        1,
        [Rat::zero(), Rat::zero()],
        [Rat::one(), Rat::one()],
        vec![term(1, 0, "10", "0"), term(0, 1, "0", "1")],
    )
    .expect("polynomial");
    let inverse = operation(OperationModel::Polynomial2dInverse(Box::new(
        Polynomial2dInverse {
            polynomial: model,
            source_domain: domain("-1", "1", "-1", "1"),
        },
    )));
    let identity = Polynomial2d::new(
        1,
        [Rat::zero(), Rat::zero()],
        [Rat::one(), Rat::one()],
        vec![term(1, 0, "1", "0"), term(0, 1, "0", "1")],
    )
    .expect("identity");
    let prefix = CoordinateOperation::compile(
        OperationReference {
            realization: Digest32::new([3; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        },
        inverse.source(),
        OperationModel::Polynomial2d(identity),
    )
    .expect("prefix");
    let chain = crate::OperationChain::compile(vec![prefix, inverse]).expect("continuous chain");
    let mut context = MetricContext::wgs84().expect("context");
    assert_eq!(
        chain
            .apply(&point("0.00001", "0"), &mut context)
            .expect("exact output")
            .point(),
        &point("0.000001", "0")
    );
    assert!(matches!(
        chain.apply(&point("0.0000031", "0"), &mut context),
        Err(GeoError::PrecisionExhausted { .. })
    ));
}
