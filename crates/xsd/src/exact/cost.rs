// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Resource governance: what an exact operation will cost, known before it runs.
//!
//! An unbounded number is a denial-of-service vector — `10^(10^9)` is a short
//! query and a gigabyte of digits — so every operation of the tower has a cost
//! method (`Integer::mul_cost`, `Decimal::div_cost`, …) returning a [`Cost`]
//! computed from the operands' sizes alone, in constant time, without touching
//! their digits. A governor charges the estimate and refuses the operation
//! before it allocates.
//!
//! # Units
//!
//! * [`Cost::work`] is an upper bound on binary `u64` limb operations (one
//!   multiply-add, compare or division step). Decimal parsing and rendering
//!   include their quadratic base conversion; arithmetic uses the operand lengths
//!   for multiplication (Karatsuba stays below it from
//!   [`crate::bigint::KARATSUBA_THRESHOLD`] limbs up); quotient length times
//!   divisor length for division; quadratic in the result for powers.
//! * [`Cost::bytes`] is an upper bound on the heap bytes of the value the
//!   operation produces — the memory it mints and hands back.
//!
//! Both are deterministic functions of the operand sizes, so the same query
//! over the same data charges the same amount on every run and every target.

/// The estimated cost of one exact operation; see the module docs for the units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Cost {
    work: u64,
    bytes: u64,
}

impl Cost {
    /// No cost.
    pub const ZERO: Self = Self { work: 0, bytes: 0 };

    /// A cost of `work` limb operations producing `bytes` heap bytes.
    #[must_use]
    pub const fn new(work: u64, bytes: u64) -> Self {
        Self { work, bytes }
    }

    /// The upper bound on limb operations.
    #[must_use]
    pub const fn work(self) -> u64 {
        self.work
    }

    /// The upper bound on heap bytes the result holds.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.bytes
    }

    /// The cost of doing both, saturating at `u64::MAX`.
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self {
            work: self.work.saturating_add(other.work),
            bytes: self.bytes.saturating_add(other.bytes),
        }
    }

    /// The cost of doing `self` and then `next`, one after the other: the work adds,
    /// and the bytes are the larger working set, since the first's is released
    /// before the second's is needed.
    #[must_use]
    pub const fn then(self, next: Self) -> Self {
        Self {
            work: self.work.saturating_add(next.work),
            bytes: if self.bytes > next.bytes {
                self.bytes
            } else {
                next.bytes
            },
        }
    }

    /// The larger of the two in each unit.
    #[must_use]
    pub const fn max(self, other: Self) -> Self {
        Self {
            work: if self.work > other.work {
                self.work
            } else {
                other.work
            },
            bytes: if self.bytes > other.bytes {
                self.bytes
            } else {
                other.bytes
            },
        }
    }
}

/// Which integer kernel a checked physical destination layout describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerOperation {
    /// Sum of two signed integers.
    Add,
    /// Difference of two signed integers.
    Subtract,
    /// Product of two signed integers.
    Multiply,
}

/// Temporary/result native heap bound, separate from saturating governor fuel.
#[derive(Debug, Clone, Copy)]
pub struct NumericOperationLayout {
    peak_bytes: usize,
    retained_bytes: usize,
}

impl NumericOperationLayout {
    /// A proven machine/inline operation with no heap destination.
    pub const INLINE: Self = Self {
        peak_bytes: 0,
        retained_bytes: 0,
    };
    /// Peak new bytes while the already-admitted inputs remain live.
    #[must_use]
    pub const fn required_bytes(self) -> usize {
        self.peak_bytes
    }
    /// Upper bound of surviving native destination capacity.
    #[must_use]
    pub const fn retained_bytes(self) -> usize {
        self.retained_bytes
    }

    pub(crate) fn bytes(
        peak_bytes: usize,
        retained_bytes: usize,
    ) -> Result<Self, crate::bigint::LimbScratchError> {
        use crate::bigint::LimbScratchError::SizeOverflow;
        std::alloc::Layout::array::<u8>(peak_bytes).map_err(|_| SizeOverflow)?;
        std::alloc::Layout::array::<u8>(retained_bytes).map_err(|_| SizeOverflow)?;
        Ok(Self {
            peak_bytes,
            retained_bytes,
        })
    }

    /// Include surviving new intermediates around a fresh destination kernel.
    /// Already-admitted borrowed input owners are excluded by the caller.
    /// # Errors
    /// Refuses checked byte/address-space overflow before allocation.
    pub fn with_live(self, live_bytes: usize) -> Result<Self, crate::bigint::LimbScratchError> {
        let peak = live_bytes
            .checked_add(self.peak_bytes)
            .ok_or(crate::bigint::LimbScratchError::SizeOverflow)?;
        Self::bytes(peak, self.retained_bytes)
    }

    pub(crate) fn words(peak: u64, retained: u64) -> Result<Self, crate::bigint::LimbScratchError> {
        use crate::bigint::LimbScratchError::SizeOverflow;
        let checked = |words: u64| -> Result<usize, crate::bigint::LimbScratchError> {
            let words = usize::try_from(words).map_err(|_| SizeOverflow)?;
            Ok(std::alloc::Layout::array::<u64>(words)
                .map_err(|_| SizeOverflow)?
                .size())
        };
        Ok(Self {
            peak_bytes: checked(peak)?,
            retained_bytes: checked(retained)?,
        })
    }
}

/// Checked physical counterpart for the existing integer destination kernels.
/// This does not treat a governor Cost::ZERO as an allocation certificate.
///
/// # Errors
/// Refuses checked length/byte overflow before creating any native destination.
pub fn integer_operation_layout(
    la: u64,
    lb: u64,
    negative_a: bool,
    negative_b: bool,
    op: IntegerOperation,
) -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    match op {
        IntegerOperation::Add | IntegerOperation::Subtract => {
            let add_magnitudes =
                (negative_a == negative_b) ^ matches!(op, IntegerOperation::Subtract);
            let capacity = la
                .max(lb)
                .checked_add(u64::from(add_magnitudes))
                .ok_or(SizeOverflow)?;
            // These kernels initialize the complete destination; four words
            // really allocate even if canonical trimming later returns inline.
            let heap = if capacity <= 3 { 0 } else { capacity };
            NumericOperationLayout::words(heap, heap)
        }
        IntegerOperation::Multiply => {
            if la == 0 || lb == 0 || (la <= 1 && lb <= 1) {
                return Ok(NumericOperationLayout::INLINE);
            }
            let result = la.checked_add(lb).ok_or(SizeOverflow)?;
            if result <= 3 {
                return Ok(NumericOperationLayout::INLINE);
            }
            if la.min(lb) < crate::bigint::KARATSUBA_THRESHOLD as u64 {
                return NumericOperationLayout::words(result, result);
            }
            // Native Karatsuba's established eight-result working law. The
            // fallible shared body retains exact-capacity destinations; the
            // original numerical recursion and threshold are unchanged.
            let peak = result.checked_mul(8).ok_or(SizeOverflow)?;
            // Recombination adds one explicit carry word to the output.
            let retained = result.checked_add(1).ok_or(SizeOverflow)?;
            NumericOperationLayout::words(peak, retained)
        }
    }
}

