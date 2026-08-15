// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The XSD temporal value space: `dateTime`, `date`, `time`, `duration` (with
//! its `dayTimeDuration`/`yearMonthDuration` subtypes), and the Gregorian family
//! (`gYear`, `gMonth`, `gDay`, `gYearMonth`, `gMonthDay`).
//!
//! Comparison follows XSD's **partial order**: dateTime/date/time values carry an
//! optional timezone; a value without a timezone is compared against one with a
//! timezone via the ±14:00 rule, which is **indeterminate** (`None`) in the
//! overlap. `duration` has a two-component (months, seconds) partial order: a pair
//! like `P1M` vs `P30D` is genuinely incomparable. The hand-rolled calendar uses a
//! proleptic Gregorian day count (valid for negative years), no external deps.

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::numeric::{Decimal, align_decimals, decimal_div_raw, decimal_mul_raw, parse_decimal};
use crate::value::XsdError;

/// Maximum timezone offset magnitude in minutes (±14:00).
const MAX_TZ_MIN: i32 = 14 * 60;
/// ±14:00 expressed in seconds, for the no-timezone comparison bound.
const TZ_BOUND_SECS: i128 = 14 * 3600;
const SECS_PER_DAY: i128 = 86_400;

/// `xsd:dateTime`.
#[derive(Debug, Clone)]
pub struct DateTime {
    year: i64,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: Decimal,
    /// Timezone offset in minutes; `None` = no timezone.
    tz: Option<i32>,
}

/// `xsd:date`.
#[derive(Debug, Clone)]
pub struct Date {
    year: i64,
    month: u8,
    day: u8,
    tz: Option<i32>,
}

/// `xsd:time`.
#[derive(Debug, Clone)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: Decimal,
    tz: Option<i32>,
}

/// `xsd:duration` and its `dayTimeDuration`/`yearMonthDuration` subtypes. The value
/// space is the pair (months, seconds); `datatype` records which lexical subtype it
/// was parsed as (for `canonical_lexical` and `datatype()`).
#[derive(Debug, Clone)]
pub struct Duration {
    months: i64,
    seconds: Decimal,
    datatype: XsdDatatype,
}

impl Duration {
    /// The originating XSD datatype (`Duration`/`DayTimeDuration`/`YearMonthDuration`).
    #[must_use]
    pub fn datatype(&self) -> XsdDatatype {
        self.datatype
    }

    /// The months component of the (months, seconds) value pair.
    #[must_use]
    pub fn months(&self) -> i64 {
        self.months
    }

    /// The seconds component (exact, may carry a fractional part) of the (months,
    /// seconds) value pair.
    #[must_use]
    pub fn seconds(&self) -> Decimal {
        self.seconds
    }
}

/// `xsd:gYear`, `xsd:gMonth`, `xsd:gDay`, `xsd:gYearMonth`, `xsd:gMonthDay`.
///
/// Fields absent for a given type are `None`; `datatype` records which of the five
/// Gregorian datatypes this value belongs to.
#[derive(Debug, Clone)]
pub struct Gregorian {
    year: Option<i64>,
    month: Option<u8>,
    day: Option<u8>,
    tz: Option<i32>,
    datatype: XsdDatatype,
}

impl Gregorian {
    /// The originating XSD Gregorian datatype.
    #[must_use]
    pub fn datatype(&self) -> XsdDatatype {
        self.datatype
    }
}

// ── Proleptic Gregorian calendar (Howard Hinnant's algorithm) ────────────────────

/// Days since 1970-01-01 for a proleptic-Gregorian civil date (valid for any year,
/// including negative). `m` in 1..=12, `d` in 1..=31.
fn days_from_civil(y: i64, m: u8, d: u8) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1; // [0,365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Howard Hinnant's `civil_from_days`: the inverse of [`days_from_civil`]. `days` is
/// the day count relative to 1970-01-01 (may be negative). Shared by
/// [`datetime_from_unix_seconds`] and the duration/timezone arithmetic below, which
/// all need to turn an exact day count back into a proleptic-Gregorian date.
///
/// Algorithm reference: <https://howardhinnant.github.io/date_algorithms.html>
fn civil_from_days(days: i64) -> (i64, u8, u8) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 {
        (mp + 3) as u8
    } else {
        (mp - 9) as u8
    };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Convert Unix epoch seconds (UTC) to an xsd:dateTime value using Howard Hinnant's
/// civil-from-days algorithm. Pure math — no clock access, wasm-safe.
/// The result always carries timezone offset 0 (UTC / "Z").
pub fn datetime_from_unix_seconds(secs: i64) -> DateTime {
    const SECS_PER_DAY_I64: i64 = 86_400;
    // Split into day offset + time-of-day.
    let days = if secs >= 0 {
        secs / SECS_PER_DAY_I64
    } else {
        (secs - SECS_PER_DAY_I64 + 1) / SECS_PER_DAY_I64
    };
    let tod = secs - days * SECS_PER_DAY_I64; // 0 .. 86399

    let (y, m, d) = civil_from_days(days);

    let hour = (tod / 3600) as u8;
    let minute = ((tod % 3600) / 60) as u8;
    let second_whole = i128::from(tod % 60);
    let second = Decimal::from_parts(second_whole, 0);

    DateTime {
        year: y,
        month: m,
        day: d,
        hour,
        minute,
        second,
        tz: Some(0),
    }
}

/// Return the Unix epoch (1970-01-01T00:00:00Z) as an xsd:dateTime value.
/// Useful as the compile-time-safe "now" fallback for wasm32 targets.
pub fn datetime_epoch() -> DateTime {
    datetime_from_unix_seconds(0)
}

// ── Parsing ──────────────────────────────────────────────────────────────────────

fn invalid(dt: XsdDatatype, lexical: &str, reason: &'static str) -> XsdError {
    XsdError::InvalidLexical {
        datatype: dt,
        lexical: lexical.to_string(),
        reason,
    }
}

/// Split a trailing timezone (`Z`, `+hh:mm`, `-hh:mm`) off the time portion. Returns
/// `(body_without_tz, tz_minutes_option)`.
fn split_tz(dt: XsdDatatype, lexical: &str, s: &str) -> Result<(String, Option<i32>), XsdError> {
    if let Some(body) = s.strip_suffix('Z') {
        return Ok((body.to_string(), Some(0)));
    }
    // A tz sign is the last '+' or '-' AND must look like "±hh:mm" (len 6).
    if s.len() >= 6 {
        let tail = &s[s.len() - 6..];
        let sign = tail.as_bytes()[0];
        if (sign == b'+' || sign == b'-') && tail.as_bytes()[3] == b':' {
            let hh: i32 = tail[1..3]
                .parse()
                .map_err(|_| invalid(dt, lexical, "bad timezone hour"))?;
            let mm: i32 = tail[4..6]
                .parse()
                .map_err(|_| invalid(dt, lexical, "bad timezone minute"))?;
            if hh > 14 || mm > 59 {
                return Err(invalid(dt, lexical, "timezone out of range"));
            }
            let mut off = hh * 60 + mm;
            if sign == b'-' {
                off = -off;
            }
            if off.abs() > MAX_TZ_MIN {
                return Err(invalid(dt, lexical, "timezone exceeds ±14:00"));
            }
            return Ok((s[..s.len() - 6].to_string(), Some(off)));
        }
    }
    Ok((s.to_string(), None))
}

/// Number of days in a given month for a proleptic-Gregorian year.
/// Uses the signed year directly; negative years follow the same leap-year rule as
/// positive ones (proleptic Gregorian: leap iff divisible by 4, except centuries
/// unless also divisible by 400).
fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            if is_leap { 29 } else { 28 }
        }
        _ => 0, // invalid month — caught by caller before reaching here
    }
}

/// Parse `[-]YYYY[Y...]-MM-DD` into `(year, month, day)`.
fn parse_ymd(dt: XsdDatatype, lexical: &str, s: &str) -> Result<(i64, u8, u8), XsdError> {
    let neg = s.starts_with('-');
    let body = if neg { &s[1..] } else { s };
    let Some((year_text, month_day)) = body.split_once('-') else {
        return Err(invalid(dt, lexical, "expected YYYY-MM-DD"));
    };
    let Some((month_text, day_text)) = month_day.split_once('-') else {
        return Err(invalid(dt, lexical, "expected YYYY-MM-DD"));
    };
    if day_text.contains('-') {
        return Err(invalid(dt, lexical, "expected YYYY-MM-DD"));
    }
    if year_text.len() < 4 || month_text.len() != 2 || day_text.len() != 2 {
        return Err(invalid(dt, lexical, "bad date field widths"));
    }
    // XSD 1.1 §3.3.7: a year wider than 4 digits must not have a leading zero.
    // Exactly 4 digits with a leading zero (e.g. "0044", "0000") are valid.
    if year_text.len() > 4 && year_text.starts_with('0') {
        return Err(invalid(
            dt,
            lexical,
            "year wider than 4 digits must not have a leading zero",
        ));
    }
    let year_mag: i64 = year_text
        .parse()
        .map_err(|_| invalid(dt, lexical, "bad year"))?;
    let month: u8 = month_text
        .parse()
        .map_err(|_| invalid(dt, lexical, "bad month"))?;
    let day: u8 = day_text
        .parse()
        .map_err(|_| invalid(dt, lexical, "bad day"))?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(invalid(dt, lexical, "month/day out of range"));
    }
    let year = if neg { -year_mag } else { year_mag };
    if day > days_in_month(year, month) {
        return Err(invalid(dt, lexical, "day out of range for month"));
    }
    Ok((year, month, day))
}

/// Parse `hh:mm:ss(.fff)?` into `(hour, minute, second)`.
fn parse_hms(dt: XsdDatatype, lexical: &str, s: &str) -> Result<(u8, u8, Decimal), XsdError> {
    let Some((hour_text, minute_second)) = s.split_once(':') else {
        return Err(invalid(dt, lexical, "expected hh:mm:ss"));
    };
    let Some((minute_text, second_text)) = minute_second.split_once(':') else {
        return Err(invalid(dt, lexical, "expected hh:mm:ss"));
    };
    if second_text.contains(':') {
        return Err(invalid(dt, lexical, "expected hh:mm:ss"));
    }
    if hour_text.len() != 2 || minute_text.len() != 2 {
        return Err(invalid(dt, lexical, "bad time field widths"));
    }
    let hour: u8 = hour_text
        .parse()
        .map_err(|_| invalid(dt, lexical, "bad hour"))?;
    let minute: u8 = minute_text
        .parse()
        .map_err(|_| invalid(dt, lexical, "bad minute"))?;
    // Reject a trailing-dot seconds lexical (e.g. "00.") — parse_decimal accepts it
    // as a valid decimal ("1.0") but it is not a valid XSD time seconds field.
    if second_text.ends_with('.') {
        return Err(invalid(dt, lexical, "seconds has trailing decimal point"));
    }
    // Reject a leading sign in the seconds field — seconds must be non-negative.
    if second_text.starts_with('-') || second_text.starts_with('+') {
        return Err(invalid(dt, lexical, "seconds must not have a sign"));
    }
    let second = parse_decimal(second_text).map_err(|_| invalid(dt, lexical, "bad second"))?;
    // XSD has no leap seconds: seconds must be in [0, 60). Whole part >= 60 is invalid.
    if second.whole_part() >= 60 {
        return Err(invalid(dt, lexical, "seconds out of range (must be < 60)"));
    }
    if minute > 59 {
        return Err(invalid(dt, lexical, "minute out of range"));
    }
    if hour > 24 {
        return Err(invalid(dt, lexical, "hour out of range"));
    }
    // Hour 24 is only valid as exactly 24:00:00 (end-of-day sentinel).
    if hour == 24 && (minute != 0 || !second.is_zero()) {
        return Err(invalid(dt, lexical, "hour 24 is only valid as 24:00:00"));
    }
    Ok((hour, minute, second))
}

