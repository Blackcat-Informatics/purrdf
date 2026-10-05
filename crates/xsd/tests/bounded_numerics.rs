// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The bounded numeric contract of `purrdf-xsd`: `xsd:integer` in `i128`, `xsd:decimal`
//! in an `i128` mantissa with at most 18 fractional digits, and arithmetic that answers
//! inside those bounds, truncates precision only where XPath F&O permits it, and raises
//! a typed error everywhere else.

use std::cmp::Ordering;

use purrdf_testkit::exact::Rational;
use purrdf_testkit::prop::prelude::*;
use purrdf_xsd::{
    BigInt, Decimal, DecimalDigits, ErrorCode, LiteralValue, XsdDatatype as D, XsdValue,
    bigint_avg_decimal, decimal_mean, literal_cmp, literal_equal, literal_total_cmp, numeric_add,
    numeric_div, numeric_mul, numeric_sub, parse,
};

/// Cases per oracle property.
const CASES: u32 = 4096;

const MAX: &str = "170141183460469231731687303715884105727";
const MAX_1: &str = "170141183460469231731687303715884105726";

fn dec(lexical: &str) -> XsdValue {
    parse(lexical, D::Decimal).unwrap_or_else(|e| panic!("{lexical}: {e}"))
}

fn canonical(result: Result<XsdValue, purrdf_xsd::XsdError>) -> String {
    result.map_or_else(
        |e| panic!("expected an answer, got {e}"),
        |v| v.canonical_lexical(),
    )
}

/// Addition whose exact result needs more digits than the mantissa holds keeps every
/// digit it can and truncates the rest toward zero; its integer part fits, so it is
/// not an overflow.
#[test]
fn decimal_addition_truncates_precision_rather_than_refusing() {
    let big = dec("1000000000000000000000000000000");
    let tiny = dec("0.000000000000000001");
    assert_eq!(
        canonical(numeric_add(&big, &tiny)),
        "1000000000000000000000000000000"
    );
    assert_eq!(
        canonical(numeric_sub(&big, &tiny)),
        "999999999999999999999999999999.99999999"
    );
    assert_eq!(
        canonical(numeric_add(&dec("-1000000000000000000000000000000"), &tiny)),
        "-999999999999999999999999999999.99999999"
    );
    // Neighbour: an exactly representable sum is exact.
    assert_eq!(canonical(numeric_add(&dec("1.25"), &dec("0.5"))), "1.75");
    // Overflow of the integer part is still refused.
    assert!(numeric_add(&dec(MAX), &dec("1")).is_err());
    assert!(numeric_add(&dec(MAX), &dec("0.5")).is_ok());
}

/// Multiplication truncates at the finest scale whose mantissa fits — the rule
/// division already follows — and overflows only on the integer part.
#[test]
fn decimal_multiplication_truncates_at_the_finest_scale_that_fits() {
    assert_eq!(
        canonical(numeric_mul(
            &dec("0.5"),
            &dec("40000000000000000000000000000000000000")
        )),
        "20000000000000000000000000000000000000"
    );
    assert_eq!(
        canonical(numeric_mul(
            &dec("12345678901234567890.123456789012345678"),
            &dec("10.5")
        )),
        "129629628462962962846.296296284629629619"
    );
    // Neighbours: an in-range product, and one truncated at scale 18 as before.
    assert_eq!(canonical(numeric_mul(&dec("1.5"), &dec("2.5"))), "3.75");
    assert_eq!(
        canonical(numeric_mul(&dec("0.000000001"), &dec("0.0000000001"))),
        "0"
    );
    assert!(numeric_mul(&dec(MAX), &dec("1.5")).is_err());
}

/// `AVG`'s finish over an arbitrary-precision running total answers one
/// representable decimal — the finest scale that fits — exactly as `numeric_div`
/// would over the same exact total.
#[test]
fn bigint_avg_truncates_at_the_finest_scale_that_fits() {
    let mut sum = BigInt::from_i128(i128::MAX);
    sum.add_i128(i128::MAX - 1);
    let avg = bigint_avg_decimal(&sum, 2).expect("the mean MAX - 0.5 truncates to MAX - 1");
    assert_eq!(avg.canonical_lexical(), MAX_1);
    // Where the total fits i128 the answer is numeric_div's.
    let total = parse("160000000000000000000000000000000000001", D::Integer).expect("fits");
    let three = parse("3", D::Integer).expect("fits");
    let quotient = numeric_div(&total, &three).expect("a quotient");
    let avg = bigint_avg_decimal(
        &BigInt::from_i128(160_000_000_000_000_000_000_000_000_000_000_000_001),
        3,
    )
    .expect("a mean");
    assert_eq!(avg.canonical_lexical(), quotient.canonical_lexical());
    assert_eq!(
        avg.canonical_lexical(),
        "53333333333333333333333333333333333333"
    );
}

