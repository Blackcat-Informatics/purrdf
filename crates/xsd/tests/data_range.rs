// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Datatype-range satisfiability vectors: the emptiness proofs a description-logic
//! reasoner depends on, the exactness boundary it must report, and the two soundness
//! invariants that tie `satisfiability` to `contains`.

use purrdf_xsd::range::{
    Cardinality, DataRange, Facet, Known, Satisfiability, cardinality, contains,
    is_exactly_decided, same_value, satisfiability,
};
use purrdf_xsd::{XsdDatatype as D, XsdValue, parse, value_eq};

/// Parse a lexical form the vector knows is valid.
fn v(lexical: &str, dt: D) -> XsdValue {
    parse(lexical, dt).unwrap_or_else(|e| panic!("parse({lexical:?}, {dt:?}) failed: {e}"))
}

/// `owl:onDatatype base` restricted by `facets`.
fn restrict(base: D, facets: Vec<Facet>) -> DataRange {
    DataRange::Restriction { base, facets }
}

/// A two-sided bound restriction whose bound values are of the base's own datatype.
fn between(base: D, lower: Facet, upper: Facet) -> DataRange {
    restrict(base, vec![lower, upper])
}

// ── Contradictory numeric bounds ─────────────────────────────────────────────────

#[test]
fn a_lower_bound_above_the_upper_bound_is_empty_in_every_numeric_space() {
    for (base, low, high) in [
        (D::Integer, v("5", D::Integer), v("3", D::Integer)),
        (D::Decimal, v("5", D::Decimal), v("3", D::Decimal)),
        (D::Double, v("5", D::Double), v("3", D::Double)),
        (D::Byte, v("5", D::Byte), v("3", D::Byte)),
    ] {
        let range = between(base, Facet::MinInclusive(low), Facet::MaxInclusive(high));
        assert_eq!(
            satisfiability(&range),
            Satisfiability::Empty,
            "{base:?} with minInclusive 5 and maxInclusive 3"
        );
        assert!(is_exactly_decided(&range));
        assert_eq!(cardinality(&range), Cardinality::Exactly(0));
    }
}

#[test]
fn the_integers_are_discrete_and_the_decimals_are_dense() {
    let open = |base, low: &str, high: &str, dt| {
        between(
            base,
            Facet::MinExclusive(v(low, dt)),
            Facet::MaxExclusive(v(high, dt)),
        )
    };

    // 4 is the witness between 3 and 5.
    let witnessed = open(D::Integer, "3", "5", D::Integer);
    assert_eq!(satisfiability(&witnessed), Satisfiability::Inhabited);
    assert_eq!(cardinality(&witnessed), Cardinality::Exactly(1));
    assert_eq!(contains(&witnessed, &v("4", D::Integer)), Known::Yes);

    // No integer lies strictly between 3 and 4.
    assert_eq!(
        satisfiability(&open(D::Integer, "3", "4", D::Integer)),
        Satisfiability::Empty
    );

    // The decimals are dense: both open intervals hold values.
    for (low, high) in [("3", "5"), ("3", "4")] {
        let range = open(D::Decimal, low, high, D::Decimal);
        assert_eq!(
            satisfiability(&range),
            Satisfiability::Inhabited,
            "decimal ({low}, {high})"
        );
        assert_eq!(cardinality(&range), Cardinality::Unbounded);
    }
}

#[test]
fn a_bound_outside_a_subtypes_own_range_is_empty() {
    // `xsd:byte` runs -128 ..= 127, so nothing in it reaches 200.
    let range = restrict(D::Byte, vec![Facet::MinInclusive(v("200", D::Integer))]);
    assert_eq!(satisfiability(&range), Satisfiability::Empty);
    assert!(is_exactly_decided(&range));

    // Inside the subtype's range the same shape is inhabited and exactly counted.
    let inside = between(
        D::Byte,
        Facet::MinInclusive(v("120", D::Byte)),
        Facet::MaxInclusive(v("127", D::Byte)),
    );
    assert_eq!(cardinality(&inside), Cardinality::Exactly(8));
}

