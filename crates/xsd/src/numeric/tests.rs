use std::cmp::Ordering;

use super::*;
use crate::XsdDatatype as D;
use crate::exact::Decimal;

fn dec(s: &str) -> Decimal {
    parse_decimal(s).unwrap()
}

fn int_val(n: i128) -> XsdValue {
    XsdValue::integer(n)
}

/// The decimal `mantissa × 10^-scale`.
fn fixed(mantissa: i128, scale: u32) -> Decimal {
    Decimal::new(exact::Integer::from_i128(mantissa), scale)
}

// ── the exact numeric order ─────────────────────────────────────────────

/// THE CYCLE the exact order exists to close.
///
/// Under §17.3 promotion these three literals form `a > b`, `a = c`, `b = c`,
/// which no transitive relation can hold — and `ORDER BY` hands its comparator
/// to a Rust sort, which may panic on one. The exact order sees the nineteenth
/// significant digit and answers consistently.
#[test]
fn the_promotion_cycle_at_nineteen_significant_digits_is_closed() {
    let a = XsdValue::Decimal(dec("1.000000000000000001"));
    let b = int_val(1);
    let c = XsdValue::Double(1.0);

    // The promoted relation, as SPARQL `<` still means it.
    assert_eq!(numeric_cmp(&a, &b), Some(Ordering::Greater));
    assert_eq!(numeric_cmp(&a, &c), Some(Ordering::Equal));
    assert_eq!(numeric_cmp(&b, &c), Some(Ordering::Equal));

    // The exact relation, which `ORDER BY` uses.
    assert_eq!(numeric_total_cmp(&a, &b), Some(Ordering::Greater));
    assert_eq!(numeric_total_cmp(&a, &c), Some(Ordering::Greater));
    assert_eq!(numeric_total_cmp(&b, &c), Some(Ordering::Equal));
    assert_eq!(numeric_total_cmp(&c, &a), Some(Ordering::Less));
}

/// The same failure one type up: `2^53 + 1` is an ordinary `xsd:integer` and an
/// unrepresentable `xsd:double`, so promotion rounds it onto `2^53`.
#[test]
fn an_integer_beyond_the_double_significand_stays_distinct() {
    let big = int_val((1i128 << 53) + 1);
    let rounded = XsdValue::Double((1u64 << 53) as f64);
    assert_eq!(numeric_cmp(&big, &rounded), Some(Ordering::Equal));
    assert_eq!(numeric_total_cmp(&big, &rounded), Some(Ordering::Greater));
    assert_eq!(numeric_total_cmp(&rounded, &big), Some(Ordering::Less));
    // And the same magnitude one below is genuinely smaller, both ways.
    let smaller = int_val((1i128 << 53) - 1);
    assert_eq!(numeric_total_cmp(&smaller, &rounded), Some(Ordering::Less));
}

/// `xsd:float` is exercised on the same seam, and `f32 → f64` widening must not
/// be what decides it: `0.1f32` is a DIFFERENT rational from `0.1` the decimal.
#[test]
fn a_float_operand_is_compared_as_the_dyadic_rational_it_is() {
    let tenth = XsdValue::Decimal(dec("0.1"));
    let float_tenth = XsdValue::Float(0.1f32);
    let double_tenth = XsdValue::Double(0.1f64);
    // 0.1f32 = 13421773 / 2^27 > 1/10; 0.1f64 = 3602879701896397 / 2^55 > 1/10.
    assert_eq!(
        numeric_total_cmp(&tenth, &float_tenth),
        Some(Ordering::Less)
    );
    assert_eq!(
        numeric_total_cmp(&tenth, &double_tenth),
        Some(Ordering::Less)
    );
    // ...and the f32 rounding is coarser, so it overshoots further.
    assert_eq!(
        numeric_total_cmp(&double_tenth, &float_tenth),
        Some(Ordering::Less)
    );
    // A dyadic decimal is EXACTLY its IEEE counterpart, in both widths.
    let half = XsdValue::Decimal(dec("0.5"));
    assert_eq!(
        numeric_total_cmp(&half, &XsdValue::Float(0.5f32)),
        Some(Ordering::Equal)
    );
    assert_eq!(
        numeric_total_cmp(&half, &XsdValue::Double(0.5f64)),
        Some(Ordering::Equal)
    );
}

/// The specials keep their places, and `NaN` keeps its absence of one.
#[test]
fn the_specials_and_the_zeroes_order_as_the_value_space_says() {
    let one = int_val(1);
    for infinite in [
        XsdValue::Double(f64::INFINITY),
        XsdValue::Float(f32::INFINITY),
    ] {
        assert_eq!(numeric_total_cmp(&one, &infinite), Some(Ordering::Less));
        assert_eq!(numeric_total_cmp(&infinite, &one), Some(Ordering::Greater));
    }
    for infinite in [
        XsdValue::Double(f64::NEG_INFINITY),
        XsdValue::Float(f32::NEG_INFINITY),
    ] {
        assert_eq!(numeric_total_cmp(&one, &infinite), Some(Ordering::Greater));
        assert_eq!(numeric_total_cmp(&infinite, &one), Some(Ordering::Less));
    }
    assert_eq!(
        numeric_total_cmp(
            &XsdValue::Double(f64::INFINITY),
            &XsdValue::Float(f32::INFINITY)
        ),
        Some(Ordering::Equal)
    );
    // Both zeroes are one value, whichever spelling and whichever branch.
    for zero in [
        XsdValue::Double(0.0),
        XsdValue::Double(-0.0),
        XsdValue::Float(-0.0f32),
    ] {
        assert_eq!(numeric_total_cmp(&int_val(0), &zero), Some(Ordering::Equal));
        assert_eq!(
            numeric_total_cmp(&XsdValue::Decimal(dec("0.000")), &zero),
            Some(Ordering::Equal)
        );
    }
    // NaN has no place in the order at all — never a silent "greater".
    for nan in [XsdValue::Double(f64::NAN), XsdValue::Float(f32::NAN)] {
        assert_eq!(numeric_total_cmp(&one, &nan), None);
        assert_eq!(numeric_total_cmp(&nan, &one), None);
        assert_eq!(numeric_total_cmp(&nan, &nan), None);
    }
}

