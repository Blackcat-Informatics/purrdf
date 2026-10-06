// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable original auxiliary integral coefficients, separate from worker state.

use std::sync::Arc;

use purrdf_core::SmallVec;
use purrdf_xsd::math::{
    CoordinateMath, FixedInterval, FloatEnclosure, MathError, MathLimits, WordInterval,
};

use super::{ChunkObserver, IntegralCoefficients, PreparedGeodesic, SolveError};
use crate::numerical::{fixed_from_rat, geo_math_error};
use crate::{GeoError, GeographicReference, MetricContext, MetricWorkObserver};

/// Immutable tables retained by one worker; sequential children borrow this
/// container, while new precisions own and admit only their added allocation.
pub(crate) struct WorkerCoefficientCache {
    inverse_flattening: crate::Rat,
    max_parameter_bits: u64,
    bits: u32,
    area_prepared: bool,
    coefficients: Arc<IntegralCoefficients>,
    previous: Option<Arc<Self>>,
    count: usize,
    owned_bytes: u64,
}
purrdf_hash::debug_non_exhaustive!(WorkerCoefficientCache { bits, count });

/// Preparation requests share the same dimensionless integral equations.
/// Meridian-only use does not spend its admitted work on unrelated area tables.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CoefficientUse {
    Complete,
    Meridian,
}

impl WorkerCoefficientCache {
    pub(crate) fn lineage_work(&self) -> Option<u64> {
        u64::try_from(self.count).ok()
    }

    /// New child heads may retain their explicitly borrowed parent tail.
    /// Arc identity proves allocation ownership; coefficient equality never
    /// substitutes for that proof or merges unrelated sibling allocations.
    pub(crate) fn lineage_bytes(current: &Arc<Self>, ancestor: Option<&Arc<Self>>) -> Option<u64> {
        let mut bytes = 0_u64;
        let mut next = Some(current);
        while let Some(entry) = next {
            if ancestor.is_some_and(|ancestor| Arc::ptr_eq(entry, ancestor)) {
                return Some(bytes);
            }
            bytes = bytes.checked_add(entry.owned_bytes)?;
            next = entry.previous.as_ref();
        }
        ancestor.is_none().then_some(bytes)
    }

    const fn allocation_bytes() -> usize {
        size_of::<Self>() + 2 * size_of::<usize>()
    }

    pub(super) fn lookup_cost(
        &self,
        parameter: &crate::Rat,
    ) -> Option<purrdf_xsd::integer::ExactArithmeticCost> {
        purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            purrdf_xsd::integer::ExactOperation::Linear,
            self.max_parameter_bits
                .max(crate::numerical::rational_operand_bits(parameter)),
            u64::try_from(self.count).ok()?.checked_mul(4)?,
        )
    }

    pub(super) fn find(
        &self,
        inverse_flattening: &crate::Rat,
        bits: u32,
        order: usize,
        request: CoefficientUse,
    ) -> Option<&Arc<IntegralCoefficients>> {
        let mut next = Some(self);
        while let Some(entry) = next {
            if entry.bits == bits
                && entry.coefficients.square_root.len() == order + 1
                && entry.inverse_flattening == *inverse_flattening
                && (request == CoefficientUse::Meridian || entry.area_prepared)
            {
                return Some(&entry.coefficients);
            }
            next = entry.previous.as_deref();
        }
        None
    }
}

impl Drop for WorkerCoefficientCache {
    fn drop(&mut self) {
        // A caller may raise precision and workspace limits. Removing each
        // unique node's tail before its ordinary drop keeps destruction off
        // the machine stack; a shared tail remains owned by its other worker.
        let mut previous = self.previous.take();
        while let Some(node) = previous {
            let Some(mut owned) = Arc::into_inner(node) else {
                break;
            };
            previous = owned.previous.take();
        }
    }
}

impl IntegralCoefficients {
    fn detached_bytes(&self) -> usize {
        size_of::<Self>()
            + 2 * size_of::<usize>()
            + [&self.square_root, &self.reciprocal]
                .iter()
                .map(|values| {
                    if values.spilled() {
                        values.len() * size_of::<FixedInterval>()
                    } else {
                        0
                    }
                })
                .sum::<usize>()
            + core::iter::once(&self.c)
                .chain(self.square_root.iter())
                .chain(self.reciprocal.iter())
                .map(FixedInterval::detached_heap_bytes)
                .sum::<usize>()
            + self.area.as_ref().map_or(0, |area| area.detached_bytes())
    }