#[test]
fn derived_integer_datatypes_carry_their_own_value_space() {
    assert_eq!(
        satisfiability(&DataRange::And(vec![
            DataRange::Datatype(D::NonNegativeInteger),
            restrict(D::Integer, vec![Facet::MaxInclusive(v("-1", D::Integer))]),
        ])),
        Satisfiability::Empty
    );
    // The unbounded derived types stay unbounded rather than capping at `i128`.
    assert_eq!(
        cardinality(&DataRange::Datatype(D::NonNegativeInteger)),
        Cardinality::Unbounded
    );
    assert_eq!(
        cardinality(&DataRange::Datatype(D::UnsignedByte)),
        Cardinality::Exactly(256)
    );
}

// ── Boolean combinations across disjoint spaces ──────────────────────────────────

#[test]
fn a_datatype_and_its_complement_are_empty() {
    let range = DataRange::And(vec![
        DataRange::Datatype(D::Integer),
        DataRange::Not(Box::new(DataRange::Datatype(D::Integer))),
    ]);
    assert_eq!(satisfiability(&range), Satisfiability::Empty);
    assert!(is_exactly_decided(&range));
}

#[test]
fn the_complement_of_the_whole_domain_is_empty() {
    assert_eq!(
        satisfiability(&DataRange::Not(Box::new(DataRange::Any))),
        Satisfiability::Empty
    );
    assert_eq!(satisfiability(&DataRange::Any), Satisfiability::Inhabited);
    assert_eq!(cardinality(&DataRange::Any), Cardinality::Unbounded);
}

#[test]
fn the_complement_of_one_datatype_is_witnessed_by_another() {
    let range = DataRange::Not(Box::new(DataRange::Datatype(D::Integer)));
    assert_eq!(satisfiability(&range), Satisfiability::Inhabited);
    // A string is one witness; an integer is not a member.
    assert_eq!(contains(&range, &v("hello", D::String)), Known::Yes);
    assert_eq!(contains(&range, &v("1", D::Integer)), Known::No);
    // A non-integral decimal is another.
    assert_eq!(contains(&range, &v("1.5", D::Decimal)), Known::Yes);
}

#[test]
fn disjoint_value_spaces_intersect_to_nothing() {
    for (left, right) in [
        (D::Integer, D::String),
        (D::Float, D::Double),
        (D::Decimal, D::Double),
        (D::HexBinary, D::Base64Binary),
        (D::Date, D::DateTime),
        (D::GYear, D::GMonth),
    ] {
        let range = DataRange::And(vec![DataRange::Datatype(left), DataRange::Datatype(right)]);
        assert_eq!(
            satisfiability(&range),
            Satisfiability::Empty,
            "{left:?} and {right:?} are disjoint value spaces"
        );
    }
}

#[test]
fn the_integers_are_a_subset_of_the_decimals() {
    let range = DataRange::And(vec![
        DataRange::Datatype(D::Integer),
        DataRange::Datatype(D::Decimal),
    ]);
    assert_eq!(satisfiability(&range), Satisfiability::Inhabited);
    assert_eq!(contains(&range, &v("5", D::Integer)), Known::Yes);
    assert_eq!(contains(&range, &v("5.5", D::Decimal)), Known::No);
}

#[test]
fn the_non_integral_decimals_are_inhabited_and_a_pinned_integer_is_not() {
    let fractional = DataRange::And(vec![
        DataRange::Datatype(D::Decimal),
        DataRange::Not(Box::new(DataRange::Datatype(D::Integer))),
    ]);
    assert_eq!(satisfiability(&fractional), Satisfiability::Inhabited);
    assert_eq!(contains(&fractional, &v("0.5", D::Decimal)), Known::Yes);
    assert_eq!(contains(&fractional, &v("1", D::Decimal)), Known::No);

    let pinned = DataRange::And(vec![
        between(
            D::Integer,
            Facet::MinInclusive(v("1", D::Integer)),
            Facet::MaxInclusive(v("1", D::Integer)),
        ),
        DataRange::Not(Box::new(DataRange::OneOf(vec![v("1", D::Integer)]))),
    ]);
    assert_eq!(satisfiability(&pinned), Satisfiability::Empty);
    assert!(is_exactly_decided(&pinned));
}

