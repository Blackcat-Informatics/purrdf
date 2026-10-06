// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Centered auxiliary-sphere integrals on the complete nonnegative-q domain.
//!
//! Set A=1+q/2 and k=q/(2+q). Then
//! sqrt(1+q sin²(sigma))=sqrt(A) sqrt(1-k cos(2 sigma)).
//! Here 0<=k<1 even when q>=1/2. Coefficients follow the binomial equation
//! and the reciprocal-product equation, with no imported tables. For a
//! circle of radius rho=(1+k_upper)/2<1, Cauchy's coefficient estimate bounds
//! the omitted integrals by 2 (k_upper/rho)^(N+1)/(1-k_upper/rho) |delta|.
//! The square-root integral also has the factor sqrt(A).
//!
//! Coefficient storage is admitted before allocation. Arithmetic uncertainty
//! or insufficient resource admission refuses; it never weakens the tail.

use purrdf_xsd::{
    BigInt,
    math::{CertifiedInterval, IntervalBound, IntervalContext, MathError},
};

#[cfg(test)]
use purrdf_xsd::math::{CoordinateMath, FixedInterval};

use super::{ChunkObserver, SolveError, coefficients};

#[cfg(test)]
pub(super) fn evaluate(
    sigma1: &FixedInterval,
    sigma2: &FixedInterval,
    q: &FixedInterval,
    c: &FixedInterval,
    order: usize,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<(FixedInterval, FixedInterval), SolveError> {
    evaluate_with(
        sigma1,
        sigma2,
        q,
        c,
        order,
        chunks,
        &mut IntervalContext::fixed(math),
    )
    .map(|(distance, longitude, _)| (distance, longitude))
}

pub(super) fn evaluate_with<I: CertifiedInterval>(
    sigma1: &I,
    sigma2: &I,
    q: &I,
    c: &I,
    order: usize,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<(I, I, I), SolveError> {
    let bits = [sigma1, sigma2, q, c]
        .iter()
        .flat_map(|value| [value.lower(), value.upper()])
        .map(IntervalBound::bits_upper_bound)
        .max()
        .unwrap_or(0)
        .max(math.limits().precision_bits as usize);
    let bytes_per_interval = bits
        .checked_add(4)
        .and_then(|bits| bits.div_ceil(8).checked_mul(8))
        .and_then(|bytes| bytes.checked_add(128))
        .ok_or(MathError::WorkspaceExhausted)?;
    let local_bytes = bytes_per_interval
        .checked_mul(128)
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(local_bytes)?;
    let inputs = CenteredInputs {
        sigma1,
        sigma2,
        q,
        c,
        minimum_order: order,
        bytes_per_interval,
    };
    let result = inputs.evaluate(chunks, math);
    math.release_workspace(local_bytes)?;
    result
}

struct CenteredInputs<'a, I: CertifiedInterval> {
    sigma1: &'a I,
    sigma2: &'a I,
    q: &'a I,
    c: &'a I,
    minimum_order: usize,
    bytes_per_interval: usize,
}

impl<I: CertifiedInterval> CenteredInputs<'_, I> {
    fn evaluate(
        &self,
        chunks: &mut ChunkObserver<'_>,
        math: &mut IntervalContext<'_>,
    ) -> Result<(I, I, I), SolveError> {
        let Self {
            sigma1,
            sigma2,
            q,
            c,
            minimum_order,
            bytes_per_interval,
        } = self;
        let zero = I::from_i64(0, math)?;
        let one = I::from_i64(1, math)?;
        let two = I::from_i64(2, math)?;
        if q.upper() < zero.lower() || c.upper() <= zero.lower() {
            return Err(MathError::Domain("centered integrals require q>=0 and c>0").into());
        }
        let nonnegative_q =
            I::from_bounds(q.lower().max(zero.lower()).clone(), q.upper().clone(), math)?;
        let a = one.add(&nonnegative_q.div(&two, math)?, math)?;
        let root_a = a.sqrt(math)?;
        // q/(2+q) is increasing on q>=0. Evaluate its endpoint images so
        // interval dependency cannot turn a broad finite q-range into k>=1.
        let q_lower = I::from_bounds(
            nonnegative_q.lower().clone(),
            nonnegative_q.lower().clone(),
            math,
        )?;
        let q_upper = I::from_bounds(
            nonnegative_q.upper().clone(),
            nonnegative_q.upper().clone(),
            math,
        )?;
        let k_lower = q_lower.div(&two.add(&q_lower, math)?, math)?;
        let k_upper = q_upper.div(&two.add(&q_upper, math)?, math)?;
        let k = I::from_bounds(k_lower.lower().clone(), k_upper.upper().clone(), math)?;
        if k.upper() >= one.lower() || c.lower() <= zero.upper() {
            return Err(MathError::PrecisionExhausted.into());
        }
        let delta = sigma2.sub(sigma1, math)?;
        if delta.lower().is_zero() && delta.upper().is_zero() {
            return Ok((zero.clone(), zero.clone(), zero));
        }
        if k.upper().is_zero() {
            return Ok((delta.clone(), delta.div(&one.add(c, math)?, math)?, zero));
        }

        let k_upper = I::from_bounds(k.upper().clone(), k.upper().clone(), math)?;
        let rho = one.add(&k_upper, math)?.div(&two, math)?;
        let ratio = k_upper.div(&rho, math)?;
        if ratio.upper() >= one.lower() {
            return Err(MathError::PrecisionExhausted.into());
        }
        // On |z|=rho, |sqrt(1+z)|<2 and
        // |1/sqrt(1+z)|<=1/sqrt(1-rho). This common tail bounds
        // distance, longitude and their Jacobi integral difference.
        let inverse_bound = one.div(&root_a.mul(&one.sub(&rho, math)?.sqrt(math)?, math)?, math)?;
        let base = delta
            .abs(math)?
            .mul(&root_a.mul(&two, math)?.add(&inverse_bound, math)?, math)?
            .div(&one.sub(&ratio, math)?, math)?;
        // The scaled integer 2^16 is exactly the target 2^(16-B). Reserving sixteen
        // precision bits for outward series arithmetic lets a higher-precision
        // invocation tighten this proof without changing any completed output law.
        let target = I::from_ratio(
            &BigInt::from_i128(1).mul_pow2(16),
            &BigInt::from_i128(1).mul_pow2(math.limits().precision_bits),
            math,
        )?;
        let mut power = one.clone();
        let mut order = 0_usize;
        let tail = loop {
            let next_power = power.mul(&ratio, math)?;
            let tail = base.mul(&next_power, math)?;
            let certified_small = tail.upper() <= target.lower();
            if order.is_multiple_of(64) {
                chunks.finish_interval(math)?;
            }
            if order >= *minimum_order && certified_small {
                break tail;
            }
            if next_power.upper() >= power.upper() && !certified_small {
                return Err(MathError::PrecisionExhausted.into());
            }
            power = next_power;
            order = order.checked_add(1).ok_or(MathError::WorkExhausted)?;
        };
        let coefficient_count = order.checked_add(1).ok_or(MathError::WorkspaceExhausted)?;
        let coefficient_bytes = coefficient_count
            .checked_mul(2)
            .and_then(|count| count.checked_mul(*bytes_per_interval))
            .ok_or(MathError::WorkspaceExhausted)?;
        math.reserve_workspace(coefficient_bytes)?;
        let series = CenteredSeries {
            delta,
            root_a,
            k,
            c,
            order,
            tail,
        };
        let result = series.evaluate(sigma1, sigma2, chunks, math);
        math.release_workspace(coefficient_bytes)?;
        result
    }
}