pub(crate) fn live_integer_bytes(
    values: &[&super::Integer],
) -> Result<usize, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    values.iter().try_fold(0_usize, |sum, value| {
        sum.checked_add(usize::try_from(value.heap_bytes()).map_err(|_| SizeOverflow)?)
            .ok_or(SizeOverflow)
    })
}

fn checked_sum_words(values: &[u64]) -> Result<u64, crate::bigint::LimbScratchError> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or(crate::bigint::LimbScratchError::SizeOverflow)
    })
}

fn owned_words(limbs: u64) -> u64 {
    if limbs <= 3 { 0 } else { limbs }
}

// Mirrors the actual destination families of mag_div_rem, not saturated fuel.
// The <=8-limb normalized branch uses stack scratch; larger division retains
// normalized numerator/divisor and quotient before returning its remainder.
fn division_words(la: u64, lb: u64) -> Result<u64, crate::bigint::LimbScratchError> {
    if la <= 8 {
        checked_sum_words(&[owned_words(la), owned_words(la.min(lb))])
    } else {
        checked_sum_words(&[
            owned_words(
                la.checked_add(1)
                    .ok_or(crate::bigint::LimbScratchError::SizeOverflow)?,
            ),
            owned_words(lb),
            owned_words(la),
        ])
    }
}

/// Complete destination family of the existing quotient/remainder kernel.
/// Inputs are borrowed and already owned by the calling frame.
/// # Errors
/// Refuses machine byte overflow before any normalized destination allocation.
pub fn integer_division_layout(
    numerator_limbs: u64,
    denominator_limbs: u64,
) -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
    let destinations = division_words(numerator_limbs, denominator_limbs)?;
    NumericOperationLayout::words(destinations, destinations)
}

/// Initial exact-division storage before the actual reduced factor/exponent
/// is known. Final coefficient and scale growth are admitted separately.
pub(crate) fn decimal_exact_initial_layout(
    la: u64,
    lb: u64,
) -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
    let (a, b, g) = (owned_words(la), owned_words(lb), owned_words(la.min(lb)));
    let gcd = checked_sum_words(&[a, b, a, b, g])?;
    let reduce_a = checked_sum_words(&[a, b, g, division_words(la, la.min(lb))?])?;
    let reduce_b = checked_sum_words(&[a, b, g, a, division_words(lb, la.min(lb))?])?;
    let factor = checked_sum_words(&[a, b, b])?;
    let peak = gcd.max(reduce_a).max(reduce_b).max(factor);
    NumericOperationLayout::words(peak, peak)
}

/// Actual original/shifted coefficient overlap for a native binary shift.
pub(crate) fn binary_shift_layout(
    bits: u64,
    shift: u32,
    current_bytes: usize,
) -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    let result = bits
        .checked_add(u64::from(shift))
        .ok_or(SizeOverflow)?
        .div_ceil(64);
    let output = owned_words(result);
    let bytes = usize::try_from(output)
        .map_err(|_| SizeOverflow)?
        .checked_mul(size_of::<u64>())
        .ok_or(SizeOverflow)?;
    let peak = current_bytes.checked_add(bytes).ok_or(SizeOverflow)?;
    NumericOperationLayout::bytes(peak, bytes)
}

/// Pow5 uses the existing word-chunk multiply body. Its original operand and
/// both old/new exact-capacity destinations coexist; no exponent-size guess.
pub(crate) fn pow5_layout(
    bits: u64,
    exponent: u32,
    current_bytes: usize,
) -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    let added = (u128::from(log2_upper_q16(5)) * u128::from(exponent)).div_ceil(65_536);
    let bits = u128::from(bits).checked_add(added).ok_or(SizeOverflow)?;
    let limbs = u64::try_from(bits.div_ceil(64)).map_err(|_| SizeOverflow)?;
    let destination = limbs.checked_add(1).ok_or(SizeOverflow)?;
    let words = if limbs <= 3 { 0 } else { destination };
    let bytes = usize::try_from(words)
        .map_err(|_| SizeOverflow)?
        .checked_mul(size_of::<u64>())
        .ok_or(SizeOverflow)?;
    let peak = bytes
        .checked_mul(2)
        .and_then(|n| n.checked_add(current_bytes))
        .ok_or(SizeOverflow)?;
    NumericOperationLayout::bytes(peak, bytes)
}

/// Checked old/new destination overlap of the native word-chunk 10^digits body.
pub(crate) fn power_of_ten_layout(
    digits: u64,
) -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    if digits == 0 {
        return Ok(NumericOperationLayout::INLINE);
    }
    let limbs = limbs_for_digits(digits);
    let capacity = limbs.checked_add(1).ok_or(SizeOverflow)?;
    let heap = if limbs <= 3 { 0 } else { capacity };
    NumericOperationLayout::words(heap.checked_mul(2).ok_or(SizeOverflow)?, heap)
}

/// Native power-of-ten creation and borrowed-coefficient product. The caller
/// adds every still-live temporary outside this kernel before admitting it.
pub(crate) fn scale_up_layout(
    coefficient: &super::Integer,
    digits: u32,
) -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    if coefficient.is_zero() {
        return Ok(NumericOperationLayout::INLINE);
    }
    if digits == 0 {
        let bytes = usize::try_from(coefficient.heap_bytes()).map_err(|_| SizeOverflow)?;
        return NumericOperationLayout::bytes(bytes, bytes);
    }
    if let Some(value) = coefficient.as_i128()
        && let Some(power) = 10_i128.checked_pow(digits)
        && value.checked_mul(power).is_some()
    {
        return Ok(NumericOperationLayout::INLINE);
    }
    let la = coefficient.limb_len();
    let power = limbs_for_digits(u64::from(digits));
    let power_capacity = power.checked_add(1).ok_or(SizeOverflow)?;
    let power_words = if power <= 3 { 0 } else { power_capacity };
    let result = la.checked_add(power).ok_or(SizeOverflow)?;
    let result_words = owned_words(result);
    let power_peak = power_words.checked_mul(2).ok_or(SizeOverflow)?;
    let product_peak = checked_sum_words(&[power_words, result_words])?;
    NumericOperationLayout::words(power_peak.max(product_peak), result_words)
}

/// Native normalization's factor stripping and actual power/division families,
/// excluding the already-live coefficient capacity.
pub(crate) fn normalization_extra_words(
    limbs: u64,
) -> Result<u64, crate::bigint::LimbScratchError> {
    if limbs <= 3 {
        return Ok(0);
    }
    let l = owned_words(limbs);
    let power = limbs
        .checked_add(1)
        .ok_or(crate::bigint::LimbScratchError::SizeOverflow)?;
    let two_copies = checked_sum_words(&[l, l])?;
    let power_divide = checked_sum_words(&[owned_words(power), division_words(limbs, power)?])?;
    Ok(two_copies.max(power_divide))
}