// ── Enumerations count values, not lexical forms ─────────────────────────────────

#[test]
fn an_enumeration_counts_distinct_values() {
    let rows: [(Vec<XsdValue>, Cardinality); 5] = [
        // One value, two lexical forms.
        (
            vec![v("1", D::Integer), v("01", D::Integer)],
            Cardinality::Exactly(1),
        ),
        // One value, two datatypes of the SAME value space.
        (
            vec![v("5", D::Integer), v("5.0", D::Decimal)],
            Cardinality::Exactly(1),
        ),
        // Two values: the float and double value spaces are disjoint.
        (
            vec![v("5", D::Float), v("5", D::Double)],
            Cardinality::Exactly(2),
        ),
        // Three values across three spaces.
        (
            vec![v("5", D::Integer), v("5", D::String), v("true", D::Boolean)],
            Cardinality::Exactly(3),
        ),
        // The value space of `xsd:double` holds exactly one NaN.
        (
            vec![v("NaN", D::Double), v("NaN", D::Double)],
            Cardinality::Exactly(1),
        ),
    ];
    for (values, want) in rows {
        let range = DataRange::OneOf(values);
        assert_eq!(cardinality(&range), want, "{range:?}");
        assert_eq!(satisfiability(&range), Satisfiability::Inhabited);
        assert!(is_exactly_decided(&range));
    }
    assert_eq!(
        satisfiability(&DataRange::OneOf(Vec::new())),
        Satisfiability::Empty
    );
}

// ── Length-selected spaces ───────────────────────────────────────────────────────

#[test]
fn contradictory_length_facets_are_empty() {
    for base in [D::String, D::HexBinary, D::Base64Binary] {
        let range = restrict(base, vec![Facet::MinLength(5), Facet::MaxLength(3)]);
        assert_eq!(
            satisfiability(&range),
            Satisfiability::Empty,
            "{base:?} with minLength 5 and maxLength 3"
        );
        assert!(is_exactly_decided(&range));
    }
}

#[test]
fn a_length_zero_space_holds_exactly_one_value() {
    let rows = [(D::String, ""), (D::HexBinary, ""), (D::Base64Binary, "")];
    for (base, empty_lexical) in rows {
        let zero = restrict(base, vec![Facet::Length(0)]);
        assert_eq!(cardinality(&zero), Cardinality::Exactly(1), "{base:?}");

        // Removing that one value empties the range.
        let minus = DataRange::And(vec![
            zero,
            DataRange::Not(Box::new(DataRange::OneOf(vec![v(empty_lexical, base)]))),
        ]);
        assert_eq!(satisfiability(&minus), Satisfiability::Empty, "{base:?}");
        assert!(is_exactly_decided(&minus));
    }
}

#[test]
fn removing_one_value_from_a_populous_length_leaves_the_rest() {
    // A single character position admits far more than one string.
    let strings = DataRange::And(vec![
        restrict(D::String, vec![Facet::Length(1)]),
        DataRange::Not(Box::new(DataRange::OneOf(vec![v("x", D::String)]))),
    ]);
    assert_eq!(satisfiability(&strings), Satisfiability::Inhabited);
    assert_eq!(contains(&strings, &v("x", D::String)), Known::No);
    assert_eq!(contains(&strings, &v("y", D::String)), Known::Yes);
    assert_eq!(contains(&strings, &v("xy", D::String)), Known::No);

    // One OCTET admits 256 byte sequences; removing one leaves 255.
    let octet = restrict(D::HexBinary, vec![Facet::Length(1)]);
    assert_eq!(cardinality(&octet), Cardinality::Exactly(256));
    let minus = DataRange::And(vec![
        octet,
        DataRange::Not(Box::new(DataRange::OneOf(vec![v("ff", D::HexBinary)]))),
    ]);
    assert_eq!(satisfiability(&minus), Satisfiability::Inhabited);
    assert_eq!(cardinality(&minus), Cardinality::Exactly(255));
    assert_eq!(contains(&minus, &v("ff", D::HexBinary)), Known::No);
    assert_eq!(contains(&minus, &v("00", D::HexBinary)), Known::Yes);
}

