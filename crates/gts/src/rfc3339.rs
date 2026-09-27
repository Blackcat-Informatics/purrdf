// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 3339 date-time text for the files and tar profiles' `modified` stamps.
//!
//! [`parse`] reads the `date-time` production of RFC 3339 §5.6 and answers the
//! instant as Unix seconds and nanoseconds in UTC. [`format`] writes the one
//! canonical UTC spelling the profiles emit, `YYYY-MM-DDTHH:MM:SS[.f]Z`.
//!
//! What `parse` accepts:
//!
//! * `full-date` and `full-time` separated by `T`, `t` or a single space (the
//!   §5.6 note permits lower case and, for readability, a space);
//! * a zone of `Z`, `z` or a numeric `+hh:mm` / `-hh:mm` offset — a timestamp
//!   with no zone is not RFC 3339 and is refused. `-00:00` (§4.3) and `+00:00`
//!   both denote UTC;
//! * a four-digit year, and the per-field ranges of §5.6 and §5.7: month 01-12,
//!   a day that exists in that month of the proleptic Gregorian calendar, hour
//!   00-23, minute 00-59, second 00-60, offset hour 00-23, offset minute 00-59;
//! * a fraction of any length, truncated (never rounded) to nanoseconds;
//! * second 60 only when the instant, normalized to UTC, is 23:59:60 on the last
//!   day of a month (§5.7, Appendix D). Unix seconds have no 61st second, so a
//!   leap second answers the Unix second of 23:59:59 with its fraction kept.
//!
//! The module runs once per archive entry, so it is plain scalar code.

use std::fmt;

use purrdf_xsd::{days_from_civil, days_in_month};

const SECONDS_PER_DAY: i64 = 86_400;
const NANOS_PER_SECOND: u32 = 1_000_000_000;
const FRACTION_DIGITS: u32 = 9;
/// The first year `format` can write (§5.6 `date-fullyear = 4DIGIT`).
const FIRST_YEAR: i64 = 0;
/// The year after the last one `format` can write.
const YEAR_AFTER_LAST: i64 = 10_000;

/// Why a text is not an RFC 3339 `date-time`, and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParseError {
    at: usize,
    kind: ParseErrorKind,
}

impl ParseError {
    /// The byte offset in the input at which the text stops being acceptable.
    #[cfg(test)]
    const fn at(&self) -> usize {
        self.at
    }

    /// What went wrong at [`Self::at`].
    #[cfg(test)]
    const fn kind(&self) -> ParseErrorKind {
        self.kind
    }
}

