// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every numeric conversion the XSD constructor casts perform, held to an exact rational
//! oracle ([`purrdf_testkit::exact`]): the source's exact value (an integer, a decimal
//! mantissa and scale, or a float's binary value) rounded with integer arithmetic only,
//! never through a float.
//!
//! Each property draws seeded random sources across the whole range and adds the hard
//! cases: exact ties, the values one unit either side of them, the subnormal band, the
//! largest finite values and the `i128` extremes.

use purrdf_testkit::exact::Rational;
use purrdf_testkit::rng::splitmix64_next;
use purrdf_xsd::{Decimal, XsdDatatype, XsdValue};

use super::{cast_numeric_value, xpath_double_to_string, xpath_float_to_string};

/// Random draws per property, besides the hard cases.
const DRAWS: usize = 40_000;

struct Draws(u64);

impl Draws {
    fn next(&mut self) -> u64 {
        splitmix64_next(&mut self.0)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    /// A random integer of exactly `precision` bits (`precision ≤ 64`).
    fn significand(&mut self, precision: u32) -> i128 {
        let low = i128::from(self.next()) & ((1 << (precision - 1)) - 1);
        (1 << (precision - 1)) | low
    }

    /// A random `i128` of a random bit width, either sign.
    fn int(&mut self) -> i128 {
        let width = 1 + self.below(127) as u32;
        let raw = (u128::from(self.next()) << 64) | u128::from(self.next());
        let magnitude = (raw >> (128 - width)) as i128;
        if self.next() & 1 == 1 {
            -magnitude
        } else {
            magnitude
        }
    }

    /// A random finite `f64`, any exponent.
    fn double(&mut self) -> f64 {
        loop {
            let value = f64::from_bits(self.next());
            if value.is_finite() {
                return value;
            }
        }
    }

    /// A random finite `f32`, any exponent.
    fn float(&mut self) -> f32 {
        loop {
            let value = f32::from_bits((self.next() >> 32) as u32);
            if value.is_finite() {
                return value;
            }
        }
    }
}

fn integer(value: i128) -> XsdValue {
    XsdValue::integer(value)
}

/// The decimal `mantissa × 10^-scale` (`scale ≤ 18`), through the public parser.
fn decimal(mantissa: i128, scale: u32) -> Decimal {
    let digits = mantissa.unsigned_abs().to_string();
    let scale = scale as usize;
    let padded = format!("{digits:0>width$}", width = scale + 1);
    let (whole, fraction) = padded.split_at(padded.len() - scale);
    let sign = if mantissa < 0 { "-" } else { "" };
    let lexical = if scale == 0 {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{fraction}")
    };
    match purrdf_xsd::parse(&lexical, XsdDatatype::Decimal) {
        Ok(XsdValue::Decimal(d)) => d,
        other => panic!("{lexical} is a decimal: {other:?}"),
    }
}

fn cast_float(source: &XsdValue) -> f32 {
    match cast_numeric_value(source, XsdDatatype::Float) {
        Some(XsdValue::Float(value)) => value,
        other => panic!("float cast of {source:?}: {other:?}"),
    }
}

fn cast_double(source: &XsdValue) -> f64 {
    match cast_numeric_value(source, XsdDatatype::Double) {
        Some(XsdValue::Double(value)) => value,
        other => panic!("double cast of {source:?}: {other:?}"),
    }
}

/// Hold one exact source's float and double casts to the oracle.
fn assert_binary_casts(source: &XsdValue, exact: &Rational) -> usize {
    assert_eq!(
        cast_float(source).to_bits(),
        exact.to_f32().to_bits(),
        "float cast of {source:?}"
    );
    assert_eq!(
        cast_double(source).to_bits(),
        exact.to_f64().to_bits(),
        "double cast of {source:?}"
    );
    1
}

/// `k × 2^j + 2^(j-1)` for a `precision`-bit `k`: exactly halfway between two
/// neighbours at that precision, with the integers one either side of it.
fn integer_ties(draws: &mut Draws, precision: u32) -> Vec<i128> {
    let mut out = Vec::new();
    for _ in 0..2_000 {
        let k = draws.significand(precision);
        let shift = 1 + draws.below(u64::from(126 - precision)) as u32;
        let tie = (k << shift) + (1 << (shift - 1));
        for value in [tie - 1, tie, tie + 1] {
            out.push(value);
            out.push(-value);
        }
    }
    out
}

#[test]
fn integer_casts_to_float_and_double_round_once_to_nearest_even() {
    let mut draws = Draws(0x1A7E_6E25);
    let mut sources: Vec<i128> = vec![
        0,
        1,
        -1,
        i128::MIN,
        i128::MIN + 1,
        i128::MAX,
        i128::MAX - 1,
        (1 << 24) + 1,
        (1 << 53) + 1,
        -(1 << 53) - 1,
        // 2^127 - 2^102, the float tie below 2^127, and its neighbours.
        i128::MAX - (1 << 102) + 1,
        i128::MAX - (1 << 102),
        i128::MAX - (1 << 102) + 2,
    ];
    sources.extend(integer_ties(&mut draws, 24));
    sources.extend(integer_ties(&mut draws, 53));
    sources.extend((0..DRAWS).map(|_| draws.int()));
    let checked: usize = sources
        .iter()
        .map(|&value| assert_binary_casts(&integer(value), &Rational::from_i128(value)))
        .sum();
    assert!(checked > DRAWS + 20_000, "{checked}");
}

/// Ties at `precision` written as decimals: an odd `2k + 1` (`k` of `precision` bits)
/// times `2^-t` is `(2k + 1) × 5^t` at scale `t`, exactly representable for `t ≤ 18`;
/// with the decimals one unit of the last place either side.
fn decimal_ties(draws: &mut Draws, precision: u32) -> Vec<(i128, u32)> {
    let mut out = Vec::new();
    for _ in 0..2_000 {
        let k = draws.significand(precision);
        let odd = 2 * k + 1;
        let scale = draws.below(19) as u32;
        let mantissa = odd * 5_i128.pow(scale);
        for value in [mantissa - 1, mantissa, mantissa + 1] {
            out.push((value, scale));
            out.push((-value, scale));
        }
        // The same tie at scale 0, an integer: (2k + 1) × 2^s.
        let shift = draws.below(u64::from(126 - precision - 1)) as u32;
        out.push((odd << shift, 0));
    }
    out
}

#[test]
fn decimal_casts_to_float_and_double_round_once_to_nearest_even() {
    let mut draws = Draws(0xDEC1_F10A);
    let mut sources: Vec<(i128, u32)> = Vec::new();
    for scale in 0..=18 {
        for mantissa in [
            i128::MAX,
            i128::MIN + 1,
            1,
            -1,
            (1 << 24) + 1,
            (1 << 53) + 1,
        ] {
            sources.push((mantissa, scale));
        }
    }
    sources.extend(decimal_ties(&mut draws, 24));
    sources.extend(decimal_ties(&mut draws, 53));
    sources.extend((0..DRAWS).map(|_| {
        let mantissa = draws.int();
        (
            if mantissa == i128::MIN { 0 } else { mantissa },
            draws.below(19) as u32,
        )
    }));
    let checked: usize = sources
        .iter()
        .map(|&(mantissa, scale)| {
            let value = XsdValue::Decimal(decimal(mantissa, scale));
            // The public conversions directly, and through the cast.
            let XsdValue::Decimal(d) = &value else {
                unreachable!()
            };
            let exact = Rational::from_decimal(mantissa, scale);
            assert_eq!(d.to_f32().to_bits(), exact.to_f32().to_bits(), "{d:?}");
            assert_eq!(d.to_f64().to_bits(), exact.to_f64().to_bits(), "{d:?}");
            assert_binary_casts(&value, &exact)
        })
        .sum();
    assert!(checked > DRAWS + 20_000, "{checked}");
}

/// Doubles that are exactly a float tie (the midpoint of two adjacent floats, normal
/// or subnormal, both signs) and the doubles one ulp either side.
fn float_ties(draws: &mut Draws) -> Vec<f64> {
    let mut out = Vec::new();
    for _ in 0..4_000 {
        let bits = (draws.next() >> 33) as u32 & 0x7F7F_FFFF;
        let low = f32::from_bits(bits);
        let high = f32::from_bits(bits + 1);
        let tie = f64::from(low).midpoint(f64::from(high));
        for value in [tie.next_down(), tie, tie.next_up()] {
            out.push(value);
            out.push(-value);
        }
    }
    out
}

#[test]
fn double_casts_to_float_round_to_nearest_even() {
    let mut draws = Draws(0xD0B1_E2F1);
    let max_tie = f64::from(f32::MAX).midpoint(2_f64.powi(128));
    let min_tie = 2_f64.powi(-150);
    let mut sources = vec![
        0.0,
        -0.0,
        f64::MAX,
        f64::MIN,
        f64::from(f32::MAX),
        max_tie,
        max_tie.next_down(),
        max_tie.next_up(),
        -max_tie,
        min_tie,
        min_tie.next_up(),
        min_tie.next_down(),
        f64::from_bits(1),
        1e-40,
        f64::from(f32::MIN_POSITIVE),
        f64::from(f32::MIN_POSITIVE).next_down(),
    ];
    sources.extend(float_ties(&mut draws));
    // The float subnormal band and its edges, 2^-160 to 2^-120.
    sources.extend((0..DRAWS / 4).map(|_| {
        let exponent = 1023 - 160 + draws.below(41);
        f64::from_bits((exponent << 52) | (draws.next() & ((1 << 52) - 1)))
    }));
    sources.extend((0..DRAWS).map(|_| draws.double()));
    let checked: usize = sources
        .iter()
        .map(|&value| {
            let cast = cast_float(&XsdValue::Double(value));
            assert_eq!(
                cast.to_bits(),
                Rational::from_f64(value).to_f32().to_bits(),
                "{value:e}"
            );
            1
        })
        .sum();
    assert!(checked > DRAWS + 30_000, "{checked}");
}

/// Hold one binary value's decimal cast to the oracle: the decimal value space is
/// unbounded, so the closest decimal F&O 3.1 §19.1.2.3 asks for is the binary value
/// itself, exactly, at every magnitude — in the bounded variant wherever that holds it.
fn assert_decimal_cast(source: &XsdValue, value: f64) -> usize {
    let cast = cast_numeric_value(source, XsdDatatype::Decimal)
        .unwrap_or_else(|| panic!("{value:e} casts to a decimal"));
    let expected = Rational::from_f64(value).canonical_terminating();
    assert_eq!(cast.canonical_lexical(), expected, "{value:e}");
    // The bounded decimal: at most eighteen fractional digits on an `i128` mantissa.
    let bounded = expected
        .split_once('.')
        .is_none_or(|(_, fraction)| fraction.len() <= 18)
        && expected.replace('.', "").parse::<i128>().is_ok();
    assert_eq!(
        matches!(cast, XsdValue::Decimal(_)),
        bounded,
        "{value:e}: {cast:?} is in the bounded variant exactly when that holds it"
    );
    1
}

#[test]
fn float_and_double_casts_to_decimal_are_their_exact_binary_values() {
    let mut draws = Draws(0xF10A_DEC1);
    let mut sources = vec![
        0.0,
        -0.0,
        0.1,
        f64::MAX,
        f64::from_bits(1),
        2_f64.powi(127),
        2_f64.powi(127).next_down(),
        -(2_f64.powi(127)),
        -(2_f64.powi(127).next_down()),
        2_f64.powi(53),
        2_f64.powi(53).next_down(),
        5e-19,
        5e-19_f64.next_up(),
        5e-19_f64.next_down(),
    ];
    // Ties at the 19th digit: an odd m × 2^-19 has exactly 19 fractional digits,
    // the last a 5; and one ulp either side.
    for _ in 0..4_000 {
        let odd = (draws.next() >> 11) | 1;
        let tie = odd as f64 * 2_f64.powi(-19);
        for value in [tie.next_down(), tie, tie.next_up()] {
            sources.push(value);
            sources.push(-value);
        }
    }
    // Magnitudes near the 18-digit quantum, where the decimal is a few units.
    sources.extend((0..DRAWS / 4).map(|_| {
        let exponent = 1023 - 70 + draws.below(20);
        f64::from_bits((exponent << 52) | (draws.next() & ((1 << 52) - 1)))
    }));
    sources.extend((0..DRAWS).map(|_| draws.double()));
    let mut checked: usize = sources
        .iter()
        .map(|&value| assert_decimal_cast(&XsdValue::Double(value), value))
        .sum();
    checked += (0..DRAWS / 2)
        .map(|_| {
            let value = draws.float();
            assert_decimal_cast(&XsdValue::Float(value), f64::from(value))
        })
        .sum::<usize>();
    for special in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(cast_numeric_value(&XsdValue::Double(special), XsdDatatype::Decimal).is_none());
    }
    assert!(checked > DRAWS + 30_000, "{checked}");
}

