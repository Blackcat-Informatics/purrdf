// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 3339 internet date and time text: the `date-time`, `full-date` and
//! `full-time` productions of RFC 3339 §5.6, and the canonical UTC writer.
//!
//! This is the workspace's one reading of RFC 3339. [`parse`] reads a
//! `date-time` and answers the instant as Unix seconds and nanoseconds in UTC;
//! [`parse_date`] and [`parse_time`] read the two halves on their own (JSON
//! Schema's `date` and `time` formats); [`format`] writes the one canonical UTC
//! spelling, `YYYY-MM-DDTHH:MM:SS[.f]Z`.
//!
//! What the readers accept:
//!
//! * a zone of `Z`, `z` or a numeric `+hh:mm` / `-hh:mm` offset — a time with
//!   no zone is not RFC 3339 and is refused. `-00:00` (§4.3) and `+00:00` both
//!   denote UTC;
//! * a four-digit year, and the per-field ranges of §5.6 and §5.7: month 01-12,
//!   a day that exists in that month of the proleptic Gregorian calendar, hour
//!   00-23, minute 00-59, second 00-60, offset hour 00-23, offset minute 00-59;
//! * a fraction of any length, truncated (never rounded) to nanoseconds;
//! * second 60 only at 23:59:60 once normalized to UTC — and, under
//!   [`LeapSecond::MonthEnd`], only on the last day of a month (§5.7,
//!   Appendix D). Unix seconds have no 61st second, so a leap second answers
//!   the Unix second of 23:59:59 with its fraction kept.
//!
//! Two behaviours are parameters, because the grammars that cite RFC 3339 differ
//! on them:
//!
//! * [`Separator`]: §5.6's ABNF separates date and time with `T` (or `t`, since
//!   ABNF strings are case-insensitive); its note also permits a space "for the
//!   sake of readability", which the GTS files and tar profiles' `modified`
//!   stamps accept and JSON Schema's `date-time` format does not;
//! * [`LeapSecond`]: Appendix D's table puts every leap second at a month end,
//!   which the GTS profiles check; JSON Schema's `date-time` and `time` formats
//!   check the time of day alone.
//!
//! `xsd:dateTime` is not this grammar — its year may be negative or wider than
//! four digits and its timezone is optional — and [`crate::temporal`] reads it;
//! the two share the calendar ([`crate::temporal::days_in_month`],
//! [`crate::temporal::days_from_civil`], [`crate::temporal::civil_from_days`]).
//!
//! The readers are plain byte-at-a-time scalar code: every consumer makes one
//! call per stamp or per validated string, and the bench
//! `purrdf-gts:rfc3339` watches the cost.

use core::fmt;

use crate::temporal::{civil_from_days, days_from_civil, days_in_month};

const SECONDS_PER_DAY: i64 = 86_400;
const NANOS_PER_SECOND: u32 = 1_000_000_000;
const FRACTION_DIGITS: u32 = 9;
/// The first year `format` can write (§5.6 `date-fullyear = 4DIGIT`).
const FIRST_YEAR: i64 = 0;
/// The year after the last one `format` can write.
const YEAR_AFTER_LAST: i64 = 10_000;

/// Which bytes may separate `full-date` from `full-time` in a `date-time`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    /// §5.6's ABNF: `T`, or `t` (ABNF strings are case-insensitive). JSON
    /// Schema's `date-time` format.
    Rfc3339Abnf,
    /// `T`, `t` or a single space, which §5.6's note permits for readability.
    /// The GTS files and tar profiles' `modified` stamps.
    GtsSpaceAllowed,
}

/// Where second 60 — a leap second — may fall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeapSecond {
    /// Only at 23:59:60 UTC on the last day of a month, where Appendix D puts
    /// every leap second. The GTS profiles' `modified` stamps.
    MonthEnd,
    /// At 23:59:60 UTC on any day. JSON Schema's `date-time` and `time`
    /// formats, whose test suite accepts a leap second on a mid-month day.
    AnyDay,
}

/// Why a text is not RFC 3339, and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    at: usize,
    kind: ParseErrorKind,
}

impl ParseError {
    /// The byte offset in the input at which the text stops being acceptable.
    #[must_use]
    pub const fn at(&self) -> usize {
        self.at
    }

    /// What went wrong at [`Self::at`].
    #[must_use]
    pub const fn kind(&self) -> ParseErrorKind {
        self.kind
    }
}

