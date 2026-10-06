// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dimensioned exact quantities and frozen point-distance certificates.
//!
//! A completed estimate contains the quantized mathematical answer. A tighter
//! invocation enclosure belongs to a separate proof receipt and never changes
//! the completed certificate or its identity.

mod input;
mod report;
mod unit;
pub use input::{numeric_radius, numeric_radius_metered};
pub(crate) use report::reported_double_with_progress;
pub use report::{reported_double, reported_double_metered};
pub use unit::{
    MetricDimension, convert_unit, convert_unit_metered, metres_from_unit, metres_from_unit_metered,
};

use purrdf_hash::{Domain, frame::frame_le};

const POINT_CERTIFICATE: Domain = Domain::new(b"purrdf/point-distance-certificate/v1");
use purrdf_xsd::{XsdDatatype, XsdValue, numeric};

use crate::context::WorkProgress;
use crate::{GeoBindingId, GeoError, Int, MetricContext, PointLaw, Rat, SemanticLawId};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation, LimbScratch};

macro_rules! quantity {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Rat);

        impl $name {
            /// Construct from an exact rational in this quantity's SI unit.
            #[must_use]
            pub const fn new(value: Rat) -> Self {
                Self(value)
            }

            /// The exact quantity in its SI unit.
            #[must_use]
            pub const fn exact(&self) -> &Rat {
                &self.0
            }

            pub(crate) fn into_exact(self) -> Rat {
                self.0
            }

            /// Original quantity storage retained alongside a following phase.
            /// This counts live integer limb capacities, not allocator overhead.
            #[must_use]
            pub fn retained_workspace_bytes(&self) -> u64 {
                (size_of::<Self>() as u64).saturating_add(self.0.allocated_bytes() as u64)
            }
        }
    };
}

quantity!(Metres, "An exact signed distance in metres.");
quantity!(SquareMetres, "An exact signed area in square metres.");

/// A finite binary64 metre threshold after SPARQL numeric promotion.
///
/// Its IEEE bits are retained. Finite negative thresholds are valid and compare
/// false against a nonnegative distance; NaN and infinities are refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct XsdDoubleMetres(u64);

impl XsdDoubleMetres {
    /// Validate an already-promoted binary64 threshold without arithmetic.
    ///
    /// # Errors
    ///
    /// Returns a typed refusal for NaN and either infinity.
    pub fn new(value: f64) -> Result<Self, GeoError> {
        if !value.is_finite() {
            return Err(GeoError::NonFiniteThreshold {
                bits: value.to_bits(),
            });
        }
        Ok(Self(value.to_bits()))
    }

    /// Apply the same numeric promotion as an XSD/SPARQL double comparison.
    ///
    /// # Errors
    ///
    /// Refuses an incompatible floating environment, a nonnumeric value or a
    /// nonfinite promoted threshold.
    pub fn from_xsd(value: &XsdValue) -> Result<Self, GeoError> {
        crate::context::validate_environment()?;
        // XSD integer casts and binary32 widening run under the same checked
        // binary64 scope as the metric kernels. Its thread control state is
        // restored before returning, including when promotion refuses.
        let _scope = purrdf_xsd::ieee::Binary64Scope::enter();
        let promoted = numeric::promote_to_double(value)
            .ok_or_else(|| GeoError::domain("a distance threshold must be numeric"))?;
        Self::new(promoted)
    }

    /// Promote an exact XSD decimal through the shared numeric home.
    ///
    /// # Errors
    ///
    /// Refuses an incompatible floating environment or a nonfinite promoted result.
    pub fn from_decimal(value: numeric::Decimal) -> Result<Self, GeoError> {
        Self::from_xsd(&XsdValue::Decimal(value))
    }

    /// Promote an XSD integer through the shared numeric home.
    ///
    /// # Errors
    ///
    /// Refuses an incompatible floating environment or a nonfinite promoted result.
    pub fn from_integer(value: i128) -> Result<Self, GeoError> {
        Self::from_xsd(&XsdValue::Integer {
            value,
            datatype: XsdDatatype::Integer,
        })
    }

