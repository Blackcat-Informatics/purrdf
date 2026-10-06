// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact rational arithmetic and the bounded `owl:rational` value carrier.
//! [`Rat`] is the shared arbitrary-precision arithmetic home. [`Rational`]
//! retains the `i128` OWL surface and its decimal-branch identity semantics.
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
//! Construction and ordering delegate to [`Rat`]'s shared exact integer
//! arithmetic, then check the bounded representation. Equality is structural
//! on the reduced form. The OWL carrier stays `Copy`, with no new datatype
//! identity or implicit float identification.

use crate::integer::Int;
use crate::numeric::Decimal;
use crate::value::XsdValue;

mod exact;
pub use exact::{Rat, RationalComparisonBody, RationalComparisonError};

pub use crate::datatype::OWL_RATIONAL;

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
        let reduced = Rat::new(Int::from_i128(numerator), Int::from_i128(denominator))
            .expect("nonzero denominator checked above");
        let numerator = reduced
            .numerator()
            .to_i128()
            .ok_or_else(|| RationalError::Overflow(reduced.numerator().abs().to_string()))?;
        let denominator = reduced
            .denominator()
            .to_i128()
            .ok_or_else(|| RationalError::Overflow(reduced.denominator().to_string()))?;
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
    ///
    /// # This is an IDENTITY question, not an order question
    ///
    /// A finite IEEE value *is* a rational — a dyadic one,
    /// `significand × 2^exponent` — so the exclusion here is semantic rather than
    /// arithmetic: the OWL 2 datatype map puts `xsd:float`/`xsd:double` on their own
    /// branch, and a reasoner that read `"0.5"^^xsd:float` as `1/2` would decide
    /// `dt-eq` for a pair the map keeps apart. That reading must not leak in here.
    ///
    /// The **ordering** across the branches is a different question with a different
    /// answer, and it is exact: [`crate::numeric_total_cmp`] compares an
    /// integer/decimal against a float/double as the rationals they both denote,
    /// with no rounding anywhere. It does not go through this type because most
    /// doubles need a denominator up to `2^1074`, which no `i128` pair holds; it
    /// cross-multiplies through [`crate::BigInt`] instead. So the tower orders
    /// exactly end to end while the OWL 2 identification stays exactly as narrow as
    /// the datatype map makes it.
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
        Rat::from_reduced(
            Int::from_i128(self.numerator),
            Int::from_i128(self.denominator),
        )
        .cmp(&Rat::from_reduced(
            Int::from_i128(other.numerator),
            Int::from_i128(other.denominator),
        ))
    }
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