/// The magnitude window and the `BigInt` fallback: a double far outside the
/// exact side's reach, a subnormal far under it, and a pair close enough that
/// the answer needs the exact cross-multiplication.
#[test]
fn magnitudes_outside_and_inside_the_aliasing_window_both_decide_exactly() {
    let biggest = XsdValue::Decimal(fixed(i128::MAX, 0));
    let smallest = XsdValue::Decimal(fixed(1, 18));
    let most_negative = XsdValue::Decimal(fixed(i128::MIN, 0));

    // Outside the window on the high side: 1e300 dwarfs any i128 mantissa.
    assert_eq!(
        numeric_total_cmp(&biggest, &XsdValue::Double(1e300)),
        Some(Ordering::Less)
    );
    assert_eq!(
        numeric_total_cmp(&most_negative, &XsdValue::Double(-1e300)),
        Some(Ordering::Greater)
    );
    // Outside on the low side: the smallest subnormal is under 1e-18.
    assert_eq!(
        numeric_total_cmp(&smallest, &XsdValue::Double(f64::from_bits(1))),
        Some(Ordering::Greater)
    );
    assert_eq!(
        numeric_total_cmp(&XsdValue::Double(f64::from_bits(1)), &smallest),
        Some(Ordering::Less)
    );

    // Inside the window and genuinely close: 2^127 as a double is exactly
    // representable, and i128::MAX = 2^127 - 1 sits one below it.
    let two_127 = XsdValue::Double(170_141_183_460_469_231_731_687_303_715_884_105_728.0);
    assert_eq!(numeric_total_cmp(&biggest, &two_127), Some(Ordering::Less));
    assert_eq!(
        numeric_total_cmp(
            &most_negative,
            &XsdValue::Double(-170_141_183_460_469_231_731_687_303_715_884_105_728.0)
        ),
        Some(Ordering::Equal),
        "i128::MIN is exactly -2^127, which is exactly a double"
    );
    // A dyadic decimal at full scale still lands exactly on its double.
    let five_tenths_at_scale_18 = XsdValue::Decimal(fixed(500_000_000_000_000_000, 18));
    assert_eq!(
        numeric_total_cmp(&five_tenths_at_scale_18, &XsdValue::Double(0.5)),
        Some(Ordering::Equal)
    );
}

/// The two ways a `u128` cross-multiplication runs out of room, both still
/// inside the aliasing window so the cheap bound cannot answer for them. The
/// `BigInt` fallback is what decides these, and it must decide them exactly.
#[test]
fn the_bigint_fallback_answers_the_pairs_that_overflow_a_u128() {
    // Non-negative IEEE exponent: `significand × 2^exp × 10^18` exceeds 2^128
    // whenever |ieee| is past ~3.4e20, while the exact side at scale 18 tops out
    // near 1.7e20.
    let biggest_at_full_scale = XsdValue::Decimal(fixed(i128::MAX, 18));
    assert_eq!(
        numeric_total_cmp(&biggest_at_full_scale, &XsdValue::Double(1e21)),
        Some(Ordering::Less)
    );
    assert_eq!(
        numeric_total_cmp(&XsdValue::Double(1e21), &biggest_at_full_scale),
        Some(Ordering::Greater)
    );

    // Negative IEEE exponent: `mantissa × 2^-exp` exceeds 2^128 once the mantissa
    // is near the i128 ceiling and the double needs two or more fractional bits
    // (`0.5` alone would not — `(2^127 - 1) << 1` still fits `u128`).
    let biggest = XsdValue::Decimal(fixed(i128::MAX, 0));
    assert_eq!(
        numeric_total_cmp(&biggest, &XsdValue::Double(0.25)),
        Some(Ordering::Greater)
    );
    assert_eq!(
        numeric_total_cmp(&XsdValue::Double(0.25), &biggest),
        Some(Ordering::Less)
    );
    // And the near-tie, one ulp of the EXACT side at a magnitude where a
    // promotion through `f64` could not have resolved it at all: `2^126` is
    // exactly a double, and the two decimals either side of it are not.
    let two_126 = XsdValue::Decimal(fixed(1i128 << 126, 0));
    assert_eq!(
        numeric_total_cmp(
            &two_126,
            &XsdValue::Double(85_070_591_730_234_615_865_843_651_857_942_052_864.0)
        ),
        Some(Ordering::Equal)
    );
    assert_eq!(
        numeric_total_cmp(
            &XsdValue::Decimal(fixed((1i128 << 126) - 1, 0)),
            &XsdValue::Double(85_070_591_730_234_615_865_843_651_857_942_052_864.0)
        ),
        Some(Ordering::Less)
    );
    assert_eq!(
        numeric_total_cmp(
            &XsdValue::Decimal(fixed((1i128 << 126) + 1, 0)),
            &XsdValue::Double(85_070_591_730_234_615_865_843_651_857_942_052_864.0)
        ),
        Some(Ordering::Greater)
    );
}

