// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 3339 §5.6 `date-time`, `full-date` and `full-time`: one grammar, with the
//! few readings the specifications that cite it leave to their consumers made
//! explicit as a policy.
//!
//! Two consumers in the workspace read RFC 3339 text and had each grown a parser:
//! the GTS files and tar profiles, whose `modified` stamps are instants, and the
//! JSON Schema `date-time`/`date`/`time` formats, which are validity questions. The
//! ABNF is the same; the readings differ in exactly four places, and
//! [`Rfc3339Options`] names each so a consumer's acceptance is a value it can
//! print, not a fact buried in a parser:
//!
//! * the separator between date and time — `T`, or also the space the §5.6 note
//!   permits "for readability" ([`Rfc3339Options::allow_space_separator`]);
//! * whether the same note's lower-case `t` and `z` are accepted
//!   ([`Rfc3339Options::allow_lowercase_t_z`]);
//! * whether the `time-offset` may be absent — RFC 3339 requires it; a consumer
//!   that also reads ISO 8601 local times relaxes it
//!   ([`Rfc3339Options::require_offset`]; an absent offset reads as UTC in
//!   [`Rfc3339::to_unix_seconds`]);
//! * when second `60` is admitted ([`LeapSecondPolicy`]): only at 23:59:60 UTC on
//!   the last day of a month, where the leap-second table has ever placed one, or
//!   at 23:59:60 UTC on any day, the reading that checks the time without the
//!   calendar.
//!
//! [`Rfc3339Options::GTS`] and [`Rfc3339Options::JSON_SCHEMA`] are the two
//! consumers' readings, byte for byte what each accepted before this module
//! existed. Everything else is the grammar with no reading to choose: a
//! four-digit year `0000`–`9999`, month `01`–`12`, a day that exists in that month
//! of the proleptic Gregorian calendar, hour `00`–`23`, minute `00`–`59`, second
//! `00`–`60`, a fraction of any length truncated (never rounded) to nanoseconds,
//! offset hour `00`–`23` and offset minute `00`–`59`, and nothing after the zone.
//! `-00:00` (§4.3) and `+00:00` both denote UTC.
//!
//! [`Rfc3339::to_unix_seconds`] is the instant: Unix seconds have no 61st second,
//! so a leap second answers the Unix second of 23:59:59, with its fraction kept.
//! [`Rfc3339::format_utc`] writes the one canonical UTC spelling,
//! `YYYY-MM-DDTHH:MM:SS[.f]Z`, with the fraction's trailing zeros removed and
//! omitted when zero.
//!
//! ```rust
//! use purrdf_xsd::rfc3339::{Rfc3339Options, parse};
//!
//! let stamp = parse("1996-12-19T16:39:57-08:00", &Rfc3339Options::GTS)?;
//! assert_eq!((stamp.year, stamp.month, stamp.day), (1996, 12, 19));
//! assert_eq!(stamp.offset_minutes, Some(-8 * 60));
//! assert_eq!(stamp.to_unix_seconds(), 851_042_397);
//! assert_eq!(stamp.format_utc().as_deref(), Some("1996-12-20T00:39:57Z"));
//!
//! // The space separator is the GTS reading; JSON Schema's date-time refuses it.
//! assert!(parse("1996-12-19 16:39:57Z", &Rfc3339Options::GTS).is_ok());
//! assert!(parse("1996-12-19 16:39:57Z", &Rfc3339Options::JSON_SCHEMA).is_err());
//! # Ok::<(), purrdf_xsd::rfc3339::Rfc3339Error>(())
//! ```

use core::fmt;

use crate::temporal::{civil_from_days, days_from_civil, days_in_month};

const SECONDS_PER_DAY: i64 = 86_400;
const NANOS_PER_SECOND: u32 = 1_000_000_000;
const FRACTION_DIGITS: u32 = 9;

/// A parsed RFC 3339 `date-time`, field by field, exactly as written: a leap
/// second keeps `second == 60`, and the offset is the one spelled (`None` only
/// when [`Rfc3339Options::require_offset`] was off and none was written).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rfc3339 {
    /// `date-fullyear`, `0..=9999`.
    pub year: i32,
    /// `date-month`, `1..=12`.
    pub month: u8,
    /// `date-mday`, `1..=31` and within the month.
    pub day: u8,
    /// `time-hour`, `0..=23`.
    pub hour: u8,
    /// `time-minute`, `0..=59`.
    pub minute: u8,
    /// `time-second`, `0..=60`.
    pub second: u8,
    /// `time-secfrac` as nanoseconds, `0..NANOS_PER_SECOND`; digits past the ninth
    /// are dropped.
    pub nanos: u32,
    /// `time-offset` in minutes east of UTC (`Z` is `Some(0)`), or `None` for an
    /// absent offset.
    pub offset_minutes: Option<i16>,
}

/// A parsed RFC 3339 `full-time`: the time-of-day fields of [`Rfc3339`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rfc3339Time {
    /// `time-hour`, `0..=23`.
    pub hour: u8,
    /// `time-minute`, `0..=59`.
    pub minute: u8,
    /// `time-second`, `0..=60`.
    pub second: u8,
    /// `time-secfrac` as nanoseconds; digits past the ninth are dropped.
    pub nanos: u32,
    /// `time-offset` in minutes east of UTC, or `None` for an absent offset.
    pub offset_minutes: Option<i16>,
}

/// When a `time-second` of `60` is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeapSecondPolicy {
    /// Only when the instant, normalized to UTC, is 23:59:60 on the last day of a
    /// month (RFC 3339 §5.7, Appendix D): the slots the leap-second table can fill.
    MonthEndUtcOnly,
    /// Whenever the time, normalized to UTC, is 23:59:60, on any day: the reading
    /// of a validator that checks the time-of-day without consulting the calendar,
    /// and the only one possible for a bare `full-time`.
    AnyDayAt2359,
}