// ── typed errors ────────────────────────────────────────────────────────────────

/// Every limit of the bounded representation reports its F&O code.
#[test]
fn every_limit_is_a_typed_error() {
    let code = |r: Result<XsdValue, purrdf_xsd::XsdError>| r.expect_err("a refusal").code();
    let big = "100000000000000000000000000000000000000000";
    assert_eq!(code(parse(big, D::Integer)), Some(ErrorCode::Foca0003));
    assert_eq!(
        code(parse(big, D::NonNegativeInteger)),
        Some(ErrorCode::Foca0003)
    );
    assert_eq!(code(parse(big, D::Decimal)), Some(ErrorCode::Foca0001));
    assert_eq!(
        code(parse("0.1000000000000000001", D::Decimal)),
        Some(ErrorCode::Foca0006)
    );
    assert_eq!(code(parse("300", D::Byte)), Some(ErrorCode::Forg0001));
    assert_eq!(code(parse("1.5", D::Integer)), Some(ErrorCode::Forg0001));
    let max = parse(MAX, D::Integer).expect("fits");
    let one = parse("1", D::Integer).expect("fits");
    let zero = parse("0", D::Integer).expect("fits");
    assert_eq!(code(numeric_add(&max, &one)), Some(ErrorCode::Foar0002));
    assert_eq!(code(numeric_mul(&max, &max)), Some(ErrorCode::Foar0002));
    assert_eq!(code(numeric_div(&one, &zero)), Some(ErrorCode::Foar0001));
    let min = parse(&i128::MIN.to_string(), D::Integer).expect("fits");
    assert_eq!(
        code(purrdf_xsd::numeric_unary_minus(&min)),
        Some(ErrorCode::Foar0002)
    );
    assert_eq!(
        code(purrdf_xsd::numeric_abs(&min)),
        Some(ErrorCode::Foar0002)
    );
    let dmax = dec(MAX);
    assert_eq!(
        code(numeric_add(&dmax, &dec("1"))),
        Some(ErrorCode::Foar0002)
    );
    assert_eq!(
        code(numeric_div(&dmax, &dec("0.5"))),
        Some(ErrorCode::Foar0002)
    );
    assert_eq!(
        Decimal::try_from_f64(f64::INFINITY).unwrap_err().code(),
        Some(ErrorCode::Foca0002)
    );
    assert_eq!(
        Decimal::try_from_f64(1e39).unwrap_err().code(),
        Some(ErrorCode::Foca0001)
    );
    assert_eq!(ErrorCode::Foar0002.qname(), "err:FOAR0002");
    // The neighbours of every refusal answer.
    assert!(parse(MAX, D::Integer).is_ok());
    assert!(parse("0.100000000000000000", D::Decimal).is_ok());
    // Trailing fractional zeros are spelling, not precision.
    assert_eq!(
        parse(&format!("0.1{}", "0".repeat(40)), D::Decimal)
            .expect("the value 0.1")
            .canonical_lexical(),
        "0.1"
    );
    assert!(parse("127", D::Byte).is_ok());
    assert!(numeric_add(&parse(MAX_1, D::Integer).expect("fits"), &one).is_ok());
    assert!(Decimal::try_from_f64(1e38).is_ok());
}

/// The negation and the absolute value of a derived integer type are `xsd:integer`
/// (F&O 3.1 §4.2): `-5` is not an `xsd:unsignedByte`, and keeping the subtype would
/// mint a literal outside its own value space.
#[test]
fn unary_results_of_a_derived_integer_are_xsd_integer() {
    let five = parse("5", D::UnsignedByte).expect("fits");
    let negated = purrdf_xsd::numeric_unary_minus(&five).expect("fits");
    assert_eq!(negated.datatype(), D::Integer);
    assert_eq!(negated.canonical_lexical(), "-5");
    let negative = parse("-5", D::NegativeInteger).expect("fits");
    let absolute = purrdf_xsd::numeric_abs(&negative).expect("fits");
    assert_eq!(absolute.datatype(), D::Integer);
    for op in [
        purrdf_xsd::numeric_ceil,
        purrdf_xsd::numeric_floor,
        purrdf_xsd::numeric_round,
    ] {
        assert_eq!(op(&five).expect("identity").datatype(), D::Integer);
    }
    // Neighbour: unary plus is the identity, type included.
    assert_eq!(
        purrdf_xsd::numeric_unary_plus(&five)
            .expect("identity")
            .datatype(),
        D::UnsignedByte
    );
}

