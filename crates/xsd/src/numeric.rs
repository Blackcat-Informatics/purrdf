// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The XSD numeric value space: `integer`, `decimal`, `float`, `double`, their
//! lexical↔value parsing + canonical mapping, the SPARQL numeric promotion lattice
//! (`integer ⊂ decimal ⊂ float ⊂ double`) used for cross-type comparison, and the
//! `op:numeric-*` operators.
//!
//! `xsd:integer` and `xsd:decimal` have one representation each, at every size:
//! [`crate::exact::Integer`] and [`crate::exact::Decimal`]. Both hold a value that
//! fits `i128` inline and compute on it with checked machine arithmetic, spilling
//! into [`crate::BigInt`] only for a result that leaves it, so a small value costs a
//! machine operation and no allocation.

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::exact::{self, DivisionPolicy, Rounding};
use crate::ieee;
use crate::value::{XsdError, XsdValue};

pub(crate) mod exact_path;

pub use exact_path::CostOp;

/// The cost of one numeric operation over `a` and `b`, for a governor to charge
/// before the operation runs, computed in constant time from the operands' sizes:
///
/// * two `xsd:integer`/`xsd:decimal` operands of which one is past the machine
///   words (an integer outside `i128`, or a decimal whose coefficient is outside it
///   or whose scale passes eighteen digits) cost what [`crate::exact::cost`] states
///   for the operation;
/// * a division under any [`DivisionPolicy`] other than the default costs the same
///   whatever its operands' size. Under the default, a quotient of two machine-word
///   operands is decided in machine words, and its exact expansion, when it
///   terminates, has at most 127 fractional digits;
/// * one such big operand meeting an `xsd:float`/`xsd:double` costs its correctly
///   rounded conversion or its exact comparison with the binary value;
/// * everything else computes in machine words: [`exact::Cost::ZERO`].
///
/// The cost covers the operation, not the rendering of its result: a caller that
/// writes the result as a lexical form charges [`numeric_render_cost`] of it too,
/// which is where a product's growing scale is paid for.
#[must_use]
#[inline]
pub fn numeric_cost(a: &XsdValue, b: &XsdValue, op: CostOp) -> exact::Cost {
    let tower_division =
        matches!(op, CostOp::Div(policy) if policy != DivisionPolicy::xsd_default());
    // Two machine-word operands under the default division: decided by the values'
    // representations alone, so the operators' fast path pays one match here.
    if !tower_division && !beyond_machine_words(a) && !beyond_machine_words(b) {
        return exact::Cost::ZERO;
    }
    tower_cost(a, b, op, tower_division)
}

/// [`numeric_cost`] once an operand is past the machine words or the division runs on
/// the tower. Out of line, so the fast path above stays one match.
#[cold]
#[inline(never)]
fn tower_cost(a: &XsdValue, b: &XsdValue, op: CostOp, tower_division: bool) -> exact::Cost {
    match (exact_path::shape_of(a), exact_path::shape_of(b)) {
        (Some(x), Some(y))
            if beyond_machine_words(a) || beyond_machine_words(b) || tower_division =>
        {
            let integer = |v: &XsdValue| matches!(v, XsdValue::Integer { .. });
            let integers = integer(a) && integer(b) && !matches!(op, CostOp::Div(_));
            exact_path::cost(x, y, integers, op)
        }
        (Some(x), None)
            if beyond_machine_words(a) && matches!(b, XsdValue::Float(_) | XsdValue::Double(_)) =>
        {
            exact_path::ieee_cost(x)
        }
        (None, Some(y))
            if beyond_machine_words(b) && matches!(a, XsdValue::Float(_) | XsdValue::Double(_)) =>
        {
            exact_path::ieee_cost(y)
        }
        _ => exact::Cost::ZERO,
    }
}

/// The cost of a unary numeric function over `value` — negation, `fn:abs`,
/// `fn:ceiling`, `fn:floor`, `fn:round` — when its operand is past the machine
/// words; zero otherwise.
#[must_use]
pub fn numeric_unary_cost(value: &XsdValue) -> exact::Cost {
    match value {
        XsdValue::Integer { value, .. } if value.as_i128().is_none() => value.unary_cost(),
        XsdValue::Decimal(decimal) if decimal_beyond_machine_words(decimal) => decimal.unary_cost(),
        _ => exact::Cost::ZERO,
    }
}

