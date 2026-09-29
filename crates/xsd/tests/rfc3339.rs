// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 3339 `date-time`, `full-date` and `full-time`: the §5.8 examples, every
//! field's range with its valid neighbour, the two separator and leap-second
//! policies, and the canonical writer.

use purrdf_hash::mix::splitmix64_next;
use purrdf_xsd::rfc3339::{
    LeapSecond, ParseErrorKind, Separator, format, parse, parse_date, parse_time,
};

/// The GTS profiles' reading: a space may separate date and time, and a leap
/// second falls only at a month end.
fn gts(text: &str) -> Result<(i64, u32), purrdf_xsd::rfc3339::ParseError> {
    parse(text, Separator::GtsSpaceAllowed, LeapSecond::MonthEnd)
}

/// JSON Schema's `date-time` format: `T` or `t` only, a leap second on any day.
fn json_schema(text: &str) -> Result<(i64, u32), purrdf_xsd::rfc3339::ParseError> {
    parse(text, Separator::Rfc3339Abnf, LeapSecond::AnyDay)
}

fn ok(text: &str) -> (i64, u32) {
    gts(text).unwrap_or_else(|err| panic!("{text:?} must parse: {err}"))
}

fn refused(text: &str) -> ParseErrorKind {
    match gts(text) {
        Ok(value) => panic!("{text:?} must be refused, parsed as {value:?}"),
        Err(err) => err.kind(),
    }
}

fn out_of_range(text: &str, field: &str) {
    match refused(text) {
        ParseErrorKind::OutOfRange { field: got, .. } if got == field => {}
        other => panic!("{text:?}: expected {field} out of range, got {other:?}"),
    }
}

#[test]
fn rfc_section_5_8_examples() {
    // 1985-04-12: 5,580 days after the epoch.
    assert_eq!(ok("1985-04-12T23:20:50.52Z"), (482_196_050, 520_000_000));
    // PST: 1996-12-20T00:39:57Z, 9,850 days after the epoch.
    assert_eq!(ok("1996-12-19T16:39:57-08:00"), (851_042_397, 0));
    assert_eq!(ok("1996-12-19T16:39:57-08:00"), ok("1996-12-20T00:39:57Z"));
    // The leap second, in UTC and in Pacific time; it shares 23:59:59's second.
    assert_eq!(ok("1990-12-31T23:59:60Z"), (662_687_999, 0));
    assert_eq!(ok("1990-12-31T15:59:60-08:00"), (662_687_999, 0));
    assert_eq!(ok("1990-12-31T23:59:59Z"), (662_687_999, 0));
    // An offset of twenty minutes: 11:40:27.87 UTC, 12,053 days before the epoch.
    assert_eq!(
        ok("1937-01-01T12:00:27.87+00:20"),
        (-1_041_337_173, 870_000_000)
    );
}

#[test]
fn separator_and_zone_letters() {
    let canonical = ok("2023-11-14T22:13:20Z");
    assert_eq!(canonical, (1_700_000_000, 0));
    assert_eq!(ok("2023-11-14t22:13:20z"), canonical);
    assert_eq!(ok("2023-11-14 22:13:20Z"), canonical);
    assert_eq!(ok("2023-11-14T22:13:20+00:00"), canonical);
    assert_eq!(ok("2023-11-14T22:13:20-00:00"), canonical);
    assert_eq!(ok("2023-11-15T00:13:20+02:00"), canonical);
    assert!(matches!(
        refused("2023-11-14_22:13:20Z"),
        ParseErrorKind::Unexpected { .. }
    ));
    assert!(matches!(
        refused("2023-11-14T22:13:20"),
        ParseErrorKind::EndOfInput { .. }
    ));
    assert!(matches!(
        refused("2023-11-14T22:13:20+0000"),
        ParseErrorKind::Unexpected { .. }
    ));
}

