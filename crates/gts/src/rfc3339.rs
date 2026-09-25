// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 3339 timestamps for the files and tar profiles' `modified` values.
//!
//! The grammar is RFC 3339 §5.6 `date-time`, read exactly:
//!
//! * a four-digit year (`0000..=9999`), two-digit month and day, validated
//!   against the proleptic Gregorian calendar;
//! * any single separator byte between date and time — §5.6's note permits a
//!   reader to accept e.g. a space in place of `T`;
//! * two-digit hour (`..=23`), minute (`..=59`) and second (`..=59`, or `60`
//!   for a leap second);
//! * an optional fraction of at least one digit, truncated to nanoseconds;
//! * an offset of `Z`, `z` or `±hh:mm` (hour `..=23`, minute `..=59`) — or
//!   **no offset at all**, read as UTC, because a profile `modified` value
//!   written without one has always meant UTC.
//!
//! A leap second is honoured only where one can occur, the last second of a
//! month in UTC, and reads as the nanosecond before the next second, the
//! instant a POSIX timestamp can represent.
//!
//! The written form is canonical and the reader accepts it: `Z`, a `T`
//! separator, and a fraction only when there is one, trailing zeros dropped.
//! Calendar arithmetic is `purrdf-xsd`'s, so this module owns only the
//! grammar.

/// `0000-01-01T00:00:00Z`, the earliest instant with a four-digit year.
const MIN_UNIX_SECONDS: i64 = -62_167_219_200;
/// `9999-12-31T23:59:59Z`, the latest instant with a four-digit year.
const MAX_UNIX_SECONDS: i64 = 253_402_300_799;
const SECONDS_PER_DAY: i128 = 86_400;
const NANOS_PER_SECOND: u32 = 1_000_000_000;

/// Write `seconds` past the Unix epoch plus `nanos` as a canonical UTC
/// timestamp: `YYYY-MM-DDTHH:MM:SS[.f]Z`.
///
/// # Errors
///
/// The instant lies outside the four-digit years, or `nanos` is a whole second
/// or more.
pub(crate) fn format(seconds: i64, nanos: u32) -> Result<String, &'static str> {
    if !(MIN_UNIX_SECONDS..=MAX_UNIX_SECONDS).contains(&seconds) {
        return Err("year outside 0000..=9999");
    }
    if nanos >= NANOS_PER_SECOND {
        return Err("nanoseconds are a whole second or more");
    }
    let canonical = purrdf_xsd::datetime_from_unix_seconds(seconds).canonical_lexical();
    if nanos == 0 {
        return Ok(canonical);
    }
    let whole = canonical
        .strip_suffix('Z')
        .ok_or("xsd:dateTime from Unix seconds is not in UTC")?;
    let fraction = format!("{nanos:09}");
    Ok(format!("{whole}.{}Z", fraction.trim_end_matches('0')))
}

