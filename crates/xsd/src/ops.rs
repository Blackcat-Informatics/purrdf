// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The value-space operator surface: SPARQL `=` / `<` and the effective boolean
//! value. These are **value-space** operations (distinct from RDF term identity —
//! see the crate docs). They are free functions, not trait impls, so they cannot be
//! confused with the structural `Eq`/`Ord` a `HashMap`/`BTreeMap` would use.

use std::cmp::Ordering;

use crate::numeric::{
    self, Decimal, numeric_add, numeric_cmp, numeric_div, numeric_mul, numeric_sub,
    numeric_unary_minus,
};
use crate::temporal::{self, duration_equal};
use crate::value::{XsdError, XsdValue};

/// SPARQL value-space comparison (`<` / `>` / `=` semantics).
///
/// Returns `None` when the two values are **incomparable** — a `NaN` operand, or two
/// values from different value-space families (e.g. a number vs a string). The
/// evaluator maps `None` to a SPARQL *type error* for the relational operators; it
/// must NOT be read as "not equal".
///
/// Integer-family subtypes (xsd:byte, xsd:long, xsd:unsignedInt, etc.) are in the
/// same numeric tower as xsd:integer — `xsd:int 5 = xsd:long 5` is `true`.
///
/// # Examples
///
/// ```rust
/// use std::cmp::Ordering;
///
/// use purrdf_xsd::{XsdDatatype, parse, value_cmp};
///
/// let int = parse("42", XsdDatatype::Integer)?;
/// let dec = parse("42.0", XsdDatatype::Decimal)?;
/// let byte = parse("5", XsdDatatype::Byte)?;
///
/// // Numeric promotion: cross-type comparison inside the numeric tower.
/// assert_eq!(value_cmp(&int, &dec), Some(Ordering::Equal));
/// assert_eq!(value_cmp(&byte, &int), Some(Ordering::Less));
///
/// // Different value-space families are incomparable — a SPARQL type error,
/// // NOT "not equal".
/// let s = parse("42", XsdDatatype::String)?;
/// assert_eq!(value_cmp(&int, &s), None);
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn value_cmp(a: &XsdValue, b: &XsdValue) -> Option<Ordering> {
    use XsdValue::{Binary, Boolean, Double, Float, Gregorian, Integer, String as Str};
    match (a, b) {
        // Numeric tower (with promotion); covers every numeric/numeric pair,
        // including all integer-family subtypes (they share the Integer variant).
        (
            Integer { .. } | XsdValue::Decimal(_) | Float(_) | Double(_),
            Integer { .. } | XsdValue::Decimal(_) | Float(_) | Double(_),
        ) => numeric_cmp(a, b),
        // `false` < `true`.
        (Boolean(x), Boolean(y)) => Some(x.cmp(y)),
        // Codepoint (Unicode scalar) order — SPARQL string ordering.
        (Str(x), Str(y)) => Some(x.cmp(y)),
        // Temporal families compare within themselves (XSD partial order).
        (XsdValue::DateTime(x), XsdValue::DateTime(y)) => temporal::cmp_datetime(x, y),
        (XsdValue::Date(x), XsdValue::Date(y)) => temporal::cmp_date(x, y),
        (XsdValue::Time(x), XsdValue::Time(y)) => temporal::cmp_time(x, y),
        (XsdValue::Duration(x), XsdValue::Duration(y)) => temporal::cmp_duration(x, y),
        // Gregorian family: same-type comparison, cross-type incomparable.
        (Gregorian(x), Gregorian(y)) => temporal::cmp_gregorian(x, y),
        // Binary value spaces: same datatype → byte-lexicographic order; different datatypes
        // are INCOMPARABLE even if the byte sequences coincide. xsd:hexBinary and
        // xsd:base64Binary are distinct value spaces in the XSD spec.
        //
        // Note on relational operators: SPARQL defines `=`/`!=` on binary operands but
        // NOT relational `<`/`>`/`<=`/`>=`. We return a deterministic byte-lexicographic
        // order here so that equality is exact and `ORDER BY` is well-defined; a
        // downstream SPARQL evaluator that needs spec-strictness may treat `<` on binary
        // as a type error at the operator layer (above this function).
        (
            Binary {
                bytes: x,
                datatype: dx,
            },
            Binary {
                bytes: y,
                datatype: dy,
            },
        ) => {
            if dx != dy {
                // Different value spaces (hexBinary vs base64Binary) → incomparable.
                None
            } else {
                // Same value space → byte-lexicographic order.
                Some(x.cmp(y))
            }
        }
        // Different value-space families are incomparable.
        _ => None,
    }
}

/// SPARQL value-space equality test (`Option<bool>` form).
///
/// For every pair except `xsd:duration`/`xsd:duration`, this is
/// `value_cmp(a, b).map(|o| o == Ordering::Equal)` — equality read off the
/// partial order, `None` for incomparable operands. **Duration pairs are the
/// one exception**: XPath F&O's `op:duration-equal` is total over the general
/// `xs:duration` (see [`crate::temporal::duration_equal`] for the full
/// argument), so `value_equal` never returns `None` for two durations, even
/// when [`value_cmp`] returns `None` for the same pair because `<`/`>` are only
/// defined for the two named subtypes.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, value_cmp, value_equal};
///
/// let p1m = parse("P1M", XsdDatatype::YearMonthDuration)?;
/// let p30d = parse("P30D", XsdDatatype::DayTimeDuration)?;
///
/// // Total: duration equality always has an answer.
/// assert_eq!(value_equal(&p1m, &p30d), Some(false));
/// // Partial: value_cmp has no defined order for incommensurable durations.
/// assert_eq!(value_cmp(&p1m, &p30d), None);
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn value_equal(a: &XsdValue, b: &XsdValue) -> Option<bool> {
    match (a, b) {
        (XsdValue::Duration(x), XsdValue::Duration(y)) => Some(duration_equal(x, y)),
        _ => value_cmp(a, b).map(|o| o == Ordering::Equal),
    }
}

/// SPARQL value-space equality (`=`), as a plain `bool`. Built on [`value_equal`],
/// which is total for durations and otherwise reads equality off [`value_cmp`]'s
/// partial order; incomparable operands (of either kind — indeterminate duration
/// order or cross-family mismatch) collapse to `false` here. When the
/// error-vs-false distinction matters (the SPARQL `=` operator raises a type
/// error on incomparable non-duration operands), use [`value_equal`] or
/// [`value_cmp`] directly and treat `None` as the error.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, value_eq};
///
/// let a = parse("1", XsdDatatype::Integer)?;
/// let b = parse("1.0", XsdDatatype::Decimal)?;
/// // One value, two datatypes: value-space equality holds.
/// assert!(value_eq(&a, &b));
///
/// let c = parse("2", XsdDatatype::Integer)?;
/// assert!(!value_eq(&a, &c));
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn value_eq(a: &XsdValue, b: &XsdValue) -> bool {
    value_equal(a, b) == Some(true)
}

