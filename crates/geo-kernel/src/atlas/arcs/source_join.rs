// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact common source lines and their complete local image certificates.
//! A local injectivity proof excludes a pair box only when its original source
//! parameter domains are disjoint or touch at one already known source node.

use super::{ChartLine, CurveIntersection};
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, fixed_from_rat, geo_math_error};
use crate::operation::OperationImageCurve;
use crate::{GeoError, MetricContext, PreparedEdge, Rat, SourceLinearEdge};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError, RootBox2};

pub(super) struct SourceJoin {
    pub(super) line: ChartLine,
    pub(super) parameters: [[Rat; 2]; 2],
    pub(super) analytic: bool,
    contact: CurveIntersection,
}

pub(super) fn prepare(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<SourceJoin>, GeoError> {
    let (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) = (left, right) else {
        return Ok(None);
    };
    if !a.same_mapping_admitted(b, context, progress)? {
        return Ok(None);
    }
    let (a0, a1) = a.source_endpoints();
    let (b0, b1) = b.source_endpoints();
    let mut intersection = crate::topology::intersect_admitted(
        a0,
        a1,
        b0,
        b1,
        &mut ExactAdmission::retaining(context, progress, retained),
    )?;
    // A source endpoint or crossing can join two opposing polynomial
    // branches. Prefer a proved pointwise reflection there; an unreflected
    // joined line through the critical point need not be injective.
    let reflected = if !matches!(intersection, crate::SegmentIntersection::Collinear { .. }) {
        a.reflected_source_towards_in(b, context, progress, retained)?
    } else {
        None
    };
    let a = reflected.as_ref().unwrap_or(a);
    let (a0, a1) = a.source_endpoints();
    if reflected.is_some() {
        intersection = crate::topology::intersect_admitted(
            a0,
            a1,
            b0,
            b1,
            &mut ExactAdmission::retaining(context, progress, retained),
        )?;
    }
    if matches!(intersection, crate::SegmentIntersection::None) {
        return Ok(None);
    }
    let Some(image) = a.joined_source_in(b, context, progress, retained)? else {
        return Ok(None);
    };
    let (start, end) = image.source_endpoints();
    let mut parameters = Vec::new();
    context.retain_workspace(4 * size_of::<Rat>() as u64, retained)?;
    parameters
        .try_reserve_exact(4)
        .map_err(|_| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })?;
    let mut admission = ExactAdmission::new(context, progress);
    for point in [a0, a1, b0, b1] {
        parameters.push(
            SourceLinearEdge::parameter_on_coord_retained(
                start,
                end,
                point,
                &mut admission,
                retained,
            )?
            .ok_or_else(|| GeoError::domain("proved common original source parameter"))?,
        );
    }
    let [a0, a1, b0, b1]: [Rat; 4] = parameters.try_into().expect("four source parameters");
    let mut parameter = |point: &crate::Coord, edge: &OperationImageCurve| {
        let (start, end) = edge.source_endpoints();
        SourceLinearEdge::parameter_on_coord_retained(start, end, point, &mut admission, retained)?
            .ok_or_else(|| GeoError::domain("proved common original source contact"))
    };
    let contact = match intersection {
        crate::SegmentIntersection::Point(point) => CurveIntersection::SymbolicSourceContact {
            left: parameter(&point, a)?,
            right: parameter(&point, b)?,
        },
        crate::SegmentIntersection::Collinear { from, to } => {
            let mut left = (parameter(&from, a)?, parameter(&to, a)?);
            let mut right = (parameter(&from, b)?, parameter(&to, b)?);
            if admission.compare(&left.0, &left.1)?.is_gt() {
                left = (left.1, left.0);
                right = (right.1, right.0);
            }
            CurveIntersection::BranchOverlap { left, right }
        }
        crate::SegmentIntersection::None => unreachable!("excluded source separation"),
    };
    let analytic = image.analytic_in_parameter(context, progress)?;
    context.retain_workspace(size_of::<OperationImageCurve>() as u64, retained)?;
    Ok(Some(SourceJoin {
        line: ChartLine::Transformed {
            curve: Box::new(image),
            subdivisions: context.policy().limits().max_subdivision_levels,
        },
        parameters: [[a0, a1], [b0, b1]],
        analytic,
        contact,
    }))
}