/// Read a timestamp as `(seconds past the Unix epoch, nanoseconds)`.
///
/// # Errors
///
/// A static description of the first component that does not fit the grammar
/// in the module documentation.
pub(crate) fn parse(text: &str) -> Result<(i64, u32), &'static str> {
    let mut cursor = Cursor {
        bytes: text.as_bytes(),
        at: 0,
    };
    let year = cursor.digits(4).ok_or("year")?;
    cursor.literal(b'-')?;
    let month = cursor.digits(2).ok_or("month")?;
    cursor.literal(b'-')?;
    let day = cursor.digits(2).ok_or("day")?;
    cursor.skip_separator()?;
    let hour = cursor.digits(2).ok_or("hour")?;
    cursor.literal(b':')?;
    let minute = cursor.digits(2).ok_or("minute")?;
    cursor.literal(b':')?;
    let second = cursor.digits(2).ok_or("second")?;
    let mut nanos = cursor.fraction()?;
    let offset_seconds = cursor.offset()?;
    if cursor.at != cursor.bytes.len() {
        return Err("trailing characters");
    }

    let year = i64::from(year);
    let month = u8::try_from(month).map_err(|_| "month")?;
    let day = u8::try_from(day).map_err(|_| "day")?;
    if !(1..=12).contains(&month) {
        return Err("month");
    }
    if day == 0 || day > purrdf_xsd::days_in_month(year, month) {
        return Err("day");
    }
    if hour > 23 {
        return Err("hour");
    }
    if minute > 59 {
        return Err("minute");
    }
    let leap = second == 60;
    let second = if leap {
        nanos = NANOS_PER_SECOND - 1;
        59
    } else if second > 59 {
        return Err("second");
    } else {
        second
    };

    let local = purrdf_xsd::days_from_civil(year, month, day) * SECONDS_PER_DAY
        + i128::from(hour * 3600 + minute * 60 + second);
    let utc = i64::try_from(local - i128::from(offset_seconds)).map_err(|_| "out of range")?;
    if leap {
        let at = purrdf_xsd::datetime_from_unix_seconds(utc);
        let last_second_of_month = at.hour() == 23
            && at.minute() == 59
            && at.second().whole_part() == 59
            && at.day() == purrdf_xsd::days_in_month(at.year(), at.month());
        if !last_second_of_month {
            return Err("leap second outside the last second of a UTC month");
        }
    }
    Ok((utc, nanos))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    /// Exactly `count` ASCII digits.
    fn digits(&mut self, count: usize) -> Option<u32> {
        let run = self.bytes.get(self.at..self.at + count)?;
        let mut value = 0_u32;
        for &byte in run {
            if !byte.is_ascii_digit() {
                return None;
            }
            value = value * 10 + u32::from(byte - b'0');
        }
        self.at += count;
        Some(value)
    }

    fn literal(&mut self, expected: u8) -> Result<(), &'static str> {
        if self.bytes.get(self.at) == Some(&expected) {
            self.at += 1;
            Ok(())
        } else {
            Err("expected separator")
        }
    }

    fn skip_separator(&mut self) -> Result<(), &'static str> {
        if self.at < self.bytes.len() {
            self.at += 1;
            Ok(())
        } else {
            Err("date-time separator")
        }
    }

    /// `.` followed by one or more digits; digits past the ninth are read and
    /// dropped.
    fn fraction(&mut self) -> Result<u32, &'static str> {
        if self.bytes.get(self.at) != Some(&b'.') {
            return Ok(0);
        }
        self.at += 1;
        let mut nanos = 0_u32;
        let mut scale = NANOS_PER_SECOND;
        let start = self.at;
        while let Some(&byte) = self.bytes.get(self.at) {
            if !byte.is_ascii_digit() {
                break;
            }
            scale /= 10;
            nanos += u32::from(byte - b'0') * scale;
            self.at += 1;
        }
        if self.at == start {
            return Err("fraction");
        }
        Ok(nanos)
    }

    /// The offset east of UTC, in seconds; absent reads as UTC.
    fn offset(&mut self) -> Result<i64, &'static str> {
        let sign = match self.bytes.get(self.at) {
            None => return Ok(0),
            Some(b'Z' | b'z') => {
                self.at += 1;
                return Ok(0);
            }
            Some(b'+') => 1,
            Some(b'-') => -1,
            Some(_) => return Err("offset"),
        };
        self.at += 1;
        let hours = self.digits(2).filter(|&h| h <= 23).ok_or("offset hour")?;
        self.literal(b':')?;
        let minutes = self.digits(2).filter(|&m| m <= 59).ok_or("offset minute")?;
        Ok(sign * i64::from(hours * 3600 + minutes * 60))
    }
}

#[cfg(test)]
mod tests {
    use super::{format, parse};

