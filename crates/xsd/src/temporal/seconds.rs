// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixed-point seconds of the temporal value space: `mantissa × 10^-scale` with an
//! `i128` mantissa and at most eighteen fractional digits.
//!
//! This is the temporal types' own precision bound, not a numeric type. XSD 1.1 lets a
//! processor limit the fractional seconds it supports (Part 2 §5.4, partial
//! implementation of infinite datatypes), and every `xsd:dateTime`, `xsd:time` and
//! `xsd:duration` this crate holds keeps its seconds here, which is what lets their
//! arithmetic stay in machine words. The numeric value space has one representation,
//! [`crate::exact::Decimal`]: every public temporal accessor takes and returns that
//! type, converting at the boundary ([`Fixed::to_exact`], [`Fixed::from_exact`]).

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::exact;
use crate::value::XsdError;

/// Max fractional digits the seconds keep; keeps the mantissa within `i128` headroom.
pub(crate) const MAX_DECIMAL_SCALE: u8 = 18;

/// An exact decimal: `value = mantissa × 10^(-scale)`, `i128`-backed (scale
/// bounded so the mantissa stays in `i128`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Fixed {
    mantissa: i128,
    scale: u8,
}

impl Fixed {
    /// Construct from raw mantissa + scale (internal/testing).
    #[must_use]
    pub(crate) fn from_parts(mantissa: i128, scale: u8) -> Self {
        Self { mantissa, scale }
    }

    /// The mantissa (signed significant digits).
    #[must_use]
    pub(crate) fn mantissa(&self) -> i128 {
        self.mantissa
    }

    /// The scale (number of fractional digits).
    #[must_use]
    pub(crate) fn scale(&self) -> u8 {
        self.scale
    }

    /// The integer (truncated-toward-zero) part of the value.
    #[must_use]
    pub(crate) fn whole_part(&self) -> i128 {
        self.mantissa / 10i128.pow(u32::from(self.scale))
    }

    /// The fractional part of the value as a `Fixed` (same scale).
    #[must_use]
    pub(crate) fn frac_part(&self) -> Self {
        Self {
            mantissa: self.mantissa % 10i128.pow(u32::from(self.scale)),
            scale: self.scale,
        }
    }

    /// True if the value is exactly zero.
    #[must_use]
    pub(crate) fn is_zero(&self) -> bool {
        self.mantissa == 0
    }

    /// Exact comparison of two decimals (total order — decimals are never NaN).
    ///
    /// ## Overflow-safety argument
    ///
    /// `scale` is a `u8` capped at `MAX_DECIMAL_SCALE` (= 18) at every construction
    /// site (`parse_decimal` enforces `frac_str.len() <= 18`; `frac_part` inherits the
    /// parent scale; `from_parts(_, 0)` for integer promotion is scale 0).
    ///
    /// For the fractional-alignment step the two frac mantissas satisfy:
    ///   `|frac_m| < 10^scale ≤ 10^18`
    /// After scaling to the common (higher) scale we multiply by at most `10^diff`
    /// where `diff ≤ 18`, giving a product `< 10^18 × 10^18 = 10^36`.
    /// `i128::MAX ≈ 1.7 × 10^38 > 10^36`, so the multiplication cannot overflow.
    ///
    /// The integer-part comparison uses `whole_part()` which returns `i128` and is
    /// exact (no multiplication); it is compared directly.
    ///
    /// There is NO `f64` path and NO `unwrap_or` swallowing a failure.
    #[must_use]
    pub(crate) fn cmp_exact(&self, other: &Self) -> Ordering {
        // Fast path: identical scale — single cmp, no arithmetic needed.
        if self.scale == other.scale {
            return self.mantissa.cmp(&other.mantissa);
        }

        // Step 1 — sign comparison.  Negative < zero < positive.
        let s_sign = self.mantissa.signum();
        let o_sign = other.mantissa.signum();
        if s_sign != o_sign {
            return s_sign.cmp(&o_sign);
        }
        // Both zero (mantissa == 0 regardless of scale) → Equal.
        if s_sign == 0 {
            return Ordering::Equal;
        }

        // Step 2 — integer part comparison (both same sign, non-zero).
        let s_whole = self.whole_part();
        let o_whole = other.whole_part();
        let whole_ord = s_whole.cmp(&o_whole);
        if whole_ord != Ordering::Equal {
            return whole_ord;
        }

        // Step 3 — fractional part comparison.
        // Each frac mantissa satisfies |frac_m| < 10^scale ≤ 10^18.
        // We scale the lower-scale fraction up to the higher scale by multiplying by
        // 10^diff (diff ≤ 18).  Product < 10^18 × 10^18 = 10^36 < i128::MAX → no
        // overflow.  (Debug assertion guards the invariant during development.)
        debug_assert!(
            self.scale <= MAX_DECIMAL_SCALE && other.scale <= MAX_DECIMAL_SCALE,
            "scale invariant violated: self.scale={}, other.scale={}",
            self.scale,
            other.scale,
        );
        let s_frac = self.frac_part().mantissa;
        let o_frac = other.frac_part().mantissa;
        let frac_ord = if self.scale > other.scale {
            let diff = u32::from(self.scale - other.scale);
            // SAFETY: o_frac < 10^other.scale ≤ 10^18; diff ≤ 18; product < 10^36 < i128::MAX
            let o_scaled = o_frac * 10i128.pow(diff);
            s_frac.cmp(&o_scaled)
        } else {
            let diff = u32::from(other.scale - self.scale);
            // SAFETY: s_frac < 10^self.scale ≤ 10^18; diff ≤ 18; product < 10^36 < i128::MAX
            let s_scaled = s_frac * 10i128.pow(diff);
            s_scaled.cmp(&o_frac)
        };
        // For negative numbers the frac mantissas are negative too (they inherit the
        // sign from `mantissa % 10^scale`), so the direct comparison is already
        // correct: a more-negative fraction means a smaller (more negative) value.
        frac_ord
    }