#[test]
fn binary_length_counts_octets_not_lexical_characters() {
    // "ff" is one octet in hex; "/w==" is one octet in base64.
    assert_eq!(
        contains(
            &restrict(D::HexBinary, vec![Facet::Length(1)]),
            &v("ff", D::HexBinary)
        ),
        Known::Yes
    );
    assert_eq!(
        contains(
            &restrict(D::Base64Binary, vec![Facet::Length(1)]),
            &v("/w==", D::Base64Binary)
        ),
        Known::Yes
    );
    // `xsd:string` counts CHARACTERS, not the UTF-8 bytes behind them.
    assert_eq!(
        contains(
            &restrict(D::String, vec![Facet::Length(1)]),
            &v("é", D::String)
        ),
        Known::Yes
    );
}

// ── The boolean space ────────────────────────────────────────────────────────────

#[test]
fn the_boolean_space_is_exhaustible() {
    let both = DataRange::OneOf(vec![v("true", D::Boolean), v("false", D::Boolean)]);
    let minus_both = DataRange::And(vec![
        DataRange::Datatype(D::Boolean),
        DataRange::Not(Box::new(both)),
    ]);
    assert_eq!(satisfiability(&minus_both), Satisfiability::Empty);
    assert!(is_exactly_decided(&minus_both));

    let minus_one = DataRange::And(vec![
        DataRange::Datatype(D::Boolean),
        DataRange::Not(Box::new(DataRange::OneOf(vec![v("true", D::Boolean)]))),
    ]);
    assert_eq!(cardinality(&minus_one), Cardinality::Exactly(1));
    assert_eq!(contains(&minus_one, &v("false", D::Boolean)), Known::Yes);
    assert_eq!(contains(&minus_one, &v("true", D::Boolean)), Known::No);
    assert_eq!(
        cardinality(&DataRange::Datatype(D::Boolean)),
        Cardinality::Exactly(2)
    );
}

#[test]
fn a_facet_on_the_boolean_space_is_not_decided() {
    // XSD gives `xsd:boolean` no ordering or length facet.
    let range = restrict(D::Boolean, vec![Facet::MinInclusive(v("true", D::Boolean))]);
    assert_eq!(satisfiability(&range), Satisfiability::Undecided);
    assert!(!is_exactly_decided(&range));
}

// ── Temporal spaces: enumerations are exact, bounds are not ──────────────────────

#[test]
fn contradictory_date_bounds_are_empty_and_satisfiable_ones_are_witnessed() {
    let contradiction = between(
        D::Date,
        Facet::MinInclusive(v("2000-01-01", D::Date)),
        Facet::MaxInclusive(v("1999-01-01", D::Date)),
    );
    assert_eq!(satisfiability(&contradiction), Satisfiability::Empty);
    assert!(is_exactly_decided(&contradiction));

    let witnessed = between(
        D::Date,
        Facet::MinInclusive(v("1999-01-01", D::Date)),
        Facet::MaxInclusive(v("2000-01-01", D::Date)),
    );
    assert_eq!(satisfiability(&witnessed), Satisfiability::Inhabited);
    // The XSD order on this space is partial, so the interval is exhibited, not
    // exactly represented — nothing is known about what its complement holds inside
    // the date space.
    assert!(!is_exactly_decided(&witnessed));
    let complement = DataRange::Not(Box::new(witnessed));
    assert!(!is_exactly_decided(&complement));
    assert_eq!(
        contains(&complement, &v("1999-06-01", D::Date)),
        Known::Unknown
    );
    // Narrowed back to the date space, the complement is undecided; the whole-domain
    // complement stays inhabited because a value of another space witnesses it.
    assert_eq!(
        satisfiability(&DataRange::And(vec![
            DataRange::Datatype(D::Date),
            complement.clone(),
        ])),
        Satisfiability::Undecided
    );
    assert_eq!(satisfiability(&complement), Satisfiability::Inhabited);
}

