// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every numeric refusal of `purrdf-xsd` is a typed error that names its XPath F&O 3.1
//! code, beside a neighbouring operation that answers; and the unary operators never
//! mint a literal outside its datatype's value space.

use purrdf_xsd::exact::{self, DivisionPolicy, ExactError};
use purrdf_xsd::numeric::numeric_div_with_policy;
use purrdf_xsd::{
    ErrorCode, XsdDatatype as D, XsdError, XsdValue, numeric_abs, numeric_ceil, numeric_div,
    numeric_floor, numeric_round, numeric_unary_minus, numeric_unary_plus, parse,
};

fn value(lexical: &str, datatype: D) -> XsdValue {
    parse(lexical, datatype).unwrap_or_else(|e| panic!("{lexical}: {e}"))
}

fn code(result: Result<XsdValue, XsdError>) -> Option<ErrorCode> {
    result.expect_err("a refusal").code()
}

/// `xsd:integer` and `xsd:decimal` have no bound, so a lexical form of any length is a
/// value; the bounded derived types refuse past their own facets with `FORG0001`.
#[test]
fn lexical_refusals_name_their_code_beside_an_accepted_neighbour() {
    let huge = format!("1{}", "0".repeat(60));
    assert!(matches!(
        value(&huge, D::Integer),
        XsdValue::BigInteger { .. }
    ));
    assert!(matches!(
        value(&format!("0.{}1", "0".repeat(60)), D::Decimal),
        XsdValue::BigDecimal(_)
    ));
    assert_eq!(code(parse("300", D::Byte)), Some(ErrorCode::Forg0001));
    assert!(parse("127", D::Byte).is_ok());
    assert_eq!(code(parse(&huge, D::Long)), Some(ErrorCode::Forg0001));
    assert!(parse(&i64::MAX.to_string(), D::Long).is_ok());
    assert_eq!(
        code(parse("-1", D::NonNegativeInteger)),
        Some(ErrorCode::Forg0001)
    );
    assert!(matches!(
        value(&huge, D::NonNegativeInteger),
        XsdValue::BigInteger { .. }
    ));
    assert_eq!(code(parse("1.5", D::Integer)), Some(ErrorCode::Forg0001));
    assert_eq!(code(parse("1.5x", D::Decimal)), Some(ErrorCode::Forg0001));
    assert!(parse("1.5", D::Decimal).is_ok());
}

/// The operator refusals: exact division by zero, and a quotient the caller's policy
/// cannot express; each beside the neighbour that answers.
#[test]
fn operator_refusals_name_their_code_beside_an_answering_neighbour() {
    let (one, two, three, zero) = (
        value("1", D::Integer),
        value("2", D::Integer),
        value("3", D::Integer),
        value("0", D::Integer),
    );
    assert_eq!(code(numeric_div(&one, &zero)), Some(ErrorCode::Foar0001));
    assert_eq!(
        code(numeric_div(
            &value("1.5", D::Decimal),
            &value("0.0", D::Decimal)
        )),
        Some(ErrorCode::Foar0001)
    );
    assert_eq!(
        numeric_div(&one, &two)
            .expect("a quotient")
            .canonical_lexical(),
        "0.5"
    );
    assert_eq!(
        code(numeric_div_with_policy(&one, &three, DivisionPolicy::Exact)),
        Some(ErrorCode::Foar0002)
    );
    assert_eq!(
        numeric_div_with_policy(&one, &two, DivisionPolicy::Exact)
            .expect("a terminating quotient")
            .canonical_lexical(),
        "0.5"
    );
    // A type error is XPTY0004, beside the numeric operand that answers.
    assert_eq!(
        code(numeric_unary_minus(&value("true", D::Boolean))),
        Some(ErrorCode::Xpty0004)
    );
    assert!(numeric_unary_minus(&one).is_ok());
}

/// The tower's own refusals carry their code too, and an `XsdError` wrapping one
/// reports the same code.
#[test]
fn tower_refusals_name_their_code() {
    let cases = [
        (ExactError::DivisionByZero, ErrorCode::Foar0001),
        (ExactError::NotFinite, ErrorCode::Foca0002),
        (ExactError::NonTerminating, ErrorCode::Foar0002),
        (ExactError::ScaleOverflow, ErrorCode::Foar0002),
    ];
    for (error, expected) in cases {
        assert_eq!(error.code(), expected, "{error}");
        assert_eq!(XsdError::Exact(error).code(), Some(expected));
    }
    assert_eq!(
        exact::Decimal::from_f64(f64::NAN).unwrap_err().code(),
        ErrorCode::Foca0002
    );
    assert!(exact::Decimal::from_f64(1e300).is_ok());
    let past = format!("1{}", "0".repeat(40));
    let big: exact::Integer = past.parse().expect("an integer");
    assert_eq!(big.to_i128().unwrap_err().code(), ErrorCode::Foca0003);
    let fits: exact::Integer = "12".parse().expect("an integer");
    assert_eq!(fits.to_i128(), Ok(12));
    let fine: exact::Decimal = "0.0000000000000000001".parse().expect("a decimal");
    assert_eq!(fine.to_bounded().unwrap_err().code(), ErrorCode::Foca0001);
    let coarse: exact::Decimal = "0.000000000000000001".parse().expect("a decimal");
    assert!(coarse.to_bounded().is_ok());
    // The bounded parsers keep their codes.
    assert_eq!(
        purrdf_xsd::numeric::parse_integer(&past)
            .unwrap_err()
            .code(),
        Some(ErrorCode::Foca0003)
    );
    assert_eq!(
        purrdf_xsd::numeric::parse_decimal("0.1000000000000000001")
            .unwrap_err()
            .code(),
        Some(ErrorCode::Foca0006)
    );
    assert!(purrdf_xsd::numeric::parse_decimal("0.100000000000000001").is_ok());
}