/// The class of a [`ParseError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseErrorKind {
    /// The input ended where the grammar needs `expected`.
    EndOfInput {
        /// What the grammar needed next.
        expected: &'static str,
    },
    /// A byte other than `expected` appeared.
    Unexpected {
        /// What the grammar needed next.
        expected: &'static str,
        /// The byte that appeared instead.
        found: u8,
    },
    /// A well-formed numeric field holds a value outside its range.
    OutOfRange {
        /// The field's name.
        field: &'static str,
        /// The value read.
        value: u32,
        /// The smallest permitted value.
        min: u32,
        /// The largest permitted value.
        max: u32,
    },
    /// Second 60 somewhere other than 23:59:60 UTC on the last day of a month.
    LeapSecond,
    /// Bytes follow a complete `date-time`.
    Trailing {
        /// The first byte after the zone.
        found: u8,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid RFC 3339 date-time at byte {}: ", self.at)?;
        match self.kind {
            ParseErrorKind::EndOfInput { expected } => {
                write!(f, "expected {expected}, found end of input")
            }
            ParseErrorKind::Unexpected { expected, found } => {
                write!(f, "expected {expected}, found {}", DisplayByte(found))
            }
            ParseErrorKind::OutOfRange { field, value, min, max } => {
                write!(f, "{field} {value} is outside {min}..={max}")
            }
            ParseErrorKind::LeapSecond => f.write_str(
                "second 60 is a leap second, allowed only at 23:59:60 UTC on the last day of a month",
            ),
            ParseErrorKind::Trailing { found } => {
                write!(f, "unexpected {} after the time zone", DisplayByte(found))
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Renders one input byte readably: printable ASCII quoted, anything else in hex.
struct DisplayByte(u8);

impl fmt::Display for DisplayByte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_ascii_graphic() || self.0 == b' ' {
            write!(f, "'{}'", char::from(self.0))
        } else {
            write!(f, "byte 0x{:02x}", self.0)
        }
    }
}

/// A forward-only reader over the input bytes.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    const fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError { at: self.at, kind }
    }

    fn mismatch(&self, expected: &'static str) -> ParseError {
        match self.peek() {
            Some(found) => self.error(ParseErrorKind::Unexpected { expected, found }),
            None => self.error(ParseErrorKind::EndOfInput { expected }),
        }
    }

    /// Consumes one byte from `allowed` and answers it.
    fn one_of(&mut self, allowed: &[u8], expected: &'static str) -> Result<u8, ParseError> {
        match self.peek() {
            Some(byte) if allowed.contains(&byte) => {
                self.at += 1;
                Ok(byte)
            }
            _ => Err(self.mismatch(expected)),
        }
    }

    /// Consumes exactly `count` ASCII digits and answers their value, then
    /// checks it against `min..=max`, blaming the field's first byte.
    fn field(
        &mut self,
        count: usize,
        expected: &'static str,
        field: &'static str,
        min: u32,
        max: u32,
    ) -> Result<u32, ParseError> {
        let start = self.at;
        let mut value = 0_u32;
        for _ in 0..count {
            match self.peek() {
                Some(byte) if byte.is_ascii_digit() => {
                    value = value * 10 + u32::from(byte - b'0');
                    self.at += 1;
                }
                _ => return Err(self.mismatch(expected)),
            }
        }
        if (min..=max).contains(&value) {
            Ok(value)
        } else {
            Err(ParseError {
                at: start,
                kind: ParseErrorKind::OutOfRange {
                    field,
                    value,
                    min,
                    max,
                },
            })
        }
    }

    /// Consumes `1*DIGIT` of a `time-secfrac` (the `.` already consumed) and
    /// answers its first nine digits as nanoseconds; later digits are dropped.
    fn fraction(&mut self) -> Result<u32, ParseError> {
        let start = self.at;
        let mut nanos = 0_u32;
        let mut kept = 0_u32;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }
            if kept < FRACTION_DIGITS {
                nanos = nanos * 10 + u32::from(byte - b'0');
                kept += 1;
            }
            self.at += 1;
        }
        if self.at == start {
            return Err(self.mismatch("a digit after '.'"));
        }
        Ok(nanos * 10_u32.pow(FRACTION_DIGITS - kept))
    }
}