/// The cost of converting `value` to the nearest `xsd:double`/`xsd:float` — a cast,
/// or a promotion against an IEEE operand — when it is past the machine words;
/// zero otherwise.
#[must_use]
pub fn numeric_to_float_cost(value: &XsdValue) -> exact::Cost {
    match value {
        XsdValue::Integer { value, .. } if value.as_i128().is_none() => value.to_float_cost(),
        XsdValue::Decimal(decimal) if decimal_beyond_machine_words(decimal) => {
            decimal.to_float_cost()
        }
        _ => exact::Cost::ZERO,
    }
}

/// The cost of rendering `value`'s canonical lexical form
/// ([`XsdValue::canonical_lexical`]) when it is past the machine words: every byte
/// of the text, the leading zeros of a long fraction included, so a value whose
/// coefficient is short but whose scale is vast is priced at the text it becomes.
/// Zero for every other value, whose text is bounded by machine words.
#[must_use]
pub fn numeric_render_cost(value: &XsdValue) -> exact::Cost {
    match value {
        XsdValue::Integer { value, .. } if value.as_i128().is_none() => value.render_cost(),
        XsdValue::Decimal(decimal) if decimal_beyond_machine_words(decimal) => {
            decimal.render_cost()
        }
        _ => exact::Cost::ZERO,
    }
}

/// Whether `value` is an `xsd:integer`/`xsd:decimal` past the machine words: an
/// integer outside `i128`, or a decimal whose coefficient is outside it or whose
/// scale passes eighteen digits. Every other value computes in machine words, and
/// the costs in this module are zero for it — a caller pricing a batch checks this
/// first and skips the pricing when no value is past them.
#[must_use]
pub fn beyond_machine_words(value: &XsdValue) -> bool {
    match value {
        XsdValue::Integer { value, .. } => value.as_i128().is_none(),
        XsdValue::Decimal(decimal) => decimal_beyond_machine_words(decimal),
        _ => false,
    }
}

/// [`is_big`] for a decimal.
fn decimal_beyond_machine_words(decimal: &exact::Decimal) -> bool {
    decimal.unscaled().as_i128().is_none() || decimal.scale() > MACHINE_WORD_SCALE
}

/// The scale past which a decimal is priced as past the machine words.
const MACHINE_WORD_SCALE: u32 = 18;

/// Whether `lexical` is in the `xsd:integer` lexical space (XSD 1.1 Part 2
/// §3.4.13.1): an optional `+` or `-`, then one or more ASCII digits. No
/// whitespace is trimmed; a caller applying the datatype's `collapse` facet
/// trims first.
#[must_use]
pub fn is_integer_lexical(lexical: &str) -> bool {
    let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit())
}

/// Whether `lexical` is in the `xsd:decimal` lexical space (XSD 1.1 Part 2
/// §3.3.3.1): an optional `+` or `-`, then digits with at most one `.`, and at
/// least one digit (`.5`, `1.`, `1.5` and `12` all qualify; no exponent).
/// No whitespace is trimmed, as for [`is_integer_lexical`].
#[must_use]
pub fn is_decimal_lexical(lexical: &str) -> bool {
    let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    !(int.is_empty() && frac.is_empty())
        && int.bytes().all(|b| b.is_ascii_digit())
        && frac.bytes().all(|b| b.is_ascii_digit())
}

/// `xsd:integer`: an optional leading `+`/`-`, then one or more ASCII digits, of
/// any length. For a derived integer datatype's range check use
/// [`parse_integer_typed`].
///
/// # Errors
///
/// [`XsdError::InvalidLexical`] for a form outside the lexical space.
pub fn parse_integer(s: &str) -> Result<exact::Integer, XsdError> {
    parse_integer_typed(s, XsdDatatype::Integer)
}