// ── the exact oracle ────────────────────────────────────────────────────────────

/// A decimal numeral of any size: up to sixty integer digits (far past `i128`) and
/// up to forty-five fractional digits (far past eighteen), with optional leading
/// zeros and sign.
fn numeral() -> impl Strategy<Value = String> {
    prop::string::regex("[+-]?0{0,2}[0-9]{1,60}(\\.[0-9]{0,45})?")
}

/// A numeral the bounded representation mostly holds, so mixed pairs are common.
fn small_numeral() -> impl Strategy<Value = String> {
    prop::string::regex("[+-]?[0-9]{1,20}(\\.[0-9]{0,18})?")
}

fn oracle(lexical: &str) -> Rational {
    Rational::parse(lexical).unwrap_or_else(|| panic!("{lexical} is a numeral"))
}

fn digits(lexical: &str) -> DecimalDigits {
    DecimalDigits::parse(lexical).unwrap_or_else(|| panic!("{lexical} is a decimal lexical"))
}

/// A numeral as the literal value it denotes: `xsd:integer` when it has no point.
fn literal(lexical: &str) -> LiteralValue {
    let datatype = if lexical.contains('.') {
        D::Decimal
    } else {
        D::Integer
    };
    LiteralValue::parse(lexical, datatype).unwrap_or_else(|e| panic!("{lexical}: {e}"))
}

/// A finite `f64` from raw bits (`None` for `NaN` and the infinities).
fn finite(bits: u64) -> Option<f64> {
    let value = f64::from_bits(bits);
    value.is_finite().then_some(value)
}