    /// The finite reported-comparison threshold.
    #[must_use]
    pub const fn reported(self) -> f64 {
        f64::from_bits(self.0)
    }

    /// The original finite IEEE bits, including signed zero.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Whether this threshold is strictly negative. Negative zero is zero.
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 >> 63 != 0 && self.0 << 1 != 0
    }
}

/// A completed correctly rounded point distance and its canonical certificate.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MetricEstimate {
    value: Metres,
    quantized_micrometres: Int,
    binding: GeoBindingId,
    reported_bits: u64,
    conversion_bound: Metres,
}

impl MetricEstimate {
    /// The exact half-even quantized result, in metres.
    #[must_use]
    pub const fn value(&self) -> &Metres {
        &self.value
    }

    /// The integer number of micrometres in the completed result.
    #[must_use]
    pub const fn quantized_micrometres(&self) -> &Int {
        &self.quantized_micrometres
    }

    /// Original integer owners copied by a completed result clone.
    pub(crate) fn owned_integer_operands(&self) -> impl Iterator<Item = &Int> {
        [self.value.exact(), self.conversion_bound.exact()]
            .into_iter()
            .flat_map(Rat::integer_operands)
            .chain([&self.quantized_micrometres])
    }

    /// The correctly rounded host conversion of the quantized result.
    #[must_use]
    pub const fn reported_double(&self) -> f64 {
        f64::from_bits(self.reported_bits)
    }

    /// Fixed maximum error between the exact quantized answer and truth.
    #[must_use]
    pub fn rounding_bound(&self) -> Metres {
        Metres::new(Rat::new(Int::one(), Int::from_i64(2_000_000)).expect("positive"))
    }

    /// Additional half-ULP bound for the reported binary64 conversion.
    #[must_use]
    pub const fn host_conversion_bound(&self) -> &Metres {
        &self.conversion_bound
    }

    /// The unchanged mathematical shortest-distance output law.
    #[must_use]
    pub fn law_id(&self) -> SemanticLawId {
        PointLaw::ShortestDistanceMicrometreV1.id()
    }

    /// Exact reference identity used to compute this answer.
    #[must_use]
    pub const fn binding_id(&self) -> GeoBindingId {
        self.binding
    }

    /// Compare the reported double using precisely the public threshold law.
    #[must_use]
    pub fn within_reported(&self, threshold: XsdDoubleMetres) -> bool {
        !threshold.is_negative() && self.reported_double() <= threshold.reported()
    }

    /// Canonical version-1 certificate bytes, independent of proof tightness.
    ///
    /// Each field is unsigned little-endian length framed, in this fixed order:
    /// format tag, law digest, binding digest, metre quantum, integer answer,
    /// fixed rounding bound, IEEE result bits, half-ULP numerator/denominator.
    #[must_use]
    pub fn certificate_bytes(&self) -> Vec<u8> {
        let law = self.law_id().digest();
        let binding = self.binding.digest();
        let integer = self.quantized_micrometres.to_string();
        let bits = self.reported_bits.to_be_bytes();
        let numerator = self.conversion_bound.exact().numerator().to_string();
        let denominator = self.conversion_bound.exact().denominator().to_string();
        framed_certificate([
            POINT_CERTIFICATE.as_bytes(),
            law.as_bytes(),
            binding.as_bytes(),
            b"0.000001",
            integer.as_bytes(),
            b"0.0000005",
            bits.as_slice(),
            numerator.as_bytes(),
            denominator.as_bytes(),
        ])
    }

    pub(crate) fn from_quantized(
        quantized_micrometres: Int,
        binding: GeoBindingId,
    ) -> Result<Self, GeoError> {
        Self::from_quantized_using(quantized_micrometres, binding, None, None)
    }