/// Exhaustive antisymmetry and transitivity over a sample that spans every
/// branch of the tower and every seam between them. A cycle anywhere here is a
/// sort that may abort the process, so this is the property that must not rot.
#[test]
fn the_exact_numeric_order_is_a_total_order_over_the_whole_tower() {
    let samples = vec![
        XsdValue::Double(f64::NEG_INFINITY),
        XsdValue::Float(f32::NEG_INFINITY),
        XsdValue::Decimal(fixed(i128::MIN, 0)),
        int_val(-3),
        XsdValue::Decimal(dec("-1.000000000000000001")),
        XsdValue::Float(-1.0f32),
        int_val(-1),
        XsdValue::Double(-0.0),
        int_val(0),
        XsdValue::Decimal(dec("0.000")),
        XsdValue::Double(f64::from_bits(1)),
        XsdValue::Decimal(fixed(1, 18)),
        XsdValue::Decimal(dec("0.1")),
        XsdValue::Double(0.1f64),
        XsdValue::Float(0.1f32),
        XsdValue::Decimal(dec("0.5")),
        XsdValue::Float(0.5f32),
        int_val(1),
        XsdValue::Double(1.0),
        XsdValue::Float(1.0f32),
        XsdValue::Decimal(dec("1.000000000000000001")),
        int_val((1i128 << 53) - 1),
        XsdValue::Double((1u64 << 53) as f64),
        int_val((1i128 << 53) + 1),
        XsdValue::Decimal(fixed(i128::MAX, 0)),
        XsdValue::Double(1e300),
        XsdValue::Double(f64::INFINITY),
        XsdValue::Float(f32::INFINITY),
    ];

    for (i, a) in samples.iter().enumerate() {
        for (j, b) in samples.iter().enumerate() {
            let forward = numeric_total_cmp(a, b).expect("no NaN in the sample");
            let back = numeric_total_cmp(b, a).expect("no NaN in the sample");
            assert_eq!(forward, back.reverse(), "not antisymmetric at ({i}, {j})");
        }
    }
    for (i, a) in samples.iter().enumerate() {
        for (j, b) in samples.iter().enumerate() {
            if numeric_total_cmp(a, b) != Some(Ordering::Less)
                && numeric_total_cmp(a, b) != Some(Ordering::Equal)
            {
                continue;
            }
            for (k, c) in samples.iter().enumerate() {
                if numeric_total_cmp(b, c) == Some(Ordering::Greater) {
                    continue;
                }
                assert_ne!(
                    numeric_total_cmp(a, c),
                    Some(Ordering::Greater),
                    "not transitive at ({i}, {j}, {k})"
                );
            }
        }
    }

    // And the whole sample sorts through it without tripping the total-order
    // check `sort_by` runs in debug builds.
    let mut order: Vec<usize> = (0..samples.len()).collect();
    order.sort_by(|x, y| {
        numeric_total_cmp(&samples[*x], &samples[*y]).expect("no NaN in the sample")
    });
    assert_eq!(order.len(), samples.len());
}

/// The promoted relation is left EXACTLY as it was: this is the divergence
/// stated as a test, so nobody can "tidy" `numeric_cmp` into the exact order and
/// silently change what SPARQL `<` and `=` mean.
#[test]
fn the_promotion_based_operator_relation_is_untouched() {
    let a = XsdValue::Decimal(dec("1.000000000000000001"));
    let c = XsdValue::Double(1.0);
    assert!(
        numeric_eq(&a, &c),
        "SPARQL `=` still promotes, and still rounds"
    );
    assert_eq!(numeric_cmp(&a, &c), Some(Ordering::Equal));
    assert_ne!(numeric_total_cmp(&a, &c), numeric_cmp(&a, &c));
}

#[test]
fn integer_parse_and_bounds() {
    assert_eq!(parse_integer("42").unwrap().as_i128().unwrap(), 42);
    assert_eq!(parse_integer("-7").unwrap().as_i128().unwrap(), -7);
    assert_eq!(parse_integer("+7").unwrap().as_i128().unwrap(), 7);
    assert_eq!(parse_integer("007").unwrap().as_i128().unwrap(), 7);
    assert_eq!(
        parse_integer(&i128::MAX.to_string())
            .unwrap()
            .as_i128()
            .unwrap(),
        i128::MAX
    );
    // i128::MAX + 1 is an integer like any other, held past the machine word.
    let past = parse_integer("170141183460469231731687303715884105728").unwrap();
    assert_eq!(past.as_i128(), None);
    assert_eq!(
        past.canonical_lexical(),
        "170141183460469231731687303715884105728"
    );
    assert!(parse_integer("1.0").is_err());
    assert!(parse_integer("").is_err());
    assert!(parse_integer("abc").is_err());
}