/// A complete injective joined image carries the original source intersection
/// exactly. This proves a symbolic shared subcurve without inventing target
/// coordinates or treating interval residual zero as equality.
pub(super) fn complete(
    source: &SourceJoin,
    edges: [&PreparedEdge; 2],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let policy = context.policy();
    super::with_line_refs(
        &edges,
        80.min(policy.limits().max_precision_bits),
        context,
        progress,
        |_, math, progress| {
            let result = (|| {
                let unit = FixedInterval::from_i64(0, math)?
                    .hull(&FixedInterval::from_i64(1, math)?, math)?;
                injective_on(
                    source,
                    &unit,
                    policy.limits().max_iterations,
                    math,
                    progress,
                )
            })();
            result.map_err(|error| geo_math_error(&error, policy))
        },
    )
}

pub(super) fn into_contact(source: SourceJoin) -> CurveIntersection {
    source.contact
}

pub(super) fn complete_local(
    source: &SourceJoin,
    domain: &RootBox2,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, MathError> {
    let map = |parameters: &[Rat; 2], parameter: &FixedInterval, math: &mut CoordinateMath| {
        let a = fixed_from_rat(&parameters[0], math)?;
        let delta = fixed_from_rat(&parameters[1], math)?.sub(&a, math)?;
        a.add(&delta.mul(parameter, math)?, math)
    };
    let ranges = [
        map(&source.parameters[0], &domain[0], math)?,
        map(&source.parameters[1], &domain[1], math)?,
    ];
    if ranges[0].upper() > ranges[1].lower() && ranges[1].upper() > ranges[0].lower() {
        return Ok(false);
    }
    let unit = FixedInterval::from_i64(0, math)?.hull(&FixedInterval::from_i64(1, math)?, math)?;
    // The exact original affine parameter maps lie in the joined source.
    // Intersecting their outward numerical hull with that proven domain
    // removes arithmetic spill beyond the native [0,1] parameter law.
    let hull = ranges[0]
        .hull(&ranges[1], math)?
        .intersection(&unit, math)?
        .ok_or(MathError::PrecisionExhausted)?;
    injective_on(source, &hull, iterations, math, progress)
}

pub(super) fn injective_on(
    source: &SourceJoin,
    hull: &FixedInterval,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, MathError> {
    let (_, derivative) = source.line.normal(hull, iterations, math, progress)?;
    crate::atlas::point::injective_component(
        &source.line,
        derivative.as_ref(),
        (hull, source.analytic, iterations),
        math,
        progress,
    )
}

pub(super) fn contains_source_endpoint(
    domain: &RootBox2,
    endpoints: &[CurveIntersection],
    math: &mut CoordinateMath,
) -> Result<bool, MathError> {
    for endpoint in endpoints {
        let (CurveIntersection::SymbolicEndpoint { left, right }
        | CurveIntersection::SymbolicSourceContact { left, right }) = endpoint
        else {
            continue;
        };
        let values = [fixed_from_rat(left, math)?, fixed_from_rat(right, math)?];
        if values.iter().zip(domain).all(|(point, domain)| {
            domain.lower() <= point.lower() && point.upper() <= domain.upper()
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numerical::frozen_decimal;
    use crate::operation::{
        CoordinateOperation, CoordinateUnit, OperationChain, OperationModel, OperationReference,
        Polynomial2d, PolynomialTerm,
    };
    use crate::{Coord, ExecutionLimits, ExecutionPolicy};
    use purrdf_hash::hex::Digest32;
    use std::sync::Arc;

    #[test]
    fn reflected_polynomial_walls_keep_complete_original_overlap_parameters() {
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 4_000_000;
        let mut context = MetricContext::new(
            crate::GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let projected = OperationReference {
            realization: Digest32::new([7; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        };
        let polynomial = Polynomial2d::new(
            2,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            vec![
                PolynomialTerm {
                    x_power: 2,
                    y_power: 0,
                    x_coefficient: Rat::one(),
                    y_coefficient: Rat::zero(),
                },
                PolynomialTerm {
                    x_power: 0,
                    y_power: 1,
                    x_coefficient: Rat::zero(),
                    y_coefficient: Rat::one(),
                },
            ],
        )
        .unwrap();
        let chain = Arc::new(
            OperationChain::compile(vec![
                CoordinateOperation::compile(
                    projected,
                    projected,
                    OperationModel::Polynomial2d(polynomial),
                )
                .unwrap(),
                CoordinateOperation::compile(
                    projected,
                    OperationReference {
                        realization: context.reference().id().digest(),
                        unit: CoordinateUnit::Degrees,
                        swapped_axes: false,
                    },
                    OperationModel::MercatorToGeographic {
                        radius: Rat::one(),
                        eccentricity_squared: Rat::zero(),
                        square_domain: true,
                    },
                )
                .unwrap(),
            ])
            .unwrap(),
        );
        let edge = |points: [(&str, &str); 2]| {
            PreparedEdge::Transformed(Box::new(
                OperationImageCurve::new(
                    Coord::xy(frozen_decimal(points[0].0), frozen_decimal(points[0].1)),
                    Coord::xy(frozen_decimal(points[1].0), frozen_decimal(points[1].1)),
                    chain.clone(),
                    context.reference().clone(),
                    None,
                    context.policy(),
                )
                .unwrap(),
            ))
        };
        let full = edge([("-0.2", "-0.2"), ("-0.2", "0.2")]);
        let reverse = edge([("0.2", "0.2"), ("0.2", "-0.2")]);
        let partial = edge([("0.2", "0.1"), ("0.2", "-0.1")]);
        let distinct = edge([("0.21", "-0.2"), ("0.21", "0.2")]);
        let first = edge([("-0.2", "-0.2"), ("0.2", "0.2")]);
        let second = edge([("0.2", "-0.2"), ("-0.2", "0.2")]);
        let negative = edge([("-0.2", "-0.2"), ("0", "-0.2")]);
        let positive = edge([("0", "-0.2"), ("0.2", "-0.2")]);
        let original_id = match &full {
            PreparedEdge::Transformed(image) => image.id(),
            _ => unreachable!(),
        };
        assert_eq!(
            super::super::intersections(&full, &reverse, &mut context).unwrap(),
            vec![CurveIntersection::BranchOverlap {
                left: (Rat::zero(), Rat::one()),
                right: (Rat::one(), Rat::zero()),
            }]
        );
        let quarter = frozen_decimal("0.25");
        let three_quarters = frozen_decimal("0.75");
        assert_eq!(
            super::super::intersections(&full, &partial, &mut context).unwrap(),
            vec![CurveIntersection::BranchOverlap {
                left: (quarter.clone(), three_quarters.clone()),
                right: (Rat::one(), Rat::zero()),
            }]
        );
        assert_eq!(
            super::super::intersections(&partial, &full, &mut context).unwrap(),
            vec![CurveIntersection::BranchOverlap {
                left: (Rat::zero(), Rat::one()),
                right: (three_quarters, quarter),
            }]
        );
        assert_eq!(
            super::super::intersections(&full, &distinct, &mut context).unwrap(),
            []
        );
        // The unreflected original source lines cross at the origin, while
        // their complete images coincide. A source point contact must not
        // prevent the independent global polynomial reflection certificate.
        assert_eq!(
            super::super::intersections(&first, &second, &mut context).unwrap(),
            vec![CurveIntersection::BranchOverlap {
                left: (Rat::zero(), Rat::one()),
                right: (Rat::zero(), Rat::one()),
            }]
        );
        assert_eq!(
            super::super::intersections(&negative, &positive, &mut context).unwrap(),
            vec![CurveIntersection::BranchOverlap {
                left: (Rat::zero(), Rat::one()),
                right: (Rat::one(), Rat::zero()),
            }]
        );
        assert_eq!(
            match &full {
                PreparedEdge::Transformed(image) => image.id(),
                _ => unreachable!(),
            },
            original_id
        );
    }
}
