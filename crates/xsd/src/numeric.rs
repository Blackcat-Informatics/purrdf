// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The XSD numeric value space: `integer`, `decimal`, `float`, `double`, their
//! lexical↔value parsing + canonical mapping, and the SPARQL numeric promotion
//! lattice (`integer ⊂ decimal ⊂ float ⊂ double`) used for cross-type comparison.

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::value::{XsdError, XsdValue};

/// An exact decimal: `value = mantissa × 10^(-scale)`. Mirrors `oxsdatatypes`'
/// `i128`-backed design (scale bounded so the mantissa stays in `i128`).
#[derive(Debug, Clone, Copy)]
pub struct Decimal {
    mantissa: i128,
    scale: u8,
}

/// Max fractional digits we retain; keeps the mantissa within `i128` headroom and
/// matches `oxsdatatypes`' precision.
const MAX_DECIMAL_SCALE: u8 = 18;

impl Decimal {
    /// Construct from raw mantissa + scale (internal/testing).
    #[must_use]
    pub(crate) fn from_parts(mantissa: i128, scale: u8) -> Self {
        Decimal { mantissa, scale }
    }

    /// The mantissa (signed significant digits).
    #[must_use]
    pub fn mantissa(&self) -> i128 {
        self.mantissa
    }

    /// The scale (number of fractional digits).
    #[must_use]
    pub fn scale(&self) -> u8 {
        self.scale
    }

    /// Lossy conversion to `f64` (for promotion to `double`/`float`).
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        self.mantissa as f64 / 10f64.powi(i32::from(self.scale))
    }

    /// The integer (truncated-toward-zero) part of the value.
    #[must_use]
    pub fn whole_part(&self) -> i128 {
        self.mantissa / 10i128.pow(u32::from(self.scale))
    }

    /// The fractional part of the value as a `Decimal` (same scale).
    #[must_use]
    pub fn frac_part(&self) -> Decimal {
        Decimal {
            mantissa: self.mantissa % 10i128.pow(u32::from(self.scale)),
            scale: self.scale,
        }
    }

    /// True if the value is exactly zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
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
    pub fn cmp_exact(&self, other: &Decimal) -> Ordering {
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

    /// XSD canonical lexical form: decimal point mandatory, no trailing fractional
    /// zeros except the one required to keep a digit after the point.
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        let neg = self.mantissa < 0;
        let digits = self.mantissa.unsigned_abs().to_string();
        let scale = usize::from(self.scale);

        let (int_part, frac_part) = if scale == 0 {
            (digits.clone(), String::new())
        } else if digits.len() > scale {
            let split = digits.len() - scale;
            (digits[..split].to_string(), digits[split..].to_string())
        } else {
            // value magnitude < 1: pad leading zeros in the fractional part.
            let pad = "0".repeat(scale - digits.len());
            ("0".to_string(), format!("{pad}{digits}"))
        };

        // Trim trailing zeros from the fractional part, keep at least one digit.
        let frac_trimmed = frac_part.trim_end_matches('0');
        let frac_final = if frac_trimmed.is_empty() {
            "0"
        } else {
            frac_trimmed
        };
        let sign = if neg { "-" } else { "" };
        format!("{sign}{int_part}.{frac_final}")
    }
}

fn invalid(dt: XsdDatatype, lexical: &str, reason: &'static str) -> XsdError {
    XsdError::InvalidLexical {
        datatype: dt,
        lexical: lexical.to_string(),
        reason,
    }
}

/// `xsd:integer`: optional leading `+`/`-`, then one or more ASCII digits.
/// Returns the raw `i128` value without any subtype range check — for range-checked
/// integer-family parsing use [`parse_integer_typed`].
pub fn parse_integer(s: &str) -> Result<i128, XsdError> {
    let dt = XsdDatatype::Integer;
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(dt, s, "expected an optional sign then digits"));
    }
    s.parse::<i128>().map_err(|_| XsdError::OutOfRange {
        datatype: dt,
        lexical: s.to_string(),
        reason: "integer magnitude exceeds i128",
    })
}

/// Parse a lexical integer form for the given `datatype`, hard-failing with
/// [`XsdError::OutOfRange`] if the value is outside the datatype's inclusive bounds.
///
/// This is the unified entry point for all integer-family datatypes; `parse` in
/// `value.rs` routes every integer-family IRI through here.
pub fn parse_integer_typed(lexical: &str, datatype: XsdDatatype) -> Result<i128, XsdError> {
    // First, parse as an unconstrained integer (which may itself fail with
    // InvalidLexical for malformed input, or OutOfRange for beyond-i128).
    // We call parse_integer but report the error under `datatype` for non-Integer
    // subtypes, so callers see the correct IRI in the error.
    let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return Err(XsdError::InvalidLexical {
            datatype,
            lexical: lexical.to_string(),
            reason: "expected an optional sign then digits",
        });
    }
    let value = lexical.parse::<i128>().map_err(|_| XsdError::OutOfRange {
        datatype,
        lexical: lexical.to_string(),
        reason: "integer magnitude exceeds i128",
    })?;

    // Now range-check against the datatype's inclusive bounds.
    if let Some((min, max)) = datatype.integer_range() {
        if value < min || value > max {
            return Err(XsdError::OutOfRange {
                datatype,
                lexical: lexical.to_string(),
                reason: "value outside datatype range",
            });
        }
    }
    Ok(value)
}

/// `xsd:decimal`: optional sign, digits with an optional single `.` (at least one
/// digit overall; `.5`, `1.`, `1.5`, `12` all valid).
pub fn parse_decimal(s: &str) -> Result<Decimal, XsdError> {
    let dt = XsdDatatype::Decimal;
    let neg = s.starts_with('-');
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);

    let (int_str, frac_str) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    if body.contains('.') && body.matches('.').count() > 1 {
        return Err(invalid(dt, s, "more than one decimal point"));
    }
    if int_str.is_empty() && frac_str.is_empty() {
        return Err(invalid(dt, s, "no digits"));
    }
    if !int_str.bytes().all(|b| b.is_ascii_digit()) || !frac_str.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid(dt, s, "non-digit character"));
    }
    if frac_str.len() > usize::from(MAX_DECIMAL_SCALE) {
        return Err(XsdError::OutOfRange {
            datatype: dt,
            lexical: s.to_string(),
            reason: "decimal scale exceeds 18",
        });
    }

    let digits = format!("{int_str}{frac_str}");
    let digits_trimmed = digits.trim_start_matches('0');
    let magnitude = if digits_trimmed.is_empty() {
        0i128
    } else {
        digits_trimmed
            .parse::<i128>()
            .map_err(|_| XsdError::OutOfRange {
                datatype: dt,
                lexical: s.to_string(),
                reason: "integer magnitude exceeds i128",
            })?
    };
    let mantissa = if neg { -magnitude } else { magnitude };
    // `frac_str.len() <= MAX_DECIMAL_SCALE <= u8::MAX`, so the cast cannot truncate.
    Ok(Decimal::from_parts(mantissa, frac_str.len() as u8))
}

/// `xsd:double`: XSD numeric float lexical, or `INF`/`+INF`/`-INF`/`NaN`.
pub fn parse_double(s: &str) -> Result<f64, XsdError> {
    parse_ieee(s, XsdDatatype::Double)
}