#[test]
fn a_space_separator_is_refused_by_the_abnf_and_accepted_by_the_gts_grammar() {
    assert!(matches!(
        json_schema("2020-01-01 00:00:00Z").map_err(|err| err.kind()),
        Err(ParseErrorKind::Unexpected { found: b' ', .. })
    ));
    assert_eq!(
        gts("2020-01-01 00:00:00Z"),
        json_schema("2020-01-01T00:00:00Z")
    );
}

#[test]
fn both_grammars_accept_lowercase_t_and_z() {
    let canonical = json_schema("2020-01-01T00:00:00Z").expect("canonical");
    assert_eq!(json_schema("2020-01-01t00:00:00z"), Ok(canonical));
    assert_eq!(gts("2020-01-01t00:00:00z"), Ok(canonical));
}

#[test]
fn a_month_end_leap_second_parses_under_both_policies() {
    for text in ["1998-12-31T23:59:60Z", "1998-12-31t15:59:60-08:00"] {
        assert_eq!(gts(text), json_schema(text), "{text}");
        assert!(gts(text).is_ok(), "{text}");
    }
}

#[test]
fn a_mid_month_leap_second_parses_only_on_any_day() {
    let text = "1998-12-15T23:59:60Z";
    assert_eq!(
        gts(text).map_err(|err| err.kind()),
        Err(ParseErrorKind::LeapSecond(LeapSecond::MonthEnd))
    );
    assert_eq!(json_schema(text), json_schema("1998-12-15T23:59:59Z"));
    // Neither policy accepts a leap second away from 23:59 UTC.
    assert_eq!(
        json_schema("1998-12-15T22:59:60Z").map_err(|err| err.kind()),
        Err(ParseErrorKind::LeapSecond(LeapSecond::AnyDay))
    );
}

#[test]
fn fraction_is_truncated_to_nanoseconds() {
    assert_eq!(ok("1970-01-01T00:00:00.1Z"), (0, 100_000_000));
    assert_eq!(ok("1970-01-01T00:00:00.123456789Z"), (0, 123_456_789));
    assert_eq!(ok("1970-01-01T00:00:00.1234567899999Z"), (0, 123_456_789));
    assert_eq!(ok("1970-01-01T00:00:00.9999999999Z"), (0, 999_999_999));
    assert_eq!(ok("1970-01-01T00:00:00.000Z"), (0, 0));
    assert!(matches!(
        refused("1970-01-01T00:00:00.Z"),
        ParseErrorKind::Unexpected { .. }
    ));
    assert!(matches!(
        refused("1970-01-01T00:00:00."),
        ParseErrorKind::EndOfInput { .. }
    ));
}

#[test]
fn doubled_zone_letter_is_refused_single_accepted() {
    assert!(matches!(
        refused("2024-01-01T00:00:00ZZ"),
        ParseErrorKind::Trailing { found: b'Z' }
    ));
    ok("2024-01-01T00:00:00Z");
}

#[test]
fn offset_followed_by_zone_letter_is_refused_offset_alone_accepted() {
    assert!(matches!(
        refused("2024-01-01T00:00:00+00:00Z"),
        ParseErrorKind::Trailing { found: b'Z' }
    ));
    ok("2024-01-01T00:00:00+00:00");
}

#[test]
fn month_13_is_refused_12_accepted() {
    out_of_range("2024-13-01T00:00:00Z", "month");
    ok("2024-12-01T00:00:00Z");
    out_of_range("2024-00-01T00:00:00Z", "month");
    ok("2024-01-01T00:00:00Z");
}

#[test]
fn day_32_is_refused_31_accepted() {
    out_of_range("2024-01-32T00:00:00Z", "day of month");
    ok("2024-01-31T00:00:00Z");
    out_of_range("2024-04-31T00:00:00Z", "day of month");
    ok("2024-04-30T00:00:00Z");
    out_of_range("2024-01-00T00:00:00Z", "day of month");
}

#[test]
fn february_29_needs_a_leap_year() {
    out_of_range("2023-02-29T00:00:00Z", "day of month");
    ok("2024-02-29T00:00:00Z");
    out_of_range("1900-02-29T00:00:00Z", "day of month");
    ok("2000-02-29T00:00:00Z");
}