/// The readings RFC 3339 leaves to a consumer. See the [module
/// documentation](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rfc3339Options {
    /// Accept a single space between `full-date` and `full-time` as well as `T`.
    pub allow_space_separator: bool,
    /// Accept `t` for the separator and `z` for the zone as well as `T` and `Z`.
    pub allow_lowercase_t_z: bool,
    /// Refuse a `date-time` or `full-time` with no `time-offset`. When off, an
    /// absent offset parses as `offset_minutes: None` and reads as UTC.
    pub require_offset: bool,
    /// When second `60` is admitted.
    pub leap_second: LeapSecondPolicy,
}

impl Rfc3339Options {
    /// The GTS files and tar profiles' reading: `T`, `t` or a space between date
    /// and time; `Z` or `z`; the offset required; a leap second only at a UTC month
    /// end.
    pub const GTS: Self = Self {
        allow_space_separator: true,
        allow_lowercase_t_z: true,
        require_offset: true,
        leap_second: LeapSecondPolicy::MonthEndUtcOnly,
    };

    /// The JSON Schema `date-time`/`time` formats' reading: `T` or `t` only; `Z` or
    /// `z`; the offset required; a leap second whenever the UTC time is 23:59:60.
    pub const JSON_SCHEMA: Self = Self {
        allow_space_separator: false,
        allow_lowercase_t_z: true,
        require_offset: true,
        leap_second: LeapSecondPolicy::AnyDayAt2359,
    };

    /// The separator bytes this reading accepts, and how to name them.
    const fn separators(self) -> (&'static [u8], &'static str) {
        match (self.allow_lowercase_t_z, self.allow_space_separator) {
            (true, true) => (b"Tt ", "'T', 't' or ' ' between date and time"),
            (true, false) => (b"Tt", "'T' or 't' between date and time"),
            (false, true) => (b"T ", "'T' or ' ' between date and time"),
            (false, false) => (b"T", "'T' between date and time"),
        }
    }

    /// The bytes that open a `time-offset` under this reading, and how to name them.
    const fn zone_starts(self) -> (&'static [u8], &'static str) {
        if self.allow_lowercase_t_z {
            (b"Zz+-", "'Z', 'z', '+' or '-' for the time zone")
        } else {
            (b"Z+-", "'Z', '+' or '-' for the time zone")
        }
    }
}

/// Why a text is not the RFC 3339 production it was read as, and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rfc3339Error {
    at: usize,
    kind: Rfc3339ErrorKind,
    production: &'static str,
}

impl Rfc3339Error {
    /// The byte offset in the input at which the text stops being acceptable.
    #[must_use]
    pub const fn at(&self) -> usize {
        self.at
    }

    /// What went wrong at [`Self::at`].
    #[must_use]
    pub const fn kind(&self) -> Rfc3339ErrorKind {
        self.kind
    }
}

/// The class of an [`Rfc3339Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rfc3339ErrorKind {
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
    /// Second 60 where the [`LeapSecondPolicy`] does not admit one.
    LeapSecond {
        /// The policy that refused it.
        policy: LeapSecondPolicy,
    },
    /// Bytes follow a complete production.
    Trailing {
        /// The first byte after it.
        found: u8,
    },
}

impl fmt::Display for Rfc3339Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid RFC 3339 {} at byte {}: ",
            self.production, self.at
        )?;
        match self.kind {
            Rfc3339ErrorKind::EndOfInput { expected } => {
                write!(f, "expected {expected}, found end of input")
            }
            Rfc3339ErrorKind::Unexpected { expected, found } => {
                write!(f, "expected {expected}, found {}", DisplayByte(found))
            }
            Rfc3339ErrorKind::OutOfRange {
                field,
                value,
                min,
                max,
            } => write!(f, "{field} {value} is outside {min}..={max}"),
            Rfc3339ErrorKind::LeapSecond {
                policy: LeapSecondPolicy::MonthEndUtcOnly,
            } => f.write_str(
                "second 60 is a leap second, allowed only at 23:59:60 UTC on the last day of a month",
            ),
            Rfc3339ErrorKind::LeapSecond {
                policy: LeapSecondPolicy::AnyDayAt2359,
            } => f.write_str("second 60 is a leap second, allowed only at 23:59:60 UTC"),
            Rfc3339ErrorKind::Trailing { found } => {
                write!(f, "unexpected {} after the {}", DisplayByte(found), self.production)
            }
        }
    }
}

impl std::error::Error for Rfc3339Error {}

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

