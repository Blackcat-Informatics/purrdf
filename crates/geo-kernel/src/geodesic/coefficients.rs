// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one binomial/reciprocal coefficient generator for auxiliary integrals.

use purrdf_xsd::math::{CertifiedInterval, IntervalBound, IntervalContext, MathError};

use purrdf_core::SmallVec;

use super::{ChunkObserver, SolveError};

type CoefficientArray<I> = crate::context::Produced<SmallVec<[I; 9]>>;

pub(super) fn detached_array_bytes<const N: usize>(
    source: &SmallVec<[purrdf_xsd::math::FixedInterval; N]>,
    owner_bytes: usize,
) -> Result<usize, MathError> {
    let buffer = if source.len() > N {
        core::alloc::Layout::array::<purrdf_xsd::math::FixedInterval>(source.len())
            .map_err(|_| MathError::WorkspaceExhausted)?
            .size()
    } else {
        0
    };
    let mut bytes = buffer;
    for value in source {
        bytes = bytes
            .checked_add(
                value
                    .detached_required_bytes()
                    .map_err(|_| MathError::WorkspaceExhausted)?,
            )
            .ok_or(MathError::WorkspaceExhausted)?;
    }
    if bytes == 0 {
        Ok(0)
    } else {
        bytes
            .checked_add(owner_bytes)
            .ok_or(MathError::WorkspaceExhausted)
    }
}

pub(super) fn mapped_array<I, J, const N: usize>(
    source: &SmallVec<[I; N]>,
    chunks: &mut ChunkObserver<'_, '_>,
    mut convert: impl FnMut(&I) -> Result<J, MathError>,
) -> Result<crate::context::Produced<SmallVec<[J; N]>>, SolveError> {
    let mut values = chunks.array::<[J; N]>(source.len())?;
    for value in source {
        values.push(convert(value)?);
    }
    Ok(values)
}

pub(super) fn detached_array<const N: usize>(
    source: &SmallVec<[purrdf_xsd::math::FixedInterval; N]>,
    chunks: &mut ChunkObserver<'_, '_>,
) -> Result<crate::context::Produced<SmallVec<[purrdf_xsd::math::FixedInterval; N]>>, SolveError> {
    let make = |memory: &mut purrdf_lex::allocation::Memory<
        '_,
        dyn purrdf_lex::allocation::Admission + '_,
    >| {
        let mut values = SmallVec::<[purrdf_xsd::math::FixedInterval; N]>::new();
        values
            .reserve_exact_with_memory(source.len(), memory)
            .map_err(crate::GeoError::Storage)?;
        for value in source {
            values.push(
                value
                    .detached_with_memory(memory)
                    .map_err(crate::GeoError::Storage)?,
            );
        }
        Ok(values)
    };
    if detached_array_bytes(source, 0)? == 0 {
        // Both the array and every detached scalar have an empty native heap
        // layout. Detach through the original scalar home without manufacturing
        // a lease for nonexistent physical storage.
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(
            &mut resident as &mut dyn purrdf_lex::allocation::Admission,
        );
        let values = make(&mut memory).map_err(SolveError::External)?;
        return Ok(crate::context::Produced::inline_array(values));
    }
    chunks.produced(make, |values, original| {
        for value in values.iter_mut() {
            value
                .publish_detached(original)
                .map_err(crate::GeoError::Storage)?;
        }
        Ok(())
    })
}