    /// XSD 1.1 canonical lexical form (Part 2 §3.3.3.1, whose canonical
    /// representation is `decimalCanonicalMap`, defined in §E.1): an
    /// integer-valued decimal has NO decimal point (`3` → `"3"`, `-2` → `"-2"`,
    /// `0` → `"0"`); a non-integer decimal keeps its fractional part with trailing
    /// zeros trimmed (`2.50` → `"2.5"`, `-0.250` → `"-0.25"`).
    #[must_use]
    pub(crate) fn canonical_lexical(&self) -> String {
        // Two allocations (the digit string and the exact-fit output) where the
        // split/pad/format form built three to four intermediate `String`s; the
        // integer and fraction parts are slices of `digits`, and the zero padding
        // is pushed directly. Byte-identical to the reference form in the tests.
        let neg = self.mantissa < 0;
        let digits = self.mantissa.unsigned_abs().to_string();
        let scale = usize::from(self.scale);
        let mut out = String::with_capacity(digits.len() + scale + 3);
        if neg {
            out.push('-');
        }
        if scale == 0 {
            out.push_str(&digits);
            return out;
        }

        let (int_part, frac_digits, pad) = if digits.len() > scale {
            let split = digits.len() - scale;
            (&digits[..split], &digits[split..], 0)
        } else {
            // value magnitude < 1: pad leading zeros in the fractional part.
            ("0", digits.as_str(), scale - digits.len())
        };

        // XSD 1.1 §E.1 `decimalCanonicalMap`: an integer-valued decimal (an empty
        // fractional part after trimming trailing zeros) has NO decimal point at all. The pad is
        // all zeros, so the padded fraction trims to empty iff `frac_digits` does.
        let frac_trimmed = frac_digits.trim_end_matches('0');
        out.push_str(int_part);
        if !frac_trimmed.is_empty() {
            out.push('.');
            for _ in 0..pad {
                out.push('0');
            }
            out.push_str(frac_trimmed);
        }
        out
    }
}

/// `xsd:decimal`: optional sign, digits with an optional single `.` (at least one
/// digit overall; `.5`, `1.`, `1.5`, `12` all valid).
pub(crate) fn parse_decimal(s: &str) -> Result<Fixed, XsdError> {
    let dt = XsdDatatype::Decimal;
    let neg = s.starts_with('-');
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);

    let (int_str, frac_str) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    // A second '.' can only live after the first one, i.e. inside `frac_str`:
    // one scan of the tail replaces the `contains` + `matches().count()` pair.
    if frac_str.contains('.') {
        return Err(XsdError::invalid(dt, s, "more than one decimal point"));
    }
    if int_str.is_empty() && frac_str.is_empty() {
        return Err(XsdError::invalid(dt, s, "no digits"));
    }
    if !int_str.bytes().all(|b| b.is_ascii_digit()) || !frac_str.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(XsdError::invalid(dt, s, "non-digit character"));
    }
    if frac_str.len() > usize::from(MAX_DECIMAL_SCALE) {
        return Err(XsdError::OutOfRange {
            datatype: dt,
            lexical: s.to_string(),
            reason: crate::value::reason::DECIMAL_TOO_PRECISE,
        });
    }

    let digits = format!("{int_str}{frac_str}");
    let digits_trimmed = digits.trim_start_matches('0');
    let out_of_range = || XsdError::OutOfRange {
        datatype: dt,
        lexical: s.to_string(),
        reason: crate::value::reason::INTEGER_TOO_LARGE,
    };
    // The magnitude is read unsigned so that `i128::MIN`, whose magnitude is one past
    // `i128::MAX`, is a mantissa like any other (it is the value `xsd:decimal` of the
    // integer `i128::MIN` holds, and its canonical lexical must read back).
    let magnitude = if digits_trimmed.is_empty() {
        0u128
    } else {
        digits_trimmed.parse::<u128>().map_err(|_| out_of_range())?
    };
    let mantissa = if neg {
        0i128.checked_sub_unsigned(magnitude)
    } else {
        i128::try_from(magnitude).ok()
    }
    .ok_or_else(out_of_range)?;
    // `frac_str.len() <= MAX_DECIMAL_SCALE <= u8::MAX`, so the cast cannot truncate.
    Ok(Fixed::from_parts(mantissa, frac_str.len() as u8))
}