/// `xsd:float`: as `double` but single-precision.
pub fn parse_float(s: &str) -> Result<f32, XsdError> {
    let dt = XsdDatatype::Float;
    match s {
        "INF" | "+INF" => return Ok(f32::INFINITY),
        "-INF" => return Ok(f32::NEG_INFINITY),
        "NaN" => return Ok(f32::NAN),
        _ => {}
    }
    reject_non_xsd_numeric(s, dt)?;
    s.parse::<f32>()
        .map_err(|_| invalid(dt, s, "not a valid float lexical"))
}

/// Shared finite-numeric parse for double; returns `f64`.
fn parse_ieee(s: &str, dt: XsdDatatype) -> Result<f64, XsdError> {
    match s {
        "INF" | "+INF" => return Ok(f64::INFINITY),
        "-INF" => return Ok(f64::NEG_INFINITY),
        "NaN" => return Ok(f64::NAN),
        _ => {}
    }
    reject_non_xsd_numeric(s, dt)?;
    s.parse::<f64>()
        .map_err(|_| invalid(dt, s, "not a valid double lexical"))
}

/// Reject lexicals Rust's float parser would accept but XSD forbids (`inf`,
/// `infinity`, `nan`, etc.): any ASCII letter other than the `e`/`E` exponent
/// marker disqualifies the form (the `INF`/`NaN` keywords are handled before here).
fn reject_non_xsd_numeric(s: &str, dt: XsdDatatype) -> Result<(), XsdError> {
    if s.bytes()
        .any(|b| b.is_ascii_alphabetic() && b != b'e' && b != b'E')
    {
        return Err(invalid(dt, s, "non-XSD numeric token"));
    }
    Ok(())
}

/// XSD canonical `double`: `m.dddEsexp`, mantissa in shortest round-trippable form,
/// `INF`/`-INF`/`NaN` for the specials.
#[must_use]
pub fn canonical_double(d: f64) -> String {
    canonical_ieee(d, d.is_nan(), d.is_infinite(), d.is_sign_negative(), || {
        format!("{d:e}")
    })
}

/// XSD canonical `float`.
#[must_use]
pub fn canonical_float(f: f32) -> String {
    canonical_ieee(
        f64::from(f),
        f.is_nan(),
        f.is_infinite(),
        f.is_sign_negative(),
        || format!("{f:e}"),
    )
}

fn canonical_ieee(
    value: f64,
    is_nan: bool,
    is_inf: bool,
    is_neg: bool,
    sci: impl Fn() -> String,
) -> String {
    if is_nan {
        return "NaN".to_string();
    }
    if is_inf {
        return if is_neg { "-INF" } else { "INF" }.to_string();
    }
    if value == 0.0 {
        return if is_neg { "-0.0E0" } else { "0.0E0" }.to_string();
    }
    // Rust's `{:e}` is the shortest round-trippable scientific form (e.g. `1e2`,
    // `1.5e0`, `5e-3`). Normalize to the XSD canonical `mantissa.frac E exp`.
    let raw = sci();
    let (mantissa, exp) = raw.split_once('e').unwrap_or((raw.as_str(), "0"));
    let mantissa = if mantissa.contains('.') {
        mantissa.to_string()
    } else {
        format!("{mantissa}.0")
    };
    format!("{mantissa}E{exp}")
}

/// SPARQL numeric promotion comparison. Promotes both operands to the least type
/// that contains them (`integer ⊂ decimal ⊂ float ⊂ double`) and compares. Returns
/// `None` when an operand is `NaN` (genuinely unordered) or non-numeric (the caller
/// — `value_cmp` — only routes numeric operands here; non-numeric → `None`).
///
/// Integer-vs-integer comparison is by value only (ignoring the subtype): per the
/// SPARQL promotion rules, `xsd:int 5 = xsd:long 5`.
#[must_use]
pub fn numeric_cmp(a: &XsdValue, b: &XsdValue) -> Option<Ordering> {
    use XsdValue::{Decimal as Dec, Double, Float, Integer};
    match (a, b) {
        // Same exact integer / decimal cases keep full precision.
        // Integer-vs-integer: compare by value, ignore subtype (xsd:int 5 == xsd:long 5).
        (Integer { value: x, .. }, Integer { value: y, .. }) => Some(x.cmp(y)),
        (Dec(x), Dec(y)) => Some(x.cmp_exact(y)),
        (Integer { value: x, .. }, Dec(y)) => Some(Decimal::from_parts(*x, 0).cmp_exact(y)),
        (Dec(x), Integer { value: y, .. }) => Some(x.cmp_exact(&Decimal::from_parts(*y, 0))),
        // Any `double` operand → compare as f64.
        (Double(_), _) | (_, Double(_)) => num_f64(a)?.partial_cmp(&num_f64(b)?),
        // Else any `float` operand → compare as f32.
        (Float(_), _) | (_, Float(_)) => num_f32(a)?.partial_cmp(&num_f32(b)?),
        // At least one operand is non-numeric.
        _ => None,
    }
}

/// SPARQL numeric value equality (`=`) via the promotion comparison.
#[must_use]
pub fn numeric_eq(a: &XsdValue, b: &XsdValue) -> bool {
    numeric_cmp(a, b) == Some(Ordering::Equal)
}

/// The numeric value as `f64`, or `None` if `v` is not a numeric value.
fn num_f64(v: &XsdValue) -> Option<f64> {
    Some(match v {
        // Spec-mandated lossy promotion: integer ⊂ double (SPARQL §17.3 numeric tower).
        // Large i128 values (> 2^53) lose low-order bits; this is required behaviour,
        // not an accident.
        XsdValue::Integer { value, .. } => *value as f64,
        XsdValue::Decimal(d) => d.to_f64(),
        XsdValue::Float(f) => f64::from(*f),
        XsdValue::Double(d) => *d,
        _ => return None,
    })
}

/// The numeric value as `f32`, or `None` if `v` is not a numeric value.
fn num_f32(v: &XsdValue) -> Option<f32> {
    Some(match v {
        // Spec-mandated lossy promotion: integer ⊂ float (SPARQL §17.3 numeric tower).
        // Large i128 values (> 2^24) lose precision; required by IEEE promotion semantics.
        XsdValue::Integer { value, .. } => *value as f32,
        // Decimal → f64 → f32: two rounds of precision loss. First round is inherent
        // (decimal to IEEE double), second round narrows to single. Both are required by
        // the SPARQL promotion rules (decimal ⊂ float); no intermediate Decimal→f32 path exists.
        XsdValue::Decimal(d) => d.to_f64() as f32,
        XsdValue::Float(f) => *f,
        // double → float narrowing: required by the numeric tower when a float operand
        // forces promotion of the other operand down (SPARQL §17.3).
        XsdValue::Double(d) => *d as f32,
        _ => return None,
    })
}

// ── Numeric arithmetic (SPARQL §17.4 / XPath op:numeric-*) ──────────────────

/// Promote both operands to the least common type in the numeric tower, perform
/// the given exact-integer operation (add/sub/mul), and return an `XsdValue::Integer`
/// result. Returns `Err(OutOfRange)` on overflow.
fn int_binop(
    x: i128,
    y: i128,
    op: impl Fn(i128, i128) -> Option<i128>,
) -> Result<XsdValue, XsdError> {
    op(x, y)
        .map(|value| XsdValue::Integer {
            value,
            datatype: XsdDatatype::Integer,
        })
        .ok_or_else(|| XsdError::OutOfRange {
            datatype: XsdDatatype::Integer,
            lexical: "overflow in integer arithmetic".to_string(),
            reason: "integer arithmetic overflow",
        })
}