/// `xsd:dateTime` = `date 'T' time tz?`.
pub fn parse_datetime(s: &str) -> Result<DateTime, XsdError> {
    let dt = XsdDatatype::DateTime;
    let (date_part, time_part) = s
        .split_once('T')
        .ok_or_else(|| invalid(dt, s, "missing 'T'"))?;
    let (time_no_tz, tz) = split_tz(dt, s, time_part)?;
    let (year, month, day) = parse_ymd(dt, s, date_part)?;
    let (hour, minute, second) = parse_hms(dt, s, &time_no_tz)?;
    Ok(DateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        tz,
    })
}

/// `xsd:date` = `date tz?`.
pub fn parse_date(s: &str) -> Result<Date, XsdError> {
    let dt = XsdDatatype::Date;
    let (body, tz) = split_tz(dt, s, s)?;
    let (year, month, day) = parse_ymd(dt, s, &body)?;
    Ok(Date {
        year,
        month,
        day,
        tz,
    })
}

/// `xsd:time` = `time tz?`.
pub fn parse_time(s: &str) -> Result<Time, XsdError> {
    let dt = XsdDatatype::Time;
    let (body, tz) = split_tz(dt, s, s)?;
    let (hour, minute, second) = parse_hms(dt, s, &body)?;
    Ok(Time {
        hour,
        minute,
        second,
        tz,
    })
}

/// `xsd:duration` and subtypes: `[-]PnYnMnDTnHnMnS` (any component group optional,
/// at least one present; the `T` separates date from time components).
pub fn parse_duration(dt: XsdDatatype, s: &str) -> Result<Duration, XsdError> {
    let neg = s.starts_with('-');
    let body = s.strip_prefix('-').unwrap_or(s);
    let body = body
        .strip_prefix('P')
        .ok_or_else(|| invalid(dt, s, "duration must start with 'P'"))?;
    let (date_part, time_part) = match body.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (body, None),
    };

    let mut months: i64 = 0;
    let mut seconds = 0i128; // whole seconds accumulator
    let mut sec_frac = Decimal::from_parts(0, 0);
    let mut any = false;

    // Date components: nY nM nD.
    let mut num = String::new();
    for ch in date_part.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else {
            let n: i64 = num
                .parse()
                .map_err(|_| invalid(dt, s, "bad duration number"))?;
            num.clear();
            any = true;
            match ch {
                'Y' => months += n * 12,
                'M' => months += n,
                'D' => seconds += i128::from(n) * SECS_PER_DAY,
                _ => return Err(invalid(dt, s, "bad duration date component")),
            }
        }
    }
    if !num.is_empty() {
        return Err(invalid(dt, s, "dangling number in duration date part"));
    }

    // Time components: nH nM n(.f)S.
    if let Some(time_part) = time_part {
        let mut tnum = String::new();
        for (i, ch) in time_part.char_indices() {
            if ch.is_ascii_digit() || ch == '.' {
                tnum.push(ch);
            } else {
                any = true;
                match ch {
                    'H' => {
                        seconds += i128::from(
                            tnum.parse::<i64>()
                                .map_err(|_| invalid(dt, s, "bad hours"))?,
                        ) * 3600;
                    }
                    'M' => {
                        seconds += i128::from(
                            tnum.parse::<i64>()
                                .map_err(|_| invalid(dt, s, "bad minutes"))?,
                        ) * 60;
                    }
                    'S' => {
                        let d = parse_decimal(&tnum).map_err(|_| invalid(dt, s, "bad seconds"))?;
                        seconds += d.whole_part();
                        sec_frac = d.frac_part();
                        if i != time_part.len() - 1 {
                            return Err(invalid(dt, s, "'S' must be last"));
                        }
                    }
                    _ => return Err(invalid(dt, s, "bad duration time component")),
                }
                tnum.clear();
            }
        }
        if !tnum.is_empty() {
            return Err(invalid(dt, s, "dangling number in duration time part"));
        }
    }
    if !any {
        return Err(invalid(dt, s, "duration has no components"));
    }

    // Combine whole + fractional seconds into one Decimal at the fraction's scale.
    let scale = sec_frac.scale();
    let combined = seconds
        .checked_mul(10i128.pow(u32::from(scale)))
        .and_then(|w| w.checked_add(sec_frac.mantissa()))
        .ok_or_else(|| XsdError::OutOfRange {
            datatype: dt,
            lexical: s.to_string(),
            reason: "duration seconds overflow",
        })?;
    let mut total_secs = Decimal::from_parts(combined, scale);
    if neg {
        months = -months;
        total_secs = Decimal::from_parts(-total_secs.mantissa(), total_secs.scale());
    }
    Ok(Duration {
        months,
        seconds: total_secs,
        datatype: dt,
    })
}

// ── Gregorian family parsing ─────────────────────────────────────────────────────

/// Parse a year part `[-]YYYY[Y...]` (no trailing components).
/// Returns `(year_magnitude_with_sign, remaining_str_after_year_digits)`.
/// The year must be ≥4 digits; >4 digits must not have a leading zero.
fn parse_year_str<'a>(
    dt: XsdDatatype,
    lexical: &str,
    s: &'a str,
) -> Result<(i64, &'a str), XsdError> {
    let neg = s.starts_with('-');
    let digits_start = usize::from(neg);
    let rest = &s[digits_start..];
    // Find how many leading ASCII digits there are.
    let n_digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if n_digits < 4 {
        return Err(invalid(dt, lexical, "year must be at least 4 digits"));
    }
    let year_digits = &rest[..n_digits];
    if n_digits > 4 && year_digits.starts_with('0') {
        return Err(invalid(
            dt,
            lexical,
            "year wider than 4 digits must not have a leading zero",
        ));
    }
    let year_mag: i64 = year_digits
        .parse()
        .map_err(|_| invalid(dt, lexical, "bad year digits"))?;
    let year = if neg { -year_mag } else { year_mag };
    let after = &rest[n_digits..];
    Ok((year, after))
}

/// Max days per month with February = 29 (no year available; allow Feb 29).
/// Index 0 = January, index 11 = December.
const MONTH_MAX_DAYS_LEAP: [u8; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Parse a 2-digit month string `MM`, returning the month value (1–12).
fn parse_month_field(dt: XsdDatatype, lexical: &str, s: &str) -> Result<u8, XsdError> {
    if s.len() != 2 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(dt, lexical, "month must be exactly 2 digits"));
    }
    let m: u8 = s
        .parse()
        .map_err(|_| invalid(dt, lexical, "bad month digits"))?;
    if !(1..=12).contains(&m) {
        return Err(invalid(dt, lexical, "month out of range (01-12)"));
    }
    Ok(m)
}

/// Parse a 2-digit day string `DD`, returning the day value (1–31).
fn parse_day_field(dt: XsdDatatype, lexical: &str, s: &str) -> Result<u8, XsdError> {
    if s.len() != 2 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(dt, lexical, "day must be exactly 2 digits"));
    }
    let d: u8 = s
        .parse()
        .map_err(|_| invalid(dt, lexical, "bad day digits"))?;
    if !(1..=31).contains(&d) {
        return Err(invalid(dt, lexical, "day out of range (01-31)"));
    }
    Ok(d)
}

/// Dispatch parser for all five Gregorian datatypes.
pub fn parse_gregorian(datatype: XsdDatatype, lexical: &str) -> Result<Gregorian, XsdError> {
    let dt = datatype;
    match dt {
        XsdDatatype::GYear => {
            // [-]YYYY[Y...][tz]
            let (body, tz) = split_tz(dt, lexical, lexical)?;
            let (year, after) = parse_year_str(dt, lexical, &body)?;
            if !after.is_empty() {
                return Err(invalid(dt, lexical, "unexpected content after gYear"));
            }
            Ok(Gregorian {
                year: Some(year),
                month: None,
                day: None,
                tz,
                datatype: dt,
            })
        }
        XsdDatatype::GMonth => {
            // --MM[tz]
            let (body, tz) = split_tz(dt, lexical, lexical)?;
            let s = body
                .strip_prefix("--")
                .ok_or_else(|| invalid(dt, lexical, "gMonth must start with '--'"))?;
            // After stripping "--", s must be exactly "MM" (2 digits)
            if s.len() != 2 {
                return Err(invalid(dt, lexical, "gMonth must be '--MM'"));
            }
            let month = parse_month_field(dt, lexical, s)?;
            Ok(Gregorian {
                year: None,
                month: Some(month),
                day: None,
                tz,
                datatype: dt,
            })
        }
        XsdDatatype::GDay => {
            // ---DD[tz]
            let (body, tz) = split_tz(dt, lexical, lexical)?;
            let s = body
                .strip_prefix("---")
                .ok_or_else(|| invalid(dt, lexical, "gDay must start with '---'"))?;
            if s.len() != 2 {
                return Err(invalid(dt, lexical, "gDay must be '---DD'"));
            }
            let day = parse_day_field(dt, lexical, s)?;
            Ok(Gregorian {
                year: None,
                month: None,
                day: Some(day),
                tz,
                datatype: dt,
            })
        }
        XsdDatatype::GYearMonth => {
            // [-]YYYY[Y...]-MM[tz]
            let (body, tz) = split_tz(dt, lexical, lexical)?;
            let (year, after) = parse_year_str(dt, lexical, &body)?;
            // after must be "-MM"
            let mm_str = after
                .strip_prefix('-')
                .ok_or_else(|| invalid(dt, lexical, "gYearMonth: expected '-MM' after year"))?;
            if mm_str.len() != 2 {
                return Err(invalid(
                    dt,
                    lexical,
                    "gYearMonth: month part must be 2 digits",
                ));
            }
            let month = parse_month_field(dt, lexical, mm_str)?;
            Ok(Gregorian {
                year: Some(year),
                month: Some(month),
                day: None,
                tz,
                datatype: dt,
            })
        }
        XsdDatatype::GMonthDay => {
            // --MM-DD[tz]
            let (body, tz) = split_tz(dt, lexical, lexical)?;
            let s = body
                .strip_prefix("--")
                .ok_or_else(|| invalid(dt, lexical, "gMonthDay must start with '--'"))?;
            // s must be "MM-DD" — exactly 5 chars
            if s.len() != 5 || s.as_bytes()[2] != b'-' {
                return Err(invalid(dt, lexical, "gMonthDay must be '--MM-DD'"));
            }
            let month = parse_month_field(dt, lexical, &s[..2])?;
            let day = parse_day_field(dt, lexical, &s[3..5])?;
            // Validate day against month; use leap reference (Feb max = 29).
            let max_day = MONTH_MAX_DAYS_LEAP[(month - 1) as usize];
            if day > max_day {
                return Err(invalid(dt, lexical, "day out of range for month"));
            }
            Ok(Gregorian {
                year: None,
                month: Some(month),
                day: Some(day),
                tz,
                datatype: dt,
            })
        }
        _ => Err(invalid(dt, lexical, "not a Gregorian datatype")),
    }
}

// ── Comparison (XSD partial order) ───────────────────────────────────────────────

/// The naive whole-seconds offset (timezone NOT applied) on the proleptic timeline.
fn naive_secs(days: i64, hour: u8, minute: u8, sec_whole: i128) -> i128 {
    i128::from(days) * SECS_PER_DAY + i128::from(hour) * 3600 + i128::from(minute) * 60 + sec_whole
}

/// Compare two `(whole_secs, frac)` points.
fn cmp_point(a_whole: i128, a_frac: &Decimal, b_whole: i128, b_frac: &Decimal) -> Ordering {
    a_whole.cmp(&b_whole).then_with(|| a_frac.cmp_exact(b_frac))
}