/// The rendering reads back, exactly and through the XSD parser, as the same bits; and
/// it has the cast-to-string shape: plain notation inside `[1e-6, 1e6)`, otherwise a
/// mantissa with one non-zero digit, a point and at least one more digit.
fn assert_round_trip(rendered: &str, bits_back: impl Fn(&Rational) -> u64, bits: u64, plain: bool) {
    let exact = Rational::parse(rendered).unwrap_or_else(|| panic!("{rendered} is a numeral"));
    assert_eq!(
        bits_back(&exact),
        bits,
        "{rendered} reads back as other bits"
    );
    if plain {
        assert!(!rendered.contains('E'), "{rendered} is in plain notation");
    } else {
        let (mantissa, _) = rendered
            .split_once('E')
            .unwrap_or_else(|| panic!("{rendered} is in scientific notation"));
        let digits = mantissa.trim_start_matches('-');
        let (lead, tail) = digits
            .split_once('.')
            .unwrap_or_else(|| panic!("{rendered} has a point"));
        assert!(
            lead.len() == 1 && lead != "0" && !tail.is_empty(),
            "{rendered} has one non-zero leading digit and a fraction"
        );
    }
}

fn in_plain_range(magnitude: f64) -> bool {
    // Exactly [10^-6, 10^6): compare the exact value with the exact bound.
    let millionth = Rational::parse("0.000001").expect("a numeral");
    Rational::from_f64(magnitude).cmp_magnitude(&millionth) != std::cmp::Ordering::Less
        && magnitude < 1e6
}