prop_test! {
    #![prop_config(Config::with_cases(CASES))]

    /// Comparison and equality of numerals of any size are the oracle's, and the
    /// canonical form denotes the same value and reads back to the same digits.
    #[test]
    fn digit_order_and_equality_are_exact(a in numeral(), b in numeral()) {
        let (da, db) = (digits(&a), digits(&b));
        let (ra, rb) = (oracle(&a), oracle(&b));
        prop_assert_eq!(da.cmp(&db), ra.cmp_value(&rb), "{} vs {}", a, b);
        prop_assert_eq!(da == db, ra.value_eq(&rb), "{} == {}", a, b);
        let canonical = da.canonical_lexical();
        prop_assert!(oracle(&canonical).value_eq(&ra), "{} -> {}", a, canonical);
        prop_assert_eq!(digits(&canonical), da);
        // Exact digit counts.
        let unsigned = canonical.trim_start_matches('-');
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        let whole_digits = whole.trim_start_matches('0').len() as u64;
        prop_assert_eq!(da.integer_digits(), whole_digits);
        prop_assert_eq!(da.fraction_digits(), fraction.len() as u64);
        // The bounded projections are exact or absent.
        let representable = da.fraction_digits() <= 18
            && ra.truncate_at_scale(da.fraction_digits() as u32).is_some();
        prop_assert_eq!(da.to_decimal().is_some(), representable, "{}", a);
        if let Some(bounded) = da.to_decimal() {
            prop_assert!(Rational::from_decimal(bounded.mantissa(), u32::from(bounded.scale())).value_eq(&ra));
        }
        match da.truncate_to_i128() {
            Some(whole) => prop_assert_eq!(Some(whole), ra.truncate_toward_zero()),
            None => prop_assert_eq!(None, ra.truncate_toward_zero()),
        }
    }

    /// The literal order, its equality and the sort order agree with the oracle
    /// over pairs that straddle the bounded representation.
    #[test]
    fn literal_order_is_exact_across_the_bounds(
        a in prop_oneof![numeral(), small_numeral()],
        b in prop_oneof![numeral(), small_numeral()],
    ) {
        let (la, lb) = (literal(&a), literal(&b));
        let want = oracle(&a).cmp_value(&oracle(&b));
        prop_assert_eq!(literal_cmp(&la, &lb), Some(want), "{} vs {}", a, b);
        prop_assert_eq!(literal_total_cmp(&la, &lb), Some(want), "{} vs {}", a, b);
        prop_assert_eq!(literal_equal(&la, &lb), Some(want == Ordering::Equal));
    }

    /// Against a binary floating-point value, the sort order is exact and the
    /// SPARQL `<` compares the correctly rounded conversion.
    #[test]
    fn literal_order_against_binary_floating_point(a in numeral(), bits in any::<u64>()) {
        let la = literal(&a);
        let exact = oracle(&a);
        // The doubles that matter most: the nearest one and its two neighbours, where
        // only an exact comparison can tell the values apart.
        let nearest = exact.to_f64();
        for neighbour in [nearest, nearest.next_up(), nearest.next_down()] {
            if neighbour.is_finite() {
                let ld = LiteralValue::Bounded(XsdValue::Double(neighbour));
                prop_assert_eq!(
                    literal_total_cmp(&la, &ld),
                    Some(exact.cmp_value(&Rational::from_f64(neighbour))),
                    "{} vs {:e}", a, neighbour
                );
            }
        }
        let Some(double) = finite(bits) else { return Ok(()); };
        let ld = LiteralValue::Bounded(XsdValue::Double(double));
        prop_assert_eq!(
            literal_total_cmp(&la, &ld),
            Some(exact.cmp_value(&Rational::from_f64(double))),
            "{} vs {:e}", a, double
        );
        prop_assert_eq!(literal_cmp(&la, &ld), exact.to_f64().partial_cmp(&double));
        let float = f32::from_bits((bits >> 32) as u32);
        if float.is_finite() {
            let lf = LiteralValue::Bounded(XsdValue::Float(float));
            prop_assert_eq!(
                literal_total_cmp(&la, &lf),
                Some(exact.cmp_value(&Rational::from_f32(float)))
            );
            prop_assert_eq!(literal_cmp(&la, &lf), exact.to_f32().partial_cmp(&float));
        }
    }

    /// Conversion to `f64`/`f32` is correctly rounded from all the digits.
    #[test]
    fn conversion_is_correctly_rounded(a in numeral()) {
        let d = digits(&a);
        let exact = oracle(&a);
        if d.is_zero() {
            prop_assert_eq!(d.to_f64().to_bits(), 0.0_f64.to_bits());
            prop_assert_eq!(d.to_f32().to_bits(), 0.0_f32.to_bits());
        } else {
            prop_assert_eq!(d.to_f64().to_bits(), exact.to_f64().to_bits(), "{}", a);
            prop_assert_eq!(d.to_f32().to_bits(), exact.to_f32().to_bits(), "{}", a);
        }
    }

    /// A sort by the literal total order is the oracle's order (and so transitive)
    /// over a bag mixing values past the bounds, bounded values and doubles.
    #[test]
    fn sorting_by_the_total_order_is_the_oracle_order(
        lexicals in prop::collection::vec(prop_oneof![numeral(), small_numeral()], 2..24),
        bits in prop::collection::vec(any::<u64>(), 0..8),
    ) {
        let mut bag: Vec<(LiteralValue, Rational)> = lexicals
            .iter()
            .map(|l| (literal(l), oracle(l)))
            .collect();
        bag.extend(bits.into_iter().filter_map(finite).map(|d| {
            (LiteralValue::Bounded(XsdValue::Double(d)), Rational::from_f64(d))
        }));
        bag.sort_by(|x, y| literal_total_cmp(&x.0, &y.0).expect("no NaN in the bag"));
        for pair in bag.windows(2) {
            prop_assert!(pair[0].1.cmp_value(&pair[1].1) != Ordering::Greater);
        }
    }
}

// ── decimal arithmetic under the precision rule ─────────────────────────────────

/// The bounded answer the precision rule gives for the exact value `exact`: the
/// mantissa truncated toward zero at the finest scale `≤ finest` that fits `i128`,
/// or `None` (an overflow) when not even scale 0 does.
fn bounded(exact: &Rational, finest: u32) -> Option<Rational> {
    (0..=finest).rev().find_map(|scale| {
        exact
            .truncate_at_scale(scale)
            .map(|m| Rational::from_decimal(m, scale))
    })
}

/// A bounded decimal operand: up to thirty-eight digits, up to eighteen of them
/// fractional; `None` when it overflows the mantissa.
fn operand() -> impl Strategy<Value = String> {
    prop::string::regex("[+-]?[0-9]{1,38}(\\.[0-9]{0,18})?")
}

fn scale_of(lexical: &str) -> u32 {
    lexical.split_once('.').map_or(0, |(_, f)| f.len() as u32)
}

fn assert_rule(result: Result<XsdValue, purrdf_xsd::XsdError>, want: Option<Rational>, what: &str) {
    match (result, want) {
        (Ok(XsdValue::Decimal(got)), Some(want)) => assert!(
            Rational::from_decimal(got.mantissa(), u32::from(got.scale())).value_eq(&want),
            "{what}: {} is not the truncation the rule names",
            got.canonical_lexical()
        ),
        (Err(error), None) => assert_eq!(error.code(), Some(ErrorCode::Foar0002), "{what}"),
        (got, want) => panic!("{what}: got {got:?}, want {want:?}"),
    }
}