/// Align two decimals to the same (higher) scale by scaling up the mantissa of
/// the lower-scale operand. Returns `(a_mantissa, b_mantissa, common_scale)`.
///
/// ## Overflow-safety argument
///
/// Both operands satisfy `|mantissa| < 10^scale ≤ 10^MAX_DECIMAL_SCALE` (= 10^18).
/// The scale-up factor is `10^diff` where `diff ≤ 18`.
/// Product `< 10^18 × 10^18 = 10^36 < i128::MAX (≈ 1.70×10^38)`. No overflow.
/// However, the mantissa of an *add/sub result* can be up to `2 × 10^36` which still
/// fits in i128; the caller must not further scale without checking.
fn align_decimals(a: &Decimal, b: &Decimal) -> (i128, i128, u8) {
    if a.scale() == b.scale() {
        return (a.mantissa(), b.mantissa(), a.scale());
    }
    if a.scale() > b.scale() {
        let diff = u32::from(a.scale() - b.scale());
        // SAFETY: b.mantissa < 10^b.scale ≤ 10^18; diff ≤ 18; product < 10^36 < i128::MAX
        let b_scaled = b.mantissa() * 10i128.pow(diff);
        (a.mantissa(), b_scaled, a.scale())
    } else {
        let diff = u32::from(b.scale() - a.scale());
        // SAFETY: a.mantissa < 10^a.scale ≤ 10^18; diff ≤ 18; product < 10^36 < i128::MAX
        let a_scaled = a.mantissa() * 10i128.pow(diff);
        (a_scaled, b.mantissa(), b.scale())
    }
}

/// Promote an `Integer` to a `Decimal` with scale 0.
fn integer_to_decimal(value: i128) -> Decimal {
    Decimal::from_parts(value, 0)
}

/// SPARQL `op:numeric-add` (`+`). Follows the numeric promotion tower:
/// `integer ⊂ decimal ⊂ float ⊂ double`. Integer addition is exact (`i128`);
/// decimal addition is exact within the representable range; float/double are IEEE.
///
/// Returns `Err(OutOfRange)` on exact-type overflow, `Err(TypeMismatch)` if either
/// operand is not numeric.
pub fn numeric_add(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    use XsdValue::{Decimal as Dec, Double, Float, Integer};
    match (a, b) {
        // Both double OR either double → f64
        (Double(_), _) | (_, Double(_)) => {
            let x = num_f64(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in add",
            })?;
            let y = num_f64(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in add",
            })?;
            Ok(XsdValue::Double(x + y))
        }
        // Either float (no double) → f32
        (Float(_), _) | (_, Float(_)) => {
            let x = num_f32(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in add",
            })?;
            let y = num_f32(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in add",
            })?;
            Ok(XsdValue::Float(x + y))
        }
        // Either decimal (no float/double) → exact decimal
        (Dec(x), Dec(y)) => decimal_add(x, y),
        (Integer { value: x, .. }, Dec(y)) => decimal_add(&integer_to_decimal(*x), y),
        (Dec(x), Integer { value: y, .. }) => decimal_add(x, &integer_to_decimal(*y)),
        // Both integer → exact i128
        (Integer { value: x, .. }, Integer { value: y, .. }) => {
            int_binop(*x, *y, i128::checked_add)
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand in add",
        }),
    }
}

fn decimal_add(a: &Decimal, b: &Decimal) -> Result<XsdValue, XsdError> {
    let (am, bm, scale) = align_decimals(a, b);
    let result = am.checked_add(bm).ok_or_else(|| XsdError::OutOfRange {
        datatype: XsdDatatype::Decimal,
        lexical: String::new(),
        reason: "decimal addition overflow",
    })?;
    Ok(XsdValue::Decimal(Decimal::from_parts(result, scale)))
}

/// SPARQL `op:numeric-subtract` (`-`). Same promotion tower as `numeric_add`.
///
/// Returns `Err(OutOfRange)` on exact-type overflow, `Err(TypeMismatch)` if either
/// operand is not numeric.
pub fn numeric_sub(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    use XsdValue::{Decimal as Dec, Double, Float, Integer};
    match (a, b) {
        (Double(_), _) | (_, Double(_)) => {
            let x = num_f64(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in sub",
            })?;
            let y = num_f64(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in sub",
            })?;
            Ok(XsdValue::Double(x - y))
        }
        (Float(_), _) | (_, Float(_)) => {
            let x = num_f32(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in sub",
            })?;
            let y = num_f32(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in sub",
            })?;
            Ok(XsdValue::Float(x - y))
        }
        (Dec(x), Dec(y)) => decimal_sub(x, y),
        (Integer { value: x, .. }, Dec(y)) => decimal_sub(&integer_to_decimal(*x), y),
        (Dec(x), Integer { value: y, .. }) => decimal_sub(x, &integer_to_decimal(*y)),
        (Integer { value: x, .. }, Integer { value: y, .. }) => {
            int_binop(*x, *y, i128::checked_sub)
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand in sub",
        }),
    }
}

fn decimal_sub(a: &Decimal, b: &Decimal) -> Result<XsdValue, XsdError> {
    let (am, bm, scale) = align_decimals(a, b);
    let result = am.checked_sub(bm).ok_or_else(|| XsdError::OutOfRange {
        datatype: XsdDatatype::Decimal,
        lexical: String::new(),
        reason: "decimal subtraction overflow",
    })?;
    Ok(XsdValue::Decimal(Decimal::from_parts(result, scale)))
}

/// SPARQL `op:numeric-multiply` (`*`). Same promotion tower as `numeric_add`.
///
/// Decimal multiplication: `new_mantissa = a.mantissa × b.mantissa`,
/// `new_scale = a.scale + b.scale`. If `new_scale > MAX_DECIMAL_SCALE`, the result
/// is rounded (truncated toward zero) to scale 18. Mantissa overflow → `OutOfRange`.
///
/// Returns `Err(OutOfRange)` on exact-type overflow, `Err(TypeMismatch)` if either
/// operand is not numeric.
pub fn numeric_mul(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    use XsdValue::{Decimal as Dec, Double, Float, Integer};
    match (a, b) {
        (Double(_), _) | (_, Double(_)) => {
            let x = num_f64(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in mul",
            })?;
            let y = num_f64(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in mul",
            })?;
            Ok(XsdValue::Double(x * y))
        }
        (Float(_), _) | (_, Float(_)) => {
            let x = num_f32(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in mul",
            })?;
            let y = num_f32(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in mul",
            })?;
            Ok(XsdValue::Float(x * y))
        }
        (Dec(x), Dec(y)) => decimal_mul(x, y),
        (Integer { value: x, .. }, Dec(y)) => decimal_mul(&integer_to_decimal(*x), y),
        (Dec(x), Integer { value: y, .. }) => decimal_mul(x, &integer_to_decimal(*y)),
        (Integer { value: x, .. }, Integer { value: y, .. }) => {
            int_binop(*x, *y, i128::checked_mul)
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand in mul",
        }),
    }
}