pub(crate) fn normalize_layout(
    coefficient: &super::Integer,
    scale: u32,
) -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    let current = usize::try_from(coefficient.heap_bytes()).map_err(|_| SizeOverflow)?;
    if scale == 0 || coefficient.as_i128().is_some() {
        return NumericOperationLayout::bytes(current, current);
    }
    let extra = usize::try_from(normalization_extra_words(coefficient.limb_len())?)
        .map_err(|_| SizeOverflow)?
        .checked_mul(size_of::<u64>())
        .ok_or(SizeOverflow)?;
    NumericOperationLayout::bytes(current.checked_add(extra).ok_or(SizeOverflow)?, current)
}

/// Checked physical layout of native decimal group conversion and final lexical.
#[derive(Debug, Clone, Copy)]
pub struct NumericRenderLayout {
    temporary_bytes: usize,
    output_bytes: usize,
    group_capacity: usize,
}

impl NumericRenderLayout {
    /// All live quotient/group storage while converting; source is already owned.
    #[must_use]
    pub const fn temporary_bytes(self) -> usize {
        self.temporary_bytes
    }
    /// Complete sign/point/padding output capacity, before output allocation.
    #[must_use]
    pub const fn output_bytes(self) -> usize {
        self.output_bytes
    }
    /// Requested native group buffer capacity.
    #[must_use]
    pub const fn group_capacity(self) -> usize {
        self.group_capacity
    }
    /// Peak new conversion temporaries and output while input owners stay live.
    /// # Errors
    /// Refuses checked composition overflow.
    pub fn required_bytes(self) -> Result<usize, crate::bigint::LimbScratchError> {
        self.temporary_bytes
            .checked_add(self.output_bytes)
            .ok_or(crate::bigint::LimbScratchError::SizeOverflow)
    }
}

/// Native checked layout for the shared 10^19 group renderer.
/// # Errors
/// Refuses byte/address-space overflow before any parser or render allocation.
pub fn numeric_render_layout(
    bits: u64,
    limbs: usize,
    scale: u32,
    negative: bool,
) -> Result<NumericRenderLayout, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    let digits = usize::try_from(digits_for_bits(bits)).map_err(|_| SizeOverflow)?;
    let groups = digits.div_ceil(19);
    let group_words = if groups <= 3 { 0 } else { groups };
    let quotient_words = if limbs <= 3 { 0 } else { limbs };
    // Native division retains old/new quotient destinations beside groups.
    let words = quotient_words
        .checked_mul(2)
        .and_then(|n| n.checked_add(group_words))
        .ok_or(SizeOverflow)?;
    let temporary_bytes = std::alloc::Layout::array::<u64>(words)
        .map_err(|_| SizeOverflow)?
        .size();
    let scale = usize::try_from(scale).map_err(|_| SizeOverflow)?;
    let body = if scale == 0 {
        digits
    } else if digits > scale {
        digits.checked_add(1).ok_or(SizeOverflow)?
    } else {
        scale.checked_add(2).ok_or(SizeOverflow)?
    };
    let output_bytes = body
        .checked_add(usize::from(negative))
        .ok_or(SizeOverflow)?;
    std::alloc::Layout::array::<u8>(output_bytes).map_err(|_| SizeOverflow)?;
    Ok(NumericRenderLayout {
        temporary_bytes,
        output_bytes,
        group_capacity: groups,
    })
}

/// Checked physical layout of the existing fallible native numeric parse.
/// This is separate from governor fuel and never rounds an overflow down.
#[derive(Debug, Clone, Copy)]
pub struct NumericParseLayout {
    peak_bytes: usize,
    retained_bytes: usize,
    invalid: bool,
    error_code: Option<crate::ErrorCode>,
}

impl NumericParseLayout {
    /// Temporary and resulting heap that can coexist during parsing.
    #[must_use]
    pub const fn required_bytes(self) -> usize {
        self.peak_bytes
    }
    /// Maximum retained magnitude payload, excluding the consumer's header.
    #[must_use]
    pub const fn retained_bytes(self) -> usize {
        self.retained_bytes
    }
    /// A lexical/value-space refusal is already known, without owned text.
    #[must_use]
    pub const fn is_invalid(self) -> bool {
        self.invalid
    }
    /// The known F&O code, if this layout represents a lexical refusal.
    #[must_use]
    pub const fn error_code(self) -> Option<crate::ErrorCode> {
        self.error_code
    }
}

fn inline_parse_layout() -> NumericParseLayout {
    NumericParseLayout {
        peak_bytes: 0,
        retained_bytes: 0,
        invalid: false,
        error_code: None,
    }
}

fn invalid_parse_layout(error_code: Option<crate::ErrorCode>) -> NumericParseLayout {
    NumericParseLayout {
        peak_bytes: 0,
        retained_bytes: 0,
        invalid: true,
        error_code,
    }
}

/// The two native decimal-chunk destinations. Each asks for previous len + 1;
/// original and replacement buffers coexist. Three-limb magnitudes stay inline.
fn magnitude_parse_layout(
    digits: usize,
) -> Result<NumericParseLayout, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    let digits = u64::try_from(digits).map_err(|_| SizeOverflow)?;
    let limbs = limbs_for_digits(digits);
    if limbs <= 3 {
        return Ok(inline_parse_layout());
    }
    let words = usize::try_from(limbs)
        .map_err(|_| SizeOverflow)?
        .checked_add(1)
        .ok_or(SizeOverflow)?;
    let retained_bytes = words.checked_mul(size_of::<u64>()).ok_or(SizeOverflow)?;
    let peak_bytes = retained_bytes
        .checked_add(retained_bytes)
        .ok_or(SizeOverflow)?;
    Ok(NumericParseLayout {
        peak_bytes,
        retained_bytes,
        invalid: false,
        error_code: None,
    })
}