/// The XSD partial-order comparison for a timezoned point: naive whole seconds +
/// fractional seconds + optional timezone (minutes). `None` = indeterminate.
fn cmp_timeline(
    a_naive: i128,
    a_frac: &Decimal,
    a_tz: Option<i32>,
    b_naive: i128,
    b_frac: &Decimal,
    b_tz: Option<i32>,
) -> Option<Ordering> {
    match (a_tz, b_tz) {
        (Some(ta), Some(tb)) => Some(cmp_point(
            a_naive - i128::from(ta) * 60,
            a_frac,
            b_naive - i128::from(tb) * 60,
            b_frac,
        )),
        (None, None) => Some(cmp_point(a_naive, a_frac, b_naive, b_frac)),
        (None, Some(tb)) => {
            let b_utc = b_naive - i128::from(tb) * 60;
            // a's UTC instant lies in [a_naive - 14h, a_naive + 14h].
            if cmp_point(a_naive + TZ_BOUND_SECS, a_frac, b_utc, b_frac) == Ordering::Less {
                Some(Ordering::Less)
            } else if cmp_point(a_naive - TZ_BOUND_SECS, a_frac, b_utc, b_frac) == Ordering::Greater
            {
                Some(Ordering::Greater)
            } else {
                None
            }
        }
        (Some(_), None) => {
            cmp_timeline(b_naive, b_frac, b_tz, a_naive, a_frac, a_tz).map(Ordering::reverse)
        }
    }
}

/// Compare two `dateTime` values (XSD partial order).
#[must_use]
pub fn cmp_datetime(a: &DateTime, b: &DateTime) -> Option<Ordering> {
    let an = naive_secs(
        days_from_civil(a.year, a.month, a.day),
        a.hour,
        a.minute,
        a.second.whole_part(),
    );
    let bn = naive_secs(
        days_from_civil(b.year, b.month, b.day),
        b.hour,
        b.minute,
        b.second.whole_part(),
    );
    cmp_timeline(
        an,
        &a.second.frac_part(),
        a.tz,
        bn,
        &b.second.frac_part(),
        b.tz,
    )
}

/// Compare two `date` values (XSD partial order; midnight on the proleptic timeline).
#[must_use]
pub fn cmp_date(a: &Date, b: &Date) -> Option<Ordering> {
    let zero = Decimal::from_parts(0, 0);
    let an = naive_secs(days_from_civil(a.year, a.month, a.day), 0, 0, 0);
    let bn = naive_secs(days_from_civil(b.year, b.month, b.day), 0, 0, 0);
    cmp_timeline(an, &zero, a.tz, bn, &zero, b.tz)
}

/// Compare two `time` values (XSD partial order; within a single notional day).
#[must_use]
pub fn cmp_time(a: &Time, b: &Time) -> Option<Ordering> {
    let an = naive_secs(0, a.hour, a.minute, a.second.whole_part());
    let bn = naive_secs(0, b.hour, b.minute, b.second.whole_part());
    cmp_timeline(
        an,
        &a.second.frac_part(),
        a.tz,
        bn,
        &b.second.frac_part(),
        b.tz,
    )
}

/// Compare two `duration` values: a two-component partial order over (months,
/// seconds). Agreement on both components gives the order; disagreement is
/// indeterminate (`None`). Totally-ordered subtypes (`dayTimeDuration` with
/// months = 0, `yearMonthDuration` with seconds = 0) always resolve.
///
/// ## Chosen `=` semantics for cross-subtype pairs
///
/// The value space is the pair `(months, seconds)` regardless of which lexical
/// subtype the duration was parsed as. Cross-subtype pairs with zero in the
/// "other" component are therefore **comparable** at the value level:
/// - `"P0M"^^yearMonthDuration` has `(months=0, seconds=0)`.
/// - `"PT0S"^^dayTimeDuration` has `(months=0, seconds=0)`.
///
/// Both reduce to the zero pair → `cmp_duration` returns `Some(Equal)`.
///
/// Non-zero cross-subtype pairs (e.g. `"P1Y"` vs `"P1D"`) disagree on at least
/// one component → `None` (genuinely incomparable per XSD §3.6.5).
#[must_use]
pub fn cmp_duration(a: &Duration, b: &Duration) -> Option<Ordering> {
    let m = a.months.cmp(&b.months);
    let s = a.seconds.cmp_exact(&b.seconds);
    match (m, s) {
        (Ordering::Equal, Ordering::Equal) => Some(Ordering::Equal),
        (Ordering::Equal, o) | (o, Ordering::Equal) => Some(o),
        (a, b) if a == b => Some(a),
        _ => None,
    }
}

/// Compare two Gregorian values (XSD partial order).
///
/// Different Gregorian types are **incomparable** (`None`): comparing a `gYear` to a
/// `gMonth` is a SPARQL type error, not a numeric comparison.
///
/// For values of the same type, absent fields are filled with reference defaults
/// anchored to 2000-01-01 — a **leap** year chosen so that `--02-29` comparisons are
/// well-defined. The reference: year=2000, month=1, day=1.  Using a leap year for the
/// reference ensures `--02-29` maps to a valid calendar date and thus participates in
/// the timeline correctly.
///
/// The resulting naive-second offset is then fed into `cmp_timeline` with the values'
/// timezone offsets, giving XSD's tz-indeterminate partial order for free.
#[must_use]
pub fn cmp_gregorian(a: &Gregorian, b: &Gregorian) -> Option<Ordering> {
    if a.datatype != b.datatype {
        return None;
    }
    let zero = Decimal::from_parts(0, 0);
    // Reference: year 2000 (leap), month 1, day 1.
    const REF_YEAR: i64 = 2000;
    const REF_MONTH: u8 = 1;
    const REF_DAY: u8 = 1;

    let ay = a.year.unwrap_or(REF_YEAR);
    let am = a.month.unwrap_or(REF_MONTH);
    let ad = a.day.unwrap_or(REF_DAY);

    let by = b.year.unwrap_or(REF_YEAR);
    let bm = b.month.unwrap_or(REF_MONTH);
    let bd = b.day.unwrap_or(REF_DAY);

    let an = naive_secs(days_from_civil(ay, am, ad), 0, 0, 0);
    let bn = naive_secs(days_from_civil(by, bm, bd), 0, 0, 0);
    cmp_timeline(an, &zero, a.tz, bn, &zero, b.tz)
}

// ── Shared exact-arithmetic helpers ───────────────────────────────────────────────

/// Build an `OutOfRange` error for an arithmetic overflow with no offending lexical
/// (the operands are values, not lexicals, at this layer).
fn arith_overflow(datatype: XsdDatatype, reason: &'static str) -> XsdError {
    XsdError::OutOfRange {
        datatype,
        lexical: String::new(),
        reason,
    }
}

/// Exact `a + b` for two seconds-shaped decimals of possibly different scales.
fn decimal_add_exact(datatype: XsdDatatype, a: &Decimal, b: &Decimal) -> Result<Decimal, XsdError> {
    let (am, bm, scale) = align_decimals(a, b);
    let sum = am
        .checked_add(bm)
        .ok_or_else(|| arith_overflow(datatype, "decimal addition overflow"))?;
    Ok(Decimal::from_parts(sum, scale))
}

/// Exact `-a`. Fails only for the unrepresentable `i128::MIN` mantissa.
fn decimal_negate(datatype: XsdDatatype, a: &Decimal) -> Result<Decimal, XsdError> {
    a.mantissa()
        .checked_neg()
        .map(|m| Decimal::from_parts(m, a.scale()))
        .ok_or_else(|| {
            arith_overflow(
                datatype,
                "decimal negation overflow (mantissa is i128::MIN)",
            )
        })
}

/// Exact `a - b`, built from [`decimal_add_exact`] + [`decimal_negate`].
fn decimal_sub_exact(datatype: XsdDatatype, a: &Decimal, b: &Decimal) -> Result<Decimal, XsdError> {
    decimal_add_exact(datatype, a, &decimal_negate(datatype, b)?)
}

/// Round a `Decimal` to the nearest `i64`, ties toward positive infinity — the same
/// rule `numeric::numeric_round` applies to `fn:round` (XPath §4.4.5), reused here for
/// `xs:yearMonthDuration` multiply/divide, whose result must be an integer number of
/// months. Errors if the rounded value does not fit `i64`.
fn round_decimal_to_i64(datatype: XsdDatatype, d: &Decimal) -> Result<i64, XsdError> {
    let whole = d.whole_part();
    let rounded = if d.scale() == 0 {
        whole
    } else {
        let frac_m = d.frac_part().mantissa();
        let threshold = 5i128 * 10i128.pow(u32::from(d.scale()) - 1);
        if frac_m >= threshold {
            whole + 1
        } else if frac_m < -threshold {
            whole - 1
        } else {
            whole
        }
    };
    i64::try_from(rounded).map_err(|_| arith_overflow(datatype, "duration months overflow"))
}

/// Add a `months_delta` to a (year, month) pair, per proleptic-Gregorian month
/// arithmetic (no day component — the caller clamps the day separately, XML Schema
/// Appendix E). Exact; errors only on genuine `i64` year overflow.
fn shift_year_month(
    datatype: XsdDatatype,
    year: i64,
    month: u8,
    months_delta: i64,
) -> Result<(i64, u8), XsdError> {
    let total = i128::from(year)
        .checked_mul(12)
        .and_then(|v| v.checked_add(i128::from(month) - 1))
        .and_then(|v| v.checked_add(i128::from(months_delta)))
        .ok_or_else(|| arith_overflow(datatype, "year-month arithmetic overflow"))?;
    let new_year = total.div_euclid(12);
    let new_month = (total.rem_euclid(12) + 1) as u8;
    let new_year =
        i64::try_from(new_year).map_err(|_| arith_overflow(datatype, "year overflow"))?;
    Ok((new_year, new_month))
}

/// A calendar date/time's naive local fields, grouped into one value so the
/// arithmetic helpers below stay within clippy's `too_many_arguments` budget.
struct CalendarPoint {
    year: i64,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: Decimal,
}

impl CalendarPoint {
    /// A date at midnight (for date-only arithmetic, whose time-of-day fields are
    /// discarded by the caller after [`add_seconds_decimal`] returns).
    fn midnight(year: i64, month: u8, day: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: Decimal::from_parts(0, 0),
        }
    }

    /// A time-of-day paired with an arbitrary reference date (for time-only
    /// arithmetic, whose date fields are discarded by the caller).
    fn on_reference_date(hour: u8, minute: u8, second: Decimal) -> Self {
        Self {
            year: 2000,
            month: 1,
            day: 1,
            hour,
            minute,
            second,
        }
    }
}

/// Add an exact seconds-shaped `delta` (a `Decimal`, e.g. a `dayTimeDuration`'s
/// seconds component, or a whole-minute timezone shift) to a calendar date/time,
/// returning the resulting `(year, month, day, hour, minute, second)`. The delta may
/// be negative and of any magnitude representable within the `Decimal`/`i64` domain;
/// day/month/year all roll over correctly. Exact — no float rounding, and no panics:
/// every intermediate step is `checked_*` and overflow maps to `OutOfRange`.
fn add_seconds_decimal(
    datatype: XsdDatatype,
    point: &CalendarPoint,
    delta: &Decimal,
) -> Result<(i64, u8, u8, u8, u8, Decimal), XsdError> {
    let CalendarPoint {
        year,
        month,
        day,
        hour,
        minute,
        second,
    } = *point;
    let second = &second;
    let scale = second.scale().max(delta.scale());
    let unit = 10i128.pow(u32::from(scale));
    let scale_up = |d: &Decimal| -> Result<i128, XsdError> {
        let diff = scale - d.scale();
        if diff == 0 {
            Ok(d.mantissa())
        } else {
            d.mantissa()
                .checked_mul(10i128.pow(u32::from(diff)))
                .ok_or_else(|| arith_overflow(datatype, "datetime arithmetic overflow"))
        }
    };

    let second_scaled = scale_up(second)?;
    let base_secs = i128::from(hour) * 3600 + i128::from(minute) * 60;
    let base_scaled = base_secs
        .checked_mul(unit)
        .and_then(|v| v.checked_add(second_scaled))
        .ok_or_else(|| arith_overflow(datatype, "datetime arithmetic overflow"))?;
    let delta_scaled = scale_up(delta)?;
    let sum_scaled = base_scaled
        .checked_add(delta_scaled)
        .ok_or_else(|| arith_overflow(datatype, "datetime arithmetic overflow"))?;

    let day_unit = unit
        .checked_mul(SECS_PER_DAY)
        .ok_or_else(|| arith_overflow(datatype, "datetime arithmetic overflow"))?;
    let day_delta = sum_scaled.div_euclid(day_unit);
    let remainder = sum_scaled.rem_euclid(day_unit);

    let tod_secs = remainder / unit; // whole seconds of day, in [0, 86400)
    let hour_new = (tod_secs / 3600) as u8;
    let minute_new = ((tod_secs % 3600) / 60) as u8;
    let sec_of_minute_scaled = remainder % (unit * 60);
    let second_new = Decimal::from_parts(sec_of_minute_scaled, scale);

    let base_days = i128::from(days_from_civil(year, month, day));
    let total_days = base_days
        .checked_add(day_delta)
        .ok_or_else(|| arith_overflow(datatype, "datetime arithmetic overflow"))?;
    let total_days =
        i64::try_from(total_days).map_err(|_| arith_overflow(datatype, "date overflow"))?;
    let (y, m, d) = civil_from_days(total_days);

    Ok((y, m, d, hour_new, minute_new, second_new))
}