/// A forward-only reader over the input bytes of one production.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
    production: &'static str,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8], production: &'static str) -> Self {
        Self {
            bytes,
            at: 0,
            production,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    const fn error_at(&self, at: usize, kind: Rfc3339ErrorKind) -> Rfc3339Error {
        Rfc3339Error {
            at,
            kind,
            production: self.production,
        }
    }

    const fn error(&self, kind: Rfc3339ErrorKind) -> Rfc3339Error {
        self.error_at(self.at, kind)
    }

    fn mismatch(&self, expected: &'static str) -> Rfc3339Error {
        match self.peek() {
            Some(found) => self.error(Rfc3339ErrorKind::Unexpected { expected, found }),
            None => self.error(Rfc3339ErrorKind::EndOfInput { expected }),
        }
    }

    /// Consumes one byte from `allowed` and answers it.
    fn one_of(&mut self, allowed: &[u8], expected: &'static str) -> Result<u8, Rfc3339Error> {
        match self.peek() {
            Some(byte) if allowed.contains(&byte) => {
                self.at += 1;
                Ok(byte)
            }
            _ => Err(self.mismatch(expected)),
        }
    }

    /// Consumes exactly `count` ASCII digits and answers their value, then checks
    /// it against `min..=max`, blaming the field's first byte.
    fn field(
        &mut self,
        count: usize,
        expected: &'static str,
        field: &'static str,
        min: u32,
        max: u32,
    ) -> Result<u32, Rfc3339Error> {
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
            Err(self.error_at(
                start,
                Rfc3339ErrorKind::OutOfRange {
                    field,
                    value,
                    min,
                    max,
                },
            ))
        }
    }

    /// Consumes `1*DIGIT` of a `time-secfrac` (the `.` already consumed) and
    /// answers its first nine digits as nanoseconds; later digits are dropped.
    fn fraction(&mut self) -> Result<u32, Rfc3339Error> {
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

    /// `full-date`: `date-fullyear "-" date-month "-" date-mday`, the day checked
    /// against the month.
    fn full_date(&mut self) -> Result<(i32, u8, u8), Rfc3339Error> {
        let year = self.field(4, "a four-digit year", "year", 0, 9999)?;
        self.one_of(b"-", "'-' after the four-digit year")?;
        let month = self.field(2, "a two-digit month", "month", 1, 12)?;
        self.one_of(b"-", "'-' after the month")?;
        let month = month as u8;
        let last_day = u32::from(days_in_month(i64::from(year), month));
        let day = self.field(2, "a two-digit day", "day of month", 1, last_day)? as u8;
        Ok((year as i32, month, day))
    }

    /// `full-time`: `partial-time time-offset`, the offset optional only when the
    /// reading says so. Answers the fields and the byte offset of the seconds field,
    /// which a leap-second refusal blames.
    fn full_time(&mut self, options: Rfc3339Options) -> Result<(Rfc3339Time, usize), Rfc3339Error> {
        let hour = self.field(2, "a two-digit hour", "hour", 0, 23)? as u8;
        self.one_of(b":", "':' after the hour")?;
        let minute = self.field(2, "a two-digit minute", "minute", 0, 59)? as u8;
        self.one_of(b":", "':' after the minute")?;
        let second_at = self.at;
        let second = self.field(2, "a two-digit second", "second", 0, 60)? as u8;
        let nanos = if self.peek() == Some(b'.') {
            self.at += 1;
            self.fraction()?
        } else {
            0
        };
        let (starts, expected) = options.zone_starts();
        let offset_minutes = match self.peek() {
            None if !options.require_offset => None,
            Some(byte) if !options.require_offset && !starts.contains(&byte) => None,
            _ => Some(self.offset(starts, expected)?),
        };
        Ok((
            Rfc3339Time {
                hour,
                minute,
                second,
                nanos,
                offset_minutes,
            },
            second_at,
        ))
    }

    /// `time-offset`: `Z` / `z` or `time-numoffset`, in minutes east of UTC.
    fn offset(&mut self, starts: &[u8], expected: &'static str) -> Result<i16, Rfc3339Error> {
        match self.one_of(starts, expected)? {
            b'Z' | b'z' => Ok(0),
            sign => {
                let hours = self.field(2, "a two-digit offset hour", "offset hour", 0, 23)?;
                self.one_of(b":", "':' in the offset")?;
                let minutes = self.field(2, "a two-digit offset minute", "offset minute", 0, 59)?;
                let magnitude = (hours * 60 + minutes) as i16;
                Ok(if sign == b'-' { -magnitude } else { magnitude })
            }
        }
    }

    /// Refuses anything after a complete production.
    fn end(&self) -> Result<(), Rfc3339Error> {
        match self.peek() {
            Some(found) => Err(self.error(Rfc3339ErrorKind::Trailing { found })),
            None => Ok(()),
        }
    }
}

/// Whether the UTC time of day at `unix` seconds is 23:59:59 — the second a leap
/// second `:60` is mapped onto.
const fn is_last_minute_utc(unix: i64) -> bool {
    unix.rem_euclid(SECONDS_PER_DAY) == SECONDS_PER_DAY - 1
}

/// Whether the UTC date at `unix` seconds is the last day of its month.
fn is_month_end_utc(unix: i64) -> bool {
    let (year, month, day) = civil_from_days(i128::from(unix.div_euclid(SECONDS_PER_DAY)));
    let year = i64::try_from(year).expect("a four-digit year's day count");
    day == days_in_month(year, month)
}

/// Parses an RFC 3339 `date-time` under `options`.
///
/// # Errors
///
/// [`Rfc3339Error`], naming the byte offset and the reason: a byte the grammar
/// does not expect (or the end of the input where it needs more), a field outside
/// its range (the day checked against its month and year), a second `60` the
/// [`LeapSecondPolicy`] does not admit, or bytes after the zone.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::rfc3339::{Rfc3339, Rfc3339ErrorKind, Rfc3339Options, parse};
///
/// let leap = parse("1990-12-31T15:59:60-08:00", &Rfc3339Options::GTS)?;
/// assert_eq!(leap.second, 60);
/// // The leap second shares 23:59:59's Unix second.
/// assert_eq!(leap.to_unix_seconds(), parse("1990-12-31T23:59:59Z", &Rfc3339Options::GTS)?.to_unix_seconds());
///
/// // Not a month end: refused by GTS, admitted by JSON Schema's time-only check.
/// let mid_month = "2024-02-28T23:59:60Z";
/// assert!(matches!(
///     parse(mid_month, &Rfc3339Options::GTS).unwrap_err().kind(),
///     Rfc3339ErrorKind::LeapSecond { .. }
/// ));
/// assert!(parse(mid_month, &Rfc3339Options::JSON_SCHEMA).is_ok());
/// # Ok::<(), purrdf_xsd::rfc3339::Rfc3339Error>(())
/// ```
pub fn parse(text: &str, options: &Rfc3339Options) -> Result<Rfc3339, Rfc3339Error> {
    let mut cursor = Cursor::new(text.as_bytes(), "date-time");
    let (year, month, day) = cursor.full_date()?;
    let (separators, expected) = options.separators();
    cursor.one_of(separators, expected)?;
    let (time, second_at) = cursor.full_time(*options)?;
    cursor.end()?;
    let value = Rfc3339 {
        year,
        month,
        day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        nanos: time.nanos,
        offset_minutes: time.offset_minutes,
    };
    if value.second == 60 {
        let unix = value.to_unix_seconds();
        let admitted = is_last_minute_utc(unix)
            && match options.leap_second {
                LeapSecondPolicy::AnyDayAt2359 => true,
                LeapSecondPolicy::MonthEndUtcOnly => is_month_end_utc(unix),
            };
        if !admitted {
            return Err(cursor.error_at(
                second_at,
                Rfc3339ErrorKind::LeapSecond {
                    policy: options.leap_second,
                },
            ));
        }
    }
    Ok(value)
}

/// Parses an RFC 3339 `full-date`, `YYYY-MM-DD`, as `(year, month, day)` — the
/// JSON Schema `date` format. The day must exist in its month of the proleptic
/// Gregorian calendar, and nothing may follow.
///
/// # Errors
///
/// [`Rfc3339Error`], as for [`parse`].
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::rfc3339::parse_full_date;
///
/// assert_eq!(parse_full_date("2020-02-29")?, (2020, 2, 29));
/// assert!(parse_full_date("2021-02-29").is_err(), "not a leap year");
/// assert!(parse_full_date("2020-02-29T00:00:00Z").is_err(), "a date, not a date-time");
/// # Ok::<(), purrdf_xsd::rfc3339::Rfc3339Error>(())
/// ```
pub fn parse_full_date(text: &str) -> Result<(i32, u8, u8), Rfc3339Error> {
    let mut cursor = Cursor::new(text.as_bytes(), "full-date");
    let date = cursor.full_date()?;
    cursor.end()?;
    Ok(date)
}

/// Parses an RFC 3339 `full-time`, `HH:MM:SS[.f](Z|±HH:MM)`, under `options` —
/// the JSON Schema `time` format. Of the options, the separator does not apply;
/// the lower-case `z`, the offset requirement and the leap-second policy do, the
/// last with no calendar to consult: a second `60` is admitted exactly when the
/// time normalized to UTC is 23:59:60, under either policy.
///
/// # Errors
///
/// [`Rfc3339Error`], as for [`parse`].
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::rfc3339::{Rfc3339Options, parse_full_time};
///
/// let time = parse_full_time("15:59:60-08:00", &Rfc3339Options::JSON_SCHEMA)?;
/// assert_eq!((time.hour, time.second, time.offset_minutes), (15, 60, Some(-480)));
/// assert!(parse_full_time("15:59:60-08:01", &Rfc3339Options::JSON_SCHEMA).is_err());
/// assert!(parse_full_time("08:30:06", &Rfc3339Options::JSON_SCHEMA).is_err(), "no offset");
/// # Ok::<(), purrdf_xsd::rfc3339::Rfc3339Error>(())
/// ```
pub fn parse_full_time(text: &str, options: &Rfc3339Options) -> Result<Rfc3339Time, Rfc3339Error> {
    let mut cursor = Cursor::new(text.as_bytes(), "full-time");
    let (time, second_at) = cursor.full_time(*options)?;
    cursor.end()?;
    if time.second == 60 {
        let minute_of_day = i32::from(time.hour) * 60 + i32::from(time.minute);
        let utc = (minute_of_day - i32::from(time.offset_minutes.unwrap_or(0))).rem_euclid(24 * 60);
        if utc != 23 * 60 + 59 {
            return Err(cursor.error_at(
                second_at,
                Rfc3339ErrorKind::LeapSecond {
                    policy: options.leap_second,
                },
            ));
        }
    }
    Ok(time)
}

impl Rfc3339 {
    /// The UTC civil breakdown of `seconds` + `nanos` since the Unix epoch, with
    /// `offset_minutes: Some(0)`, or `None` when `nanos` is a second or more or the
    /// year falls outside `0000..=9999` — the years a `date-fullyear` can spell.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::rfc3339::Rfc3339;
    ///
    /// let stamp = Rfc3339::from_unix_seconds(951_782_400, 0).expect("in range");
    /// assert_eq!((stamp.year, stamp.month, stamp.day), (2000, 2, 29));
    /// assert_eq!(Rfc3339::from_unix_seconds(i64::MAX, 0), None);
    /// assert_eq!(Rfc3339::from_unix_seconds(0, 1_000_000_000), None);
    /// ```
    #[must_use]
    pub fn from_unix_seconds(seconds: i64, nanos: u32) -> Option<Self> {
        if nanos >= NANOS_PER_SECOND {
            return None;
        }
        let days = seconds.div_euclid(SECONDS_PER_DAY);
        let second_of_day = seconds.rem_euclid(SECONDS_PER_DAY) as u32;
        let (year, month, day) = civil_from_days(i128::from(days));
        let year = i32::try_from(year)
            .ok()
            .filter(|year| (0..=9999).contains(year))?;
        Some(Self {
            year,
            month,
            day,
            hour: (second_of_day / 3600) as u8,
            minute: (second_of_day / 60 % 60) as u8,
            second: (second_of_day % 60) as u8,
            nanos,
            offset_minutes: Some(0),
        })
    }

    /// The instant as seconds since the Unix epoch: the civil fields at the offset
    /// (an absent offset reads as UTC), with a leap second `60` answering the Unix
    /// second of 23:59:59, since Unix time has no 61st second.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::rfc3339::{Rfc3339Options, parse};
    ///
    /// let stamp = parse("1985-04-12T23:20:50.52Z", &Rfc3339Options::GTS)?;
    /// assert_eq!((stamp.to_unix_seconds(), stamp.nanos), (482_196_050, 520_000_000));
    /// # Ok::<(), purrdf_xsd::rfc3339::Rfc3339Error>(())
    /// ```
    #[must_use]
    pub fn to_unix_seconds(&self) -> i64 {
        let days = i64::try_from(days_from_civil(i64::from(self.year), self.month, self.day))
            .expect("an i32 year's day count fits i64");
        let second_of_day = i64::from(self.hour) * 3600
            + i64::from(self.minute) * 60
            + i64::from(self.second.min(59));
        days * SECONDS_PER_DAY + second_of_day - i64::from(self.offset_minutes.unwrap_or(0)) * 60
    }

    /// The canonical UTC spelling of the instant, `YYYY-MM-DDTHH:MM:SS[.f]Z`: the
    /// fields normalized to UTC, the fraction omitted when zero and otherwise
    /// carrying the nanoseconds without trailing zeros. `None` when the UTC year
    /// lies outside `0000..=9999` (a text can parse and still normalize past a
    /// four-digit year) or `nanos` is a second or more.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::rfc3339::{Rfc3339Options, parse};
    ///
    /// let opts = Rfc3339Options::GTS;
    /// assert_eq!(
    ///     parse("1985-04-12t23:20:50.520z", &opts)?.format_utc().as_deref(),
    ///     Some("1985-04-12T23:20:50.52Z")
    /// );
    /// assert_eq!(
    ///     parse("1990-12-31T15:59:60.25-08:00", &opts)?.format_utc().as_deref(),
    ///     Some("1990-12-31T23:59:59.25Z")
    /// );
    /// assert_eq!(parse("9999-12-31T23:59:59-01:00", &opts)?.format_utc(), None);
    /// # Ok::<(), purrdf_xsd::rfc3339::Rfc3339Error>(())
    /// ```
    #[must_use]
    pub fn format_utc(&self) -> Option<String> {
        let utc = Self::from_unix_seconds(self.to_unix_seconds(), self.nanos)?;
        let mut out = String::with_capacity(30);
        push_digits(&mut out, utc.year as u32, 4);
        out.push('-');
        push_digits(&mut out, u32::from(utc.month), 2);
        out.push('-');
        push_digits(&mut out, u32::from(utc.day), 2);
        out.push('T');
        push_digits(&mut out, u32::from(utc.hour), 2);
        out.push(':');
        push_digits(&mut out, u32::from(utc.minute), 2);
        out.push(':');
        push_digits(&mut out, u32::from(utc.second), 2);
        if utc.nanos != 0 {
            let mut fraction = utc.nanos;
            let mut width = FRACTION_DIGITS;
            while fraction.is_multiple_of(10) {
                fraction /= 10;
                width -= 1;
            }
            out.push('.');
            push_digits(&mut out, fraction, width);
        }
        out.push('Z');
        Some(out)
    }
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

#[cfg(test)]
mod tests {
    use super::{
        LeapSecondPolicy, Rfc3339, Rfc3339ErrorKind, Rfc3339Options, parse, parse_full_date,
        parse_full_time,
    };

    use purrdf_testkit::rng::splitmix64_next as splitmix64;

    // ---- the GTS oracle: what the files and tar profiles accepted -------------------

    const GTS: Rfc3339Options = Rfc3339Options::GTS;

    fn ok(text: &str) -> (i64, u32) {
        let value = parse(text, &GTS).unwrap_or_else(|err| panic!("{text:?} must parse: {err}"));
        (value.to_unix_seconds(), value.nanos)
    }

    fn refused(text: &str) -> Rfc3339ErrorKind {
        match parse(text, &GTS) {
            Ok(value) => panic!("{text:?} must be refused, parsed as {value:?}"),
            Err(err) => err.kind(),
        }
    }

    fn out_of_range(text: &str, field: &str) {
        match refused(text) {
            Rfc3339ErrorKind::OutOfRange { field: got, .. } if got == field => {}
            other => panic!("{text:?}: expected {field} out of range, got {other:?}"),
        }
    }

    /// The GTS formatter: an instant to its canonical UTC text.
    fn format(seconds: i64, nanos: u32) -> Option<String> {
        Rfc3339::from_unix_seconds(seconds, nanos).and_then(|stamp| stamp.format_utc())
    }

    const LEAP_REFUSED: Rfc3339ErrorKind = Rfc3339ErrorKind::LeapSecond {
        policy: LeapSecondPolicy::MonthEndUtcOnly,
    };

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
    fn fields_are_kept_as_written() {
        let stamp = parse("1990-12-31T15:59:60.25-08:00", &GTS).expect("valid");
        assert_eq!(
            stamp,
            Rfc3339 {
                year: 1990,
                month: 12,
                day: 31,
                hour: 15,
                minute: 59,
                second: 60,
                nanos: 250_000_000,
                offset_minutes: Some(-480),
            }
        );
        assert_eq!(
            parse("2023-11-14T22:13:20+05:30", &GTS)
                .expect("valid")
                .offset_minutes,
            Some(330)
        );
        assert_eq!(
            parse("2023-11-14T22:13:20-00:00", &GTS)
                .expect("valid")
                .offset_minutes,
            Some(0)
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
            Rfc3339ErrorKind::Unexpected { .. }
        ));
        assert!(matches!(
            refused("2023-11-14T22:13:20"),
            Rfc3339ErrorKind::EndOfInput { .. }
        ));
        assert!(matches!(
            refused("2023-11-14T22:13:20+0000"),
            Rfc3339ErrorKind::Unexpected { .. }
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
            Rfc3339ErrorKind::Unexpected { .. }
        ));
        assert!(matches!(
            refused("1970-01-01T00:00:00."),
            Rfc3339ErrorKind::EndOfInput { .. }
        ));
    }

    #[test]
    fn doubled_zone_letter_is_refused_single_accepted() {
        assert!(matches!(
            refused("2024-01-01T00:00:00ZZ"),
            Rfc3339ErrorKind::Trailing { found: b'Z' }
        ));
        ok("2024-01-01T00:00:00Z");
    }

    #[test]
    fn offset_followed_by_zone_letter_is_refused_offset_alone_accepted() {
        assert!(matches!(
            refused("2024-01-01T00:00:00+00:00Z"),
            Rfc3339ErrorKind::Trailing { found: b'Z' }
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
        assert_eq!(refused("1990-12-31T15:59:60Z"), LEAP_REFUSED);
        // 23:59:60 local but 22:59:60 UTC.
        assert_eq!(refused("1990-12-31T23:59:60+01:00"), LEAP_REFUSED);
        // 23:59:60 UTC, but not the last day of the month.
        assert_eq!(refused("1990-12-30T23:59:60Z"), LEAP_REFUSED);
        assert_eq!(refused("2024-02-28T23:59:60Z"), LEAP_REFUSED);
        // The RFC's valid example, and other month ends in UTC.
        ok("1990-12-31T15:59:60-08:00");
        ok("2024-02-29T23:59:60Z");
        ok("1997-06-30T23:59:60.5Z");
        // Local date differs from the UTC date that makes it valid.
        ok("1998-01-01T00:59:60+01:00");
        let err = parse("1990-12-31T15:59:60Z", &GTS).expect_err("refused above");
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
            Rfc3339ErrorKind::Unexpected { found: b'0', .. }
        ));
        ok("9999-12-31T23:59:59Z");
        ok("0000-01-01T00:00:00Z");
        assert!(matches!(
            refused("999-01-01T00:00:00Z"),
            Rfc3339ErrorKind::Unexpected { .. }
        ));
    }

    #[test]
    fn error_messages_name_the_position_and_problem() {
        let text = parse("2024-13-01T00:00:00Z", &GTS)
            .expect_err("month 13")
            .to_string();
        assert_eq!(
            text,
            "invalid RFC 3339 date-time at byte 5: month 13 is outside 1..=12"
        );
        let text = parse("2024-01-01T00:00:00", &GTS)
            .expect_err("no zone")
            .to_string();
        assert!(
            text.contains("byte 19") && text.contains("end of input"),
            "{text}"
        );
        let text = parse("2024-01-01T00:00:00Z\n", &GTS)
            .expect_err("trailing")
            .to_string();
        assert!(text.contains("byte 0x0a"), "{text}");
        let text = parse("1990-12-30T23:59:60Z", &GTS)
            .expect_err("leap second")
            .to_string();
        assert_eq!(
            text,
            "invalid RFC 3339 date-time at byte 17: second 60 is a leap second, allowed only at 23:59:60 UTC on the last day of a month"
        );
        let text = parse("2024-02-28T22:59:60Z", &Rfc3339Options::JSON_SCHEMA)
            .expect_err("leap second")
            .to_string();
        assert_eq!(
            text,
            "invalid RFC 3339 date-time at byte 17: second 60 is a leap second, allowed only at 23:59:60 UTC"
        );
        let text = parse_full_date("2020-01-01X")
            .expect_err("trailing")
            .to_string();
        assert_eq!(
            text,
            "invalid RFC 3339 full-date at byte 10: unexpected 'X' after the full-date"
        );
        let text = parse_full_time("08:30:06", &Rfc3339Options::JSON_SCHEMA)
            .expect_err("no offset")
            .to_string();
        assert_eq!(
            text,
            "invalid RFC 3339 full-time at byte 8: expected 'Z', 'z', '+' or '-' for the time zone, found end of input"
        );
    }

    #[test]
    fn format_writes_canonical_utc() {
        assert_eq!(format(0, 0).as_deref(), Some("1970-01-01T00:00:00Z"));
        assert_eq!(
            format(1_700_000_000, 0).as_deref(),
            Some("2023-11-14T22:13:20Z")
        );
        assert_eq!(
            format(482_196_050, 520_000_000).as_deref(),
            Some("1985-04-12T23:20:50.52Z")
        );
        assert_eq!(
            format(-1, 1).as_deref(),
            Some("1969-12-31T23:59:59.000000001Z")
        );
        assert_eq!(
            format(-1_041_337_173, 870_000_000).as_deref(),
            Some("1937-01-01T11:40:27.87Z")
        );
        assert_eq!(
            format(951_782_400, 0).as_deref(),
            Some("2000-02-29T00:00:00Z")
        );
    }

    #[test]
    fn format_refuses_outside_four_digit_years() {
        let first = ok("0000-01-01T00:00:00Z").0;
        let last = ok("9999-12-31T23:59:59Z").0;
        assert_eq!(format(first, 0).as_deref(), Some("0000-01-01T00:00:00Z"));
        assert!(format(first - 1, 999_999_999).is_none());
        assert_eq!(
            format(last, 999_999_999).as_deref(),
            Some("9999-12-31T23:59:59.999999999Z")
        );
        assert!(format(last + 1, 0).is_none());
        assert!(format(i64::MIN, 0).is_none());
        assert!(format(i64::MAX, 0).is_none());
        assert!(format(0, 1_000_000_000).is_none());
        assert!(format(0, 999_999_999).is_some());
    }

    #[test]
    fn parse_accepts_instants_that_normalize_past_four_digit_years() {
        // Valid text; the UTC instant lies in year 10000 or year -1.
        let late = parse("9999-12-31T23:59:59-01:00", &GTS).expect("valid text");
        assert!(format(late.to_unix_seconds(), 0).is_none());
        assert_eq!(late.format_utc(), None);
        let early = parse("0000-01-01T00:00:00+01:00", &GTS).expect("valid text");
        assert!(format(early.to_unix_seconds(), 0).is_none());
        assert_eq!(early.format_utc(), None);
    }

    #[test]
    fn format_then_parse_round_trips_seeded_instants() {
        let first = ok("0000-01-01T00:00:00Z").0;
        let last = ok("9999-12-31T23:59:59Z").0;
        let span = u64::try_from(last - first + 1).expect("positive span");
        let mut state = 0x5075_7252_4446_3339_u64;
        for round in 0..20_000 {
            let seconds =
                first + i64::try_from(splitmix64(&mut state) % span).expect("span fits i64");
            let nanos = match round % 4 {
                0 => 0,
                1 => (splitmix64(&mut state) % 1000) as u32 * 1_000_000,
                _ => (splitmix64(&mut state) % 1_000_000_000) as u32,
            };
            let text = format(seconds, nanos).expect("in range");
            let parsed = parse(&text, &GTS).unwrap_or_else(|err| panic!("{text}: {err}"));
            assert_eq!(
                (parsed.to_unix_seconds(), parsed.nanos),
                (seconds, nanos),
                "{text}"
            );
            // Canonical: re-formatting the parse is byte-identical.
            assert_eq!(parsed.format_utc().as_deref(), Some(text.as_str()));
            // And the UTC breakdown is the parse itself.
            assert_eq!(Rfc3339::from_unix_seconds(seconds, nanos), Some(parsed));
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
            let stamp = parse(input, &GTS).expect("valid");
            assert_eq!(stamp.format_utc().as_deref(), Some(canonical), "{input}");
        }
    }

    // ---- the JSON Schema oracle: the `date-time`, `date` and `time` formats ---------

    const JSON: Rfc3339Options = Rfc3339Options::JSON_SCHEMA;

    /// The official JSON-Schema-Test-Suite `date-time` cases (draft 2020-12,
    /// optional/format) and the crate's own refused/accepted neighbours.
    #[test]
    fn json_schema_date_time_cases() {
        for (text, valid) in [
            ("1963-06-19T08:30:06.283185Z", true),
            ("1963-06-19T08:30:06Z", true),
            ("1937-01-01T12:00:27.87+00:20", true),
            ("1990-12-31T15:59:50.123-08:00", true),
            ("1998-12-31T23:59:60Z", true),
            ("1998-12-31T15:59:60.123-08:00", true),
            ("1998-12-31T23:59:61Z", false),
            ("1998-12-31T23:58:60Z", false),
            ("1998-12-31T22:59:60Z", false),
            ("1990-02-31T15:59:59.123-08:00", false),
            ("1990-12-31T15:59:59-24:00", false),
            ("1963-06-19T08:30:06.28123+01:00Z", false),
            ("1990-12-31T24:00:00Z", false),
            ("1990-12-31T15:60:00Z", false),
            ("1990-12-31T10:00:00+10:60", false),
            ("06/19/1963 08:30:06 PST", false),
            ("1963-06-19t08:30:06.283185z", true),
            ("2013-350T01:01:01", false),
            ("1963-6-19T08:30:06.283185Z", false),
            ("1963-06-1T08:30:06.283185Z", false),
            ("1963-06-1\u{9ea}T00:00:00Z", false),
            ("1963-06-11T0\u{9ea}:00:00Z", false),
            ("+11963-06-19T08:30:06.283185Z", false),
            ("1985-04-12T23:20:50+01", false),
            ("2016-12-31T24:59:60+01:00", false),
            ("1985-04-12T00:59:59.999999999999999Z", true),
            ("1985-04-12T23:20:50Z\n", false),
            ("1985-04-12T23:20Z", false),
            ("1985-04-12T23:20:50Ztail", false),
            ("1985-04-12T23:60:00+00:01", false),
            ("2021-02-28T00:00:00Z", true),
            ("2020-02-30T00:00:00Z", false),
            ("2021-02-29T00:00:00Z", false),
            ("2020-02-29T00:00:00Z", true),
            ("0100-02-29T00:00:00Z", false),
            ("0400-02-29T00:00:00Z", true),
            ("2100-02-29T00:00:00Z", false),
            // The crate's own pair.
            ("1998-12-31T23:59:60+01:00", false),
            // The space separator is not JSON Schema's reading.
            ("2023-11-14 22:13:20Z", false),
            ("", false),
            ("2023-11-14T", false),
        ] {
            assert_eq!(parse(text, &JSON).is_ok(), valid, "{text:?}");
        }
    }

    /// The `date` cases.
    #[test]
    fn json_schema_date_cases() {
        for (text, valid) in [
            ("1963-06-19", true),
            ("2020-01-31", true),
            ("2020-01-32", false),
            ("2021-02-28", true),
            ("2020-02-30", false),
            ("2020-03-31", true),
            ("2020-03-32", false),
            ("2020-04-30", true),
            ("2020-04-31", false),
            ("2020-05-31", true),
            ("2020-05-32", false),
            ("2020-06-30", true),
            ("2020-06-31", false),
            ("2020-07-31", true),
            ("2020-07-32", false),
            ("2020-08-31", true),
            ("2020-08-32", false),
            ("2020-09-30", true),
            ("2020-09-31", false),
            ("2020-10-31", true),
            ("2020-10-32", false),
            ("2020-11-30", true),
            ("2020-11-31", false),
            ("2020-12-31", true),
            ("2020-12-32", false),
            ("06/19/1963", false),
            ("2013-350", false),
            ("1998-1-20", false),
            ("1998-01-1", false),
            ("1998-13-01", false),
            ("2021-02-29", false),
            ("2020-02-29", true),
            ("1963-06-1\u{9ea}", false),
            ("2020-0\u{9ea}-01", false),
            ("20230328", false),
            ("2023-W01", false),
            ("2023-W13-2", false),
            ("2022W527", false),
            ("2020-11-28T23:55:45Z", false),
            ("0100-02-29", false),
            ("0400-02-29", true),
            ("2100-02-29", false),
            (" 2024-01-15", false),
            ("2024-01-15 ", false),
            ("2024-00-15", false),
            ("2024-01-00", false),
            ("", false),
            ("2020 -01-01", false),
            ("2020-01-01X", false),
            ("2020-01-01Z", false),
            ("2020-01-01 00:00:00Z", false),
            ("0001-01-01", true),
            ("20-01-01", false),
            ("998-01-01", false),
        ] {
            assert_eq!(parse_full_date(text).is_ok(), valid, "{text:?}");
        }
        assert_eq!(parse_full_date("2020-02-29"), Ok((2020, 2, 29)));
        assert_eq!(parse_full_date("0000-12-31"), Ok((0, 12, 31)));
    }

    /// The `time` cases.
    #[test]
    fn json_schema_time_cases() {
        for (text, valid) in [
            ("08:30:06Z", true),
            ("008:030:006Z", false),
            ("8:3:6Z", false),
            ("8:0030:6Z", false),
            ("23:59:60Z", true),
            ("22:59:60Z", false),
            ("23:58:60Z", false),
            ("23:59:60+00:00", true),
            ("22:59:60+00:00", false),
            ("23:58:60+00:00", false),
            ("01:29:60+01:30", true),
            ("23:29:60+23:30", true),
            ("23:59:60+01:00", false),
            ("23:59:60+00:30", false),
            ("15:59:60-08:00", true),
            ("00:29:60-23:30", true),
            ("23:59:60-01:00", false),
            ("23:59:60-00:30", false),
            ("23:20:50.52Z", true),
            ("08:30:06.283185Z", true),
            ("08:30:06+00:20", true),
            ("08:30:06-08:00", true),
            ("12:34:56-00:00", true),
            ("08:30:06-8:000", false),
            ("08:30:06z", true),
            ("24:00:00Z", false),
            ("00:60:00Z", false),
            ("00:00:61Z", false),
            ("01:02:03+24:00", false),
            ("01:02:03+00:60", false),
            ("01:02:03Z+00:30", false),
            ("08:30:06 PST", false),
            ("01:01:01,1111", false),
            ("12:00:00", false),
            ("12:00:00.52", false),
            ("1\u{9e8}:00:00Z", false),
            ("08:30:06#00:20", false),
            ("ab:cd:ef", false),
            ("2020-11-28T23:55:45Z", false),
            ("08:30:06+0130", false),
            ("08:30:06+01", false),
            ("08:30:06Z\n", false),
            ("24:59:00+01:00", false),
            ("23:60:00+00:01", false),
            ("00:59:59.999999999999999Z", true),
            ("08:30:06.Z", false),
            ("12:00Z", false),
            ("08:30:06,5Z", false),
            (" 08:30:06Z", false),
            // The crate's own pair.
            ("15:59:60-08:01", false),
            ("15:59:60-08:00", true),
        ] {
            assert_eq!(parse_full_time(text, &JSON).is_ok(), valid, "{text:?}");
        }
        let time = parse_full_time("23:20:50.52Z", &JSON).expect("valid");
        assert_eq!(
            (
                time.hour,
                time.minute,
                time.second,
                time.nanos,
                time.offset_minutes
            ),
            (23, 20, 50, 520_000_000, Some(0))
        );
        // The leap-second refusal blames the seconds field under either policy.
        let err = parse_full_time("22:59:60Z", &GTS).expect_err("not 23:59 UTC");
        assert_eq!(err.at(), 6);
        assert_eq!(
            err.kind(),
            Rfc3339ErrorKind::LeapSecond {
                policy: LeapSecondPolicy::MonthEndUtcOnly
            }
        );
        assert!(parse_full_time("23:59:60Z", &GTS).is_ok());
    }

    // ---- where the two readings differ, and the readings neither takes ---------------

    #[test]
    fn the_two_presets_differ_exactly_where_documented() {
        // Space separator: GTS only.
        assert!(parse("2023-11-14 22:13:20Z", &GTS).is_ok());
        assert!(matches!(
            parse("2023-11-14 22:13:20Z", &JSON).unwrap_err().kind(),
            Rfc3339ErrorKind::Unexpected { found: b' ', .. }
        ));
        // Leap second away from a month end: JSON Schema only.
        assert!(parse("2024-02-28T23:59:60Z", &JSON).is_ok());
        assert_eq!(refused("2024-02-28T23:59:60Z"), LEAP_REFUSED);
        // Everything else agrees.
        for text in [
            "1998-12-31T23:59:60Z",
            "1963-06-19t08:30:06.283185z",
            "1985-04-12T00:59:59.999999999999999Z",
            "0000-01-01T00:00:00+23:59",
        ] {
            assert!(parse(text, &GTS).is_ok(), "{text}");
            assert!(parse(text, &JSON).is_ok(), "{text}");
            assert_eq!(parse(text, &GTS), parse(text, &JSON));
        }
        for text in [
            "1998-12-31T23:59:60+01:00",
            "2023-11-14T22:13:20",
            "1985-04-12T23:20:50+01",
            "1990-12-31T15:59:59-24:00",
        ] {
            assert!(parse(text, &GTS).is_err(), "{text}");
            assert!(parse(text, &JSON).is_err(), "{text}");
        }
    }

    #[test]
    fn strict_case_refuses_lowercase_letters_and_accepts_upper() {
        let strict = Rfc3339Options {
            allow_lowercase_t_z: false,
            ..GTS
        };
        assert!(parse("2023-11-14T22:13:20Z", &strict).is_ok());
        assert!(parse("2023-11-14 22:13:20Z", &strict).is_ok());
        assert!(matches!(
            parse("2023-11-14t22:13:20Z", &strict).unwrap_err().kind(),
            Rfc3339ErrorKind::Unexpected { found: b't', .. }
        ));
        assert!(matches!(
            parse("2023-11-14T22:13:20z", &strict).unwrap_err().kind(),
            Rfc3339ErrorKind::Unexpected { found: b'z', .. }
        ));
        assert!(parse_full_time("22:13:20Z", &strict).is_ok());
        assert!(parse_full_time("22:13:20z", &strict).is_err());
    }

    #[test]
    fn optional_offset_reads_an_absent_zone_as_utc() {
        let local = Rfc3339Options {
            require_offset: false,
            ..JSON
        };
        let stamp = parse("2023-11-14T22:13:20", &local).expect("no zone needed");
        assert_eq!(stamp.offset_minutes, None);
        assert_eq!(stamp.to_unix_seconds(), 1_700_000_000);
        assert_eq!(stamp.format_utc().as_deref(), Some("2023-11-14T22:13:20Z"));
        // With a fraction, and with a zone still written.
        assert_eq!(
            parse("2023-11-14T22:13:20.5", &local).expect("valid").nanos,
            500_000_000
        );
        assert_eq!(
            parse("2023-11-14T22:13:20+02:00", &local)
                .expect("valid")
                .offset_minutes,
            Some(120)
        );
        // A leap second with no zone is read at UTC.
        assert!(parse("1998-12-31T23:59:60", &local).is_ok());
        assert!(parse("1998-12-31T22:59:60", &local).is_err());
        // Anything else after the seconds is still trailing.
        assert!(matches!(
            parse("2023-11-14T22:13:20x", &local).unwrap_err().kind(),
            Rfc3339ErrorKind::Trailing { found: b'x' }
        ));
        let time = parse_full_time("22:13:20", &local).expect("no zone needed");
        assert_eq!(time.offset_minutes, None);
        assert!(parse_full_time("23:59:60", &local).is_ok());
        assert!(parse_full_time("22:59:60", &local).is_err());
        // The neighbouring strict reading still refuses.
        assert!(parse("2023-11-14T22:13:20", &JSON).is_err());
    }
}