/// Parse a lexical integer form for the given integer-family `datatype`, of any
/// length, hard-failing with [`XsdError::OutOfRange`] (`err:FORG0001`) when the value
/// is outside the datatype's inclusive bounds.
///
/// This is the unified entry point for all integer-family datatypes; `parse` in
/// `value.rs` routes every integer-family IRI through here.
///
/// # Errors
///
/// [`XsdError::InvalidLexical`] for a form outside the lexical space, and
/// [`XsdError::OutOfRange`] for a value outside a bounded derived type.
pub fn parse_integer_typed(
    lexical: &str,
    datatype: XsdDatatype,
) -> Result<exact::Integer, XsdError> {
    if !is_integer_lexical(lexical) {
        return Err(XsdError::invalid(
            datatype,
            lexical,
            "expected an optional sign then digits",
        ));
    }
    let value = match lexical.parse::<i128>() {
        Ok(small) => exact::Integer::from_i128(small),
        Err(_) => lexical.parse::<exact::Integer>().map_err(|_| {
            XsdError::invalid(datatype, lexical, "expected an optional sign then digits")
        })?,
    };
    if !datatype.admits_integer(&value) {
        return Err(XsdError::OutOfRange {
            datatype,
            lexical: lexical.to_string(),
            reason: crate::value::reason::OUTSIDE_DATATYPE,
        });
    }
    Ok(value)
}

/// `xsd:decimal`: an optional sign, digits with an optional single `.` (at least one
/// digit overall; `.5`, `1.`, `1.5`, `12` all valid), of any length and scale.
///
/// # Errors
///
/// [`XsdError::InvalidLexical`] for a form outside the lexical space.
pub fn parse_decimal(s: &str) -> Result<exact::Decimal, XsdError> {
    let dt = XsdDatatype::Decimal;
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (int_str, frac_str) = body.split_once('.').unwrap_or((body, ""));
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
    s.parse::<exact::Decimal>()
        .map_err(|_| XsdError::invalid(dt, s, "not an xsd:decimal lexical form"))
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
        .map_err(|_| XsdError::invalid(dt, s, "not a valid float lexical"))
}

/// XSD **1.0**-pinned `xsd:double` parse: identical to [`parse_double`] but rejects
/// the XSD 1.1-only `+INF` spelling of positive infinity. XSD 1.0 (referenced by
/// the ShEx/SHACL/SPARQL conformance suites) spells positive infinity only `INF`;
/// spec-pinned consumers call this to opt out of the 1.1-only lexical without
/// turning the whole crate back to 1.0.
pub fn parse_double_xsd10(s: &str) -> Result<f64, XsdError> {
    if s == "+INF" {
        return Err(XsdError::invalid(
            XsdDatatype::Double,
            s,
            "XSD 1.0 spells positive infinity INF",
        ));
    }
    parse_double(s)
}

/// XSD **1.0**-pinned `xsd:float` parse: identical to [`parse_float`] but rejects
/// the XSD 1.1-only `+INF` spelling of positive infinity. XSD 1.0 (referenced by
/// the ShEx/SHACL/SPARQL conformance suites) spells positive infinity only `INF`;
/// spec-pinned consumers call this to opt out of the 1.1-only lexical without
/// turning the whole crate back to 1.0.
pub fn parse_float_xsd10(s: &str) -> Result<f32, XsdError> {
    if s == "+INF" {
        return Err(XsdError::invalid(
            XsdDatatype::Float,
            s,
            "XSD 1.0 spells positive infinity INF",
        ));
    }
    parse_float(s)
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
        .map_err(|_| XsdError::invalid(dt, s, "not a valid double lexical"))
}