fn decimal_mul(a: &Decimal, b: &Decimal) -> Result<XsdValue, XsdError> {
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
        Ok(XsdValue::Decimal(Decimal::from_parts(
            new_mantissa,
            raw_scale as u8,
        )))
    } else {
        // Truncate toward zero to MAX_DECIMAL_SCALE fractional digits.
        let excess = raw_scale - u32::from(MAX_DECIMAL_SCALE);
        // SAFETY: excess ≤ 36 (max raw_scale is 36); 10^excess ≤ 10^36 < i128::MAX.
        // But we cannot represent 10^36 in i128 (i128::MAX ≈ 1.70×10^38 > 10^36),
        // however 10^38 > i128::MAX, so we need to be careful.
        // excess ≤ raw_scale - 0 ≤ 18 + 18 = 36; 10^36 ≈ 1×10^36 < 1.70×10^38 = i128::MAX.
        // So 10i128.pow(excess) does not overflow for excess ≤ 36.
        let divisor = 10i128.pow(excess);
        let truncated = new_mantissa / divisor;
        Ok(XsdValue::Decimal(Decimal::from_parts(
            truncated,
            MAX_DECIMAL_SCALE,
        )))
    }
}

/// SPARQL `op:numeric-divide` (`/`). Integer ÷ integer returns **decimal** (not
/// integer), per XPath `op:numeric-divide` semantics. All other pairs follow the
/// numeric promotion tower.
///
/// Division by zero:
/// - `xsd:integer` or `xsd:decimal` divisor = 0 → `Err(DivisionByZero)` (hard error).
/// - `xsd:float` or `xsd:double` divisor = 0.0 → IEEE result (±INF, or NaN for 0÷0).
///
/// Returns `Err(TypeMismatch)` if either operand is not numeric.
pub fn numeric_div(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    use XsdValue::{Decimal as Dec, Double, Float, Integer};
    match (a, b) {
        (Double(_), _) | (_, Double(_)) => {
            let x = num_f64(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in div",
            })?;
            let y = num_f64(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in div",
            })?;
            Ok(XsdValue::Double(x / y))
        }
        (Float(_), _) | (_, Float(_)) => {
            let x = num_f32(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in div",
            })?;
            let y = num_f32(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in div",
            })?;
            Ok(XsdValue::Float(x / y))
        }
        // Integer ÷ Integer → Decimal (XPath op:numeric-divide spec rule)
        (Integer { value: x, .. }, Integer { value: y, .. }) => {
            if *y == 0 {
                return Err(XsdError::DivisionByZero {
                    datatype: XsdDatatype::Integer,
                });
            }
            decimal_div(&integer_to_decimal(*x), &integer_to_decimal(*y))
        }
        (Dec(x), Dec(y)) => {
            if y.is_zero() {
                return Err(XsdError::DivisionByZero {
                    datatype: XsdDatatype::Decimal,
                });
            }
            decimal_div(x, y)
        }
        (Integer { value: x, .. }, Dec(y)) => {
            if y.is_zero() {
                return Err(XsdError::DivisionByZero {
                    datatype: XsdDatatype::Decimal,
                });
            }
            decimal_div(&integer_to_decimal(*x), y)
        }
        (Dec(x), Integer { value: y, .. }) => {
            if *y == 0 {
                return Err(XsdError::DivisionByZero {
                    datatype: XsdDatatype::Decimal,
                });
            }
            decimal_div(x, &integer_to_decimal(*y))
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand in div",
        }),
    }
}

/// Exact decimal long division, producing up to `MAX_DECIMAL_SCALE` (18) fractional
/// digits by truncation toward zero.
///
/// Algorithm: scale the dividend mantissa up by `10^target_scale` to capture enough
/// fractional precision, then integer-divide by the divisor mantissa. The result
/// mantissa is `(dividend_m × 10^shift) / divisor_m` at scale `target_scale`.
///
/// The shift factor is chosen as `MAX_DECIMAL_SCALE + divisor.scale()` minus the
/// dividend scale so that the final result lands at exactly scale `MAX_DECIMAL_SCALE`.
fn decimal_div(dividend: &Decimal, divisor: &Decimal) -> Result<XsdValue, XsdError> {
    // We want: result = dividend / divisor at scale MAX_DECIMAL_SCALE.
    // dividend = dm × 10^(-ds), divisor = vm × 10^(-vs).
    // result mantissa at scale S = dm × 10^(S + vs - ds) / vm
    // where S = MAX_DECIMAL_SCALE.
    let dm = dividend.mantissa();
    let vm = divisor.mantissa();
    // Combined scale shift: (MAX_DECIMAL_SCALE + vs) - ds.
    // vs and ds are both ≤ 18, and MAX_DECIMAL_SCALE = 18, so the net exponent
    // is in [-18, 36]. We must keep the dividend mantissa from overflowing i128.
    let target_scale = MAX_DECIMAL_SCALE;
    let vs = i32::from(divisor.scale());
    let ds = i32::from(dividend.scale());
    let shift_exp: i32 = i32::from(target_scale) + vs - ds;
    // Scale dm up (or down) by 10^shift_exp.
    let scaled_dm: i128 = if shift_exp >= 0 {
        // Scale up: dm × 10^shift_exp. Max shift is 18 + 18 - 0 = 36.
        // 10^36 ≈ 10^36, and i128::MAX ≈ 1.70×10^38, so we can represent 10^36.
        // dm itself can be up to i128::MAX / 10 (from multiplication), but in the
        // typical case |dm| ≤ 10^18. If the scale-up overflows, return OutOfRange.
        let factor = 10i128.pow(shift_exp as u32);
        dm.checked_mul(factor).ok_or_else(|| XsdError::OutOfRange {
            datatype: XsdDatatype::Decimal,
            lexical: String::new(),
            reason: "decimal division intermediate overflow",
        })?
    } else {
        // Scale down: dm / 10^(-shift_exp). Precision is lost; this only happens
        // when the dividend has more fractional digits than target_scale + vs,
        // which is unusual but possible.
        let factor = 10i128.pow((-shift_exp) as u32);
        dm / factor
    };
    Ok(XsdValue::Decimal(Decimal::from_parts(
        scaled_dm / vm,
        target_scale,
    )))
}

/// SPARQL `op:numeric-unary-minus` (unary `-`). Negates the value, preserving its type.
///
/// For integers, negation uses checked arithmetic; `i128::MIN` negated overflows →
/// `Err(OutOfRange)`. For float/double, IEEE negation (−0.0 negates to +0.0 and
/// vice-versa; NaN negates to NaN with sign flipped per IEEE 754-2008 §6.3).
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_unary_minus(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { value, datatype } => value
            .checked_neg()
            .map(|v| XsdValue::Integer {
                value: v,
                datatype: *datatype,
            })
            .ok_or_else(|| XsdError::OutOfRange {
                datatype: *datatype,
                lexical: value.to_string(),
                reason: "integer unary minus overflow (i128::MIN has no positive counterpart)",
            }),
        XsdValue::Decimal(d) => {
            // Decimal negation: negate the mantissa. No overflow: i128::MIN has no
            // positive counterpart, but parse_decimal rejects values that would place
            // the mantissa at i128::MIN (it parses the magnitude separately as unsigned).
            // Defensive check retained for safety.
            d.mantissa()
                .checked_neg()
                .map(|m| XsdValue::Decimal(Decimal::from_parts(m, d.scale())))
                .ok_or_else(|| XsdError::OutOfRange {
                    datatype: XsdDatatype::Decimal,
                    lexical: d.canonical_lexical(),
                    reason: "decimal unary minus overflow (mantissa is i128::MIN)",
                })
        }
        XsdValue::Float(f) => Ok(XsdValue::Float(-f)),
        XsdValue::Double(d) => Ok(XsdValue::Double(-d)),
        _ => Err(XsdError::TypeMismatch {
            reason: "unary minus applied to non-numeric value",
        }),
    }
}