/// Parses an RFC 3339 `date-time` into Unix seconds and nanoseconds in UTC.
pub(crate) fn parse(text: &str) -> Result<(i64, u32), ParseError> {
    let mut cursor = Cursor::new(text.as_bytes());

    let year = cursor.field(4, "a four-digit year", "year", 0, 9999)?;
    cursor.one_of(b"-", "'-' after the four-digit year")?;
    let month = cursor.field(2, "a two-digit month", "month", 1, 12)?;
    cursor.one_of(b"-", "'-' after the month")?;
    let year = i64::from(year);
    let month = month as u8;
    let last_day = u32::from(days_in_month(year, month));
    let day = cursor.field(2, "a two-digit day", "day of month", 1, last_day)? as u8;
    cursor.one_of(b"Tt ", "'T', 't' or ' ' between date and time")?;
    let hour = cursor.field(2, "a two-digit hour", "hour", 0, 23)?;
    cursor.one_of(b":", "':' after the hour")?;
    let minute = cursor.field(2, "a two-digit minute", "minute", 0, 59)?;
    cursor.one_of(b":", "':' after the minute")?;
    let second_at = cursor.at;
    let second = cursor.field(2, "a two-digit second", "second", 0, 60)?;
    let nanos = if cursor.peek() == Some(b'.') {
        cursor.at += 1;
        cursor.fraction()?
    } else {
        0
    };

    let offset_seconds = match cursor.one_of(b"Zz+-", "'Z', 'z', '+' or '-' for the time zone")? {
        b'Z' | b'z' => 0,
        sign => {
            let hours = cursor.field(2, "a two-digit offset hour", "offset hour", 0, 23)?;
            cursor.one_of(b":", "':' in the offset")?;
            let minutes = cursor.field(2, "a two-digit offset minute", "offset minute", 0, 59)?;
            let magnitude = i64::from(hours * 3600 + minutes * 60);
            if sign == b'-' { -magnitude } else { magnitude }
        }
    };
    if let Some(found) = cursor.peek() {
        return Err(cursor.error(ParseErrorKind::Trailing { found }));
    }

    // A leap second counts as the 23:59:59 Unix second; §5.7 is checked in UTC.
    let second_of_day = i64::from(hour * 3600 + minute * 60 + second.min(59));
    let seconds = day_number(year, month, day) * SECONDS_PER_DAY + second_of_day - offset_seconds;
    if second == 60 && !is_utc_leap_second_slot(seconds) {
        return Err(ParseError {
            at: second_at,
            kind: ParseErrorKind::LeapSecond,
        });
    }
    Ok((seconds, nanos))
}

/// Whether `seconds` (the Unix second a `:60` was mapped onto) is 23:59:59 UTC
/// on the last day of a month, i.e. whether a leap second may follow it.
fn is_utc_leap_second_slot(seconds: i64) -> bool {
    if seconds.rem_euclid(SECONDS_PER_DAY) != SECONDS_PER_DAY - 1 {
        return false;
    }
    let (year, month, day) = civil_from_day_number(seconds.div_euclid(SECONDS_PER_DAY));
    day == days_in_month(year, month)
}

/// Writes `seconds` + `nanos` since the Unix epoch as canonical RFC 3339 UTC,
/// `YYYY-MM-DDTHH:MM:SS[.f]Z`. The fraction is omitted when `nanos` is zero and
/// otherwise carries the nanoseconds without trailing zeros.
pub(crate) fn format(seconds: i64, nanos: u32) -> Result<String, &'static str> {
    if nanos >= NANOS_PER_SECOND {
        return Err("nanoseconds must be less than one second");
    }
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    if days < day_number(FIRST_YEAR, 1, 1) || days >= day_number(YEAR_AFTER_LAST, 1, 1) {
        return Err("instant lies outside the four-digit years 0000..=9999");
    }
    let second_of_day = seconds.rem_euclid(SECONDS_PER_DAY) as u32;
    let (year, month, day) = civil_from_day_number(days);

    let mut out = String::with_capacity(30);
    push_digits(&mut out, year as u32, 4);
    out.push('-');
    push_digits(&mut out, u32::from(month), 2);
    out.push('-');
    push_digits(&mut out, u32::from(day), 2);
    out.push('T');
    push_digits(&mut out, second_of_day / 3600, 2);
    out.push(':');
    push_digits(&mut out, second_of_day / 60 % 60, 2);
    out.push(':');
    push_digits(&mut out, second_of_day % 60, 2);
    if nanos != 0 {
        let mut fraction = nanos;
        let mut width = FRACTION_DIGITS;
        while fraction.is_multiple_of(10) {
            fraction /= 10;
            width -= 1;
        }
        out.push('.');
        push_digits(&mut out, fraction, width);
    }
    out.push('Z');
    Ok(out)
}

/// Appends `value` as exactly `width` decimal digits, zero-padded on the left.
/// `value` must fit in `width` digits.
fn push_digits(out: &mut String, value: u32, width: u32) {
    let mut divisor = 10_u32.pow(width - 1);
    while divisor > 0 {
        out.push(char::from(b'0' + (value / divisor % 10) as u8));
        divisor /= 10;
    }
}

