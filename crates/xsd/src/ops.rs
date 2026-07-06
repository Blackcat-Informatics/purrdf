// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The value-space operator surface: SPARQL `=` / `<` and the effective boolean
//! value. These are **value-space** operations (distinct from RDF term identity —
//! see the crate docs). They are free functions, not trait impls, so they cannot be
//! confused with the structural `Eq`/`Ord` a `HashMap`/`BTreeMap` would use.

use std::cmp::Ordering;

use crate::numeric::numeric_cmp;
use crate::value::XsdValue;

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
        (XsdValue::DateTime(x), XsdValue::DateTime(y)) => crate::temporal::cmp_datetime(x, y),
        (XsdValue::Date(x), XsdValue::Date(y)) => crate::temporal::cmp_date(x, y),
        (XsdValue::Time(x), XsdValue::Time(y)) => crate::temporal::cmp_time(x, y),
        (XsdValue::Duration(x), XsdValue::Duration(y)) => crate::temporal::cmp_duration(x, y),
        // Gregorian family: same-type comparison, cross-type incomparable.
        (Gregorian(x), Gregorian(y)) => crate::temporal::cmp_gregorian(x, y),
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

/// SPARQL value-space equality (`=`). Convenience over [`value_cmp`]; returns
/// `false` for incomparable operands. When the error-vs-false distinction matters
/// (the SPARQL `=` operator raises a type error on incomparable operands), use
/// [`value_cmp`] and treat `None` as the error.
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
    value_cmp(a, b) == Some(Ordering::Equal)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::XsdDatatype::{
        Boolean, Byte, Decimal, Double, Float, Int, Integer, Long, Short, String, UnsignedByte,
    };
    use crate::value::parse;

    fn v(lex: &str, dt: crate::XsdDatatype) -> XsdValue {
        parse(lex, dt).unwrap()
    }

    /// The SPARQL operator-mapping table (numeric tower + string + boolean). Each row
    /// asserts `value_cmp` (and hence `=`/`<`/`>`). Temporal rows are added in Task 4.
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
        ];
        for (la, da, lb, db, want) in rows {
            assert_eq!(
                value_cmp(&v(la, da), &v(lb, db)),
                want,
                "value_cmp({la:?}^^{da:?}, {lb:?}^^{db:?})"
            );
        }
    }

    #[test]
    fn value_eq_incomparable_is_false_not_error() {
        assert!(value_eq(&v("1", Integer), &v("1.0", Decimal)));
        assert!(!value_eq(&v("1", Integer), &v("1", String)));
        // NaN: not equal, and value_cmp distinguishes the type-error (None).
        let nan = v("NaN", Double);
        assert!(!value_eq(&nan, &nan));
        assert_eq!(value_cmp(&nan, &nan), None);
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
}
