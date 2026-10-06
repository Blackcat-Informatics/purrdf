// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete polynomial prefix images on a constant output axis. Every source
//! panel is proved monotone or enclosed by attained extremal values. Continuity
//! then proves the entire closed interval, including every critical branch.

use super::{
    ExactAdmission, GeoError, Linear, MetricContext, OperationImageCurve, OperationModel, Rat,
    SmallVec, SourceLinearEdge, WorkProgress,
};
use crate::numerical::{geo_math_error, math_share_rationals, with_math_for_sources_retained};
use crate::operation::{CoordinateOperation, OperationChain, OperationSolverLimits};
use crate::{Coord, Int, PreparationBudget};
use purrdf_lex::walk::WorkList;
use purrdf_xsd::integer::ExactArithmeticCost;
use purrdf_xsd::math::{CoordinateMath, MathError};
use std::sync::Arc;

struct Range {
    coordinates: [Rat; 4],
    parameters: [Rat; 2],
}

pub(in super::super) fn selected_range(
    image: &OperationImageCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<OperationImageCurve>, GeoError> {
    context.charge_work(1)?;
    progress.context_poll(context)?;
    if image.chain.operations().len() == 1
        || !matches!(
            image.chain.operations()[0].model(),
            OperationModel::Polynomial2d(_)
        )
    {
        return Ok(None);
    }
    let (a, b) = image.source_endpoints();
    let heights = [a.z(), b.z()]
        .into_iter()
        .flatten()
        .collect::<SmallVec<[&Rat; 2]>>();
    if !ExactAdmission::new(context, progress)
        .rational(Linear, &heights, 1, || Ok(a.z() == b.z()))?
    {
        return Ok(None);
    }
    context.prepare_integer_scratch_for_observed(image.max_original_operand_bits(), progress)?;
    let policy = context.policy();
    let source = [
        Some(a.x()),
        Some(a.y()),
        a.z(),
        Some(b.x()),
        Some(b.y()),
        b.z(),
        image.epoch.as_ref(),
    ]
    .into_iter()
    .flatten()
    .collect::<SmallVec<[&Rat; 7]>>();
    let (range, bytes) = with_math_for_sources_retained(
        context,
        progress,
        112.min(policy.limits().max_precision_bits),
        8192,
        &source,
        |math, progress| {
            prove_range(image, policy.limits(), math, progress)
                .map_err(|error| geo_math_error(&error, policy))
        },
    )?;
    let Some(range) = range else {
        return Ok(None);
    };
    let mut temporary = bytes;
    let result = rebuild(image, range, context, progress, &mut temporary);
    if let Ok(Some(value)) = &result
        && let Err(error) = context.retain_workspace(value.retained_workspace_bytes(), retained)
    {
        drop(result);
        context.release_workspace(temporary)?;
        return Err(error);
    }
    context.release_workspace(temporary)?;
    result
}

fn dyadic(numerator: u128, depth: u32, math: &mut CoordinateMath) -> Result<Rat, MathError> {
    let cost = ExactArithmeticCost::dyadic_rational(u64::from(numerator.bit_width()), depth)
        .ok_or(MathError::WorkExhausted)?;
    math.admit_exact(
        cost.work_items,
        usize::try_from(cost.output_bits).map_err(|_| MathError::WorkspaceExhausted)?,
    )?;
    // Both magnitudes fit the native inline integer representation. No heap
    // destination escapes this factory; the shared dyadic body canonicalizes.
    Ok(Rat::from_dyadic_integer(&Int::from_u128(numerator), depth))
}

fn prefix(
    image: &OperationImageCurve,
    parameters: [&Rat; 2],
    limits: &crate::ExecutionLimits,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<super::super::super::PanelEnclosure, MathError> {
    image.panel_through_in(
        parameters[0],
        parameters[1],
        (
            OperationSolverLimits {
                iterations: limits.max_iterations,
                subdivisions: limits.max_subdivision_levels,
                quantize_inverse: false,
            },
            super::super::super::PanelKind::Directional,
            1,
        ),
        math,
        progress,
    )
}

fn prove_range(
    image: &OperationImageCurve,
    limits: &crate::ExecutionLimits,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<(Option<Range>, u64), MathError> {
    let zero = Rat::zero();
    let one = Rat::one();
    let whole = prefix(image, [&zero, &one], limits, math, progress)?;
    let Some(axis) = (0..2)
        .find(|axis| whole.coordinates[1 - axis].lower() == whole.coordinates[1 - axis].upper())
    else {
        return Ok((None, 0));
    };
    let first = prefix(image, [&zero; 2], limits, math, progress)?;
    let last = prefix(image, [&one; 2], limits, math, progress)?;
    if [
        first.coordinates[axis].clone(),
        last.coordinates[axis].clone(),
    ]
    .iter()
    .any(|value| value.lower() != value.upper())
    {
        return Ok((None, 0));
    }
    let mut lower = first.coordinates[axis].minimum(&last.coordinates[axis], math)?;
    let mut upper = first.coordinates[axis].maximum(&last.coordinates[axis], math)?;
    let ascending = first.coordinates[axis].lower() <= last.coordinates[axis].lower();
    let mut extrema = if ascending {
        [(0u128, 0u32), (1, 0)]
    } else {
        [(1, 0), (0, 0)]
    };
    let mut panels = WorkList::<(u128, u32), 128>::with((0, 0));
    while let Some((index, depth)) = panels.pop() {
        progress.math_poll(math)?;
        let a = dyadic(index, depth, math)?;
        let b = dyadic(index + 1, depth, math)?;
        let panel = prefix(image, [&a, &b], limits, math, progress)?;
        let Some(rate) = panel.directional.as_ref() else {
            return Err(MathError::PrecisionExhausted);
        };
        let monotone = !rate[axis].lower().is_negative()
            || rate[axis].upper().is_negative()
            || rate[axis].upper().is_zero();
        if monotone
            || (panel.coordinates[axis].lower() >= lower.lower()
                && panel.coordinates[axis].upper() <= upper.upper())
        {
            continue;
        }
        if depth >= limits.max_subdivision_levels || depth >= 127 {
            return Err(MathError::PrecisionExhausted);
        }
        let middle_index = index
            .checked_mul(2)
            .and_then(|index| index.checked_add(1))
            .ok_or(MathError::PrecisionExhausted)?;
        let middle = dyadic(middle_index, depth + 1, math)?;
        let value = prefix(image, [&middle; 2], limits, math, progress)?.coordinates[axis].clone();
        if value.lower() != value.upper() {
            return Err(MathError::PrecisionExhausted);
        }
        if value.lower() < lower.lower() {
            lower = value.clone();
            extrema[0] = (middle_index, depth + 1);
        }
        if value.upper() > upper.upper() {
            upper = value;
            extrema[1] = (middle_index, depth + 1);
        }
        panels.push((index * 2 + 1, depth + 1));
        panels.push((index * 2, depth + 1));
    }
    let constant = &whole.coordinates[1 - axis];
    let values = if axis == 0 {
        [&lower, constant, &upper, constant]
    } else {
        [constant, &lower, constant, &upper]
    };
    let coordinates =
        values.map(|value| crate::numerical::exact_bounds_in(value, math).map(|values| values.0));
    let [a, b, c, d] = coordinates;
    let parameters = [
        dyadic(extrema[0].0, extrema[0].1, math)?,
        dyadic(extrema[1].0, extrema[1].1, math)?,
    ];
    let ([a, b, c, d, t0, t1], bytes) = math_share_rationals(
        [a?, b?, c?, d?, parameters[0].clone(), parameters[1].clone()],
        math,
        progress,
    )?;
    Ok((
        Some(Range {
            coordinates: [a, b, c, d],
            parameters: [t0, t1],
        }),
        bytes,
    ))
}

fn rebuild(
    image: &OperationImageCurve,
    range: Range,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<OperationImageCurve>, GeoError> {
    let (start, end) = image.source_endpoints();
    let mut admission = ExactAdmission::new(context, progress);
    let first = SourceLinearEdge::interpolate_coord_retained(
        start,
        end,
        &range.parameters[0],
        &mut admission,
        retained,
    )?;
    let last = SourceLinearEdge::interpolate_coord_retained(
        start,
        end,
        &range.parameters[1],
        &mut admission,
        retained,
    )?;
    let [a, b, c, d] = range.coordinates;
    let mut copy_optional = |value: Option<&Rat>| {
        value
            .map(|value| admission.rational_owner(Linear, &[value], retained, || value.clone()))
            .transpose()
    };
    let first = Coord::new(a, b, copy_optional(first.z())?, copy_optional(first.m())?);
    let last = Coord::new(c, d, copy_optional(last.z())?, copy_optional(last.m())?);
    let count = image.chain.operations().len() - 1;
    context.retain_workspace(
        (count as u64)
            .checked_mul(size_of::<CoordinateOperation>() as u64)
            .ok_or(GeoError::ArithmeticOverflow("selected suffix storage"))?,
        retained,
    )?;
    context.charge_work(count as u64)?;
    progress.context_poll(context)?;
    let mut operations = Vec::new();
    operations
        .try_reserve_exact(count)
        .map_err(|_| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })?;
    for operation in &image.chain.operations()[1..] {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        operations.push(operation.clone());
    }
    let mut budget = PreparationBudget::new(context.remaining_child()?.policy());
    let result = {
        let mut observer = progress.nested(
            context.work_items(),
            context.current_workspace_bytes(),
            context.workspace_peak(),
        );
        OperationChain::compile_in_budget_metered(operations, &mut budget, &mut observer)
    };
    let chain = progress.absorb_nested(context, result)?;
    let chain_bytes = chain.retained_workspace_bytes()?;
    context.retain_workspace(chain_bytes, retained)?;
    let chain = Arc::new(chain);
    let mut value = super::prepare_image(image, first, last, chain, context, progress, retained)?;
    // The selected image creates its suffix owner; it is not an unrelated
    // caller-held chain. Its conservative canonical chain receipt survives
    // transfer into the combined graph's immutable source owner.
    value.retained_bytes = value
        .retained_bytes
        .checked_add(chain_bytes)
        .ok_or(GeoError::ArithmeticOverflow("selected image chain storage"))?;
    Ok(Some(value))
}