#[test]
fn numeric_string_renderings_read_back_as_the_same_bits() {
    let mut draws = Draws(0x5791_6E57);
    let mut doubles = vec![
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        1e-6,
        1e-6_f64.next_up(),
        1e-6_f64.next_down(),
        1e6,
        1e6_f64.next_down(),
        0.1,
        123_456.5,
    ];
    doubles.extend((0..DRAWS / 4).map(|_| {
        // Around the plain-notation range.
        let exponent = 1023 - 24 + draws.below(48);
        f64::from_bits((exponent << 52) | (draws.next() & ((1 << 52) - 1)))
    }));
    doubles.extend((0..DRAWS).map(|_| draws.double()));
    let mut checked = 0_usize;
    for value in doubles.iter().flat_map(|&v| [v, -v]) {
        if value == 0.0 {
            continue;
        }
        let bits = value.to_bits();
        let back = |exact: &Rational| exact.to_f64().to_bits();
        assert_round_trip(
            &xpath_double_to_string(value),
            back,
            bits,
            in_plain_range(value.abs()),
        );
        assert_round_trip(
            &purrdf_xsd::numeric::canonical_double(value),
            back,
            bits,
            false,
        );
        match purrdf_xsd::parse(&xpath_double_to_string(value), XsdDatatype::Double) {
            Ok(XsdValue::Double(parsed)) => assert_eq!(parsed.to_bits(), bits),
            other => panic!("{value:e}: {other:?}"),
        }
        checked += 1;
    }
    let mut floats = vec![
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        1e-6,
        1e-6_f32.next_up(),
        1e-6_f32.next_down(),
        1e6,
        1e6_f32.next_down(),
        0.1,
    ];
    floats.extend((0..DRAWS).map(|_| draws.float()));
    for value in floats.iter().flat_map(|&v| [v, -v]) {
        if value == 0.0 {
            continue;
        }
        let bits = u64::from(value.to_bits());
        let back = |exact: &Rational| u64::from(exact.to_f32().to_bits());
        assert_round_trip(
            &xpath_float_to_string(value),
            back,
            bits,
            in_plain_range(f64::from(value.abs())),
        );
        assert_round_trip(
            &purrdf_xsd::numeric::canonical_float(value),
            back,
            bits,
            false,
        );
        match purrdf_xsd::parse(&xpath_float_to_string(value), XsdDatatype::Float) {
            Ok(XsdValue::Float(parsed)) => assert_eq!(u64::from(parsed.to_bits()), bits),
            other => panic!("{value:e}: {other:?}"),
        }
        checked += 1;
    }
    for (value, expected) in [
        (0.0, "0"),
        (-0.0, "-0"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "INF"),
        (f64::NEG_INFINITY, "-INF"),
    ] {
        assert_eq!(xpath_double_to_string(value), expected);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the special values narrow exactly"
        )]
        let narrow = value as f32;
        assert_eq!(xpath_float_to_string(narrow), expected);
    }
    assert!(checked > 2 * DRAWS, "{checked}");
}