#[test]
fn an_exclusive_temporal_bound_meeting_its_inclusive_twin_is_empty() {
    let range = between(
        D::DateTime,
        Facet::MinExclusive(v("2000-01-01T00:00:00Z", D::DateTime)),
        Facet::MaxInclusive(v("2000-01-01T00:00:00Z", D::DateTime)),
    );
    assert_eq!(satisfiability(&range), Satisfiability::Empty);
}

#[test]
fn a_temporal_enumeration_and_its_complement_are_decided_exactly() {
    let dates = DataRange::OneOf(vec![v("1999-01-01", D::Date), v("2000-01-01", D::Date)]);
    assert_eq!(cardinality(&dates), Cardinality::Exactly(2));
    assert!(is_exactly_decided(&dates));

    let complement = DataRange::Not(Box::new(dates.clone()));
    assert_eq!(satisfiability(&complement), Satisfiability::Inhabited);
    assert!(is_exactly_decided(&complement));
    assert_eq!(contains(&complement, &v("1999-01-01", D::Date)), Known::No);
    assert_eq!(contains(&complement, &v("2001-01-01", D::Date)), Known::Yes);

    let both = DataRange::And(vec![dates, complement]);
    assert_eq!(satisfiability(&both), Satisfiability::Empty);
    assert!(is_exactly_decided(&both));
}

#[test]
fn a_timezone_shifted_temporal_literal_is_the_same_value() {
    let listed = DataRange::OneOf(vec![v("2002-10-10T17:00:00Z", D::DateTime)]);
    assert_eq!(
        contains(&listed, &v("2002-10-10T12:00:00-05:00", D::DateTime)),
        Known::Yes
    );
    assert_eq!(cardinality(&listed), Cardinality::Exactly(1));
}

#[test]
fn the_duration_subtypes_are_inhabited_but_not_exact() {
    for dt in [D::DayTimeDuration, D::YearMonthDuration] {
        let range = DataRange::Datatype(dt);
        assert_eq!(satisfiability(&range), Satisfiability::Inhabited, "{dt:?}");
        assert!(!is_exactly_decided(&range), "{dt:?}");
        // Nothing is known about the rest of the duration space.
        let rest = DataRange::And(vec![
            DataRange::Datatype(D::Duration),
            DataRange::Not(Box::new(range)),
        ]);
        assert_eq!(satisfiability(&rest), Satisfiability::Undecided, "{dt:?}");
        assert_eq!(
            contains(&rest, &v("PT1S", D::DayTimeDuration)),
            Known::Unknown,
            "{dt:?}"
        );
    }
    // The general duration IS exact.
    assert!(is_exactly_decided(&DataRange::Datatype(D::Duration)));
}

// ── Opaque ranges ────────────────────────────────────────────────────────────────

#[test]
fn an_opaque_range_is_undecided_but_cannot_defeat_a_witness() {
    assert_eq!(
        satisfiability(&DataRange::Opaque),
        Satisfiability::Undecided
    );
    assert!(!is_exactly_decided(&DataRange::Opaque));
    assert_eq!(cardinality(&DataRange::Opaque), Cardinality::Undecided);
    assert_eq!(
        contains(&DataRange::Opaque, &v("1", D::Integer)),
        Known::Unknown
    );

    // A witness on the other side of a union stands whatever the opaque operand is.
    let union = DataRange::Or(vec![DataRange::Datatype(D::Integer), DataRange::Opaque]);
    assert_eq!(satisfiability(&union), Satisfiability::Inhabited);
    assert!(!is_exactly_decided(&union));

    // An intersection with it settles nothing.
    let intersection = DataRange::And(vec![DataRange::Datatype(D::Integer), DataRange::Opaque]);
    assert_eq!(satisfiability(&intersection), Satisfiability::Undecided);
    assert!(!is_exactly_decided(&intersection));

    // The complement of an unmodelled range is unmodelled too — an unmodelled value
    // space may overlap a modelled one, so nothing may be assumed empty.
    assert_eq!(
        satisfiability(&DataRange::Not(Box::new(DataRange::Opaque))),
        Satisfiability::Undecided
    );
}