/// Days from 1970-01-01 to the given proleptic-Gregorian date.
fn day_number(year: i64, month: u8, day: u8) -> i64 {
    i64::try_from(days_from_civil(year, month, day))
        .expect("every year this module reaches is within a few thousand of 1970")
}

/// The proleptic-Gregorian date `days` after 1970-01-01.
fn civil_from_day_number(days: i64) -> (i64, u8, u8) {
    // Estimate the year from the mean Gregorian year (146,097 days per 400
    // years); the estimate is off by at most one, which the loops correct.
    let mut year = 1970 + (days * 400).div_euclid(146_097);
    while day_number(year, 1, 1) > days {
        year -= 1;
    }
    while day_number(year + 1, 1, 1) <= days {
        year += 1;
    }
    let mut remaining = days - day_number(year, 1, 1);
    let mut month = 1_u8;
    loop {
        let length = i64::from(days_in_month(year, month));
        if remaining < length {
            break;
        }
        remaining -= length;
        month += 1;
    }
    (year, month, remaining as u8 + 1)
}

#[cfg(test)]
mod tests {
    use super::{ParseErrorKind, format, parse};

    fn ok(text: &str) -> (i64, u32) {
        parse(text).unwrap_or_else(|err| panic!("{text:?} must parse: {err}"))
    }

    fn refused(text: &str) -> ParseErrorKind {
        match parse(text) {
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
        assert_eq!(refused("1990-12-31T15:59:60Z"), ParseErrorKind::LeapSecond);
        // 23:59:60 local but 22:59:60 UTC.
        assert_eq!(
            refused("1990-12-31T23:59:60+01:00"),
            ParseErrorKind::LeapSecond
        );
        // 23:59:60 UTC, but not the last day of the month.
        assert_eq!(refused("1990-12-30T23:59:60Z"), ParseErrorKind::LeapSecond);
        assert_eq!(refused("2024-02-28T23:59:60Z"), ParseErrorKind::LeapSecond);
        // The RFC's valid example, and other month ends in UTC.
        ok("1990-12-31T15:59:60-08:00");
        ok("2024-02-29T23:59:60Z");
        ok("1997-06-30T23:59:60.5Z");
        // Local date differs from the UTC date that makes it valid.
        ok("1998-01-01T00:59:60+01:00");
        let err = parse("1990-12-31T15:59:60Z").expect_err("refused above");
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
    fn error_messages_name_the_position_and_problem() {
        let text = parse("2024-13-01T00:00:00Z")
            .expect_err("month 13")
            .to_string();
        assert_eq!(
            text,
            "invalid RFC 3339 date-time at byte 5: month 13 is outside 1..=12"
        );
        let text = parse("2024-01-01T00:00:00")
            .expect_err("no zone")
            .to_string();
        assert!(
            text.contains("byte 19") && text.contains("end of input"),
            "{text}"
        );
        let text = parse("2024-01-01T00:00:00Z\n")
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

    /// SplitMix64: a fixed-seed generator so the round trip is reproducible.
    struct SplitMix64(u64);

    impl SplitMix64 {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        fn below(&mut self, bound: u64) -> u64 {
            self.next() % bound
        }
    }

    #[test]
    fn format_then_parse_round_trips_seeded_instants() {
        let first = ok("0000-01-01T00:00:00Z").0;
        let last = ok("9999-12-31T23:59:59Z").0;
        let span = u64::try_from(last - first + 1).expect("positive span");
        let mut rng = SplitMix64(0x5075_7252_4446_3339);
        for round in 0..20_000 {
            let seconds = first + i64::try_from(rng.below(span)).expect("span fits i64");
            let nanos = match round % 4 {
                0 => 0,
                1 => (rng.below(1000) as u32) * 1_000_000,
                _ => rng.below(1_000_000_000) as u32,
            };
            let text = format(seconds, nanos).expect("in range");
            assert_eq!(parse(&text), Ok((seconds, nanos)), "{text}");
            // Canonical: re-formatting the parse is byte-identical.
            assert_eq!(format(seconds, nanos).as_deref(), Ok(text.as_str()));
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
}