#[test]
fn hour_24_is_refused_23_accepted() {
    out_of_range("2024-01-01T24:00:00Z", "hour");
    ok("2024-01-01T23:00:00Z");
    out_of_range("2024-01-01T00:60:00Z", "minute");
    ok("2024-01-01T00:59:00Z");
    out_of_range("2024-01-01T00:00:61Z", "second");
}

#[test]
fn leap_second_only_at_utc_month_end() {
    // Not 23:59 UTC.
    assert_eq!(
        refused("1990-12-31T15:59:60Z"),
        ParseErrorKind::LeapSecond(LeapSecond::MonthEnd)
    );
    // 23:59:60 local but 22:59:60 UTC.
    assert_eq!(
        refused("1990-12-31T23:59:60+01:00"),
        ParseErrorKind::LeapSecond(LeapSecond::MonthEnd)
    );
    // 23:59:60 UTC, but not the last day of the month.
    assert_eq!(
        refused("1990-12-30T23:59:60Z"),
        ParseErrorKind::LeapSecond(LeapSecond::MonthEnd)
    );
    assert_eq!(
        refused("2024-02-28T23:59:60Z"),
        ParseErrorKind::LeapSecond(LeapSecond::MonthEnd)
    );
    // The RFC's valid example, and other month ends in UTC.
    ok("1990-12-31T15:59:60-08:00");
    ok("2024-02-29T23:59:60Z");
    ok("1997-06-30T23:59:60.5Z");
    // Local date differs from the UTC date that makes it valid.
    ok("1998-01-01T00:59:60+01:00");
    let err = gts("1990-12-31T15:59:60Z").expect_err("refused above");
    assert_eq!(
        err.at(),
        17,
        "the leap-second error blames the seconds field"
    );
}

#[test]
fn offset_minute_60_is_refused_59_accepted() {
    out_of_range("2024-01-01T00:00:00+00:60", "offset minute");
    assert_eq!(ok("2024-01-01T00:00:00+00:59"), ok("2023-12-31T23:01:00Z"));
    out_of_range("2024-01-01T00:00:00+24:00", "offset hour");
    assert_eq!(ok("2024-01-01T00:00:00-23:59"), ok("2024-01-01T23:59:00Z"));
}

#[test]
fn five_digit_year_is_refused_four_accepted() {
    assert!(matches!(
        refused("10000-01-01T00:00:00Z"),
        ParseErrorKind::Unexpected { found: b'0', .. }
    ));
    ok("9999-12-31T23:59:59Z");
    ok("0000-01-01T00:00:00Z");
    assert!(matches!(
        refused("999-01-01T00:00:00Z"),
        ParseErrorKind::Unexpected { .. }
    ));
}

#[test]
fn full_date_needs_a_real_calendar_day() {
    assert!(parse_date("2021-02-29").is_err());
    assert_eq!(parse_date("2020-02-29"), Ok((2020, 2, 29)));
    assert!(parse_date("2020-02-29T").is_err());
    assert!(parse_date("20-02-29").is_err());
}

#[test]
fn full_time_needs_a_zone_and_a_utc_23_59_leap_second() {
    assert!(parse_time("08:30:06").is_err());
    assert_eq!(parse_time("08:30:06Z"), Ok((30_606, 0)));
    assert!(parse_time("22:59:60Z").is_err());
    assert_eq!(parse_time("23:59:60Z"), Ok((86_399, 0)));
    assert!(parse_time("15:59:60-08:01").is_err());
    assert_eq!(parse_time("15:59:60-08:00"), Ok((86_399, 0)));
    assert_eq!(parse_time("00:30:00.5+01:00"), Ok((84_600, 500_000_000)));
}