// ── The exactness boundary ───────────────────────────────────────────────────────

#[test]
fn ranges_over_the_non_temporal_spaces_with_applicable_facets_are_exact() {
    let ranges = vec![
        DataRange::Any,
        DataRange::Datatype(D::Integer),
        DataRange::Datatype(D::Decimal),
        DataRange::Datatype(D::Float),
        DataRange::Datatype(D::Double),
        DataRange::Datatype(D::Boolean),
        DataRange::Datatype(D::String),
        DataRange::Datatype(D::HexBinary),
        DataRange::Datatype(D::Base64Binary),
        DataRange::Datatype(D::UnsignedShort),
        restrict(D::Integer, vec![Facet::MinInclusive(v("0", D::Integer))]),
        restrict(D::Decimal, vec![Facet::MaxExclusive(v("1.5", D::Decimal))]),
        restrict(D::Float, vec![Facet::MinExclusive(v("1.5", D::Float))]),
        restrict(D::Double, vec![Facet::MaxInclusive(v("INF", D::Double))]),
        restrict(D::String, vec![Facet::MinLength(2), Facet::MaxLength(9)]),
        restrict(D::HexBinary, vec![Facet::Length(4)]),
        restrict(D::Base64Binary, vec![Facet::MaxLength(3)]),
        DataRange::OneOf(vec![v("1", D::Integer), v("a", D::String)]),
    ];
    for range in &ranges {
        assert!(is_exactly_decided(range), "{range:?}");
        assert_ne!(
            satisfiability(range),
            Satisfiability::Undecided,
            "{range:?}"
        );
        assert_ne!(cardinality(range), Cardinality::Undecided, "{range:?}");
    }
    // Exactness is closed under the boolean operators.
    let combined = DataRange::Not(Box::new(DataRange::And(vec![
        DataRange::Or(ranges.clone()),
        DataRange::Not(Box::new(DataRange::And(ranges))),
    ])));
    assert!(is_exactly_decided(&combined));
    assert_ne!(satisfiability(&combined), Satisfiability::Undecided);
}

#[test]
fn an_inapplicable_facet_is_not_silently_ignored() {
    let rows = [
        // A bound facet on a length-selected space.
        restrict(D::String, vec![Facet::MinInclusive(v("a", D::String))]),
        restrict(
            D::HexBinary,
            vec![Facet::MaxInclusive(v("ff", D::HexBinary))],
        ),
        // A length facet on a number.
        restrict(D::Integer, vec![Facet::MinLength(1)]),
        restrict(D::Double, vec![Facet::Length(2)]),
        // A length facet on a temporal space.
        restrict(D::Date, vec![Facet::MaxLength(4)]),
        // A bound whose value comes from another value space.
        restrict(D::Double, vec![Facet::MinInclusive(v("1", D::Integer))]),
        restrict(D::Float, vec![Facet::MinInclusive(v("1", D::Double))]),
        restrict(D::Integer, vec![Facet::MinInclusive(v("1", D::Float))]),
        restrict(D::Date, vec![Facet::MinInclusive(v("2000", D::GYear))]),
        // Nothing compares with NaN.
        restrict(D::Double, vec![Facet::MinInclusive(v("NaN", D::Double))]),
    ];
    for range in rows {
        assert!(!is_exactly_decided(&range), "{range:?}");
        assert_eq!(
            satisfiability(&range),
            Satisfiability::Undecided,
            "{range:?}"
        );
    }
}

