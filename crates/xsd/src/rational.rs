// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `owl:rational` — exact rationals over `i128`, and their identity with the
//! decimal branch of the OWL 2 datatype map.
//!
//! OWL 2's numeric value spaces nest: the integers inside the decimals, the
//! decimals inside the rationals, the rationals inside `owl:real` — while
//! `xsd:float` and `xsd:double` are DISJOINT branches. So `"0.5"^^xsd:decimal`
//! and `"1/2"^^owl:rational` denote ONE value, `"5"^^xsd:integer` and
//! `"5/1"^^owl:rational` denote one value, and `"0.5"^^xsd:float` is equal to
//! neither. A reasoner that misses the first identification silently loses a
//! `dt-eq`-style decision; one that invents a float identification makes a
//! wrong one. Both directions are decided here, exactly.
//!
//! # Why a standalone type rather than an [`XsdValue`] variant
//!
//! [`XsdValue`] is matched exhaustively across the SPARQL evaluator, SHACL and
//! ShEx — surfaces whose specifications do not know `owl:rational` and would
//! each need an invented semantics for a new variant. The rational value space
//! is an OWL 2 concern, consumed by the reasoner's concrete domain, so it lives
//! beside [`XsdValue`] with an exact, total injection FROM the numeric branch
//! ([`Rational::from_xsd`]) instead of enlarging every match in the workspace.
//!
//! # Representation
//!
//! `numerator / denominator` with `denominator > 0` and `gcd = 1`, both `i128`.
//! Construction reduces; reduction uses only `gcd` and division, so it cannot
//! overflow. Equality is STRUCTURAL on the reduced form — no cross
//! multiplication, so no overflow path exists on the identity question this
//! module is for. Ordering does cross-multiply, in `u128` magnitude space with
//! the signs handled first, which is exact for every representable pair.
//!
//! This is deliberately a value type and not an arithmetic tower: `+ - × ÷`
//! belong to the consumer that needs them, and the representation is the stable
//! seam such a consumer extends.

use crate::numeric::Decimal;
use crate::value::XsdValue;

/// The `owl:rational` datatype IRI.
pub const OWL_RATIONAL: &str = "http://www.w3.org/2002/07/owl#rational";

/// An exact rational: `numerator / denominator`, reduced, `denominator > 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}

/// Why a lexical form is not an `owl:rational` literal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RationalError {
    /// The lexical form is not `integer '/' positive-integer` (OWL 2 §4.1: no
    /// whitespace, and the denominator is an unsigned integer).
    Lexical(String),
    /// The denominator is zero, which names no rational number.
    ZeroDenominator,
    /// A component does not fit the `i128` representation.
    Overflow(String),
}

impl std::fmt::Display for RationalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lexical(lex) => {
                write!(f, "not an owl:rational lexical form: {lex:?}")
            }
            Self::ZeroDenominator => f.write_str("owl:rational denominator is zero"),
            Self::Overflow(lex) => {
                write!(f, "owl:rational component exceeds i128: {lex:?}")
            }
        }
    }
}

impl std::error::Error for RationalError {}