/// SPARQL `op:numeric-unary-plus` (unary `+`). Identity for numeric types.
///
/// Returns `Err(TypeMismatch)` for non-numeric operands (e.g. `+true` is a type
/// error in SPARQL/XPath, not a no-op).
pub fn numeric_unary_plus(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { .. }
        | XsdValue::Decimal(_)
        | XsdValue::Float(_)
        | XsdValue::Double(_) => Ok(a.clone()),
        _ => Err(XsdError::TypeMismatch {
            reason: "unary plus applied to non-numeric value",
        }),
    }
}

// ── Numeric math functions (SPARQL §17.4.4 / XPath fn:abs, fn:ceiling, etc.) ──

/// SPARQL `fn:abs` — absolute value, preserving the operand's numeric type.
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_abs(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { value, datatype } => value
            .checked_abs()
            .map(|v| XsdValue::Integer {
                value: v,
                datatype: *datatype,
            })
            .ok_or_else(|| XsdError::OutOfRange {
                datatype: *datatype,
                lexical: value.to_string(),
                reason: "abs overflow (i128::MIN has no positive counterpart)",
            }),
        XsdValue::Decimal(d) => d
            .mantissa()
            .checked_abs()
            .map(|m| XsdValue::Decimal(Decimal::from_parts(m, d.scale())))
            .ok_or_else(|| XsdError::OutOfRange {
                datatype: XsdDatatype::Decimal,
                lexical: d.canonical_lexical(),
                reason: "abs overflow (mantissa is i128::MIN)",
            }),
        XsdValue::Float(f) => Ok(XsdValue::Float(f.abs())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.abs())),
        _ => Err(XsdError::TypeMismatch {
            reason: "abs applied to non-numeric value",
        }),
    }
}

/// SPARQL `fn:ceiling` — smallest integer not less than the value, preserving the
/// operand's numeric type (`integer → integer`, `decimal → decimal` exact,
/// `float/double → float/double`).
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_ceil(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        // Integer is already an integer; ceiling is identity.
        XsdValue::Integer { .. } => Ok(a.clone()),
        XsdValue::Decimal(d) => {
            // ceiling(n.frac) = whole_part + (if frac > 0 { 1 } else { 0 })
            let whole = d.whole_part();
            let frac_m = d.frac_part().mantissa();
            let result = if frac_m > 0 {
                whole.checked_add(1).ok_or_else(|| XsdError::OutOfRange {
                    datatype: XsdDatatype::Decimal,
                    lexical: d.canonical_lexical(),
                    reason: "ceiling overflow",
                })?
            } else {
                whole
            };
            Ok(XsdValue::Decimal(Decimal::from_parts(result, 0)))
        }
        XsdValue::Float(f) => Ok(XsdValue::Float(f.ceil())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.ceil())),
        _ => Err(XsdError::TypeMismatch {
            reason: "ceiling applied to non-numeric value",
        }),
    }
}

/// SPARQL `fn:floor` — largest integer not greater than the value, preserving the
/// operand's numeric type (`integer → integer`, `decimal → decimal` exact,
/// `float/double → float/double`).
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_floor(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        // Integer is already an integer; floor is identity.
        XsdValue::Integer { .. } => Ok(a.clone()),
        XsdValue::Decimal(d) => {
            // floor(n.frac) = whole_part - (if frac < 0 { 1 } else { 0 })
            let whole = d.whole_part();
            let frac_m = d.frac_part().mantissa();
            let result = if frac_m < 0 {
                whole.checked_sub(1).ok_or_else(|| XsdError::OutOfRange {
                    datatype: XsdDatatype::Decimal,
                    lexical: d.canonical_lexical(),
                    reason: "floor overflow",
                })?
            } else {
                whole
            };
            Ok(XsdValue::Decimal(Decimal::from_parts(result, 0)))
        }
        XsdValue::Float(f) => Ok(XsdValue::Float(f.floor())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.floor())),
        _ => Err(XsdError::TypeMismatch {
            reason: "floor applied to non-numeric value",
        }),
    }
}