    fn from_quantized_using(
        quantized_micrometres: Int,
        binding: GeoBindingId,
        scratch: Option<&LimbScratch>,
        zero_bound: Option<&Rat>,
    ) -> Result<Self, GeoError> {
        if quantized_micrometres.is_negative() {
            return Err(GeoError::domain("a shortest distance cannot be negative"));
        }
        let value = match scratch {
            Some(scratch) => Rat::from_decimal_in(&quantized_micrometres, 6, scratch)
                .map_err(GeoError::NumericalScratch)?,
            None => Rat::from_decimal(quantized_micrometres.clone(), 6),
        };
        let reported = match scratch {
            Some(scratch) => purrdf_xsd::ieee::ratio::to_binary64_in(
                value.numerator(),
                value.denominator(),
                purrdf_xsd::ieee::ratio::Rounding::NearestEven,
                scratch,
            )
            .map_err(GeoError::NumericalScratch)?
            .expect("a canonical denominator is positive"),
            None => value.to_f64(),
        };
        if !reported.is_finite() {
            return Err(GeoError::ArithmeticOverflow("reported distance"));
        }
        let conversion_bound = Metres::new(if reported.to_bits() & 0x7ff0_0000_0000_0000 == 0 {
            match zero_bound {
                Some(bound) => bound.clone(),
                None => binary64_half_ulp_using(reported.to_bits(), scratch)?,
            }
        } else {
            binary64_half_ulp_using(reported.to_bits(), scratch)?
        });
        Ok(Self {
            value: Metres::new(value),
            quantized_micrometres,
            binding,
            reported_bits: reported.to_bits(),
            conversion_bound,
        })
    }
}

/// A certified invocation enclosure, kept separate from completed response bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricProofReceipt {
    /// Inclusive lower bound on the unrounded mathematical distance, in metres.
    pub lower: Metres,
    /// Inclusive upper bound on the unrounded mathematical distance, in metres.
    pub upper: Metres,
    /// Precision used by the decisive certified computation.
    pub precision_bits: u32,
    /// Work items actually consumed by this invocation.
    pub work_items: u64,
}

#[derive(Debug)]
pub(crate) struct ReceiptOwnership {
    bytes: u64,
    cost: ExactArithmeticCost,
}

impl ReceiptOwnership {
    pub(crate) fn for_operands<'a>(
        values: impl IntoIterator<Item = &'a Rat>,
    ) -> Result<Self, GeoError> {
        let mut bytes = 0_u64;
        let mut bits = 0;
        let mut count = 0_u64;
        for value in values {
            bytes = bytes
                .checked_add(value.shared_owned_bytes() as u64)
                .ok_or(GeoError::ArithmeticOverflow("immutable proof storage"))?;
            bits = bits
                .max(value.numerator().bit_len())
                .max(value.denominator().bit_len());
            count = count
                .checked_add(2)
                .ok_or(GeoError::ArithmeticOverflow("immutable proof operands"))?;
        }
        let cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, bits, count).ok_or(
            GeoError::ArithmeticOverflow("immutable proof copy admission"),
        )?;
        Ok(Self { bytes, cost })
    }

    pub(crate) fn prepare<T>(
        self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        copy: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        context.admit_workspace(self.bytes)?;
        let result = progress.exact(context, self.cost, copy);
        context.release_workspace(self.bytes)?;
        result
    }
}

impl MetricProofReceipt {
    pub(crate) fn into_shared_owned(
        mut self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        let admission = ReceiptOwnership::for_operands([self.lower.exact(), self.upper.exact()])?;
        admission.prepare(context, progress, || {
            self.lower = Metres::new(self.lower.into_exact().into_shared());
            self.upper = Metres::new(self.upper.into_exact().into_shared());
            Ok(self)
        })
    }
}

/// The shared canonical field-framing home for numerical result certificates.
pub(crate) fn framed_certificate<'a>(fields: impl IntoIterator<Item = &'a [u8]>) -> Vec<u8> {
    let mut output = Vec::with_capacity(256);
    for field in fields {
        frame_le(&mut output, field);
    }
    output
}

