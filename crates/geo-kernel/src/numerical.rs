// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one bridge between exact carrier rationals and XSD numerical enclosures.

use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError, MathLimits};

use crate::context::WorkProgress;
use crate::{ExecutionPolicy, GeoError, Int, MetricContext, Rat};

/// One expression's selected native destination capability. Resident adapters
/// keep their existing infallible allocation contract; an admitted expression
/// always uses its original arena and never falls back after refusal.
type DiagnosticNumberFactory<'a> =
    dyn Fn(&Rat) -> Result<crate::context::Retained<Rat>, GeoError> + 'a;

/// Typed admission adapters to the original XSD integer and rational operations.
/// The integer and rational entries retain their different value/normalization
/// laws; each delegates computation to that type's existing admitted or resident
/// home. Copying a coordinate retains all its ordinates, unlike copying one Rat.
#[derive(Clone, Copy)]
pub(crate) struct ExactValues<'a>(
    Option<&'a purrdf_xsd::integer::LimbScratch>,
    Option<&'a DiagnosticNumberFactory<'a>>,
);

impl ExactValues<'_> {
    pub(crate) const fn admitted(scratch: &purrdf_xsd::integer::LimbScratch) -> ExactValues<'_> {
        ExactValues(Some(scratch), None)
    }

    pub(crate) const fn resident() -> Self {
        Self(None, None)
    }

    pub(crate) fn for_math(math: &CoordinateMath) -> ExactValues<'_> {
        ExactValues(math.limb_scratch(), None)
    }

    pub(crate) fn with_diagnostics<'a>(
        scratch: Option<&'a purrdf_xsd::integer::LimbScratch>,
        diagnostic: &'a DiagnosticNumberFactory<'a>,
    ) -> ExactValues<'a> {
        ExactValues(scratch, Some(diagnostic))
    }

    pub(crate) fn diagnostic_number(
        &self,
        value: &Rat,
    ) -> Result<crate::context::Retained<Rat>, GeoError> {
        if let Some(make) = self.1 {
            return make(value);
        }
        GeoError::diagnostic_number(value, &mut WorkProgress::integer(None))
    }

    pub(crate) fn original(
        &self,
        value: &Rat,
    ) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => value.copy_in(scratch),
            None => Ok(value.clone()),
        }
    }

    pub(crate) fn coordinate(
        &self,
        value: &crate::Coord,
    ) -> Result<crate::Coord, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => value.copy_in(scratch),
            None => Ok(value.clone()),
        }
    }

    /// Copy only the topology law's two projected ordinates. The caller's
    /// exact-operation certificate therefore need not price unused Z/M limbs.
    pub(crate) fn planar_coordinate(
        &self,
        value: &crate::Coord,
    ) -> Result<crate::Coord, purrdf_xsd::integer::LimbScratchError> {
        Ok(crate::Coord::xy(
            self.original(value.x())?,
            self.original(value.y())?,
        ))
    }

    pub(crate) fn decimal_digits(
        &self,
        value: &Int,
    ) -> Result<purrdf_xsd::bigint::PreparedDecimalDigits, purrdf_xsd::integer::LimbScratchError>
    {
        match self.0 {
            Some(scratch) => value.prepare_decimal_digits_in(scratch),
            None => value.prepare_decimal_digits(),
        }
    }

    pub(crate) fn point(
        &self,
        value: &crate::LonLat,
    ) -> Result<crate::LonLat, purrdf_xsd::integer::LimbScratchError> {
        value.copy_with(self)
    }

    pub(crate) fn location(
        &self,
        longitude: &Rat,
        latitude: &Rat,
    ) -> Result<crate::LonLat, GeoError> {
        self.location_result(longitude, latitude)?
    }

    pub(crate) fn location_result(
        &self,
        longitude: &Rat,
        latitude: &Rat,
    ) -> Result<Result<crate::LonLat, GeoError>, purrdf_xsd::integer::LimbScratchError> {
        if let Err(error) = crate::LonLat::validate_with(longitude, latitude, self)? {
            return Ok(Err(error));
        }
        Ok(Ok(crate::LonLat::from_validated(
            self.original(longitude)?,
            self.original(latitude)?,
        )))
    }

    pub(crate) fn planar_location(
        &self,
        point: &crate::LonLat,
    ) -> Result<crate::Coord, purrdf_xsd::integer::LimbScratchError> {
        Ok(crate::Coord::xy(
            self.original(point.longitude())?,
            self.original(point.latitude())?,
        ))
    }

    pub(crate) fn compare_coordinate(
        &self,
        a: &crate::Coord,
        b: &crate::Coord,
    ) -> Result<core::cmp::Ordering, purrdf_xsd::integer::LimbScratchError> {
        let x = self.compare(a.x(), b.x())?;
        if x.is_eq() {
            self.compare(a.y(), b.y())
        } else {
            Ok(x)
        }
    }

    pub(crate) fn extreme<'a>(
        &self,
        a: &'a Rat,
        b: &'a Rat,
        maximum: bool,
    ) -> Result<&'a Rat, purrdf_xsd::integer::LimbScratchError> {
        Ok(if self.compare(a, b)?.is_gt() == maximum {
            a
        } else {
            b
        })
    }

    pub(crate) fn normalize(
        &self,
        angle: &Rat,
        origin: i64,
    ) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => angle.modulo_integer_in(origin, 360, scratch),
            None => Ok(angle.modulo_integer(origin, 360)),
        }
        .map(|value| value.expect("positive period"))
    }

    pub(crate) fn integer_absolute(
        &self,
        value: &Int,
    ) -> Result<Int, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => value.abs_in(scratch),
            None => Ok(value.abs()),
        }
    }

    pub(crate) fn integer_shift(
        &self,
        value: &Int,
        bits: u32,
    ) -> Result<Int, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => value.shl_in(bits, scratch),
            None => Ok(value.shl(bits)),
        }
    }

    pub(crate) fn integer_multiply(
        &self,
        a: &Int,
        b: &Int,
    ) -> Result<Int, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.mul_in(b, scratch),
            None => Ok(a.mul(b)),
        }
    }

    pub(crate) fn integer_root(
        &self,
        value: &Int,
    ) -> Result<Option<Int>, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => value.sqrt_floor_in(scratch),
            None => Ok(value.sqrt_floor()),
        }
    }

    pub(crate) fn dyadic(
        &self,
        value: &Int,
        bits: u32,
    ) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => Rat::from_dyadic_integer_in(value, bits, scratch),
            None => Ok(Rat::from_dyadic_integer(value, bits)),
        }
    }

    pub(crate) fn binary64(
        &self,
        value: f64,
    ) -> Result<Option<Rat>, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => Rat::from_binary64_in(value, scratch),
            None => Ok(Rat::from_binary64(value)),
        }
    }

    pub(crate) fn decimal(
        &self,
        mantissa: &Int,
        places: u32,
    ) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => Rat::from_decimal_in(mantissa, places, scratch),
            None => Ok(Rat::from_decimal(mantissa.clone(), places)),
        }
    }

    pub(crate) fn round_to_scale(
        &self,
        value: &Rat,
        places: u32,
    ) -> Result<Int, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => value.round_to_scale_in(places, scratch),
            None => Ok(value.round_to_scale(places)),
        }
    }

    /// Call the original lexical body only after its exact construction cost
    /// and destination layout have been admitted by the enclosing expression.
    pub(crate) fn decimal_lexical(&self, text: &str) -> Result<Option<Rat>, GeoError> {
        let Some(scratch) = self.0 else {
            return Ok(Rat::parse_decimal(text));
        };
        match Rat::parse_decimal_in(
            text,
            &mut |_, _| Ok::<(), core::convert::Infallible>(()),
            scratch,
        ) {
            Ok(value) => Ok(value),
            Err(purrdf_xsd::integer::DecimalConstructionError::Storage(error)) => {
                Err(GeoError::NumericalScratch(error))
            }
            Err(purrdf_xsd::integer::DecimalConstructionError::Admission(never)) => match never {},
        }
    }

    pub(crate) fn binary64_half_ulp(&self, bits: u64) -> Result<Rat, GeoError> {
        crate::metric::binary64_half_ulp_using(bits, self.0)
    }

    pub(crate) fn integer_divide(
        &self,
        a: &Int,
        b: &Int,
    ) -> Result<Option<(Int, Int)>, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.div_rem_in(b, scratch),
            None => Ok(a.div_rem(b)),
        }
    }

    pub(crate) fn integer_gcd(
        &self,
        a: &Int,
        b: &Int,
    ) -> Result<Int, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.gcd_in(b, scratch),
            None => Ok(a.gcd(b)),
        }
    }

    pub(crate) fn integer_subtract(
        &self,
        a: &Int,
        b: &Int,
    ) -> Result<Int, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.sub_in(b, scratch),
            None => Ok(a.sub(b)),
        }
    }

    pub(crate) fn integer_add(
        &self,
        a: &Int,
        b: &Int,
    ) -> Result<Int, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.add_in(b, scratch),
            None => Ok(a.add(b)),
        }
    }

    pub(crate) fn add(
        &self,
        a: &Rat,
        b: &Rat,
    ) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.add_in(b, scratch),
            None => Ok(a.add(b)),
        }
    }

    pub(crate) fn subtract(
        &self,
        a: &Rat,
        b: &Rat,
    ) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.sub_in(b, scratch),
            None => Ok(a.sub(b)),
        }
    }

    pub(crate) fn multiply(
        &self,
        a: &Rat,
        b: &Rat,
    ) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.mul_in(b, scratch),
            None => Ok(a.mul(b)),
        }
    }

    pub(crate) fn divide(
        &self,
        a: &Rat,
        b: &Rat,
    ) -> Result<Option<Rat>, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.div_in(b, scratch),
            None => Ok(a.div(b)),
        }
    }

    pub(crate) fn reciprocal(
        &self,
        value: &Rat,
    ) -> Result<Option<Rat>, purrdf_xsd::integer::LimbScratchError> {
        // Reciprocal swaps canonical magnitudes. In the admitted branch the
        // original copy enters the arena first, so its clones share that owner.
        Ok(self.original(value)?.recip())
    }

    pub(crate) fn absolute(
        &self,
        value: &Rat,
    ) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => value.abs_in(scratch),
            None => Ok(value.abs()),
        }
    }

    pub(crate) fn negate(&self, value: &Rat) -> Result<Rat, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => value.neg_in(scratch),
            None => Ok(value.neg()),
        }
    }

    pub(crate) fn compare(
        &self,
        a: &Rat,
        b: &Rat,
    ) -> Result<core::cmp::Ordering, purrdf_xsd::integer::LimbScratchError> {
        match self.0 {
            Some(scratch) => a.compare_in(b, scratch),
            None => Ok(a.cmp(b)),
        }
    }
}

/// Spell already admitted native decimal groups into one exactly sized string.
/// The original integer conversion and formatter remain the XSD home's bodies.
pub(crate) fn decimal_text_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
    digits: &purrdf_xsd::bigint::PreparedDecimalDigits,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<String, GeoError> {
    let mut text = String::new();
    memory
        .reserve_string(&mut text, digits.len())
        .map_err(GeoError::Storage)?;
    core::fmt::write(&mut text, format_args!("{digits}"))
        .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::FormattingFailed))?;
    if text.len() != digits.len() {
        return Err(GeoError::ArithmeticOverflow(
            "native decimal spelling length",
        ));
    }
    Ok(text)
}