/// Physical storage for the fallible native numeric reader
/// [`crate::value::try_parse_numeric`]. Invalid classification is part of this
/// plan; the caller must not invoke an owned-error parser for an invalid plan.
/// Returns None for another datatype, whose native layout remains required.
///
/// # Errors
/// Refuses checked layout overflow before the native parser allocates.
pub fn numeric_parse_layout(
    lexical: &str,
    datatype: crate::XsdDatatype,
    xsd10: bool,
) -> Result<Option<NumericParseLayout>, crate::bigint::LimbScratchError> {
    use crate::bigint::LimbScratchError::SizeOverflow;
    use crate::numeric::NumericReadError;
    use crate::{ErrorCode, XsdDatatype as D};
    if datatype.is_integer_family() {
        match crate::numeric::read_integer_typed(lexical, datatype) {
            Ok(_) => return Ok(Some(inline_parse_layout())),
            Err(NumericReadError::Invalid(_)) => {
                return Ok(Some(invalid_parse_layout(Some(ErrorCode::Forg0001))));
            }
            Err(NumericReadError::OutOfRange(reason)) => {
                if lexical.parse::<i128>().is_ok() {
                    return Ok(Some(invalid_parse_layout(crate::value::reason::classify(
                        datatype, reason,
                    ))));
                }
            }
        }
        let negative = lexical.starts_with('-');
        let accepted = match datatype {
            D::Integer => true,
            D::NonNegativeInteger | D::PositiveInteger => !negative,
            D::NonPositiveInteger | D::NegativeInteger => negative,
            _ => false,
        };
        if !accepted {
            return Ok(Some(invalid_parse_layout(crate::value::reason::classify(
                datatype,
                crate::value::reason::OUTSIDE_DATATYPE,
            ))));
        }
        let digits = lexical
            .strip_prefix(['+', '-'])
            .unwrap_or(lexical)
            .trim_start_matches('0')
            .len();
        return magnitude_parse_layout(digits).map(Some);
    }
    match datatype {
        D::Decimal => {
            match crate::numeric::read_decimal(lexical) {
                Ok(_) => return Ok(Some(inline_parse_layout())),
                Err(NumericReadError::Invalid(_)) => {
                    return Ok(Some(invalid_parse_layout(Some(ErrorCode::Forg0001))));
                }
                Err(NumericReadError::OutOfRange(_)) => {}
            }
            let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
            let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
            if u32::try_from(fraction.len()).is_err() {
                return Ok(Some(invalid_parse_layout(Some(ErrorCode::Foar0002))));
            }
            let whole = whole.trim_start_matches('0');
            let fraction = fraction.trim_end_matches('0');
            let digits = if whole.is_empty() {
                fraction.trim_start_matches('0').len()
            } else {
                whole
                    .len()
                    .checked_add(fraction.len())
                    .ok_or(SizeOverflow)?
            };
            magnitude_parse_layout(digits).map(Some)
        }
        D::Float | D::Double => {
            let special =
                matches!(lexical, "INF" | "-INF" | "NaN") || (!xsd10 && lexical == "+INF");
            let letters = lexical
                .bytes()
                .any(|byte| byte.is_ascii_alphabetic() && byte != b'e' && byte != b'E');
            let valid = special
                || (!letters
                    && match datatype {
                        D::Float => lexical.parse::<f32>().is_ok(),
                        _ => lexical.parse::<f64>().is_ok(),
                    });
            Ok(Some(if valid {
                inline_parse_layout()
            } else {
                invalid_parse_layout(Some(ErrorCode::Forg0001))
            }))
        }
        _ => Ok(None),
    }
}

/// The cost of parsing a lexical form of `len` bytes into any exact value.
#[must_use]
pub const fn parse(len: u64) -> Cost {
    let limbs = limbs_for_digits(len);
    Cost::new(
        len.saturating_mul(limbs.saturating_add(1))
            .saturating_add(1),
        limb_bytes(limbs).saturating_mul(2),
    )
}

/// The cost of rendering a value of `limbs` limbs (plus `scale` placed fractional
/// digits for a decimal) as its canonical lexical form.
#[must_use]
pub const fn render(limbs: u64, scale: u64) -> Cost {
    let digits = limbs.saturating_mul(20).saturating_add(3);
    render_parts(limbs, digits, digits.saturating_add(scale))
}

// Display divides by 10^19, once per coefficient group, not once per
// output digit. Each pass initializes its quotient and computes division and
// remainder over at most `limbs` words. Linear terms cover the initial clone,
// trimming and amortized group growth; fractional padding only costs text.
const fn render_parts(limbs: u64, digits: u64, text: u64) -> Cost {
    let groups = digits.div_ceil(19);
    let work = groups
        .saturating_mul(limbs.saturating_mul(3).saturating_add(10))
        .saturating_add(limbs)
        // Decimal formatting reads coefficient text before writing the final
        // padded text. Each is bounded by the final rendered length.
        .saturating_add(text.saturating_mul(2))
        .saturating_add(1);
    // Coefficient/quotient copies and old/new group buffers can coexist.
    // Decimal formatting additionally holds its coefficient text, padded text
    // and destination string, including geometric spare string capacity.
    let words = limbs
        .saturating_mul(3)
        .saturating_add(groups.saturating_mul(3));
    Cost::new(
        work,
        limb_bytes(words).saturating_add(text.saturating_mul(6)),
    )
}

/// Heap bytes of `limbs` binary `u64` limbs.
#[must_use]
pub const fn limb_bytes(limbs: u64) -> u64 {
    limbs.saturating_mul(8)
}

/// Limbs holding `digits` decimal digits.
#[must_use]
pub(crate) const fn limbs_for_digits(digits: u64) -> u64 {
    // log2(10) < 3322/1000; u128 keeps even u64::MAX digits exact.
    ((digits as u128 * 3322).div_ceil(64_000)) as u64
}

/// A constant-time decimal digit upper bound from an exact binary bit length.
pub(crate) const fn digits_for_bits(bits: u64) -> u64 {
    if bits == 0 {
        1
    } else {
        // log10(2) < 301029995663981196/10^18; both products fit u128.
        ((bits as u128 * 301_029_995_663_981_196).div_ceil(1_000_000_000_000_000_000)) as u64
    }
}

/// A certified decimal digit lower bound from the same binary bit length.
pub(crate) const fn minimum_digits_for_bits(bits: u64) -> u64 {
    if bits == 0 {
        1
    } else {
        (((bits - 1) as u128 * 301_029_995_663_981_195) / 1_000_000_000_000_000_000) as u64 + 1
    }
}

/// `a ± b` over `la`- and `lb`-limb magnitudes.
#[must_use]
pub(crate) const fn add(la: u64, lb: u64) -> Cost {
    let result = if la > lb { la } else { lb }.saturating_add(1);
    Cost::new(result, limb_bytes(result))
}

/// `a × b`. Karatsuba's recursion holds its half-size sums and partial products
/// beside the result along one path of the recursion — a geometric series under
/// the top level's — so its working set is a constant multiple of the result's.
#[must_use]
pub(crate) const fn mul(la: u64, lb: u64) -> Cost {
    let result = la.saturating_add(lb);
    let work = la
        .saturating_mul(lb)
        .saturating_add(result.saturating_mul(8))
        .saturating_add(1);
    let shorter = if la < lb { la } else { lb };
    let working = if shorter >= crate::bigint::KARATSUBA_THRESHOLD as u64 {
        8
    } else {
        2
    };
    Cost::new(work, limb_bytes(result).saturating_mul(working))
}

/// `a ÷ b` with remainder.
#[must_use]
pub(crate) const fn div(la: u64, lb: u64) -> Cost {
    let quotient = la.saturating_sub(lb).saturating_add(1);
    let work = quotient
        .saturating_mul(lb.saturating_add(1))
        .saturating_mul(4)
        .saturating_add(la)
        .saturating_add(1);
    // Normalized division retains the dividend-sized remainder allocation
    // beside the quotient and normalized divisor, even after zero trimming.
    Cost::new(
        work,
        limb_bytes(
            la.saturating_mul(2)
                .saturating_add(lb.saturating_mul(2))
                .saturating_add(4),
        ),
    )
}

/// The exact number of bits in a binary limb, in Q16 fixed point.
pub(crate) const LOG2_LIMB_Q16_UP: u64 = 64 * 65_536;
const LOG2_LIMB_Q16_DOWN: u64 = LOG2_LIMB_Q16_UP;