#[test]
fn an_enumerated_float_zero_is_exhibited_rather_than_exact() {
    let zero = DataRange::OneOf(vec![v("0.0", D::Double)]);
    assert_eq!(satisfiability(&zero), Satisfiability::Inhabited);
    assert!(!is_exactly_decided(&zero));
    // A set that must hold one zero and not the other is outside the interval algebra,
    // so the double space of the complement is unknown.
    let complement = DataRange::Not(Box::new(zero));
    assert!(!is_exactly_decided(&complement));
    assert_eq!(
        satisfiability(&DataRange::And(vec![
            DataRange::Datatype(D::Double),
            complement,
        ])),
        Satisfiability::Undecided
    );
    // A non-zero enumeration is exact.
    assert!(is_exactly_decided(&DataRange::OneOf(vec![v(
        "1.0",
        D::Double
    )])));
}

// ── The float and double spaces ──────────────────────────────────────────────────

#[test]
fn a_bound_facet_excludes_the_nan_of_its_space() {
    let bounded = restrict(D::Double, vec![Facet::MinInclusive(v("0", D::Double))]);
    assert_eq!(contains(&bounded, &v("NaN", D::Double)), Known::No);
    assert_eq!(contains(&bounded, &v("INF", D::Double)), Known::Yes);
    assert_eq!(contains(&bounded, &v("-INF", D::Double)), Known::No);

    // NaN is still a member of the unrestricted space, and of its own singleton.
    assert_eq!(
        contains(&DataRange::Datatype(D::Double), &v("NaN", D::Double)),
        Known::Yes
    );
    let just_nan = DataRange::OneOf(vec![v("NaN", D::Double)]);
    assert_eq!(cardinality(&just_nan), Cardinality::Exactly(1));
    assert_eq!(contains(&just_nan, &v("0.0", D::Double)), Known::No);
}

#[test]
fn adjacent_float_values_leave_no_room_between_them() {
    let low = v("1.0", D::Double);
    let next = XsdValue::Double(1.0_f64.next_up());
    let gap = between(
        D::Double,
        Facet::MinExclusive(low.clone()),
        Facet::MaxExclusive(next.clone()),
    );
    assert_eq!(satisfiability(&gap), Satisfiability::Empty);
    // One more step apart and the intervening value is the witness.
    let wider = between(
        D::Double,
        Facet::MinExclusive(low),
        Facet::MaxExclusive(XsdValue::Double(1.0_f64.next_up().next_up())),
    );
    assert_eq!(satisfiability(&wider), Satisfiability::Inhabited);
    assert_eq!(contains(&wider, &next), Known::Yes);
}

// ── Value-space identity ─────────────────────────────────────────────────────────

#[test]
fn value_space_identity_differs_from_sparql_equality_in_exactly_two_places() {
    // 1. One NaN per value space.
    let nan = v("NaN", D::Double);
    assert!(same_value(&nan, &nan));
    assert!(!value_eq(&nan, &nan));

    // 2. Identity holds only within one value space.
    let decimal = v("5.0", D::Decimal);
    let float = v("5", D::Float);
    let double = v("5", D::Double);
    assert!(!same_value(&decimal, &float));
    assert!(!same_value(&decimal, &double));
    assert!(!same_value(&float, &double));
    assert!(value_eq(&decimal, &float));
    assert!(value_eq(&decimal, &double));
    assert!(value_eq(&float, &double));

    // The integer value space is a subset of the decimal value space, so these agree.
    let integer = v("5", D::Integer);
    assert!(same_value(&integer, &decimal));
    assert!(value_eq(&integer, &decimal));

    // Everywhere else the two answers coincide.
    let pairs = [
        (v("1", D::Integer), v("01", D::Integer)),
        (v("1", D::Integer), v("2", D::Integer)),
        (v("a", D::String), v("a", D::String)),
        (v("a", D::String), v("b", D::String)),
        (v("1", D::Integer), v("1", D::String)),
        (v("true", D::Boolean), v("1", D::Boolean)),
        (v("2000-01-01", D::Date), v("2000-01-01", D::Date)),
        (v("ff", D::HexBinary), v("FF", D::HexBinary)),
        (v("ff", D::HexBinary), v("/w==", D::Base64Binary)),
    ];
    for (a, b) in pairs {
        assert_eq!(
            same_value(&a, &b),
            value_eq(&a, &b),
            "same_value and value_eq disagree on ({a:?}, {b:?})"
        );
    }
}