/// Stream the XSD home's already admitted decimal groups into one exact frame.
pub(crate) fn frame_decimal_digits(
    hash: &mut purrdf_hash::blake3::Hasher,
    digits: &purrdf_xsd::bigint::PreparedDecimalDigits,
) -> Result<(), GeoError> {
    purrdf_hash::frame::frame_le_display_into(hash, digits.len(), digits)
        .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::FormattingFailed))
}

/// Preserve the original reference preimage while using the caller's native
/// arena for the sole integer renderer. Each group dies before the next field.
pub(crate) fn reference_identity_using(
    reference: &crate::GeographicReference,
    values: &ExactValues<'_>,
) -> Result<crate::GeoBindingId, GeoError> {
    reference.compute_identity_with(|value, hash| {
        let digits = values
            .decimal_digits(value)
            .map_err(GeoError::NumericalScratch)?;
        frame_decimal_digits(hash, &digits)
    })
}

/// A borrowed exact-predicate admission; all arithmetic remains in its original
/// rational/integer home and shares the XSD limb-work and scratch bounds.
pub(crate) struct ExactAdmission<'context, 'observer> {
    context: &'context mut MetricContext,
    progress: &'context mut WorkProgress<'observer>,
    retained: Option<&'context mut u64>,
}
impl<'context, 'observer> ExactAdmission<'context, 'observer> {
    pub(crate) fn new(
        context: &'context mut MetricContext,
        progress: &'context mut WorkProgress<'observer>,
    ) -> Self {
        Self {
            context,
            progress,
            retained: None,
        }
    }

    /// Keep every original rational operation's complete owner allowance,
    /// including intermediates and error paths, until the enclosing proof
    /// drops all returned values and releases its accumulated counter.
    pub(crate) fn retaining(
        context: &'context mut MetricContext,
        progress: &'context mut WorkProgress<'observer>,
        retained: &'context mut u64,
    ) -> Self {
        Self {
            context,
            progress,
            retained: Some(retained),
        }
    }

    /// Publish a native field record before its actual destination array grows.
    /// Detached fields retain their original lease, while the array's original
    /// control and buffer owner cover each replacement until it actually dies.
    pub(crate) fn push_detached<T: crate::context::NativeDetached>(
        &mut self,
        output: &mut crate::context::RetainedBuilder<T>,
        value: T,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        let value =
            crate::context::persistent_numeric(value, self.context, self.progress, retained)?;
        output.push_admitted(value, self.context, self.progress, retained)
    }

    /// Borrow the existing metadata scope without changing the enclosing
    /// arithmetic-retention law or introducing another allocation account.
    pub(crate) fn with_metadata<T>(
        &mut self,
        evaluate: impl FnOnce(&mut ExactAdmission<'_, 'observer>, &mut u64) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let original = self.retained.as_deref_mut();
        self.context
            .with_metadata(self.progress, |context, progress, retained| {
                let mut admission = ExactAdmission {
                    context,
                    progress,
                    retained: original,
                };
                evaluate(&mut admission, retained)
            })
    }

    pub(crate) fn rational<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let cost = rational_cost(operation, operands, count)
            .ok_or(GeoError::ArithmeticOverflow("exact arithmetic admission"))?;
        if let Some(retained) = self.retained.as_deref_mut() {
            retained_exact(self.context, self.progress, cost, retained, |_| evaluate())
        } else {
            self.progress.exact(self.context, cost, evaluate)
        }
    }

    /// Execute through the original physical destinations after complete admission.
    pub(crate) fn rational_with<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce(
            &purrdf_xsd::integer::LimbScratch,
        ) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
    ) -> Result<T, GeoError> {
        self.with_destinations(operation, operands, count, |scratch| {
            evaluate(scratch).map_err(GeoError::NumericalScratch)
        })
    }

    fn with_destinations<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce(&purrdf_xsd::integer::LimbScratch) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let cost = rational_cost(operation, operands, count).ok_or(
            GeoError::ArithmeticOverflow("native exact arithmetic admission"),
        )?;
        self.with_cost(cost, evaluate)
    }

    fn with_cost<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce(&purrdf_xsd::integer::LimbScratch) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.with_cost_progress(cost, |scratch, _| evaluate(scratch))
    }

    fn with_cost_progress<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce(
            &purrdf_xsd::integer::LimbScratch,
            &mut WorkProgress<'_>,
        ) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.progress.preflight_exact(
            self.context,
            cost,
            if self.retained.is_some() {
                cost.workspace_bytes
            } else {
                0
            },
        )?;
        self.context
            .prepare_integer_scratch_for_cost_observed(cost, self.progress)?;
        let scratch = self
            .context
            .integer_scratch()
            .map_or_else(purrdf_xsd::integer::LimbScratch::inline, Clone::clone);
        let result = if let Some(retained) = self.retained.as_deref_mut() {
            retained_exact(self.context, self.progress, cost, retained, |progress| {
                evaluate(&scratch, progress)
            })
        } else {
            self.progress.exact_with(self.context, cost, |_, progress| {
                evaluate(&scratch, progress)
            })
        };
        drop(scratch);
        self.progress
            .settle_result(result, self.context.release_unused_integer_scratch())
    }

    fn with_cost_values<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.with_cost_progress(cost, |scratch, progress| {
            let progress = core::cell::RefCell::new(progress);
            let diagnostic =
                |value: &Rat| GeoError::diagnostic_number(value, &mut progress.borrow_mut());
            evaluate(&ExactValues::with_diagnostics(Some(scratch), &diagnostic))
        })
    }

    pub(crate) fn expression<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
    ) -> Result<T, GeoError> {
        let cost = rational_cost(operation, operands, count).ok_or(
            GeoError::ArithmeticOverflow("native exact arithmetic admission"),
        )?;
        self.with_cost_values(cost, |values| {
            evaluate(values).map_err(GeoError::NumericalScratch)
        })
    }

    pub(crate) fn expression_result<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let cost = rational_cost(operation, operands, count).ok_or(
            GeoError::ArithmeticOverflow("native exact arithmetic admission"),
        )?;
        self.with_cost_values(cost, evaluate)
    }

    pub(crate) fn expression_cost<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.with_cost_values(cost, evaluate)
    }

    pub(crate) fn copy(&mut self, value: &Rat) -> Result<Rat, GeoError> {
        self.rational_with(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[value],
            1,
            |scratch| value.copy_in(scratch),
        )
    }

    pub(crate) fn dyadic(&mut self, numerator: &Int, bits: u32) -> Result<Rat, GeoError> {
        let cost =
            purrdf_xsd::integer::ExactArithmeticCost::dyadic_rational(numerator.bit_len(), bits)
                .ok_or(GeoError::ArithmeticOverflow("exact dyadic admission"))?;
        self.expression_cost(cost, |values| Ok(values.dyadic(numerator, bits)?))
    }

    pub(crate) fn bounds(&mut self, interval: &FixedInterval) -> Result<(Rat, Rat), GeoError> {
        Ok((
            self.dyadic(interval.lower(), interval.precision_bits())?,
            self.dyadic(interval.upper(), interval.precision_bits())?,
        ))
    }

    pub(crate) fn add(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        self.rational_with(
            purrdf_xsd::integer::ExactOperation::RationalAdd,
            &[a, b],
            1,
            |scratch| a.add_in(b, scratch),
        )
    }

    pub(crate) fn subtract(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        self.rational_with(
            purrdf_xsd::integer::ExactOperation::RationalAdd,
            &[a, b],
            1,
            |scratch| a.sub_in(b, scratch),
        )
    }

    pub(crate) fn multiply(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        self.rational_with(
            purrdf_xsd::integer::ExactOperation::RationalMultiply,
            &[a, b],
            1,
            |scratch| a.mul_in(b, scratch),
        )
    }

    pub(crate) fn divide(&mut self, a: &Rat, b: &Rat) -> Result<Option<Rat>, GeoError> {
        self.rational_with(
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[a, b],
            1,
            |scratch| a.div_in(b, scratch),
        )
    }

    pub(crate) fn absolute(&mut self, value: &Rat) -> Result<Rat, GeoError> {
        self.rational_with(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[value],
            1,
            |scratch| value.abs_in(scratch),
        )
    }

    pub(crate) fn negate(&mut self, value: &Rat) -> Result<Rat, GeoError> {
        self.rational_with(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[value],
            1,
            |scratch| value.neg_in(scratch),
        )
    }

    /// Keep the original operation's complete result allowance live through a
    /// larger returned owner. The caller drops that owner before releasing the
    /// accumulated allowance, including when a later preparation refuses.
    pub(crate) fn rational_owner<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        retained: &mut u64,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
    ) -> Result<T, GeoError> {
        self.rational_owner_counted(operation, operands, 1, retained, evaluate)
    }

    /// The same retained owner admission for a complete group of original
    /// operations, such as copying every ordinate of one borrowed coordinate.
    pub(crate) fn rational_owner_counted<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        count: u64,
        retained: &mut u64,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
    ) -> Result<T, GeoError> {
        ExactAdmission::retaining(self.context, self.progress, retained)
            .expression(operation, operands, count, evaluate)
    }

    /// Admit the shared rational comparison's actual scan/arithmetic branch.
    pub(crate) fn compare(&mut self, a: &Rat, b: &Rat) -> Result<core::cmp::Ordering, GeoError> {
        a.compare_admitted_by(b, |cost, evaluate| {
            if cost.workspace_bytes == 0 {
                self.progress.exact(self.context, cost, || {
                    evaluate(None).map_err(GeoError::NumericalScratch)
                })
            } else {
                self.with_cost(cost, |scratch| {
                    evaluate(Some(scratch)).map_err(GeoError::NumericalScratch)
                })
            }
        })
        .map_err(|error| match error {
            purrdf_xsd::integer::RationalComparisonError::CostOverflow => {
                GeoError::ArithmeticOverflow("exact comparison admission")
            }
            purrdf_xsd::integer::RationalComparisonError::Admission(error) => error,
        })
    }

    /// Cache a monotone key, preserving original exact comparison for ties.
    pub(crate) fn order_key(&mut self, value: &Rat) -> Result<u64, GeoError> {
        let mut cost = purrdf_xsd::integer::ExactArithmeticCost::rational_binary64(
            value.numerator().bit_len(),
            value.denominator().bit_len(),
        )
        .ok_or(GeoError::ArithmeticOverflow("exact ordering key"))?;
        cost.work_items = cost
            .work_items
            .checked_add(8)
            .ok_or(GeoError::ArithmeticOverflow(
                "ordering key bit transformation",
            ))?;
        self.with_cost(cost, |scratch| {
            let key = purrdf_xsd::ieee::ratio::binary64_order_key_in(
                value.numerator().as_ref(),
                value.denominator().as_ref(),
                scratch,
            )
            .map_err(GeoError::NumericalScratch)?;
            key.ok_or(GeoError::ArithmeticOverflow("canonical ordering key"))
        })
    }

    /// Distinct monotone keys prove strict order. A key tie always falls back
    /// to the original admitted exact comparison, including rounded zeros.
    pub(crate) fn compare_ordered(
        &mut self,
        a: (&Rat, u64),
        b: (&Rat, u64),
    ) -> Result<core::cmp::Ordering, GeoError> {
        self.context.charge_work(1)?;
        self.progress.context_poll(self.context)?;
        let order = a.1.cmp(&b.1);
        if order.is_eq() {
            self.compare(a.0, b.0)
        } else {
            Ok(order)
        }
    }

    /// Keep the complete original operation allowance while its result is live.
    /// Detach the result under that allowance before returning the reusable
    /// destination. The immutable limbs retain their original physical owner.
    pub(crate) fn rational_retained(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        retained: &mut u64,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<Rat, purrdf_xsd::integer::LimbScratchError>,
    ) -> Result<(Rat, u64), GeoError> {
        let cost = rational_cost(operation, operands, 1)
            .ok_or(GeoError::ArithmeticOverflow("retained exact arithmetic"))?;
        let previous = *retained;
        let value = self.rational_owner(operation, operands, retained, evaluate)?;
        let result = (|| {
            let required = crate::context::persistent_numeric_bytes(&value, self.progress)?;
            let allowance = cost.workspace_bytes.max(required);
            self.context
                .retain_workspace(allowance - cost.workspace_bytes, retained)?;
            self.progress.context_poll(self.context)?;
            let value = crate::context::produce_persistent_numeric(value, self.progress)?;
            // The computation's provisional bound protected every intermediate
            // and the detached publication. Only the actual immutable allocation
            // survives; inline rationals need no heap grant. Transfer that owner
            // instead of retaining dead arithmetic scratch with every result.
            self.context
                .release_retained_workspace(allowance - required, retained)?;
            Ok((value.into_value(), required))
        })();
        let result = self
            .progress
            .settle_result(result, self.context.release_unused_integer_scratch());
        match result {
            Ok(value) => Ok(value),
            Err(original) => {
                // The immutable factory destroys every partial value before
                // this refund. Preserve the original physical refusal.
                if let Some(admitted) = retained.checked_sub(previous) {
                    let _ = self.context.release_retained_workspace(admitted, retained);
                }
                Err(original)
            }
        }
    }

    /// The caller drops the replaced result before releasing its allowance.
    pub(crate) fn release_result(
        &mut self,
        bytes: u64,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        self.context.release_retained_workspace(bytes, retained)
    }
}