#[test]
fn parse_integer_typed_range_checks() {
    // xsd:byte: -128..127
    assert_eq!(
        parse_integer_typed("127", D::Byte)
            .unwrap()
            .as_i128()
            .unwrap(),
        127
    );
    assert_eq!(
        parse_integer_typed("-128", D::Byte)
            .unwrap()
            .as_i128()
            .unwrap(),
        -128
    );
    assert!(parse_integer_typed("128", D::Byte).is_err());
    assert!(parse_integer_typed("-129", D::Byte).is_err());

    // xsd:unsignedByte: 0..255
    assert_eq!(
        parse_integer_typed("255", D::UnsignedByte)
            .unwrap()
            .as_i128()
            .unwrap(),
        255
    );
    assert_eq!(
        parse_integer_typed("0", D::UnsignedByte)
            .unwrap()
            .as_i128()
            .unwrap(),
        0
    );
    assert!(parse_integer_typed("256", D::UnsignedByte).is_err());
    assert!(parse_integer_typed("-1", D::UnsignedByte).is_err());

    // xsd:positiveInteger: >= 1
    assert_eq!(
        parse_integer_typed("1", D::PositiveInteger)
            .unwrap()
            .as_i128()
            .unwrap(),
        1
    );
    assert!(parse_integer_typed("0", D::PositiveInteger).is_err());

    // xsd:negativeInteger: <= -1
    assert_eq!(
        parse_integer_typed("-1", D::NegativeInteger)
            .unwrap()
            .as_i128()
            .unwrap(),
        -1
    );
    assert!(parse_integer_typed("0", D::NegativeInteger).is_err());

    // xsd:nonNegativeInteger: >= 0
    assert_eq!(
        parse_integer_typed("0", D::NonNegativeInteger)
            .unwrap()
            .as_i128()
            .unwrap(),
        0
    );
    assert!(parse_integer_typed("-1", D::NonNegativeInteger).is_err());

    // xsd:nonPositiveInteger: <= 0
    assert_eq!(
        parse_integer_typed("0", D::NonPositiveInteger)
            .unwrap()
            .as_i128()
            .unwrap(),
        0
    );
    assert!(parse_integer_typed("1", D::NonPositiveInteger).is_err());

    // xsd:unsignedLong boundary: u64::MAX should pass; u64::MAX+1 should fail
    let u64max = u64::MAX.to_string();
    assert_eq!(
        parse_integer_typed(&u64max, D::UnsignedLong)
            .unwrap()
            .as_i128()
            .unwrap(),
        i128::from(u64::MAX)
    );
    assert!(parse_integer_typed("18446744073709551616", D::UnsignedLong).is_err());

    // xsd:int: 2147483647 ok, 2147483648 fails
    assert_eq!(
        parse_integer_typed("2147483647", D::Int)
            .unwrap()
            .as_i128()
            .unwrap(),
        2_147_483_647
    );
    assert!(parse_integer_typed("2147483648", D::Int).is_err());
}

#[test]
fn decimal_parse_and_canonical() {
    assert_eq!(dec("12.34").canonical_lexical(), "12.34");
    // XSD 1.1 §E.1 `decimalCanonicalMap`: integer-valued decimals have no point.
    assert_eq!(dec("12.00").canonical_lexical(), "12");
    assert_eq!(dec("100").canonical_lexical(), "100");
    assert_eq!(dec("-0.5").canonical_lexical(), "-0.5");
    assert_eq!(dec(".5").canonical_lexical(), "0.5");
    assert_eq!(dec("1.").canonical_lexical(), "1");
    assert_eq!(dec("0.005").canonical_lexical(), "0.005");
    assert!(parse_decimal("1.2.3").is_err());
    assert!(parse_decimal("").is_err());
}

#[test]
fn round_decimal_nearest_ties_toward_positive_infinity() {
    // `fn:round` = floor(x + 0.5): round to nearest, ties toward +infinity.
    let round = |s: &str| as_decimal(&numeric_round(&dec_val(s)).unwrap());
    // Positive: half rounds up, sub-half stays.
    assert_eq!(round("2.5").cmp(&dec("3")), Ordering::Equal);
    assert_eq!(round("1.1").cmp(&dec("1")), Ordering::Equal);
    // Negative below the half-point rounds AWAY from zero (the fixed bug:
    // `-1.6` must be `-2`, not `-1`).
    assert_eq!(round("-1.6").cmp(&dec("-2")), Ordering::Equal);
    // Negative sub-half stays toward zero.
    assert_eq!(round("-1.4").cmp(&dec("-1")), Ordering::Equal);
    // Negative ties round toward +infinity (toward zero): `-1.5` → `-1`,
    // `-2.5` → `-2`.
    assert_eq!(round("-1.5").cmp(&dec("-1")), Ordering::Equal);
    assert_eq!(round("-2.5").cmp(&dec("-2")), Ordering::Equal);
}

#[test]
fn decimal_exact_comparison_across_scales() {
    assert_eq!(dec("1.5").cmp(&dec("1.50")), Ordering::Equal);
    assert_eq!(dec("1.5").cmp(&dec("1.05")), Ordering::Greater);
    assert_eq!(dec("0.1").cmp(&dec("0.2")), Ordering::Less);
}

// ── cmp_exact correctness tests ──────────────────────────────────────────────

/// Cross-scale equality: 1.50 (mantissa=150, scale=2) == 1.5 (mantissa=15, scale=1).
#[test]
fn cmp_exact_cross_scale_equal() {
    let a = fixed(150, 2); // 1.50
    let b = fixed(15, 1); // 1.5
    assert_eq!(a.cmp(&b), Ordering::Equal);
    assert_eq!(b.cmp(&a), Ordering::Equal);
}

/// Cross-scale strict order: 1.5 < 1.50001.
#[test]
fn cmp_exact_cross_scale_strict() {
    let a = dec("1.5");
    let b = dec("1.50001");
    assert_eq!(a.cmp(&b), Ordering::Less);
    assert_eq!(b.cmp(&a), Ordering::Greater);
}

/// Negative cross-scale: -1.5 vs -1.50001.
/// -1.50001 < -1.5 (more negative).
#[test]
fn cmp_exact_negative_cross_scale() {
    let a = dec("-1.5");
    let b = dec("-1.50001");
    assert_eq!(a.cmp(&b), Ordering::Greater); // -1.5 > -1.50001
    assert_eq!(b.cmp(&a), Ordering::Less);
}

/// Mixed signs: any positive > any negative.
#[test]
fn cmp_exact_mixed_signs() {
    assert_eq!(dec("0.001").cmp(&dec("-999.9")), Ordering::Greater);
    assert_eq!(dec("-0.001").cmp(&dec("999.9")), Ordering::Less);
}