/// SPARQL `fn:round` — round to the nearest integer, with half-values rounded
/// toward positive infinity (`fn:round` semantics per XPath §4.4.5). Preserves the
/// operand's numeric type.
///
/// Examples: `round(2.5) = 3`, `round(-2.5) = -2`, `round(2.4999) = 2`.
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_round(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        // Integer is already integral; round is identity.
        XsdValue::Integer { .. } => Ok(a.clone()),
        XsdValue::Decimal(d) => {
            // XPath fn:round: half-values round toward +infinity.
            // For positive: round-half-up. For negative: round-half toward zero (not
            // away), so -2.5 rounds to -2, not -3.
            //
            // Algorithm: frac_m is the fractional mantissa (same sign as d.mantissa).
            // scale is the number of fractional digits.
            // half-threshold = 10^(scale-1) × 5, same sign as d.
            let whole = d.whole_part();
            let frac_m = d.frac_part().mantissa();
            if d.scale() == 0 {
                // Already integral (scale 0 means the value IS an integer).
                return Ok(XsdValue::Decimal(Decimal::from_parts(whole, 0)));
            }
            let scale = u32::from(d.scale());
            // threshold = 5 × 10^(scale-1). For scale 1 that is 5, scale 2 → 50, etc.
            let threshold = 5i128 * 10i128.pow(scale - 1);
            // frac_m has the same sign as the mantissa (it is mantissa % 10^scale).
            // We round toward +inf on the half: add 1 if frac_m >= +threshold (positive
            // half case), leave alone otherwise. For negatives, frac_m is negative and
            // -2.5 → whole=-2, frac=-5 (at scale 1), threshold=5: frac_m=-5 < 5, so
            // whole stays -2 (rounds toward zero = toward +inf for negative half).
            let result = if frac_m >= threshold {
                whole.checked_add(1).ok_or_else(|| XsdError::OutOfRange {
                    datatype: XsdDatatype::Decimal,
                    lexical: d.canonical_lexical(),
                    reason: "round overflow",
                })?
            } else {
                whole
            };
            Ok(XsdValue::Decimal(Decimal::from_parts(result, 0)))
        }
        XsdValue::Float(f) => {
            // f32::round() is round-half-away-from-zero; XPath fn:round is
            // round-half-toward-+infinity. For positive they agree. For negative halves
            // they differ: f32::round(-2.5) = -3 but fn:round(-2.5) = -2.
            // Correction: for negative values at the half-point, add 1.0.
            let r = if *f == f.floor() + 0.5 && *f < 0.0 {
                f.ceil()
            } else {
                f.round()
            };
            Ok(XsdValue::Float(r))
        }
        XsdValue::Double(d) => {
            // Same correction as float.
            let r = if *d == d.floor() + 0.5 && *d < 0.0 {
                d.ceil()
            } else {
                d.round()
            };
            Ok(XsdValue::Double(r))
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "round applied to non-numeric value",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::XsdDatatype as D;
    use pretty_assertions::assert_eq;

    fn dec(s: &str) -> Decimal {
        parse_decimal(s).unwrap()
    }

    fn int_val(n: i128) -> XsdValue {
        XsdValue::Integer {
            value: n,
            datatype: D::Integer,
        }
    }

    #[test]
    fn integer_parse_and_bounds() {
        assert_eq!(parse_integer("42").unwrap(), 42);
        assert_eq!(parse_integer("-7").unwrap(), -7);
        assert_eq!(parse_integer("+7").unwrap(), 7);
        assert_eq!(parse_integer("007").unwrap(), 7);
        assert_eq!(parse_integer(&i128::MAX.to_string()).unwrap(), i128::MAX);
        // i128::MAX + 1 overflows -> hard OutOfRange, not saturation.
        assert!(matches!(
            parse_integer("170141183460469231731687303715884105728"),
            Err(XsdError::OutOfRange { .. })
        ));
        assert!(parse_integer("1.0").is_err());
        assert!(parse_integer("").is_err());
        assert!(parse_integer("abc").is_err());
    }

    #[test]
    fn parse_integer_typed_range_checks() {
        // xsd:byte: -128..127
        assert_eq!(parse_integer_typed("127", D::Byte).unwrap(), 127);
        assert_eq!(parse_integer_typed("-128", D::Byte).unwrap(), -128);
        assert!(parse_integer_typed("128", D::Byte).is_err());
        assert!(parse_integer_typed("-129", D::Byte).is_err());

        // xsd:unsignedByte: 0..255
        assert_eq!(parse_integer_typed("255", D::UnsignedByte).unwrap(), 255);
        assert_eq!(parse_integer_typed("0", D::UnsignedByte).unwrap(), 0);
        assert!(parse_integer_typed("256", D::UnsignedByte).is_err());
        assert!(parse_integer_typed("-1", D::UnsignedByte).is_err());

        // xsd:positiveInteger: >= 1
        assert_eq!(parse_integer_typed("1", D::PositiveInteger).unwrap(), 1);
        assert!(parse_integer_typed("0", D::PositiveInteger).is_err());

        // xsd:negativeInteger: <= -1
        assert_eq!(parse_integer_typed("-1", D::NegativeInteger).unwrap(), -1);
        assert!(parse_integer_typed("0", D::NegativeInteger).is_err());

        // xsd:nonNegativeInteger: >= 0
        assert_eq!(parse_integer_typed("0", D::NonNegativeInteger).unwrap(), 0);
        assert!(parse_integer_typed("-1", D::NonNegativeInteger).is_err());

        // xsd:nonPositiveInteger: <= 0
        assert_eq!(parse_integer_typed("0", D::NonPositiveInteger).unwrap(), 0);
        assert!(parse_integer_typed("1", D::NonPositiveInteger).is_err());

        // xsd:unsignedLong boundary: u64::MAX should pass; u64::MAX+1 should fail
        let u64max = u64::MAX.to_string();
        assert_eq!(
            parse_integer_typed(&u64max, D::UnsignedLong).unwrap(),
            u64::MAX as i128
        );
        assert!(parse_integer_typed("18446744073709551616", D::UnsignedLong).is_err());

        // xsd:int: 2147483647 ok, 2147483648 fails
        assert_eq!(
            parse_integer_typed("2147483647", D::Int).unwrap(),
            2147483647
        );
        assert!(parse_integer_typed("2147483648", D::Int).is_err());
    }

    #[test]
    fn decimal_parse_and_canonical() {
        assert_eq!(dec("12.34").canonical_lexical(), "12.34");
        assert_eq!(dec("12.00").canonical_lexical(), "12.0");
        assert_eq!(dec("100").canonical_lexical(), "100.0");
        assert_eq!(dec("-0.5").canonical_lexical(), "-0.5");
        assert_eq!(dec(".5").canonical_lexical(), "0.5");
        assert_eq!(dec("1.").canonical_lexical(), "1.0");
        assert_eq!(dec("0.005").canonical_lexical(), "0.005");
        assert!(parse_decimal("1.2.3").is_err());
        assert!(parse_decimal("").is_err());
    }

    #[test]
    fn decimal_exact_comparison_across_scales() {
        assert_eq!(dec("1.5").cmp_exact(&dec("1.50")), Ordering::Equal);
        assert_eq!(dec("1.5").cmp_exact(&dec("1.05")), Ordering::Greater);
        assert_eq!(dec("0.1").cmp_exact(&dec("0.2")), Ordering::Less);
    }

    // ── cmp_exact correctness tests ──────────────────────────────────────────────

    /// Cross-scale equality: 1.50 (mantissa=150, scale=2) == 1.5 (mantissa=15, scale=1).
    #[test]
    fn cmp_exact_cross_scale_equal() {
        let a = Decimal::from_parts(150, 2); // 1.50
        let b = Decimal::from_parts(15, 1); // 1.5
        assert_eq!(a.cmp_exact(&b), Ordering::Equal);
        assert_eq!(b.cmp_exact(&a), Ordering::Equal);
    }

    /// Cross-scale strict order: 1.5 < 1.50001.
    #[test]
    fn cmp_exact_cross_scale_strict() {
        let a = dec("1.5");
        let b = dec("1.50001");
        assert_eq!(a.cmp_exact(&b), Ordering::Less);
        assert_eq!(b.cmp_exact(&a), Ordering::Greater);
    }

    /// Negative cross-scale: -1.5 vs -1.50001.
    /// -1.50001 < -1.5 (more negative).
    #[test]
    fn cmp_exact_negative_cross_scale() {
        let a = dec("-1.5");
        let b = dec("-1.50001");
        assert_eq!(a.cmp_exact(&b), Ordering::Greater); // -1.5 > -1.50001
        assert_eq!(b.cmp_exact(&a), Ordering::Less);
    }

    /// Mixed signs: any positive > any negative.
    #[test]
    fn cmp_exact_mixed_signs() {
        assert_eq!(dec("0.001").cmp_exact(&dec("-999.9")), Ordering::Greater);
        assert_eq!(dec("-0.001").cmp_exact(&dec("999.9")), Ordering::Less);
    }

    /// Both-zero regardless of scale.
    #[test]
    fn cmp_exact_zero_any_scale() {
        let z0 = Decimal::from_parts(0, 0);
        let z5 = Decimal::from_parts(0, 5);
        let z18 = Decimal::from_parts(0, 18);
        assert_eq!(z0.cmp_exact(&z5), Ordering::Equal);
        assert_eq!(z5.cmp_exact(&z18), Ordering::Equal);
        assert_eq!(z18.cmp_exact(&z0), Ordering::Equal);
    }

    /// Large-mantissa regression: two large decimals at scale 0 vs scale 1 that the
    /// old 10^diff widening path would overflow on (mantissa near i128::MAX).
    ///
    /// The old code attempted: (i128::MAX / 10) * 10  which checks out but
    /// i128::MAX * 10 overflows — so we construct a pair where the lower-scale value's
    /// mantissa is large enough that multiplying by 10^diff would exceed i128::MAX.
    ///
    /// Specifically: mantissa = i128::MAX (scale 0) vs mantissa = i128::MAX (scale 1).
    /// Value A = i128::MAX × 10^0 = i128::MAX (≈ 1.70141…×10^38)
    /// Value B = i128::MAX × 10^(-1) ≈ 1.70141…×10^37
    /// So A > B.  The old code would try to scale A's mantissa up by 10 → overflow.
    #[test]
    fn cmp_exact_large_mantissa_no_overflow() {
        // A = i128::MAX at scale 0; B = i128::MAX at scale 1
        // A = 170141183460469231731687303715884105727
        // B = 17014118346046923173168730371588410572.7
        // True order: A > B
        let a = Decimal::from_parts(i128::MAX, 0);
        let b = Decimal::from_parts(i128::MAX, 1);
        assert_eq!(a.cmp_exact(&b), Ordering::Greater);
        assert_eq!(b.cmp_exact(&a), Ordering::Less);
    }

    /// Regression vector for the exact f64 collapse bug: two large unequal decimals
    /// at different scales that the old f64 path would round to the same f64 value
    /// and therefore return Equal incorrectly.
    ///
    /// f64 has ~15.9 significant decimal digits.  Construct two values that differ
    /// only in the 18th digit — well below f64 resolution — but whose true order
    /// is strict.
    ///
    /// A = 100000000000000000.1  (mantissa=1000000000000000001, scale=1)
    /// B = 100000000000000000.2  (mantissa=1000000000000000002, scale=1)
    /// Both have the same f64 representation (the fractional digit is lost), but
    /// A < B is exact.
    #[test]
    fn cmp_exact_large_f64_collapse_regression() {
        // 100000000000000000.1 and 100000000000000000.2 — same scale, near i64::MAX magnitude
        let a = Decimal::from_parts(1_000_000_000_000_000_001, 1);
        let b = Decimal::from_parts(1_000_000_000_000_000_002, 1);
        // Both collapse to the same f64 — the old path returns Equal incorrectly.
        assert_eq!(a.to_f64(), b.to_f64(), "f64 collapse precondition");
        // cmp_exact must return Less (A < B), not Equal.
        assert_eq!(a.cmp_exact(&b), Ordering::Less);
        assert_eq!(b.cmp_exact(&a), Ordering::Greater);
    }

    /// Same as above but across scales (scale 1 vs scale 2).
    #[test]
    fn cmp_exact_large_f64_collapse_cross_scale_regression() {
        // A = 100000000000000000.1  (scale 1)
        // B = 100000000000000000.11 (scale 2) = 10000000000000000011 mantissa
        // A < B (0.1 < 0.11).  Both f64-identical at this magnitude.
        let a = Decimal::from_parts(1_000_000_000_000_000_001, 1); // .1 at scale 1
        let b = Decimal::from_parts(10_000_000_000_000_000_011, 2); // .11 at scale 2
        assert_eq!(a.to_f64(), b.to_f64(), "f64 collapse precondition");
        assert_eq!(a.cmp_exact(&b), Ordering::Less);
        assert_eq!(b.cmp_exact(&a), Ordering::Greater);
    }

    #[test]
    fn double_specials_and_canonical() {
        assert_eq!(parse_double("INF").unwrap(), f64::INFINITY);
        assert_eq!(parse_double("-INF").unwrap(), f64::NEG_INFINITY);
        assert!(parse_double("NaN").unwrap().is_nan());
        assert!(parse_double("inf").is_err());
        assert!(parse_double("Infinity").is_err());
        assert_eq!(canonical_double(1.0), "1.0E0");
        assert_eq!(canonical_double(1.5), "1.5E0");
        assert_eq!(canonical_double(100.0), "1.0E2");
        assert_eq!(canonical_double(0.005), "5.0E-3");
        assert_eq!(canonical_double(f64::INFINITY), "INF");
        assert_eq!(canonical_double(f64::NEG_INFINITY), "-INF");
        assert_eq!(canonical_double(f64::NAN), "NaN");
    }

    #[test]
    fn numeric_promotion() {
        // "1"^^integer = "1.0"^^decimal
        assert!(numeric_eq(&int_val(1), &XsdValue::Decimal(dec("1.0"))));
        // integer vs double
        assert_eq!(
            numeric_cmp(&int_val(2), &XsdValue::Double(2.5)),
            Some(Ordering::Less)
        );
        // decimal vs float
        assert_eq!(
            numeric_cmp(&XsdValue::Decimal(dec("1.5")), &XsdValue::Float(1.25)),
            Some(Ordering::Greater)
        );
        // NaN is unordered and unequal.
        assert_eq!(numeric_cmp(&XsdValue::Double(f64::NAN), &int_val(1)), None);
        assert!(!numeric_eq(
            &XsdValue::Double(f64::NAN),
            &XsdValue::Double(f64::NAN)
        ));
        // +0 == -0.
        assert!(numeric_eq(&XsdValue::Double(0.0), &XsdValue::Double(-0.0)));

        // Cross-subtype integer equality: xsd:int 5 == xsd:long 5.
        let int5 = XsdValue::Integer {
            value: 5,
            datatype: D::Int,
        };
        let long5 = XsdValue::Integer {
            value: 5,
            datatype: D::Long,
        };
        assert!(numeric_eq(&int5, &long5));
        assert_eq!(numeric_cmp(&int5, &long5), Some(Ordering::Equal));
    }

    // ── Arithmetic tests ─────────────────────────────────────────────────────

    fn dec_val(s: &str) -> XsdValue {
        XsdValue::Decimal(parse_decimal(s).unwrap())
    }

    fn float_val(f: f32) -> XsdValue {
        XsdValue::Float(f)
    }

    fn double_val(d: f64) -> XsdValue {
        XsdValue::Double(d)
    }

    /// Helper: extract the Decimal from an XsdValue::Decimal, panic otherwise.
    fn as_decimal(v: &XsdValue) -> Decimal {
        match v {
            XsdValue::Decimal(d) => *d,
            other => panic!("expected Decimal, got {other:?}"),
        }
    }

    /// Helper: extract the i128 from an XsdValue::Integer, panic otherwise.
    fn as_integer(v: &XsdValue) -> i128 {
        match v {
            XsdValue::Integer { value, .. } => *value,
            other => panic!("expected Integer, got {other:?}"),
        }
    }

    /// Helper: extract f64 from XsdValue::Double, panic otherwise.
    fn as_double(v: &XsdValue) -> f64 {
        match v {
            XsdValue::Double(d) => *d,
            other => panic!("expected Double, got {other:?}"),
        }
    }

    /// Helper: extract f32 from XsdValue::Float, panic otherwise.
    fn as_float(v: &XsdValue) -> f32 {
        match v {
            XsdValue::Float(f) => *f,
            other => panic!("expected Float, got {other:?}"),
        }
    }

    // -- integer + integer → integer --

    #[test]
    fn add_integer_integer() {
        let result = numeric_add(&int_val(3), &int_val(4)).unwrap();
        assert_eq!(as_integer(&result), 7);
    }

    #[test]
    fn add_integer_overflow() {
        // i128::MAX + 1 must be OutOfRange, never wrap.
        let max = int_val(i128::MAX);
        let one = int_val(1);
        assert!(matches!(
            numeric_add(&max, &one),
            Err(XsdError::OutOfRange { .. })
        ));
    }

    // -- integer division returns Decimal (SPARQL §17.4 / XPath op:numeric-divide) --

    #[test]
    fn div_integer_integer_returns_decimal() {
        // 1 / 2 must be Decimal(0.5), NOT Integer(0)
        let result = numeric_div(&int_val(1), &int_val(2)).unwrap();
        assert!(
            matches!(result, XsdValue::Decimal(_)),
            "expected Decimal, got {result:?}"
        );
        let d = as_decimal(&result);
        // 0.5 at scale 18: mantissa = 5×10^17
        assert_eq!(d.to_f64(), 0.5, "1/2 must equal 0.5");
    }

    #[test]
    fn div_4_2_is_decimal_two() {
        // 4 / 2 must be Decimal(2.0), NOT Integer(2)
        let result = numeric_div(&int_val(4), &int_val(2)).unwrap();
        assert!(matches!(result, XsdValue::Decimal(_)));
        let d = as_decimal(&result);
        assert_eq!(d.to_f64(), 2.0, "4/2 must equal 2.0 as decimal");
    }

    #[test]
    fn div_1_3_is_18_digit_decimal() {
        // 1 / 3 → Decimal, 18 fractional digits of 3s
        let result = numeric_div(&int_val(1), &int_val(3)).unwrap();
        let d = as_decimal(&result);
        // Canonical form should start with "0.333333333333333333"
        let lex = d.canonical_lexical();
        assert!(
            lex.starts_with("0.333333333333333333"),
            "expected 0.333...333 (18 threes), got {lex}"
        );
        // Exactly 18 fractional digits
        let frac = lex.split('.').nth(1).unwrap_or("");
        assert_eq!(
            frac.len(),
            18,
            "should have 18 fractional digits, got {frac}"
        );
    }

    // -- decimal exactness: 0.1 + 0.2 == 0.3 (the classic float failure) --

    #[test]
    fn decimal_add_exact_no_float_error() {
        // IEEE double: 0.1 + 0.2 ≠ 0.3; exact decimal: 0.1 + 0.2 == 0.3.
        let result = numeric_add(&dec_val("0.1"), &dec_val("0.2")).unwrap();
        let d = as_decimal(&result);
        let expected = parse_decimal("0.3").unwrap();
        assert_eq!(
            d.cmp_exact(&expected),
            Ordering::Equal,
            "0.1 + 0.2 must equal 0.3 exactly in decimal; got {}",
            d.canonical_lexical()
        );
    }

    // -- numeric promotion: integer + double → double --

    #[test]
    fn add_integer_double_promotes_to_double() {
        let result = numeric_add(&int_val(1), &double_val(1.5)).unwrap();
        assert!(
            matches!(result, XsdValue::Double(_)),
            "expected Double, got {result:?}"
        );
        let d = as_double(&result);
        assert_eq!(d, 2.5);
    }

    // -- numeric promotion: decimal + float → float --

    #[test]
    fn add_decimal_float_promotes_to_float() {
        let result = numeric_add(&dec_val("1.5"), &float_val(0.5)).unwrap();
        assert!(
            matches!(result, XsdValue::Float(_)),
            "expected Float, got {result:?}"
        );
        // 1.5 + 0.5 = 2.0
        assert_eq!(as_float(&result), 2.0_f32);
    }

    // -- numeric promotion: integer × decimal → decimal --

    #[test]
    fn mul_integer_decimal_promotes_to_decimal() {
        // 3 × 1.5 = 4.5
        let result = numeric_mul(&int_val(3), &dec_val("1.5")).unwrap();
        assert!(
            matches!(result, XsdValue::Decimal(_)),
            "expected Decimal, got {result:?}"
        );
        let d = as_decimal(&result);
        let expected = parse_decimal("4.5").unwrap();
        assert_eq!(
            d.cmp_exact(&expected),
            Ordering::Equal,
            "3 × 1.5 must equal 4.5; got {}",
            d.canonical_lexical()
        );
    }

    // -- division by zero --

    #[test]
    fn div_integer_by_zero_is_error() {
        assert!(matches!(
            numeric_div(&int_val(5), &int_val(0)),
            Err(XsdError::DivisionByZero {
                datatype: XsdDatatype::Integer
            })
        ));
    }

    #[test]
    fn div_decimal_by_zero_is_error() {
        assert!(matches!(
            numeric_div(&dec_val("5.0"), &dec_val("0")),
            Err(XsdError::DivisionByZero {
                datatype: XsdDatatype::Decimal
            })
        ));
    }

    #[test]
    fn div_double_by_zero_is_inf_not_error() {
        // IEEE 754: positive / +0.0 = +INF
        let result = numeric_div(&double_val(5.0), &double_val(0.0)).unwrap();
        let d = as_double(&result);
        assert!(
            d.is_infinite() && d.is_sign_positive(),
            "5.0 / 0.0 must be +INF"
        );
    }

    #[test]
    fn div_double_zero_by_zero_is_nan_not_error() {
        // IEEE 754: 0.0 / 0.0 = NaN (no error)
        let result = numeric_div(&double_val(0.0), &double_val(0.0)).unwrap();
        let d = as_double(&result);
        assert!(d.is_nan(), "0.0 / 0.0 must be NaN");
    }

    // -- unary minus --

    #[test]
    fn unary_minus_integer() {
        assert_eq!(as_integer(&numeric_unary_minus(&int_val(5)).unwrap()), -5);
        assert_eq!(as_integer(&numeric_unary_minus(&int_val(-3)).unwrap()), 3);
    }

    #[test]
    fn unary_minus_decimal() {
        let result = numeric_unary_minus(&dec_val("1.5")).unwrap();
        let d = as_decimal(&result);
        assert_eq!(d.canonical_lexical(), "-1.5");
    }

    #[test]
    fn unary_minus_float() {
        let result = numeric_unary_minus(&float_val(2.5)).unwrap();
        assert_eq!(as_float(&result), -2.5_f32);
    }

    #[test]
    fn unary_minus_double() {
        // Use a value that is not an approx of a named constant (clippy::approx_constant).
        let result = numeric_unary_minus(&double_val(1.23456)).unwrap();
        assert!((as_double(&result) - (-1.23456)).abs() < 1e-12);
    }

    // -- unary plus --

    #[test]
    fn unary_plus_is_identity_for_numerics() {
        // integer
        let i = int_val(42);
        let r = numeric_unary_plus(&i).unwrap();
        assert_eq!(as_integer(&r), 42);
        // decimal
        let d_in = dec_val("1.5");
        let d_out = numeric_unary_plus(&d_in).unwrap();
        assert_eq!(as_decimal(&d_out).canonical_lexical(), "1.5");
        // float
        let f_in = float_val(3.0);
        let f_out = numeric_unary_plus(&f_in).unwrap();
        assert_eq!(as_float(&f_out), 3.0_f32);
        // double — use a value that is not an approx of a named constant
        let dbl_in = double_val(9.876);
        let dbl_out = numeric_unary_plus(&dbl_in).unwrap();
        assert!((as_double(&dbl_out) - 9.876).abs() < 1e-12);
    }

    #[test]
    fn unary_plus_non_numeric_is_error() {
        let boolean = XsdValue::Boolean(true);
        assert!(matches!(
            numeric_unary_plus(&boolean),
            Err(XsdError::TypeMismatch { .. })
        ));
        let string = XsdValue::String("hello".to_string());
        assert!(matches!(
            numeric_unary_plus(&string),
            Err(XsdError::TypeMismatch { .. })
        ));
    }
}