/// Generate sqrt(1+z) and 1/(1+d sqrt(1+z)) coefficients from their equations.
/// The caller admits both complete arrays before entering this function.
pub(super) fn generate<I: CertifiedInterval>(
    d: &I,
    order: usize,
    chunks: &mut ChunkObserver<'_, '_>,
    math: &mut IntervalContext<'_>,
) -> Result<(CoefficientArray<I>, CoefficientArray<I>), SolveError> {
    if math.binary64_ops().is_none()
        && math.limits().precision_bits >= 192
        && let Some(scratch) = math.limb_scratch()
    {
        // Both arrays retain two endpoints per coefficient, while convolution
        // uses bounded temporary intervals. Include every currently live value
        // (including an outer coefficient pair during centered fallback).
        // All retained coefficients are bounded by one, so below 192 bits
        // their endpoints fit Int's three inline limbs and consume no slots.
        // In that case only the arithmetic's actual wide temporaries check out
        // destinations; do not impose a fictitious minimum array allocation.
        let destinations = order
            .checked_add(1)
            .and_then(|count| count.checked_mul(4))
            .and_then(|count| count.checked_add(64))
            .ok_or(MathError::WorkspaceExhausted)?;
        scratch
            .require_available(destinations)
            .map_err(MathError::LimbScratch)?;
    }
    let one = I::from_i64(1, math)?;
    let zero = I::from_i64(0, math)?;
    if d.upper() < zero.lower() {
        return Err(
            MathError::Domain("reciprocal coefficient parameter must be nonnegative").into(),
        );
    }
    // Both callers prove the actual parameter is nonnegative. Intersect with
    // that analytic domain if outward arithmetic straddles its boundary.
    let d = I::from_bounds(d.lower().max(zero.lower()).clone(), d.upper().clone(), math)?;
    let capacity = order.checked_add(1).ok_or(MathError::WorkspaceExhausted)?;
    let mut square_root = chunks.array::<[I; 9]>(capacity)?;
    let mut reciprocal = chunks.array::<[I; 9]>(capacity)?;
    square_root.push(one.clone());
    let denominator = one.add(&d, math)?;
    reciprocal.push(one.div(&denominator, math)?);
    let mut items = 0_u8;
    for n in 1..=order {
        let n_integer = i64::try_from(n).map_err(|_| MathError::WorkExhausted)?;
        let twice_n = n_integer.checked_mul(2).ok_or(MathError::WorkExhausted)?;
        let numerator = I::from_i64(
            3_i64.checked_sub(twice_n).ok_or(MathError::WorkExhausted)?,
            math,
        )?;
        let divisor = I::from_i64(twice_n, math)?;
        let coefficient = square_root[n - 1]
            .mul(&numerator, math)?
            .div(&divisor, math)?;
        // Every binomial coefficient has magnitude at most one.
        square_root.push(unit_coefficient(&coefficient, &one, math)?);
        let mut convolution = I::from_i64(0, math)?;
        for k in 1..=n {
            convolution = convolution.add(&square_root[k].mul(&reciprocal[n - k], math)?, math)?;
            items += 1;
            if items == 64 {
                chunks.finish_interval(math)?;
                items = 0;
            }
        }
        let coefficient = d
            .mul(&convolution, math)?
            .neg(math)?
            .div(&denominator, math)?;
        // For d>=0 this reciprocal is holomorphic on |z|<1, with modulus<=1.
        // Taking Cauchy radii up to one proves |coefficient|<=1 independently
        // of arithmetic width and prevents retained magnitudes from growing.
        reciprocal.push(unit_coefficient(&coefficient, &one, math)?);
        chunks.finish_interval(math)?;
    }
    Ok((square_root, reciprocal))
}

fn unit_coefficient<I: CertifiedInterval>(
    value: &I,
    one: &I,
    math: &mut IntervalContext<'_>,
) -> Result<I, MathError> {
    let lower = value.lower().max(&one.upper().negated()).clone();
    let upper = value.upper().min(one.upper()).clone();
    if lower > upper {
        return Err(MathError::PrecisionExhausted);
    }
    I::from_bounds(lower, upper, math)
}

#[cfg(test)]
mod tests {
    use purrdf_xsd::math::{FixedInterval, IntervalContext};

    use crate::context::WorkProgress;
    use crate::geodesic::{ChunkObserver, SolveError};
    use crate::{ExecutionLimits, ExecutionPolicy, GeographicReference, MetricContext};

    #[test]
    fn inline_coefficient_arrays_do_not_require_pooled_endpoints() {
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_scratch_destinations: 8,
            ..ExecutionLimits::UNBOUNDED
        })
        .unwrap();
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        crate::numerical::with_math(
            &mut context,
            &mut WorkProgress::new(None),
            80,
            0,
            |math, progress| {
                let result = (|| -> Result<(), SolveError> {
                    let one = FixedInterval::from_i64(1, math)?;
                    let two = FixedInterval::from_i64(2, math)?;
                    let d = one.div(&two, math)?;
                    let coefficients = super::generate(
                        &d,
                        128,
                        &mut ChunkObserver::borrowed(progress),
                        &mut IntervalContext::fixed(math),
                    )?;
                    assert_eq!(coefficients.0.len(), 129);
                    assert_eq!(coefficients.1.len(), 129);
                    let scratch = math.limb_scratch().unwrap();
                    assert_eq!(scratch.available(), scratch.destination_capacity());
                    Ok(())
                })();
                result.map_err(|error| match error {
                    SolveError::Math(error) => crate::numerical::geo_math_error(&error, policy),
                    SolveError::External(error) => error,
                    SolveError::Iterations => unreachable!("coefficient recurrence has no solver"),
                })
            },
        )
        .unwrap();
    }
}