/// SPARQL Effective Boolean Value (value-space rules).
///
/// `None` means **type error** (the value has no EBV — the evaluator raises), which
/// is distinct from `Some(false)`. A consumer must never read `None` as `false`.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, effective_boolean_value, parse};
///
/// let nonempty = parse("hi", XsdDatatype::String)?;
/// assert_eq!(effective_boolean_value(&nonempty), Some(true));
///
/// let zero = parse("0", XsdDatatype::Integer)?;
/// assert_eq!(effective_boolean_value(&zero), Some(false));
///
/// // Temporal values have NO effective boolean value: `None`, not `false`.
/// let date = parse("2024-01-01", XsdDatatype::Date)?;
/// assert_eq!(effective_boolean_value(&date), None);
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn effective_boolean_value(v: &XsdValue) -> Option<bool> {
    Some(match v {
        XsdValue::Boolean(b) => *b,
        XsdValue::String(s) => !s.is_empty(),
        XsdValue::Integer { value, .. } => *value != 0,
        XsdValue::Decimal(d) => d.mantissa() != 0,
        XsdValue::Float(f) => !f.is_nan() && *f != 0.0,
        XsdValue::Double(d) => !d.is_nan() && *d != 0.0,
        // Temporal values have no effective boolean value (SPARQL type error).
        _ => return None,
    })
}

// ── The value-space operator algebra (SEP-0002: `+` `-` `*` `/` unary `-`) ───────
//
// `value_cmp` above is pure family dispatch with the math delegated; these five
// functions are its arithmetic counterpart. Numeric operands delegate to
// `crate::numeric`'s promotion-tower operators unchanged; temporal operands
// (`dateTime`/`date`/`time`/`duration`/the Gregorian family) delegate to the
// primitives `crate::temporal` already ships. `temporal.rs` itself stays
// decoupled from `XsdValue` — it imports only `XsdDatatype`/`Decimal`/`XsdError`
// — so all family dispatch lives here, in one place, rather than leaking into
// the value-space-agnostic temporal module.

/// Extract an *exact* (`xsd:integer`/`xsd:decimal`) scalar factor for duration
/// scaling (`value_mul`/`value_div`'s `DUR × Nx` / `DUR ÷ Nx` rows).
/// `xsd:float`/`xsd:double` are refused — matching RDF4J's `isDecimalDatatype`
/// gate — because an inexact binary factor cannot scale an exact `xsd:duration`
/// without silent rounding, which this crate's no-silent-truncation rule
/// forbids.
fn exact_factor(v: &XsdValue) -> Result<Decimal, XsdError> {
    match v {
        XsdValue::Integer { value, .. } => Ok(numeric::integer_to_decimal(*value)),
        XsdValue::Decimal(d) => Ok(*d),
        _ => Err(XsdError::TypeMismatch {
            reason: "duration scale factor must be xsd:integer or xsd:decimal (exact)",
        }),
    }
}

/// Temporal dispatch for `value_add`: instant/Gregorian `+` duration (either
/// operand order) and duration `+` duration. Not exhaustive — reached only for
/// operand pairs `value_add` has already classified as temporal; every pair
/// this table does not list falls through to the final `TypeMismatch` arm.
fn temporal_add(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    match (a, b) {
        (XsdValue::DateTime(dt), XsdValue::Duration(d))
        | (XsdValue::Duration(d), XsdValue::DateTime(dt)) => {
            temporal::add_duration_to_datetime(dt, d).map(XsdValue::DateTime)
        }
        (XsdValue::Date(dt), XsdValue::Duration(d))
        | (XsdValue::Duration(d), XsdValue::Date(dt)) => {
            temporal::add_duration_to_date(dt, d).map(XsdValue::Date)
        }
        (XsdValue::Time(t), XsdValue::Duration(d)) | (XsdValue::Duration(d), XsdValue::Time(t)) => {
            temporal::add_duration_to_time(t, d).map(XsdValue::Time)
        }
        (XsdValue::Gregorian(g), XsdValue::Duration(d))
        | (XsdValue::Duration(d), XsdValue::Gregorian(g)) => {
            temporal::add_duration_to_gregorian(g, d).map(XsdValue::Gregorian)
        }
        (XsdValue::Duration(x), XsdValue::Duration(y)) => {
            temporal::add_durations(x, y).map(XsdValue::Duration)
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "add: unsupported temporal operand pair",
        }),
    }
}

/// Temporal dispatch for `value_sub`: instant `-` instant (same type only, →
/// `dayTimeDuration`), instant/Gregorian `-` duration, and duration `-`
/// duration. **No commuted rows** — `duration - instant` is meaningless and
/// falls through to the final `TypeMismatch` arm, same as any other pair this
/// table does not list.
fn temporal_sub(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    match (a, b) {
        (XsdValue::DateTime(x), XsdValue::DateTime(y)) => {
            temporal::subtract_datetimes(x, y).map(XsdValue::Duration)
        }
        (XsdValue::Date(x), XsdValue::Date(y)) => {
            temporal::subtract_dates(x, y).map(XsdValue::Duration)
        }
        (XsdValue::Time(x), XsdValue::Time(y)) => {
            temporal::subtract_times(x, y).map(XsdValue::Duration)
        }
        (XsdValue::DateTime(dt), XsdValue::Duration(d)) => {
            temporal::subtract_duration_from_datetime(dt, d).map(XsdValue::DateTime)
        }
        (XsdValue::Date(dt), XsdValue::Duration(d)) => {
            temporal::subtract_duration_from_date(dt, d).map(XsdValue::Date)
        }
        (XsdValue::Time(t), XsdValue::Duration(d)) => {
            temporal::subtract_duration_from_time(t, d).map(XsdValue::Time)
        }
        (XsdValue::Gregorian(g), XsdValue::Duration(d)) => {
            temporal::subtract_duration_from_gregorian(g, d).map(XsdValue::Gregorian)
        }
        (XsdValue::Duration(x), XsdValue::Duration(y)) => {
            temporal::subtract_durations(x, y).map(XsdValue::Duration)
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "sub: unsupported temporal operand pair",
        }),
    }
}

/// Temporal dispatch for `value_mul`, reached whenever at least one operand is
/// a `Duration` (from either of `value_mul`'s two discriminant checks): `DUR ×
/// Nx` in either operand order, scaling both duration components through
/// [`exact_factor`]. `DUR × DUR` and `DUR × Float`/`Double` both fail inside
/// `exact_factor` (a `Duration` or an inexact factor is never `Integer`/
/// `Decimal`), so they need no separate arm here.
fn temporal_mul(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    match (a, b) {
        (XsdValue::Duration(d), other) | (other, XsdValue::Duration(d)) => {
            let factor = exact_factor(other)?;
            temporal::multiply_duration(d, &factor).map(XsdValue::Duration)
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "multiply: unsupported temporal operand pair",
        }),
    }
}