/// An upper bound on `log2(m)`, `m ≥ 1`, in Q16 fixed point, at most two
/// units (`2^-15`) above the true logarithm, so a power's size estimate is
/// within a few bits of the result even at exponents in the millions.
///
/// The integer part is the leading bit; the sixteen fractional bits are read off
/// by repeated squaring of the normalized significand `x ∈ [1, 2)` (each square
/// that reaches 2 is a one bit, halved back), in Q62 fixed point rounded up at
/// every step, so the computed `x` never falls below the true one and the bits
/// never undershoot. One unit on top covers the truncation of the sixteenth bit.
pub(crate) const fn log2_upper_q16(m: u128) -> u64 {
    const ONE: u128 = 1 << 62;
    if m <= 1 {
        return 0;
    }
    let lead = m.ilog2();
    let mut x = if lead <= 62 {
        m << (62 - lead)
    } else {
        let shift = lead - 62;
        (m >> shift) + if m & ((1 << shift) - 1) != 0 { 1 } else { 0 }
    };
    let mut fraction = 0_u64;
    let mut bit = 0;
    while bit < 16 {
        // x < 2^63 + 2^-k slack, so x² < 2^127: inside u128.
        let square = x * x;
        x = (square >> 62) + if square & (ONE - 1) != 0 { 1 } else { 0 };
        fraction <<= 1;
        if x >= 2 * ONE {
            fraction |= 1;
            x = (x >> 1) + (x & 1);
        }
        bit += 1;
    }
    (lead as u64) * 65_536 + fraction + 1
}

/// `a^exp` for an `a` with `log2|a| ≤ log2_q16 / 65536`: the result has at most
/// `exp × log2|a| / 64 + 1` limbs, and square-and-multiply does at most
/// two thirds of the square of that in limb products (the squarings form a
/// geometric series under the final one).
#[must_use]
pub(crate) const fn pow(log2_q16: u64, exp: u32) -> Cost {
    let bits = (log2_q16 as u128) * (exp as u128);
    let limbs = bits.div_ceil(LOG2_LIMB_Q16_DOWN as u128) + 1;
    let result = if limbs > u64::MAX as u128 {
        u64::MAX
    } else {
        limbs as u64
    };
    let work = result
        .saturating_mul(result)
        .saturating_add(result.saturating_mul(8))
        .saturating_add(1);
    Cost::new(work, limb_bytes(result))
}

/// `gcd(a, b)` by Euclid's algorithm.
#[must_use]
pub(crate) const fn gcd(la: u64, lb: u64) -> Cost {
    let longer = if la > lb { la } else { lb };
    let work = la
        .saturating_add(1)
        .saturating_mul(lb.saturating_add(1))
        .saturating_mul(8)
        .saturating_add(longer.saturating_mul(64));
    Cost::new(work, limb_bytes(longer.saturating_add(1)))
}

/// `a × 10^digits`, including forming the binary power of ten and multiplying.
#[must_use]
pub(crate) const fn shift10(la: u64, digits: u64) -> Cost {
    if digits == 0 {
        return Cost::new(la, limb_bytes(la));
    }
    let power = limbs_for_digits(digits);
    let result = la.saturating_add(power);
    Cost::new(
        power
            .saturating_mul(power)
            .saturating_add(la.saturating_mul(power))
            .saturating_add(result.saturating_mul(16))
            .saturating_add(1),
        limb_bytes(result).saturating_mul(10),
    )
}

/// The size of an `xsd:integer`/`xsd:decimal` value — its coefficient's length in
/// limbs and digits, its scale and its sign — which is all every estimate here
/// reads.
///
/// A shape is obtained in constant time from a value ([`Self::of_value`]), or in one
/// pass over a lexical form without parsing it ([`Self::of_lexical`]), and shapes
/// combine into upper bounds on the shapes of results ([`Self::sum`],
/// [`Self::product`], [`Self::quotient`]). So a governor can price a whole chain —
/// a `SUM` over a group, the moments of a `VARIANCE` — before running any of it,
/// by walking the shapes in the chain's order and adding up the step costs
/// ([`Self::add_cost`] and its siblings).
///
/// ```rust
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::exact::cost::Shape;
///
/// let tiny = Shape::of_lexical(&format!("0.{}1", "0".repeat(99_999)), XsdDatatype::Decimal)
///     .expect("a decimal lexical");
/// let one = Shape::of_lexical("1", XsdDatatype::Integer).expect("an integer lexical");
/// assert!(!tiny.is_bounded() && one.is_bounded());
/// // Adding them aligns the integer to 100 000 fractional digits, and the sum's text
/// // is as long.
/// let sum = tiny.sum(one);
/// assert!(tiny.add_cost(one).bytes() > 40_000);
/// assert!(sum.render_cost().bytes() > 100_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// The coefficient's size in binary `u64` limbs (zero for zero).
    pub(crate) limbs: u64,
    /// The coefficient's decimal digits (`1` for zero).
    pub(crate) digits: u64,
    /// A certified lower bound; generated result bounds use one.
    pub(crate) minimum_digits: u64,
    /// Whether `digits` is exact rather than a conservative upper bound.
    pub(crate) digits_exact: bool,
    /// The number of fractional digits (`0` for an integer).
    pub(crate) scale: u64,
    /// `-1`, `0` or `1` by the sign of the value.
    pub(crate) sign: i32,
}

impl Shape {
    /// The shape of `mantissa × 10^-scale`, an inline coefficient.
    pub(crate) const fn of_i128(mantissa: i128, scale: u64) -> Self {
        let magnitude = mantissa.unsigned_abs();
        let (limbs, digits) = if magnitude == 0 {
            (0, 1)
        } else {
            let digits = magnitude.ilog10() as u64 + 1;
            ((magnitude.ilog2() as u64 + 1).div_ceil(64), digits)
        };
        Self {
            limbs,
            digits,
            minimum_digits: digits,
            digits_exact: true,
            scale,
            sign: if mantissa < 0 {
                -1
            } else if mantissa > 0 {
                1
            } else {
                0
            },
        }
    }

    /// The shape of an `xsd:integer`/`xsd:decimal` value of any size, in constant
    /// time; `None` for every other value.
    #[must_use]
    pub fn of_value(value: &crate::XsdValue) -> Option<Self> {
        crate::numeric::exact_path::shape_of(value)
    }

    /// The shape of the value `lexical` denotes under `datatype`, an integer-family
    /// datatype or `xsd:decimal`, read in one pass without parsing it; `None` for any
    /// other datatype or a lexical form outside its lexical space (whose parse fails
    /// and so computes nothing).
    #[must_use]
    pub fn of_lexical(lexical: &str, datatype: crate::XsdDatatype) -> Option<Self> {
        let valid = if datatype == crate::XsdDatatype::Decimal {
            crate::numeric::is_decimal_lexical(lexical)
        } else {
            datatype.is_integer_family() && crate::numeric::is_integer_lexical(lexical)
        };
        if !valid {
            return None;
        }
        let negative = lexical.starts_with('-');
        let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
        let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
        let whole = whole.trim_start_matches('0');
        let fraction = fraction.trim_end_matches('0');
        let digits = if whole.is_empty() {
            fraction.trim_start_matches('0').len()
        } else {
            whole.len() + fraction.len()
        } as u64;
        if digits == 0 {
            return Some(Self::of_i128(0, 0));
        }
        Some(Self {
            limbs: limbs_for_digits(digits),
            digits,
            minimum_digits: digits,
            digits_exact: true,
            scale: fraction.len() as u64,
            sign: if negative { -1 } else { 1 },
        })
    }