fn retained_exact<T>(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    cost: purrdf_xsd::integer::ExactArithmeticCost,
    retained: &mut u64,
    evaluate: impl FnOnce(&mut WorkProgress<'_>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    progress.preflight_exact(context, cost, cost.workspace_bytes)?;
    context.retain_workspace(cost.workspace_bytes, retained)?;
    progress.exact_with(context, cost, |_, progress| evaluate(progress))
}

/// The original exact body with optional ownership of its returned rational.
pub(crate) fn exact_retained_rational(
    admission: Option<&mut ExactAdmission<'_, '_>>,
    operation: purrdf_xsd::integer::ExactOperation,
    operands: &[&Rat],
    retained: &mut u64,
    evaluate: impl FnOnce(&ExactValues<'_>) -> Result<Rat, purrdf_xsd::integer::LimbScratchError>,
) -> Result<(Rat, u64), GeoError> {
    if let Some(admission) = admission {
        admission.rational_retained(operation, operands, retained, evaluate)
    } else {
        evaluate(&ExactValues::resident())
            .map(|value| (value, 0))
            .map_err(GeoError::NumericalScratch)
    }
}

/// Keep one original arithmetic body for legacy and admitted entry points.
pub(crate) fn compare_rational(
    admission: Option<&mut ExactAdmission<'_, '_>>,
    a: &Rat,
    b: &Rat,
) -> Result<core::cmp::Ordering, GeoError> {
    match admission {
        Some(admission) => admission.compare(a, b),
        None => Ok(a.cmp(b)),
    }
}

/// Canonical equality reads original limbs without producing arithmetic scratch.
pub(crate) fn equal_rational(
    admission: Option<&mut ExactAdmission<'_, '_>>,
    a: &Rat,
    b: &Rat,
) -> Result<bool, GeoError> {
    match admission {
        Some(admission) => {
            let cost = canonical_rational_scan_cost(&[a, b])
                .ok_or(GeoError::ArithmeticOverflow("canonical equality admission"))?;
            admission
                .progress
                .exact(admission.context, cost, || Ok(a == b))
        }
        None => Ok(a == b),
    }
}

/// Original borrowed canonical limb scans create no destination or carry bit.
pub(crate) fn canonical_rational_scan_cost(
    operands: &[&Rat],
) -> Option<purrdf_xsd::integer::ExactArithmeticCost> {
    let mut cost = rational_cost(purrdf_xsd::integer::ExactOperation::Linear, operands, 1)?;
    cost.workspace_bytes = 0;
    cost.output_bits = operands
        .iter()
        .map(|value| rational_operand_bits(value))
        .max()
        .unwrap_or(1);
    Some(cost)
}

/// Keep one original arithmetic body for legacy and admitted entry points.
pub(crate) fn exact_rational<T>(
    admission: Option<&mut ExactAdmission<'_, '_>>,
    operation: purrdf_xsd::integer::ExactOperation,
    operands: &[&Rat],
    evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
) -> Result<T, GeoError> {
    exact_rational_counted(admission, operation, operands, 1, evaluate)
}

/// The same original body with an explicit complete rational-operation count.
pub(crate) fn exact_rational_counted<T>(
    admission: Option<&mut ExactAdmission<'_, '_>>,
    operation: purrdf_xsd::integer::ExactOperation,
    operands: &[&Rat],
    count: u64,
    evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
) -> Result<T, GeoError> {
    if let Some(admission) = admission {
        admission.expression(operation, operands, count, evaluate)
    } else {
        evaluate(&ExactValues::resident()).map_err(GeoError::NumericalScratch)
    }
}

pub(crate) fn rational_cost(
    operation: purrdf_xsd::integer::ExactOperation,
    operands: &[&Rat],
    count: u64,
) -> Option<purrdf_xsd::integer::ExactArithmeticCost> {
    purrdf_xsd::integer::ExactArithmeticCost::for_rational_operands(
        operation,
        operands
            .iter()
            .map(|value| (value.numerator().bit_len(), value.denominator().bit_len())),
        count,
    )
}

/// Admit every original numerator/denominator copy in a borrowed operand stream.
/// The sum records each actual field rather than charging the widest field an
/// inferred table population or constructing an owned metadata array.
pub(crate) fn rational_copy_cost<'a>(
    operands: impl IntoIterator<Item = &'a Rat>,
) -> Option<purrdf_xsd::integer::ExactArithmeticCost> {
    use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
    operands.into_iter().try_fold(
        ExactArithmeticCost {
            work_items: 0,
            workspace_bytes: 0,
            output_bits: 0,
        },
        |cost, value| {
            cost.followed_by(ExactArithmeticCost::for_rational_operands(
                ExactOperation::Linear,
                [(value.numerator().bit_len(), value.denominator().bit_len())],
                2,
            )?)
        },
    )
}

/// Bound the one normalized numerator/denominator decimal-field rendering.
/// Callers add their framing/container/hash storage around these exact fields.
pub(crate) fn rational_field_cost(
    value: &Rat,
) -> Result<(purrdf_xsd::integer::ExactArithmeticCost, u64), GeoError> {
    use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
    let mut cost = ExactArithmeticCost {
        work_items: 0,
        workspace_bytes: 0,
        output_bits: 0,
    };
    let mut bytes = 0_u64;
    for integer in [value.numerator(), value.denominator()] {
        bytes = bytes
            .checked_add(integer.bit_len().div_ceil(3) + 2)
            .ok_or(GeoError::ArithmeticOverflow("rational field bytes"))?;
        cost = cost
            .followed_by(
                ExactArithmeticCost::for_operation(
                    ExactOperation::DecimalRender,
                    integer.bit_len(),
                    1,
                )
                .ok_or(GeoError::ArithmeticOverflow("rational field rendering"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("rational field work"))?;
    }
    Ok((cost, bytes))
}

/// The ordinary small root population stays in the existing inline home.
/// An arbitrary larger population spills through the original retained array.
#[expect(
    clippy::large_enum_variant,
    reason = "Inline root records require no new control allocation; larger populations use admitted spill storage."
)]
pub(crate) enum NativeRootValues {
    Inline(purrdf_core::SmallVec<[purrdf_xsd::math::RootIsolation2; 4]>),
    Spill(crate::context::RetainedBuilder<purrdf_xsd::math::RootIsolation2>),
}

impl Default for NativeRootValues {
    fn default() -> Self {
        Self::Inline(purrdf_core::SmallVec::default())
    }
}

impl NativeRootValues {
    pub(crate) fn admitted_bytes(&self) -> usize {
        match self {
            Self::Inline(_) => 0,
            Self::Spill(values) => values.admitted_bytes(),
        }
    }
    fn replacement_bytes(&self, progress: &WorkProgress<'_>) -> Result<Option<usize>, GeoError> {
        let required = self.len().checked_add(1).ok_or(GeoError::Storage(
            purrdf_lex::allocation::StorageError::SizeOverflow,
        ))?;
        match self {
            Self::Inline(values) if required <= values.inline_size() => Ok(None),
            Self::Inline(_) => {
                crate::context::RetainedBuilder::<purrdf_xsd::math::RootIsolation2>::default()
                    .replacement_bytes(required, progress)
            }
            Self::Spill(values) => values.replacement_bytes(required, progress),
        }
    }
    fn try_push(
        &mut self,
        root: purrdf_xsd::math::RootIsolation2,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        match self {
            Self::Inline(values) if values.len() < values.inline_size() => {
                values.push(root);
                Ok(())
            }
            Self::Spill(values) => values.push(root, progress),
            Self::Inline(values) => {
                let required = values.len().checked_add(1).ok_or(GeoError::Storage(
                    purrdf_lex::allocation::StorageError::SizeOverflow,
                ))?;
                let mut replacement = crate::context::RetainedBuilder::default();
                replacement.reserve(required, progress)?;
                for value in core::mem::take(values) {
                    replacement.push_reserved(value);
                }
                replacement.push_reserved(root);
                *self = Self::Spill(replacement);
                Ok(())
            }
        }
    }
    fn clear(&mut self) {
        match self {
            Self::Inline(values) => values.clear(),
            Self::Spill(values) => values.truncate(0),
        }
    }
}

impl core::ops::Deref for NativeRootValues {
    type Target = [purrdf_xsd::math::RootIsolation2];
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Inline(values) => values,
            Self::Spill(values) => values,
        }
    }
}
impl core::ops::DerefMut for NativeRootValues {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Inline(values) => values,
            Self::Spill(values) => values,
        }
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "Consuming inline root records moves their original fixed-capacity carrier without allocating during cleanup."
)]
pub(crate) enum NativeRootIter {
    Inline(purrdf_core::small::IntoIter<[purrdf_xsd::math::RootIsolation2; 4]>),
    Spill(crate::context::retained::RetainedIter<purrdf_xsd::math::RootIsolation2>),
}