/// Temporal dispatch for `value_div`, reached only when the left operand `a`
/// is a `Duration` (division has no commuted row: `Nx ÷ DUR` is a type error,
/// handled by `numeric_div`'s own catch-all before this function is ever
/// called). `DUR ÷ Nx` scales through [`exact_factor`]; `DUR ÷ DUR` goes
/// through [`temporal::divide_durations`], which is commensurability- (not
/// tag-) gated and reports `XsdError::Indeterminate` for an incommensurable
/// pair.
fn temporal_div(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    let XsdValue::Duration(dividend) = a else {
        unreachable!("temporal_div is only reached with a Duration left operand")
    };
    match b {
        XsdValue::Integer { .. } | XsdValue::Decimal(_) => {
            let factor = exact_factor(b)?;
            temporal::divide_duration(dividend, &factor).map(XsdValue::Duration)
        }
        XsdValue::Duration(divisor) => {
            temporal::divide_durations(dividend, divisor).map(XsdValue::Decimal)
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "divide: duration divisor must be xsd:integer, xsd:decimal, or another duration",
        }),
    }
}

/// SPARQL value-space addition (`+`, `op:numeric-add` / SEP-0002's temporal
/// rows combined). Numeric operands (`xsd:integer`/`decimal`/`float`/`double`)
/// follow [`crate::numeric::numeric_add`]'s promotion tower; temporal operands
/// (`dateTime`/`date`/`time`/the Gregorian family plus a duration, in either
/// order, or duration `+` duration) delegate to [`crate::temporal`].
/// `xsd:boolean`/`xsd:string`/binary operands have no arithmetic value.
///
/// Exhaustive over all 12 [`XsdValue`] variants — no wildcard arm — so a
/// variant added to the enum in the future is a compile error here rather than
/// silently absorbed into the wrong branch.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, value_add};
///
/// let a = parse("40", XsdDatatype::Integer)?;
/// let b = parse("2", XsdDatatype::Integer)?;
/// assert_eq!(value_add(&a, &b)?.canonical_lexical(), "42");
///
/// // The SEP-0002 temporal surface: a whole-day duration added to a date.
/// let d = parse("2024-01-31", XsdDatatype::Date)?;
/// let one_day = parse("P1D", XsdDatatype::DayTimeDuration)?;
/// assert_eq!(value_add(&d, &one_day)?.canonical_lexical(), "2024-02-01");
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn value_add(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { .. }
        | XsdValue::Decimal(_)
        | XsdValue::Float(_)
        | XsdValue::Double(_) => numeric_add(a, b),
        XsdValue::DateTime(_)
        | XsdValue::Date(_)
        | XsdValue::Time(_)
        | XsdValue::Duration(_)
        | XsdValue::Gregorian(_) => temporal_add(a, b),
        XsdValue::Boolean(_) | XsdValue::String(_) | XsdValue::Binary { .. } => {
            Err(XsdError::TypeMismatch {
                reason: "add: operand has no XSD arithmetic value",
            })
        }
    }
}

/// SPARQL value-space subtraction (`-`, `op:numeric-subtract` / SEP-0002's
/// temporal rows combined). Numeric operands follow
/// [`crate::numeric::numeric_sub`]'s promotion tower; temporal operands cover
/// instant `-` instant (same type only, → `dayTimeDuration`), instant/Gregorian
/// `-` duration, and duration `-` duration — **no commuted temporal row**:
/// `duration - instant` is a type error, unlike `value_add`'s commuted rows.
///
/// Exhaustive over all 12 [`XsdValue`] variants — no wildcard arm.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, value_sub};
///
/// let a = parse("2002-01-02T10:00:00", XsdDatatype::DateTime)?;
/// let b = parse("2001-01-01T10:00:00", XsdDatatype::DateTime)?;
/// let diff = value_sub(&a, &b)?;
/// assert_eq!(diff.datatype(), XsdDatatype::DayTimeDuration);
/// assert_eq!(diff.canonical_lexical(), "P366D");
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn value_sub(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { .. }
        | XsdValue::Decimal(_)
        | XsdValue::Float(_)
        | XsdValue::Double(_) => numeric_sub(a, b),
        XsdValue::DateTime(_)
        | XsdValue::Date(_)
        | XsdValue::Time(_)
        | XsdValue::Duration(_)
        | XsdValue::Gregorian(_) => temporal_sub(a, b),
        XsdValue::Boolean(_) | XsdValue::String(_) | XsdValue::Binary { .. } => {
            Err(XsdError::TypeMismatch {
                reason: "sub: operand has no XSD arithmetic value",
            })
        }
    }
}

/// SPARQL value-space multiplication (`*`, `op:numeric-multiply` / SEP-0002's
/// temporal rows combined). Numeric `×` numeric follows
/// [`crate::numeric::numeric_mul`]'s promotion tower; `xsd:duration × xsd:integer
/// | xsd:decimal` is accepted **in either operand order**, scaling both
/// duration components exactly. `duration × float | double` and
/// `duration × duration` are type errors (the exact-tier scale-factor check
/// this module applies internally).
///
/// Unlike `value_add`/`value_sub`/`value_div`, this dispatcher needs **two**
/// discriminants, not one: `Integer × Duration` is a numeric LEFT operand
/// inside a temporal domain (the commuted form of `Duration × Integer`), so a
/// single `match a` would send it to `numeric_mul`'s catch-all and wrongly
/// reject a valid expression. The other three operators have no such commuted
/// numeric-into-temporal row and stay single-discriminant.
///
/// Exhaustive over all 12 [`XsdValue`] variants — no wildcard arm.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, value_mul};
///
/// let a = parse("6", XsdDatatype::Integer)?;
/// let b = parse("7", XsdDatatype::Integer)?;
/// assert_eq!(value_mul(&a, &b)?.canonical_lexical(), "42");
///
/// // The two-discriminant case: a numeric LEFT operand times a duration.
/// let three = parse("3", XsdDatatype::Integer)?;
/// let one_day = parse("P1D", XsdDatatype::DayTimeDuration)?;
/// assert_eq!(value_mul(&three, &one_day)?.canonical_lexical(), "P3D");
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn value_mul(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { .. }
        | XsdValue::Decimal(_)
        | XsdValue::Float(_)
        | XsdValue::Double(_) => {
            if matches!(b, XsdValue::Duration(_)) {
                temporal_mul(a, b)
            } else {
                numeric_mul(a, b)
            }
        }
        XsdValue::Duration(_) => temporal_mul(a, b),
        XsdValue::DateTime(_) | XsdValue::Date(_) | XsdValue::Time(_) | XsdValue::Gregorian(_) => {
            Err(XsdError::TypeMismatch {
                reason: "multiply: instant/Gregorian operands have no product",
            })
        }
        XsdValue::Boolean(_) | XsdValue::String(_) | XsdValue::Binary { .. } => {
            Err(XsdError::TypeMismatch {
                reason: "multiply: operand has no XSD arithmetic value",
            })
        }
    }
}