/// The class of a [`ParseError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
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
    /// Second 60 where the policy allows no leap second.
    LeapSecond(LeapSecond),
    /// Bytes follow a complete production.
    Trailing {
        /// The first byte after it.
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
            ParseErrorKind::OutOfRange {
                field,
                value,
                min,
                max,
            } => {
                write!(f, "{field} {value} is outside {min}..={max}")
            }
            ParseErrorKind::LeapSecond(LeapSecond::MonthEnd) => f.write_str(
                "second 60 is a leap second, allowed only at 23:59:60 UTC on the last day of a month",
            ),
            ParseErrorKind::LeapSecond(LeapSecond::AnyDay) => {
                f.write_str("second 60 is a leap second, allowed only at 23:59:60 UTC")
            }
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

    /// Refuses any byte left after a complete production.
    fn finish(&self) -> Result<(), ParseError> {
        match self.peek() {
            Some(found) => Err(self.error(ParseErrorKind::Trailing { found })),
            None => Ok(()),
        }
    }

    /// `full-date = date-fullyear "-" date-month "-" date-mday`.
    fn full_date(&mut self) -> Result<(i64, u8, u8), ParseError> {
        let year = self.field(4, "a four-digit year", "year", 0, 9999)?;
        self.one_of(b"-", "'-' after the four-digit year")?;
        let month = self.field(2, "a two-digit month", "month", 1, 12)?;
        self.one_of(b"-", "'-' after the month")?;
        let year = i64::from(year);
        // `field` bounded the month to 1..=12.
        let month = month as u8;
        let last_day = u32::from(days_in_month(year, month));
        // `field` bounded the day to 1..=31.
        let day = self.field(2, "a two-digit day", "day of month", 1, last_day)? as u8;
        Ok((year, month, day))
    }

    /// `full-time = partial-time time-offset`.
    fn full_time(&mut self) -> Result<Time, ParseError> {
        let hour = self.field(2, "a two-digit hour", "hour", 0, 23)?;
        self.one_of(b":", "':' after the hour")?;
        let minute = self.field(2, "a two-digit minute", "minute", 0, 59)?;
        self.one_of(b":", "':' after the minute")?;
        let second_at = self.at;
        let second = self.field(2, "a two-digit second", "second", 0, 60)?;
        let nanos = if self.peek() == Some(b'.') {
            self.at += 1;
            self.fraction()?
        } else {
            0
        };
        let offset_seconds = match self.one_of(b"Zz+-", "'Z', 'z', '+' or '-' for the time zone")? {
            b'Z' | b'z' => 0,
            sign => {
                let hours = self.field(2, "a two-digit offset hour", "offset hour", 0, 23)?;
                self.one_of(b":", "':' in the offset")?;
                let minutes = self.field(2, "a two-digit offset minute", "offset minute", 0, 59)?;
                let magnitude = i64::from(hours * 3600 + minutes * 60);
                if sign == b'-' { -magnitude } else { magnitude }
            }
        };
        Ok(Time {
            // A leap second counts as the 23:59:59 Unix second.
            second_of_day: i64::from(hour * 3600 + minute * 60 + second.min(59)),
            leap: second == 60,
            second_at,
            nanos,
            offset_seconds,
        })
    }
}

/// A `full-time` as read, before its leap second is judged.
struct Time {
    /// The local second of the day, a leap second counted as second 59.
    second_of_day: i64,
    /// Whether the seconds field read 60.
    leap: bool,
    /// The byte offset of the seconds field, which a leap-second error blames.
    second_at: usize,
    nanos: u32,
    offset_seconds: i64,
}

impl Time {
    /// The leap-second error, unless the UTC instant `seconds` (a `:60` mapped
    /// onto the Unix second before it) may be followed by a leap second under
    /// `policy`.
    fn check_leap(&self, seconds: i64, policy: LeapSecond) -> Result<(), ParseError> {
        if !self.leap || is_utc_leap_second_slot(seconds, policy) {
            return Ok(());
        }
        Err(ParseError {
            at: self.second_at,
            kind: ParseErrorKind::LeapSecond(policy),
        })
    }
}