const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Rational {
    /// Construct from a numerator and a non-zero denominator, reducing.
    ///
    /// # Errors
    ///
    /// [`RationalError::ZeroDenominator`] when `denominator == 0`.
    pub fn new(numerator: i128, denominator: i128) -> Result<Self, RationalError> {
        if denominator == 0 {
            return Err(RationalError::ZeroDenominator);
        }
        let negative = (numerator < 0) != (denominator < 0);
        let num_mag = numerator.unsigned_abs();
        let den_mag = denominator.unsigned_abs();
        // `den_mag > 0` (checked above), so the gcd is at least 1 and division
        // is total here.
        let divisor = gcd(num_mag, den_mag);
        let (num_mag, den_mag) = (num_mag / divisor, den_mag / divisor);
        // The magnitudes only shrank, so the casts back cannot overflow — except
        // the i128::MIN magnitude, which survives reduction only when the
        // denominator reduces to 1 and the value is exactly i128::MIN.
        let numerator = if negative {
            i128::try_from(num_mag).map(|n| -n).or_else(|_| {
                if num_mag == i128::MIN.unsigned_abs() {
                    Ok(i128::MIN)
                } else {
                    Err(RationalError::Overflow(format!("{num_mag}")))
                }
            })?
        } else {
            i128::try_from(num_mag).map_err(|_| RationalError::Overflow(format!("{num_mag}")))?
        };
        let denominator =
            i128::try_from(den_mag).map_err(|_| RationalError::Overflow(format!("{den_mag}")))?;
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// Parse the OWL 2 lexical form: `integer '/' positive-integer`, no
    /// whitespace (`"1/3"`, `"-2/6"`).
    ///
    /// # Errors
    ///
    /// [`RationalError`] naming what failed; a zero denominator is refused
    /// rather than read as "unknown".
    pub fn parse(lexical: &str) -> Result<Self, RationalError> {
        let Some((num_text, den_text)) = lexical.split_once('/') else {
            return Err(RationalError::Lexical(lexical.to_owned()));
        };
        let plausible_num = !num_text.is_empty()
            && num_text
                .strip_prefix(['-', '+'])
                .unwrap_or(num_text)
                .bytes()
                .all(|b| b.is_ascii_digit())
            && num_text != "-"
            && num_text != "+";
        let plausible_den = !den_text.is_empty() && den_text.bytes().all(|b| b.is_ascii_digit());
        if !plausible_num || !plausible_den {
            return Err(RationalError::Lexical(lexical.to_owned()));
        }
        let numerator: i128 = num_text
            .parse()
            .map_err(|_| RationalError::Overflow(lexical.to_owned()))?;
        let denominator: i128 = den_text
            .parse()
            .map_err(|_| RationalError::Overflow(lexical.to_owned()))?;
        Self::new(numerator, denominator)
    }

    /// The reduced numerator.
    #[must_use]
    pub const fn numerator(&self) -> i128 {
        self.numerator
    }

    /// The reduced denominator (`> 0`).
    #[must_use]
    pub const fn denominator(&self) -> i128 {
        self.denominator
    }

    /// The exact rational a [`Decimal`] denotes: `mantissa / 10^scale`, reduced.
    ///
    /// Total and exact — every decimal IS a rational, which is the nesting the
    /// OWL 2 datatype map defines and the identification this module exists to
    /// decide.
    #[must_use]
    pub fn from_decimal(decimal: &Decimal) -> Self {
        let denominator = 10i128.pow(u32::from(decimal.scale()));
        Self::new(decimal.mantissa(), denominator)
            .unwrap_or_else(|_| unreachable!("10^scale is never zero"))
    }

    /// The exact rational `value` denotes, when `value` sits on the
    /// integer/decimal branch of the OWL 2 numeric tower.
    ///
    /// `None` for every other variant — including `xsd:float` and `xsd:double`,
    /// whose value spaces the OWL 2 datatype map keeps DISJOINT from the reals,
    /// so answering for them would identify values the map separates.
    #[must_use]
    pub fn from_xsd(value: &XsdValue) -> Option<Self> {
        match value {
            XsdValue::Integer { value, .. } => {
                Some(Self::new(*value, 1).unwrap_or_else(|_| unreachable!("1 is non-zero")))
            }
            XsdValue::Decimal(decimal) => Some(Self::from_decimal(decimal)),
            _ => None,
        }
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        let sign = |r: &Self| r.numerator.signum();
        match sign(self).cmp(&sign(other)) {
            Ordering::Equal => {}
            unequal => return unequal,
        }
        // Same sign. Compare |a/b| vs |c/d| as a·d vs c·b in u128 — exact for
        // every representable pair because each product of two i128 magnitudes
        // that both survived reduction fits u128's doubled width only if the
        // inputs are small enough; where it would not, split multiplication
        // keeps it exact.
        let lhs = wide_mul(
            self.numerator.unsigned_abs(),
            other.denominator.unsigned_abs(),
        );
        let rhs = wide_mul(
            other.numerator.unsigned_abs(),
            self.denominator.unsigned_abs(),
        );
        let magnitude = lhs.cmp(&rhs);
        if sign(self) >= 0 {
            magnitude
        } else {
            magnitude.reverse()
        }
    }
}

/// `a × b` as `(high, low)` 128-bit halves — exact 256-bit magnitude compare.
fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    const MASK: u128 = (1u128 << 64) - 1;
    let (a_hi, a_lo) = (a >> 64, a & MASK);
    let (b_hi, b_lo) = (b >> 64, b & MASK);
    let ll = a_lo * b_lo;
    let lh = a_lo * b_hi;
    let hl = a_hi * b_lo;
    let hh = a_hi * b_hi;
    let mid = (ll >> 64) + (lh & MASK) + (hl & MASK);
    let low = (ll & MASK) | (mid << 64);
    let high = hh + (lh >> 64) + (hl >> 64) + (mid >> 64);
    (high, low)
}