/// Both-zero regardless of scale.
#[test]
fn cmp_exact_zero_any_scale() {
    let z0 = fixed(0, 0);
    let z5 = fixed(0, 5);
    let z18 = fixed(0, 18);
    assert_eq!(z0.cmp(&z5), Ordering::Equal);
    assert_eq!(z5.cmp(&z18), Ordering::Equal);
    assert_eq!(z18.cmp(&z0), Ordering::Equal);
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
    let a = fixed(i128::MAX, 0);
    let b = fixed(i128::MAX, 1);
    assert_eq!(a.cmp(&b), Ordering::Greater);
    assert_eq!(b.cmp(&a), Ordering::Less);
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
    let a = fixed(1_000_000_000_000_000_001, 1);
    let b = fixed(1_000_000_000_000_000_002, 1);
    // Both collapse to the same f64 — the old path returns Equal incorrectly.
    assert_eq!(a.to_f64(), b.to_f64(), "f64 collapse precondition");
    // cmp_exact must return Less (A < B), not Equal.
    assert_eq!(a.cmp(&b), Ordering::Less);
    assert_eq!(b.cmp(&a), Ordering::Greater);
}

/// Same as above but across scales (scale 1 vs scale 2).
#[test]
fn cmp_exact_large_f64_collapse_cross_scale_regression() {
    // A = 100000000000000000.1  (scale 1)
    // B = 100000000000000000.11 (scale 2) = 10000000000000000011 mantissa
    // A < B (0.1 < 0.11).  Both f64-identical at this magnitude.
    let a = fixed(1_000_000_000_000_000_001, 1); // .1 at scale 1
    let b = fixed(10_000_000_000_000_000_011, 2); // .11 at scale 2
    assert_eq!(a.to_f64(), b.to_f64(), "f64 collapse precondition");
    assert_eq!(a.cmp(&b), Ordering::Less);
    assert_eq!(b.cmp(&a), Ordering::Greater);
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
fn parse_float_still_accepts_plus_inf() {
    // Regression guard: the XSD 1.1 default parse must keep accepting `+INF`.
    assert_eq!(parse_float("+INF").unwrap(), f32::INFINITY);
}

#[test]
fn parse_double_still_accepts_plus_inf() {
    // Regression guard: the XSD 1.1 default parse must keep accepting `+INF`.
    assert_eq!(parse_double("+INF").unwrap(), f64::INFINITY);
}

#[test]
fn parse_float_xsd10_rejects_plus_inf() {
    assert!(parse_float_xsd10("+INF").is_err());
}

#[test]
fn parse_double_xsd10_rejects_plus_inf() {
    assert!(parse_double_xsd10("+INF").is_err());
}

#[test]
fn parse_float_xsd10_accepts_xsd10_lexicals() {
    assert_eq!(parse_float_xsd10("INF").unwrap(), f32::INFINITY);
    assert_eq!(parse_float_xsd10("-INF").unwrap(), f32::NEG_INFINITY);
    assert!(parse_float_xsd10("NaN").unwrap().is_nan());
    assert_eq!(parse_float_xsd10("1.5").unwrap(), 1.5f32);
    assert_eq!(parse_float_xsd10("1e10").unwrap(), 1e10f32);
}

#[test]
fn parse_double_xsd10_accepts_xsd10_lexicals() {
    assert_eq!(parse_double_xsd10("INF").unwrap(), f64::INFINITY);
    assert_eq!(parse_double_xsd10("-INF").unwrap(), f64::NEG_INFINITY);
    assert!(parse_double_xsd10("NaN").unwrap().is_nan());
    assert_eq!(parse_double_xsd10("1.5").unwrap(), 1.5f64);
    assert_eq!(parse_double_xsd10("1e10").unwrap(), 1e10f64);
}

#[test]
fn value_parse_xsd10_rejects_plus_inf_for_float_and_double() {
    assert!(crate::value::parse_xsd10("+INF", D::Double).is_err());
    assert!(crate::value::parse_xsd10("+INF", D::Float).is_err());
}

#[test]
fn value_parse_xsd10_unaffected_for_non_float_double_datatypes() {
    // Non-float/double datatypes must behave exactly as `parse`.
    let xsd10 = crate::value::parse_xsd10("1", D::Integer).unwrap();
    let xsd11 = crate::value::parse("1", D::Integer).unwrap();
    assert_eq!(xsd10.canonical_lexical(), xsd11.canonical_lexical());
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
        value: exact::Integer::from_i128(5),
        datatype: D::Int,
    };
    let long5 = XsdValue::Integer {
        value: exact::Integer::from_i128(5),
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
        XsdValue::Decimal(d) => d.clone(),
        other => panic!("expected Decimal, got {other:?}"),
    }
}