/// Every integer-family target with its inclusive range as XSD 1.1 Part 2 §3.4 states
/// it — written out here rather than read from the implementation under test. `None`
/// is no bound: `xsd:integer` and the sign-restricted facets are unbounded.
const INTEGER_TARGETS: [(XsdDatatype, Option<i128>, Option<i128>); 13] = [
    (XsdDatatype::Integer, None, None),
    (XsdDatatype::Long, Some(-(1 << 63)), Some((1 << 63) - 1)),
    (XsdDatatype::Int, Some(-(1 << 31)), Some((1 << 31) - 1)),
    (XsdDatatype::Short, Some(-(1 << 15)), Some((1 << 15) - 1)),
    (XsdDatatype::Byte, Some(-(1 << 7)), Some((1 << 7) - 1)),
    (XsdDatatype::UnsignedLong, Some(0), Some((1 << 64) - 1)),
    (XsdDatatype::UnsignedInt, Some(0), Some((1 << 32) - 1)),
    (XsdDatatype::UnsignedShort, Some(0), Some((1 << 16) - 1)),
    (XsdDatatype::UnsignedByte, Some(0), Some((1 << 8) - 1)),
    (XsdDatatype::NonNegativeInteger, Some(0), None),
    (XsdDatatype::PositiveInteger, Some(1), None),
    (XsdDatatype::NonPositiveInteger, None, Some(0)),
    (XsdDatatype::NegativeInteger, None, Some(-1)),
];