struct CenteredSeries<'a, I: CertifiedInterval> {
    delta: I,
    root_a: I,
    k: I,
    c: &'a I,
    order: usize,
    tail: I,
}

impl<I: CertifiedInterval> CenteredSeries<'_, I> {
    fn evaluate(
        &self,
        sigma1: &I,
        sigma2: &I,
        chunks: &mut ChunkObserver<'_>,
        math: &mut IntervalContext<'_>,
    ) -> Result<(I, I, I), SolveError> {
        let Self {
            delta,
            root_a,
            k,
            c,
            order,
            tail,
        } = self;
        let order = *order;
        let one = I::from_i64(1, math)?;
        let two = I::from_i64(2, math)?;
        let d = c.mul(root_a, math)?;
        let (square_root, reciprocal) = coefficients::generate(&d, order, chunks, math)?;

        let angle1 = sigma1.mul(&two, math)?;
        let angle2 = sigma2.mul(&two, math)?;
        let (sin1, cos1) = angle1.sin_cos(math)?;
        let (sin2, cos2) = angle2.sin_cos(math)?;
        let mut endpoint1 = sin1;
        let mut endpoint2 = sin2;
        let mut previous = delta.clone();
        let mut previous2 = delta.clone();
        let mut power = one;
        let negative_k = k.neg(math)?;
        let mut distance = delta.clone();
        let mut inverse_root = delta.clone();
        let mut longitude = delta.mul(&reciprocal[0], math)?;
        for n in 1..=order {
            let n_integer = i64::try_from(n).map_err(|_| MathError::WorkExhausted)?;
            let n_value = I::from_i64(n_integer, math)?;
            let even = n_value.mul(&two, math)?;
            let boundary = endpoint2.sub(&endpoint1, math)?.div(&even, math)?;
            let integral = if n == 1 {
                boundary
            } else {
                let previous_factor = I::from_i64(n_integer - 1, math)?.div(&n_value, math)?;
                boundary.add(&previous2.mul(&previous_factor, math)?, math)?
            };
            power = power.mul(&negative_k, math)?;
            let term = power.mul(&integral, math)?;
            distance = distance.add(&term.mul(&square_root[n], math)?, math)?;
            // binom(-1/2,n)=(1-2n)binom(1/2,n), so one generated
            // binomial array supplies both distance and Jacobi integrals.
            inverse_root = inverse_root.add(
                &term
                    .mul(&square_root[n], math)?
                    .mul(&I::from_i64(1 - 2 * n_integer, math)?, math)?,
                math,
            )?;
            longitude = longitude.add(&term.mul(&reciprocal[n], math)?, math)?;
            previous2 = previous;
            previous = integral;
            endpoint1 = endpoint1.mul(&cos1, math)?;
            endpoint2 = endpoint2.mul(&cos2, math)?;
            if n.is_multiple_of(64) {
                chunks.finish_interval(math)?;
            }
        }
        let padding = I::from_bounds(tail.upper().negated(), tail.upper().clone(), math)?;
        let distance = distance.mul(root_a, math)?;
        let jacobi = distance.sub(&inverse_root.div(root_a, math)?, math)?;
        Ok((
            distance.add(&padding, math)?,
            longitude.add(&padding, math)?,
            jacobi.add(&padding, math)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use purrdf_xsd::{
        BigInt,
        math::{CoordinateMath, FixedInterval, MathError, MathLimits},
    };

    use super::{ChunkObserver, SolveError};

    fn evaluate(
        sigma1: &FixedInterval,
        sigma2: &FixedInterval,
        q: &FixedInterval,
        c: &FixedInterval,
        order: usize,
        math: &mut CoordinateMath,
    ) -> Result<(FixedInterval, FixedInterval), SolveError> {
        super::evaluate(
            sigma1,
            sigma2,
            q,
            c,
            order,
            &mut ChunkObserver::new(None),
            math,
        )
    }

    fn math() -> CoordinateMath {
        CoordinateMath::new(MathLimits {
            precision_bits: 128,
            max_work: 4_000_000,
            max_workspace_bytes: 64 * 1024 * 1024,
        })
        .unwrap()
    }

    fn ratio(numerator: i128, denominator: i128, math: &mut CoordinateMath) -> FixedInterval {
        FixedInterval::from_ratio(
            &BigInt::from_i128(numerator),
            &BigInt::from_i128(denominator),
            math,
        )
        .unwrap()
    }

    // Independent quadratic AGM mathematics, DLMF equations 19.8.1--19.8.6:
    // https://dlmf.nist.gov/19.8.E6 . No centered powers or coefficient
    // convolution enter this oracle. M lies between each geometric/arithmetic
    // pair. The remaining positive E-sum is bounded below by zero and above
    // by eight sevenths of its next term: subsequent squared terms decrease
    // by at least 1/16 while their weight doubles.
    fn agm_complete_three(math: &mut CoordinateMath) -> (FixedInterval, FixedInterval) {
        let one = ratio(1, 1, math);
        let two = ratio(2, 1, math);
        let mut a = one.clone();
        let mut b = ratio(1, 2, math);
        let mut sum = ratio(3, 8, math);
        let mut weight = one.clone();
        for _ in 0..8 {
            let c = a.sub(&b, math).unwrap().div(&two, math).unwrap();
            sum = sum
                .add(&c.square(math).unwrap().mul(&weight, math).unwrap(), math)
                .unwrap();
            let next_a = a.add(&b, math).unwrap().div(&two, math).unwrap();
            b = a.mul(&b, math).unwrap().sqrt(math).unwrap();
            a = next_a;
            weight = weight.mul(&two, math).unwrap();
        }
        let next_c = a
            .sub(&b, math)
            .unwrap()
            .div(&two, math)
            .unwrap()
            .abs(math)
            .unwrap();
        let tail = next_c
            .square(math)
            .unwrap()
            .mul(&weight, math)
            .unwrap()
            .mul(&ratio(8, 7, math), math)
            .unwrap();
        let tail =
            FixedInterval::from_bounds(BigInt::from_i128(0), tail.upper().clone(), math).unwrap();
        sum = sum.add(&tail, math).unwrap();
        let mean = FixedInterval::from_bounds(b.lower().clone(), a.upper().clone(), math).unwrap();
        let k = FixedInterval::pi(math)
            .unwrap()
            .div(&two.mul(&mean, math).unwrap(), math)
            .unwrap();
        let e = k.mul(&one.sub(&sum, math).unwrap(), math).unwrap();
        // Reflection of sigma turns D=1+3 sin²sigma into 4(1-3/4 sin²sigma).
        // Integrating the rationalized reciprocal by parts gives J=(4E-K)/3.
        let distance = e.mul(&two, math).unwrap();
        let longitude = e
            .mul(&ratio(4, 1, math), math)
            .unwrap()
            .sub(&k, math)
            .unwrap()
            .div(&ratio(3, 1, math), math)
            .unwrap();
        (distance, longitude)
    }

    #[test]
    fn high_eccentricity_integrals_agree_with_independent_certified_agm() {
        let mut math = math();
        let zero = ratio(0, 1, &mut math);
        let half_pi = FixedInterval::pi(&mut math)
            .unwrap()
            .div(&ratio(2, 1, &mut math), &mut math)
            .unwrap();
        let q = ratio(3, 1, &mut math);
        let c = ratio(1, 2, &mut math);
        let retained_before = math.workspace_reserved();
        let actual = evaluate(&zero, &half_pi, &q, &c, 8, &mut math).unwrap();
        let expected = agm_complete_three(&mut math);
        for (actual, expected) in [(&actual.0, &expected.0), (&actual.1, &expected.1)] {
            assert!(
                actual.lower() <= expected.upper() && expected.lower() <= actual.upper(),
                "independent complete-integral enclosures must overlap"
            );
            let width = actual.upper().sub(actual.lower());
            assert!(
                width < BigInt::from_i128(1).mul_pow2(36),
                "certified width below 2^-92"
            );
        }
        assert_eq!(
            math.workspace_reserved(),
            retained_before,
            "every retained reservation released"
        );
        let reversed = evaluate(&half_pi, &zero, &q, &c, 8, &mut math).unwrap();
        for (forward, reverse) in [(&actual.0, &reversed.0), (&actual.1, &reversed.1)] {
            assert!(forward.lower() <= &reverse.lower().negated());
            assert!(&reverse.upper().negated() <= forward.upper());
        }
    }

    #[test]
    fn exact_flat_integrals_and_zero_extent_keep_analytic_values() {
        let mut math = math();
        let zero = ratio(0, 1, &mut math);
        let one = ratio(1, 1, &mut math);
        let c = ratio(1, 2, &mut math);
        let (distance, longitude) = evaluate(&zero, &one, &zero, &c, 8, &mut math).unwrap();
        assert_eq!(distance, one);
        assert_eq!(longitude, ratio(2, 3, &mut math));
        let q = ratio(3, 1, &mut math);
        let values: [FixedInterval; 2] =
            evaluate(&zero, &zero, &q, &c, 8, &mut math).unwrap().into();
        for value in values {
            assert!(value.lower().is_zero() && value.upper().is_zero());
        }
        let broad_q =
            FixedInterval::from_bounds(zero.lower().clone(), q.upper().clone(), &mut math).unwrap();
        let values: [FixedInterval; 2] = evaluate(&zero, &zero, &broad_q, &c, 8, &mut math)
            .unwrap()
            .into();
        for value in values {
            assert!(value.lower().is_zero() && value.upper().is_zero());
        }
        assert_eq!(math.workspace_reserved(), 0);
    }

    #[test]
    fn partial_integrals_fit_independent_monotone_rectangle_enclosures() {
        let mut math = math();
        let start = ratio(1, 4, &mut math);
        let end = ratio(3, 4, &mut math);
        let q = ratio(3, 1, &mut math);
        let c = ratio(1, 2, &mut math);
        let actual = evaluate(&start, &end, &q, &c, 8, &mut math).unwrap();
        let one = ratio(1, 1, &mut math);
        let width = ratio(1, 128, &mut math);
        let mut distance_lower = ratio(0, 1, &mut math);
        let mut distance_upper = distance_lower.clone();
        let mut longitude_lower = distance_lower.clone();
        let mut longitude_upper = distance_lower.clone();
        for bin in 0_i128..64 {
            let mut heights = Vec::with_capacity(2);
            for endpoint in [bin, bin + 1] {
                let angle = start
                    .add(
                        &width
                            .mul(&ratio(endpoint, 1, &mut math), &mut math)
                            .unwrap(),
                        &mut math,
                    )
                    .unwrap();
                let sin = angle.sin_cos(&mut math).unwrap().0;
                let root = one
                    .add(
                        &q.mul(&sin.square(&mut math).unwrap(), &mut math).unwrap(),
                        &mut math,
                    )
                    .unwrap()
                    .sqrt(&mut math)
                    .unwrap();
                let reciprocal = one
                    .div(
                        &one.add(&c.mul(&root, &mut math).unwrap(), &mut math)
                            .unwrap(),
                        &mut math,
                    )
                    .unwrap();
                heights.push((root, reciprocal));
            }
            distance_lower = distance_lower
                .add(&heights[0].0.mul(&width, &mut math).unwrap(), &mut math)
                .unwrap();
            distance_upper = distance_upper
                .add(&heights[1].0.mul(&width, &mut math).unwrap(), &mut math)
                .unwrap();
            longitude_lower = longitude_lower
                .add(&heights[1].1.mul(&width, &mut math).unwrap(), &mut math)
                .unwrap();
            longitude_upper = longitude_upper
                .add(&heights[0].1.mul(&width, &mut math).unwrap(), &mut math)
                .unwrap();
        }
        // On [1/4,3/4], sin² is increasing, so these rectangles certify
        // both integrals without a Taylor difference or a series recurrence.
        for (value, lower, upper) in [
            (actual.0, distance_lower, distance_upper),
            (actual.1, longitude_lower, longitude_upper),
        ] {
            assert!(lower.lower() <= value.lower() && value.upper() <= upper.upper());
        }
    }

    #[test]
    fn public_prepared_meridian_uses_the_complete_oblate_domain() {
        use crate::{
            AxisOrder, ExecutionPolicy, GeographicReference, LonLat, MetricContext,
            PreparedEllipsoid, Rat, geodesic::PreparedGeodesic,
        };
        let reference = GeographicReference::new(
            PreparedEllipsoid::new(Rat::from_i64(10), Rat::from_i64(2)).unwrap(),
            purrdf_hash::hex::Digest32::new([7; 32]),
            AxisOrder::LonLat,
        );
        let mut context =
            MetricContext::new(reference.clone(), ExecutionPolicy::geometry()).unwrap();
        let a = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        let b = LonLat::new(Rat::zero(), Rat::from_i64(90)).unwrap();
        let result = PreparedGeodesic::new(reference)
            .distance(&a, &b, &mut context)
            .unwrap();
        let mut oracle = math();
        let distance = agm_complete_three(&mut oracle)
            .0
            .mul(&ratio(5, 1, &mut oracle), &mut oracle)
            .unwrap();
        let (lower, upper) = crate::numerical::exact_bounds(&distance);
        assert_eq!(lower.round_to_scale(6), upper.round_to_scale(6));
        assert_eq!(result.quantized_micrometres(), &lower.round_to_scale(6));
    }

    #[test]
    fn fallback_refusals_release_retained_workspace() {
        let mut setup = math();
        let zero = ratio(0, 1, &mut setup);
        let one = ratio(1, 1, &mut setup);
        let q = ratio(3, 1, &mut setup);
        let c = ratio(1, 2, &mut setup);
        for (max_work, max_workspace_bytes, expected) in [
            (4_000_000, 16_384, MathError::WorkspaceExhausted),
            (100, 64 * 1024 * 1024, MathError::WorkExhausted),
        ] {
            let mut math = CoordinateMath::new(MathLimits {
                precision_bits: 128,
                max_work,
                max_workspace_bytes,
            })
            .unwrap();
            assert!(
                matches!(evaluate(&zero, &one, &q, &c, 8, &mut math), Err(SolveError::Math(error)) if error == expected)
            );
            assert_eq!(math.workspace_reserved(), 0);
        }
        let negative = ratio(-1, 1, &mut setup);
        assert!(matches!(
            evaluate(&zero, &one, &negative, &c, 8, &mut setup),
            Err(SolveError::Math(MathError::Domain(_)))
        ));
        assert_eq!(setup.workspace_reserved(), 0);
    }

    #[test]
    fn observer_cancellation_during_generation_releases_all_local_storage() {
        struct StopDuringGeneration {
            calls: usize,
            work: u64,
        }
        impl crate::MetricWorkObserver for StopDuringGeneration {
            fn charge_chunk(
                &mut self,
                work: u64,
                _workspace_growth: u64,
            ) -> Result<(), crate::GeoError> {
                self.calls += 1;
                self.work += work;
                if self.calls >= 7 {
                    Err(crate::GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        let mut math = math();
        let zero = ratio(0, 1, &mut math);
        let one = ratio(1, 1, &mut math);
        let q = ratio(3, 1, &mut math);
        let c = ratio(1, 2, &mut math);
        let mut observer = StopDuringGeneration { calls: 0, work: 0 };
        let mut chunks = ChunkObserver::new(Some(&mut observer));
        let result = super::evaluate(&zero, &one, &q, &c, 8, &mut chunks, &mut math);
        assert!(matches!(
            result,
            Err(SolveError::External(crate::GeoError::Cancelled))
        ));
        assert_eq!(observer.calls, 7);
        assert!(observer.work > 0);
        assert_eq!(math.workspace_reserved(), 0);
    }

    #[test]
    fn point_target_requires_both_rounding_and_half_micrometre_width() {
        let mut math = math();
        for (lower, upper, expected) in [
            (9_999_996, 10_000_004, false),
            (9_999_998, 10_000_002, true),
            (10_000_004, 10_000_006, false),
        ] {
            let lower = ratio(lower, 10_000_000, &mut math);
            let upper = ratio(upper, 10_000_000, &mut math);
            let enclosure =
                FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), &mut math)
                    .unwrap();
            assert_eq!(
                super::super::point_decisive(&enclosure, None, &mut math).unwrap(),
                expected
            );
        }
    }
}