/// Reject lexicals Rust's float parser would accept but XSD forbids (`inf`,
/// `infinity`, `nan`, etc.): any ASCII letter other than the `e`/`E` exponent
/// marker disqualifies the form (the `INF`/`NaN` keywords are handled before here).
fn reject_non_xsd_numeric(s: &str, dt: XsdDatatype) -> Result<(), XsdError> {
    if s.bytes()
        .any(|b| b.is_ascii_alphabetic() && b != b'e' && b != b'E')
    {
        return Err(XsdError::invalid(dt, s, "non-XSD numeric token"));
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
    // One exact-fit buffer instead of two `format!` intermediates; the mantissa
    // and exponent digits come from `sci()` untouched.
    let mut out = String::with_capacity(mantissa.len() + exp.len() + 3);
    out.push_str(mantissa);
    if !mantissa.contains('.') {
        out.push_str(".0");
    }
    out.push('E');
    out.push_str(exp);
    out
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
        // Integer-vs-integer: compare by value, ignore subtype (xsd:int 5 == xsd:long 5).
        (Integer { value: x, .. }, Integer { value: y, .. }) => Some(x.cmp(y)),
        (Dec(x), Dec(y)) => Some(x.cmp(y)),
        (Integer { value: x, .. }, Dec(y)) => Some(exact_path::integer_cmp_decimal(x, y)),
        (Dec(x), Integer { value: y, .. }) => Some(exact_path::integer_cmp_decimal(y, x).reverse()),
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

/// The **exact** total order over the XSD numeric tower — the relation `ORDER BY`,
/// `DISTINCT`, `MIN` and `MAX` sort by, and a genuine total order, which
/// [`numeric_cmp`] is not.
///
/// Every value in the tower except `NaN` is exactly a rational number:
/// `xsd:integer` is `m/1`, `xsd:decimal` is `mantissa / 10^scale`, and a finite
/// `xsd:float`/`xsd:double` is a **dyadic** rational `significand × 2^exponent`
/// (that is what an IEEE binary format *is*). This function compares those
/// rationals exactly, cross-multiplying into [`crate::BigInt`] on the rare pair
/// where a `u128` cannot hold the products. `NaN` is the one input with no place
/// in the order and answers `None`; the infinities sit at the two ends.
///
/// # This deliberately diverges from SPARQL's promotion-based `<`
///
/// SPARQL 1.1/1.2 §17.3 maps a cross-type numeric comparison onto the promotion
/// lattice `integer ⊂ decimal ⊂ float ⊂ double`, which routes ANY pair containing
/// a float or double operand through `f32`/`f64` — a lossy projection.
/// [`numeric_cmp`] implements that mapping faithfully and is what this crate's
/// `<`, `>`, `=` operators keep using, because that is what the operators are
/// specified to mean. This function does NOT, and the divergence is not a liberty
/// taken for convenience:
///
/// **The two disagree in exactly the cases where the spec's own mapping is
/// intransitive.** Promotion and exactness agree whenever the promotion is
/// lossless; they can only disagree when rounding maps two *distinct* exact values
/// onto one IEEE value. But the moment it does, the promoted relation has a cycle.
/// Take `a = "1.000000000000000001"^^xsd:decimal`, `b = "1"^^xsd:integer` and
/// `c = "1.0E0"^^xsd:double`:
///
/// * `a` vs `b` is decimal-vs-integer, which promotion compares EXACTLY, so
///   `a > b`;
/// * `a` vs `c` promotes `a` to `f64`, which rounds it to `1.0`, so `a = c`;
/// * `b` vs `c` promotes `b` to `f64` exactly, so `b = c`.
///
/// From `a = c` and `b = c` a transitive relation must conclude `a = b`, and the
/// first line says otherwise. That is a genuine cycle over three ordinary literals,
/// and `ORDER BY` hands its comparator to a Rust sort, which is entitled to
/// **panic** when the comparator is not a total order — reachable from any query
/// whose sort key is an attacker-supplied literal with more than sixteen
/// significant digits. There is no way to be both transitive and promotion-faithful
/// here, so the ordering picks transitive; the exact answer is also strictly more
/// informative, since it distinguishes values the promoted relation conflates and
/// never conflates values the promoted relation distinguishes.
///
/// The divergence is confined to the ORDER. `=` and `<` still answer exactly what
/// §17.3 says, through [`numeric_cmp`].
///
/// # Examples
///
/// ```rust
/// use std::cmp::Ordering;
///
/// use purrdf_xsd::{XsdDatatype, numeric_cmp, numeric_total_cmp, parse};
///
/// let a = parse("1.000000000000000001", XsdDatatype::Decimal)?;
/// let c = parse("1.0E0", XsdDatatype::Double)?;
///
/// // Promotion rounds the decimal to 1.0 and calls the two equal.
/// assert_eq!(numeric_cmp(&a, &c), Some(Ordering::Equal));
/// // The exact order sees the nineteenth significant digit.
/// assert_eq!(numeric_total_cmp(&a, &c), Some(Ordering::Greater));
///
/// // NaN is the one value with no place in the order.
/// let nan = parse("NaN", XsdDatatype::Double)?;
/// assert_eq!(numeric_total_cmp(&nan, &c), None);
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn numeric_total_cmp(a: &XsdValue, b: &XsdValue) -> Option<Ordering> {
    use XsdValue::{Double, Float};
    match (a, b) {
        // `f32 → f64` is an exact widening (every `f32` is an `f64`), so every
        // IEEE-vs-IEEE pair is already decided without loss.
        (Float(x), Float(y)) => x.partial_cmp(y),
        (Double(x), Double(y)) => x.partial_cmp(y),
        (Float(x), Double(y)) => f64::from(*x).partial_cmp(y),
        (Double(x), Float(y)) => x.partial_cmp(&f64::from(*y)),
        // ── The exact-vs-IEEE pairs, the only lossy ones under promotion ─────
        (_, Float(y)) if a.is_exact_numeric() => exact_path::cmp_ieee(a, f64::from(*y)),
        (_, Double(y)) if a.is_exact_numeric() => exact_path::cmp_ieee(a, *y),
        (Float(x), _) if b.is_exact_numeric() => {
            exact_path::cmp_ieee(b, f64::from(*x)).map(Ordering::reverse)
        }
        (Double(x), _) if b.is_exact_numeric() => {
            exact_path::cmp_ieee(b, *x).map(Ordering::reverse)
        }
        // Integer and decimal pairs: `numeric_cmp` already decides them exactly.
        _ => numeric_cmp(a, b),
    }
}

/// The numeric value as `f64`, or `None` if `v` is not a numeric value.
fn num_f64(v: &XsdValue) -> Option<f64> {
    Some(match v {
        // Spec-mandated promotion: integer ⊂ double and decimal ⊂ double (SPARQL
        // §17.3), each correctly rounded once.
        XsdValue::Integer { value, .. } => value.to_f64(),
        XsdValue::Decimal(d) => d.to_f64(),
        XsdValue::Float(f) => f64::from(*f),
        XsdValue::Double(d) => *d,
        _ => return None,
    })
}

/// The numeric value as `f32`, or `None` if `v` is not a numeric value.
fn num_f32(v: &XsdValue) -> Option<f32> {
    Some(match v {
        // integer ⊂ float and decimal ⊂ float: the exact value rounded ONCE, straight
        // to single precision (XPath `xs:float` casting). Going through `f64` first
        // would round twice, which can land one ulp off.
        XsdValue::Integer { value, .. } => value.to_f32(),
        XsdValue::Decimal(d) => d.to_f32(),
        XsdValue::Float(f) => *f,
        // double → float narrowing: required by the numeric tower when a float operand
        // forces promotion of the other operand down (SPARQL §17.3).
        XsdValue::Double(d) => *d as f32,
        _ => return None,
    })
}

// ── Numeric arithmetic (SPARQL §17.4 / XPath op:numeric-*) ──────────────────

/// One of the operators [`binary`] dispatches.
#[derive(Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
}

impl BinOp {
    const fn mismatch(self) -> XsdError {
        XsdError::TypeMismatch {
            reason: match self {
                Self::Add => "non-numeric operand in add",
                Self::Sub => "non-numeric operand in sub",
                Self::Mul => "non-numeric operand in mul",
            },
        }
    }
}

/// `a op b` over the promotion tower: IEEE when either operand is a float or a
/// double, else an exact integer or decimal result.
fn binary(a: &XsdValue, b: &XsdValue, op: BinOp) -> Result<XsdValue, XsdError> {
    use XsdValue::{Decimal as Dec, Double, Float, Integer};
    match (a, b) {
        (Integer { value: x, .. }, Integer { value: y, .. }) => Ok(Integer {
            value: match op {
                BinOp::Add => x + y,
                BinOp::Sub => x - y,
                BinOp::Mul => x * y,
            },
            datatype: XsdDatatype::Integer,
        }),
        (Double(_), _) | (_, Double(_)) => {
            let (x, y) = (
                num_f64(a).ok_or_else(|| op.mismatch())?,
                num_f64(b).ok_or_else(|| op.mismatch())?,
            );
            Ok(Double(match op {
                BinOp::Add => ieee::f64_add(x, y),
                BinOp::Sub => ieee::f64_sub(x, y),
                BinOp::Mul => ieee::f64_mul(x, y),
            }))
        }
        (Float(_), _) | (_, Float(_)) => {
            let (x, y) = (
                num_f32(a).ok_or_else(|| op.mismatch())?,
                num_f32(b).ok_or_else(|| op.mismatch())?,
            );
            Ok(Float(match op {
                BinOp::Add => ieee::f32_add(x, y),
                BinOp::Sub => ieee::f32_sub(x, y),
                BinOp::Mul => ieee::f32_mul(x, y),
            }))
        }
        (Dec(_) | Integer { .. }, Dec(_) | Integer { .. }) => {
            exact_path::decimal_binop(a, b, op.into())
        }
        _ => Err(op.mismatch()),
    }
}

impl From<BinOp> for exact_path::Op {
    fn from(op: BinOp) -> Self {
        match op {
            BinOp::Add => Self::Add,
            BinOp::Sub => Self::Sub,
            BinOp::Mul => Self::Mul,
        }
    }
}

/// `op:numeric-add` — the numeric-TIER `+` operator only. Follows the numeric
/// promotion tower: `integer ⊂ decimal ⊂ float ⊂ double`. Integer and decimal
/// addition are exact at every size; float/double are IEEE: the sum correctly rounded once into binary32/binary64 on
/// every target, the x87 included ([`crate::ieee`]). `numeric_sub`, `numeric_mul`
/// and `numeric_div` hold the same law.
///
/// This is a narrower contract than [`crate::ops::value_add`], the SPARQL-
/// facing `+`: every call site over an `XsdValue` of unknown family should go
/// through `value_add`, which also accepts SEP-0002's temporal operand pairs;
/// call this function directly only where the operands are already known
/// numeric.
///
/// Returns `Err(TypeMismatch)` if either operand is not numeric.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, numeric_add, parse};
///
/// let a = parse("40", XsdDatatype::Integer)?;
/// let b = parse("2", XsdDatatype::Integer)?;
/// assert_eq!(numeric_add(&a, &b)?.canonical_lexical(), "42");
///
/// // Mixed integer + decimal promotes to decimal, exactly.
/// let half = parse("0.5", XsdDatatype::Decimal)?;
/// assert_eq!(numeric_add(&a, &half)?.canonical_lexical(), "40.5");
///
/// // A non-numeric operand is a type error.
/// let s = parse("oops", XsdDatatype::String)?;
/// assert!(numeric_add(&a, &s).is_err());
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn numeric_add(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    binary(a, b, BinOp::Add)
}

/// `op:numeric-subtract` — the numeric-TIER `-` operator only. Same promotion
/// tower as [`numeric_add`]; see its doc for why [`crate::ops::value_sub`],
/// not this function, is the SPARQL-facing entry point for an `XsdValue` of
/// unknown family.
///
/// # Errors
///
/// `Err(TypeMismatch)` if either operand is not numeric.
pub fn numeric_sub(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    binary(a, b, BinOp::Sub)
}

/// `op:numeric-multiply` — the numeric-TIER `*` operator only. Same promotion
/// tower as [`numeric_add`]; see its doc for why [`crate::ops::value_mul`],
/// not this function, is the SPARQL-facing entry point for an `XsdValue` of
/// unknown family (it also accepts `xsd:duration × xsd:integer|xsd:decimal`,
/// which this function does not).
///
/// Integer and decimal products are exact: a decimal product's scale is the sum of
/// its operands' scales.
///
/// # Errors
///
/// `Err(TypeMismatch)` if either operand is not numeric, and
/// `Err(Exact(ScaleOverflow))` for a decimal product whose scale would pass
/// `u32::MAX` digits.
pub fn numeric_mul(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    binary(a, b, BinOp::Mul)
}

/// `op:numeric-divide` — the numeric-TIER `/` operator only. Integer ÷ integer
/// returns **decimal** (not integer), per XPath `op:numeric-divide` semantics.
/// All other pairs follow the numeric promotion tower.
///
/// See [`numeric_add`]'s doc for why [`crate::ops::value_div`], not this
/// function, is the SPARQL-facing entry point for an `XsdValue` of unknown
/// family (it also accepts `xsd:duration ÷ xsd:integer|xsd:decimal|xsd:duration`,
/// which this function does not).
///
/// Division by zero:
/// - `xsd:integer` or `xsd:decimal` divisor = 0 → `Err(DivisionByZero)` (hard error).
/// - `xsd:float` or `xsd:double` divisor = 0.0 → IEEE result (±INF, or NaN for 0÷0).
///
/// Returns `Err(TypeMismatch)` if either operand is not numeric.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, numeric_div, parse};
///
/// // Integer ÷ integer yields DECIMAL (XPath rule), exactly.
/// let seven = parse("7", XsdDatatype::Integer)?;
/// let two = parse("2", XsdDatatype::Integer)?;
/// let q = numeric_div(&seven, &two)?;
/// assert_eq!(q.datatype(), XsdDatatype::Decimal);
/// assert_eq!(q.canonical_lexical(), "3.5");
///
/// // Exact-type division by zero is a hard error (not a saturated value).
/// let zero = parse("0", XsdDatatype::Integer)?;
/// assert!(numeric_div(&seven, &zero).is_err());
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn numeric_div(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    numeric_div_with_policy(a, b, DivisionPolicy::xsd_default())
}

/// [`numeric_div`] with a caller-chosen precision for integer and decimal
/// quotients ([`DivisionPolicy`]); `float`/`double` operands divide in IEEE
/// arithmetic.
///
/// [`numeric_div`] is this function at [`DivisionPolicy::xsd_default`]: the exact
/// quotient whenever its decimal expansion terminates, and otherwise eighteen
/// fractional digits rounded half to even.
///
/// # Errors
///
/// `Err(DivisionByZero)` for a zero integer or decimal divisor;
/// `Err(Exact(NonTerminating))` under [`DivisionPolicy::Exact`] for a quotient
/// with no finite decimal expansion; `Err(TypeMismatch)` for a non-numeric
/// operand.
///
/// ```rust
/// use purrdf_xsd::exact::DivisionPolicy;
/// use purrdf_xsd::numeric::numeric_div_with_policy;
/// use purrdf_xsd::{XsdDatatype, parse};
///
/// let one = parse("1", XsdDatatype::Integer)?;
/// let eight = parse("8", XsdDatatype::Integer)?;
/// let three = parse("3", XsdDatatype::Integer)?;
/// let exact = DivisionPolicy::Exact;
/// assert_eq!(numeric_div_with_policy(&one, &eight, exact)?.canonical_lexical(), "0.125");
/// assert!(numeric_div_with_policy(&one, &three, exact).is_err());
/// let default = DivisionPolicy::xsd_default();
/// assert_eq!(
///     numeric_div_with_policy(&one, &three, default)?.canonical_lexical(),
///     "0.333333333333333333"
/// );
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn numeric_div_with_policy(
    a: &XsdValue,
    b: &XsdValue,
    policy: DivisionPolicy,
) -> Result<XsdValue, XsdError> {
    let mismatch = || XsdError::TypeMismatch {
        reason: "non-numeric operand in div",
    };
    match (a, b) {
        (XsdValue::Double(_), _) | (_, XsdValue::Double(_)) => {
            let (x, y) = (
                num_f64(a).ok_or_else(mismatch)?,
                num_f64(b).ok_or_else(mismatch)?,
            );
            Ok(XsdValue::Double(ieee::f64_div(x, y)))
        }
        (XsdValue::Float(_), _) | (_, XsdValue::Float(_)) => {
            let (x, y) = (
                num_f32(a).ok_or_else(mismatch)?,
                num_f32(b).ok_or_else(mismatch)?,
            );
            Ok(XsdValue::Float(ieee::f32_div(x, y)))
        }
        _ if a.is_exact_numeric() && b.is_exact_numeric() => exact_path::div(a, b, policy),
        _ => Err(mismatch()),
    }
}

/// `op:numeric-unary-minus` — the numeric-TIER unary `-` only: an `xsd:integer` for
/// an integer-family operand (F&O 3.1 §4.2), otherwise the operand's type.
///
/// See [`numeric_add`]'s doc for why [`crate::ops::value_unary_minus`], not
/// this function, is the SPARQL-facing entry point for an `XsdValue` of
/// unknown family (it also accepts `xsd:duration`, a PurRDF extension this
/// function does not implement).
///
/// Integer and decimal negation is exact at every size. For float/double, IEEE
/// negation (−0.0 negates to +0.0 and vice-versa; NaN negates to NaN with sign
/// flipped per IEEE 754-2008 §6.3).
///
/// # Errors
///
/// `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_unary_minus(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { value, .. } => Ok(integer_result(-value)),
        XsdValue::Decimal(d) => Ok(XsdValue::Decimal(-d)),
        XsdValue::Float(f) => Ok(XsdValue::Float(-f)),
        XsdValue::Double(d) => Ok(XsdValue::Double(-d)),
        _ => Err(XsdError::TypeMismatch {
            reason: "unary minus applied to non-numeric value",
        }),
    }
}