// ── Soundness invariants ─────────────────────────────────────────────────────────

mod prop {
    use super::{DataRange, Facet, Known, Satisfiability, contains, satisfiability, v};
    use proptest::prelude::*;
    use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
    use purrdf_xsd::{XsdDatatype as D, XsdValue};

    /// A fixed RNG seed: this repository forbids a nondeterministic test outcome, so the
    /// generated cases are the same on every run and every machine.
    const SEED: [u8; 32] = *b"purrdf-xsd data range seed 00001";

    /// The values every generated range is probed with.
    fn probes() -> Vec<XsdValue> {
        let mut out = Vec::new();
        for n in 0..5 {
            out.push(v(&n.to_string(), D::Integer));
        }
        for lexical in ["0.5", "2.5", "-1"] {
            out.push(v(lexical, D::Decimal));
        }
        for lexical in ["", "a", "ab"] {
            out.push(v(lexical, D::String));
        }
        out.push(v("true", D::Boolean));
        out.push(v("false", D::Boolean));
        out.push(v("NaN", D::Double));
        out.push(v("1.0", D::Double));
        out
    }

    /// Small random ranges over a tiny signature.
    fn ranges() -> impl Strategy<Value = DataRange> {
        let leaf = prop_oneof![
            Just(DataRange::Any),
            Just(DataRange::Opaque),
            Just(DataRange::Datatype(D::Integer)),
            Just(DataRange::Datatype(D::Decimal)),
            Just(DataRange::Datatype(D::String)),
            Just(DataRange::Datatype(D::Boolean)),
            Just(DataRange::Datatype(D::Double)),
            (0i32..5).prop_map(|n| DataRange::OneOf(vec![v(&n.to_string(), D::Integer)])),
            (0usize..3).prop_map(|i| DataRange::OneOf(vec![v(["", "a", "ab"][i], D::String)])),
            (0i32..5, 0i32..5).prop_map(|(low, high)| DataRange::Restriction {
                base: D::Integer,
                facets: vec![
                    Facet::MinInclusive(v(&low.to_string(), D::Integer)),
                    Facet::MaxExclusive(v(&high.to_string(), D::Integer)),
                ],
            }),
            (0u64..3, 0u64..3).prop_map(|(low, high)| DataRange::Restriction {
                base: D::String,
                facets: vec![Facet::MinLength(low), Facet::MaxLength(high)],
            }),
        ];
        leaf.prop_recursive(3, 16, 3, |inner| {
            prop_oneof![
                inner.clone().prop_map(|r| DataRange::Not(Box::new(r))),
                proptest::collection::vec(inner.clone(), 1..3).prop_map(DataRange::And),
                proptest::collection::vec(inner, 1..3).prop_map(DataRange::Or),
            ]
        })
    }

    /// The two invariants that keep the module usable by a reasoner: a proved-empty
    /// range holds nothing, and a range that holds something is not proved empty.
    #[test]
    fn emptiness_and_membership_never_contradict() {
        let config = Config {
            cases: 1024,
            failure_persistence: None,
            ..Config::default()
        };
        let mut runner =
            TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &SEED));
        let values = probes();
        runner
            .run(&ranges(), |range| {
                let verdict = satisfiability(&range);
                for value in &values {
                    let held = contains(&range, value);
                    if verdict == Satisfiability::Empty {
                        prop_assert_ne!(held, Known::Yes, "{:?} holds {:?}", range, value);
                    }
                    if held == Known::Yes {
                        prop_assert_ne!(
                            verdict,
                            Satisfiability::Empty,
                            "{:?} is empty yet holds {:?}",
                            range,
                            value
                        );
                    }
                }
                Ok(())
            })
            .expect("satisfiability and contains agree on every generated range");
    }
}
