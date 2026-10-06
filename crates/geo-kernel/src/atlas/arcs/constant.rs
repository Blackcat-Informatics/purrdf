// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete original-law constant-image contacts, never rank-deficient roots.

use super::{CurveIntersection, admit_contact_count, axis_overlap};
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, exact_rational};
use crate::{GeoError, LonLat, MetricContext, PreparedEdge, Rat, SourceLinearEdge};
use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd, RationalCompare};
use std::borrow::Cow;

struct ConstantImage<'a> {
    point: Cow<'a, LonLat>,
    retained: u64,
}

fn constant_image<'a>(
    edge: &'a PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ConstantImage<'a>>, GeoError> {
    if let Some(point) = constant_point(edge, context, progress)? {
        return Ok(Some(ConstantImage {
            point: Cow::Borrowed(point),
            retained: 0,
        }));
    }
    let PreparedEdge::Transformed(image) = edge else {
        return Ok(None);
    };
    let policy = context.policy();
    let (coordinates, retained) = super::with_line_refs_retained(
        &[edge],
        80.min(policy.limits().max_precision_bits),
        false,
        context,
        progress,
        |_, math, progress| {
            let result = (|| {
                let enclosure = match image.enclosure_in(
                    &Rat::zero(),
                    &Rat::one(),
                    crate::operation::OperationSolverLimits {
                        iterations: policy.limits().max_iterations,
                        subdivisions: policy.limits().max_subdivision_levels,
                        quantize_inverse: false,
                    },
                    math,
                    progress,
                ) {
                    Ok(enclosure) => enclosure,
                    // An unresolved angular chart cannot prove this entire
                    // image constant. The original smooth-normal contact
                    // worker must still certify the possible nonconstant
                    // pair, including panels that contain an exact pole.
                    Err(purrdf_xsd::math::MathError::PrecisionExhausted) => {
                        return Ok((None, 0));
                    }
                    Err(error) => return Err(error),
                };
                if enclosure.longitude.lower() != enclosure.longitude.upper()
                    || enclosure.latitude.lower() != enclosure.latitude.upper()
                {
                    return Ok((None, 0));
                }
                // A complete original degree image enclosed by two exact
                // singleton intervals is a mathematical point. No rounded
                // scalar result or equality of samples supplies this witness.
                let longitude = crate::numerical::exact_bounds_in(&enclosure.longitude, math)?.0;
                let latitude = crate::numerical::exact_bounds_in(&enclosure.latitude, math)?.0;
                crate::numerical::math_share_rationals([longitude, latitude], math, progress)
                    .map(|(coordinates, bytes)| (Some(coordinates), bytes))
            })();
            result.map_err(|error| crate::numerical::geo_math_error(&error, policy))
        },
    )?;
    let Some([longitude, latitude]) = coordinates else {
        return Ok(None);
    };
    let result = (|| {
        let longitude = crate::atlas::point::longitude_admitted(&longitude, context, progress)?;
        let cost = crate::numerical::rational_cost(RationalCompare, &[&longitude, &latitude], 4)
            .ok_or(GeoError::ArithmeticOverflow(
                "constant image point admission",
            ))?;
        progress.exact(context, cost, || LonLat::new(longitude, latitude))
    })();
    match result {
        Ok(point) => Ok(Some(ConstantImage {
            point: Cow::Owned(point),
            retained,
        })),
        Err(error) => {
            context.release_workspace(retained)?;
            Err(error)
        }
    }
}