/// The bounds of [`INTEGER_TARGETS`] that exist, for drawing values around them.
fn integer_bounds() -> impl Iterator<Item = i128> {
    INTEGER_TARGETS
        .into_iter()
        .flat_map(|(_, min, max)| [min, max])
        .flatten()
        .chain([i128::MIN, i128::MAX])
}

/// Hold one source's cast to every integer-family target to the oracle: the exact
/// value truncated toward zero, at any magnitude (`None` for `NaN` and the
/// infinities, which have no integer value), an error outside the target's range,
/// and otherwise that integer typed as the target.
fn assert_integer_casts(source: &XsdValue, exact: Option<&Rational>) -> usize {
    use purrdf_testkit::exact::Direction;
    let truncated = exact.map(|value| value.round_to_scale(0, Direction::TowardZero));
    for (target, min, max) in INTEGER_TARGETS {
        let inside = |value: &Rational| {
            min.is_none_or(|min| value.cmp_value(&Rational::from_i128(min)).is_ge())
                && max.is_none_or(|max| value.cmp_value(&Rational::from_i128(max)).is_le())
        };
        let expected = truncated
            .as_ref()
            .filter(|value| inside(value))
            .map(Rational::canonical_terminating);
        let cast = cast_numeric_value(source, target).map(|cast| {
            assert_eq!(cast.datatype(), target, "{source:?} cast to {target:?}");
            assert!(
                matches!(cast, XsdValue::Integer { .. }),
                "{source:?} cast to {target:?} is {cast:?}"
            );
            assert_eq!(
                cast.as_i128().is_none(),
                cast.canonical_lexical().parse::<i128>().is_err(),
                "{cast:?} is held inline exactly when i128 holds it"
            );
            cast.canonical_lexical()
        });
        assert_eq!(cast, expected, "{source:?} cast to {target:?}");
    }
    INTEGER_TARGETS.len()
}