/// Helper: extract the i128 from an XsdValue::Integer, panic otherwise.
fn as_integer(v: &XsdValue) -> i128 {
    match v {
        XsdValue::Integer { value, .. } => value.as_i128().expect("a machine-word integer"),
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
    // i128::MAX + 1 is the exact integer past i128, never a wrap or a refusal.
    let max = int_val(i128::MAX);
    let one = int_val(1);
    let sum = numeric_add(&max, &one).expect("exact");
    assert_eq!(sum.as_i128(), None);
    assert_eq!(
        sum.canonical_lexical(),
        "170141183460469231731687303715884105728"
    );
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

// -- `bigint_avg_decimal`: AVG's finish once a running SUM has escaped `i128` --

// -- decimal exactness: 0.1 + 0.2 == 0.3 (the classic float failure) --

#[test]
fn decimal_add_exact_no_float_error() {
    // IEEE double: 0.1 + 0.2 ≠ 0.3; exact decimal: 0.1 + 0.2 == 0.3.
    let result = numeric_add(&dec_val("0.1"), &dec_val("0.2")).unwrap();
    let d = as_decimal(&result);
    let expected = parse_decimal("0.3").unwrap();
    assert_eq!(
        d.cmp(&expected),
        Ordering::Equal,
        "0.1 + 0.2 must equal 0.3 exactly in decimal; got {}",
        d.canonical_lexical()
    );
}

/// Aligning a large scale-0 mantissa to scale 18 leaves `i128` before the
/// addition does: `1e30 + 1e-18` needs the mantissa `1e48`. That is an
/// `OutOfRange` overflow (it used to panic under overflow checks and wrap
/// to a wrong sum without them); the neighbouring `1e19 + 1e-18` (mantissa
/// `1e37`) still fits and must still add exactly — for `-` too.
#[test]
fn decimal_alignment_overflow_is_exact_and_its_neighbour_adds() {
    let tiny = dec_val("0.000000000000000001");
    let huge = dec_val("1000000000000000000000000000000");
    for (result, expected) in [
        (
            numeric_add(&huge, &tiny),
            "1000000000000000000000000000000.000000000000000001",
        ),
        (
            numeric_add(&tiny, &huge),
            "1000000000000000000000000000000.000000000000000001",
        ),
        (
            numeric_sub(&huge, &tiny),
            "999999999999999999999999999999.999999999999999999",
        ),
        (
            numeric_add(&int_val(10i128.pow(30)), &tiny),
            "1000000000000000000000000000000.000000000000000001",
        ),
    ] {
        let result = result.expect("exact");
        assert!(beyond_machine_words(&result), "{result:?}");
        assert_eq!(result.canonical_lexical(), expected);
    }
    let fits = dec_val("10000000000000000000");
    assert_eq!(
        numeric_add(&fits, &tiny).unwrap().canonical_lexical(),
        "10000000000000000000.000000000000000001"
    );
    assert_eq!(
        numeric_sub(&fits, &tiny).unwrap().canonical_lexical(),
        "9999999999999999999.999999999999999999"
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

/// Promotion reads the correctly rounded value: `72922151633738826.80` is
/// `7.292215163373883e16` as a double (the old two-rounding conversion gave
/// `…882e16`, so `=` against the correctly rounded double was false), and
/// `18446745173221179393.0` is `(2^24 + 2) × 2^40` as a float (narrowing the
/// double rounds twice, to `2^64`). The neighbouring double one ulp down
/// still compares unequal, so the `=` is observed, not assumed.
#[test]
fn decimal_promotion_rounds_once_to_double_and_to_float() {
    let witness = dec_val("72922151633738826.80");
    let correct = 7.292_215_163_373_883e16_f64;
    assert_eq!(
        numeric_cmp(&witness, &XsdValue::Double(correct)),
        Some(Ordering::Equal)
    );
    assert_eq!(
        numeric_cmp(&witness, &XsdValue::Double(correct.next_down())),
        Some(Ordering::Greater)
    );
    let sum = numeric_add(&witness, &XsdValue::Double(0.0)).unwrap();
    assert_eq!(as_double(&sum).to_bits(), correct.to_bits());

    let above_tie = dec_val("18446745173221179393.0");
    let correct_f32 = f32::from_bits(((127 + 64) << 23) | 1);
    let sum = numeric_add(&above_tie, &float_val(0.0)).unwrap();
    assert_eq!(as_float(&sum).to_bits(), correct_f32.to_bits());
    assert_eq!(
        numeric_cmp(&above_tie, &float_val(correct_f32)),
        Some(Ordering::Equal)
    );
    assert_eq!(
        numeric_cmp(&above_tie, &float_val(2f32.powi(64))),
        Some(Ordering::Greater)
    );
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
        d.cmp(&expected),
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

/// `xsd:double` and `xsd:float` arithmetic is the IEEE result on every target, the x87
/// included: each operand pair below is a double-rounding witness -- the reference
/// shows the result rounded through the x87's register format differs -- and the
/// numeric operators return the correctly rounded bits.
#[test]
fn binary_arithmetic_rounds_once_on_double_rounding_witnesses() {
    use crate::ieee::reference as soft;

    let one = 1.0_f64;
    let little = f64::from_bits(0x3ca0_0000_0800_0000); // 2^-53 + 2^-78
    // The observing oracle: through the register's 64 bits the sum is 1.
    assert_eq!(soft::add_via(one, little, soft::X87_EXTENDED), 1.0);
    let succ = 1.0 + f64::EPSILON;
    // Through the lexical space, as a query writes them.
    let lexical = canonical_double(little);
    let parsed = parse_double(&lexical).expect("round-trips");
    assert_eq!(parsed.to_bits(), little.to_bits(), "{lexical}");
    let sum = numeric_add(&double_val(one), &double_val(parsed)).unwrap();
    assert_eq!(as_double(&sum).to_bits(), succ.to_bits());
    assert_eq!(sum.canonical_lexical(), "1.0000000000000002E0");
    // Promotion from an integer operand does not change the law.
    let sum = numeric_add(&int_val(1), &double_val(little)).unwrap();
    assert_eq!(as_double(&sum).to_bits(), succ.to_bits());
    // `(1 + ulp) − (−(2^-53 − 2^-78))` is the same witness as a difference.
    let less = f64::from_bits(0x3c9f_ffff_f000_0000);
    let difference = numeric_sub(&double_val(succ), &double_val(-less)).unwrap();
    assert_eq!(as_double(&difference).to_bits(), succ.to_bits());
    assert_eq!(
        soft::add_via(succ, less, soft::X87_EXTENDED),
        f64::from_bits(0x3ff0_0000_0000_0002)
    );

    // Subnormal products and quotients: rounded at 53 bits and again when stored
    // without the scaling.
    let half_min_plus_one = f64::from_bits(0x0008_0000_0000_0001);
    let (a, b) = (
        f64::from_bits(((1023 - 512) << 52) + 4),
        f64::from_bits(((1023 - 511) << 52) - 2),
    );
    assert_ne!(soft::mul_via(a, b, soft::X87_DOUBLE), half_min_plus_one);
    let product = numeric_mul(&double_val(a), &double_val(b)).unwrap();
    assert_eq!(as_double(&product).to_bits(), half_min_plus_one.to_bits());
    let (a, b) = (
        f64::from_bits((1023 - 512) << 52),
        f64::from_bits(((1023 + 511) << 52) - 2),
    );
    assert_ne!(soft::div_via(a, b, soft::X87_DOUBLE), half_min_plus_one);
    let quotient = numeric_div(&double_val(a), &double_val(b)).unwrap();
    assert_eq!(as_double(&quotient).to_bits(), half_min_plus_one.to_bits());

    // xsd:float: the binary32 subnormal witnesses, rounded at 24 bits and again when
    // stored without the scaling.
    let f32_half_min_plus_one = f32::from_bits(0x0040_0001);
    let (a, b) = (
        f32::from_bits(((127 - 64) << 23) + 4),
        f32::from_bits(((127 - 63) << 23) - 2),
    );
    assert_ne!(
        soft::mul32_via(a, b, soft::X87_SINGLE),
        f32_half_min_plus_one
    );
    let product = numeric_mul(&float_val(a), &float_val(b)).unwrap();
    assert_eq!(
        as_float(&product).to_bits(),
        f32_half_min_plus_one.to_bits()
    );
    let (a, b) = (
        f32::from_bits((127 - 64) << 23),
        f32::from_bits(((127 + 63) << 23) - 2),
    );
    assert_ne!(
        soft::div32_via(a, b, soft::X87_SINGLE),
        f32_half_min_plus_one
    );
    let quotient = numeric_div(&float_val(a), &float_val(b)).unwrap();
    assert_eq!(
        as_float(&quotient).to_bits(),
        f32_half_min_plus_one.to_bits()
    );
}

/// Random `xsd:double`/`xsd:float` operands through all four operators, held to the
/// integer reference bit for bit.
#[test]
fn binary_arithmetic_equals_the_software_reference() {
    use crate::ieee::reference as soft;

    let mut state = 0x0b1a_4e57_u64;
    let mut next = || purrdf_testkit::rng::splitmix64_next(&mut state);
    for index in 0..20_000_u32 {
        // Near one (dense ties) or anywhere in the finite range.
        let draw = |bits: u64| {
            if index % 2 == 0 {
                f64::from_bits((bits & 0x800f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000)
            } else {
                f64::from_bits(bits & 0xffef_ffff_ffff_ffff)
            }
        };
        let (x, y) = (draw(next()), draw(next()));
        let d = |v: &XsdValue| as_double(v).to_bits();
        assert_eq!(
            d(&numeric_add(&double_val(x), &double_val(y)).unwrap()),
            soft::add(x, y).to_bits()
        );
        assert_eq!(
            d(&numeric_sub(&double_val(x), &double_val(y)).unwrap()),
            soft::sub(x, y).to_bits()
        );
        assert_eq!(
            d(&numeric_mul(&double_val(x), &double_val(y)).unwrap()),
            soft::mul(x, y).to_bits()
        );
        assert_eq!(
            d(&numeric_div(&double_val(x), &double_val(y)).unwrap()),
            soft::div(x, y).to_bits()
        );
        let (p, q) = (x as f32, y as f32);
        if p.is_finite() && q.is_finite() {
            let f = |v: &XsdValue| as_float(v).to_bits();
            assert_eq!(
                f(&numeric_add(&float_val(p), &float_val(q)).unwrap()),
                soft::add32(p, q).to_bits()
            );
            assert_eq!(
                f(&numeric_sub(&float_val(p), &float_val(q)).unwrap()),
                soft::sub32(p, q).to_bits()
            );
            assert_eq!(
                f(&numeric_mul(&float_val(p), &float_val(q)).unwrap()),
                soft::mul32(p, q).to_bits()
            );
            if q != 0.0 {
                assert_eq!(
                    f(&numeric_div(&float_val(p), &float_val(q)).unwrap()),
                    soft::div32(p, q).to_bits()
                );
            }
        }
    }
}

/// The decimal-to-double conversion's exact fast path divides once: a decimal whose
/// quotient `mantissa / 10^scale` rounds differently through the x87's 64-bit register
/// converts to the correctly rounded double, and so compares and promotes by it.
#[test]
fn decimal_to_double_fast_path_rounds_once_on_a_double_rounding_witness() {
    use crate::ieee::reference as soft;

    // The first scale, and the first mantissa from 2^52 upward at it, whose quotient
    // is a witness. (No quotient by 10 is one: the binary expansion of a tenth
    // repeats `0011`, which never rounds onto a midpoint at 64 bits.)
    let (scale, witness) = (1_u8..=22)
        .find_map(|scale| {
            let power = 10_f64.powi(i32::from(scale));
            (1_i128 << 52..(1 << 52) + 5_000)
                .find(|&m| {
                    let a = m as f64;
                    soft::div_via(a, power, soft::X87_EXTENDED).to_bits()
                        != soft::div(a, power).to_bits()
                })
                .map(|m| (scale, m))
        })
        .expect("a double-rounding witness at some scale");
    let expected = soft::div(witness as f64, 10_f64.powi(i32::from(scale)));
    let decimal = fixed(witness, u32::from(scale));
    assert_eq!(
        decimal.to_f64().to_bits(),
        expected.to_bits(),
        "{witness} / 10^{scale}"
    );
    // The same bits through promotion into double arithmetic.
    let sum = numeric_add(&XsdValue::Decimal(decimal), &double_val(0.0)).unwrap();
    assert_eq!(as_double(&sum).to_bits(), expected.to_bits());
}

/// Hold `numeric_div` on two decimals to the default policy, from the exact rational
/// `dividend / divisor` with integer arithmetic only ([`purrdf_testkit::exact`]): the
/// exact quotient whenever its expansion terminates, and otherwise eighteen
/// fractional digits rounded half to even.
fn assert_decimal_quotient(dividend: &Decimal, divisor: &Decimal) {
    use purrdf_testkit::exact::{Direction, Rational as Oracle};
    let got = numeric_div(
        &XsdValue::Decimal(dividend.clone()),
        &XsdValue::Decimal(divisor.clone()),
    )
    .unwrap_or_else(|error| panic!("{dividend:?} / {divisor:?}: {error}"));
    let oracle = |d: &Decimal| Oracle::parse(&d.canonical_lexical()).expect("a decimal numeral");
    let exact = oracle(dividend)
        .div(&oracle(divisor))
        .expect("a nonzero divisor");
    let value = Oracle::parse(&got.canonical_lexical()).expect("a numeral");
    match exact.to_canonical_decimal() {
        Some(terminating) => assert_eq!(
            got.canonical_lexical(),
            terminating,
            "{dividend:?} / {divisor:?} terminates and is exact"
        ),
        None => assert!(
            value.value_eq(&exact.round_to_scale(18, Direction::HalfEven)),
            "{dividend:?} / {divisor:?} = {got:?}"
        ),
    }
}

#[test]
fn decimal_division_matches_the_exact_oracle() {
    let mut state = 0xD1F1_DE5A_u64;
    let mut next = || purrdf_testkit::rng::splitmix64_next(&mut state);
    let int = |next: &mut dyn FnMut() -> u64| {
        let width = 1 + (next() % 127) as u32;
        let raw = (u128::from(next()) << 64) | u128::from(next());
        let magnitude = (raw >> (128 - width)) as i128;
        if next() & 1 == 1 {
            -magnitude
        } else {
            magnitude
        }
    };
    let scale = |next: &mut dyn FnMut() -> u64| (next() % 19) as u32;
    let mut cases = 0_usize;
    let extremes = [
        i128::MIN,
        i128::MIN + 1,
        i128::MAX,
        i128::MAX - 1,
        -1,
        1,
        2,
        3,
        7,
        10,
        -10,
        1_000_000_000_000_000_000_000,
        100_000_000_000_000_000_000,
    ];
    // Every extreme over every extreme, at a spread of scales — the overflow
    // boundary (MAX / 0.1, MIN / -1) and the exact large quotients (10^21 / 2,
    // MIN / 2) among them.
    for &dm in &extremes {
        for &vm in &extremes {
            for (ds, vs) in [(0, 0), (0, 1), (1, 0), (0, 18), (18, 0), (18, 18), (5, 9)] {
                assert_decimal_quotient(&fixed(dm, ds), &fixed(vm, vs));
                cases += 1;
            }
        }
    }
    for _ in 0..30_000 {
        let (dm, vm) = (int(&mut next), int(&mut next));
        if vm == 0 {
            continue;
        }
        let (ds, vs) = (scale(&mut next), scale(&mut next));
        assert_decimal_quotient(&fixed(dm, ds), &fixed(vm, vs));
        cases += 1;
    }
    // Exact quotients: dividend = quotient × divisor, whenever that product fits.
    for _ in 0..20_000 {
        let (q, vm) = (int(&mut next) >> 40, int(&mut next) >> 40);
        let Some(dm) = q.checked_mul(vm).filter(|_| vm != 0) else {
            continue;
        };
        let (ds, vs) = (scale(&mut next), scale(&mut next));
        assert_decimal_quotient(&fixed(dm, ds), &fixed(vm, vs));
        cases += 1;
    }
    // Near the overflow boundary: large dividends over small divisors.
    for _ in 0..10_000 {
        let dm = (i128::MAX - (int(&mut next) >> 64).abs()) * if next() & 1 == 1 { -1 } else { 1 };
        let vm = int(&mut next) >> 100;
        if vm == 0 {
            continue;
        }
        let (ds, vs) = (scale(&mut next), scale(&mut next));
        assert_decimal_quotient(&fixed(dm, ds), &fixed(vm, vs));
        cases += 1;
    }
    assert!(cases > 50_000, "{cases}");
}

#[test]
fn decimal_negation_and_abs_of_the_smallest_mantissa_are_exact() {
    let min = XsdValue::Decimal(fixed(i128::MIN, 0));
    let two_pow_127 = "170141183460469231731687303715884105728";
    assert_eq!(
        numeric_unary_minus(&min)
            .expect("exact")
            .canonical_lexical(),
        two_pow_127
    );
    assert_eq!(
        numeric_abs(&min).expect("exact").canonical_lexical(),
        two_pow_127
    );
    let scaled = XsdValue::Decimal(fixed(i128::MIN, 18));
    assert_eq!(
        numeric_abs(&scaled).expect("exact").canonical_lexical(),
        "170141183460469231731.687303715884105728"
    );
    // One above it negates and takes its absolute value.
    let next = XsdValue::Decimal(fixed(i128::MIN + 1, 0));
    let Ok(XsdValue::Decimal(abs)) = numeric_abs(&next) else {
        panic!("abs of i128::MIN + 1");
    };
    assert_eq!(abs.unscaled().as_i128(), Some(i128::MAX));
    let Ok(XsdValue::Decimal(negated)) = numeric_unary_minus(&next) else {
        panic!("negation of i128::MIN + 1");
    };
    assert_eq!(negated.unscaled().as_i128(), Some(i128::MAX));
}