/// Parses an RFC 3339 `date-time` into Unix seconds and nanoseconds in UTC.
///
/// # Errors
///
/// [`ParseError`] names the first byte at which `text` stops being a
/// `date-time` under `separator` and `leap_second`.
pub fn parse(
    text: &str,
    separator: Separator,
    leap_second: LeapSecond,
) -> Result<(i64, u32), ParseError> {
    let mut cursor = Cursor::new(text.as_bytes());
    let (year, month, day) = cursor.full_date()?;
    match separator {
        Separator::Rfc3339Abnf => cursor.one_of(b"Tt", "'T' or 't' between date and time")?,
        Separator::GtsSpaceAllowed => {
            cursor.one_of(b"Tt ", "'T', 't' or ' ' between date and time")?
        }
    };
    let time = cursor.full_time()?;
    cursor.finish()?;
    let seconds =
        day_number(year, month, day) * SECONDS_PER_DAY + time.second_of_day - time.offset_seconds;
    time.check_leap(seconds, leap_second)?;
    Ok((seconds, time.nanos))
}

/// Parses an RFC 3339 `full-date` into its year, month and day.
///
/// # Errors
///
/// [`ParseError`] names the first byte at which `text` stops being a
/// `full-date`.
pub fn parse_date(text: &str) -> Result<(u16, u8, u8), ParseError> {
    let mut cursor = Cursor::new(text.as_bytes());
    let (year, month, day) = cursor.full_date()?;
    cursor.finish()?;
    // `full_date` bounded the year to 0..=9999.
    Ok((year as u16, month, day))
}

/// Parses an RFC 3339 `full-time` into its UTC second of the day and
/// nanoseconds. A leap second answers second 86,399 with its fraction kept.
///
/// A `full-time` names no date, so a leap second is judged by the time of day
/// alone ([`LeapSecond::AnyDay`]): second 60 is accepted exactly when the time,
/// normalized to UTC, is 23:59:60.
///
/// # Errors
///
/// [`ParseError`] names the first byte at which `text` stops being a
/// `full-time`.
pub fn parse_time(text: &str) -> Result<(u32, u32), ParseError> {
    let mut cursor = Cursor::new(text.as_bytes());
    let time = cursor.full_time()?;
    cursor.finish()?;
    let seconds = (time.second_of_day - time.offset_seconds).rem_euclid(SECONDS_PER_DAY);
    time.check_leap(seconds, LeapSecond::AnyDay)?;
    // `rem_euclid` bounded the second to 0..86_400.
    Ok((seconds as u32, time.nanos))
}

/// Whether `seconds` (the Unix second a `:60` was mapped onto) is 23:59:59 UTC,
/// on the last day of a month under [`LeapSecond::MonthEnd`], i.e. whether a
/// leap second may follow it.
fn is_utc_leap_second_slot(seconds: i64, policy: LeapSecond) -> bool {
    if seconds.rem_euclid(SECONDS_PER_DAY) != SECONDS_PER_DAY - 1 {
        return false;
    }
    match policy {
        LeapSecond::AnyDay => true,
        LeapSecond::MonthEnd => {
            let (year, month, day) =
                civil_from_days(i128::from(seconds.div_euclid(SECONDS_PER_DAY)));
            // A day number from an `i64` second is far inside `i64` years.
            day == days_in_month(year as i64, month)
        }
    }
}

/// Writes `seconds` + `nanos` since the Unix epoch as canonical RFC 3339 UTC,
/// `YYYY-MM-DDTHH:MM:SS[.f]Z`. The fraction is omitted when `nanos` is zero and
/// otherwise carries the nanoseconds without trailing zeros.
///
/// # Errors
///
/// When `nanos` is a second or more, or the instant lies outside the
/// four-digit years 0000..=9999.
pub fn format(seconds: i64, nanos: u32) -> Result<String, &'static str> {
    if nanos >= NANOS_PER_SECOND {
        return Err("nanoseconds must be less than one second");
    }
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    if days < day_number(FIRST_YEAR, 1, 1) || days >= day_number(YEAR_AFTER_LAST, 1, 1) {
        return Err("instant lies outside the four-digit years 0000..=9999");
    }
    // `rem_euclid` bounded the second to 0..86_400.
    let second_of_day = seconds.rem_euclid(SECONDS_PER_DAY) as u32;
    let (year, month, day) = civil_from_days(i128::from(days));

    let mut out = String::with_capacity(30);
    // The range check above bounded the year to 0..=9999.
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
        // A decimal digit, 0..=9.
        out.push(char::from(b'0' + (value / divisor % 10) as u8));
        divisor /= 10;
    }
}

/// Days from 1970-01-01 to the given proleptic-Gregorian date.
fn day_number(year: i64, month: u8, day: u8) -> i64 {
    i64::try_from(days_from_civil(year, month, day))
        .expect("every year this module reaches is within a few thousand of 1970")
}