/// The `xsd:integer` result of a unary operator over an integer-family operand.
///
/// F&O 3.1 §4.2 types these results as `xs:integer`, and only permits a subtype
/// when the value is in it: `-5` is not an `xsd:unsignedByte`, so keeping the
/// operand's subtype would mint a literal outside its own value space.
const fn integer_result(value: exact::Integer) -> XsdValue {
    XsdValue::Integer {
        value,
        datatype: XsdDatatype::Integer,
    }
}

/// SPARQL `op:numeric-unary-plus` (unary `+`). Identity for numeric types.
///
/// # Errors
///
/// `Err(TypeMismatch)` for non-numeric operands (e.g. `+true` is a type error in
/// SPARQL/XPath, not a no-op).
pub fn numeric_unary_plus(a: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_numeric() {
        Ok(a.clone())
    } else {
        Err(XsdError::TypeMismatch {
            reason: "unary plus applied to non-numeric value",
        })
    }
}

// ── Numeric math functions (SPARQL §17.4.4 / XPath fn:abs, fn:ceiling, etc.) ──

/// SPARQL `fn:abs` — absolute value: an `xsd:integer` for an integer-family
/// operand, otherwise the operand's numeric type.
///
/// # Errors
///
/// `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_abs(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { value, .. } => Ok(integer_result(value.abs())),
        XsdValue::Decimal(d) => Ok(XsdValue::Decimal(if d.is_negative() {
            -d
        } else {
            d.clone()
        })),
        XsdValue::Float(f) => Ok(XsdValue::Float(f.abs())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.abs())),
        _ => Err(XsdError::TypeMismatch {
            reason: "abs applied to non-numeric value",
        }),
    }
}