    /// Whether a value of this shape fits the machine-word variants (an `i128`
    /// coefficient, at most eighteen fractional digits), where every operation is
    /// machine arithmetic and costs nothing here.
    #[must_use]
    pub const fn is_bounded(self) -> bool {
        self.digits <= 38 && self.scale <= 18
    }

    /// The coefficient's length in binary `u64` limbs.
    #[must_use]
    pub const fn limbs(self) -> u64 {
        self.limbs
    }

    /// The number of fractional digits.
    #[must_use]
    pub const fn scale(self) -> u64 {
        self.scale
    }

    /// An upper bound on the shape of `self ± other`: the larger scale, and one
    /// more integer digit than the longer integer part.
    #[must_use]
    pub const fn sum(self, other: Self) -> Self {
        let scale = if self.scale > other.scale {
            self.scale
        } else {
            other.scale
        };
        let whole_a = self.digits.saturating_sub(self.scale);
        let whole_b = other.digits.saturating_sub(other.scale);
        let whole = if whole_a > whole_b { whole_a } else { whole_b };
        Self::bound(whole.saturating_add(1), scale)
    }

    /// An upper bound on the shape of `self × other`: the scales and the digits add.
    #[must_use]
    pub const fn product(self, other: Self) -> Self {
        let digits = self.digits.saturating_add(other.digits);
        let scale = self.scale.saturating_add(other.scale);
        Self::bound(digits.saturating_sub(scale), scale)
    }

    /// An upper bound on the shape of `self ÷ other` under `policy`: the policy's
    /// scale (for [`super::DivisionPolicy::Exact`], the dividend's scale and four
    /// digits per divisor digit, which every terminating expansion fits in), and the
    /// dividend's integer digits plus the divisor's fractional ones.
    #[must_use]
    pub const fn quotient(self, other: Self, policy: super::DivisionPolicy) -> Self {
        let scale = match policy {
            super::DivisionPolicy::Scale { scale, .. } => scale as u64,
            super::DivisionPolicy::Exact => {
                self.scale.saturating_add(other.digits.saturating_mul(4))
            }
        };
        let whole = self
            .digits
            .saturating_sub(self.scale)
            .saturating_add(other.scale)
            .saturating_add(1);
        Self::bound(whole, scale)
    }

    /// The shape of a nonzero value with `whole` integer digits and `scale`
    /// fractional ones.
    const fn bound(whole: u64, scale: u64) -> Self {
        let digits = whole.saturating_add(scale);
        Self {
            limbs: limbs_for_digits(digits),
            digits,
            minimum_digits: 1,
            digits_exact: false,
            scale,
            sign: -1,
        }
    }

    /// The cost of `self + other` or `self − other`.
    #[must_use]
    pub const fn add_cost(self, other: Self) -> Cost {
        decimal_add(self, other)
    }

    /// The cost of `self × other`.
    #[must_use]
    pub const fn mul_cost(self, other: Self) -> Cost {
        decimal_mul(self, other)
    }

    /// The cost of `self ÷ other` under `policy`.
    #[must_use]
    pub const fn div_cost(self, other: Self, policy: super::DivisionPolicy) -> Cost {
        decimal_div(self, other, policy)
    }

    /// The comparison bound: certified separated leading positions are cheap;
    /// otherwise coefficient conversion and alignment are charged together.
    #[must_use]
    pub const fn cmp_cost(self, other: Self) -> Cost {
        decimal_cmp(self, other)
    }

    /// A coefficient/scale bound covering either operand of a running comparison.
    #[must_use]
    pub fn comparison_bound(self, other: Self) -> Self {
        Self::bound(
            self.digits
                .saturating_sub(self.scale)
                .max(other.digits.saturating_sub(other.scale)),
            self.scale.max(other.scale),
        )
    }

    /// The cost of rendering a value of this shape as its canonical lexical form.
    #[must_use]
    pub const fn render_cost(self) -> Cost {
        render_shape(self)
    }

    /// The cost of converting a value of this shape to the nearest `f64`/`f32`, or
    /// of comparing it exactly with one, whichever is dearer.
    #[must_use]
    pub const fn ieee_cost(self) -> Cost {
        decimal_to_float(self).max(decimal_cmp_f64(self))
    }

    /// The position of the leading digit: `|value| ∈ [10^(e−1), 10^e)` for the
    /// returned `e` (meaningless for zero).
    pub(crate) const fn magnitude_exponent(self) -> i64 {
        clamp_exponent(self.digits as i128 - self.scale as i128)
    }

    /// Certified leading-position bounds available without decimal conversion.
    /// Lexical digits bound every possible binary length before parsing; values
    /// use the bounds read from their actual binary length.
    pub(crate) const fn comparison_exponents(self) -> (i64, i64) {
        let (lower, upper) = if self.digits_exact && self.digits > 38 {
            const DENOMINATOR: u128 = 1_000_000_000_000_000_000;
            let low_bits = (self.digits.saturating_sub(1) as u128 * 3_321_928_094_887_362_347
                / DENOMINATOR)
                + 1;
            let high_bits = (self.digits as u128 * 3_321_928_094_887_362_348).div_ceil(DENOMINATOR);
            let low_digits = ((low_bits - 1) * 301_029_995_663_981_195 / DENOMINATOR) + 1;
            let high_digits = (high_bits * 301_029_995_663_981_196).div_ceil(DENOMINATOR);
            (
                if low_digits > u64::MAX as u128 {
                    u64::MAX
                } else {
                    low_digits as u64
                },
                if high_digits > u64::MAX as u128 {
                    u64::MAX
                } else {
                    high_digits as u64
                },
            )
        } else {
            (self.minimum_digits, self.digits)
        };
        (
            clamp_exponent(lower as i128 - self.scale as i128),
            clamp_exponent(upper as i128 - self.scale as i128),
        )
    }

    /// The decimal digits of the canonical lexical form, sign and point included:
    /// the bytes rendering it produces.
    pub(crate) const fn rendered_len(self) -> u64 {
        let body = if self.scale >= self.digits {
            // `0.` and the leading zeros.
            self.scale.saturating_add(2)
        } else if self.scale == 0 {
            self.digits
        } else {
            self.digits.saturating_add(1)
        };
        body.saturating_add(if self.sign < 0 { 1 } else { 0 })
    }
}

const fn clamp_exponent(value: i128) -> i64 {
    if value > i64::MAX as i128 {
        i64::MAX
    } else if value < i64::MIN as i128 {
        i64::MIN
    } else {
        value as i64
    }
}

/// Rendering a value of this shape as its canonical lexical form: one pass over
/// the coefficient's digits, then the bytes of the text (the leading zeros of a
/// small fraction included).
#[must_use]
pub(crate) const fn render_shape(value: Shape) -> Cost {
    render_parts(value.limbs, value.digits, value.rendered_len())
}