pub(super) fn intersections(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Vec<CurveIntersection>>, GeoError> {
    let a = constant_image(left, context, progress)?;
    let b = match constant_image(right, context, progress) {
        Ok(value) => value,
        Err(error) => {
            let bytes = a.as_ref().map_or(0, |image| image.retained);
            drop(a);
            context.release_workspace(bytes)?;
            return Err(error);
        }
    };
    let retained = a
        .as_ref()
        .map_or(0, |image| image.retained)
        .checked_add(b.as_ref().map_or(0, |image| image.retained))
        .ok_or(GeoError::ArithmeticOverflow(
            "constant image point retention",
        ))?;
    let result = intersections_with_points(
        left,
        right,
        a.as_ref().map(|image| image.point.as_ref()),
        b.as_ref().map(|image| image.point.as_ref()),
        context,
        progress,
    );
    drop(a);
    drop(b);
    context.release_workspace(retained)?;
    result
}

fn intersections_with_points(
    left: &PreparedEdge,
    right: &PreparedEdge,
    a: Option<&LonLat>,
    b: Option<&LonLat>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Vec<CurveIntersection>>, GeoError> {
    let ((Some(point), _) | (None, Some(point))) = (a, b) else {
        return Ok(None);
    };
    let other = if a.is_some() { right } else { left };
    if let (Some(a), Some(b)) = (a, b) {
        let equal = ExactAdmission::new(context, progress).rational(
            Linear,
            &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
            8,
            || Ok(a.same_location(b)),
        )?;
        return if equal {
            contacts(
                point,
                true,
                vec![(Rat::zero(), Rat::one())],
                context,
                progress,
            )
            .map(Some)
        } else {
            Ok(Some(Vec::new()))
        };
    }
    let parameters = match other {
        PreparedEdge::SourceLinear(line) => linear_parameters(line, point, context, progress)?,
        PreparedEdge::ShortestGeodesic(arc) => {
            if !crate::atlas::point::contact_with_progress(other, point, context, progress)? {
                return Ok(Some(Vec::new()));
            }
            vec![axis_overlap::parameter(arc, point, context, progress)?]
        }
        PreparedEdge::AzimuthLength(arc) => {
            let Some(parameters) =
                crate::atlas::point::axis_contact_parameters(arc, point, context, progress)?
            else {
                return Ok(None);
            };
            parameters
        }
        PreparedEdge::Transformed(_) => {
            let Some(parameters) = crate::atlas::point::monotone_parameters(
                other,
                point,
                80.min(context.policy().limits().max_precision_bits),
                context,
                progress,
            )?
            else {
                return Ok(None);
            };
            parameters
        }
    };
    contacts(point, a.is_some(), parameters, context, progress).map(Some)
}

pub(super) fn constant_point<'a>(
    edge: &'a PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<&'a LonLat>, GeoError> {
    let Some(start) = edge.start() else {
        return Ok(None);
    };
    let start = start.point();
    let constant = match edge {
        PreparedEdge::SourceLinear(line) => {
            let end = line.end().point();
            ExactAdmission::new(context, progress).rational(
                Linear,
                &[
                    start.longitude(),
                    start.latitude(),
                    end.longitude(),
                    end.latitude(),
                ],
                8,
                || {
                    // A non-polar written -180→180 curve makes a full turn.
                    Ok(start == end || start.is_pole() && start.latitude() == end.latitude())
                },
            )?
        }
        PreparedEdge::ShortestGeodesic(arc) => {
            let end = arc.end().point();
            ExactAdmission::new(context, progress).rational(
                Linear,
                &[
                    start.longitude(),
                    start.latitude(),
                    end.longitude(),
                    end.latitude(),
                ],
                8,
                || Ok(start.same_location(end)),
            )?
        }
        PreparedEdge::AzimuthLength(arc) => arc.length().exact().is_zero(),
        PreparedEdge::Transformed(_) => false,
    };
    Ok(constant.then_some(start))
}

fn contacts(
    point: &LonLat,
    left_constant: bool,
    parameters: Vec<(Rat, Rat)>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    admit_contact_count(parameters.len() as u64, context)?;
    let parameter_bytes = parameters.iter().fold(
        (parameters.capacity() as u64).saturating_mul(size_of::<(Rat, Rat)>() as u64),
        |bytes, (lower, upper)| {
            bytes
                .saturating_add(lower.allocated_bytes() as u64)
                .saturating_add(upper.allocated_bytes() as u64)
        },
    );
    let point_bytes = (point.longitude().allocated_bytes() as u64)
        .checked_add(point.latitude().allocated_bytes() as u64)
        .ok_or(GeoError::ArithmeticOverflow("constant coordinate storage"))?;
    let bytes = (parameters.len() as u64)
        .checked_mul((size_of::<CurveIntersection>() as u64).saturating_add(point_bytes))
        .and_then(|bytes| bytes.checked_add(parameter_bytes))
        .ok_or(GeoError::ArithmeticOverflow("constant contact inventory"))?;
    context.admit_workspace(bytes)?;
    let result = (|| {
        let mut output = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let original = ExactAdmission::new(context, progress).rational(
                Linear,
                &[point.longitude(), point.latitude()],
                2,
                || Ok(point.clone()),
            )?;
            let whole = (Rat::zero(), Rat::one());
            let (left, right) = if left_constant {
                (whole, parameter)
            } else {
                (parameter, whole)
            };
            output.push(CurveIntersection::ConstantContact {
                left,
                right,
                point: original,
            });
        }
        Ok(output)
    })();
    context.release_workspace(bytes)?;
    result
}

fn linear_parameters(
    line: &SourceLinearEdge,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<(Rat, Rat)>, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    let mut parameters = Vec::new();
    for (parameter, endpoint) in [(0, line.start().point()), (1, line.end().point())] {
        if admission.rational(
            Linear,
            &[
                point.longitude(),
                point.latitude(),
                endpoint.longitude(),
                endpoint.latitude(),
            ],
            8,
            || Ok(point.same_location(endpoint)),
        )? {
            parameters.push(Rat::from_i64(parameter));
        }
    }
    if !point.is_pole() {
        for shift in [-360, 0, 360] {
            let offset = Rat::from_i64(shift);
            let longitude = exact_rational(
                Some(&mut admission),
                RationalAdd,
                &[point.longitude(), &offset],
                || point.longitude().add(&offset),
            )?;
            let admitted = admission.rational(RationalCompare, &[&longitude], 2, || {
                Ok(longitude >= Rat::from_i64(-180) && longitude <= Rat::from_i64(180))
            })?;
            if !admitted {
                continue;
            }
            let alias =
                admission.rational(RationalCompare, &[&longitude, point.latitude()], 8, || {
                    LonLat::new(longitude.clone(), point.latitude().clone())
                })?;
            if let Some(parameter) = SourceLinearEdge::parameter_on_admitted(
                line.start().point(),
                line.end().point(),
                &alias,
                &mut admission,
            )? {
                let mut repeated = false;
                for prior in &parameters {
                    if admission
                        .rational(Linear, &[prior, &parameter], 1, || Ok(prior == &parameter))?
                    {
                        repeated = true;
                        break;
                    }
                }
                if !repeated {
                    parameters.push(parameter);
                }
            }
        }
    }
    let borrowed = parameters.iter().collect::<Vec<_>>();
    let mut ordered = admission.rational(Linear, &borrowed, parameters.len() as u64, || {
        Ok(parameters.clone())
    })?;
    admission.rational(
        RationalCompare,
        &borrowed,
        (ordered.len() as u64).pow(2),
        || {
            ordered.sort_unstable();
            Ok(())
        },
    )?;
    admission.rational(Linear, &borrowed, ordered.len() as u64, || {
        Ok(ordered.into_iter().map(|p| (p.clone(), p)).collect())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AzimuthLengthArc, Metres};

    fn azimuth_arc(
        start: (i64, i64),
        azimuth: i64,
        length: i64,
        context: &mut MetricContext,
    ) -> PreparedEdge {
        PreparedEdge::AzimuthLength(Box::new(
            AzimuthLengthArc::new(
                super::super::tests::coordinate(start.0, start.1),
                Rat::from_i64(azimuth),
                Metres::new(Rat::from_i64(length)),
                context,
            )
            .unwrap(),
        ))
    }

    #[test]
    fn original_source_constants_and_full_turns_keep_different_parameter_laws() {
        use super::super::tests::{linear, shortest};
        let mut context = MetricContext::wgs84().unwrap();
        let fixed = linear((0, 0), (0, 0));
        let arc = shortest((-10, -10), (10, 10), &mut context);
        let contacts = super::super::intersections(&fixed, &arc, &mut context).unwrap();
        let [CurveIntersection::ConstantContact { left, right, .. }] = contacts.as_slice() else {
            panic!("one exact source constant against unique shortest")
        };
        let half = Rat::one().div(&Rat::from_i64(2)).unwrap();
        assert_eq!(left, &(Rat::zero(), Rat::one()));
        assert!(right.0 <= half && half <= right.1);
        let pole_row = linear((15, 90), (-70, 90));
        let meridian = shortest((35, 80), (35, 90), &mut context);
        let contacts = super::super::intersections(&pole_row, &meridian, &mut context).unwrap();
        assert!(
            matches!(contacts.as_slice(), [CurveIntersection::ConstantContact {left,right,point}]
            if left==&(Rat::zero(),Rat::one()) && right==&(Rat::one(),Rat::one()) && point.is_pole())
        );
        let circle = linear((-180, 0), (180, 0));
        let point = shortest((0, 0), (0, 0), &mut context);
        let contacts = super::super::intersections(&circle, &point, &mut context).unwrap();
        assert!(
            matches!(contacts.as_slice(), [CurveIntersection::ConstantContact {left,right,..}]
            if left==&(half.clone(),half) && right==&(Rat::zero(),Rat::one()))
        );
        let seam = shortest((180, 0), (-180, 0), &mut context);
        let contacts = super::super::intersections(&seam, &circle, &mut context).unwrap();
        assert_eq!(contacts.len(), 2);
        for (contact, parameter) in contacts.iter().zip([Rat::zero(), Rat::one()]) {
            assert!(
                matches!(contact, CurveIntersection::ConstantContact {left,right,..}
                if left==&(Rat::zero(),Rat::one()) && right==&(parameter.clone(),parameter))
            );
        }
        let zero = azimuth_arc((0, 0), 73, 0, &mut context);
        let contacts = super::super::intersections(&zero, &arc, &mut context).unwrap();
        assert!(
            matches!(contacts.as_slice(), [CurveIntersection::ConstantContact {left,..}]
            if left==&(Rat::zero(),Rat::one()))
        );
    }

    #[test]
    fn repeated_axis_arcs_publish_every_isolated_parameter_phase() {
        use super::super::tests::shortest;
        let mut context = MetricContext::wgs84().unwrap();
        for (start, azimuth, point) in [
            ((0, 0), 90, (90, 0)),
            ((0, 0), 270, (-90, 0)),
            ((25, 0), 0, (25, 45)),
            ((0, 90), 37, (123, 90)),
        ] {
            let selected = azimuth_arc(start, azimuth, 95_000_000, &mut context);
            let constant = shortest(point, point, &mut context);
            let contacts = super::super::intersections(&constant, &selected, &mut context).unwrap();
            assert_eq!(contacts.len(), 3, "complete repeated axial phases");
            let mut prior = None;
            for contact in contacts {
                let CurveIntersection::ConstantContact {
                    left,
                    right,
                    point: original,
                } = contact
                else {
                    panic!("one isolated nonconstant visit per receipt")
                };
                assert_eq!(left, (Rat::zero(), Rat::one()));
                assert!(right.0 <= right.1 && right.0 >= Rat::zero() && right.1 <= Rat::one());
                if let Some(prior) = prior {
                    assert!(prior < right.0);
                }
                prior = Some(right.1);
                assert!(original.same_location(
                    &LonLat::new(Rat::from_i64(point.0), Rat::from_i64(point.1)).unwrap()
                ));
            }
        }
    }

    #[test]
    fn repeated_phases_are_complete_or_return_typed_admission_failure() {
        use crate::{ExecutionPolicy, GeographicReference};
        let mut context = MetricContext::wgs84().unwrap();
        let selected = azimuth_arc((0, 0), 90, 95_000_000, &mut context);
        let point = super::super::tests::shortest((90, 0), (90, 0), &mut context);
        let expected = super::super::intersections(&point, &selected, &mut context).unwrap();
        for cap in [3, 4] {
            let mut limits = *ExecutionPolicy::geometry().limits();
            limits.max_output_elements = cap;
            let mut admitted = MetricContext::new(
                GeographicReference::wgs84(),
                ExecutionPolicy::new(limits).unwrap(),
            )
            .unwrap();
            assert_eq!(
                super::super::intersections(&point, &selected, &mut admitted).unwrap(),
                expected
            );
        }
        let mut limits = *ExecutionPolicy::geometry().limits();
        limits.max_output_elements = 2;
        let mut refused = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            super::super::intersections(&point, &selected, &mut refused),
            Err(GeoError::OutputExhausted { limit: 2 })
        ));
        limits.max_work_items = 1;
        let mut refused = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            super::super::intersections(&point, &selected, &mut refused),
            Err(GeoError::WorkExhausted { limit: 1 })
        ));
    }

    #[test]
    fn strict_principal_radius_bound_proves_general_azimuth_start_uniqueness() {
        let mut context = MetricContext::wgs84().unwrap();
        let selected = azimuth_arc((10, 20), 37, 2_000_000, &mut context);
        let point = super::super::tests::shortest((10, 20), (10, 20), &mut context);
        for (left, right, constant_first) in [(&point, &selected, true), (&selected, &point, false)]
        {
            let contacts = super::super::intersections(left, right, &mut context).unwrap();
            let [CurveIntersection::ConstantContact { left, right, .. }] = contacts.as_slice()
            else {
                panic!("one original start witness under proved injectivity")
            };
            let (constant, event) = if constant_first {
                (left, right)
            } else {
                (right, left)
            };
            assert_eq!(constant, &(Rat::zero(), Rat::one()));
            assert_eq!(event, &(Rat::zero(), Rat::zero()));
        }
    }
}