/// An integer-rounding function (`fn:ceiling`, `fn:floor`, `fn:round`) over an exact
/// operand: an `xsd:integer` stays itself (typed `xsd:integer`), a decimal rounds to
/// an integral decimal in direction `rounding`.
fn round_exact(a: &XsdValue, rounding: Rounding) -> Option<XsdValue> {
    match a {
        XsdValue::Integer { value, .. } => Some(integer_result(value.clone())),
        XsdValue::Decimal(d) => Some(XsdValue::Decimal(exact::Decimal::from_integer(
            d.round_to_integer(rounding),
        ))),
        _ => None,
    }
}

/// SPARQL `fn:ceiling` — smallest integer not less than the value (`integer →
/// integer`, `decimal → decimal` exact, `float/double → float/double`).
///
/// # Errors
///
/// `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_ceil(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Float(f) => Ok(XsdValue::Float(f.ceil())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.ceil())),
        _ => round_exact(a, Rounding::Ceiling).ok_or(XsdError::TypeMismatch {
            reason: "ceiling applied to non-numeric value",
        }),
    }
}

/// SPARQL `fn:floor` — largest integer not greater than the value (`integer →
/// integer`, `decimal → decimal` exact, `float/double → float/double`).
///
/// # Errors
///
/// `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_floor(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Float(f) => Ok(XsdValue::Float(f.floor())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.floor())),
        _ => round_exact(a, Rounding::Floor).ok_or(XsdError::TypeMismatch {
            reason: "floor applied to non-numeric value",
        }),
    }
}

/// SPARQL `fn:round` — round to the nearest integer, with half-values rounded
/// toward positive infinity (`fn:round` semantics per XPath §4.4.5). Preserves the
/// operand's numeric family.
///
/// Examples: `round(2.5) = 3`, `round(-2.5) = -2`, `round(2.4999) = 2`.
///
/// # Errors
///
/// `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_round(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Float(f) => {
            // f32::round() is round-half-away-from-zero; XPath fn:round is
            // round-half-toward-+infinity. For positive they agree. For negative halves
            // they differ: f32::round(-2.5) = -3 but fn:round(-2.5) = -2.
            let r = if *f == ieee::f32_add(f.floor(), 0.5) && *f < 0.0 {
                f.ceil()
            } else {
                f.round()
            };
            Ok(XsdValue::Float(r))
        }
        XsdValue::Double(d) => {
            let r = if *d == ieee::f64_add(d.floor(), 0.5) && *d < 0.0 {
                d.ceil()
            } else {
                d.round()
            };
            Ok(XsdValue::Double(r))
        }
        _ => round_exact(a, Rounding::HalfCeiling).ok_or(XsdError::TypeMismatch {
            reason: "round applied to non-numeric value",
        }),
    }
}

#[cfg(test)]
mod tests;