/// Align two decimals to the same (higher) scale by scaling up the mantissa of
/// the lower-scale operand. Returns `(a_mantissa, b_mantissa, common_scale)`, or
/// `None` when the scaled-up mantissa leaves `i128` — the same overflow the
/// following addition/subtraction would report, reached one step earlier.
///
/// The scale-up factor is `10^diff` with `diff ≤ MAX_DECIMAL_SCALE` (18), but
/// the mantissa being scaled is NOT bounded by `10^scale`: a scale-0 decimal
/// may carry any `i128` mantissa (`1e30` is a valid `xsd:decimal`), so
/// `1e30 + 0.000000000000000001` must scale `1e30` by `10^18` — past `i128`.
/// The multiplication is therefore checked; an unchecked one panicked under
/// overflow checks and silently wrapped to a wrong sum without them.
pub(crate) fn align_decimals(a: &Fixed, b: &Fixed) -> Option<(i128, i128, u8)> {
    if a.scale() == b.scale() {
        return Some((a.mantissa(), b.mantissa(), a.scale()));
    }
    if a.scale() > b.scale() {
        let diff = u32::from(a.scale() - b.scale());
        let b_scaled = b.mantissa().checked_mul(10i128.pow(diff))?;
        Some((a.mantissa(), b_scaled, a.scale()))
    } else {
        let diff = u32::from(b.scale() - a.scale());
        let a_scaled = a.mantissa().checked_mul(10i128.pow(diff))?;
        Some((a_scaled, b.mantissa(), b.scale()))
    }
}

/// The exact-decimal-multiplication math behind [`decimal_mul`], returning the raw
/// `Fixed` rather than a wrapped `XsdValue`. Shared with `temporal.rs` (duration ×
/// numeric, XPath F&O `op:multiply-yearMonthDuration`/`op:multiply-dayTimeDuration`).
pub(crate) fn decimal_mul_raw(a: &Fixed, b: &Fixed) -> Result<Fixed, XsdError> {
    let new_mantissa =
        a.mantissa()
            .checked_mul(b.mantissa())
            .ok_or_else(|| XsdError::OutOfRange {
                datatype: XsdDatatype::Decimal,
                lexical: String::new(),
                reason: "decimal multiplication overflow",
            })?;
    let raw_scale = u32::from(a.scale()) + u32::from(b.scale());
    if raw_scale <= u32::from(MAX_DECIMAL_SCALE) {
        Ok(Fixed::from_parts(new_mantissa, raw_scale as u8))
    } else {
        // Truncate toward zero to MAX_DECIMAL_SCALE fractional digits.
        // SAFETY: excess ≤ 36 (max raw_scale is 36); 10^excess ≤ 10^36 < i128::MAX.
        // But we cannot represent 10^36 in i128 (i128::MAX ≈ 1.70×10^38 > 10^36),
        // however 10^38 > i128::MAX, so we need to be careful.
        // excess ≤ raw_scale - 0 ≤ 18 + 18 = 36; 10^36 ≈ 1×10^36 < 1.70×10^38 = i128::MAX.
        // So 10i128.pow(excess) does not overflow for excess ≤ 36.
        let excess = raw_scale - u32::from(MAX_DECIMAL_SCALE);
        let divisor = 10i128.pow(excess);
        let truncated = new_mantissa / divisor;
        Ok(Fixed::from_parts(truncated, MAX_DECIMAL_SCALE))
    }
}