/// Reading decimal digits converts only the coefficient, without fractional padding.
pub(crate) const fn coefficient_conversion(value: Shape) -> Cost {
    if value.digits <= 38 {
        Cost::ZERO
    } else {
        render_shape(Shape { scale: 0, ..value })
    }
}

/// `a + b` / `a − b` over decimals: the coefficient with the smaller scale is
/// shifted up to the larger one first.
#[must_use]
pub(crate) const fn decimal_add(a: Shape, b: Shape) -> Cost {
    let gap = a.scale.abs_diff(b.scale);
    let (la, lb) = if a.scale < b.scale {
        (a.limbs.saturating_add(limbs_for_digits(gap)), b.limbs)
    } else {
        (a.limbs, b.limbs.saturating_add(limbs_for_digits(gap)))
    };
    let shifted = if a.scale < b.scale { a.limbs } else { b.limbs };
    shift10(shifted, gap).saturating_add(add(la, lb))
}

/// Comparing two decimals: decided by the signs or the leading-digit positions
/// alone unless both agree, and then by aligning the coefficients, whose scale gap
/// equals their digit-count gap and so never exceeds the longer coefficient.
#[must_use]
pub(crate) const fn decimal_cmp(a: Shape, b: Shape) -> Cost {
    if a.sign != b.sign || a.sign == 0 {
        return Cost::new(1, 0);
    }
    let (a_low, a_high) = a.comparison_exponents();
    let (b_low, b_high) = b.comparison_exponents();
    if a_low > b_high || b_low > a_high {
        return Cost::new(1, 0);
    }
    let conversion = coefficient_conversion(a).saturating_add(coefficient_conversion(b));
    if a.digits_exact && b.digits_exact && a.magnitude_exponent() != b.magnitude_exponent() {
        return conversion.saturating_add(Cost::new(1, 0));
    }
    // Binary values may need decimal conversion to resolve their leading
    // positions before the actual comparison can take its shortcut.
    conversion.saturating_add(decimal_add(a, b))
}

/// `a × b` over decimals: the coefficient product, then stripping the trailing
/// zeros the canonical form drops (one pass over the product).
#[must_use]
pub(crate) const fn decimal_mul(a: Shape, b: Shape) -> Cost {
    let product = a.limbs.saturating_add(b.limbs);
    mul(a.limbs, b.limbs).saturating_add(Cost::new(
        product
            .saturating_mul(product.saturating_add(1))
            .saturating_mul(64)
            .saturating_add(1),
        limb_bytes(product).saturating_mul(4),
    ))
}

/// Negation, the absolute value or a rounding to an integer: a few passes over the
/// coefficient, and a result no longer than it plus one limb.
#[must_use]
pub(crate) const fn decimal_unary(a: Shape) -> Cost {
    // The operand's copy, the kept digits, the discarded ones and the rounded
    // result can be live at once, with the canonical form's own copy.
    Cost::new(
        a.limbs
            .saturating_mul(a.limbs.saturating_add(1))
            .saturating_mul(64)
            .saturating_add(2),
        limb_bytes(a.limbs.saturating_add(1)).saturating_mul(6),
    )
}

/// The decimal exponents past which a value of a binary format is an infinity or
/// rounds to a signed zero, decided before any digit is touched: `|v| ≥ 10^(e−1)`
/// with `e ≥ overflow` is past the largest finite value, and `|v| < 10^e` with
/// `e ≤ underflow` is below half the smallest subnormal.
pub(crate) const F64_DECIMAL_EXPONENTS: (i64, i64) = (310, -324);
/// [`F64_DECIMAL_EXPONENTS`] for binary32.
pub(crate) const F32_DECIMAL_EXPONENTS: (i64, i64) = (40, -46);

/// Converting a decimal to the nearest `f64`/`f32`: free past the format's range
/// (an infinity or a signed zero, decided from the shape), otherwise forming
/// `10^scale` and one division whose quotient is at most five limbs.
#[must_use]
pub(crate) const fn decimal_to_float(a: Shape) -> Cost {
    let exponent = a.magnitude_exponent();
    if a.sign == 0
        || (a.digits_exact
            && (exponent >= F64_DECIMAL_EXPONENTS.0 || exponent <= F64_DECIMAL_EXPONENTS.1))
    {
        return Cost::new(1, 0);
    }
    let denominator = limbs_for_digits(a.scale);
    let la = a.limbs.saturating_add(5);
    shift10(0, a.scale).saturating_add(div(
        if la > denominator.saturating_add(5) {
            la
        } else {
            denominator.saturating_add(5)
        },
        denominator,
    ))
}

/// Comparing a decimal with a finite `f64` exactly: free when the signs or the
/// magnitudes' binades decide, otherwise scaling the coefficient by at most `2^1074`
/// and the binary significand by `10^scale` (whose scale is then within 330 digits
/// of the coefficient's length) and comparing the two.
#[must_use]
pub(crate) const fn decimal_cmp_f64(a: Shape) -> Cost {
    let exponent = a.magnitude_exponent();
    if a.sign == 0 {
        return Cost::new(1, 0);
    }
    let conversion = coefficient_conversion(a);
    if a.digits_exact
        && (exponent >= F64_DECIMAL_EXPONENTS.0 || exponent <= F64_DECIMAL_EXPONENTS.1)
    {
        return conversion.saturating_add(Cost::new(1, 0));
    }
    // The binary shift adds at most seventeen u64 limbs.
    let left = a.limbs.saturating_add(17);
    let right = limbs_for_digits(a.scale).saturating_add(3);
    conversion
        .saturating_add(shift10(17, a.scale))
        .saturating_add(Cost::new(
            left.saturating_mul(37)
                .saturating_add(right.saturating_mul(2))
                .saturating_add(1),
            limb_bytes(left.saturating_add(right)).saturating_mul(4),
        ))
}

/// `a ÷ b` over decimals under `policy`: a rounded quotient scales one operand by
/// the scale gap and divides; an exact one takes a gcd, two reductions, strips the
/// divisor's twos and fives (at most `64·lb` of each, one linear
/// pass apiece) and multiplies by a factor of at most `3·lb` limbs.
#[must_use]
pub(crate) const fn decimal_div(a: Shape, b: Shape, policy: super::DivisionPolicy) -> Cost {
    let (la, lb) = (a.limbs, b.limbs);
    match policy {
        super::DivisionPolicy::Scale { scale, .. } => {
            let shift = (scale as i64)
                .saturating_add(b.scale as i64)
                .saturating_sub(a.scale as i64);
            let digits = shift.unsigned_abs();
            let (numerator, denominator) = if shift >= 0 {
                (la.saturating_add(limbs_for_digits(digits)), lb)
            } else {
                (la, lb.saturating_add(limbs_for_digits(digits)))
            };
            let shifted = if shift >= 0 { la } else { lb };
            shift10(shifted, digits).saturating_add(div(numerator, denominator))
        }
        super::DivisionPolicy::Exact => {
            let strip = lb.saturating_mul(128).saturating_mul(lb.saturating_add(1));
            let reduction = gcd(la, lb)
                .saturating_add(div(la, 1))
                .saturating_add(div(lb, 1))
                .saturating_add(Cost::new(strip, 0))
                .saturating_add(mul(la, lb.saturating_mul(3)));
            // div_exact finishes coefficient * 10^(sb - sa - k) when the
            // exponent is positive. k >= 0, and the coefficient before that
            // scale-up occupies at most la + 3*lb limbs.
            let gap = b.scale.saturating_sub(a.scale);
            if gap == 0 {
                reduction
            } else {
                reduction.saturating_add(shift10(la.saturating_add(lb.saturating_mul(3)), gap))
            }
        }
    }
}

