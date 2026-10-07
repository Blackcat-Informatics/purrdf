// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The XSD value space for JavaScript: the canonical lexical form of a typed value and
//! the value-space comparison of two typed values, both exact for integers and decimals
//! of any length.
//!
//! JavaScript has no exact decimal type and a `number` holds 53 bits, so a host that
//! compares or normalizes numeric literals by converting them loses digits. These
//! functions keep the value in the engine and hand back text or an ordering instead.

use purrdf::xsd::{XsdValue, parse_by_iri, value_cmp};
use wasm_bindgen::prelude::*;

/// The value of `lexical` read as `datatype`, or `None` when the datatype is not an XSD
/// datatype the engine maps or the lexical form is not in its lexical space.
fn typed_value(lexical: &str, datatype: &str) -> Option<XsdValue> {
    parse_by_iri(lexical, datatype).ok().flatten()
}

/// The XSD canonical lexical form of `lexical` read as the datatype IRI `datatype`
/// (`"+007"` as `xsd:integer` is `"7"`, `"1.50"` as `xsd:decimal` is `"1.5"`).
///
/// Integers and decimals are exact at any length. Returns `undefined` when `datatype`
/// is not an XSD datatype the engine maps, or when `lexical` is not in its lexical
/// space.
#[wasm_bindgen(js_name = xsdCanonicalLexical)]
#[must_use]
pub fn xsd_canonical_lexical(lexical: &str, datatype: &str) -> Option<String> {
    typed_value(lexical, datatype).map(|value| value.canonical_lexical())
}

/// Compare two typed values in the XSD value space: `-1` when the left value is the
/// smaller, `0` when they are equal, `1` when it is the larger.
///
/// Numeric datatypes compare across each other (`xsd:integer` against `xsd:decimal`),
/// exactly at any length. Returns `undefined` when either datatype is not an XSD
/// datatype the engine maps, either lexical form is not in its datatype's lexical
/// space, or the two values are incomparable (a `NaN`, or two value-space families
/// such as a number and a string).
///
/// The order is an `f64` because a JavaScript number is one. An optional `f64` also
/// returns through the call's result slot, so the generated glue enters the instance
/// before it reads any argument, as every other entry point's does: on a poisoned
/// instance the call refuses at the poison gate rather than with a glue `TypeError`.
#[wasm_bindgen(js_name = xsdValueCompare)]
#[must_use]
pub fn xsd_value_compare(
    left_lexical: &str,
    left_datatype: &str,
    right_lexical: &str,
    right_datatype: &str,
) -> Option<f64> {
    let left = typed_value(left_lexical, left_datatype)?;
    let right = typed_value(right_lexical, right_datatype)?;
    value_cmp(&left, &right).map(|ordering| f64::from(ordering as i8))
}

#[cfg(test)]
mod tests {
    use purrdf::xsd::datatype::{XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER, XSD_STRING};

    use super::*;

    /// `i128::MAX + 1`, the first integer past the bounded representation.
    const PAST_I128: &str = "170141183460469231731687303715884105728";
    /// A sixty-digit integer.
    const SIXTY_DIGITS: &str = "123456789012345678901234567890123456789012345678901234567890";

    #[test]
    fn integers_past_i128_are_canonical_and_exact() {
        assert_eq!(
            xsd_canonical_lexical(PAST_I128, XSD_INTEGER).as_deref(),
            Some(PAST_I128)
        );
        assert_eq!(
            xsd_canonical_lexical(SIXTY_DIGITS, XSD_INTEGER).as_deref(),
            Some(SIXTY_DIGITS)
        );
        assert_eq!(
            xsd_canonical_lexical(&format!("+000{SIXTY_DIGITS}"), XSD_INTEGER).as_deref(),
            Some(SIXTY_DIGITS)
        );
        assert_eq!(
            xsd_canonical_lexical(&format!("-{PAST_I128}"), XSD_INTEGER),
            Some(format!("-{PAST_I128}"))
        );
    }

    #[test]
    fn long_decimals_are_canonical_and_exact() {
        let forty_fraction_digits = "0.1000000000000000000000000000000000000001";
        assert_eq!(
            xsd_canonical_lexical(forty_fraction_digits, XSD_DECIMAL).as_deref(),
            Some(forty_fraction_digits)
        );
        assert_eq!(
            xsd_canonical_lexical("1.50", XSD_DECIMAL).as_deref(),
            Some("1.5")
        );
        assert_eq!(
            xsd_canonical_lexical(&format!("{SIXTY_DIGITS}.000"), XSD_DECIMAL).as_deref(),
            Some(SIXTY_DIGITS)
        );
    }

    #[test]
    fn a_malformed_or_unmapped_lexical_has_no_canonical_form() {
        assert_eq!(xsd_canonical_lexical("12x", XSD_INTEGER), None);
        assert_eq!(
            xsd_canonical_lexical("12", XSD_INTEGER).as_deref(),
            Some("12")
        );
        assert_eq!(
            xsd_canonical_lexical("12", "http://example.org/datatype"),
            None
        );
    }

    #[test]
    fn long_integers_compare_exactly() {
        let smaller = SIXTY_DIGITS;
        let larger = "123456789012345678901234567890123456789012345678901234567891";
        assert_eq!(
            xsd_value_compare(smaller, XSD_INTEGER, larger, XSD_INTEGER),
            Some(-1.0)
        );
        assert_eq!(
            xsd_value_compare(larger, XSD_INTEGER, smaller, XSD_INTEGER),
            Some(1.0)
        );
        assert_eq!(
            xsd_value_compare(smaller, XSD_INTEGER, &format!("0{smaller}"), XSD_INTEGER),
            Some(0.0)
        );
        // 10^42 + 1 against 10^42 + 2: equal as any 64-bit float.
        let plus_one = format!("1{}1", "0".repeat(41));
        let plus_two = format!("1{}2", "0".repeat(41));
        assert_eq!(
            xsd_value_compare(&plus_one, XSD_INTEGER, &plus_two, XSD_INTEGER),
            Some(-1.0)
        );
        assert_eq!(
            xsd_value_compare(
                &plus_one,
                XSD_INTEGER,
                &format!("{plus_one}.0"),
                XSD_DECIMAL
            ),
            Some(0.0)
        );
    }

    #[test]
    fn malformed_and_incomparable_values_have_no_order() {
        assert_eq!(
            xsd_value_compare("12x", XSD_INTEGER, "12", XSD_INTEGER),
            None
        );
        assert_eq!(
            xsd_value_compare("12", XSD_INTEGER, "12", XSD_INTEGER),
            Some(0.0)
        );
        assert_eq!(xsd_value_compare("NaN", XSD_DOUBLE, "1", XSD_DOUBLE), None);
        assert_eq!(xsd_value_compare("1", XSD_INTEGER, "1", XSD_STRING), None);
        assert_eq!(
            xsd_value_compare("1", XSD_INTEGER, "1", XSD_DOUBLE),
            Some(0.0)
        );
    }
}