/// One side of an [`instant_diff`] subtraction: days-since-epoch, time-of-day, and
/// timezone.
struct Instant {
    days: i64,
    hour: u8,
    minute: u8,
    second: Decimal,
    tz: Option<i32>,
}

/// The exact instant difference `a - b` as a `dayTimeDuration` `Duration`. Mirrors
/// the XSD partial order's tz-indeterminacy rule (see [`cmp_timeline`]): mixing a
/// timezoned and an untimezoned operand has no well-defined instant difference, so —
/// rather than guessing an implicit timezone this crate has no execution context to
/// supply — that case is a hard `TypeMismatch`, not a best-effort answer.
/// Both-timezoned and both-untimezoned pairs are always determinate.
fn instant_diff(datatype: XsdDatatype, a: &Instant, b: &Instant) -> Result<Duration, XsdError> {
    if a.tz.is_some() != b.tz.is_some() {
        return Err(XsdError::TypeMismatch {
            reason: "subtract: indeterminate timezone mix (one operand has a timezone, the other does not)",
        });
    }
    let scale = a.second.scale().max(b.second.scale());
    let unit = 10i128.pow(u32::from(scale));
    let scale_up = |d: &Decimal| -> Result<i128, XsdError> {
        let diff = scale - d.scale();
        if diff == 0 {
            Ok(d.mantissa())
        } else {
            d.mantissa()
                .checked_mul(10i128.pow(u32::from(diff)))
                .ok_or_else(|| arith_overflow(datatype, "instant subtraction overflow"))
        }
    };
    let overflow = || arith_overflow(datatype, "instant subtraction overflow");

    let a_second_scaled = scale_up(&a.second)?;
    let b_second_scaled = scale_up(&b.second)?;
    let a_tz_secs = i128::from(a.tz.unwrap_or(0)) * 60;
    let b_tz_secs = i128::from(b.tz.unwrap_or(0)) * 60;
    let a_naive =
        i128::from(a.days) * SECS_PER_DAY + i128::from(a.hour) * 3600 + i128::from(a.minute) * 60
            - a_tz_secs;
    let b_naive =
        i128::from(b.days) * SECS_PER_DAY + i128::from(b.hour) * 3600 + i128::from(b.minute) * 60
            - b_tz_secs;
    let a_total = a_naive
        .checked_mul(unit)
        .and_then(|v| v.checked_add(a_second_scaled))
        .ok_or_else(overflow)?;
    let b_total = b_naive
        .checked_mul(unit)
        .and_then(|v| v.checked_add(b_second_scaled))
        .ok_or_else(overflow)?;
    let diff = a_total.checked_sub(b_total).ok_or_else(overflow)?;

    Ok(Duration {
        months: 0,
        seconds: Decimal::from_parts(diff, scale),
        datatype: XsdDatatype::DayTimeDuration,
    })
}

/// Require `dur` to be declared `xsd:yearMonthDuration` (not the general
/// `xsd:duration`, whose seconds component is not statically known to be zero).
fn require_year_month_duration(dur: &Duration) -> Result<(), XsdError> {
    if dur.datatype == XsdDatatype::YearMonthDuration {
        Ok(())
    } else {
        Err(XsdError::TypeMismatch {
            reason: "operation requires a yearMonthDuration operand",
        })
    }
}

/// Require `dur` to be declared `xsd:dayTimeDuration` (not the general
/// `xsd:duration`, whose months component is not statically known to be zero).
fn require_day_time_duration(dur: &Duration) -> Result<(), XsdError> {
    if dur.datatype == XsdDatatype::DayTimeDuration {
        Ok(())
    } else {
        Err(XsdError::TypeMismatch {
            reason: "operation requires a dayTimeDuration operand",
        })
    }
}

// ── Timezone adjustment (XPath F&O §9.6) ──────────────────────────────────────────

/// Validate and normalize an `fn:adjust-*-to-timezone` `$timezone` argument, given in
/// seconds (the natural unit of a `dayTimeDuration`, which is what F&O types this
/// parameter as). `None` means "remove the timezone". A `Some` value must be an exact
/// whole number of minutes with magnitude `<= 14:00`, matching the lexical timezone
/// grammar this crate already enforces in `split_tz`.
fn validate_timezone_seconds(
    datatype: XsdDatatype,
    secs: Option<i64>,
) -> Result<Option<i32>, XsdError> {
    let Some(secs) = secs else {
        return Ok(None);
    };
    if secs % 60 != 0 {
        return Err(XsdError::InvalidLexical {
            datatype,
            lexical: secs.to_string(),
            reason: "timezone must be a whole number of minutes",
        });
    }
    let minutes = secs / 60;
    if minutes.abs() > i64::from(MAX_TZ_MIN) {
        return Err(XsdError::InvalidLexical {
            datatype,
            lexical: secs.to_string(),
            reason: "timezone exceeds ±14:00",
        });
    }
    // SAFETY: |minutes| <= MAX_TZ_MIN (840), well within i32.
    Ok(Some(minutes as i32))
}

/// `fn:adjust-dateTime-to-timezone($input, $timezone)` (XPath F&O §9.6.1). `timezone`
/// is given in seconds (a `dayTimeDuration` magnitude); `None` removes the timezone.
/// If `dt` already has a timezone and `timezone` is `Some`, the local fields are
/// shifted so the result denotes the **same instant**, expressed in the new offset
/// (day/month/year may roll over). If `dt` has no timezone, `timezone` is attached
/// with no shift — the local fields are left as-is.
pub fn adjust_datetime_to_timezone(
    dt: &DateTime,
    timezone: Option<i64>,
) -> Result<DateTime, XsdError> {
    let new_tz = validate_timezone_seconds(XsdDatatype::DateTime, timezone)?;
    let retagged = |tz| DateTime {
        year: dt.year,
        month: dt.month,
        day: dt.day,
        hour: dt.hour,
        minute: dt.minute,
        second: dt.second,
        tz,
    };
    match (dt.tz, new_tz) {
        (_, None) => Ok(retagged(None)),
        (None, Some(tz)) => Ok(retagged(Some(tz))),
        (Some(old), Some(new)) => {
            let delta = Decimal::from_parts(i128::from(new - old) * 60, 0);
            let point = CalendarPoint {
                year: dt.year,
                month: dt.month,
                day: dt.day,
                hour: dt.hour,
                minute: dt.minute,
                second: dt.second,
            };
            let (y, m, d, h, mi, s) = add_seconds_decimal(XsdDatatype::DateTime, &point, &delta)?;
            Ok(DateTime {
                year: y,
                month: m,
                day: d,
                hour: h,
                minute: mi,
                second: s,
                tz: Some(new),
            })
        }
    }
}

/// `fn:adjust-date-to-timezone($input, $timezone)` (XPath F&O §9.6.2). Same rule as
/// [`adjust_datetime_to_timezone`], applied to a date: when both sides have a
/// timezone, the date is treated as midnight in its own offset, shifted to the new
/// offset (which may roll the date to the previous or next day), and only the
/// resulting date is kept.
pub fn adjust_date_to_timezone(d: &Date, timezone: Option<i64>) -> Result<Date, XsdError> {
    let new_tz = validate_timezone_seconds(XsdDatatype::Date, timezone)?;
    let retagged = |tz| Date {
        year: d.year,
        month: d.month,
        day: d.day,
        tz,
    };
    match (d.tz, new_tz) {
        (_, None) => Ok(retagged(None)),
        (None, Some(tz)) => Ok(retagged(Some(tz))),
        (Some(old), Some(new)) => {
            let delta = Decimal::from_parts(i128::from(new - old) * 60, 0);
            let point = CalendarPoint::midnight(d.year, d.month, d.day);
            let (y, m, dd, _h, _mi, _s) = add_seconds_decimal(XsdDatatype::Date, &point, &delta)?;
            Ok(Date {
                year: y,
                month: m,
                day: dd,
                tz: Some(new),
            })
        }
    }
}

/// `fn:adjust-time-to-timezone($input, $timezone)` (XPath F&O §9.6.3). Same rule as
/// [`adjust_datetime_to_timezone`], applied to a time: when both sides have a
/// timezone, the time is combined with an arbitrary reference date (F&O's own
/// worked example uses 1972-12-31; the date is discarded afterward, so any valid
/// date gives the same time-of-day result) and shifted; only the resulting
/// time-of-day is kept.
pub fn adjust_time_to_timezone(t: &Time, timezone: Option<i64>) -> Result<Time, XsdError> {
    let new_tz = validate_timezone_seconds(XsdDatatype::Time, timezone)?;
    let retagged = |tz| Time {
        hour: t.hour,
        minute: t.minute,
        second: t.second,
        tz,
    };
    match (t.tz, new_tz) {
        (_, None) => Ok(retagged(None)),
        (None, Some(tz)) => Ok(retagged(Some(tz))),
        (Some(old), Some(new)) => {
            let delta = Decimal::from_parts(i128::from(new - old) * 60, 0);
            let point = CalendarPoint {
                year: 1972,
                month: 12,
                day: 31,
                hour: t.hour,
                minute: t.minute,
                second: t.second,
            };
            let (_y, _m, _d, h, mi, s) = add_seconds_decimal(XsdDatatype::Time, &point, &delta)?;
            Ok(Time {
                hour: h,
                minute: mi,
                second: s,
                tz: Some(new),
            })
        }
    }
}

// ── Duration ↔ calendar arithmetic (XPath F&O §9.7.5–9.7.14; XML Schema Appendix E) ─

/// `op:add-yearMonthDuration-to-dateTime`. The day-of-month is clamped to the target
/// month's last day when the original day does not exist there (XML Schema Appendix
/// E, e.g. 2024-01-31 + P1M = 2024-02-29).
pub fn add_year_month_duration_to_datetime(
    dt: &DateTime,
    dur: &Duration,
) -> Result<DateTime, XsdError> {
    require_year_month_duration(dur)?;
    let (y, m) = shift_year_month(XsdDatatype::DateTime, dt.year, dt.month, dur.months)?;
    let d = dt.day.min(days_in_month(y, m));
    Ok(DateTime {
        year: y,
        month: m,
        day: d,
        hour: dt.hour,
        minute: dt.minute,
        second: dt.second,
        tz: dt.tz,
    })
}

/// `op:subtract-yearMonthDuration-from-dateTime`.
pub fn subtract_year_month_duration_from_datetime(
    dt: &DateTime,
    dur: &Duration,
) -> Result<DateTime, XsdError> {
    require_year_month_duration(dur)?;
    let months = dur
        .months
        .checked_neg()
        .ok_or_else(|| arith_overflow(XsdDatatype::Duration, "duration negation overflow"))?;
    let (y, m) = shift_year_month(XsdDatatype::DateTime, dt.year, dt.month, months)?;
    let d = dt.day.min(days_in_month(y, m));
    Ok(DateTime {
        year: y,
        month: m,
        day: d,
        hour: dt.hour,
        minute: dt.minute,
        second: dt.second,
        tz: dt.tz,
    })
}