prop_test! {
    #![prop_config(Config::with_cases(CASES))]

    /// `+`, `−`, `×` and `÷` over bounded decimals: the exact result where it is
    /// representable, the rule's truncation where it is not, and `err:FOAR0002`
    /// exactly when the integer part does not fit.
    #[test]
    fn decimal_arithmetic_follows_the_precision_rule(a in operand(), b in operand()) {
        let (Ok(xa), Ok(xb)) = (parse(&a, D::Decimal), parse(&b, D::Decimal)) else {
            return Ok(());
        };
        let (ra, rb) = (oracle(&a), oracle(&b));
        let (sa, sb) = (scale_of(&a), scale_of(&b));
        assert_rule(numeric_add(&xa, &xb), bounded(&ra.add(&rb), sa.max(sb)), &format!("{a} + {b}"));
        assert_rule(numeric_sub(&xa, &xb), bounded(&ra.sub(&rb), sa.max(sb)), &format!("{a} - {b}"));
        assert_rule(numeric_mul(&xa, &xb), bounded(&ra.mul(&rb), (sa + sb).min(18)), &format!("{a} * {b}"));
        if rb.cmp_value(&Rational::from_i128(0)) != Ordering::Equal {
            assert_rule(numeric_div(&xa, &xb), bounded(&ra.div(&rb), 18), &format!("{a} / {b}"));
        }
    }

    /// The mean of an exact running total of any size is the rule's truncation of
    /// the exact mean, and equals `numeric_div(total, count)` whenever the total is a
    /// bounded value: `AVG` agrees with `SUM / COUNT`.
    #[test]
    fn the_mean_is_the_truncated_exact_mean(
        values in prop::collection::vec(prop_oneof![operand(), numeral()], 1..12),
    ) {
        let scale = values.iter().map(|v| scale_of(v)).max().unwrap_or(0);
        let mut sum = BigInt::zero();
        let mut exact = Rational::from_i128(0);
        for value in &values {
            let body = value.replace('.', "");
            let mantissa = BigInt::from_digits(&body).expect("digits");
            sum.add_assign(&mantissa.mul_pow10(scale - scale_of(value)));
            exact = exact.add(&oracle(value));
        }
        let count = values.len() as u64;
        let mean = exact.div(&Rational::from_i128(i128::from(count as u32)));
        let got = decimal_mean(&sum, scale, count);
        assert_rule(got.clone().map(XsdValue::Decimal), bounded(&mean, 18), &format!("mean of {values:?}"));
        if let Ok(total) = parse(&sum.to_decimal_lexical(scale), D::Decimal) {
            let count = parse(&count.to_string(), D::Integer).expect("fits");
            match (got, numeric_div(&total, &count)) {
                (Ok(m), Ok(XsdValue::Decimal(q))) => {
                    prop_assert_eq!(m.canonical_lexical(), q.canonical_lexical());
                }
                (m, q) => prop_assert!(false, "AVG {:?} vs SUM/COUNT {:?}", m, q),
            }
        }
    }
}

/// Every error kind presents its condition and, where F&O names one, its code — in
/// the typed presentation and in the `Display` text every surface reports.
#[test]
fn every_error_presents_its_code() {
    use purrdf_xsd::XsdError;
    for (error, identity, code) in [
        (
            parse("x", D::Integer).unwrap_err(),
            "xsd-invalid-lexical",
            Some("err:FORG0001"),
        ),
        (
            parse("100000000000000000000000000000000000000000", D::Integer).unwrap_err(),
            "xsd-out-of-range",
            Some("err:FOCA0003"),
        ),
        (
            XsdError::DivisionByZero {
                datatype: D::Decimal,
            },
            "xsd-division-by-zero",
            Some("err:FOAR0001"),
        ),
        (
            XsdError::TypeMismatch { reason: "a reason" },
            "xsd-type-mismatch",
            Some("err:XPTY0004"),
        ),
        (
            XsdError::Indeterminate { reason: "a reason" },
            "xsd-indeterminate",
            None,
        ),
    ] {
        let presentation = error.presentation();
        assert_eq!(presentation.message_id(), identity);
        assert_eq!(presentation.english(), error.to_string());
        match code {
            Some(code) => {
                assert!(error.to_string().ends_with(&format!("({code})")), "{error}");
                assert!(presentation.parameters().iter().any(|p| p.name() == "code"));
            }
            None => assert!(!presentation.parameters().iter().any(|p| p.name() == "code")),
        }
    }
}