    fn detached(&self) -> Self {
        let copy = |values: &SmallVec<[FixedInterval; 9]>| {
            let mut result = SmallVec::with_capacity(values.len());
            result.extend(values.iter().map(FixedInterval::detached));
            result
        };
        Self {
            c: self.c.detached(),
            square_root: copy(&self.square_root),
            reciprocal: copy(&self.reciprocal),
            area: self.area.as_ref().map(|area| Arc::new(area.detached())),
        }
    }
}

#[derive(Debug)]
pub(super) struct PreparedCoefficients {
    bits: u32,
    fixed: Arc<IntegralCoefficients>,
    higher: Option<(u32, Arc<IntegralCoefficients>)>,
    pub(super) floating: IntegralCoefficients<FloatEnclosure>,
    pub(super) double: IntegralCoefficients<WordInterval>,
}

impl PreparedCoefficients {
    fn workspace_bytes(&self) -> usize {
        let mut bytes = size_of::<Self>() + 2 * size_of::<usize>();
        for coefficients in
            core::iter::once(&self.fixed).chain(self.higher.iter().map(|(_, value)| value))
        {
            bytes =
                bytes.saturating_add(size_of::<IntegralCoefficients>() + 2 * size_of::<usize>());
            bytes = bytes
                .saturating_add(if coefficients.square_root.spilled() {
                    coefficients.square_root.capacity() * size_of::<FixedInterval>()
                } else {
                    0
                })
                .saturating_add(if coefficients.reciprocal.spilled() {
                    coefficients.reciprocal.capacity() * size_of::<FixedInterval>()
                } else {
                    0
                });
            if let Some(area) = &coefficients.area {
                bytes = bytes.saturating_add(area.workspace_bytes());
            }
            for interval in core::iter::once(&coefficients.c)
                .chain(coefficients.square_root.iter())
                .chain(coefficients.reciprocal.iter())
            {
                bytes =
                    bytes.saturating_add(interval.workspace_bytes() - size_of::<FixedInterval>());
            }
        }
        bytes
    }
    pub(super) fn fixed_shared(
        &self,
        bits: u32,
        order: usize,
    ) -> Option<&Arc<IntegralCoefficients>> {
        let coefficients = if bits == self.bits {
            Some(&self.fixed)
        } else {
            self.higher
                .as_ref()
                .filter(|(precision, _)| *precision == bits)
                .map(|(_, value)| value)
        }?;
        (order == coefficients.square_root.len() - 1).then_some(coefficients)
    }
}