/// Generate the identical source-bound certificate method at its one home.
/// The caller supplies its frozen completed-set bound; invocation proof and
/// resource counters never enter the canonical bytes.
macro_rules! source_bound_certificate {
    ($bound:literal) => {
        /// Conservative owned carrier/result storage allowance still charged to
        /// the producing context. It excludes temporary numerical workspaces.
        /// Keep it admitted while writing this carrier; after dropping the
        /// result, release its typed receipt or drop the invocation context.
        /// This is an admitted storage bound, not allocator or RSS measurement.
        #[must_use]
        pub const fn retained_workspace_bytes(&self) -> u64 {
            self.output_receipt.workspace_bytes()
        }
        /// Copy the ownership receipt before dropping this result. Release it
        /// through the original context after all result owners have been dropped.
        #[must_use]
        pub fn output_receipt(&self) -> $crate::MaterializedOutputReceipt {
            self.output_receipt.clone()
        }
        /// Canonical complete-set certificate without invocation work counters.
        #[must_use]
        pub fn certificate_bytes(&self) -> Vec<u8> {
            $crate::metric::framed_certificate([
                self.law.digest().as_bytes().as_slice(),
                self.source.as_bytes().as_slice(),
                $bound.as_slice(),
            ])
        }
    };
}
pub(crate) use source_bound_certificate;

/// Commit an answer only if the complete enclosure rounds to one result.
#[cfg(test)]
pub(crate) fn complete_point_enclosure(
    lower: &Rat,
    upper: &Rat,
    binding: GeoBindingId,
    precision_bits: u32,
) -> Result<MetricEstimate, GeoError> {
    complete_point_enclosure_using(lower, upper, binding, precision_bits, None, None)
}

fn complete_point_enclosure_using(
    lower: &Rat,
    upper: &Rat,
    binding: GeoBindingId,
    precision_bits: u32,
    scratch: Option<&LimbScratch>,
    zero_bound: Option<&Rat>,
) -> Result<MetricEstimate, GeoError> {
    let compare = |a: &Rat, b: &Rat| match scratch {
        Some(scratch) => a.compare_in(b, scratch).map_err(GeoError::NumericalScratch),
        None => Ok(a.cmp(b)),
    };
    if compare(lower, upper)?.is_gt() || compare(lower, &Rat::zero())?.is_lt() {
        return Err(GeoError::domain("invalid nonnegative distance enclosure"));
    }
    let round = |value: &Rat| match scratch {
        Some(scratch) => value
            .round_to_scale_in(6, scratch)
            .map_err(GeoError::NumericalScratch),
        None => Ok(value.round_to_scale(6)),
    };
    let rounded_lower = round(lower)?;
    let rounded_upper = round(upper)?;
    if rounded_lower != rounded_upper {
        return Err(GeoError::PrecisionExhausted {
            bits: precision_bits,
        });
    }
    match scratch {
        Some(_) => {
            MetricEstimate::from_quantized_using(rounded_lower, binding, scratch, zero_bound)
        }
        None => MetricEstimate::from_quantized(rounded_lower, binding),
    }
}