#[test]
fn float_double_and_decimal_casts_to_integer_types_truncate_toward_zero() {
    let mut draws = Draws(0x7E0C_A7E5);
    let limit = 2_f64.powi(127);
    let mut doubles = vec![
        0.0,
        -0.0,
        limit,
        limit.next_down(),
        limit.next_up(),
        -limit,
        (-limit).next_up(),
        (-limit).next_down(),
        f64::MAX,
        f64::MIN,
        f64::from_bits(1),
    ];
    // Just inside and just outside ±1.
    for one in [1.0_f64, -1.0] {
        doubles.extend([one, one.next_up(), one.next_down(), one * 0.5, one * 1.5]);
    }
    // One unit and a fraction either side of each target's bounds, where a double
    // can hold them; the 2^63 and 2^64 bounds by their neighbouring doubles.
    let mut decimals: Vec<(i128, u32)> = Vec::new();
    for bound in integer_bounds() {
        {
            for delta in [-1_i128, 0, 1] {
                let Some(value) = bound.checked_add(delta) else {
                    continue;
                };
                #[allow(clippy::cast_precision_loss, reason = "the nearest double is the case")]
                let double = value as f64;
                doubles.extend([double, double.next_up(), double.next_down()]);
                decimals.push((value, 0));
                // ±0.5 around the bound, at scale 1, while the mantissa fits.
                if let Some(tenfold) = value.checked_mul(10) {
                    for half in [-5, 5] {
                        if let Some(mantissa) = tenfold.checked_add(half) {
                            decimals.push((mantissa, 1));
                        }
                    }
                }
            }
        }
    }
    // Magnitudes from 2^-4 to 2^130, where truncation and every range bound live.
    doubles.extend((0..DRAWS / 2).map(|_| {
        let exponent = 1023 - 4 + draws.below(135);
        let sign = draws.next() & (1 << 63);
        f64::from_bits(sign | (exponent << 52) | (draws.next() & ((1 << 52) - 1)))
    }));
    doubles.extend((0..DRAWS / 4).map(|_| draws.double()));
    // Decimals at every scale: the whole part is extracted exactly.
    decimals.extend((0..DRAWS / 2).map(|_| (draws.int(), draws.below(19) as u32)));
    decimals.extend([
        (i128::MAX, 0),
        (i128::MIN + 1, 0),
        (i128::MAX, 18),
        (-1, 18),
    ]);

    let mut checked = 0_usize;
    for &value in &doubles {
        checked += assert_integer_casts(&XsdValue::Double(value), Some(&Rational::from_f64(value)));
    }
    let floats: Vec<f32> = (0..DRAWS / 4)
        .map(|_| draws.float())
        .chain([
            0.0,
            -0.0,
            0.999_999_94,
            -0.999_999_94,
            1.0,
            -1.0,
            f32::MAX,
            f32::MIN,
        ])
        .collect();
    for &value in &floats {
        checked += assert_integer_casts(&XsdValue::Float(value), Some(&Rational::from_f32(value)));
    }
    for &(mantissa, scale) in &decimals {
        let source = XsdValue::Decimal(decimal(mantissa, scale));
        checked += assert_integer_casts(&source, Some(&Rational::from_decimal(mantissa, scale)));
    }
    // No integer value: an error for every target.
    for special in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        checked += assert_integer_casts(&XsdValue::Double(special), None);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the special values narrow exactly"
        )]
        let narrow = special as f32;
        checked += assert_integer_casts(&XsdValue::Float(narrow), None);
    }
    assert!(checked > 13 * DRAWS, "{checked}");
}