/// Generate and detach one complete original coefficient record. Cache keys
/// retain the exact flattening; interval overlap cannot alias different axes.
pub(super) fn prepare_worker_coefficients(
    ellipsoid: &crate::PreparedEllipsoid,
    bits: u32,
    order: usize,
    request: CoefficientUse,
    previous: Option<Arc<WorkerCoefficientCache>>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<(Arc<WorkerCoefficientCache>, Arc<IntegralCoefficients>, u64), SolveError> {
    let new_count = previous
        .as_ref()
        .map_or(Some(1), |value| value.count.checked_add(1))
        .ok_or(MathError::WorkspaceExhausted)?;
    let division = crate::numerical::rational_cost(
        purrdf_xsd::integer::ExactOperation::RationalDivide,
        &[ellipsoid.semiminor(), ellipsoid.semimajor()],
        1,
    )
    .ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(division)?;
    chunks.finish(math)?;
    let c = fixed_from_rat(
        &ellipsoid
            .semiminor()
            .div(ellipsoid.semimajor())
            .expect("positive prepared axes"),
        math,
    )?;
    let mut coefficients = IntegralCoefficients::new(&c, order, chunks, math)?;
    if request == CoefficientUse::Complete {
        let ep2 = FixedInterval::from_i64(1, math)?
            .sub(&c.square(math)?, math)?
            .div(&c.square(math)?, math)?;
        coefficients.area = super::inverse::area::prepare(&ep2, chunks, math)?.map(Arc::new);
    }
    let parameter = ellipsoid.inverse_flattening();
    let max_parameter_bits = previous
        .as_ref()
        .map_or(0, |value| value.max_parameter_bits)
        .max(crate::numerical::rational_operand_bits(parameter));
    let key_heap = parameter.shared_owned_bytes();
    let owned_bytes = coefficients
        .detached_bytes()
        .checked_add(WorkerCoefficientCache::allocation_bytes())
        .and_then(|value| value.checked_add(key_heap))
        .ok_or(MathError::WorkspaceExhausted)?;
    // Both the exact parameter copy and all coefficient copies are preparation.
    // Expose their complete storage and CPU admission before allocating them.
    let values = 2 * (order + 1)
        + 3
        + coefficients
            .area
            .as_ref()
            .map_or(0, |area| area.detached_values());
    let cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
        purrdf_xsd::integer::ExactOperation::Linear,
        u64::from(bits) + 64,
        u64::try_from(values)
            .map_err(|_| MathError::WorkExhausted)?
            .checked_mul(2)
            .ok_or(MathError::WorkExhausted)?,
    )
    .ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(cost)?;
    math.admit_exact_cost(
        crate::numerical::rational_cost(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[parameter],
            4,
        )
        .ok_or(MathError::WorkExhausted)?,
    )?;
    math.admit_exact(1, size_of::<WorkerCoefficientCache>() * 8)?;
    let copy_bytes = owned_bytes
        .checked_add(parameter.allocated_bytes())
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(copy_bytes)?;
    let result = (|| {
        chunks.finish(math)?;
        let detached = Arc::new(coefficients.detached());
        let cache = Arc::new(WorkerCoefficientCache {
            inverse_flattening: parameter.clone().into_shared(),
            max_parameter_bits,
            bits,
            area_prepared: request == CoefficientUse::Complete,
            coefficients: detached.clone(),
            previous,
            count: new_count,
            owned_bytes: owned_bytes as u64,
        });
        Ok((cache, detached, owned_bytes as u64))
    })();
    math.release_workspace(copy_bytes)?;
    result
}

impl PreparedGeodesic {
    pub(super) fn worker_coefficients(
        &self,
        bits: u32,
        order: usize,
        context: &mut MetricContext,
        chunks: &mut ChunkObserver<'_>,
    ) -> Result<Option<Arc<IntegralCoefficients>>, GeoError> {
        if let Some(value) = self
            .preparation
            .as_ref()
            .and_then(|value| value.fixed_shared(bits, order))
        {
            return Ok(Some(value.clone()));
        }
        if let Some(cache) = context.geodesic_coefficients() {
            let cost = cache
                .lookup_cost(self.reference.ellipsoid().inverse_flattening())
                .ok_or(GeoError::ArithmeticOverflow("prepared coefficient lookup"))?;
            context.charge_work(cost.work_items)?;
            context.admit_workspace(cost.workspace_bytes)?;
            let observed = chunks.context_entry(context);
            context.release_workspace(cost.workspace_bytes)?;
            observed.map_err(super::solve_external_error)?;
        }
        if let Some(value) = context.geodesic_coefficients().and_then(|value| {
            value.find(
                self.reference.ellipsoid().inverse_flattening(),
                bits,
                order,
                CoefficientUse::Complete,
            )
        }) {
            return Ok(Some(value.clone()));
        }
        // Ordinary scalar construction retains its established cheap path.
        // Long proofs prepare once, independently of their endpoint sources.
        if bits <= 112 {
            return Ok(None);
        }
        let policy = context.policy();
        chunks
            .context_entry(context)
            .map_err(super::solve_external_error)?;
        let mut math = context
            .coordinate_math(MathLimits {
                precision_bits: bits,
                max_work: policy
                    .limits()
                    .max_work_items
                    .saturating_sub(context.work_items()),
                max_workspace_bytes: usize::try_from(context.remaining_workspace()).map_err(
                    |_| GeoError::MemoryExhausted {
                        limit: policy.limits().max_workspace_bytes,
                    },
                )?,
            })
            .map_err(|error| geo_math_error(&error, policy))?;
        let interval_bytes = usize::try_from(bits.div_ceil(8))
            .unwrap_or(usize::MAX)
            .saturating_mul(8)
            .saturating_add(size_of::<FixedInterval>());
        let reservation =
            interval_bytes
                .checked_mul(2 * (order + 1) + 64)
                .ok_or(GeoError::MemoryExhausted {
                    limit: policy.limits().max_workspace_bytes,
                })?;
        math.reserve_workspace(reservation)
            .map_err(|error| geo_math_error(&error, policy))?;
        let result = prepare_worker_coefficients(
            self.reference.ellipsoid(),
            bits,
            order,
            CoefficientUse::Complete,
            context.geodesic_coefficients().cloned(),
            chunks,
            &mut math,
        );
        let observed = chunks.finish(&math);
        math.release_workspace(reservation)
            .map_err(|error| geo_math_error(&error, policy))?;
        context.charge_work(math.work_used())?;
        context.admit_workspace(math.workspace_peak() as u64)?;
        context.release_workspace(math.workspace_peak() as u64)?;
        observed.map_err(super::solve_external_error)?;
        let (cache, coefficients, bytes) = result.map_err(|error| match error {
            SolveError::Math(error) => geo_math_error(&error, policy),
            SolveError::External(error) => error,
            SolveError::Iterations => GeoError::ConvergenceExhausted { iterations: 0 },
        })?;
        context.retain_geodesic_coefficients(cache, bytes)?;
        Ok(Some(coefficients))
    }