/// Every error presents a stable identity, its fields and its code as typed
/// arguments, in English identical to its `Display`, which ends with the code.
#[test]
fn every_error_presents_its_code() {
    let errors = [
        parse("1.5", D::Integer).unwrap_err(),
        parse("300", D::Byte).unwrap_err(),
        numeric_div(&value("1", D::Integer), &value("0", D::Integer)).unwrap_err(),
        numeric_unary_minus(&value("true", D::Boolean)).unwrap_err(),
        XsdError::Exact(ExactError::NonTerminating),
    ];
    let identities = [
        "xsd-invalid-lexical",
        "xsd-out-of-range",
        "xsd-division-by-zero",
        "xsd-type-mismatch",
        "xsd-exact",
    ];
    for (error, identity) in errors.iter().zip(identities) {
        let presentation = error.presentation();
        assert_eq!(presentation.message_id(), identity, "{error}");
        assert_eq!(presentation.english(), error.to_string());
        let code = error.code().expect("every numeric refusal has a code");
        assert!(
            error.to_string().ends_with(&format!("({})", code.qname())),
            "{error}"
        );
    }
    for code in ErrorCode::ALL {
        assert_eq!(code.qname(), format!("err:{}", code.local_name()));
        assert_eq!(code.to_string(), code.qname());
    }
}

/// Negation and the absolute value of a derived integer type are `xsd:integer`, at any
/// size: `-5` is not an `xsd:unsignedByte`. Unary plus is the identity, type included.
#[test]
fn unary_results_of_a_derived_integer_are_xsd_integer() {
    let five = value("5", D::UnsignedByte);
    let negated = numeric_unary_minus(&five).expect("a value");
    assert_eq!(negated.datatype(), D::Integer);
    assert_eq!(negated.canonical_lexical(), "-5");
    let absolute = numeric_abs(&value("-5", D::NegativeInteger)).expect("a value");
    assert_eq!(absolute.datatype(), D::Integer);
    assert_eq!(absolute.canonical_lexical(), "5");
    for op in [numeric_ceil, numeric_floor, numeric_round] {
        let rounded = op(&five).expect("identity");
        assert_eq!(rounded.datatype(), D::Integer);
        assert_eq!(rounded.canonical_lexical(), "5");
    }
    let huge = value(&format!("-1{}", "0".repeat(40)), D::NegativeInteger);
    for result in [numeric_unary_minus(&huge), numeric_abs(&huge)] {
        let result = result.expect("a value");
        assert_eq!(result.datatype(), D::Integer);
        assert!(matches!(result, XsdValue::BigInteger { .. }));
        assert_eq!(result.canonical_lexical(), format!("1{}", "0".repeat(40)));
    }
    for op in [numeric_ceil, numeric_floor, numeric_round] {
        assert_eq!(op(&huge).expect("identity").datatype(), D::Integer);
    }
    assert_eq!(
        numeric_unary_plus(&five).expect("identity").datatype(),
        D::UnsignedByte
    );
}

/// A value of the wrong sign for one of the four sign-restricted integer types is
/// outside that type's value space (`FORG0001`) at every magnitude, as it is for the
/// bounded types — never the `i128` narrowing code (`FOCA0003`) of a value too large to
/// read: `xsd:nonNegativeInteger` refuses `-10^60` as it refuses `-1`, beside `10^60`,
/// which it holds.
#[test]
fn a_huge_value_outside_a_sign_restricted_type_is_forg0001() {
    let huge = format!("1{}", "0".repeat(60));
    for (refused, held, datatype) in [
        (format!("-{huge}"), huge.clone(), D::NonNegativeInteger),
        (format!("-{huge}"), huge.clone(), D::PositiveInteger),
        (huge.clone(), format!("-{huge}"), D::NegativeInteger),
        (huge.clone(), format!("-{huge}"), D::NonPositiveInteger),
    ] {
        assert_eq!(
            code(parse(&refused, datatype)),
            Some(ErrorCode::Forg0001),
            "{refused} as {datatype:?}"
        );
        assert!(
            matches!(value(&held, datatype), XsdValue::BigInteger { .. }),
            "{held} as {datatype:?}"
        );
    }
    // The bounded types already answer FORG0001 past their facets, at every size.
    assert_eq!(code(parse(&huge, D::Long)), Some(ErrorCode::Forg0001));
    assert_eq!(
        code(parse(&format!("-{huge}"), D::UnsignedByte)),
        Some(ErrorCode::Forg0001)
    );
}