/// SPARQL value-space division (`/`, `op:numeric-divide` / SEP-0002's temporal
/// rows combined). Numeric `÷` numeric follows
/// [`crate::numeric::numeric_div`]'s promotion tower (integer `÷` integer
/// yields `xsd:decimal`, per XPath); `xsd:duration ÷ xsd:integer | xsd:decimal`
/// scales the duration, and `xsd:duration ÷ xsd:duration` yields an
/// `xsd:decimal` ratio for commensurable operands
/// ([`crate::temporal::divide_durations`]). **Division is not commutative**:
/// `numeric ÷ duration` is a type error (unlike `value_mul`), so this
/// dispatcher needs only one discriminant.
///
/// Exhaustive over all 12 [`XsdValue`] variants — no wildcard arm.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, value_div};
///
/// let seven = parse("7", XsdDatatype::Integer)?;
/// let two = parse("2", XsdDatatype::Integer)?;
/// assert_eq!(value_div(&seven, &two)?.canonical_lexical(), "3.5");
///
/// // Duration ÷ duration, commensurable (both purely day-shaped): xsd:decimal.
/// let thirty_days = parse("P30D", XsdDatatype::Duration)?;
/// let one_day = parse("P1D", XsdDatatype::DayTimeDuration)?;
/// let ratio = value_div(&thirty_days, &one_day)?;
/// assert_eq!(ratio.datatype(), XsdDatatype::Decimal);
/// assert_eq!(ratio.canonical_lexical(), "30");
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn value_div(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { .. }
        | XsdValue::Decimal(_)
        | XsdValue::Float(_)
        | XsdValue::Double(_) => numeric_div(a, b),
        XsdValue::Duration(_) => temporal_div(a, b),
        XsdValue::DateTime(_) | XsdValue::Date(_) | XsdValue::Time(_) | XsdValue::Gregorian(_) => {
            Err(XsdError::TypeMismatch {
                reason: "divide: instant/Gregorian operands have no quotient",
            })
        }
        XsdValue::Boolean(_) | XsdValue::String(_) | XsdValue::Binary { .. } => {
            Err(XsdError::TypeMismatch {
                reason: "divide: operand has no XSD arithmetic value",
            })
        }
    }
}