/// `op:add-dayTimeDuration-to-dateTime`.
pub fn add_day_time_duration_to_datetime(
    dt: &DateTime,
    dur: &Duration,
) -> Result<DateTime, XsdError> {
    require_day_time_duration(dur)?;
    let point = CalendarPoint {
        year: dt.year,
        month: dt.month,
        day: dt.day,
        hour: dt.hour,
        minute: dt.minute,
        second: dt.second,
    };
    let (y, m, d, h, mi, s) = add_seconds_decimal(XsdDatatype::DateTime, &point, &dur.seconds)?;
    Ok(DateTime {
        year: y,
        month: m,
        day: d,
        hour: h,
        minute: mi,
        second: s,
        tz: dt.tz,
    })
}

/// `op:subtract-dayTimeDuration-from-dateTime`.
pub fn subtract_day_time_duration_from_datetime(
    dt: &DateTime,
    dur: &Duration,
) -> Result<DateTime, XsdError> {
    require_day_time_duration(dur)?;
    let delta = decimal_negate(XsdDatatype::Duration, &dur.seconds)?;
    let point = CalendarPoint {
        year: dt.year,
        month: dt.month,
        day: dt.day,
        hour: dt.hour,
        minute: dt.minute,
        second: dt.second,
    };
    let (y, m, d, h, mi, s) = add_seconds_decimal(XsdDatatype::DateTime, &point, &delta)?;
    Ok(DateTime {
        year: y,
        month: m,
        day: d,
        hour: h,
        minute: mi,
        second: s,
        tz: dt.tz,
    })
}

/// `op:add-yearMonthDuration-to-date`. Same Appendix E day-clamping rule as
/// [`add_year_month_duration_to_datetime`].
pub fn add_year_month_duration_to_date(d: &Date, dur: &Duration) -> Result<Date, XsdError> {
    require_year_month_duration(dur)?;
    let (y, m) = shift_year_month(XsdDatatype::Date, d.year, d.month, dur.months)?;
    let dd = d.day.min(days_in_month(y, m));
    Ok(Date {
        year: y,
        month: m,
        day: dd,
        tz: d.tz,
    })
}

/// `op:subtract-yearMonthDuration-from-date`.
pub fn subtract_year_month_duration_from_date(d: &Date, dur: &Duration) -> Result<Date, XsdError> {
    require_year_month_duration(dur)?;
    let months = dur
        .months
        .checked_neg()
        .ok_or_else(|| arith_overflow(XsdDatatype::Duration, "duration negation overflow"))?;
    let (y, m) = shift_year_month(XsdDatatype::Date, d.year, d.month, months)?;
    let dd = d.day.min(days_in_month(y, m));
    Ok(Date {
        year: y,
        month: m,
        day: dd,
        tz: d.tz,
    })
}

/// `op:add-dayTimeDuration-to-date`. The duration is applied at midnight and only the
/// resulting date is kept (per F&O; a `dayTimeDuration` may still roll the date
/// forward or backward by whole days).
pub fn add_day_time_duration_to_date(d: &Date, dur: &Duration) -> Result<Date, XsdError> {
    require_day_time_duration(dur)?;
    let point = CalendarPoint::midnight(d.year, d.month, d.day);
    let (y, m, dd, _h, _mi, _s) = add_seconds_decimal(XsdDatatype::Date, &point, &dur.seconds)?;
    Ok(Date {
        year: y,
        month: m,
        day: dd,
        tz: d.tz,
    })
}

/// `op:subtract-dayTimeDuration-from-date`.
pub fn subtract_day_time_duration_from_date(d: &Date, dur: &Duration) -> Result<Date, XsdError> {
    require_day_time_duration(dur)?;
    let delta = decimal_negate(XsdDatatype::Duration, &dur.seconds)?;
    let point = CalendarPoint::midnight(d.year, d.month, d.day);
    let (y, m, dd, _h, _mi, _s) = add_seconds_decimal(XsdDatatype::Date, &point, &delta)?;
    Ok(Date {
        year: y,
        month: m,
        day: dd,
        tz: d.tz,
    })
}

/// `op:add-dayTimeDuration-to-time`. Applied against an arbitrary reference date
/// (the date is discarded, so wrap-around across midnight is invisible in the
/// result — only the time-of-day is returned).
pub fn add_day_time_duration_to_time(t: &Time, dur: &Duration) -> Result<Time, XsdError> {
    require_day_time_duration(dur)?;
    let point = CalendarPoint::on_reference_date(t.hour, t.minute, t.second);
    let (_y, _m, _d, h, mi, s) = add_seconds_decimal(XsdDatatype::Time, &point, &dur.seconds)?;
    Ok(Time {
        hour: h,
        minute: mi,
        second: s,
        tz: t.tz,
    })
}

/// `op:subtract-dayTimeDuration-from-time`.
pub fn subtract_day_time_duration_from_time(t: &Time, dur: &Duration) -> Result<Time, XsdError> {
    require_day_time_duration(dur)?;
    let delta = decimal_negate(XsdDatatype::Duration, &dur.seconds)?;
    let point = CalendarPoint::on_reference_date(t.hour, t.minute, t.second);
    let (_y, _m, _d, h, mi, s) = add_seconds_decimal(XsdDatatype::Time, &point, &delta)?;
    Ok(Time {
        hour: h,
        minute: mi,
        second: s,
        tz: t.tz,
    })
}

// ── Instant subtraction → dayTimeDuration (XPath F&O §9.7.2–9.7.4) ───────────────

/// `op:subtract-dateTimes`. Both operands must agree on whether they carry a
/// timezone (both, or neither) — see `instant_diff`.
pub fn subtract_datetimes(a: &DateTime, b: &DateTime) -> Result<Duration, XsdError> {
    let a_inst = Instant {
        days: days_from_civil(a.year, a.month, a.day),
        hour: a.hour,
        minute: a.minute,
        second: a.second,
        tz: a.tz,
    };
    let b_inst = Instant {
        days: days_from_civil(b.year, b.month, b.day),
        hour: b.hour,
        minute: b.minute,
        second: b.second,
        tz: b.tz,
    };
    instant_diff(XsdDatatype::DateTime, &a_inst, &b_inst)
}

/// `op:subtract-dates`. Each date is treated as midnight in its own timezone before
/// subtracting (per F&O); see `instant_diff` for the timezone-mixing rule.
pub fn subtract_dates(a: &Date, b: &Date) -> Result<Duration, XsdError> {
    let zero = Decimal::from_parts(0, 0);
    let a_inst = Instant {
        days: days_from_civil(a.year, a.month, a.day),
        hour: 0,
        minute: 0,
        second: zero,
        tz: a.tz,
    };
    let b_inst = Instant {
        days: days_from_civil(b.year, b.month, b.day),
        hour: 0,
        minute: 0,
        second: zero,
        tz: b.tz,
    };
    instant_diff(XsdDatatype::Date, &a_inst, &b_inst)
}

/// `op:subtract-times`. Both times are referred to the same (arbitrary, cancelling)
/// reference date; see `instant_diff` for the timezone-mixing rule.
pub fn subtract_times(a: &Time, b: &Time) -> Result<Duration, XsdError> {
    let a_inst = Instant {
        days: 0,
        hour: a.hour,
        minute: a.minute,
        second: a.second,
        tz: a.tz,
    };
    let b_inst = Instant {
        days: 0,
        hour: b.hour,
        minute: b.minute,
        second: b.second,
        tz: b.tz,
    };
    instant_diff(XsdDatatype::Time, &a_inst, &b_inst)
}

// ── Duration arithmetic (XPath F&O §8.4) ─────────────────────────────────────────

/// The two duration values must be the *same declared* `yearMonthDuration`/
/// `dayTimeDuration` subtype. F&O defines `+`/`-`/`*`//` only for those two
/// subtypes (not the general `xsd:duration`, whose (months, seconds) pair mixes
/// units that cannot be added component-wise without loss); mixing subtypes, or
/// using the general `xsd:duration`, is a type error.
fn require_same_duration_subtype(a: &Duration, b: &Duration) -> Result<XsdDatatype, XsdError> {
    if a.datatype != b.datatype {
        return Err(XsdError::TypeMismatch {
            reason: "duration arithmetic requires matching yearMonthDuration/dayTimeDuration subtypes",
        });
    }
    match a.datatype {
        XsdDatatype::YearMonthDuration | XsdDatatype::DayTimeDuration => Ok(a.datatype),
        _ => Err(XsdError::TypeMismatch {
            reason: "duration arithmetic requires a yearMonthDuration or dayTimeDuration operand",
        }),
    }
}

/// `op:add-yearMonthDurations` / `op:add-dayTimeDurations`.
pub fn add_durations(a: &Duration, b: &Duration) -> Result<Duration, XsdError> {
    let datatype = require_same_duration_subtype(a, b)?;
    match datatype {
        XsdDatatype::YearMonthDuration => {
            let months = a
                .months
                .checked_add(b.months)
                .ok_or_else(|| arith_overflow(datatype, "yearMonthDuration addition overflow"))?;
            Ok(Duration {
                months,
                seconds: Decimal::from_parts(0, 0),
                datatype,
            })
        }
        _ => {
            let seconds = decimal_add_exact(datatype, &a.seconds, &b.seconds)?;
            Ok(Duration {
                months: 0,
                seconds,
                datatype,
            })
        }
    }
}

/// `op:subtract-yearMonthDurations` / `op:subtract-dayTimeDurations`.
pub fn subtract_durations(a: &Duration, b: &Duration) -> Result<Duration, XsdError> {
    let datatype = require_same_duration_subtype(a, b)?;
    match datatype {
        XsdDatatype::YearMonthDuration => {
            let months = a.months.checked_sub(b.months).ok_or_else(|| {
                arith_overflow(datatype, "yearMonthDuration subtraction overflow")
            })?;
            Ok(Duration {
                months,
                seconds: Decimal::from_parts(0, 0),
                datatype,
            })
        }
        _ => {
            let seconds = decimal_sub_exact(datatype, &a.seconds, &b.seconds)?;
            Ok(Duration {
                months: 0,
                seconds,
                datatype,
            })
        }
    }
}

/// `op:multiply-yearMonthDuration` / `op:multiply-dayTimeDuration`. `factor` is a
/// `Decimal` rather than F&O's `xs:double`: this crate keeps its stored values exact
/// (no floats), so the multiplication stays exact too. `yearMonthDuration` results
/// are rounded to the nearest whole month, ties toward positive infinity (matching
/// `fn:round`, since months cannot be fractional).
pub fn multiply_duration(dur: &Duration, factor: &Decimal) -> Result<Duration, XsdError> {
    match dur.datatype {
        XsdDatatype::YearMonthDuration => {
            let months_dec = Decimal::from_parts(i128::from(dur.months), 0);
            let product = decimal_mul_raw(&months_dec, factor).map_err(|_| {
                arith_overflow(dur.datatype, "yearMonthDuration multiplication overflow")
            })?;
            let months = round_decimal_to_i64(dur.datatype, &product)?;
            Ok(Duration {
                months,
                seconds: Decimal::from_parts(0, 0),
                datatype: dur.datatype,
            })
        }
        XsdDatatype::DayTimeDuration => {
            let seconds = decimal_mul_raw(&dur.seconds, factor).map_err(|_| {
                arith_overflow(dur.datatype, "dayTimeDuration multiplication overflow")
            })?;
            Ok(Duration {
                months: 0,
                seconds,
                datatype: dur.datatype,
            })
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "multiply requires a yearMonthDuration or dayTimeDuration operand",
        }),
    }
}