/// The exact-decimal-division math behind [`decimal_div`], returning the raw
/// `Fixed` rather than a wrapped `XsdValue`. Shared with `temporal.rs` (duration ÷
/// numeric and `dayTimeDuration` ÷ `dayTimeDuration`, XPath F&O
/// `op:divide-dayTimeDuration`/`op:divide-dayTimeDuration-by-dayTimeDuration`).
///
/// Every current caller (in this module and in `temporal.rs`) already checks its own
/// divisor and returns a datatype-specific `Err(DivisionByZero)` before ever reaching
/// here, so this function's own zero check normally never fires. It is not, however,
/// a `debug_assert!`: this is `pub(crate)` and reachable from more than one call
/// site, so a caller added later that forgets its own check must still get a typed
/// `Err`, in every build profile, rather than the `i128` division below panicking —
/// on `wasm32-unknown-unknown` with `panic = "abort"` an integer-division panic tears
/// down the whole module, not just the one call, so this is not merely a debug-time
/// convenience. The datatype this reports is always [`XsdDatatype::Decimal`]
/// (the type this raw function itself operates over) rather than whatever
/// caller-specific datatype (`Integer`, `YearMonthDuration`, …) a pre-check would
/// have reported — irrelevant in practice, since every real caller's own check
/// already reports the right datatype and never lets a zero divisor reach here.
pub(crate) fn decimal_div_raw(dividend: &Fixed, divisor: &Fixed) -> Result<Fixed, XsdError> {
    if divisor.is_zero() {
        return Err(XsdError::DivisionByZero {
            datatype: XsdDatatype::Decimal,
        });
    }
    // dividend = dm × 10^-ds and divisor = vm × 10^-vs, so the quotient's mantissa at
    // scale S is trunc(dm × 10^(S + vs - ds) / vm). The result is that mantissa at the
    // finest scale S ≤ MAX_DECIMAL_SCALE whose mantissa fits the i128 the value space
    // holds: the exact quotient whenever it is representable, else the quotient
    // truncated toward zero at that scale (XPath F&O 3.1 §4.2, which §4.2.4 follows,
    // leaves the precision of a decimal quotient implementation-defined; this crate
    // truncates, as documented on `decimal_div`). Only a quotient whose integer part
    // alone exceeds i128 is an overflow (`err:FOAR0002`).
    let dm = dividend.mantissa();
    let vm = divisor.mantissa();
    let vs = i32::from(divisor.scale());
    let ds = i32::from(dividend.scale());
    let negative = (dm < 0) != (vm < 0);
    let fits = |magnitude: u128| {
        if negative {
            0_i128.checked_sub_unsigned(magnitude)
        } else {
            i128::try_from(magnitude).ok()
        }
    };
    let (numerator, denominator) = (dm.unsigned_abs(), vm.unsigned_abs());
    for scale in (0..=MAX_DECIMAL_SCALE).rev() {
        let shift = i32::from(scale) + vs - ds;
        let magnitude = if shift >= 0 {
            // shift ≤ 18 + 18 = 36, and 10^36 < 2^128.
            crate::wide::mul_div(numerator, 10_u128.pow(shift.unsigned_abs()), denominator)
        } else {
            // ⌊⌊n / 10^k⌋ / d⌋ = ⌊n / (10^k · d)⌋ for positive integers; k ≤ 18.
            Some(numerator / 10_u128.pow(shift.unsigned_abs()) / denominator)
        };
        if let Some(mantissa) = magnitude.and_then(fits) {
            return Ok(Fixed::from_parts(mantissa, scale));
        }
    }
    Err(XsdError::OutOfRange {
        datatype: XsdDatatype::Decimal,
        lexical: String::new(),
        reason: "decimal quotient exceeds the i128 mantissa at every scale",
    })
}

impl Fixed {
    /// The exact decimal this fixed-point value denotes.
    pub(crate) fn to_exact(self) -> exact::Decimal {
        exact::Decimal::new(
            exact::Integer::from_i128(self.mantissa),
            u32::from(self.scale),
        )
    }

    /// `value` in the seconds' fixed point: [`XsdError::OutOfRange`] when it needs
    /// more than eighteen fractional digits or a mantissa past `i128` — the bound the
    /// temporal value space keeps (`datatype` names the type the caller computes).
    pub(crate) fn from_exact(
        value: &exact::Decimal,
        datatype: XsdDatatype,
    ) -> Result<Self, XsdError> {
        let refuse = || XsdError::OutOfRange {
            datatype,
            lexical: value.canonical_lexical(),
            reason: "a temporal value keeps at most eighteen fractional digits of seconds in an i128",
        };
        let scale = u8::try_from(value.scale())
            .ok()
            .filter(|scale| *scale <= MAX_DECIMAL_SCALE);
        match (value.unscaled().as_i128(), scale) {
            (Some(mantissa), Some(scale)) => Ok(Self::from_parts(mantissa, scale)),
            _ => Err(refuse()),
        }
    }
}