#[test]
fn error_messages_name_the_position_and_problem() {
    let text = gts("2024-13-01T00:00:00Z")
        .expect_err("month 13")
        .to_string();
    assert_eq!(
        text,
        "invalid RFC 3339 date-time at byte 5: month 13 is outside 1..=12"
    );
    let text = gts("2024-01-01T00:00:00").expect_err("no zone").to_string();
    assert!(
        text.contains("byte 19") && text.contains("end of input"),
        "{text}"
    );
    let text = gts("2024-01-01T00:00:00Z\n")
        .expect_err("trailing")
        .to_string();
    assert!(text.contains("byte 0x0a"), "{text}");
}

#[test]
fn format_writes_canonical_utc() {
    assert_eq!(format(0, 0).as_deref(), Ok("1970-01-01T00:00:00Z"));
    assert_eq!(
        format(1_700_000_000, 0).as_deref(),
        Ok("2023-11-14T22:13:20Z")
    );
    assert_eq!(
        format(482_196_050, 520_000_000).as_deref(),
        Ok("1985-04-12T23:20:50.52Z")
    );
    assert_eq!(
        format(-1, 1).as_deref(),
        Ok("1969-12-31T23:59:59.000000001Z")
    );
    assert_eq!(
        format(-1_041_337_173, 870_000_000).as_deref(),
        Ok("1937-01-01T11:40:27.87Z")
    );
    assert_eq!(
        format(951_782_400, 0).as_deref(),
        Ok("2000-02-29T00:00:00Z")
    );
}

#[test]
fn format_refuses_outside_four_digit_years() {
    let first = ok("0000-01-01T00:00:00Z").0;
    let last = ok("9999-12-31T23:59:59Z").0;
    assert_eq!(format(first, 0).as_deref(), Ok("0000-01-01T00:00:00Z"));
    assert!(format(first - 1, 999_999_999).is_err());
    assert_eq!(
        format(last, 999_999_999).as_deref(),
        Ok("9999-12-31T23:59:59.999999999Z")
    );
    assert!(format(last + 1, 0).is_err());
    assert!(format(i64::MIN, 0).is_err());
    assert!(format(i64::MAX, 0).is_err());
    assert!(format(0, 1_000_000_000).is_err());
    assert!(format(0, 999_999_999).is_ok());
}

#[test]
fn parse_accepts_instants_that_normalize_past_four_digit_years() {
    // Valid text; the UTC instant lies in year 10000 or year -1.
    let late = ok("9999-12-31T23:59:59-01:00").0;
    assert!(format(late, 0).is_err());
    let early = ok("0000-01-01T00:00:00+01:00").0;
    assert!(format(early, 0).is_err());
}

#[test]
fn format_then_parse_round_trips_seeded_instants() {
    let first = ok("0000-01-01T00:00:00Z").0;
    let last = ok("9999-12-31T23:59:59Z").0;
    let span = u64::try_from(last - first + 1).expect("positive span");
    let mut state = 0x5075_7252_4446_3339_u64;
    let mut below = |bound: u64| splitmix64_next(&mut state) % bound;
    for round in 0..20_000 {
        let seconds = first + i64::try_from(below(span)).expect("span fits i64");
        let nanos = match round % 4 {
            0 => 0,
            1 => u32::try_from(below(1000)).expect("small") * 1_000_000,
            _ => u32::try_from(below(1_000_000_000)).expect("small"),
        };
        let text = format(seconds, nanos).expect("in range");
        assert_eq!(gts(&text), Ok((seconds, nanos)), "{text}");
        assert_eq!(json_schema(&text), Ok((seconds, nanos)), "{text}");
    }
}

#[test]
fn parse_then_format_is_canonical() {
    for (input, canonical) in [
        ("1996-12-19T16:39:57-08:00", "1996-12-20T00:39:57Z"),
        ("1985-04-12t23:20:50.520z", "1985-04-12T23:20:50.52Z"),
        (
            "2024-02-29 12:00:00.000000000+05:30",
            "2024-02-29T06:30:00Z",
        ),
        ("1990-12-31T15:59:60.25-08:00", "1990-12-31T23:59:59.25Z"),
    ] {
        let (seconds, nanos) = ok(input);
        assert_eq!(format(seconds, nanos).as_deref(), Ok(canonical), "{input}");
    }
}
