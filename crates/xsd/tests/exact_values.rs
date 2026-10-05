// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `xsd:integer` and `xsd:decimal` values of any size through the public value
//! layer: parsing, the numeric operators, comparison, the exact order, the
//! division policy, cost estimates and data ranges. Each refusal runs beside the
//! neighbouring value that is accepted.

use std::cmp::Ordering;

use purrdf_xsd::exact::{DivisionPolicy, ExactError, Rounding};
use purrdf_xsd::numeric::{CostOp, numeric_cost, numeric_div_with_policy};
use purrdf_xsd::range::{
    DataRange, Facet, Known, Satisfiability, contains, same_value, satisfiability,
};
use purrdf_xsd::{
    XsdDatatype as D, XsdError, XsdValue, numeric_abs, numeric_add, numeric_ceil, numeric_cmp,
    numeric_div, numeric_floor, numeric_mul, numeric_round, numeric_sub, numeric_total_cmp,
    numeric_unary_minus, parse, value_eq,
};

const I128_MAX_PLUS_ONE: &str = "170141183460469231731687303715884105728";

fn int(text: &str) -> XsdValue {
    parse(text, D::Integer).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn dec(text: &str) -> XsdValue {
    parse(text, D::Decimal).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn dbl(text: &str) -> XsdValue {
    parse(text, D::Double).unwrap_or_else(|error| panic!("{text}: {error}"))
}

#[test]
fn integers_past_i128_parse_exactly_and_in_range_ones_stay_bounded() {
    let max = int(&i128::MAX.to_string());
    assert!(matches!(max, XsdValue::Integer { .. }));
    let past = int(I128_MAX_PLUS_ONE);
    assert!(matches!(past, XsdValue::BigInteger { .. }));
    assert_eq!(past.canonical_lexical(), I128_MAX_PLUS_ONE);
    assert_eq!(past.datatype(), D::Integer);
    assert!(past.is_numeric());
    let below = int("-170141183460469231731687303715884105729");
    assert!(matches!(below, XsdValue::BigInteger { .. }));
    let huge = format!("-{}", "9".repeat(300));
    assert_eq!(int(&huge).canonical_lexical(), huge);
    assert_eq!(int("+000123").canonical_lexical(), "123");
}

#[test]
fn derived_integer_bounds_still_refuse_and_unbounded_derived_types_do_not() {
    // Bounded derived types: one past the bound refuses, the bound parses.
    assert!(matches!(
        parse("128", D::Byte),
        Err(XsdError::OutOfRange { .. })
    ));
    assert!(parse("127", D::Byte).is_ok());
    assert!(parse("18446744073709551616", D::UnsignedLong).is_err());
    assert!(parse("18446744073709551615", D::UnsignedLong).is_ok());
    // Unbounded derived types take a value of any size of the right sign.
    let big = "1".repeat(60);
    assert!(matches!(
        parse(&big, D::NonNegativeInteger),
        Ok(XsdValue::BigInteger { .. })
    ));
    assert!(parse(&big, D::PositiveInteger).is_ok());
    assert!(parse(&format!("-{big}"), D::NonNegativeInteger).is_err());
    assert!(parse(&format!("-{big}"), D::NegativeInteger).is_ok());
    assert!(parse(&big, D::NegativeInteger).is_err());
    assert!(parse(&big, D::Long).is_err());
    // Malformed text is still a lexical error at any length.
    assert!(matches!(
        parse(&format!("{big}x"), D::Integer),
        Err(XsdError::InvalidLexical { .. })
    ));
}

#[test]
fn decimals_past_the_bounded_form_parse_exactly() {
    let long = dec("0.1000000000000000000000000001");
    assert!(matches!(long, XsdValue::BigDecimal(_)));
    assert_eq!(long.canonical_lexical(), "0.1000000000000000000000000001");
    // Trailing zeros past eighteen digits are not significant.
    let padded = dec("2.50000000000000000000000000");
    assert!(matches!(padded, XsdValue::Decimal(_)));
    assert_eq!(padded.canonical_lexical(), "2.5");
    let wide = dec(&format!("{}.5", "7".repeat(50)));
    assert!(matches!(wide, XsdValue::BigDecimal(_)));
    assert!(parse("1.2.3", D::Decimal).is_err());
    assert!(parse(".5", D::Decimal).is_ok());
}

#[test]
fn arithmetic_that_overflowed_now_returns_the_exact_value() {
    let max = int(&i128::MAX.to_string());
    let one = int("1");
    let sum = numeric_add(&max, &one).expect("exact");
    assert_eq!(sum.canonical_lexical(), I128_MAX_PLUS_ONE);
    assert!(matches!(sum, XsdValue::BigInteger { .. }));
    // Coming back into range returns the bounded variant.
    let back = numeric_sub(&sum, &one).expect("exact");
    assert!(matches!(back, XsdValue::Integer { value, .. } if value == i128::MAX));
    let square = numeric_mul(&max, &max).expect("exact");
    assert_eq!(
        square.canonical_lexical(),
        "28948022309329048855892746252171976962977213799489202546401021394546514198529"
    );
    let min = int(&i128::MIN.to_string());
    assert_eq!(
        numeric_unary_minus(&min)
            .expect("exact")
            .canonical_lexical(),
        I128_MAX_PLUS_ONE
    );
    assert_eq!(
        numeric_abs(&min).expect("exact").canonical_lexical(),
        I128_MAX_PLUS_ONE
    );

    let long = dec("0.1000000000000000000000000001");
    let zero = int("0");
    assert_eq!(
        numeric_add(&long, &zero)
            .expect("exact")
            .canonical_lexical(),
        "0.1000000000000000000000000001"
    );
    // A product past eighteen fractional digits is exact (it used to truncate).
    let tiny = dec("0.0000000001");
    assert_eq!(
        numeric_mul(&tiny, &tiny)
            .expect("exact")
            .canonical_lexical(),
        "0.00000000000000000001"
    );
    // In-range products stay on the bounded path and answer as before.
    assert_eq!(
        numeric_mul(&dec("1.5"), &dec("2.25"))
            .expect("bounded")
            .canonical_lexical(),
        "3.375"
    );
}

#[test]
fn division_follows_the_policy_and_refuses_only_what_it_must() {
    let one = int("1");
    let three = int("3");
    let eight = int("8");
    // The default: eighteen digits, truncated — the bounded quotient.
    assert_eq!(
        numeric_div(&one, &three)
            .expect("default")
            .canonical_lexical(),
        "0.333333333333333333"
    );
    // A quotient whose eighteen digits overflow the bounded coefficient keeps them.
    let huge = int(&"9".repeat(30));
    assert_eq!(
        numeric_div(&huge, &three)
            .expect("default")
            .canonical_lexical(),
        "333333333333333333333333333333"
    );
    let big_by_seven = numeric_div(&int(I128_MAX_PLUS_ONE), &int("7")).expect("default");
    assert_eq!(
        big_by_seven.canonical_lexical(),
        "24305883351495604533098186245126300818.285714285714285714"
    );
    // The exact policy: terminating quotients exactly, a non-terminating one refused.
    let exact = DivisionPolicy::Exact;
    assert_eq!(
        numeric_div_with_policy(&one, &eight, exact)
            .expect("terminates")
            .canonical_lexical(),
        "0.125"
    );
    let refused = numeric_div_with_policy(&one, &three, exact).expect_err("1/3");
    assert_eq!(refused, XsdError::Exact(ExactError::NonTerminating));
    // Another scale and rounding.
    assert_eq!(
        numeric_div_with_policy(
            &int("2"),
            &three,
            DivisionPolicy::scale(4, Rounding::HalfEven)
        )
        .expect("rounded")
        .canonical_lexical(),
        "0.6667"
    );
    // Division by zero is refused under every policy; a unit divisor is not.
    for policy in [DivisionPolicy::default(), exact] {
        assert!(matches!(
            numeric_div_with_policy(&one, &int("0"), policy),
            Err(XsdError::DivisionByZero { .. })
        ));
        assert!(numeric_div_with_policy(&one, &one, policy).is_ok());
    }
    // Floats keep IEEE division under any policy.
    assert_eq!(
        numeric_div_with_policy(&dbl("1"), &dbl("0"), exact)
            .expect("IEEE")
            .canonical_lexical(),
        "INF"
    );
}

#[test]
fn comparison_and_the_exact_order_see_every_digit() {
    let a = int(&format!("1{}1", "0".repeat(41)));
    let b = int(&format!("1{}2", "0".repeat(41)));
    assert_eq!(numeric_cmp(&a, &b), Some(Ordering::Less));
    assert!(!value_eq(&a, &b));
    assert!(value_eq(&a, &dec(&format!("1{}1.0", "0".repeat(41)))));
    // Big against bounded, either side.
    assert_eq!(
        numeric_cmp(&int(I128_MAX_PLUS_ONE), &int(&i128::MAX.to_string())),
        Some(Ordering::Greater)
    );
    assert_eq!(
        numeric_cmp(&dec("0.1000000000000000000000000001"), &dec("0.1")),
        Some(Ordering::Greater)
    );
    // SPARQL `<` against a double promotes (correctly rounded); the order does not.
    let two_pow_127 = dbl("1.7014118346046923E38");
    assert_eq!(
        numeric_cmp(&int(I128_MAX_PLUS_ONE), &two_pow_127),
        Some(Ordering::Equal)
    );
    let near = int("170141183460469231731687303715884105729");
    assert_eq!(numeric_cmp(&near, &two_pow_127), Some(Ordering::Equal));
    assert_eq!(
        numeric_total_cmp(&near, &two_pow_127),
        Some(Ordering::Greater)
    );
    assert_eq!(numeric_total_cmp(&two_pow_127, &near), Some(Ordering::Less));
    assert_eq!(numeric_total_cmp(&near, &dbl("INF")), Some(Ordering::Less));
    assert_eq!(numeric_total_cmp(&near, &dbl("NaN")), None);
    // Promotion to double is correctly rounded.
    let sum = numeric_add(&int(&"9".repeat(400)), &dbl("1")).expect("IEEE");
    assert_eq!(sum.canonical_lexical(), "INF");
    let sum = numeric_add(&dec("0.1000000000000000000000000001"), &dbl("0")).expect("IEEE");
    assert_eq!(sum.canonical_lexical(), "1.0E-1");
}

#[test]
fn rounding_functions_keep_the_family() {
    let big = dec(&format!("{}.5", "1".repeat(40)));
    assert_eq!(
        numeric_round(&big).expect("exact").canonical_lexical(),
        format!("{}2", "1".repeat(39))
    );
    assert_eq!(
        numeric_floor(&big).expect("exact").canonical_lexical(),
        "1".repeat(40)
    );
    assert_eq!(
        numeric_ceil(&big).expect("exact").canonical_lexical(),
        format!("{}2", "1".repeat(39))
    );
    let negative = dec(&format!("-{}.5", "1".repeat(40)));
    assert_eq!(
        numeric_round(&negative).expect("exact").canonical_lexical(),
        format!("-{}", "1".repeat(40))
    );
    assert_eq!(
        numeric_round(&int(I128_MAX_PLUS_ONE))
            .expect("id")
            .canonical_lexical(),
        I128_MAX_PLUS_ONE
    );
}

#[test]
fn cost_is_charged_only_for_tower_operands() {
    let small = int("5");
    let big = int(&"9".repeat(1000));
    assert_eq!(numeric_cost(&small, &small, CostOp::Mul).work(), 0);
    let cost = numeric_cost(&big, &big, CostOp::Mul);
    assert!(cost.work() > 10_000 && cost.bytes() > 400, "{cost:?}");
    assert_eq!(numeric_cost(&big, &dbl("1"), CostOp::Add).work(), 0);
    assert!(
        numeric_cost(&big, &small, CostOp::Div(DivisionPolicy::default())).work()
            >= numeric_cost(&big, &small, CostOp::Add).work()
    );
}

#[test]
fn data_ranges_and_value_identity_are_exact_at_any_size() {
    // Two different 60-digit values are two different values.
    let a = int(&format!("1{}", "0".repeat(59)));
    let b = int(&format!("1{}1", "0".repeat(58)));
    assert!(!same_value(&a, &b));
    assert!(same_value(&a, &a.clone()));
    assert!(same_value(&a, &dec(&format!("1{}.000", "0".repeat(59)))));
    // A one-of over both is two values; intersecting their singletons is empty.
    let one_of = |value: &XsdValue| DataRange::OneOf(vec![value.clone()]);
    assert_eq!(
        satisfiability(&DataRange::And(vec![one_of(&a), one_of(&b)])),
        Satisfiability::Empty
    );
    assert_eq!(
        satisfiability(&DataRange::And(vec![one_of(&a), one_of(&a)])),
        Satisfiability::Inhabited
    );
    // A bound at 10^42 + 1 admits 10^42 + 1 and refuses 10^42 + 2.
    let bound = int(&format!("1{}1", "0".repeat(41)));
    let max_inclusive = DataRange::Restriction {
        base: D::Integer,
        facets: vec![Facet::MaxInclusive(bound.clone())],
    };
    assert_eq!(contains(&max_inclusive, &bound), Known::Yes);
    assert_eq!(
        contains(&max_inclusive, &int(&format!("1{}2", "0".repeat(41)))),
        Known::No
    );
    let min_exclusive = DataRange::Restriction {
        base: D::Integer,
        facets: vec![
            Facet::MinExclusive(bound.clone()),
            Facet::MaxExclusive(int(&format!("1{}2", "0".repeat(41)))),
        ],
    };
    // No integer lies strictly between two consecutive integers…
    assert_eq!(satisfiability(&min_exclusive), Satisfiability::Empty);
    // …but a decimal does.
    let decimal_gap = DataRange::Restriction {
        base: D::Decimal,
        facets: vec![
            Facet::MinExclusive(bound),
            Facet::MaxExclusive(int(&format!("1{}2", "0".repeat(41)))),
        ],
    };
    assert_eq!(satisfiability(&decimal_gap), Satisfiability::Inhabited);
}