/// What `values` cost to fold into one running total in this order — `SUM`, the
/// first moment of a `VARIANCE` — and an upper bound on the total's shape (`None`
/// for no value): the step costs of a chain whose accumulator takes each sum's
/// bounded shape, done one after another ([`Cost::then`]: the work adds, the bytes
/// are the largest step's). Zero while every step stays inside the bounded
/// variants, where the fold is machine arithmetic.
///
/// The accumulator's shape is bounded by the values seen so far, not grown by a
/// digit per step: `k` values each below `10^W` (with `W` the longest integer part
/// among them) sum to less than `k · 10^W ≤ 10^(W + ⌈log10 k⌉)`, at the largest
/// scale among them. So `n` values of `d` digits cost `n` additions of about
/// `d + log10 n` digits — linear in `n` — where a digit per step would charge
/// `n²/2` and refuse a fold the fold itself would finish.
#[must_use]
pub fn sum_chain(values: impl IntoIterator<Item = Shape>) -> (Cost, Option<Shape>) {
    let mut chain = SumChain::default();
    for value in values {
        chain.push(value);
    }
    chain.finish()
}

/// Incremental form of [`sum_chain`], pricing each addition before it runs.
#[derive(Debug, Default)]
pub struct SumChain {
    total: Cost,
    running: Option<Shape>,
    longest_whole: u64,
    largest_scale: u64,
    count: u64,
}

impl SumChain {
    /// Add one operand shape and return this addition's incremental cost.
    // Keep the extracted recurrence in its caller, without per-operand state/ABI traffic.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn push(&mut self, value: Shape) -> Cost {
        self.count = self.count.saturating_add(1);
        let whole = value.digits.saturating_sub(value.scale);
        if whole > self.longest_whole {
            self.longest_whole = whole;
        }
        if value.scale > self.largest_scale {
            self.largest_scale = value.scale;
        }
        let mut step = Cost::ZERO;
        self.running = Some(match self.running {
            None => value,
            Some(acc) => {
                // ⌈log10 count⌉ for count ≥ 2: the digits of count − 1.
                let carry = u64::from((self.count - 1).ilog10()) + 1;
                let next =
                    Shape::bound(self.longest_whole.saturating_add(carry), self.largest_scale);
                if !(acc.is_bounded() && value.is_bounded() && next.is_bounded()) {
                    step = acc.add_cost(value);
                    self.total = self.total.then(step);
                }
                next
            }
        });
        step
    }

    /// Return the complete chain cost and its resulting shape bound.
    #[inline]
    #[must_use]
    pub const fn finish(&self) -> (Cost, Option<Shape>) {
        (self.total, self.running)
    }
}

/// An upper bound on what comparing `values` costs when each takes part in at
/// most `rounds` comparisons as the side that moves — `⌈log2 n⌉` rounds for a
/// sort, one for a running `MIN`/`MAX`.
///
/// A comparison costs more than a constant only between two values of one sign
/// and one leading-digit position, where it aligns their coefficients; so the
/// values are grouped by that key, and a group holding a value past the bounded
/// variants is charged its size × `rounds` alignments against its largest
/// member. Every other group compares in machine words, or decides by the key
/// alone, and is free.
#[must_use]
pub fn compare_chain(values: &[Shape], rounds: u64) -> Cost {
    let mut keyed: Vec<_> = comparison_keys(values).collect();
    compare_keyed(&mut keyed, rounds)
}

/// Native comparison-domain tuples, without allocation. Operational callers
/// collect these into a buffer whose actual layout has already been admitted.
pub fn comparison_keys(values: &[Shape]) -> impl Iterator<Item = (i32, i64, i64, Shape)> + '_ {
    values.iter().map(|shape| {
        let (lower, upper) = shape.comparison_exponents();
        (shape.sign, lower, upper, *shape)
    })
}

/// The same comparison-chain kernel on caller-owned tuples. Sorting cannot grow
/// or retain the supplied array and allocates no additional scratch.
pub fn compare_keyed(keyed: &mut [(i32, i64, i64, Shape)], rounds: u64) -> Cost {
    keyed.sort_unstable_by_key(|&(sign, lower, _, _)| (sign, lower));
    let mut total = Cost::ZERO;
    let mut start = 0;
    while start < keyed.len() {
        let sign = keyed[start].0;
        let mut upper = keyed[start].2;
        let mut end = start + 1;
        // Every potentially overlapping pair belongs to one component. Between
        // components the production binary bounds decide without conversion.
        while end < keyed.len() && keyed[end].0 == sign && keyed[end].1 <= upper {
            upper = upper.max(keyed[end].2);
            end += 1;
        }
        let group = &keyed[start..end];
        if group.len() > 1 && !group.iter().all(|(_, _, _, shape)| shape.is_bounded()) {
            let largest = group
                .iter()
                .map(|&(_, _, _, shape)| shape)
                .reduce(Shape::comparison_bound)
                .expect("a group is nonempty");
            let one = largest.cmp_cost(largest);
            let count = (group.len() as u64).saturating_mul(rounds);
            total = total.saturating_add(Cost::new(one.work().saturating_mul(count), one.bytes()));
        }
        start = end;
    }
    total
}

/// The rounds of [`compare_chain`] a sort of `n` values makes each value take part
/// in as the moving side: `⌈log2 n⌉`, and at least one.
#[must_use]
pub const fn sort_rounds(n: usize) -> u64 {
    let rounds = n.saturating_sub(1).bit_width() as u64;
    if rounds == 0 { 1 } else { rounds }
}

#[cfg(test)]
mod comparison_bounds_tests {
    use super::{Shape, decimal_cmp};

    #[test]
    fn extreme_digit_and_scale_bounds_preserve_containment() {
        let shape = Shape {
            limbs: u64::MAX,
            digits: u64::MAX,
            minimum_digits: u64::MAX,
            digits_exact: true,
            scale: u64::MAX,
            sign: 1,
        };
        let (lower, upper) = shape.comparison_exponents();
        assert!(lower <= 0 && upper >= 0);
        let one = Shape { scale: 1, ..shape };
        let two = Shape { scale: 2, ..shape };
        assert_eq!(one.comparison_exponents(), (i64::MAX, i64::MAX));
        assert_eq!(two.comparison_exponents(), (i64::MAX, i64::MAX));
        assert!(decimal_cmp(one, two).work() > 1);
    }
}