/// The same complete response, admitting every final original-rational
/// comparison/round before copying into the worker's reusable destinations.
pub(crate) fn complete_point_enclosure_observed(
    lower: &Rat,
    upper: &Rat,
    binding: GeoBindingId,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<MetricEstimate, GeoError> {
    let shape = |value: &Rat| (value.numerator().bit_len(), value.denominator().bit_len());
    let round =
        |value: &Rat| ExactArithmeticCost::rational_round(shape(value).0, shape(value).1, 6);
    // Scaling n/d by 10^6 and rounding adds at most 22 bits to the
    // nonnegative binary exponent bound, including its final carry.
    let rounded_bits = shape(upper)
        .0
        .saturating_sub(shape(upper).1)
        .saturating_add(22);
    let conversion_bits = if shape(lower).0.saturating_add(19) <= shape(lower).1 {
        rounded_bits.max(1076)
    } else {
        rounded_bits.saturating_add(80)
    };
    let cost = ExactArithmeticCost::for_rational_operands(
        ExactOperation::RationalCompare,
        [shape(lower), shape(upper)],
        2,
    )
    .and_then(|value| value.followed_by(round(lower)?))
    .and_then(|value| value.followed_by(round(upper)?))
    .and_then(|value| value.followed_by(ExactArithmeticCost::decimal_rational(rounded_bits, 6)?))
    .and_then(|value| {
        value.followed_by(ExactArithmeticCost::for_operation(
            ExactOperation::Divide,
            conversion_bits,
            4,
        )?)
    })
    .ok_or(GeoError::WorkExhausted {
        limit: context.policy().limits().max_work_items,
    })?;
    context.prepare_integer_scratch_for_cost_observed(cost, progress)?;
    if (lower.is_zero() || shape(lower).0.saturating_add(19) <= shape(lower).1)
        && context.zero_conversion_bound().is_none()
    {
        let bytes = 17 * size_of::<u64>() + size_of::<Vec<u64>>() + 2 * size_of::<usize>();
        let constant_cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, 1076, 4)
            .ok_or(GeoError::WorkExhausted {
                limit: context.policy().limits().max_work_items,
            })?;
        context.admit_workspace(bytes as u64)?;
        let result = progress.exact_context(context, constant_cost, |_| {
            Ok(binary64_half_ulp(0).into_shared())
        });
        context.release_workspace(bytes as u64)?;
        context.retain_zero_conversion_bound(result?)?;
    }
    let precision = context.policy().limits().max_precision_bits;
    progress.exact_context(context, cost, |context| {
        complete_point_enclosure_using(
            lower,
            upper,
            binding,
            precision,
            context.integer_scratch(),
            context.zero_conversion_bound(),
        )
    })
}

/// Half the spacing of the binary64 binade containing a finite value.
/// Subnormal values and zero use half the minimum subnormal spacing.
pub(crate) fn binary64_half_ulp(bits: u64) -> Rat {
    binary64_half_ulp_using(bits, None).expect("unbounded integer storage")
}

pub(crate) fn binary64_half_ulp_cost(bits: u64) -> Option<ExactArithmeticCost> {
    let power = binary64_half_ulp_power(bits);
    if power < 0 {
        ExactArithmeticCost::dyadic_rational(1, power.unsigned_abs())
    } else {
        ExactArithmeticCost::for_operation(
            ExactOperation::Linear,
            u64::from(power.unsigned_abs()).checked_add(1)?,
            1,
        )
    }
}

fn binary64_half_ulp_power(bits: u64) -> i32 {
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0 {
        -1075
    } else {
        exponent - 1023 - 53
    }
}