/// `op:divide-yearMonthDuration` / `op:divide-dayTimeDuration`. `divisor` is a
/// `Decimal` for the same exactness reason as [`multiply_duration`]; a zero divisor
/// is `Err(DivisionByZero)`.
pub fn divide_duration(dur: &Duration, divisor: &Decimal) -> Result<Duration, XsdError> {
    if divisor.is_zero() {
        return Err(XsdError::DivisionByZero {
            datatype: dur.datatype,
        });
    }
    match dur.datatype {
        XsdDatatype::YearMonthDuration => {
            let months_dec = Decimal::from_parts(i128::from(dur.months), 0);
            let quotient = decimal_div_raw(&months_dec, divisor)?;
            let months = round_decimal_to_i64(dur.datatype, &quotient)?;
            Ok(Duration {
                months,
                seconds: Decimal::from_parts(0, 0),
                datatype: dur.datatype,
            })
        }
        XsdDatatype::DayTimeDuration => {
            let seconds = decimal_div_raw(&dur.seconds, divisor)?;
            Ok(Duration {
                months: 0,
                seconds,
                datatype: dur.datatype,
            })
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "divide requires a yearMonthDuration or dayTimeDuration operand",
        }),
    }
}

/// `op:divide-yearMonthDuration-by-yearMonthDuration` → `xs:decimal`.
pub fn divide_year_month_durations(a: &Duration, b: &Duration) -> Result<Decimal, XsdError> {
    require_year_month_duration(a)?;
    require_year_month_duration(b)?;
    if b.months == 0 {
        return Err(XsdError::DivisionByZero {
            datatype: XsdDatatype::YearMonthDuration,
        });
    }
    decimal_div_raw(
        &Decimal::from_parts(i128::from(a.months), 0),
        &Decimal::from_parts(i128::from(b.months), 0),
    )
}

/// `op:divide-dayTimeDuration-by-dayTimeDuration` → `xs:decimal`.
pub fn divide_day_time_durations(a: &Duration, b: &Duration) -> Result<Decimal, XsdError> {
    require_day_time_duration(a)?;
    require_day_time_duration(b)?;
    if b.seconds.is_zero() {
        return Err(XsdError::DivisionByZero {
            datatype: XsdDatatype::DayTimeDuration,
        });
    }
    decimal_div_raw(&a.seconds, &b.seconds)
}

// ── Canonical lexical mapping ────────────────────────────────────────────────────

fn fmt_year(year: i64) -> String {
    if year < 0 {
        format!("-{:04}", -year)
    } else {
        format!("{year:04}")
    }
}

fn fmt_tz(tz: Option<i32>) -> String {
    match tz {
        None => String::new(),
        Some(0) => "Z".to_string(),
        Some(off) => {
            let sign = if off < 0 { '-' } else { '+' };
            let a = off.abs();
            format!("{sign}{:02}:{:02}", a / 60, a % 60)
        }
    }
}

/// Canonical seconds field: two integer digits, fractional part trimmed of trailing
/// zeros (and dropped entirely if zero).
fn fmt_seconds(sec: &Decimal) -> String {
    let whole = sec.whole_part();
    let frac = sec.frac_part();
    if frac.is_zero() {
        format!("{whole:02}")
    } else {
        // `canonical_lexical` yields e.g. "0.5"; take the fractional digits.
        let canon = frac.canonical_lexical();
        let digits = canon.split_once('.').map_or("", |(_, f)| f);
        format!("{whole:02}.{digits}")
    }
}

impl DateTime {
    /// XSD canonical lexical form.
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        format!(
            "{}-{:02}-{:02}T{:02}:{:02}:{}{}",
            fmt_year(self.year),
            self.month,
            self.day,
            self.hour,
            self.minute,
            fmt_seconds(&self.second),
            fmt_tz(self.tz),
        )
    }

    /// Gregorian year component.
    #[must_use]
    pub fn year(&self) -> i64 {
        self.year
    }

    /// Gregorian month component (1–12).
    #[must_use]
    pub fn month(&self) -> u8 {
        self.month
    }

    /// Gregorian day component (1–31).
    #[must_use]
    pub fn day(&self) -> u8 {
        self.day
    }

    /// Hour component (0–24).
    #[must_use]
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// Minute component (0–59).
    #[must_use]
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// Second component as a Decimal.
    #[must_use]
    pub fn second(&self) -> Decimal {
        self.second
    }

    /// Timezone offset in minutes; None = no timezone.
    #[must_use]
    pub fn timezone_minutes(&self) -> Option<i64> {
        self.tz.map(i64::from)
    }
}

impl Date {
    /// XSD canonical lexical form.
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        format!(
            "{}-{:02}-{:02}{}",
            fmt_year(self.year),
            self.month,
            self.day,
            fmt_tz(self.tz)
        )
    }

    /// Gregorian year component.
    #[must_use]
    pub fn year(&self) -> i64 {
        self.year
    }

    /// Gregorian month component (1–12).
    #[must_use]
    pub fn month(&self) -> u8 {
        self.month
    }

    /// Gregorian day component (1–31).
    #[must_use]
    pub fn day(&self) -> u8 {
        self.day
    }

    /// Timezone offset in minutes; None = no timezone.
    #[must_use]
    pub fn timezone_minutes(&self) -> Option<i64> {
        self.tz.map(i64::from)
    }
}

impl Time {
    /// XSD canonical lexical form.
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        format!(
            "{:02}:{:02}:{}{}",
            self.hour,
            self.minute,
            fmt_seconds(&self.second),
            fmt_tz(self.tz)
        )
    }

    /// Hour component (0–24).
    #[must_use]
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// Minute component (0–59).
    #[must_use]
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// Second component as a Decimal.
    #[must_use]
    pub fn second(&self) -> Decimal {
        self.second
    }

    /// Timezone offset in minutes; None = no timezone.
    #[must_use]
    pub fn timezone_minutes(&self) -> Option<i64> {
        self.tz.map(i64::from)
    }
}

impl Duration {
    /// Canonical lexical form `[-]PnYnMnDTnHnMnS` (general duration grammar).
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        let neg = self.months < 0 || self.seconds.mantissa() < 0;
        let months = self.months.unsigned_abs();
        let years = months / 12;
        let rem_months = months % 12;
        let total_secs = self.seconds.whole_part().unsigned_abs();
        let frac = self.seconds.frac_part();
        let days = total_secs / 86_400;
        let rem = total_secs % 86_400;
        let hours = rem / 3600;
        let mins = (rem % 3600) / 60;
        let secs = rem % 60;

        use std::fmt::Write as _;
        let mut out = String::new();
        if neg {
            out.push('-');
        }
        out.push('P');
        if years > 0 {
            let _ = write!(out, "{years}Y");
        }
        if rem_months > 0 {
            let _ = write!(out, "{rem_months}M");
        }
        if days > 0 {
            let _ = write!(out, "{days}D");
        }
        let has_time = hours > 0 || mins > 0 || secs > 0 || !frac.is_zero();
        if has_time {
            out.push('T');
            if hours > 0 {
                let _ = write!(out, "{hours}H");
            }
            if mins > 0 {
                let _ = write!(out, "{mins}M");
            }
            if secs > 0 || !frac.is_zero() {
                if frac.is_zero() {
                    let _ = write!(out, "{secs}S");
                } else {
                    let canon = frac.canonical_lexical();
                    let digits = canon.split_once('.').map_or("", |(_, f)| f);
                    let _ = write!(out, "{secs}.{digits}S");
                }
            }
        }
        // The zero duration canonicalizes to "PT0S".
        if out == "P" || out == "-P" {
            out.push_str("T0S");
        }
        out
    }
}