#[cfg(test)]
mod tests {
    use super::{OWL_RATIONAL, Rational, RationalError};
    use crate::datatype::XsdDatatype;
    use crate::value::parse_by_iri;

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    fn xsd(local: &str, lexical: &str) -> crate::value::XsdValue {
        parse_by_iri(lexical, &format!("{XSD}{local}"))
            .expect("parse")
            .expect("supported")
    }

    /// THE PARITY IDENTIFICATION: one value, two datatypes, two lexical forms.
    ///
    /// gmeow's datatype decider makes exactly this identification on its exact-ℚ
    /// tower; a port that lost it would silently stop deciding a `dt-eq`-style
    /// equality. Structural equality on the reduced form is what makes it exact
    /// with no overflow path.
    #[test]
    fn a_decimal_and_a_rational_denoting_one_value_are_equal() {
        let half = Rational::parse("1/2").expect("lexical");
        let decimal_half = Rational::from_xsd(&xsd("decimal", "0.5")).expect("numeric branch");
        assert_eq!(half, decimal_half);

        let third = Rational::parse("1/3").expect("lexical");
        let two_sixths = Rational::parse("2/6").expect("lexical");
        assert_eq!(third, two_sixths, "reduction identifies 1/3 with 2/6");

        // The CONTROL: a nearby but distinct value must stay distinct, so the
        // assertion above turns on identity rather than on everything equating.
        let point_six = Rational::from_xsd(&xsd("decimal", "0.6")).expect("numeric branch");
        assert_ne!(half, point_six);
        assert!(half < point_six, "and the order agrees with the reals");
    }

    /// Integers sit inside the rationals; floats sit on a DISJOINT branch.
    #[test]
    fn the_tower_nests_integers_and_excludes_floats() {
        let five = Rational::from_xsd(&xsd("integer", "5")).expect("numeric branch");
        assert_eq!(five, Rational::parse("5/1").expect("lexical"));
        assert_eq!(five, Rational::parse("10/2").expect("lexical"));

        assert!(
            Rational::from_xsd(&xsd("float", "0.5")).is_none(),
            "xsd:float is disjoint from the reals in the OWL 2 map; identifying \
             \"0.5\"^^xsd:float with 1/2 would equate values the map separates"
        );
    }

    /// The refusals are named, never read as \"unknown\".
    #[test]
    fn malformed_and_zero_denominator_lexicals_are_refused_by_name() {
        assert!(matches!(
            Rational::parse("1/0"),
            Err(RationalError::ZeroDenominator)
        ));
        for bad in [
            "1", "1/ 2", "1 /2", "a/b", "1/-2", "--1/2", "/2", "1/", "+/3",
        ] {
            assert!(
                matches!(Rational::parse(bad), Err(RationalError::Lexical(_))),
                "{bad:?} must be refused as a lexical error"
            );
        }
    }

    /// Negative values reduce with the sign on the numerator, and order holds.
    #[test]
    fn negatives_normalize_and_order_exactly() {
        let a = Rational::new(-2, 6).expect("non-zero");
        assert_eq!((a.numerator(), a.denominator()), (-1, 3));
        let b = Rational::new(2, -6).expect("non-zero");
        assert_eq!(a, b, "the sign lives on the numerator after reduction");
        assert!(a < Rational::parse("1/3").expect("lexical"));
        assert!(Rational::parse("-1/2").expect("lexical") < a);
    }

    /// Ordering is exact at the extremes of the representation, where a naive
    /// cross-multiplication overflows.
    #[test]
    fn ordering_survives_extreme_magnitudes() {
        let huge = Rational::new(i128::MAX, 3).expect("non-zero");
        let huger = Rational::new(i128::MAX, 2).expect("non-zero");
        assert!(huge < huger);
        let tiny = Rational::new(3, i128::MAX).expect("non-zero");
        let tinier = Rational::new(2, i128::MAX).expect("non-zero");
        assert!(tinier < tiny);
        assert!(Rational::new(i128::MIN, 1).expect("non-zero") < tiny);
    }

    /// `parse_by_iri` deliberately does NOT accept `owl:rational`: [`super`]'s
    /// value space is a separate, OWL-2-only surface, and the shared XSD entry
    /// point answering `Ok(None)` (\"recognized as unsupported\") for it is the
    /// three-valued honesty the range decider depends on.
    #[test]
    fn the_shared_entry_point_still_reports_rational_unsupported() {
        assert!(
            parse_by_iri("1/2", OWL_RATIONAL)
                .expect("no lexical error")
                .is_none(),
            "owl:rational is decided by this module, not silently absorbed into XsdValue"
        );
        assert!(XsdDatatype::from_iri(OWL_RATIONAL).is_none());
    }
}