fn binary64_half_ulp_using(bits: u64, scratch: Option<&LimbScratch>) -> Result<Rat, GeoError> {
    let power = binary64_half_ulp_power(bits);
    Ok(if power < 0 {
        match scratch {
            Some(scratch) => {
                Rat::from_dyadic_integer_in(&Int::one(), power.unsigned_abs(), scratch)
                    .map_err(GeoError::NumericalScratch)?
            }
            None => Rat::from_dyadic_integer(&Int::one(), power.unsigned_abs()),
        }
    } else {
        Rat::from_int(match scratch {
            Some(scratch) => Int::one()
                .shl_in(power.unsigned_abs(), scratch)
                .map_err(GeoError::NumericalScratch)?,
            None => Int::one().shl(power.unsigned_abs()),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{ReceiptOwnership, XsdDoubleMetres, complete_point_enclosure};
    use crate::{GeoError, GeographicReference, Int, MetricContext, Rat, context::WorkProgress};
    use purrdf_xsd::numeric;

    fn decimal(value: &str) -> Rat {
        Rat::parse_decimal(value).expect("test decimal")
    }

    #[test]
    fn immutable_receipt_admission_observes_refusal_before_copying_storage() {
        struct Refuse;
        impl crate::MetricWorkObserver for Refuse {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        let value = Rat::new(Int::one().shl(448).add(&Int::one()), Int::one()).unwrap();
        let admission = ReceiptOwnership::for_operands([&value]).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let before = context.retained_workspace_bytes();
        let mut observer = Refuse;
        let mut progress = WorkProgress::new(Some(&mut observer));
        let copied = std::cell::Cell::new(false);
        let result = admission.prepare(&mut context, &mut progress, || {
            copied.set(true);
            Ok(value.into_shared())
        });
        assert_eq!(result, Err(GeoError::Cancelled));
        assert!(!copied.get());
        assert_eq!(context.retained_workspace_bytes(), before);
        assert_eq!(
            context.remaining_workspace(),
            context.policy().limits().max_workspace_bytes
        );
    }

    #[test]
    fn tolerance_does_not_license_unresolved_rounding() {
        let binding = GeographicReference::wgs84().id();
        assert!(matches!(
            complete_point_enclosure(
                &decimal("1.00000049999999"),
                &decimal("1.00000050000001"),
                binding,
                512,
            ),
            Err(GeoError::PrecisionExhausted { bits: 512 })
        ));
        let tie =
            complete_point_enclosure(&decimal("1.0000005"), &decimal("1.0000005"), binding, 128)
                .expect("a proven half-even tie");
        assert_eq!(tie.value().exact(), &Rat::one());
    }

    #[test]
    fn equivalent_enclosures_have_identical_completed_certificates() {
        let binding = GeographicReference::wgs84().id();
        let broad =
            complete_point_enclosure(&decimal("12.3456781"), &decimal("12.3456784"), binding, 80)
                .expect("decisive rounding");
        let narrow = complete_point_enclosure(
            &decimal("12.34567821"),
            &decimal("12.34567822"),
            binding,
            256,
        )
        .expect("same decisive rounding");
        assert_eq!(broad, narrow);
        assert_eq!(broad.certificate_bytes(), narrow.certificate_bytes());
    }

    #[test]
    fn public_predicate_uses_promoted_reported_double() {
        let result = complete_point_enclosure(
            &decimal("0.1"),
            &decimal("0.1"),
            GeographicReference::wgs84().id(),
            64,
        )
        .expect("exact rounding");
        let threshold = XsdDoubleMetres::from_decimal(numeric::parse_decimal("0.1").unwrap())
            .expect("finite promoted decimal");
        assert!(result.within_reported(threshold));
        assert!(!result.within_reported(XsdDoubleMetres::from_integer(-1).unwrap()));
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(XsdDoubleMetres::new(value).is_err());
        }
    }

    #[test]
    fn threshold_integer_promotion_preserves_half_even_ties() {
        for (source, expected) in [
            (9_007_199_254_740_993, 9_007_199_254_740_992_f64),
            (9_007_199_254_740_995, 9_007_199_254_740_996_f64),
            (-9_007_199_254_740_993, -9_007_199_254_740_992_f64),
        ] {
            assert_eq!(
                XsdDoubleMetres::from_integer(source).unwrap().bits(),
                expected.to_bits()
            );
        }
    }

    #[cfg(any(
        target_arch = "x86_64",
        all(target_arch = "x86", target_feature = "sse2")
    ))]
    #[test]
    fn threshold_promotion_refuses_changed_controls_before_conversion() {
        use purrdf_xsd::ieee::control;
        let saved = control::mxcsr();
        for change in [1 << 15, 1 << 6, 1 << 13, 2 << 13, 3 << 13] {
            {
                let _guard = control::Mxcsr::load(saved | change);
                let changed = control::mxcsr();
                for value in [
                    purrdf_xsd::XsdValue::Integer {
                        value: 9_007_199_254_740_993,
                        datatype: purrdf_xsd::XsdDatatype::Integer,
                    },
                    purrdf_xsd::XsdValue::Float(f32::from_bits(1)),
                ] {
                    assert!(matches!(
                        XsdDoubleMetres::from_xsd(&value),
                        Err(GeoError::FloatEnvironment(_))
                    ));
                    assert_eq!(control::mxcsr(), changed);
                }
                // Already-promoted IEEE bits require no floating arithmetic.
                assert_eq!(XsdDoubleMetres::new(f64::from_bits(1)).unwrap().bits(), 1);
            }
            assert_eq!(control::mxcsr(), saved);
        }
    }
}