impl Gregorian {
    /// XSD canonical lexical form.
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        let tz = fmt_tz(self.tz);
        match self.datatype {
            XsdDatatype::GYear => {
                format!("{}{tz}", fmt_year(self.year.unwrap_or(0)))
            }
            XsdDatatype::GMonth => {
                format!("--{:02}{tz}", self.month.unwrap_or(1))
            }
            XsdDatatype::GDay => {
                format!("---{:02}{tz}", self.day.unwrap_or(1))
            }
            XsdDatatype::GYearMonth => {
                format!(
                    "{}-{:02}{tz}",
                    fmt_year(self.year.unwrap_or(0)),
                    self.month.unwrap_or(1)
                )
            }
            XsdDatatype::GMonthDay => {
                format!(
                    "--{:02}-{:02}{tz}",
                    self.month.unwrap_or(1),
                    self.day.unwrap_or(1)
                )
            }
            _ => String::new(), // unreachable for well-formed Gregorian values
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn datetime_parse_canonical_roundtrip() {
        let dt = parse_datetime("2024-02-29T12:30:00Z").unwrap();
        assert_eq!(dt.canonical_lexical(), "2024-02-29T12:30:00Z");
        let dt = parse_datetime("2024-02-29T12:30:00.500-05:00").unwrap();
        assert_eq!(dt.canonical_lexical(), "2024-02-29T12:30:00.5-05:00");
        let dt = parse_datetime("-0044-03-15T12:00:00").unwrap();
        assert_eq!(dt.canonical_lexical(), "-0044-03-15T12:00:00");
    }

    #[test]
    fn datetime_ordering_with_timezones() {
        let a = parse_datetime("2024-01-01T00:00:00Z").unwrap();
        let b = parse_datetime("2024-01-01T01:00:00+01:00").unwrap(); // same instant
        assert_eq!(cmp_datetime(&a, &b), Some(Ordering::Equal));
        let c = parse_datetime("2024-01-01T00:00:01Z").unwrap();
        assert_eq!(cmp_datetime(&a, &c), Some(Ordering::Less));
    }

    #[test]
    fn datetime_indeterminate_when_one_lacks_timezone() {
        // No tz vs a tz'd value within the ±14h overlap → indeterminate.
        let no_tz = parse_datetime("2024-01-01T12:00:00").unwrap();
        let tzd = parse_datetime("2024-01-01T12:00:00Z").unwrap();
        assert_eq!(cmp_datetime(&no_tz, &tzd), None);
        // Far enough apart → determinate.
        let early = parse_datetime("2024-01-01T00:00:00").unwrap();
        let late_z = parse_datetime("2024-01-02T20:00:00Z").unwrap();
        assert_eq!(cmp_datetime(&early, &late_z), Some(Ordering::Less));
    }

    #[test]
    fn duration_partial_order() {
        let p1y = parse_duration(XsdDatatype::Duration, "P1Y").unwrap();
        let p13m = parse_duration(XsdDatatype::Duration, "P13M").unwrap();
        assert_eq!(cmp_duration(&p1y, &p13m), Some(Ordering::Less)); // 12mo < 13mo
        // P1M vs P30D: months differ one way, seconds the other → indeterminate.
        let p1m = parse_duration(XsdDatatype::Duration, "P1M").unwrap();
        let p30d = parse_duration(XsdDatatype::Duration, "P30D").unwrap();
        assert_eq!(cmp_duration(&p1m, &p30d), None);
        // dayTimeDuration is totally ordered.
        let h1 = parse_duration(XsdDatatype::DayTimeDuration, "PT1H").unwrap();
        let h2 = parse_duration(XsdDatatype::DayTimeDuration, "PT2H").unwrap();
        assert_eq!(cmp_duration(&h1, &h2), Some(Ordering::Less));
    }

    #[test]
    fn duration_canonical() {
        assert_eq!(
            parse_duration(XsdDatatype::Duration, "P1Y2M3DT4H5M6S")
                .unwrap()
                .canonical_lexical(),
            "P1Y2M3DT4H5M6S"
        );
        assert_eq!(
            parse_duration(XsdDatatype::Duration, "PT0S")
                .unwrap()
                .canonical_lexical(),
            "PT0S"
        );
        assert_eq!(
            parse_duration(XsdDatatype::DayTimeDuration, "PT1.5S")
                .unwrap()
                .canonical_lexical(),
            "PT1.5S"
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_datetime("2024-01-01").is_err()); // no time
        assert!(parse_date("2024-13-01").is_err()); // month 13
        assert!(parse_time("12:00").is_err()); // no seconds
        assert!(parse_duration(XsdDatatype::Duration, "1Y").is_err()); // no P
        assert!(parse_datetime("2024-01-01T12:00:00+15:00").is_err()); // tz > 14h
    }

    #[test]
    fn gregorian_parse_roundtrip() {
        let g = parse_gregorian(XsdDatatype::GYear, "2024").unwrap();
        assert_eq!(g.canonical_lexical(), "2024");
        let g = parse_gregorian(XsdDatatype::GYear, "2024Z").unwrap();
        assert_eq!(g.canonical_lexical(), "2024Z");
        let g = parse_gregorian(XsdDatatype::GMonth, "--05").unwrap();
        assert_eq!(g.canonical_lexical(), "--05");
        let g = parse_gregorian(XsdDatatype::GDay, "---15").unwrap();
        assert_eq!(g.canonical_lexical(), "---15");
        let g = parse_gregorian(XsdDatatype::GYearMonth, "2024-05").unwrap();
        assert_eq!(g.canonical_lexical(), "2024-05");
        let g = parse_gregorian(XsdDatatype::GMonthDay, "--02-29").unwrap();
        assert_eq!(g.canonical_lexical(), "--02-29");
    }

    #[test]
    fn gregorian_cmp_same_type() {
        let a = parse_gregorian(XsdDatatype::GYear, "2023").unwrap();
        let b = parse_gregorian(XsdDatatype::GYear, "2024").unwrap();
        assert_eq!(cmp_gregorian(&a, &b), Some(Ordering::Less));
        let c = parse_gregorian(XsdDatatype::GMonth, "--03").unwrap();
        let d = parse_gregorian(XsdDatatype::GMonth, "--11").unwrap();
        assert_eq!(cmp_gregorian(&c, &d), Some(Ordering::Less));
    }

    #[test]
    fn gregorian_cross_type_incomparable() {
        let a = parse_gregorian(XsdDatatype::GYear, "2024").unwrap();
        let b = parse_gregorian(XsdDatatype::GMonth, "--05").unwrap();
        assert_eq!(cmp_gregorian(&a, &b), None);
    }
    #[test]
    fn datetime_from_unix_seconds_epoch() {
        let dt = datetime_from_unix_seconds(0);
        assert_eq!(dt.canonical_lexical(), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn datetime_from_unix_seconds_known_timestamp() {
        // 2024-03-15T10:30:00Z = 2024-03-15 is day 19796 since epoch.
        // 10*3600 + 30*60 = 37800 seconds into the day.
        // 19796 * 86400 + 37800 = 1710495000
        let dt = datetime_from_unix_seconds(1_710_498_600);
        assert_eq!(dt.canonical_lexical(), "2024-03-15T10:30:00Z");
    }

    #[test]
    fn datetime_accessors_work() {
        let dt = parse_datetime("2024-03-15T10:30:45.5Z").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.timezone_minutes(), Some(0));
    }

    // ── fn:adjust-*-to-timezone (XPath F&O §9.6) ──────────────────────────────────

    #[test]
    fn adjust_datetime_shifts_same_instant() {
        // F&O worked example: 2002-03-07T10:00:00-07:00 adjusted to +10:00 becomes
        // 2002-03-08T03:00:00+10:00 — the same instant, re-expressed.
        let dt = parse_datetime("2002-03-07T10:00:00-07:00").unwrap();
        let adjusted = adjust_datetime_to_timezone(&dt, Some(10 * 3600)).unwrap();
        assert_eq!(adjusted.canonical_lexical(), "2002-03-08T03:00:00+10:00");
        // Same instant: comparing original and adjusted must be exactly Equal.
        assert_eq!(cmp_datetime(&dt, &adjusted), Some(Ordering::Equal));

        let a = parse_datetime("2024-01-01T00:00:00Z").unwrap();
        let b = adjust_datetime_to_timezone(&a, Some(3600)).unwrap();
        assert_eq!(b.canonical_lexical(), "2024-01-01T01:00:00+01:00");
        assert_eq!(cmp_datetime(&a, &b), Some(Ordering::Equal));
    }

    #[test]
    fn adjust_datetime_without_timezone_attaches_no_shift() {
        let dt = parse_datetime("2024-01-01T12:00:00").unwrap();
        let adjusted = adjust_datetime_to_timezone(&dt, Some(3600)).unwrap();
        // Local fields unchanged; only the timezone is attached.
        assert_eq!(adjusted.canonical_lexical(), "2024-01-01T12:00:00+01:00");
    }

    #[test]
    fn adjust_datetime_removes_timezone() {
        let dt = parse_datetime("2024-01-01T12:00:00Z").unwrap();
        let adjusted = adjust_datetime_to_timezone(&dt, None).unwrap();
        assert_eq!(adjusted.canonical_lexical(), "2024-01-01T12:00:00");
        assert_eq!(adjusted.timezone_minutes(), None);
    }

    #[test]
    fn adjust_datetime_timezone_boundary() {
        let dt = parse_datetime("2024-01-01T12:00:00Z").unwrap();
        // ±14:00 is accepted.
        assert!(adjust_datetime_to_timezone(&dt, Some(14 * 3600)).is_ok());
        assert!(adjust_datetime_to_timezone(&dt, Some(-14 * 3600)).is_ok());
        // Beyond ±14:00 is rejected.
        assert!(adjust_datetime_to_timezone(&dt, Some(14 * 3600 + 60)).is_err());
        assert!(adjust_datetime_to_timezone(&dt, Some(-14 * 3600 - 60)).is_err());
        // Non-whole-minute offsets are rejected.
        assert!(adjust_datetime_to_timezone(&dt, Some(90)).is_err());
        assert!(adjust_datetime_to_timezone(&dt, Some(3661)).is_err());
    }

    #[test]
    fn adjust_date_rolls_over_and_removes() {
        // Midnight in +10:00 shifted to -07:00: local = 00:00 - 17h, which rolls
        // back to the previous day (2002-03-06T07:00, date truncated to the date).
        let d = parse_date("2002-03-07+10:00").unwrap();
        let adjusted = adjust_date_to_timezone(&d, Some(-7 * 3600)).unwrap();
        assert_eq!(adjusted.canonical_lexical(), "2002-03-06-07:00");

        let no_tz = parse_date("2024-06-15").unwrap();
        let attached = adjust_date_to_timezone(&no_tz, Some(0)).unwrap();
        assert_eq!(attached.canonical_lexical(), "2024-06-15Z");

        let removed = adjust_date_to_timezone(&d, None).unwrap();
        assert_eq!(removed.canonical_lexical(), "2002-03-07");
    }

    #[test]
    fn adjust_time_wraps_and_removes() {
        let t = parse_time("10:00:00-07:00").unwrap();
        let adjusted = adjust_time_to_timezone(&t, Some(10 * 3600)).unwrap();
        assert_eq!(adjusted.canonical_lexical(), "03:00:00+10:00");

        let removed = adjust_time_to_timezone(&t, None).unwrap();
        assert_eq!(removed.canonical_lexical(), "10:00:00");

        let no_tz = parse_time("08:30:00").unwrap();
        let attached = adjust_time_to_timezone(&no_tz, Some(-5 * 3600)).unwrap();
        assert_eq!(attached.canonical_lexical(), "08:30:00-05:00");
    }

    // ── Duration ↔ calendar arithmetic: Appendix E month-end clamping ────────────

    fn ymd(dt: XsdDatatype, s: &str) -> Duration {
        parse_duration(dt, s).unwrap()
    }

    #[test]
    fn add_year_month_duration_clamps_to_month_end() {
        // Jan 31 + P1M: Feb 29 in a leap year, Feb 28 otherwise.
        let leap = parse_date("2024-01-31").unwrap();
        let p1m = ymd(XsdDatatype::YearMonthDuration, "P1M");
        assert_eq!(
            add_year_month_duration_to_date(&leap, &p1m)
                .unwrap()
                .canonical_lexical(),
            "2024-02-29"
        );
        let non_leap = parse_date("2023-01-31").unwrap();
        assert_eq!(
            add_year_month_duration_to_date(&non_leap, &p1m)
                .unwrap()
                .canonical_lexical(),
            "2023-02-28"
        );
        // Aug 31 + P1M = Sep 30 (Sep has 30 days).
        let aug31 = parse_date("2024-08-31").unwrap();
        assert_eq!(
            add_year_month_duration_to_date(&aug31, &p1m)
                .unwrap()
                .canonical_lexical(),
            "2024-09-30"
        );
        // Same clamping rule applies to dateTime.
        let leap_dt = parse_datetime("2024-01-31T10:15:00").unwrap();
        assert_eq!(
            add_year_month_duration_to_datetime(&leap_dt, &p1m)
                .unwrap()
                .canonical_lexical(),
            "2024-02-29T10:15:00"
        );
    }

    #[test]
    fn subtract_year_month_duration_clamps_to_month_end() {
        // Mar 31 - P1M = Feb 29 (2024 is a leap year).
        let d = parse_date("2024-03-31").unwrap();
        let p1m = ymd(XsdDatatype::YearMonthDuration, "P1M");
        assert_eq!(
            subtract_year_month_duration_from_date(&d, &p1m)
                .unwrap()
                .canonical_lexical(),
            "2024-02-29"
        );
    }

    #[test]
    fn day_time_duration_add_subtract_datetime() {
        let dt = parse_datetime("2024-01-01T23:00:00Z").unwrap();
        let p2h = ymd(XsdDatatype::DayTimeDuration, "PT2H");
        // Crosses midnight into the next day.
        assert_eq!(
            add_day_time_duration_to_datetime(&dt, &p2h)
                .unwrap()
                .canonical_lexical(),
            "2024-01-02T01:00:00Z"
        );
        assert_eq!(
            subtract_day_time_duration_from_datetime(&dt, &p2h)
                .unwrap()
                .canonical_lexical(),
            "2024-01-01T21:00:00Z"
        );
    }

    #[test]
    fn day_time_duration_add_to_date_rolls_over() {
        let d = parse_date("2024-01-31").unwrap();
        let p2d = ymd(XsdDatatype::DayTimeDuration, "P2D");
        assert_eq!(
            add_day_time_duration_to_date(&d, &p2d)
                .unwrap()
                .canonical_lexical(),
            "2024-02-02"
        );
        assert_eq!(
            subtract_day_time_duration_from_date(&d, &p2d)
                .unwrap()
                .canonical_lexical(),
            "2024-01-29"
        );
    }

    #[test]
    fn day_time_duration_add_to_time_wraps() {
        let t = parse_time("23:30:00").unwrap();
        let p1h = ymd(XsdDatatype::DayTimeDuration, "PT1H");
        // Wraps past midnight; only the time-of-day survives.
        assert_eq!(
            add_day_time_duration_to_time(&t, &p1h)
                .unwrap()
                .canonical_lexical(),
            "00:30:00"
        );
        assert_eq!(
            subtract_day_time_duration_from_time(&t, &p1h)
                .unwrap()
                .canonical_lexical(),
            "22:30:00"
        );
    }

    #[test]
    fn day_time_duration_add_with_fractional_seconds() {
        // `dt`'s seconds field and the duration's seconds component start at
        // DIFFERENT scales (2 fractional digits vs. 3) — `add_seconds_decimal`
        // must align them to the higher scale before adding rather than silently
        // truncating either side, which would be a silent precision loss.
        let dt = parse_datetime("2024-01-01T00:00:00.25Z").unwrap();
        let delta = ymd(XsdDatatype::DayTimeDuration, "PT0.125S");
        assert_eq!(
            add_day_time_duration_to_datetime(&dt, &delta)
                .unwrap()
                .canonical_lexical(),
            "2024-01-01T00:00:00.375Z"
        );
        assert_eq!(
            subtract_day_time_duration_from_datetime(&dt, &delta)
                .unwrap()
                .canonical_lexical(),
            "2024-01-01T00:00:00.125Z"
        );
    }

    #[test]
    fn day_time_duration_add_to_time_with_fractional_seconds() {
        // Same scale-alignment concern as
        // `day_time_duration_add_with_fractional_seconds`, exercised through the
        // time-only entry point (an independent call site of the same shared
        // `add_seconds_decimal` helper).
        let t = parse_time("00:00:00.5").unwrap();
        let delta = ymd(XsdDatatype::DayTimeDuration, "PT0.125S");
        assert_eq!(
            add_day_time_duration_to_time(&t, &delta)
                .unwrap()
                .canonical_lexical(),
            "00:00:00.625"
        );
    }

    #[test]
    fn duration_to_calendar_arithmetic_rejects_wrong_subtype() {
        let dt = parse_datetime("2024-01-01T00:00:00Z").unwrap();
        let general = ymd(XsdDatatype::Duration, "P1Y1D");
        assert!(add_year_month_duration_to_datetime(&dt, &general).is_err());
        assert!(add_day_time_duration_to_datetime(&dt, &general).is_err());
    }

    // ── subtract-{dateTimes,dates,times} → dayTimeDuration ────────────────────────

    #[test]
    fn subtract_datetimes_across_timezones() {
        let a = parse_datetime("2000-10-30T06:12:00-05:00").unwrap();
        let b = parse_datetime("1999-11-28T09:00:00-13:00").unwrap();
        let d = subtract_datetimes(&a, &b).unwrap();
        assert_eq!(d.datatype(), XsdDatatype::DayTimeDuration);
        // Sanity: the difference should be a large positive dayTimeDuration
        // (a is about a year after b).
        assert!(d.seconds().mantissa() > 0);

        // Same instant, expressed in different offsets → zero difference.
        let x = parse_datetime("2024-01-01T00:00:00Z").unwrap();
        let y = parse_datetime("2024-01-01T01:00:00+01:00").unwrap();
        let zero = subtract_datetimes(&x, &y).unwrap();
        assert!(zero.seconds().is_zero());
    }

    #[test]
    fn subtract_datetimes_indeterminate_mix_is_error() {
        let with_tz = parse_datetime("2024-01-01T00:00:00Z").unwrap();
        let without_tz = parse_datetime("2024-01-01T00:00:00").unwrap();
        assert!(subtract_datetimes(&with_tz, &without_tz).is_err());
        assert!(subtract_datetimes(&without_tz, &with_tz).is_err());
        // Both untimezoned is fine (naive difference).
        let a = parse_datetime("2024-01-02T00:00:00").unwrap();
        let b = parse_datetime("2024-01-01T00:00:00").unwrap();
        let d = subtract_datetimes(&a, &b).unwrap();
        assert_eq!(d.seconds().canonical_lexical(), "86400");
    }

    #[test]
    fn subtract_dates_and_times() {
        let a = parse_date("2024-01-03").unwrap();
        let b = parse_date("2024-01-01").unwrap();
        let d = subtract_dates(&a, &b).unwrap();
        assert_eq!(d.seconds().canonical_lexical(), "172800"); // 2 days

        let t1 = parse_time("10:00:00").unwrap();
        let t2 = parse_time("08:30:00").unwrap();
        let dt = subtract_times(&t1, &t2).unwrap();
        assert_eq!(dt.seconds().canonical_lexical(), "5400"); // 1.5 hours
    }

    /// `instant_diff` (shared by `subtract_datetimes`/`subtract_dates`/
    /// `subtract_times`) must align mismatched fractional-second scales before
    /// subtracting, exactly as `add_seconds_decimal` must — see
    /// `day_time_duration_add_with_fractional_seconds`'s identical concern on the
    /// addition side.
    #[test]
    fn subtract_datetimes_with_fractional_seconds() {
        let a = parse_datetime("2024-01-01T00:00:00.5Z").unwrap();
        let b = parse_datetime("2024-01-01T00:00:00.125Z").unwrap();
        let d = subtract_datetimes(&a, &b).unwrap();
        assert_eq!(d.seconds().canonical_lexical(), "0.375");

        // Reversed operands: the sign flips, magnitude unchanged.
        let d = subtract_datetimes(&b, &a).unwrap();
        assert_eq!(d.seconds().canonical_lexical(), "-0.375");
    }

    #[test]
    fn subtract_times_with_fractional_seconds() {
        let a = parse_time("00:00:01.75").unwrap();
        let b = parse_time("00:00:00.25").unwrap();
        let d = subtract_times(&a, &b).unwrap();
        assert_eq!(d.seconds().canonical_lexical(), "1.5");
    }

    /// `op:subtract-dates`'s timezone-indeterminacy rule (a hard `TypeMismatch`
    /// when exactly one side carries a timezone — see `instant_diff`'s docs), pinned
    /// independently of `subtract_datetimes_indeterminate_mix_is_error`: the dateTime
    /// form is not the only caller of the shared `instant_diff` helper, and a defect
    /// specific to the date-only zeroed-time-of-day path would have no other test to
    /// catch it.
    #[test]
    fn subtract_dates_indeterminate_mix_is_error() {
        let with_tz = parse_date("2024-01-01Z").unwrap();
        let without_tz = parse_date("2024-01-01").unwrap();
        assert!(subtract_dates(&with_tz, &without_tz).is_err());
        assert!(subtract_dates(&without_tz, &with_tz).is_err());
        // Both timezoned, and both untimezoned, are fine.
        assert!(subtract_dates(&with_tz, &with_tz).is_ok());
        assert!(subtract_dates(&without_tz, &without_tz).is_ok());
    }

    /// `op:subtract-times`'s twin of `subtract_dates_indeterminate_mix_is_error` —
    /// see that test's doc comment for why the dateTime coverage alone does not
    /// pin this.
    #[test]
    fn subtract_times_indeterminate_mix_is_error() {
        let with_tz = parse_time("10:00:00Z").unwrap();
        let without_tz = parse_time("10:00:00").unwrap();
        assert!(subtract_times(&with_tz, &without_tz).is_err());
        assert!(subtract_times(&without_tz, &with_tz).is_err());
        assert!(subtract_times(&with_tz, &with_tz).is_ok());
        assert!(subtract_times(&without_tz, &without_tz).is_ok());
    }

    // ── Duration arithmetic (XPath F&O §8.4) ─────────────────────────────────────

    #[test]
    fn duration_add_subtract_same_subtype() {
        let a = ymd(XsdDatatype::YearMonthDuration, "P1Y");
        let b = ymd(XsdDatatype::YearMonthDuration, "P6M");
        assert_eq!(add_durations(&a, &b).unwrap().canonical_lexical(), "P1Y6M");
        assert_eq!(
            subtract_durations(&a, &b).unwrap().canonical_lexical(),
            "P6M"
        );

        let x = ymd(XsdDatatype::DayTimeDuration, "PT1H");
        let y = ymd(XsdDatatype::DayTimeDuration, "PT30M");
        assert_eq!(
            add_durations(&x, &y).unwrap().canonical_lexical(),
            "PT1H30M"
        );
        assert_eq!(
            subtract_durations(&x, &y).unwrap().canonical_lexical(),
            "PT30M"
        );
    }

    #[test]
    fn duration_add_rejects_mixed_subtype() {
        let ym = ymd(XsdDatatype::YearMonthDuration, "P1Y");
        let dt = ymd(XsdDatatype::DayTimeDuration, "PT1H");
        assert!(add_durations(&ym, &dt).is_err());
        // The general xsd:duration is also rejected — F&O defines +/-/*// only for
        // the two named subtypes.
        let general_a = ymd(XsdDatatype::Duration, "P1Y1D");
        let general_b = ymd(XsdDatatype::Duration, "P1D");
        assert!(add_durations(&general_a, &general_b).is_err());
    }

    #[test]
    fn duration_multiply_and_divide() {
        let d = ymd(XsdDatatype::DayTimeDuration, "PT1H");
        let two = Decimal::from_parts(2, 0);
        assert_eq!(
            multiply_duration(&d, &two).unwrap().canonical_lexical(),
            "PT2H"
        );
        assert_eq!(
            divide_duration(&d, &two).unwrap().canonical_lexical(),
            "PT30M"
        );

        // yearMonthDuration multiply rounds to the nearest whole month.
        let ym = ymd(XsdDatatype::YearMonthDuration, "P1M"); // 1 month
        let three = Decimal::from_parts(3, 0);
        assert_eq!(multiply_duration(&ym, &three).unwrap().months(), 3);
        let half = parse_decimal("0.5").unwrap();
        // 1 month * 0.5 = 0.5, rounds to 1 (ties toward +infinity).
        assert_eq!(multiply_duration(&ym, &half).unwrap().months(), 1);
    }

    /// `round_decimal_to_i64`'s "ties toward positive infinity" rule (mirroring
    /// `fn:round`, XPath F&O §4.4.5 — see that function's doc comment), pinned on
    /// the NEGATIVE side: `duration_multiply_and_divide` above only exercises a
    /// positive tie (`0.5 → 1`), which cannot distinguish "round half away from
    /// zero" from "round half toward positive infinity" — the two rules agree for
    /// every positive tie and disagree for every negative one. `-0.5`/`-1.5`/`-2.5`
    /// round to `0`/`-1`/`-2` (toward +infinity, i.e. AWAY from what "round half
    /// away from zero" would give: `-1`/`-2`/`-3`).
    #[test]
    fn year_month_duration_multiply_rounds_negative_ties_toward_positive_infinity() {
        let half = parse_decimal("0.5").unwrap();

        let neg1 = ymd(XsdDatatype::YearMonthDuration, "-P1M");
        assert_eq!(multiply_duration(&neg1, &half).unwrap().months(), 0);

        let neg3 = ymd(XsdDatatype::YearMonthDuration, "-P3M");
        assert_eq!(multiply_duration(&neg3, &half).unwrap().months(), -1);

        let neg5 = ymd(XsdDatatype::YearMonthDuration, "-P5M");
        assert_eq!(multiply_duration(&neg5, &half).unwrap().months(), -2);
    }

    /// Same negative-tie rule, reached through `divide_duration` rather than
    /// `multiply_duration` — a different call site of the same
    /// `round_decimal_to_i64` helper.
    #[test]
    fn year_month_duration_divide_rounds_negative_ties_toward_positive_infinity() {
        let two = Decimal::from_parts(2, 0);
        // -1 month / 2 = -0.5 → 0, not -1.
        let neg1 = ymd(XsdDatatype::YearMonthDuration, "-P1M");
        assert_eq!(divide_duration(&neg1, &two).unwrap().months(), 0);
        // -3 months / 2 = -1.5 → -1, not -2.
        let neg3 = ymd(XsdDatatype::YearMonthDuration, "-P3M");
        assert_eq!(divide_duration(&neg3, &two).unwrap().months(), -1);
    }

    #[test]
    fn duration_divide_by_zero_is_error() {
        let d = ymd(XsdDatatype::DayTimeDuration, "PT1H");
        let zero = Decimal::from_parts(0, 0);
        assert!(matches!(
            divide_duration(&d, &zero),
            Err(XsdError::DivisionByZero { .. })
        ));
    }

    #[test]
    fn day_time_duration_divided_by_day_time_duration() {
        let a = ymd(XsdDatatype::DayTimeDuration, "PT1H");
        let b = ymd(XsdDatatype::DayTimeDuration, "PT30M");
        let q = divide_day_time_durations(&a, &b).unwrap();
        assert_eq!(q.canonical_lexical(), "2");

        let zero = ymd(XsdDatatype::DayTimeDuration, "PT0S");
        assert!(matches!(
            divide_day_time_durations(&a, &zero),
            Err(XsdError::DivisionByZero { .. })
        ));
    }

    #[test]
    fn year_month_duration_divided_by_year_month_duration() {
        let a = ymd(XsdDatatype::YearMonthDuration, "P1Y");
        let b = ymd(XsdDatatype::YearMonthDuration, "P6M");
        let q = divide_year_month_durations(&a, &b).unwrap();
        assert_eq!(q.canonical_lexical(), "2");
    }

    // ── Component accessors ────────────────────────────────────────────────────────

    #[test]
    fn duration_component_accessors() {
        let d = parse_duration(XsdDatatype::Duration, "P1Y2M3DT4H5M6S").unwrap();
        assert_eq!(d.months(), 14); // 1*12 + 2
        // 3 days + 4h + 5m + 6s = 259200 + 14400 + 300 + 6 = 273906
        assert_eq!(d.seconds().canonical_lexical(), "273906");
    }
}