impl Iterator for NativeRootIter {
    type Item = purrdf_xsd::math::RootIsolation2;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Inline(values) => values.next(),
            Self::Spill(values) => values.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Inline(values) => values.size_hint(),
            Self::Spill(values) => values.size_hint(),
        }
    }
}
impl ExactSizeIterator for NativeRootIter {}
impl IntoIterator for NativeRootValues {
    type Item = purrdf_xsd::math::RootIsolation2;
    type IntoIter = NativeRootIter;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Inline(values) => NativeRootIter::Inline(values.into_iter()),
            Self::Spill(values) => NativeRootIter::Spill(values.into_iter()),
        }
    }
}

/// Original root result buffer, shared with the solver through a stack borrow.
/// Numerical callbacks and container growth borrow the one progress account
/// sequentially; no mutable borrow spans a callback or a native allocation.
pub(crate) struct NativeRootStorage<'a, 'progress, 'observer> {
    pub(crate) values: NativeRootValues,
    pub(crate) progress: &'a core::cell::RefCell<&'progress mut WorkProgress<'observer>>,
}

impl purrdf_xsd::math::RootStorage2 for NativeRootStorage<'_, '_, '_> {
    fn storage_bytes(&self) -> Result<usize, MathError> {
        Ok(self.values.admitted_bytes())
    }
    fn insertion_layout(&self) -> Result<Option<(usize, usize)>, MathError> {
        let result = self.values.replacement_bytes(&self.progress.borrow());
        result
            .map(|next| next.map(|bytes| (bytes, self.values.admitted_bytes())))
            .map_err(|error| self.progress.borrow_mut().math_failure(error))
    }
    fn roots(&self) -> &[purrdf_xsd::math::RootIsolation2] {
        &self.values
    }
    fn roots_mut(&mut self) -> &mut [purrdf_xsd::math::RootIsolation2] {
        &mut self.values
    }
    fn clear(&mut self) {
        self.values.clear();
    }
    fn reset(&mut self) {
        drop(core::mem::take(&mut self.values));
    }
    fn try_push(&mut self, root: purrdf_xsd::math::RootIsolation2) -> Result<(), MathError> {
        let result = self.values.try_push(root, &mut self.progress.borrow_mut());
        result.map_err(|error| self.progress.borrow_mut().math_failure(error))
    }
}

// Original systems all carry WorkProgress. This native trait adapter shares
// that one storage callback without repeating a buffer/allocation body.
macro_rules! native_root_storage {
    () => {
        fn with_storage<T>(
            &mut self,
            resident_bytes: usize,
            make: impl FnOnce(
                &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
            ) -> Result<T, purrdf_lex::allocation::StorageError>,
        ) -> Result<T, purrdf_lex::allocation::StorageError> {
            self.progress.with_storage(resident_bytes, make)
        }
    };
    (cell) => {
        fn with_storage<T>(
            &mut self,
            resident_bytes: usize,
            make: impl FnOnce(
                &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
            ) -> Result<T, purrdf_lex::allocation::StorageError>,
        ) -> Result<T, purrdf_lex::allocation::StorageError> {
            self.progress
                .borrow_mut()
                .with_storage(resident_bytes, make)
        }
    };
}
pub(crate) use native_root_storage;

/// Extend a numerical traversal through the native work-list spill home.
/// The old spill remains live until its replacement has moved every entry.
pub(crate) fn push_math_work<T, const N: usize>(
    pending: &mut purrdf_lex::walk::WorkList<T, N>,
    value: T,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    retained: &mut usize,
) -> Result<(), MathError> {
    let replacement = pending.insertion_layout().map_err(MathError::Storage)?;
    math_grow_work(replacement, math, progress, retained, |memory| {
        pending.try_push_admitted(value, memory)
    })
}

/// Native priority queues use the same policy/physical replacement admission
/// as traversal spills. Ordering remains in the original binary-heap body.
pub(crate) fn push_math_heap<T: Ord>(
    heap: &mut std::collections::BinaryHeap<T>,
    value: T,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    retained: &mut usize,
) -> Result<(), MathError> {
    let replacement =
        purrdf_lex::allocation::heap_insertion_layout(heap).map_err(MathError::Storage)?;
    math_grow_work(replacement, math, progress, retained, |memory| {
        memory.push_heap(heap, value)
    })
}

fn math_grow_work(
    replacement: Option<(usize, usize)>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    retained: &mut usize,
    make: impl FnOnce(
        &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
    ) -> Result<(), purrdf_lex::allocation::StorageError>,
) -> Result<(), MathError> {
    let next = match replacement {
        Some((bytes, previous)) => {
            let total = retained
                .checked_add(bytes)
                .and_then(|total| total.checked_sub(previous))
                .ok_or(MathError::Storage(
                    purrdf_lex::allocation::StorageError::SizeOverflow,
                ))?;
            math.reserve_workspace(bytes)?;
            Some((bytes, previous, total))
        }
        None => None,
    };
    let result = progress
        .with_storage(*retained, make)
        .map_err(MathError::Storage);
    if let Err(error) = result {
        if let Some((bytes, _, _)) = next {
            let _ = math.release_workspace(bytes);
        }
        return Err(error);
    }
    if let Some((_, previous, total)) = next {
        *retained = total;
        math.release_workspace(previous)?;
    }
    Ok(())
}

/// Destroy the original pending entries and spill before refunding its layout.
pub(crate) fn release_math_work<T, const N: usize>(
    pending: purrdf_lex::walk::WorkList<T, N>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    retained: usize,
) -> Result<(), MathError> {
    let destroyed = progress
        .with_storage(retained, |memory| pending.release_admitted(memory))
        .map_err(MathError::Storage);
    let released = math.release_workspace(retained);
    destroyed.and(released)
}

/// Refund a priority queue only after its actual entries and backing array die.
pub(crate) fn release_math_heap<T: Ord>(
    heap: std::collections::BinaryHeap<T>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    retained: usize,
) -> Result<(), MathError> {
    let destroyed = progress
        .with_storage(retained, |memory| memory.release_heap(heap))
        .map_err(MathError::Storage);
    let released = math.release_workspace(retained);
    destroyed.and(released)
}

