// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact comparison of JSON number lexemes.
//!
//! RFC 8259 §6 puts no bound on a number's digits or on its exponent, and JSON
//! Schema compares numbers by their mathematical value (`1.0` equals `1`,
//! `1e999999999999999999999` is a number like any other). Converting either
//! side to a machine float rounds, and converting to a machine integer
//! overflows, so [`cmp`] never converts: it normalizes each lexeme to its
//! significant digits and its order of magnitude, the latter a [`BigInt`] so no
//! exponent is too large, and compares those.

use core::cmp::Ordering;

use crate::bigint::BigInt;

/// A JSON number reduced to `±0.d₁d₂…dₙ × 10^order` with `d₁ ≠ 0` and `dₙ ≠ 0`,
/// or zero.
struct Normalized {
    negative: bool,
    /// The significant digits; empty for zero.
    digits: Vec<u8>,
    order: BigInt,
}

/// Reads `number = [ minus ] int [ frac ] [ exp ]` (RFC 8259 §6).
fn normalize(lexeme: &str) -> Option<Normalized> {
    let bytes = lexeme.as_bytes();
    let (negative, rest) = match bytes.first() {
        Some(b'-') => (true, &bytes[1..]),
        _ => (false, bytes),
    };
    let int_len = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
    // `int = zero / ( digit1-9 *DIGIT )`.
    if int_len == 0 || (int_len > 1 && rest[0] == b'0') {
        return None;
    }
    let (int, rest) = rest.split_at(int_len);
    let (frac, rest) = match rest.split_first() {
        Some((b'.', after)) => {
            let frac_len = after
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            if frac_len == 0 {
                return None;
            }
            after.split_at(frac_len)
        }
        _ => (&rest[..0], rest),
    };
    let exponent = match rest.split_first() {
        None => BigInt::zero(),
        Some((b'e' | b'E', after)) => {
            // `exp = e [ minus / plus ] 1*DIGIT`; `from_digits` reads exactly that
            // sign-and-digits shape and refuses anything else.
            BigInt::from_digits(core::str::from_utf8(after).ok()?)?
        }
        Some(_) => return None,
    };
    // The value is `int frac × 10^(exponent − |frac|)`.
    let mantissa: Vec<u8> = int.iter().chain(frac).copied().collect();
    let leading = mantissa.iter().take_while(|&&digit| digit == b'0').count();
    let significant = &mantissa[leading..];
    let trailing = significant
        .iter()
        .rev()
        .take_while(|&&digit| digit == b'0')
        .count();
    let digits = significant[..significant.len() - trailing].to_vec();
    if digits.is_empty() {
        return Some(Normalized {
            negative: false,
            digits,
            order: BigInt::zero(),
        });
    }
    // order = exponent − |frac| + trailing + |digits|; each term is a slice
    // length, far inside `i128`.
    let mut order = exponent;
    order.add_i128(
        i128::try_from(digits.len() + trailing).unwrap_or(i128::MAX)
            - i128::try_from(frac.len()).unwrap_or(i128::MAX),
    );
    Some(Normalized {
        negative,
        digits,
        order,
    })
}

/// The order of two JSON number lexemes by the numbers they denote, exactly:
/// `1.0` and `10e-1` equal `1`, `-0` equals `0`, and no length of digits or
/// exponent loses precision. `None` when either side is not an RFC 8259 §6
/// `number` (a leading `+`, a leading zero before more digits, a bare `.`,
/// an empty exponent, or anything else outside the grammar).
#[must_use]
pub fn cmp(left: &str, right: &str) -> Option<Ordering> {
    let left = normalize(left)?;
    let right = normalize(right)?;
    let magnitude = |a: &Normalized, b: &Normalized| {
        // Digit strings without leading or trailing zeros at one order of
        // magnitude compare as their bytes do.
        a.order.cmp(&b.order).then_with(|| a.digits.cmp(&b.digits))
    };
    Some(match (left.digits.is_empty(), right.digits.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) if right.negative => Ordering::Greater,
        (true, false) => Ordering::Less,
        (false, true) if left.negative => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => match (left.negative, right.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => magnitude(&left, &right),
            (true, true) => magnitude(&right, &left),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::cmp;
    use core::cmp::Ordering::{Equal, Greater, Less};

    #[test]
    fn equal_values_with_different_spellings_compare_equal() {
        assert_eq!(cmp("1", "1.0"), Some(Equal));
        assert_eq!(cmp("1", "10e-1"), Some(Equal));
        assert_eq!(cmp("100", "1E+2"), Some(Equal));
        assert_eq!(cmp("-0", "0.000e7"), Some(Equal));
        assert_eq!(
            cmp("1e999999999999999999999", "10e999999999999999999998"),
            Some(Equal)
        );
    }

    #[test]
    fn close_values_are_ordered_exactly() {
        assert_eq!(cmp("1.0000000000000001", "1"), Some(Greater));
        assert_eq!(cmp("-1.0000000000000001", "-1"), Some(Less));
        assert_eq!(
            cmp("18446744073709551615", "18446744073709551616"),
            Some(Less)
        );
        assert_eq!(cmp("1e-999999999999999999999", "0"), Some(Greater));
        assert_eq!(cmp("-1e-999999999999999999999", "0"), Some(Less));
        assert_eq!(cmp("0.12", "0.123"), Some(Less));
        assert_eq!(cmp("0.13", "0.123"), Some(Greater));
    }

    #[test]
    fn a_leading_plus_is_refused_and_the_unsigned_spelling_accepted() {
        assert_eq!(cmp("+1", "1"), None);
        assert_eq!(cmp("1", "1"), Some(Equal));
    }

    #[test]
    fn a_leading_zero_before_digits_is_refused_and_a_lone_zero_accepted() {
        assert_eq!(cmp("01", "1"), None);
        assert_eq!(cmp("0", "0.5"), Some(Less));
    }

    #[test]
    fn a_bare_point_is_refused_and_a_digit_after_it_accepted() {
        assert_eq!(cmp("1.", "1"), None);
        assert_eq!(cmp(".5", "1"), None);
        assert_eq!(cmp("1.5", "1"), Some(Greater));
    }

    #[test]
    fn an_empty_exponent_is_refused_and_a_signed_one_accepted() {
        assert_eq!(cmp("1e", "1"), None);
        assert_eq!(cmp("1e+", "1"), None);
        assert_eq!(cmp("1e-0", "1"), Some(Equal));
    }

    #[test]
    fn text_outside_the_grammar_is_refused_and_a_number_accepted() {
        for text in [
            "", "-", "NaN", "Infinity", "1 ", " 1", "0x10", "1_000", "1e1.5",
        ] {
            assert_eq!(cmp(text, "1"), None, "{text:?}");
        }
        assert_eq!(cmp("-12.5e3", "-12500"), Some(Equal));
    }
}