    #[test]
    fn the_canonical_form_round_trips() {
        for (seconds, nanos, text) in [
            (0, 0, "1970-01-01T00:00:00Z"),
            (1_700_000_000, 0, "2023-11-14T22:13:20Z"),
            (1_700_000_000, 500_000_000, "2023-11-14T22:13:20.5Z"),
            (1_700_000_000, 1, "2023-11-14T22:13:20.000000001Z"),
            (-62_167_219_200, 0, "0000-01-01T00:00:00Z"),
            (
                253_402_300_799,
                999_999_999,
                "9999-12-31T23:59:59.999999999Z",
            ),
        ] {
            assert_eq!(format(seconds, nanos).as_deref(), Ok(text));
            assert_eq!(parse(text), Ok((seconds, nanos)));
        }
    }

    #[test]
    fn instants_outside_four_digit_years_are_not_written() {
        assert!(format(253_402_300_800, 0).is_err());
        assert!(format(-62_167_219_201, 0).is_err());
        assert!(format(0, 1_000_000_000).is_err());
    }

    #[test]
    fn the_spellings_rfc_3339_permits_all_read_as_one_instant() {
        let utc = Ok((1_700_000_000, 0));
        for text in [
            "2023-11-14T22:13:20Z",
            "2023-11-14t22:13:20z",
            "2023-11-14 22:13:20Z",
            "2023-11-14T22:13:20",
            "2023-11-14T22:13:20+00:00",
            "2023-11-14T22:13:20-00:00",
            "2023-11-15T00:13:20+02:00",
            "2023-11-14T17:43:20-04:30",
            "2023-11-14T22:13:20.0Z",
        ] {
            assert_eq!(parse(text), utc, "{text}");
        }
    }

    #[test]
    fn a_fraction_is_truncated_to_nanoseconds() {
        assert_eq!(
            parse("2023-11-14T22:13:20.1234567899999Z"),
            Ok((1_700_000_000, 123_456_789))
        );
        assert_eq!(parse("2023-11-14T22:13:20.Z"), Err("fraction"));
    }

    #[test]
    fn a_leap_second_is_honoured_only_at_the_end_of_a_utc_month() {
        // 2016-12-31T23:59:60Z was a real leap second.
        assert_eq!(
            parse("2016-12-31T23:59:60Z"),
            Ok((1_483_228_799, 999_999_999))
        );
        // The same instant written in another offset is still the end of the month in UTC.
        assert_eq!(
            parse("2016-12-31T18:59:60-05:00"),
            Ok((1_483_228_799, 999_999_999))
        );
        assert!(parse("2016-12-30T23:59:60Z").is_err());
        assert!(parse("2016-12-31T23:58:60Z").is_err());
        assert!(parse("2016-12-31T23:59:61Z").is_err());
    }

    #[test]
    fn calendar_and_clock_fields_are_range_checked() {
        assert!(parse("2024-02-29T00:00:00Z").is_ok());
        for text in [
            "2023-02-29T00:00:00Z",
            "2023-00-10T00:00:00Z",
            "2023-13-10T00:00:00Z",
            "2023-04-31T00:00:00Z",
            "2023-04-00T00:00:00Z",
            "2023-04-30T24:00:00Z",
            "2023-04-30T23:60:00Z",
            "2023-04-30T23:59:59+24:00",
            "2023-04-30T23:59:59+23:60",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
        assert!(parse("2023-04-30T23:59:59+23:59").is_ok());
    }

    #[test]
    fn malformed_spellings_are_refused() {
        for text in [
            "",
            "12023-11-14T22:13:20Z",
            "-2023-11-14T22:13:20Z",
            "2023-11-14",
            "2023-11-14T",
            "2023-11-14T22:13Z",
            "2023-11-14T22:13:20ZZ",
            "2023-11-14T22:13:20+00:00Z",
            "2023-11-14T22:13:20+0000",
            "2023-11-14T22:13:20 ",
            "2023-11-14\u{e9}22:13:20Z",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
    }
}