/// Copy a worker-owned original reference only after its actual parameter limbs
/// and prepared bound storage have been admitted in the one arithmetic home.
pub(crate) fn reference_clone(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<crate::GeographicReference, GeoError> {
    let cost = reference_copy_cost(context.reference())?;
    context.charge_work(cost.work_items)?;
    progress.context_poll(context)?;
    crate::context::retained::produce_persistent_numeric_from(context.reference(), progress)
        .map(crate::context::retained::PersistentNumeric::into_value)
}

pub(crate) fn reference_copy_cost(
    reference: &crate::GeographicReference,
) -> Result<purrdf_xsd::integer::ExactArithmeticCost, GeoError> {
    let ellipsoid = reference.ellipsoid();
    let (minimum, maximum) = ellipsoid.normal_metric_bounds_ref();
    rational_cost(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[
            ellipsoid.semimajor(),
            ellipsoid.semiminor(),
            ellipsoid.inverse_flattening(),
            minimum.exact(),
            maximum.exact(),
            ellipsoid.eccentricity_squared_ref(),
        ],
        6,
    )
    .ok_or(GeoError::ArithmeticOverflow("original reference copy"))
}

/// Compare two original declarations only after admitting both complete limb
/// scans through the existing reference operand inventory. Equality borrows
/// every field, so it has no temporary result or heap scratch to reserve.
pub(crate) fn reference_matches(
    reference: &crate::GeographicReference,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    compare_reference(
        reference,
        None,
        context,
        progress,
        crate::GeographicReference::eq,
    )
}

/// Admit the same borrowed declaration scan before testing the original
/// physical surface, which deliberately ignores the carrier's axis order.
pub(crate) fn reference_same_surface(
    reference: &crate::GeographicReference,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    compare_reference(
        reference,
        None,
        context,
        progress,
        crate::GeographicReference::same_surface,
    )
}

/// Admit both explicitly supplied declarations before comparing their original
/// physical surfaces. The context supplies the budget, not either declaration.
#[cfg(test)]
pub(crate) fn reference_same_surface_pair(
    left: &crate::GeographicReference,
    right: &crate::GeographicReference,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    compare_reference(
        left,
        Some(right),
        context,
        progress,
        crate::GeographicReference::same_surface,
    )
}

fn compare_reference(
    reference: &crate::GeographicReference,
    other: Option<&crate::GeographicReference>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    compare: fn(&crate::GeographicReference, &crate::GeographicReference) -> bool,
) -> Result<bool, GeoError> {
    let mut cost = reference_copy_cost(reference)?
        .followed_by(reference_copy_cost(
            other.unwrap_or_else(|| context.reference()),
        )?)
        .ok_or(GeoError::ArithmeticOverflow(
            "original reference comparison",
        ))?;
    cost.workspace_bytes = 0;
    progress.exact_context(context, cost, |context| {
        Ok(compare(
            reference,
            other.unwrap_or_else(|| context.reference()),
        ))
    })
}

/// Admit the original reference-ID rendering before a cold computation or a
/// cached read. An absent source selects the context's immutable reference;
/// an explicit source checks an independently supplied carrier declaration.
pub(crate) fn reference_identity(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    source: Option<&crate::GeographicReference>,
) -> Result<crate::GeoBindingId, GeoError> {
    let reference = source
        .cloned()
        .unwrap_or_else(|| context.reference().clone());
    let cost = reference_identity_cost(&reference)?;
    let identity = if let Some(identity) = reference.cached_identity() {
        progress.exact(context, cost, || Ok(identity))?
    } else {
        ExactAdmission::new(context, progress)
            .expression_cost(cost, |values| reference_identity_using(&reference, values))?
    };
    source
        .unwrap_or_else(|| context.reference())
        .cache_identity(identity);
    Ok(identity)
}

/// The same original ID admission inside an already governed numerical phase.
/// This covers independently supplied immutable references before their cold
/// canonical rational rendering; cached reads keep the same declared bound.
pub(crate) fn math_reference_identity(
    reference: &crate::GeographicReference,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<crate::GeoBindingId, MathError> {
    let cost = reference_identity_cost(reference).map_err(|_| MathError::WorkExhausted)?;
    math.admit_exact_cost(cost)?;
    progress.math_poll(math)?;
    let identity = if let Some(identity) = reference.cached_identity() {
        identity
    } else {
        let inline;
        let scratch = if let Some(scratch) = math.limb_scratch() {
            scratch
        } else if cost.destination_limbs().is_some_and(|limbs| limbs <= 3) {
            inline = purrdf_xsd::integer::LimbScratch::inline();
            &inline
        } else {
            return Err(progress.math_failure(GeoError::ArithmeticOverflow(
                "reference identity requires its prepared integer arena",
            )));
        };
        reference_identity_using(reference, &ExactValues::admitted(scratch))
            .map_err(|error| progress.math_failure(error))?
    };
    reference.cache_identity(identity);
    progress.math_poll(math)?;
    Ok(identity)
}

/// Format the original declared source/target mismatch only after admitting
/// both cold identities and their two fixed-width hexadecimal fields.
pub(crate) fn missing_operation(
    source: &crate::GeographicReference,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<GeoError, GeoError> {
    let source = reference_identity(context, progress, Some(source))?;
    let target = reference_identity(context, progress, None)?;
    missing_operation_ids(source, target, context, progress)
}

/// Format two already admitted immutable binding IDs through the same fixed
/// hexadecimal fields as a full original-reference mismatch.
pub(crate) fn missing_operation_ids(
    source: crate::GeoBindingId,
    target: crate::GeoBindingId,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<GeoError, GeoError> {
    let source = source.digest();
    let target = target.digest();
    let cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
        purrdf_xsd::integer::ExactOperation::Linear,
        128 * 8,
        2,
    )
    .ok_or(GeoError::ArithmeticOverflow(
        "reference mismatch formatting",
    ))?;
    progress.exact(context, cost, || Ok(()))?;
    let error = missing_binding_error(source, target, progress);
    if matches!(error.original(), GeoError::MissingOperation { .. }) {
        Ok(error)
    } else {
        Err(error)
    }
}

/// Produce the original fixed-width mismatch fields under their original owner.
/// Callers govern identity/work at their existing semantic admission sites.
pub(crate) fn missing_binding_error(
    source: purrdf_hash::hex::Digest32,
    target: purrdf_hash::hex::Digest32,
    progress: &mut WorkProgress<'_>,
) -> GeoError {
    progress.retained_error(|memory| {
        Ok(GeoError::MissingOperation {
            source: memory.format(&source)?,
            target: memory.format(&target)?,
        })
    })
}

/// Bound cold original reference-ID rendering and canonical hash framing.
/// Warm cache hits preserve the same conservative configuration/work admission.
pub(crate) fn reference_identity_cost(
    reference: &crate::GeographicReference,
) -> Result<purrdf_xsd::integer::ExactArithmeticCost, GeoError> {
    use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
    let mut cost = ExactArithmeticCost {
        work_items: 0,
        workspace_bytes: 0,
        output_bits: 0,
    };
    // Domain, datum, axes, unit declarations and the eight length frames.
    let mut bytes = 256_u64;
    for value in [
        reference.ellipsoid().semimajor(),
        reference.ellipsoid().inverse_flattening(),
    ] {
        let (render, text) = rational_field_cost(value)?;
        cost = cost
            .followed_by(render)
            .ok_or(GeoError::ArithmeticOverflow("reference identity work"))?;
        bytes = bytes
            .checked_add(text)
            .ok_or(GeoError::ArithmeticOverflow("reference identity bytes"))?;
    }
    let arithmetic_bits = cost.output_bits;
    let hash = ExactArithmeticCost::for_operation(
        ExactOperation::Linear,
        bytes
            .checked_mul(8)
            .ok_or(GeoError::ArithmeticOverflow("reference identity hash work"))?,
        8,
    )
    .ok_or(GeoError::ArithmeticOverflow("reference identity hash work"))?;
    cost = cost
        .followed_by(hash)
        .ok_or(GeoError::ArithmeticOverflow("reference identity work"))?;
    // The canonical preimage is streamed through the hash home. Its byte
    // length prices that scan, not the width of a numerical destination.
    cost.output_bits = arithmetic_bits;
    cost.workspace_bytes = cost
        .workspace_bytes
        .checked_add(bytes)
        .ok_or(GeoError::ArithmeticOverflow("reference identity scratch"))?;
    Ok(cost)
}

/// Admit the linear original-coordinate equality proof before the shared
/// physical alias law scans any caller-provided rational limbs.
pub(crate) fn same_location_admitted(
    a: &crate::LonLat,
    b: &crate::LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    ExactAdmission::new(context, progress).expression_result(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
        4,
        |_| Ok(a.same_location(b)),
    )
}

/// Admit one exact rational body in a standalone numerical proof context.
pub(crate) fn math_exact_rational<T>(
    operation: purrdf_xsd::integer::ExactOperation,
    operands: &[&Rat],
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
) -> Result<T, MathError> {
    math_exact_rationals(operation, operands, 1, math, progress, evaluate)
}

pub(crate) fn math_exact_rationals<T>(
    operation: purrdf_xsd::integer::ExactOperation,
    operands: &[&Rat],
    count: u64,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
) -> Result<T, MathError> {
    let cost = rational_cost(operation, operands, count).ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(cost)?;
    progress.math_poll(math)?;
    let result = {
        let progress = core::cell::RefCell::new(&mut *progress);
        let diagnostic =
            |value: &Rat| GeoError::diagnostic_number(value, &mut progress.borrow_mut());
        evaluate(&ExactValues::with_diagnostics(
            math.limb_scratch(),
            &diagnostic,
        ))
        .map_err(MathError::LimbScratch)?
    };
    progress.math_poll(math)?;
    Ok(result)
}

pub(crate) fn fixed_from_rat(
    value: &Rat,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    // The original ratio kernel borrows source limbs. Its intermediate products,
    // quotient, remainder and endpoints use the already admitted concrete arena;
    // copying both operands into two more destinations is unnecessary.
    FixedInterval::from_ratio(
        value.numerator().as_ref(),
        value.denominator().as_ref(),
        math,
    )
}

/// Bridge an inclusive exact rational enclosure onto the one dyadic grid.
pub(crate) fn fixed_from_bounds(
    lower: &Rat,
    upper: &Rat,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    if lower > upper {
        return Err(MathError::Domain("reversed rational enclosure"));
    }
    let lower = fixed_from_rat(lower, math)?;
    let upper = fixed_from_rat(upper, math)?;
    FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), math)
}

pub(crate) fn exact_bounds(value: &FixedInterval) -> (Rat, Rat) {
    (
        Rat::from_dyadic_integer(value.lower(), value.precision_bits()),
        Rat::from_dyadic_integer(value.upper(), value.precision_bits()),
    )
}

/// Exact canonical receipt endpoints using the enclosing admitted arena.
pub(crate) fn exact_bounds_in(
    value: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<(Rat, Rat), MathError> {
    let bits = value
        .lower()
        .bit_len()
        .max(value.upper().bit_len())
        .max(u64::from(value.precision_bits()) + 1);
    math.admit_exact(
        4,
        usize::try_from(bits).map_err(|_| MathError::WorkspaceExhausted)?,
    )?;
    if let Some(scratch) = math.limb_scratch() {
        Ok((
            Rat::from_dyadic_integer_in(value.lower(), value.precision_bits(), scratch)
                .map_err(MathError::LimbScratch)?,
            Rat::from_dyadic_integer_in(value.upper(), value.precision_bits(), scratch)
                .map_err(MathError::LimbScratch)?,
        ))
    } else {
        Ok(exact_bounds(value))
    }
}

pub(crate) fn carrier_integer(value: &Int) -> Int {
    value.clone()
}

/// Build an exact completed decimal only after admitting its original integer
/// copy and complete power-of-two/five cancellation. The caller retains
/// output storage separately while subsequent numerical bodies run.
pub(crate) fn math_quantized_decimal(
    value: &Int,
    places: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, MathError> {
    let cost = purrdf_xsd::integer::ExactArithmeticCost::decimal_rational(value.bit_len(), places)
        .ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(cost)?;
    progress.math_poll(math)?;
    let answer = match math.limb_scratch() {
        Some(scratch) => {
            Rat::from_decimal_in(value, places, scratch).map_err(MathError::LimbScratch)?
        }
        None => Rat::from_decimal(carrier_integer(value), places),
    };
    progress.math_poll(math)?;
    Ok(answer)
}

/// Correctly round an original exact source through the shared rational/IEEE
/// body, including proven half-even ties that no nonzero interval can isolate.
pub(crate) fn math_round_decimal(
    value: &Rat,
    places: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Int, MathError> {
    let cost = purrdf_xsd::integer::ExactArithmeticCost::rational_round(
        value.numerator().bit_len(),
        value.denominator().bit_len(),
        places,
    )
    .ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(cost)?;
    progress.math_poll(math)?;
    let answer = match math.limb_scratch() {
        Some(scratch) => value
            .round_to_scale_in(places, scratch)
            .map_err(MathError::LimbScratch)?,
        None => value.round_to_scale(places),
    };
    progress.math_poll(math)?;
    Ok(answer)
}

pub(crate) fn geo_math_error(error: &MathError, policy: ExecutionPolicy) -> GeoError {
    let limits = policy.limits();
    match error {
        // Native mathematics supplies an immutable static cause. Borrow it
        // directly: constructing a geometry error must not allocate outside
        // the caller's account, and its variant/presentation stay unchanged.
        MathError::Domain(message) => GeoError::Domain(std::borrow::Cow::Borrowed(*message)),
        MathError::PrecisionExhausted => GeoError::PrecisionExhausted {
            bits: limits.max_precision_bits,
        },
        MathError::WorkExhausted => GeoError::WorkExhausted {
            limit: limits.max_work_items,
        },
        MathError::WorkspaceExhausted => GeoError::MemoryExhausted {
            limit: limits.max_workspace_bytes,
        },
        MathError::LimbScratch(error) => GeoError::NumericalScratch(*error),
        MathError::TaylorScratch(error) => GeoError::NumericalTaylorScratch(*error),
        MathError::Storage(error) => GeoError::Storage(*error),
        MathError::Binary64Range => GeoError::ArithmeticOverflow("finite numerical enclosure"),
        MathError::Cancelled => GeoError::Cancelled,
        MathError::ConvergenceExhausted { iterations } => GeoError::ConvergenceExhausted {
            iterations: *iterations,
        },
        MathError::AmbiguousRoots { roots } => GeoError::AmbiguousTransform {
            roots: u64::try_from(*roots).unwrap_or(u64::MAX),
        },
        MathError::Environment(error) => GeoError::FloatEnvironment(*error),
        _ => GeoError::PrecisionExhausted {
            bits: limits.max_precision_bits,
        },
    }
}

/// Maximum original operand width, including the reference axes, for checked
/// destination preparation. This reads integer metadata without allocating.
pub(crate) fn scratch_source_bits(values: &[&Rat], ellipsoid: &crate::PreparedEllipsoid) -> u64 {
    scratch_source_bits_in(values.iter().copied(), ellipsoid)
}

/// Stream original operands without allocating a source collection.
pub(crate) fn scratch_source_bits_in<'a>(
    values: impl Iterator<Item = &'a Rat>,
    ellipsoid: &'a crate::PreparedEllipsoid,
) -> u64 {
    values
        .chain([ellipsoid.semimajor(), ellipsoid.semiminor()])
        .map(rational_operand_bits)
        .max()
        .unwrap_or(0)
}

/// Read complete original rational magnitude metadata without copying limbs.
pub(crate) fn rational_operand_bits(value: &Rat) -> u64 {
    value
        .numerator()
        .bit_len()
        .max(value.denominator().bit_len())
}

/// Prepare immutable rational ownership while its numerical phase is live.
/// Limb arithmetic remains in XSD; the complete shared buffers and headers are
/// reserved and observed before detaching reusable destinations.
pub(crate) fn math_share_rationals<const N: usize>(
    values: [Rat; N],
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<([Rat; N], u64), MathError> {
    let bytes = crate::context::persistent_numeric_bytes(&values, progress)
        .map_err(|error| progress.math_failure(error))?;
    let bits = values.iter().map(rational_operand_bits).max().unwrap_or(0);
    math.admit_exact(
        (N as u64).checked_mul(4).ok_or(MathError::WorkExhausted)?,
        usize::try_from(bits).map_err(|_| MathError::WorkspaceExhausted)?,
    )?;
    let bytes_usize = usize::try_from(bytes).map_err(|_| MathError::WorkspaceExhausted)?;
    math.reserve_workspace(bytes_usize)?;
    let result = progress.math_poll(math).and_then(|()| {
        crate::context::produce_persistent_numeric(values, progress)
            .map_err(|error| progress.math_failure(error))
    });
    match result {
        Ok(values) => Ok((values.into_value(), bytes)),
        Err(error) => {
            let _ = math.release_workspace(bytes_usize);
            Err(error)
        }
    }
}

/// Detach a borrowed native field record under its original physical lease.
/// Its concrete scalar layout and retained control are admitted before the
/// shared factory copies any fields; the caller owns its live policy receipt.
pub(crate) fn math_share_fields<T: crate::context::NativeDetached>(
    value: &T,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    retained: &mut usize,
    policy: ExecutionPolicy,
) -> Result<T, GeoError> {
    let bytes = usize::try_from(crate::context::persistent_numeric_bytes(value, progress)?)
        .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?;
    let total = retained.checked_add(bytes).ok_or(GeoError::Storage(
        purrdf_lex::allocation::StorageError::SizeOverflow,
    ))?;
    math.reserve_workspace(bytes)
        .map_err(|error| geo_math_error(&error, policy))?;
    let result = progress
        .math_poll(math)
        .map_err(|error| geo_math_error(&error, policy))
        .and_then(|()| crate::context::produce_persistent_numeric_from(value, progress));
    match result {
        Ok(value) => {
            *retained = total;
            Ok(value.into_value())
        }
        Err(error) => {
            let _ = math.release_workspace(bytes);
            Err(error)
        }
    }
}

/// Execute one bounded numerical phase under the unchanged outer admission.
/// The context owns no floating guard; a borrowed floating subcontext must pause
/// its guard before invoking WorkProgress, which revalidates after callbacks.
/// Every exit accounts actual work/peak and releases all phase reservations.
/// A coefficient shape request can restart the phase at the same precision
/// after its temporaries are dropped and a larger arena is admitted. The
/// callback must construct attempt-local owners each time and transfer external
/// state only on success. Failed work remains charged across these attempts.
pub(crate) fn with_math<T>(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    precision_bits: u32,
    reservation: usize,
    evaluate: impl FnMut(&mut CoordinateMath, &mut WorkProgress<'_>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    with_math_for_sources(
        context,
        progress,
        precision_bits,
        reservation,
        &[],
        evaluate,
    )
}

/// Admit the complete original source widths before preparing reusable limbs.
/// The output grid never substitutes for the exact numerator/denominator width.
pub(crate) fn with_math_for_sources<T>(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    precision_bits: u32,
    reservation: usize,
    sources: &[&Rat],
    mut evaluate: impl FnMut(&mut CoordinateMath, &mut WorkProgress<'_>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    with_math_for_sources_retained(
        context,
        progress,
        precision_bits,
        reservation,
        sources,
        |math, progress| evaluate(math, progress).map(|value| (value, 0)),
    )
    .map(|(value, _)| value)
}

/// Transfer a result's already admitted numerical storage into its outer owner.
/// The producer reserves and observes every detached copy before constructing it,
/// and reports only storage still live at return. That allowance is subtracted
/// from the phase release; the caller drops the owner before releasing it.
pub(crate) fn with_math_for_sources_retained<T>(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    precision_bits: u32,
    reservation: usize,
    sources: &[&Rat],
    mut evaluate: impl FnMut(&mut CoordinateMath, &mut WorkProgress<'_>) -> Result<(T, u64), GeoError>,
) -> Result<(T, u64), GeoError> {
    loop {
        if let Some(error) = progress.latched_error() {
            return Err(error);
        }
        let policy = context.policy();
        if precision_bits > policy.limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted {
                bits: policy.limits().max_precision_bits,
            });
        }
        // CoordinateMath::new requires max_work > 0. Check that same minimum
        // before preparing an arena for a phase that cannot begin; this does not
        // charge work or change any successful phase's accounting.
        context.next_work(1)?;
        let source_bits = scratch_source_bits(sources, context.reference().ellipsoid());
        context.prepare_integer_scratch_at_precision_observed(
            source_bits,
            precision_bits,
            progress,
        )?;
        progress.context_poll(context)?;
        let mut math = context
            .coordinate_math(MathLimits {
                precision_bits,
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
        math.set_binary64_backend(context.binary64_backend())
            .map_err(|error| geo_math_error(&error, policy))?;
        progress.math_phase(context);
        let result = math
            .reserve_workspace(reservation)
            .map_err(|error| geo_math_error(&error, policy))
            .and_then(|()| {
                progress
                    .math_poll(&math)
                    .map_err(|error| geo_math_error(&error, policy))?;
                evaluate(&mut math, progress)
            });
        let output_bytes = result.as_ref().map_or(0, |(_, bytes)| *bytes);
        let transfer = usize::try_from(output_bytes)
            .ok()
            .filter(|bytes| *bytes <= math.workspace_reserved())
            .ok_or(GeoError::ArithmeticOverflow(
                "numerical result storage transfer",
            ));
        let observed = progress
            .math_poll(&math)
            .map_err(|error| geo_math_error(&error, policy));
        let released = math
            .release_workspace(math.workspace_reserved())
            .map_err(|error| geo_math_error(&error, policy));
        let charged = context.charge_work(math.work_used());
        let peak_bytes = math.workspace_peak() as u64;
        let peak = context.admit_workspace(peak_bytes);
        let kept = if peak.is_ok() && transfer.is_ok() {
            output_bytes
        } else {
            0
        };
        let peak_release = if peak.is_ok() {
            context.release_workspace(peak_bytes - kept)
        } else {
            Ok(())
        };
        let (prepared_coefficients, coefficient_bytes) = progress.take_prepared_coefficients();
        let completed = (|| {
            if let Some(error) = progress.latched_error() {
                return Err(error);
            }
            charged?;
            peak?;
            peak_release?;
            transfer?;
            released?;
            observed?;
            if result.is_ok()
                && let Some(scratch) = math.taylor_scratch()
            {
                context.adopt_taylor_scratch(scratch)?;
            }
            if result.is_ok()
                && coefficient_bytes != 0
                && let Some(cache) = prepared_coefficients
            {
                context.retain_geodesic_coefficients(cache, coefficient_bytes)?;
            }
            Ok(())
        })();
        if let Err(error) = completed {
            drop(result);
            context.release_workspace(kept)?;
            drop(math);
            let _ = context.release_unused_integer_scratch();
            return Err(error);
        }
        drop(math);
        if let Err(error) = &result {
            match context.grow_requested_integer_scratch(error, progress) {
                Ok(true) => continue,
                Ok(false) => {}
                Err(error) => {
                    let released = context.release_unused_integer_scratch();
                    return progress.settle_result(Err(error), released);
                }
            }
        }
        let released = context.release_unused_integer_scratch();
        return progress.settle_result(result, released);
    }
}

/// Decode an internally frozen exact decimal constant in its one helper home.
pub(crate) fn frozen_decimal(text: &str) -> Rat {
    Rat::parse_decimal(text).expect("frozen exact decimal")
}

/// Construct a declared decimal quantum before any power allocation, keeping
/// its admitted output allowance live through the consuming exact proof.
pub(crate) fn with_decimal_quantum<T>(
    places: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    consume: impl FnOnce(&Rat, &mut MetricContext, &mut WorkProgress<'_>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let maximum = context.policy().limits().max_precision_bits;
    if places > maximum {
        return Err(GeoError::PrecisionExhausted { bits: maximum });
    }
    let cost = purrdf_xsd::integer::ExactArithmeticCost::decimal(1, u64::from(places)).ok_or(
        GeoError::ArithmeticOverflow("declared decimal grid exponent"),
    )?;
    if cost.output_bits > u64::from(maximum) {
        return Err(GeoError::PrecisionExhausted { bits: maximum });
    }
    progress.preflight_exact(context, cost, cost.workspace_bytes)?;
    context.admit_workspace(cost.workspace_bytes)?;
    let result = (|| {
        let quantum =
            progress.exact(context, cost, || Ok(Rat::from_decimal(Int::one(), places)))?;
        consume(&quantum, context, progress)
    })();
    context.release_workspace(cost.workspace_bytes)?;
    result
}

/// Certified angular L1 rounding displacement, optionally including a physical
/// height quantum. The shared original bound uses pi < 22/7 and the ellipsoid's
/// prepared normal-metric upper radius; refinement never chooses the grid.
pub(crate) fn validate_angular_output_grid(
    places: u32,
    radius: &Rat,
    height_places: Option<u32>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    use purrdf_xsd::integer::ExactOperation as Op;
    with_decimal_quantum(places, context, progress, |quantum, context, progress| {
        let mut retained = 0;
        let result = (|| {
            let factor = Rat::new(Int::from_u64(22), Int::from_u64(1260)).expect("positive");
            let mut admission = ExactAdmission::new(context, progress);
            let upper = admission.rational_owner(
                Op::RationalMultiply,
                &[radius, &factor],
                &mut retained,
                |values| values.multiply(radius, &factor),
            )?;
            let error_upper = admission.rational_owner(
                Op::RationalMultiply,
                &[&upper, quantum],
                &mut retained,
                |values| values.multiply(&upper, quantum),
            )?;
            let error_upper = if let Some(height_places) = height_places {
                with_decimal_quantum(
                    height_places,
                    context,
                    progress,
                    |height, context, progress| {
                        let two = Rat::from_i64(2);
                        let mut admission = ExactAdmission::new(context, progress);
                        let half = admission.rational_owner(
                            Op::RationalDivide,
                            &[height, &two],
                            &mut retained,
                            |values| Ok(values.divide(height, &two)?.expect("positive divisor")),
                        )?;
                        admission.rational_owner(
                            Op::RationalAdd,
                            &[&error_upper, &half],
                            &mut retained,
                            |values| values.add(&error_upper, &half),
                        )
                    },
                )?
            } else {
                error_upper
            };
            validate_rounding_bound(&error_upper, context, progress)
        })();
        // The scope drops every exact intermediate before returning here;
        // its complete owner allowances also cover all refusal paths.
        context.release_workspace(retained)?;
        result
    })
}

/// A Cartesian response has at most three coordinates: sqrt(3)/2 < 1 bounds
/// its Euclidean half-quantum displacement by one declared metre quantum.
pub(crate) fn validate_metric_output_grid(
    places: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    with_decimal_quantum(places, context, progress, validate_rounding_bound)
}

fn validate_rounding_bound(
    error_upper: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let maximum = Rat::new(Int::one(), Int::from_u64(1_000_000)).expect("positive");
    let bits = context.policy().limits().max_precision_bits;
    ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[error_upper, &maximum],
        1,
        || {
            if error_upper <= &maximum {
                Ok(())
            } else {
                Err(GeoError::PrecisionExhausted { bits })
            }
        },
    )
}

/// Copy original rational limbs only after the one shared exact admission.
pub(crate) fn copy_rat(
    value: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    ExactAdmission::new(context, progress).expression(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[value],
        1,
        |values| values.original(value),
    )
}
/// Compare original rationals through their shared admitted branch body.
pub(crate) fn compare_rat(
    a: &Rat,
    b: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<core::cmp::Ordering, GeoError> {
    ExactAdmission::new(context, progress).compare(a, b)
}
/// Fixed completed conservative path bound; proof tightness cannot change it.
pub(crate) fn quantized_source_linear_bound(
    start: &crate::LonLat,
    end: &crate::LonLat,
    explicit_reference: Option<&crate::GeographicReference>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    let reference = if let Some(reference) = explicit_reference {
        let cost = reference_copy_cost(reference)?;
        progress.exact(context, cost, || Ok(reference.clone()))?
    } else {
        reference_clone(context, progress)?
    };
    let constant_pole = ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[start.latitude(), end.latitude()],
        3,
        || Ok(start.is_pole() && start.latitude() == end.latitude()),
    )?;
    if constant_pole {
        return Ok(Rat::zero());
    }
    let variation = crate::SourceLinearEdge::variation_with(
        start,
        end,
        Some(&mut ExactAdmission::new(context, progress)),
    )?;
    let (_, maximum) = reference.ellipsoid().normal_metric_bounds_ref();
    let sources = [&variation, maximum.exact()];
    let policy = context.policy();
    let limit = policy.limits().max_precision_bits;
    let mut bits = 96.min(limit);
    loop {
        let bound =
            with_math_for_sources(context, progress, bits, 4096, &sources, |math, progress| {
                use purrdf_xsd::math::FixedInterval;
                let evaluate = (|| {
                    let bound =
                        crate::SourceLinearEdge::enclose_upper_bound(&variation, &reference, math)?;
                    let scaled = bound.mul(&FixedInterval::from_i64(1_000_000, math)?, math)?;
                    let (lower, upper) = scaled.ceil_bounds(math)?;
                    if lower != upper {
                        return Ok(None);
                    }
                    let cost = purrdf_xsd::integer::ExactArithmeticCost::for_rational_operands(
                        purrdf_xsd::integer::ExactOperation::RationalReduce,
                        [(lower.bit_len(), 20)],
                        1,
                    )
                    .ok_or(MathError::WorkExhausted)?;
                    math.admit_exact_cost(cost)?;
                    progress.math_poll(math)?;
                    Ok(Some(Rat::from_decimal(carrier_integer(&lower), 6)))
                })();
                evaluate.map_err(|error| geo_math_error(&error, policy))
            })?;
        if let Some(bound) = bound {
            return Ok(bound);
        }
        if bits == limit {
            return Err(GeoError::PrecisionExhausted { bits });
        }
        bits = bits.saturating_mul(2).min(limit);
    }
}

#[cfg(test)]
mod tests {
    use super::{exact_bounds, fixed_from_rat, geo_math_error, with_math_for_sources};
    use crate::{Int, MetricContext, Rat, context::WorkProgress};
    use purrdf_xsd::math::FixedInterval;

    #[test]
    fn static_native_domain_cause_never_allocates_or_changes_presentation() {
        let original = purrdf_xsd::math::MathError::Domain("reversed rational enclosure");
        let expected = crate::GeoError::domain("reversed rational enclosure");
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let error = geo_math_error(&original, crate::ExecutionPolicy::geometry());
        let copy = error.clone();
        assert_eq!(window.close().allocations, 0);
        assert_eq!(error, expected);
        assert_eq!(copy.detail(), expected.detail());
        assert_eq!(copy.is_expression_error(), expected.is_expression_error());
        assert_eq!(copy.to_string(), expected.to_string());
    }

    #[test]
    fn independent_wide_reference_equality_is_admitted_before_its_limb_scan() {
        use crate::{
            AxisOrder, ExecutionLimits, ExecutionPolicy, GeoError, GeographicReference,
            PreparationBudget, PreparedEllipsoid,
        };
        let adequate = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 1_000_000_000,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let independent = || {
            let mut budget = PreparationBudget::new(adequate);
            GeographicReference::new(
                PreparedEllipsoid::new_in_budget(
                    Rat::from_int(Int::one().shl(4096).add(&Int::one())),
                    Rat::from_i64(300),
                    &mut budget,
                )
                .unwrap(),
                purrdf_hash::hex::Digest32::new([103; 32]),
                AxisOrder::LonLat,
            )
        };
        let source = independent();
        let target = independent();
        // Equal canonical values were constructed independently; sharing a
        // cloned source allocation cannot make this a constant pointer test.
        assert_eq!(source, target);
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 1,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(target.clone(), policy).unwrap();
        context.begin_integer(1).unwrap();
        struct NoScan;
        impl crate::MetricWorkObserver for NoScan {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                panic!("refused borrowed limb scan must not start")
            }
        }
        let mut observer = NoScan;
        let mut progress = WorkProgress::integer(Some(&mut observer));
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let refused = super::reference_matches(&source, &mut context, &mut progress);
        let refused_surface = super::reference_same_surface(&source, &mut context, &mut progress);
        let refused_pair =
            super::reference_same_surface_pair(&source, &target, &mut context, &mut progress);
        let allocation = window.close();
        assert_eq!(refused, Err(GeoError::WorkExhausted { limit: 1 }));
        assert_eq!(refused_surface, Err(GeoError::WorkExhausted { limit: 1 }));
        assert_eq!(refused_pair, Err(GeoError::WorkExhausted { limit: 1 }));
        assert_eq!(allocation.allocations, 0);
        assert_eq!(allocation.requested_bytes, 0);
        assert_eq!(context.work_items(), 0);
        assert_eq!(context.workspace_peak(), 0);
        assert!(source.cached_identity().is_none());
        assert!(target.cached_identity().is_none());
        // The explicit pair must not accidentally compare with the budget's
        // unrelated WGS84 declaration instead of the supplied right operand.
        let mut pair_context = MetricContext::new(GeographicReference::wgs84(), adequate).unwrap();
        let mut pair_progress = WorkProgress::integer(None);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        assert!(
            super::reference_same_surface_pair(
                &source,
                &target,
                &mut pair_context,
                &mut pair_progress,
            )
            .unwrap()
        );
        assert_eq!(window.close().allocations, 0);
        assert_eq!(pair_context.workspace_peak(), 0);
        let mut context = MetricContext::new(target, adequate).unwrap();
        let mut progress = WorkProgress::integer(None);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        assert!(super::reference_matches(&source, &mut context, &mut progress).unwrap());
        assert_eq!(window.close().allocations, 0);
        assert!(context.work_items() > 1);
        assert_eq!(context.workspace_peak(), 0);
        let different = source.clone().with_axes(AxisOrder::LatLon);
        assert!(!super::reference_matches(&different, &mut context, &mut progress).unwrap());
        assert!(super::reference_same_surface(&different, &mut context, &mut progress).unwrap());
        assert!(
            super::reference_same_surface_pair(
                &source,
                &different,
                &mut pair_context,
                &mut pair_progress,
            )
            .unwrap()
        );
        let different = GeographicReference::new(
            source.ellipsoid().clone(),
            purrdf_hash::hex::Digest32::new([104; 32]),
            AxisOrder::LonLat,
        );
        assert!(!super::reference_same_surface(&different, &mut context, &mut progress).unwrap());
        assert!(
            !super::reference_same_surface_pair(
                &source,
                &different,
                &mut pair_context,
                &mut pair_progress,
            )
            .unwrap()
        );
    }

    #[test]
    fn retained_exact_owners_preflight_work_and_complete_live_overlap() {
        use crate::{ExecutionLimits, ExecutionPolicy, GeoError, GeographicReference};
        use purrdf_xsd::integer::ExactOperation;
        struct NoBody;
        impl crate::MetricWorkObserver for NoBody {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                panic!("refused original operation must not open an observed phase")
            }
        }
        let value = Rat::from_int(Int::one().shl(4096).add(&Int::one()));
        let cost = super::rational_cost(ExactOperation::Linear, &[&value], 1).unwrap();
        for memory in [false, true] {
            for returned_rat in [false, true] {
                let limits = ExecutionLimits {
                    max_work_items: if memory { 1_000_000_000 } else { 1 },
                    // One result reservation fits; the simultaneous result
                    // and arithmetic allowance deliberately does not.
                    max_workspace_bytes: if memory {
                        2 * cost.workspace_bytes - 1
                    } else {
                        64 * 1024 * 1024
                    },
                    ..ExecutionLimits::GEOMETRY
                };
                let mut context = MetricContext::new(
                    GeographicReference::wgs84(),
                    ExecutionPolicy::new(limits).unwrap(),
                )
                .unwrap();
                context.begin_integer(1).unwrap();
                let mut observer = NoBody;
                let mut progress = WorkProgress::integer(Some(&mut observer));
                let mut retained = 0;
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let mut admission = super::ExactAdmission::new(&mut context, &mut progress);
                let result = if returned_rat {
                    admission
                        .rational_retained(ExactOperation::Linear, &[&value], &mut retained, |_| {
                            panic!("refused original copy must not execute")
                        })
                        .map(|_| ())
                } else {
                    admission.rational_owner_counted(
                        ExactOperation::Linear,
                        &[&value],
                        1,
                        &mut retained,
                        |_| panic!("refused original owner must not execute"),
                    )
                };
                let measured = window.close();
                assert!(if memory {
                    matches!(result, Err(GeoError::MemoryExhausted { limit }) if limit == limits.max_workspace_bytes)
                } else {
                    matches!(result, Err(GeoError::WorkExhausted { limit: 1 }))
                });
                assert_eq!(measured.allocations, 0);
                assert_eq!(measured.requested_bytes, 0);
                assert_eq!(retained, 0);
                assert_eq!(context.current_workspace_bytes(), 0);
                assert_eq!(context.workspace_peak(), 0);
                assert_eq!(context.work_items(), 0);
            }
        }
    }

    #[test]
    fn returned_rationals_keep_only_live_native_storage_after_arithmetic() {
        use purrdf_xsd::integer::ExactOperation;
        let wide = Rat::from_int(Int::one().shl(4096).add(&Int::one()));
        let mut context = MetricContext::wgs84().unwrap();
        context.prepare_integer_scratch_for(4096).unwrap();
        let baseline = context.current_workspace_bytes();
        let mut progress = WorkProgress::integer(None);
        let mut retained = 0;
        let mut admission = super::ExactAdmission::new(&mut context, &mut progress);
        let (inline, bytes) = admission
            .rational_retained(
                ExactOperation::Linear,
                &[&Rat::one()],
                &mut retained,
                |values| values.original(&Rat::one()),
            )
            .unwrap();
        assert_eq!(inline, Rat::one());
        assert_eq!(bytes, 0);
        assert_eq!(retained, 0);
        drop(inline);
        let (value, bytes) = admission
            .rational_retained(ExactOperation::Linear, &[&wide], &mut retained, |values| {
                values.original(&wide)
            })
            .unwrap();
        assert_eq!(value, wide);
        assert!(bytes > 0);
        assert_eq!(retained, bytes);
        let clone = value.clone();
        drop(value);
        // Immutable clones share the original allocation, while no returned
        // value pins a reusable arithmetic destination or its temporary fee.
        assert_eq!(context.current_workspace_bytes(), baseline + bytes);
        let scratch = context.integer_scratch().unwrap();
        assert_eq!(scratch.available(), scratch.destination_capacity());
        drop(context);
        assert_eq!(clone, wide);
    }

    #[test]
    fn prepared_order_keys_reuse_destinations_and_exact_ties() {
        let denominator = Int::one().shl(448).add(&Int::from_i64(11));
        let left = Rat::new(denominator.sub(&Int::from_i64(4)), denominator.clone()).unwrap();
        let right = Rat::new(denominator.sub(&Int::from_i64(3)), denominator).unwrap();
        let expected = [&left, &right].map(|value| {
            purrdf_xsd::ieee::ratio::binary64_order_key(
                value.numerator().as_ref(),
                value.denominator().as_ref(),
            )
            .unwrap()
        });
        assert_eq!(expected[0], expected[1], "the exact neighbours share a key");
        assert!(left < right);
        let mut context = MetricContext::wgs84().unwrap();
        context.prepare_integer_scratch_for(512).unwrap();
        context.begin_integer(1).unwrap();
        context.admit_workspace(17).unwrap();
        let live = context.current_workspace_bytes();
        let mut progress = WorkProgress::integer(None);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for _ in 0..16 {
            let mut admission = super::ExactAdmission::new(&mut context, &mut progress);
            assert_eq!(admission.order_key(&left).unwrap(), expected[0]);
            assert_eq!(admission.order_key(&right).unwrap(), expected[1]);
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(context.current_workspace_bytes(), live);
        let scratch = context.integer_scratch().unwrap();
        assert_eq!(scratch.available(), scratch.destination_capacity());
        let mut admission = super::ExactAdmission::new(&mut context, &mut progress);
        assert!(
            admission
                .compare_ordered((&left, expected[0]), (&right, expected[1]))
                .unwrap()
                .is_lt()
        );
        assert!(
            admission
                .compare_ordered((&right, expected[1]), (&left, expected[0]))
                .unwrap()
                .is_gt()
        );
        assert_eq!(context.current_workspace_bytes(), live);
        context.release_workspace(17).unwrap();
    }

    #[test]
    fn retained_order_key_destination_refuses_without_heap_fallback() {
        let value = Rat::new(Int::one().shl(448).add(&Int::one()), Int::from_i64(7)).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        context
            .prepare_integer_scratch_with_capacity(512, 1)
            .unwrap();
        context.begin_integer(1).unwrap();
        context.admit_workspace(17).unwrap();
        let live = context.current_workspace_bytes();
        let held = Int::one()
            .shl_in(448, context.integer_scratch().unwrap())
            .unwrap();
        assert_eq!(context.integer_scratch().unwrap().available(), 0);
        let mut progress = WorkProgress::integer(None);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = super::ExactAdmission::new(&mut context, &mut progress).order_key(&value);
        assert_eq!(window.close().allocations, 0);
        assert_eq!(
            result,
            Err(crate::GeoError::NumericalScratch(
                purrdf_xsd::integer::LimbScratchError::Exhausted { destinations: 1 },
            )),
        );
        assert_eq!(context.current_workspace_bytes(), live);
        assert_eq!(context.integer_scratch().unwrap().available(), 0);
        drop(held);
        assert_eq!(context.integer_scratch().unwrap().available(), 1);
        context.release_workspace(17).unwrap();
    }

    #[test]
    fn admitted_order_charges_only_original_branches_and_preserves_first_refusal() {
        let left = Rat::parse_decimal("12.5").unwrap();
        let right = Rat::parse_decimal("13.5").unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::integer(None);
        assert_eq!(
            super::compare_rat(&left, &right, &mut context, &mut progress).unwrap(),
            left.cmp(&right)
        );
        // One complete source/sign scan, then one borrowed numerator scan.
        assert_eq!(context.work_items(), 32);
        assert_eq!(context.current_workspace_bytes(), 0);
        let negative = right.neg();
        assert!(
            super::compare_rat(&left, &negative, &mut context, &mut progress)
                .unwrap()
                .is_gt()
        );
        assert_eq!(context.work_items(), 48);

        struct StopBeforeProducts {
            calls: u64,
        }
        impl crate::MetricWorkObserver for StopBeforeProducts {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), crate::GeoError> {
                self.calls += 1;
                if self.calls == 3 {
                    Err(crate::GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        let denominator = Int::one().shl(448);
        let a = Rat::new(Int::from_i64(7), denominator.clone()).unwrap();
        let b = Rat::new(Int::from_i64(9), denominator.add(&Int::one())).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let mut observer = StopBeforeProducts { calls: 0 };
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        {
            let mut progress = WorkProgress::integer(Some(&mut observer));
            assert_eq!(
                super::compare_rat(&a, &b, &mut context, &mut progress),
                Err(crate::GeoError::Cancelled)
            );
            assert_eq!(progress.latched_error(), Some(crate::GeoError::Cancelled));
            assert_eq!(
                super::compare_rat(&a, &b, &mut context, &mut progress),
                Err(crate::GeoError::Cancelled)
            );
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(observer.calls, 3);
        assert_eq!(context.current_workspace_bytes(), 0);
    }

    #[test]
    fn numerical_output_transfers_only_its_preadmitted_live_owner() {
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(None);
        with_math_for_sources(&mut context, &mut progress, 256, 0, &[], |_, _| Ok(())).unwrap();
        let baseline = context.remaining_workspace();
        let (owner, retained) = super::with_math_for_sources_retained(
            &mut context,
            &mut progress,
            256,
            4096,
            &[],
            |math, progress| {
                let value = FixedInterval::from_i64(1, math).unwrap();
                let bytes = value.detached_heap_bytes();
                math.reserve_workspace(bytes).unwrap();
                progress.math_poll(math).unwrap();
                Ok((value.detached(), bytes as u64))
            },
        )
        .unwrap();
        assert!(retained > 0);
        assert_eq!(context.remaining_workspace(), baseline - retained);
        assert!(owner.lower() > &Int::zero());
        drop(owner);
        context.release_workspace(retained).unwrap();
        assert_eq!(context.remaining_workspace(), baseline);
        let error = super::with_math_for_sources_retained(
            &mut context,
            &mut progress,
            256,
            0,
            &[],
            |_, _| Ok((String::from("a forged transfer is refused"), u64::MAX)),
        )
        .unwrap_err();
        assert!(matches!(error, crate::GeoError::ArithmeticOverflow(_)));
        assert_eq!(context.remaining_workspace(), baseline);
    }

    #[test]
    fn original_source_width_prepares_scratch_before_fixed_conversion() {
        let original = Rat::new(Int::one(), Int::one().shl(10_000).add(&Int::one())).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(None);
        let policy = context.policy();
        let enclosure = with_math_for_sources(
            &mut context,
            &mut progress,
            112,
            0,
            &[&original],
            |math, _| {
                assert!(math.limb_scratch().unwrap().limb_capacity() > 10_000 / 64);
                fixed_from_rat(&original, math).map_err(|error| geo_math_error(&error, policy))
            },
        )
        .unwrap();
        // These tiny dyadic endpoints are inline. Original-source admission
        // happened before conversion, but no returned value retains the arena.
        assert!(context.integer_scratch().is_none());
        let (lower, upper) = exact_bounds(&enclosure);
        assert!(lower <= original && upper >= original);
    }

    #[test]
    fn numerical_phases_retain_taylor_storage_once_and_children_grow_independently() {
        fn phase(context: &mut MetricContext, live_jets: usize) {
            let mut progress = WorkProgress::new(None);
            let policy = context.policy();
            with_math_for_sources(context, &mut progress, 112, 0, &[], |math, progress| {
                purrdf_xsd::math::with_taylor_workspace_observed_with_storage(
                    8,
                    live_jets,
                    math,
                    progress,
                    |math, progress| progress.math_poll(math),
                    WorkProgress::taylor_factory(),
                    |workspace, math, _| {
                        let constant =
                            workspace.constant(FixedInterval::from_i64(3, math)?, math)?;
                        assert!(workspace.coefficient(constant, 1)?.is_exact_zero());
                        Ok(())
                    },
                )
                .map_err(|error| geo_math_error(&error, policy))
            })
            .unwrap();
        }
        let mut parent = MetricContext::wgs84().unwrap();
        phase(&mut parent, 4);
        let original = parent.taylor_scratch().unwrap().clone();
        let baseline = parent.retained_workspace_bytes();
        phase(&mut parent, 4);
        assert_eq!(parent.retained_workspace_bytes(), baseline);
        assert!(parent.taylor_scratch().unwrap().shares_storage(&original));
        let mut child = parent.remaining_child().unwrap();
        assert!(child.taylor_scratch().unwrap().shares_storage(&original));
        phase(&mut child, 16);
        let replacement = child.taylor_scratch().unwrap();
        assert!(!replacement.shares_storage(&original));
        assert!(child.retained_workspace_bytes() >= replacement.retained_bytes() as u64);
        assert_eq!(parent.retained_workspace_bytes(), baseline);
        assert!(parent.taylor_scratch().unwrap().shares_storage(&original));
        assert!(child.set_retained_workspace(0).is_err());
    }

    #[test]
    fn prepared_meridians_bind_exact_axes_and_release_every_limb_destination() {
        let latitude = Rat::from_i64(40);
        let mut context = MetricContext::wgs84().unwrap();
        for ellipsoid in [
            crate::PreparedEllipsoid::wgs84(),
            crate::PreparedEllipsoid::cgcs2000(),
            crate::PreparedEllipsoid::new(
                Rat::from_i64(12_756_274),
                crate::PreparedEllipsoid::wgs84()
                    .inverse_flattening()
                    .clone(),
            )
            .unwrap(),
        ] {
            let mut progress = WorkProgress::new(None);
            let policy = context.policy();
            let expected =
                with_math_for_sources(&mut context, &mut progress, 112, 0, &[], |math, _| {
                    crate::geodesic::meridian_arc(&latitude, &ellipsoid, math)
                        .and_then(|value| value.round_decimal(6, math))
                        .map_err(|error| geo_math_error(&error, policy))
                })
                .unwrap();
            // Operation inputs remain admitted while a numerical phase may
            // first prepare these tables. Promotion must preserve that scope.
            context.admit_workspace(17).unwrap();
            let measured = with_math_for_sources(
                &mut context,
                &mut progress,
                112,
                0,
                &[],
                |math, progress| {
                    crate::geodesic::meridian_arc_observed(&latitude, &ellipsoid, math, progress)
                        .and_then(|value| value.round_decimal(6, math))
                        .map_err(|error| geo_math_error(&error, policy))
                },
            )
            .unwrap();
            assert_eq!(expected.0, expected.1);
            assert_eq!(measured, expected);
            context.release_workspace(17).unwrap();
            context.begin(0).unwrap();
            let retained = context.retained_workspace_bytes();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let warmed = with_math_for_sources(
                &mut context,
                &mut progress,
                112,
                0,
                &[],
                |math, progress| {
                    crate::geodesic::meridian_arc_observed(&latitude, &ellipsoid, math, progress)
                        .and_then(|value| value.round_decimal(6, math))
                        .map_err(|error| geo_math_error(&error, policy))
                },
            )
            .unwrap();
            assert_eq!(window.close().allocations, 0);
            assert_eq!(warmed, expected);
            assert_eq!(context.retained_workspace_bytes(), retained);
            let scratch = context.integer_scratch().unwrap();
            assert_eq!(scratch.available(), scratch.destination_capacity());
        }
    }
}