/// SPARQL value-space unary minus (`-x`, `op:numeric-unary-minus` extended to
/// `xsd:duration`). Numeric operands follow
/// [`crate::numeric::numeric_unary_minus`]; a duration negates both components
/// together through [`crate::temporal::negate_duration`]. **This is a purrdf
/// extension** — F&O's unary minus is numeric-only (§4.2.8) and defines no
/// duration form; unary plus stays numeric-only in this crate too, so
/// `+(?duration)` is a type error while `-(?duration)` is not.
///
/// Exhaustive over all 12 [`XsdValue`] variants — no wildcard arm.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, value_unary_minus};
///
/// let five = parse("5", XsdDatatype::Integer)?;
/// assert_eq!(value_unary_minus(&five)?.canonical_lexical(), "-5");
///
/// let d = parse("P1Y2M", XsdDatatype::YearMonthDuration)?;
/// assert_eq!(value_unary_minus(&d)?.canonical_lexical(), "-P1Y2M");
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn value_unary_minus(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { .. }
        | XsdValue::Decimal(_)
        | XsdValue::Float(_)
        | XsdValue::Double(_) => numeric_unary_minus(a),
        XsdValue::Duration(d) => temporal::negate_duration(d).map(XsdValue::Duration),
        XsdValue::DateTime(_)
        | XsdValue::Date(_)
        | XsdValue::Time(_)
        | XsdValue::Gregorian(_)
        | XsdValue::Boolean(_)
        | XsdValue::String(_)
        | XsdValue::Binary { .. } => Err(XsdError::TypeMismatch {
            reason: "unary minus: operand has no XSD arithmetic value",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::XsdDatatype::{
        Boolean, Byte, Date, DateTime, DayTimeDuration, Decimal, Double, Duration, Float, GDay,
        GMonth, GMonthDay, GYear, GYearMonth, HexBinary, Int, Integer, Long, Short, String, Time,
        UnsignedByte, YearMonthDuration,
    };
    use crate::value::parse;

    fn v(lex: &str, dt: crate::XsdDatatype) -> XsdValue {
        parse(lex, dt).unwrap()
    }

    /// The SPARQL operator-mapping table over the numeric tower, strings,
    /// booleans, and the temporal families. Each row below the numeric/string/
    /// boolean baseline is one of SEP-0002's 11 comparison rows (1–4:
    /// `yearMonthDuration`/`dayTimeDuration` `<`/`>`; 6–8: `date` `=`/`<`/`>`;
    /// 14–16: `time` `=`/`<`/`>`; plus the cross-subtype duration row showing
    /// `<`/`>` are genuinely undefined for `P1M` vs `P30D`), all asserting
    /// `value_cmp`. SEP-0002 row 5 (`duration =`) is the one exception: `=` is
    /// total over durations while `value_cmp` stays a partial order for them
    /// (see `duration_equality_is_total_while_ordering_is_partial`, below), so
    /// it is asserted separately via `value_equal` right after the loop, at the same
    /// `P1M`/`P30D` witness pair the indeterminate `value_cmp` row above uses —
    /// together the two rows are the "total-vs-partial" split, in one place.
    #[test]
    fn operator_mapping_table() {
        let eq = Some(Ordering::Equal);
        let lt = Some(Ordering::Less);
        let gt = Some(Ordering::Greater);

        // (lhs_lex, lhs_dt, rhs_lex, rhs_dt, expected value_cmp)
        let rows = [
            ("1", Integer, "1", Integer, eq),
            ("1", Integer, "1.0", Decimal, eq), // promotion
            ("1", Integer, "2", Integer, lt),
            ("2.5", Decimal, "2", Integer, gt),
            ("1", Integer, "1.0E0", Double, eq), // promotion to double
            ("1.5", Decimal, "1.25", Float, gt), // promotion to float
            ("3", Integer, "2.9", Double, gt),
            ("abc", String, "abd", String, lt), // codepoint order
            ("abc", String, "abc", String, eq),
            ("false", Boolean, "true", Boolean, lt),
            ("true", Boolean, "true", Boolean, eq),
            // Cross-family: incomparable.
            ("1", Integer, "1", String, None),
            ("true", Boolean, "1", Integer, None),
            // Cross-subtype integer equality.
            ("5", Int, "5", Long, eq),
            ("5", Byte, "5", Integer, eq),
            ("3", Short, "4", Int, lt),
            ("2", UnsignedByte, "2.5", Double, lt),
            // SEP-0002 rows 1-4: yearMonthDuration / dayTimeDuration `<`/`>`.
            ("P1Y", YearMonthDuration, "P1Y1M", YearMonthDuration, lt),
            ("P1Y1M", YearMonthDuration, "P1Y", YearMonthDuration, gt),
            ("P1D", DayTimeDuration, "P2D", DayTimeDuration, lt),
            ("P2D", DayTimeDuration, "P1D", DayTimeDuration, gt),
            // SEP-0002 rows 6-8: `date` `=`/`<`/`>` at the §6.5 witness.
            ("2024-01-31", Date, "2024-01-31", Date, eq),
            ("2024-01-30", Date, "2024-01-31", Date, lt),
            ("2024-01-31", Date, "2024-01-30", Date, gt),
            // SEP-0002 rows 14-16: `time` `=`/`<`/`>` at the §6.5 witness.
            ("12:00:00", Time, "12:00:00", Time, eq),
            ("11:00:00", Time, "12:00:00", Time, lt),
            ("12:00:00", Time, "11:00:00", Time, gt),
            // Beside SEP-0002 row 5 (asserted via `value_equal` below, not
            // `value_cmp`): the same P1M/P30D pair has NO defined `value_cmp`
            // order — cross-subtype durations are value-incommensurable.
            ("P1M", YearMonthDuration, "P30D", DayTimeDuration, None),
        ];
        for (la, da, lb, db, want) in rows {
            assert_eq!(
                value_cmp(&v(la, da), &v(lb, db)),
                want,
                "value_cmp({la:?}^^{da:?}, {lb:?}^^{db:?})"
            );
        }

        // SEP-0002 row 5: `duration =` is total (op:duration-equal), unlike the
        // `value_cmp`-based row directly above over the same witness pair.
        assert_eq!(
            value_equal(&v("P1M", YearMonthDuration), &v("P30D", DayTimeDuration)),
            Some(false)
        );
    }

    #[test]
    fn value_eq_incomparable_is_false_not_error() {
        assert!(value_eq(&v("1", Integer), &v("1.0", Decimal)));
        assert!(!value_eq(&v("1", Integer), &v("1", String)));
        // NaN: not equal, and value_cmp distinguishes the type-error (None).
        let nan = v("NaN", Double);
        assert!(!value_eq(&nan, &nan));
        assert_eq!(value_cmp(&nan, &nan), None);
        // Duration: op:duration-equal is total, so `false`, not an error —
        // even though value_cmp has no order for this pair (see the test below).
        let p1m = v("P1M", YearMonthDuration);
        let p30d = v("P30D", DayTimeDuration);
        assert!(!value_eq(&p1m, &p30d));
        assert_eq!(value_cmp(&p1m, &p30d), None);
    }

    /// `duration_equal`/`value_equal` are total; `cmp_duration`/`value_cmp` are only
    /// a partial order. This is the artifact that pins the split side by side, so a
    /// regression that re-derives `=` from `value_cmp` (collapsing the two) fails
    /// here rather than silently reintroducing the type error op:duration-equal
    /// forbids.
    #[test]
    fn duration_equality_is_total_while_ordering_is_partial() {
        use crate::XsdDatatype::{DayTimeDuration, YearMonthDuration};
        use crate::temporal::{cmp_duration, duration_equal};

        let p1m = v("P1M", YearMonthDuration);
        let p30d = v("P30D", DayTimeDuration);
        let (XsdValue::Duration(p1m_d), XsdValue::Duration(p30d_d)) = (&p1m, &p30d) else {
            unreachable!("parsed as Duration")
        };
        assert!(!duration_equal(p1m_d, p30d_d));
        assert_eq!(cmp_duration(p1m_d, p30d_d), None);

        // Zero is one value in both subtypes' value spaces.
        let ym_zero = v("P0M", YearMonthDuration);
        let dt_zero = v("PT0S", DayTimeDuration);
        let (XsdValue::Duration(ym_zero_d), XsdValue::Duration(dt_zero_d)) = (&ym_zero, &dt_zero)
        else {
            unreachable!("parsed as Duration")
        };
        assert!(duration_equal(ym_zero_d, dt_zero_d));
    }

    /// `value_equal` at the surface `ops.rs` presents to `value_eq`: total for
    /// durations, a passthrough of `value_cmp` for everything else.
    #[test]
    fn value_equal_is_total_for_durations_and_a_cmp_passthrough_otherwise() {
        use crate::XsdDatatype::{DayTimeDuration, YearMonthDuration};

        // Duration/Duration: total.
        let p1m = v("P1M", YearMonthDuration);
        let p30d = v("P30D", DayTimeDuration);
        assert_eq!(value_equal(&p1m, &p30d), Some(false));
        let ym_zero = v("P0M", YearMonthDuration);
        let dt_zero = v("PT0S", DayTimeDuration);
        assert_eq!(value_equal(&ym_zero, &dt_zero), Some(true));

        // Non-duration incomparable pair: unchanged passthrough of value_cmp (None).
        assert_eq!(value_equal(&v("1", Integer), &v("1", String)), None);

        // Equal numeric cross-type pair: passthrough of value_cmp (Some(true)).
        assert_eq!(
            value_equal(&v("1", Integer), &v("1.0", Decimal)),
            Some(true)
        );
    }

    #[test]
    fn effective_boolean_values() {
        assert_eq!(effective_boolean_value(&v("true", Boolean)), Some(true));
        assert_eq!(effective_boolean_value(&v("0", Boolean)), Some(false));
        assert_eq!(effective_boolean_value(&v("", String)), Some(false));
        assert_eq!(effective_boolean_value(&v("x", String)), Some(true));
        assert_eq!(effective_boolean_value(&v("0", Integer)), Some(false));
        assert_eq!(effective_boolean_value(&v("5", Integer)), Some(true));
        assert_eq!(effective_boolean_value(&v("NaN", Double)), Some(false));
        // Derived integer EBV: non-zero byte is true.
        assert_eq!(effective_boolean_value(&v("0", Byte)), Some(false));
        assert_eq!(effective_boolean_value(&v("1", Byte)), Some(true));
    }

    // ── The witness-classified value_* operator table (SEP-0002 §6.5) ────────────

    /// One operator: `value_add`/`value_sub`/`value_mul`/`value_div`, or unary
    /// `value_unary_minus`. Local to the test module — the production dispatch
    /// in `value_mul` doesn't need a reified operator, but the exhaustive sweep
    /// below does, to drive all five entry points from one loop.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Op {
        Add,
        Sub,
        Mul,
        Div,
        Neg,
    }

    /// The 18-datatype universe (`N ∪ instant ∪ DUR ∪ G ∪ {boolean, string,
    /// hexBinary}`) and its one canonical witness lexical per datatype — a
    /// hand-enumerated literal, not derived from `value_add`/`value_sub`/
    /// `value_mul`/`value_div`'s own dispatch logic, so the test stays
    /// independent of the implementation it checks: acceptance is
    /// value-dependent (`gYear + P1Y` succeeds, `gYear + P1D` does not under
    /// `SecondAction::Absent`), so a `(left, op, right)` triple alone cannot
    /// classify a family and the witness choice IS the classification. The
    /// duration witnesses are whole-day forms (`P1D`, not `PT1H`) so a sub-day
    /// remainder never confounds a Gregorian row with the classifier's
    /// independent sub-day-remainder rule.
    const UNIVERSE: &[(crate::XsdDatatype, &str)] = &[
        (Integer, "3"),
        (Decimal, "3.5"),
        (Float, "3.5"),
        (Double, "3.5"),
        (DateTime, "2024-01-31T12:00:00Z"),
        (Date, "2024-01-31"),
        (Time, "12:00:00"),
        (Duration, "P1Y1M1D"),
        (DayTimeDuration, "P1D"),
        (YearMonthDuration, "P1Y1M"),
        (GYearMonth, "2024-01"),
        (GYear, "2024"),
        (GMonth, "--01"),
        (GMonthDay, "--01-15"),
        (GDay, "---15"),
        (Boolean, "true"),
        (String, "abc"),
        (HexBinary, "0F"),
    ];

    /// The full SEP-0002 arithmetic table, evaluated at [`UNIVERSE`]'s
    /// witnesses: every `(left, op, right, result)` triple this crate accepts,
    /// as a literal array sorted by [`UNIVERSE`]'s own declaration order (left,
    /// then right, then operator) — NOT derived from the implementation's
    /// output. [`sep0002_operator_table`] builds the full 18×4×18 cross product
    /// (plus 18 unary pairs) itself and asserts every triple ABSENT here is
    /// `Err`, so this array's membership is what fixes the row set, not the
    /// other way around.
    ///
    /// Three blocks are witness-dependent in a way that is easy to get wrong by
    /// generalizing from the family tables alone (recorded here so nobody
    /// "corrects" this array to match a family-level intuition instead of the
    /// witnesses): `duration ÷ duration` only accepts the same-shape pairs
    /// (`dayTimeDuration ÷ dayTimeDuration`, `yearMonthDuration ÷
    /// yearMonthDuration`) — every pair touching the general (`Shape::Mixed`)
    /// witness, including `duration ÷ duration` itself, is incommensurable and
    /// therefore absent; `dayTimeDuration − yearMonthDuration` and
    /// `yearMonthDuration − dayTimeDuration` are absent (mixed-sign result,
    /// `OutOfRange`) despite `duration − duration` in general being accepted;
    /// and `gYear` accepts NO duration operand at these witnesses (`P1Y1M1D`
    /// and `P1Y1M` are both 13 months, not a whole number of years).
    const ACCEPTED: &[(
        crate::XsdDatatype,
        Op,
        Option<crate::XsdDatatype>,
        crate::XsdDatatype,
    )] = &[
        (Integer, Op::Add, Some(Integer), Integer),
        (Integer, Op::Sub, Some(Integer), Integer),
        (Integer, Op::Mul, Some(Integer), Integer),
        (Integer, Op::Div, Some(Integer), Decimal),
        (Integer, Op::Add, Some(Decimal), Decimal),
        (Integer, Op::Sub, Some(Decimal), Decimal),
        (Integer, Op::Mul, Some(Decimal), Decimal),
        (Integer, Op::Div, Some(Decimal), Decimal),
        (Integer, Op::Add, Some(Float), Float),
        (Integer, Op::Sub, Some(Float), Float),
        (Integer, Op::Mul, Some(Float), Float),
        (Integer, Op::Div, Some(Float), Float),
        (Integer, Op::Add, Some(Double), Double),
        (Integer, Op::Sub, Some(Double), Double),
        (Integer, Op::Mul, Some(Double), Double),
        (Integer, Op::Div, Some(Double), Double),
        (Integer, Op::Mul, Some(Duration), Duration),
        (Integer, Op::Mul, Some(DayTimeDuration), DayTimeDuration),
        (Integer, Op::Mul, Some(YearMonthDuration), YearMonthDuration),
        (Integer, Op::Neg, None, Integer),
        (Decimal, Op::Add, Some(Integer), Decimal),
        (Decimal, Op::Sub, Some(Integer), Decimal),
        (Decimal, Op::Mul, Some(Integer), Decimal),
        (Decimal, Op::Div, Some(Integer), Decimal),
        (Decimal, Op::Add, Some(Decimal), Decimal),
        (Decimal, Op::Sub, Some(Decimal), Decimal),
        (Decimal, Op::Mul, Some(Decimal), Decimal),
        (Decimal, Op::Div, Some(Decimal), Decimal),
        (Decimal, Op::Add, Some(Float), Float),
        (Decimal, Op::Sub, Some(Float), Float),
        (Decimal, Op::Mul, Some(Float), Float),
        (Decimal, Op::Div, Some(Float), Float),
        (Decimal, Op::Add, Some(Double), Double),
        (Decimal, Op::Sub, Some(Double), Double),
        (Decimal, Op::Mul, Some(Double), Double),
        (Decimal, Op::Div, Some(Double), Double),
        (Decimal, Op::Mul, Some(Duration), Duration),
        (Decimal, Op::Mul, Some(DayTimeDuration), DayTimeDuration),
        (Decimal, Op::Mul, Some(YearMonthDuration), YearMonthDuration),
        (Decimal, Op::Neg, None, Decimal),
        (Float, Op::Add, Some(Integer), Float),
        (Float, Op::Sub, Some(Integer), Float),
        (Float, Op::Mul, Some(Integer), Float),
        (Float, Op::Div, Some(Integer), Float),
        (Float, Op::Add, Some(Decimal), Float),
        (Float, Op::Sub, Some(Decimal), Float),
        (Float, Op::Mul, Some(Decimal), Float),
        (Float, Op::Div, Some(Decimal), Float),
        (Float, Op::Add, Some(Float), Float),
        (Float, Op::Sub, Some(Float), Float),
        (Float, Op::Mul, Some(Float), Float),
        (Float, Op::Div, Some(Float), Float),
        (Float, Op::Add, Some(Double), Double),
        (Float, Op::Sub, Some(Double), Double),
        (Float, Op::Mul, Some(Double), Double),
        (Float, Op::Div, Some(Double), Double),
        (Float, Op::Neg, None, Float),
        (Double, Op::Add, Some(Integer), Double),
        (Double, Op::Sub, Some(Integer), Double),
        (Double, Op::Mul, Some(Integer), Double),
        (Double, Op::Div, Some(Integer), Double),
        (Double, Op::Add, Some(Decimal), Double),
        (Double, Op::Sub, Some(Decimal), Double),
        (Double, Op::Mul, Some(Decimal), Double),
        (Double, Op::Div, Some(Decimal), Double),
        (Double, Op::Add, Some(Float), Double),
        (Double, Op::Sub, Some(Float), Double),
        (Double, Op::Mul, Some(Float), Double),
        (Double, Op::Div, Some(Float), Double),
        (Double, Op::Add, Some(Double), Double),
        (Double, Op::Sub, Some(Double), Double),
        (Double, Op::Mul, Some(Double), Double),
        (Double, Op::Div, Some(Double), Double),
        (Double, Op::Neg, None, Double),
        (DateTime, Op::Sub, Some(DateTime), DayTimeDuration),
        (DateTime, Op::Add, Some(Duration), DateTime),
        (DateTime, Op::Sub, Some(Duration), DateTime),
        (DateTime, Op::Add, Some(DayTimeDuration), DateTime),
        (DateTime, Op::Sub, Some(DayTimeDuration), DateTime),
        (DateTime, Op::Add, Some(YearMonthDuration), DateTime),
        (DateTime, Op::Sub, Some(YearMonthDuration), DateTime),
        (Date, Op::Sub, Some(Date), DayTimeDuration),
        (Date, Op::Add, Some(Duration), Date),
        (Date, Op::Sub, Some(Duration), Date),
        (Date, Op::Add, Some(DayTimeDuration), Date),
        (Date, Op::Sub, Some(DayTimeDuration), Date),
        (Date, Op::Add, Some(YearMonthDuration), Date),
        (Date, Op::Sub, Some(YearMonthDuration), Date),
        (Time, Op::Sub, Some(Time), DayTimeDuration),
        (Time, Op::Add, Some(DayTimeDuration), Time),
        (Time, Op::Sub, Some(DayTimeDuration), Time),
        (Duration, Op::Mul, Some(Integer), Duration),
        (Duration, Op::Div, Some(Integer), Duration),
        (Duration, Op::Mul, Some(Decimal), Duration),
        (Duration, Op::Div, Some(Decimal), Duration),
        (Duration, Op::Add, Some(DateTime), DateTime),
        (Duration, Op::Add, Some(Date), Date),
        (Duration, Op::Add, Some(Duration), Duration),
        (Duration, Op::Sub, Some(Duration), Duration),
        (Duration, Op::Add, Some(DayTimeDuration), Duration),
        (Duration, Op::Sub, Some(DayTimeDuration), Duration),
        (Duration, Op::Add, Some(YearMonthDuration), Duration),
        (Duration, Op::Sub, Some(YearMonthDuration), Duration),
        (Duration, Op::Add, Some(GMonthDay), GMonthDay),
        (Duration, Op::Add, Some(GDay), GDay),
        (Duration, Op::Neg, None, Duration),
        (DayTimeDuration, Op::Mul, Some(Integer), DayTimeDuration),
        (DayTimeDuration, Op::Div, Some(Integer), DayTimeDuration),
        (DayTimeDuration, Op::Mul, Some(Decimal), DayTimeDuration),
        (DayTimeDuration, Op::Div, Some(Decimal), DayTimeDuration),
        (DayTimeDuration, Op::Add, Some(DateTime), DateTime),
        (DayTimeDuration, Op::Add, Some(Date), Date),
        (DayTimeDuration, Op::Add, Some(Time), Time),
        (DayTimeDuration, Op::Add, Some(Duration), Duration),
        (DayTimeDuration, Op::Sub, Some(Duration), Duration),
        (
            DayTimeDuration,
            Op::Add,
            Some(DayTimeDuration),
            DayTimeDuration,
        ),
        (
            DayTimeDuration,
            Op::Sub,
            Some(DayTimeDuration),
            DayTimeDuration,
        ),
        (DayTimeDuration, Op::Div, Some(DayTimeDuration), Decimal),
        (DayTimeDuration, Op::Add, Some(YearMonthDuration), Duration),
        (DayTimeDuration, Op::Add, Some(GMonthDay), GMonthDay),
        (DayTimeDuration, Op::Add, Some(GDay), GDay),
        (DayTimeDuration, Op::Neg, None, DayTimeDuration),
        (YearMonthDuration, Op::Mul, Some(Integer), YearMonthDuration),
        (YearMonthDuration, Op::Div, Some(Integer), YearMonthDuration),
        (YearMonthDuration, Op::Mul, Some(Decimal), YearMonthDuration),
        (YearMonthDuration, Op::Div, Some(Decimal), YearMonthDuration),
        (YearMonthDuration, Op::Add, Some(DateTime), DateTime),
        (YearMonthDuration, Op::Add, Some(Date), Date),
        (YearMonthDuration, Op::Add, Some(Duration), Duration),
        (YearMonthDuration, Op::Sub, Some(Duration), Duration),
        (YearMonthDuration, Op::Add, Some(DayTimeDuration), Duration),
        (
            YearMonthDuration,
            Op::Add,
            Some(YearMonthDuration),
            YearMonthDuration,
        ),
        (
            YearMonthDuration,
            Op::Sub,
            Some(YearMonthDuration),
            YearMonthDuration,
        ),
        (YearMonthDuration, Op::Div, Some(YearMonthDuration), Decimal),
        (YearMonthDuration, Op::Add, Some(GYearMonth), GYearMonth),
        (YearMonthDuration, Op::Add, Some(GMonth), GMonth),
        (YearMonthDuration, Op::Add, Some(GMonthDay), GMonthDay),
        (YearMonthDuration, Op::Add, Some(GDay), GDay),
        (YearMonthDuration, Op::Neg, None, YearMonthDuration),
        (GYearMonth, Op::Add, Some(YearMonthDuration), GYearMonth),
        (GYearMonth, Op::Sub, Some(YearMonthDuration), GYearMonth),
        (GMonth, Op::Add, Some(YearMonthDuration), GMonth),
        (GMonth, Op::Sub, Some(YearMonthDuration), GMonth),
        (GMonthDay, Op::Add, Some(Duration), GMonthDay),
        (GMonthDay, Op::Sub, Some(Duration), GMonthDay),
        (GMonthDay, Op::Add, Some(DayTimeDuration), GMonthDay),
        (GMonthDay, Op::Sub, Some(DayTimeDuration), GMonthDay),
        (GMonthDay, Op::Add, Some(YearMonthDuration), GMonthDay),
        (GMonthDay, Op::Sub, Some(YearMonthDuration), GMonthDay),
        (GDay, Op::Add, Some(Duration), GDay),
        (GDay, Op::Sub, Some(Duration), GDay),
        (GDay, Op::Add, Some(DayTimeDuration), GDay),
        (GDay, Op::Sub, Some(DayTimeDuration), GDay),
        (GDay, Op::Add, Some(YearMonthDuration), GDay),
        (GDay, Op::Sub, Some(YearMonthDuration), GDay),
    ];

    /// The witness-classified operator table: builds the FULL 18×4×18 = 1296
    /// binary cross product plus 18 unary pairs over [`UNIVERSE`] and asserts
    /// that every triple present in [`ACCEPTED`] succeeds with exactly its
    /// pinned result datatype, and every triple ABSENT from [`ACCEPTED`] is
    /// `Err` — so the accepted/rejected partition is the property under test,
    /// not [`ACCEPTED`]'s length alone.
    #[test]
    fn sep0002_operator_table() {
        fn witness(dt: crate::XsdDatatype) -> XsdValue {
            let (_, lex) = UNIVERSE
                .iter()
                .find(|(d, _)| *d == dt)
                .expect("every UNIVERSE member has a witness");
            v(lex, dt)
        }

        for &(ldt, _) in UNIVERSE {
            for &(rdt, _) in UNIVERSE {
                for op in [Op::Add, Op::Sub, Op::Mul, Op::Div] {
                    let a = witness(ldt);
                    let b = witness(rdt);
                    let result = match op {
                        Op::Add => value_add(&a, &b),
                        Op::Sub => value_sub(&a, &b),
                        Op::Mul => value_mul(&a, &b),
                        Op::Div => value_div(&a, &b),
                        Op::Neg => unreachable!("unary is driven separately below"),
                    };
                    let expected = ACCEPTED
                        .iter()
                        .find(|(l, o, r, _)| *l == ldt && *o == op && *r == Some(rdt));
                    match (result, expected) {
                        (Ok(got), Some((_, _, _, want))) => assert_eq!(
                            got.datatype(),
                            *want,
                            "{ldt:?} {op:?} {rdt:?}: wrong result datatype"
                        ),
                        (Err(_), None) => { /* correctly rejected */ }
                        (Ok(got), None) => panic!(
                            "{ldt:?} {op:?} {rdt:?}: unexpectedly accepted -> {:?}",
                            got.datatype()
                        ),
                        (Err(e), Some(_)) => {
                            panic!("{ldt:?} {op:?} {rdt:?}: unexpectedly rejected -> {e:?}")
                        }
                    }
                }
            }
            // Unary minus.
            let a = witness(ldt);
            let result = value_unary_minus(&a);
            let expected = ACCEPTED
                .iter()
                .find(|(l, o, r, _)| *l == ldt && *o == Op::Neg && r.is_none());
            match (result, expected) {
                (Ok(got), Some((_, _, _, want))) => {
                    assert_eq!(
                        got.datatype(),
                        *want,
                        "{ldt:?} unary-: wrong result datatype"
                    );
                }
                (Err(_), None) => { /* correctly rejected */ }
                (Ok(got), None) => panic!(
                    "{ldt:?} unary-: unexpectedly accepted -> {:?}",
                    got.datatype()
                ),
                (Err(e), Some(_)) => panic!("{ldt:?} unary-: unexpectedly rejected -> {e:?}"),
            }
        }
    }

    /// Rejected pairs whose EXACT `XsdError` VARIANT is pinned (not merely
    /// `.is_err()`), covering the minimum set of pairs needed to exercise
    /// every distinct `TypeMismatch` rejection arm in the `value_*` dispatch,
    /// plus the three witness-stability rows [`ACCEPTED`]'s own doc comment records: `duration
    /// ÷ duration` (`Indeterminate` — value-incommensurable, not a type error),
    /// `dayTimeDuration − yearMonthDuration` (`OutOfRange` — a mixed-sign
    /// result, not a type error), and `gYear + yearMonthDuration` (rejected,
    /// exact variant pinned below).
    #[test]
    fn sep0002_rejected_pairs() {
        // DUR × DUR — a duration is never an exact scale factor for another.
        assert!(matches!(
            value_mul(&v("P1Y1M1D", Duration), &v("P1Y1M1D", Duration)),
            Err(XsdError::TypeMismatch { .. })
        ));
        // DUR × Float / DUR × Double — the exact-tier rule (RDF4J's
        // `isDecimalDatatype` gate): an inexact factor cannot scale an exact
        // duration without silent rounding.
        assert!(matches!(
            value_mul(&v("P1Y1M1D", Duration), &v("3.5", Float)),
            Err(XsdError::TypeMismatch { .. })
        ));
        assert!(matches!(
            value_mul(&v("P1Y1M1D", Duration), &v("3.5", Double)),
            Err(XsdError::TypeMismatch { .. })
        ));
        // Nx ÷ DUR — division is not commutative like multiplication.
        assert!(matches!(
            value_div(&v("3", Integer), &v("P1D", DayTimeDuration)),
            Err(XsdError::TypeMismatch { .. })
        ));
        // DUR − instant — subtraction has no commuted temporal row.
        assert!(matches!(
            value_sub(
                &v("P1Y1M1D", Duration),
                &v("2024-01-31T12:00:00Z", DateTime)
            ),
            Err(XsdError::TypeMismatch { .. })
        ));
        // instant + instant — not in the addition table (only instant ± DUR is).
        assert!(matches!(
            value_add(
                &v("2024-01-31T12:00:00Z", DateTime),
                &v("2024-01-31T12:00:00Z", DateTime)
            ),
            Err(XsdError::TypeMismatch { .. })
        ));
        // time + yearMonthDuration — MonthAction::Absent for a nonzero months
        // component: `time` has no months field.
        assert!(matches!(
            value_add(&v("12:00:00", Time), &v("P1Y1M", YearMonthDuration)),
            Err(XsdError::TypeMismatch { .. })
        ));
        // Boolean + Duration — a non-arithmetic UNIVERSE member exercising the
        // `Boolean`/`String`/`Binary` reject arm, not just the exhaustive match.
        assert!(matches!(
            value_add(&v("true", Boolean), &v("P1D", DayTimeDuration)),
            Err(XsdError::TypeMismatch { .. })
        ));

        // Witness-stability row: duration ÷ duration at the general (Shape::Mixed)
        // witness is value-incommensurable, not a type error.
        assert!(matches!(
            value_div(&v("P1Y1M1D", Duration), &v("P1Y1M1D", Duration)),
            Err(XsdError::Indeterminate {
                reason: "incommensurable duration operands"
            })
        ));
        // Witness-stability row: dayTimeDuration - yearMonthDuration produces a
        // mixed-sign (months, seconds) pair at these witnesses, rejected by
        // `Duration::new` as OutOfRange — despite `DUR - DUR` in general being
        // accepted.
        assert!(matches!(
            value_sub(&v("P1D", DayTimeDuration), &v("P1Y1M", YearMonthDuration)),
            Err(XsdError::OutOfRange { .. })
        ));
        // Witness-stability row: gYear accepts no duration operand at these
        // witnesses (13 months is not a whole number of years).
        assert!(matches!(
            value_add(&v("2024", GYear), &v("P1Y1M", YearMonthDuration)),
            Err(XsdError::TypeMismatch { .. })
        ));
    }
}