    /// Bytes retained by this immutable auxiliary coefficient allocation,
    /// including the shared ownership header and endpoint limb capacities.
    /// Reference-only construction retains no numerical coefficient allocation.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> u64 {
        self.preparation
            .as_ref()
            .map_or(0, |prepared| prepared.workspace_bytes() as u64)
    }

    /// Prepare immutable auxiliary coefficients and arithmetic tables once.
    /// Clones share the same tables; each worker retains its own thread-bound
    /// context and scoped numerical capability.
    ///
    /// This has the same reference identity as [`Self::new`]. Preparation cost
    /// and its admitted workspace remain separate implementation evidence.
    ///
    /// # Errors
    /// Refuses reference mismatch, invalid floating environment, cancellation
    /// and numerical/resource exhaustion before publishing any preparation.
    pub fn prepare(
        reference: GeographicReference,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        Self::prepare_observed(reference, context, None)
    }

    /// Prepare the same immutable tables with bounded governor charging.
    ///
    /// # Errors
    /// Adds the external observer's refusals to [`Self::prepare`].
    pub fn prepare_metered(
        reference: GeographicReference,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_observed(reference, context, Some(observer))
    }

    fn prepare_observed(
        reference: GeographicReference,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin(0)?;
        context.charge_work(1)?;
        let mut chunks = ChunkObserver::new(observer);
        chunks.initial().map_err(super::solve_external_error)?;
        if !chunks.reference_matches(&reference, context)? {
            return Err(GeoError::config(
                "prepared reference and worker context differ",
            ));
        }
        chunks.reference_identity(Some(&reference), context)?;
        chunks.prepare_scratch(
            context,
            super::scratch_source_bits(&[], reference.ellipsoid()),
        )?;
        let policy = context.policy();
        let bits = policy.limits().max_precision_bits.min(80);
        let mut math = context
            .coordinate_math(MathLimits {
                precision_bits: bits,
                max_work: policy
                    .limits()
                    .max_work_items
                    .saturating_sub(context.work_items()),
                max_workspace_bytes: usize::try_from(context.remaining_workspace()).map_err(
                    |_| GeoError::MemoryExhausted {
                        limit: policy.limits().max_workspace_bytes,
                    },
                )?,
            })
            .map_err(|error| geo_math_error(&error, policy))?;
        let initial_workspace = math.workspace_peak();
        let retained = 2 * 9 * size_of::<FixedInterval>()
            + 2 * 9 * size_of::<FloatEnclosure>()
            + size_of::<PreparedCoefficients>()
            + 2 * size_of::<IntegralCoefficients>()
            + 2 * (super::integral_order(reference.ellipsoid(), 112) + 1)
                * size_of::<FixedInterval>()
            + 2 * (size_of::<super::inverse::area::PreparedArea>() + 2 * size_of::<usize>())
            + 128;
        math.reserve_workspace(retained)
            .map_err(|error| geo_math_error(&error, policy))?;
        chunks
            .context_entry(context)
            .map_err(super::solve_external_error)?;
        let result = (|| {
            context.install_arithmetic(&mut math)?;
            let c = fixed_from_rat(
                &reference
                    .ellipsoid()
                    .semiminor()
                    .div(reference.ellipsoid().semimajor())
                    .expect("positive prepared axes"),
                &mut math,
            )?;
            let mut fixed = IntegralCoefficients::new(&c, 8, &mut chunks, &mut math)?;
            let ep2 = FixedInterval::from_i64(1, &mut math)?
                .sub(&c.square(&mut math)?, &mut math)?
                .div(&c.square(&mut math)?, &mut math)?;
            fixed.area = super::inverse::area::prepare(&ep2, &mut chunks, &mut math)?.map(Arc::new);
            let convert = |source: &SmallVec<[FixedInterval; 9]>, math: &mut CoordinateMath| {
                let mut result: SmallVec<[FloatEnclosure; 9]> =
                    SmallVec::with_capacity(source.len());
                for coefficient in source {
                    result.push(FloatEnclosure::from_fixed(coefficient, math)?);
                }
                Ok::<_, MathError>(result)
            };
            let floating = IntegralCoefficients {
                c: FloatEnclosure::from_fixed(&c, &mut math)?,
                square_root: convert(&fixed.square_root, &mut math)?,
                reciprocal: convert(&fixed.reciprocal, &mut math)?,
                area: None,
            };
            let widen = |source: &SmallVec<[FixedInterval; 9]>, math: &mut CoordinateMath| {
                let mut result: SmallVec<[WordInterval; 9]> = SmallVec::with_capacity(source.len());
                for coefficient in source {
                    result.push(WordInterval::from_fixed(coefficient, math)?);
                }
                Ok::<_, MathError>(result)
            };
            let double = IntegralCoefficients {
                c: WordInterval::from_fixed(&c, &mut math)?,
                square_root: widen(&fixed.square_root, &mut math)?,
                reciprocal: widen(&fixed.reciprocal, &mut math)?,
                area: None,
            };
            let higher = if policy.limits().max_precision_bits >= 112 {
                // Both preparation contexts are live. Use the original outer
                // scratch allowance and retained reservation as the higher
                // phase's workspace base, including cancellation before its
                // first coefficient is constructed.
                let outer_workspace = math
                    .workspace_reserved()
                    .checked_add(initial_workspace)
                    .ok_or(MathError::WorkspaceExhausted)?;
                let mut higher_math = context.coordinate_math(MathLimits {
                    precision_bits: 112,
                    max_work: policy
                        .limits()
                        .max_work_items
                        .saturating_sub(context.work_items())
                        .saturating_sub(math.work_used()),
                    max_workspace_bytes: usize::try_from(context.remaining_workspace())
                        .map_err(|_| MathError::WorkspaceExhausted)?
                        .checked_sub(outer_workspace)
                        .ok_or(MathError::WorkspaceExhausted)?,
                })?;
                higher_math.reserve_workspace(retained)?;
                math.reserve_workspace(higher_math.workspace_peak())?;
                math.release_workspace(higher_math.workspace_peak())?;
                chunks.finish(&math)?;
                chunks.entry_workspace = chunks
                    .entry_workspace
                    .checked_add(outer_workspace as u64)
                    .ok_or(MathError::WorkspaceExhausted)?;
                chunks.entry_work = context.work_items().saturating_add(math.work_used());
                let order = super::integral_order(reference.ellipsoid(), 112);
                // Preparation is a separate bounded phase with the same cumulative
                // observer; no per-call coefficient arrays are cloned afterward.
                let result = (|| {
                    let c = fixed_from_rat(
                        &reference
                            .ellipsoid()
                            .semiminor()
                            .div(reference.ellipsoid().semimajor())
                            .expect("positive prepared axes"),
                        &mut higher_math,
                    )?;
                    let mut result =
                        IntegralCoefficients::new(&c, order, &mut chunks, &mut higher_math)?;
                    let ep2 = FixedInterval::from_i64(1, &mut higher_math)?
                        .sub(&c.square(&mut higher_math)?, &mut higher_math)?
                        .div(&c.square(&mut higher_math)?, &mut higher_math)?;
                    result.area =
                        super::inverse::area::prepare(&ep2, &mut chunks, &mut higher_math)?
                            .map(Arc::new);
                    Ok::<_, SolveError>(result)
                })();
                let observed = chunks.finish(&higher_math);
                math.admit_exact(higher_math.work_used(), 112)?;
                math.reserve_workspace(higher_math.workspace_peak())?;
                math.release_workspace(higher_math.workspace_peak())?;
                observed?;
                chunks.entry_work = context.work_items();
                chunks.entry_workspace = chunks
                    .entry_workspace
                    .checked_sub(outer_workspace as u64)
                    .ok_or(MathError::WorkspaceExhausted)?;
                Some((112, Arc::new(result?)))
            } else {
                None
            };
            Ok::<_, SolveError>(Arc::new(PreparedCoefficients {
                bits,
                fixed: Arc::new(fixed),
                higher,
                floating,
                double,
            }))
        })();
        let observer_result = if matches!(&result, Err(SolveError::External(_))) {
            Ok(())
        } else {
            chunks.finish(&math)
        };
        math.release_workspace(retained)
            .map_err(|error| geo_math_error(&error, policy))?;
        context.charge_work(math.work_used())?;
        context.admit_workspace(math.workspace_peak() as u64)?;
        context.release_workspace(math.workspace_peak() as u64)?;
        observer_result.map_err(super::solve_external_error)?;
        let preparation = result.map_err(|error| match error {
            SolveError::Math(error) => geo_math_error(&error, policy),
            SolveError::External(error) => error,
            SolveError::Iterations => GeoError::ConvergenceExhausted { iterations: 0 },
        })?;
        Ok(Self {
            reference,
            preparation: Some(preparation),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::PreparedGeodesic;
    use crate::{GeographicReference, LonLat, MetricContext, Rat};

    #[global_allocator]
    static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

    #[test]
    fn published_proofs_release_destinations_and_meter_owned_preparation_once() {
        #[derive(Default)]
        struct Count {
            work: u64,
            bytes: u64,
        }
        impl crate::MetricWorkObserver for Count {
            fn charge_chunk(&mut self, work: u64, bytes: u64) -> Result<(), crate::GeoError> {
                self.work += work;
                self.bytes += bytes;
                Ok(())
            }
        }
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let prepared = PreparedGeodesic::prepare(reference, &mut context).unwrap();
        let a = LonLat::new(Rat::zero(), Rat::parse_decimal("21.004101257892").unwrap()).unwrap();
        let b = LonLat::new(
            Rat::parse_decimal("179.43641220015808594").unwrap(),
            Rat::parse_decimal("-21.004101257877442853").unwrap(),
        )
        .unwrap();
        let expected = prepared.inverse(&a, &b, &mut context).unwrap();
        let mut count = Count::default();
        let (answer, proof) = prepared
            .inverse_with_proof_metered(&a, &b, &mut context, &mut count)
            .unwrap();
        assert_eq!(answer, expected);
        assert_eq!(proof.distance.work_items, context.work_items());
        assert_eq!(count.work, context.work_items());
        assert_eq!(count.bytes, context.workspace_peak());
        let scratch = context.integer_scratch().unwrap();
        assert_eq!(scratch.available(), scratch.destination_capacity());
        let proof_before = proof.clone();
        context.prepare_integer_scratch_for(8192).unwrap();
        assert_eq!(proof, proof_before);
        let (_, distance_proof) = prepared.distance_with_proof(&a, &b, &mut context).unwrap();
        let scratch = context.integer_scratch().unwrap();
        assert_eq!(scratch.available(), scratch.destination_capacity());
        drop(context);
        assert_eq!(proof, proof_before);
        assert!(distance_proof.lower <= distance_proof.upper);
    }

    #[test]
    fn zero_and_mixed_zero_batches_share_the_exact_bound_without_pinning_destinations() {
        let reference = GeographicReference::wgs84();
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 64_000_000,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        let prepared = PreparedGeodesic::prepare(reference, &mut context).unwrap();
        let origin = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        let neighbor = LonLat::new(Rat::parse_decimal("0.000001").unwrap(), Rat::zero()).unwrap();
        let zero = prepared.inverse(&origin, &origin, &mut context).unwrap();
        let nonzero = prepared.inverse(&origin, &neighbor, &mut context).unwrap();
        for mixed in [false, true] {
            let mut output = vec![None; 4096];
            let inputs = (0..4096).map(|i| {
                (
                    &origin,
                    if mixed && i % 2 != 0 {
                        &neighbor
                    } else {
                        &origin
                    },
                )
            });
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            prepared
                .inverse_batch_borrowed(inputs, &mut output, &mut context)
                .unwrap();
            assert_eq!(window.close().allocations, 0);
            for (i, answer) in output.iter().enumerate() {
                assert_eq!(
                    answer.as_ref().unwrap(),
                    if mixed && i % 2 != 0 { &nonzero } else { &zero }
                );
            }
            let scratch = context.integer_scratch().unwrap();
            assert_eq!(scratch.available(), scratch.destination_capacity());
        }
        let scratch = context.integer_scratch().unwrap();
        assert_eq!(scratch.available(), scratch.destination_capacity());
    }

    #[test]
    fn higher_worker_tables_preserve_answers_without_inner_allocations_or_pinned_destinations() {
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let prepared = PreparedGeodesic::prepare(reference, &mut context).unwrap();
        for (latitude1, latitude2, longitude, bits) in [
            (
                "21.004101257892",
                "-21.004101257877442853",
                "179.43641220015808594",
                224,
            ),
            (
                "7.326494029601",
                "-7.326494029600942956",
                "179.401396884582420845",
                448,
            ),
        ] {
            // Original public CC0 GeodTest cut-locus neighbors, not fitted
            // parameters. Their canonical fields are independent of caching.
            let a = LonLat::new(Rat::zero(), Rat::parse_decimal(latitude1).unwrap()).unwrap();
            let b = LonLat::new(
                Rat::parse_decimal(longitude).unwrap(),
                Rat::parse_decimal(latitude2).unwrap(),
            )
            .unwrap();
            // A projection may have prepared only meridian integrals at this
            // precision. Full inverse metadata must independently prepare its
            // area witness once, then reuse it without inner allocations.
            let policy = context.policy();
            let mut progress = crate::context::WorkProgress::new(None);
            crate::numerical::with_math(
                &mut context,
                &mut progress,
                bits,
                65_536,
                |math, progress| {
                    super::super::meridian_arc_observed(
                        a.latitude(),
                        prepared.reference().ellipsoid(),
                        math,
                        progress,
                    )
                    .map(|_| ())
                    .map_err(|error| crate::numerical::geo_math_error(&error, policy))
                },
            )
            .unwrap();
            let answer = prepared.inverse(&a, &b, &mut context).unwrap();
            let cache = context.geodesic_coefficients().unwrap();
            assert!(
                cache
                    .find(
                        prepared.reference().ellipsoid().inverse_flattening(),
                        bits,
                        super::super::integral_order(prepared.reference().ellipsoid(), bits),
                        super::CoefficientUse::Complete,
                    )
                    .is_some()
            );
            let scratch = context.integer_scratch().unwrap();
            assert_eq!(scratch.available(), scratch.destination_capacity());
            let retained = context.retained_workspace_bytes();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let warmed = prepared.inverse(&a, &b, &mut context).unwrap();
            let allocations = window.close();
            assert_eq!(warmed, answer);
            assert_eq!(allocations.allocations, 0, "{bits}-bit complete metadata");
            assert_eq!(context.retained_workspace_bytes(), retained);
            let scratch = context.integer_scratch().unwrap();
            assert_eq!(scratch.available(), scratch.destination_capacity());
        }
    }

    #[test]
    fn sequential_children_borrow_cached_tables_without_charging_the_owner_heap_twice() {
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let prepared = PreparedGeodesic::prepare(reference, &mut context).unwrap();
        let a = LonLat::new(Rat::zero(), Rat::parse_decimal("21.004101257892").unwrap()).unwrap();
        let b = LonLat::new(
            Rat::parse_decimal("179.43641220015808594").unwrap(),
            Rat::parse_decimal("-21.004101257877442853").unwrap(),
        )
        .unwrap();
        let expected = prepared.inverse(&a, &b, &mut context).unwrap();
        let parent_bytes = context.retained_workspace_bytes();
        let mut child = context.remaining_child().unwrap();
        assert_eq!(child.retained_workspace_bytes(), 0);
        assert!(std::sync::Arc::ptr_eq(
            context.geodesic_coefficients().unwrap(),
            child.geodesic_coefficients().unwrap()
        ));
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let answer = prepared.inverse(&a, &b, &mut child).unwrap();
        assert_eq!(window.close().allocations, 0);
        assert_eq!(answer, expected);
        assert_eq!(child.retained_workspace_bytes(), 0);
        assert_eq!(context.retained_workspace_bytes(), parent_bytes);
    }

    #[test]
    fn immutable_coefficients_preserve_completed_answers_and_identity() {
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let plain = PreparedGeodesic::new(reference.clone());
        let prepared = PreparedGeodesic::prepare(reference, &mut context).unwrap();
        assert_eq!(plain, prepared);
        assert_eq!(plain.retained_workspace_bytes(), 0);
        assert!(prepared.retained_workspace_bytes() > 0);
        let cloned = prepared.clone();
        assert!(std::sync::Arc::ptr_eq(
            prepared.preparation.as_ref().unwrap(),
            cloned.preparation.as_ref().unwrap()
        ));
        let a = LonLat::new(Rat::zero(), Rat::parse_decimal("36.530042355041").unwrap()).unwrap();
        let b = LonLat::new(
            Rat::parse_decimal("5.762344694676510456").unwrap(),
            Rat::parse_decimal("-48.164270779097768864").unwrap(),
        )
        .unwrap();
        let scalar = plain.distance(&a, &b, &mut context).unwrap();
        let cached = prepared.distance(&a, &b, &mut context).unwrap();
        assert_eq!(scalar, cached);
        assert_eq!(scalar.certificate_bytes(), cached.certificate_bytes());
        let scalar_inverse = plain.inverse(&a, &b, &mut context).unwrap();
        let cached_inverse = prepared.inverse(&a, &b, &mut context).unwrap();
        assert_eq!(scalar_inverse, cached_inverse);
        assert_eq!(
            scalar_inverse.certificate_bytes(),
            cached_inverse.certificate_bytes()
        );
        let azimuth = scalar_inverse.forward_azimuth().unwrap();
        let length = scalar_inverse.distance().value();
        let scalar_direct = plain.direct(&a, azimuth, length, &mut context).unwrap();
        let cached_direct = prepared.direct(&a, azimuth, length, &mut context).unwrap();
        assert_eq!(scalar_direct, cached_direct);
        assert_eq!(
            scalar_direct.certificate_bytes(),
            cached_direct.certificate_bytes()
        );
    }

    #[test]
    fn warmed_prepared_distance_inverse_and_direct_allocate_nothing() {
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let prepared = PreparedGeodesic::prepare(reference, &mut context).unwrap();
        let a = LonLat::new(Rat::zero(), Rat::parse_decimal("36.530042355041").unwrap()).unwrap();
        let b = LonLat::new(
            Rat::parse_decimal("5.762344694676510456").unwrap(),
            Rat::parse_decimal("-48.164270779097768864").unwrap(),
        )
        .unwrap();
        prepared.distance(&a, &b, &mut context).unwrap();
        let inverse = prepared.inverse(&a, &b, &mut context).unwrap();
        let azimuth = inverse.forward_azimuth().unwrap();
        let length = inverse.distance().value();
        prepared.direct(&a, azimuth, length, &mut context).unwrap();
        let instrument = purrdf_alloc_probe::CurrentThreadWindow::open();
        std::hint::black_box(Vec::<u8>::with_capacity(1024));
        assert_eq!(instrument.close().allocations, 1);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for _ in 0..16 {
            std::hint::black_box(prepared.distance(&a, &b, &mut context).unwrap());
            std::hint::black_box(prepared.inverse(&a, &b, &mut context).unwrap());
            std::hint::black_box(prepared.direct(&a, azimuth, length, &mut context).unwrap());
        }
        let counters = window.close();
        assert_eq!(counters.allocations, 0);
    }
}
