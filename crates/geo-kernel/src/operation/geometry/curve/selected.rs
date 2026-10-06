// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact original-parameter selected-set reductions. These certificates change
//! neither the retained public image nor its parameter-dependent metric law.

use super::super::OperationImageCurve;
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, reference_clone};
use crate::operation::{OperationModel, Polynomial2d};
use crate::{Coord, OperationChain};
use crate::{GeoError, MetricContext, Rat, SourceLinearEdge};
use purrdf_core::SmallVec;
use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd};
use std::sync::Arc;

mod range;
pub(super) use range::selected_range;

/// Reflect the original affine source about the polynomial normalization
/// origin. Even monomial parity proves equality at every original parameter;
/// the unchanged height, epoch and suffix preserve that pointwise equality.
pub(super) fn reflected_source_towards(
    image: &OperationImageCurve,
    other: &OperationImageCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<OperationImageCurve>, GeoError> {
    let operation = &image.chain.operations()[0];
    let OperationModel::Polynomial2d(polynomial) = operation.model() else {
        return Ok(None);
    };
    let (a, b) = image.source_endpoints();
    let (c, d) = other.source_endpoints();
    for flipped in [[true, false], [false, true], [true, true]] {
        let mut temporary = 0;
        let result = (|| {
            let mut admission = ExactAdmission::new(context, progress);
            let mut zero = [false; 2];
            let mut coordinates = [[a.x(), a.y()], [b.x(), b.y()]];
            if operation.source().swapped_axes {
                for value in &mut coordinates {
                    value.reverse();
                }
            }
            for axis in 0..2 {
                zero[axis] = admission
                    .compare(coordinates[0][axis], &polynomial.origin()[axis])?
                    .is_eq()
                    && admission
                        .compare(coordinates[1][axis], &polynomial.origin()[axis])?
                        .is_eq();
            }
            if !even_reflection(polynomial, flipped, zero, &mut admission)? {
                return Ok(None);
            }
            let reflect = |point: &Coord,
                           admission: &mut ExactAdmission<'_, '_>,
                           temporary: &mut u64| {
                // Fixed inline ordinates keep metadata construction allocation-free.
                let mut values = SmallVec::<[Rat; 2]>::new();
                for original_axis in 0..2 {
                    let model_axis = original_axis ^ usize::from(operation.source().swapped_axes);
                    let value = [point.x(), point.y()][original_axis];
                    let origin = &polynomial.origin()[model_axis];
                    let value = if flipped[model_axis] {
                        let twice = admission.rational_owner(
                            RationalAdd,
                            &[origin, origin],
                            temporary,
                            || origin.add(origin),
                        )?;
                        admission.rational_owner(
                            RationalAdd,
                            &[&twice, value],
                            temporary,
                            || twice.sub(value),
                        )?
                    } else {
                        admission.rational_owner(Linear, &[value], temporary, || value.clone())?
                    };
                    values.push(value);
                }
                let copy = |value: Option<&Rat>,
                            admission: &mut ExactAdmission<'_, '_>,
                            temporary: &mut u64| {
                    value
                        .map(|value| {
                            admission.rational_owner(Linear, &[value], temporary, || value.clone())
                        })
                        .transpose()
                };
                let y = values.pop().expect("second original ordinate");
                let x = values.pop().expect("first original ordinate");
                Ok::<_, GeoError>(Coord::new(
                    x,
                    y,
                    copy(point.z(), admission, temporary)?,
                    copy(point.m(), admission, temporary)?,
                ))
            };
            let start = reflect(a, &mut admission, &mut temporary)?;
            let end = reflect(b, &mut admission, &mut temporary)?;
            if matches!(
                crate::topology::intersect_admitted(
                    &start,
                    &end,
                    c,
                    d,
                    &mut ExactAdmission::retaining(context, progress, &mut temporary)
                )?,
                crate::SegmentIntersection::None
            ) {
                return Ok(None);
            }
            match prepare_image(
                image,
                start,
                end,
                image.chain.clone(),
                context,
                progress,
                &mut temporary,
            ) {
                // A reflected source outside the explicitly declared domain is
                // an unavailable optional proof; the original solver remains.
                Err(GeoError::Domain(_)) => Ok(None),
                result => result.map(Some),
            }
        })();
        if let Ok(Some(value)) = &result
            && let Err(error) = context.retain_workspace(value.retained_workspace_bytes(), retained)
        {
            drop(result);
            context.release_workspace(temporary)?;
            return Err(error);
        }
        context.release_workspace(temporary)?;
        if result.as_ref().is_err() || matches!(result, Ok(Some(_))) {
            return result;
        }
    }
    Ok(None)
}

fn even_reflection(
    polynomial: &Polynomial2d,
    flipped: [bool; 2],
    zero: [bool; 2],
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<bool, GeoError> {
    for term in polynomial.terms() {
        let active = admission.rational(
            Linear,
            &[&term.x_coefficient, &term.y_coefficient],
            2,
            || Ok(term.x_coefficient.signum() != 0 || term.y_coefficient.signum() != 0),
        )?;
        if !active || (zero[0] && term.x_power != 0) || (zero[1] && term.y_power != 0) {
            continue;
        }
        let degree = u32::from(term.x_power) * u32::from(flipped[0])
            + u32::from(term.y_power) * u32::from(flipped[1]);
        if !degree.is_multiple_of(2) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn restricted(
    image: &OperationImageCurve,
    lower: &Rat,
    upper: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<OperationImageCurve, GeoError> {
    let mut temporary = 0;
    let result = (|| {
        let (a, b) = image.source_endpoints();
        let mut admission = ExactAdmission::new(context, progress);
        let start = SourceLinearEdge::interpolate_coord_retained(
            a,
            b,
            lower,
            &mut admission,
            &mut temporary,
        )?;
        let end = SourceLinearEdge::interpolate_coord_retained(
            a,
            b,
            upper,
            &mut admission,
            &mut temporary,
        )?;
        prepare_image(
            image,
            start,
            end,
            image.chain.clone(),
            context,
            progress,
            &mut temporary,
        )
    })();
    if let Ok(value) = &result
        && let Err(error) = context.retain_workspace(value.retained_workspace_bytes(), retained)
    {
        drop(result);
        context.release_workspace(temporary)?;
        return Err(error);
    }
    context.release_workspace(temporary)?;
    result
}

pub(super) fn joined_source(
    image: &OperationImageCurve,
    other: &OperationImageCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<OperationImageCurve>, GeoError> {
    if !image.same_mapping_admitted(other, context, progress)? {
        return Ok(None);
    }
    let (a, b) = image.source_endpoints();
    let (c, d) = other.source_endpoints();
    let mut admission = ExactAdmission::new(context, progress);
    if crate::topology::orientation_admitted(a, b, c, &mut admission)? != 0
        || crate::topology::orientation_admitted(a, b, d, &mut admission)? != 0
    {
        return Ok(None);
    }
    let points = [a, b, c, d];
    let mut first = a;
    let mut last = a;
    for point in points {
        let compare = |a: &Coord, b: &Coord, admission: &mut ExactAdmission<'_, '_>| {
            let x = admission.compare(a.x(), b.x())?;
            if x.is_eq() {
                admission.compare(a.y(), b.y())
            } else {
                Ok(x)
            }
        };
        if compare(point, first, &mut admission)?.is_lt() {
            first = point;
        }
        if compare(point, last, &mut admission)?.is_gt() {
            last = point;
        }
    }
    for point in points {
        let Some(parameter) = SourceLinearEdge::parameter_on_coord_retained(
            first,
            last,
            point,
            &mut admission,
            retained,
        )?
        else {
            return Ok(None);
        };
        let z = match (first.z(), last.z()) {
            (Some(a), Some(b)) => Some(SourceLinearEdge::interpolate_ordinate_retained(
                a,
                b,
                &parameter,
                &mut admission,
                retained,
            )?),
            (None, None) => None,
            _ => return Ok(None),
        };
        let heights = z.iter().chain(point.z()).collect::<SmallVec<[&Rat; 2]>>();
        if !admission.rational(Linear, &heights, 1, || Ok(z.as_ref() == point.z()))? {
            return Ok(None);
        }
    }
    let start = SourceLinearEdge::interpolate_coord_retained(
        first,
        first,
        &Rat::zero(),
        &mut admission,
        retained,
    )?;
    let end = SourceLinearEdge::interpolate_coord_retained(
        last,
        last,
        &Rat::zero(),
        &mut admission,
        retained,
    )?;
    prepare_image(
        image,
        start,
        end,
        image.chain.clone(),
        context,
        progress,
        retained,
    )
    .map(Some)
}

pub(in super::super::super) fn prepare_image(
    image: &OperationImageCurve,
    start: Coord,
    end: Coord,
    chain: Arc<OperationChain>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<OperationImageCurve, GeoError> {
    let reference_cost = crate::numerical::reference_copy_cost(context.reference())?;
    progress.preflight_exact(context, reference_cost, reference_cost.workspace_bytes)?;
    context.retain_workspace(reference_cost.workspace_bytes, retained)?;
    let reference = reference_clone(context, progress)?;
    let epoch = if let Some(epoch) = image.epoch.as_ref() {
        Some(ExactAdmission::new(context, progress).rational_owner(
            Linear,
            &[epoch],
            retained,
            || epoch.clone(),
        )?)
    } else {
        None
    };
    let policy = context.remaining_child()?.policy();
    let result = {
        let mut observer = progress.nested(
            context.work_items(),
            context.current_workspace_bytes(),
            context.workspace_peak(),
        );
        OperationImageCurve::new_metered(start, end, chain, reference, epoch, policy, &mut observer)
    };
    progress.absorb_nested(context, result)
}

/// A polynomial reflection certificate proves the complete original image is
/// already covered by its second parameter half. Every active monomial has an
/// even sum of powers of the varying coordinates, which are centered at the
/// original source midpoint. Consequently P(t)=P(1-t) for every real t, not
/// merely at sampled endpoints. A constant actual height and the unchanged
/// suffix chain preserve that equality, even for bent or pole-crossing images.
pub(super) fn reflected_half(
    image: &OperationImageCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<OperationImageCurve>, GeoError> {
    let mut source_bytes = 0;
    let result = reflected_half_inner(image, context, progress, &mut source_bytes);
    if let Ok(Some(value)) = &result
        && let Err(error) = context.retain_workspace(value.retained_workspace_bytes(), retained)
    {
        drop(result);
        context.release_workspace(source_bytes)?;
        return Err(error);
    }
    context.release_workspace(source_bytes)?;
    result
}

fn reflected_half_inner(
    image: &OperationImageCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<OperationImageCurve>, GeoError> {
    context.charge_work(1)?;
    progress.context_poll(context)?;
    let operation = &image.chain.operations()[0];
    let OperationModel::Polynomial2d(polynomial) = operation.model() else {
        return Ok(None);
    };
    let (start, end) = image.source_endpoints();
    let mut admission = ExactAdmission::new(context, progress);
    let operands = [
        Some(start.x()),
        Some(start.y()),
        start.z(),
        Some(end.x()),
        Some(end.y()),
        end.z(),
    ]
    .into_iter()
    .flatten()
    .collect::<SmallVec<[&Rat; 6]>>();
    let same_height = admission.rational(Linear, &operands, 2, || Ok(start.z() == end.z()))?;
    if !same_height {
        return Ok(None);
    }
    let half = crate::numerical::frozen_decimal("0.5");
    let middle =
        SourceLinearEdge::interpolate_coord_retained(start, end, &half, &mut admission, retained)?;
    let mut coordinates = [
        [start.x(), start.y()],
        [end.x(), end.y()],
        [middle.x(), middle.y()],
    ];
    if operation.source().swapped_axes {
        for value in &mut coordinates {
            value.reverse();
        }
    }
    let [a, b, m] = coordinates;
    let mut varying = [false; 2];
    let mut zero = [false; 2];
    for axis in 0..2 {
        let origin = &polynomial.origin()[axis];
        varying[axis] =
            admission.rational(Linear, &[a[axis], b[axis]], 1, || Ok(a[axis] != b[axis]))?;
        let centered =
            admission.rational(Linear, &[m[axis], origin], 1, || Ok(m[axis] == origin))?;
        if varying[axis] && !centered {
            return Ok(None);
        }
        zero[axis] = !varying[axis] && centered;
    }
    if !varying.into_iter().any(|value| value) {
        return Ok(None);
    }
    if !even_reflection(polynomial, varying, zero, &mut admission)? {
        return Ok(None);
    }
    let end = SourceLinearEdge::interpolate_coord_retained(
        end,
        end,
        &Rat::zero(),
        &mut admission,
        retained,
    )?;
    // Every new source owner remains admitted until the original preparation
    // body consumes it and publishes the new immutable image's actual receipt.
    prepare_image(
        image,
        middle,
        end,
        image.chain.clone(),
        context,
        progress,
        retained,
    )
    .map(Some)
}
