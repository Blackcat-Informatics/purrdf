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
//!
//! # Duration ↔ calendar arithmetic: two call surfaces, one dispatch path
//!
//! This module exposes two surfaces for adding/subtracting a duration against a
//! calendar value. The F&O-named single-component functions
//! (`add_year_month_duration_to_datetime`, `add_day_time_duration_to_date`, and
//! their seven siblings) stay published for callers who hold a duration statically
//! known to be `yearMonthDuration`- or `dayTimeDuration`-shaped and want F&O's own
//! operator names. The general `add_duration_to_{datetime,date,time}` /
//! `subtract_duration_from_{datetime,date,time}` functions accept any
//! `xsd:duration` value — including one whose months AND seconds components are
//! both nonzero, which no single-component function can order correctly — and are
//! the functions a caller dispatching on a bare `xsd:duration` tag should call.
//! The single-component functions are deliberately kept even where nothing in this
//! workspace calls them: they remain the published F&O surface of the crate, not
//! dead code.
//!
//! `add_duration_to_gregorian` / `subtract_duration_from_gregorian` complete the
//! same general family for the five Gregorian types (`gYearMonth`, `gYear`,
//! `gMonth`, `gMonthDay`, `gDay`), which F&O does not define single-component
//! operators for at all. They refuse rather than fabricate an absent field —
//! see [`add_duration_to_gregorian`]'s own doc for the one deliberate,
//! spec-justified exception.

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::numeric::{Decimal, align_decimals, decimal_div_raw, decimal_mul_raw, parse_decimal};
use crate::value::XsdError;

/// Maximum timezone offset magnitude in minutes (±14:00).
const MAX_TZ_MIN: i32 = 14 * 60;
/// ±14:00 expressed in seconds, for the no-timezone comparison bound.
const TZ_BOUND_SECS: i128 = 14 * 3600;
const SECS_PER_DAY: i128 = 86_400;
/// A non-leap probe year, shared by every Gregorian day-validity check that
/// needs "the fewest days a given month can have" — a plain, non-century,
/// non-leap year gives every month (including February) its minimum length in
/// a single lookup via [`days_in_month`], so `day <= days_in_month(NON_LEAP_PROBE_YEAR, m)`
/// is the general "does `day` exist in every year's version of month `m`"
/// test, not a February-only special case. Also one of the three probe years
/// in [`SecondAction::YearIndependentDays`]'s unanimity check — see that match
/// arm's doc for why three probes (not two) are required and why they
/// suffice.
const NON_LEAP_PROBE_YEAR: i64 = 2001;
/// A leap probe year. Paired with [`NON_LEAP_PROBE_YEAR`] and
/// [`THIRD_PROBE_YEAR`] by [`SecondAction::YearIndependentDays`]'s
/// three-probe unanimity check, and reused (with [`FEB29_ALT_PROBE_YEAR`]) as
/// one of the two probes for a `--02-29` source specifically.
const LEAP_PROBE_YEAR: i64 = 2000;
/// A non-leap probe year immediately followed by a leap year (2003 → 2004).
/// [`LEAP_PROBE_YEAR`] (2000) is followed by non-leap 2001, and
/// [`NON_LEAP_PROBE_YEAR`] (2001) is followed by non-leap 2002; between the
/// three probe years, every reachable (this-year, next-year) leap pattern —
/// (leap, non-leap), (non-leap, leap), (non-leap, non-leap) — is realized.
/// (leap, leap) is unreachable: two Gregorian leap years are never
/// consecutive.
const THIRD_PROBE_YEAR: i64 = 2003;
/// A second leap probe year, distinct from [`LEAP_PROBE_YEAR`] and chosen on
/// the far side of a century boundary that is itself divisible by 400 (so
/// still leap), used only for a `--02-29` source. Every leap year's immediate
/// neighbors are non-leap (the same non-consecutive-leap-years fact as
/// [`THIRD_PROBE_YEAR`]'s doc), so a walk anchored at *any* leap year sees the
/// same (non-leap, non-leap) neighbor pattern: there is only one reachable
/// pattern for a Feb-29 source, and this second, unrelated leap year is
/// probed purely as a belt-and-braces check on the walk arithmetic itself
/// (including the divisible-by-400 century rule), not because a second
/// pattern exists to disagree.
const FEB29_ALT_PROBE_YEAR: i64 = 2400;

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

/// Which of a `Duration`'s two components carry a nonzero value. `range.rs` states
/// the fact this classification exists to encode once instead of scattering it:
/// "The `duration` family is ONE space because the `xsd:dayTimeDuration` and
/// `xsd:yearMonthDuration` value spaces overlap at the zero duration." A
/// division-commensurability rule (or any other duration-group operator) that
/// branches on `Shape` — via a `match` on `(Shape, Shape)`, never a derived `Ord` +
/// `max` — cannot invent an ordering between `Months` and `Seconds`, which are
/// genuinely incomparable summands of the direct sum `D = Y ⊕ T`.
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Both components are zero — the point where the two summands meet.
    Zero,
    /// Only the months component is nonzero.
    Months,
    /// Only the seconds component is nonzero.
    Seconds,
    /// Both components are nonzero; no single summand describes the value.
    Mixed,
}

impl Duration {
    /// The single construction point for every `Duration` value, in this module
    /// AND for any external caller that holds a `(months, seconds)` pair it
    /// computed by hand. Public precisely so such a caller — the motivating one
    /// being an aggregate fold that accumulates raw `(months, seconds)`
    /// components across many rows before ever needing a `Duration` at all
    /// (folding through this constructor at every intermediate row would wrongly
    /// reject a running total that is momentarily mixed-sign even though the
    /// FINAL total is not, since the value space's sign-coherence rule is a
    /// property of the result, not of every partial sum on the way to it) — can
    /// still validate its own final pair through the exact same two invariants
    /// every other `Duration`, whether parsed or computed by this module's own
    /// arithmetic, already goes through. There is no other way to build a
    /// `Duration` from raw components: the fields stay private and this crate
    /// exposes no raw-mantissa constructor for [`Decimal`] either, so an invalid
    /// pair is simply unconstructible, from inside this module or out.
    ///
    /// Enforces two invariants so that every other function on `Duration` — most
    /// importantly [`Duration::canonical_lexical`] — is correct by construction
    /// rather than merely aspirational:
    ///
    /// - **Sign coherence.** XSD 1.1 Part 2 §3.3.6 defines the duration value space
    ///   as the pair `(months, seconds)` with both components non-negative or both
    ///   non-positive; a pair with strictly opposite signs lies outside the range of
    ///   the lexical mapping, so there is no correct string to emit for it —
    ///   `canonical_lexical` would render `(months: 12, seconds: -86400)` as
    ///   `"-P1Y1D"`, which *denotes* `(-12, -86400)`, a silently wrong value. Making
    ///   such a pair unconstructible is cheaper and safer than trying to render it.
    /// - **Tag/component coherence.** The same pattern-facet rule `parse_duration`
    ///   enforces on the lexical form (XSD 1.1 Part 2 §3.4.26–§3.4.27): a value
    ///   tagged `yearMonthDuration` must have a zero `seconds` component, and one
    ///   tagged `dayTimeDuration` must have a zero `months` component. Applying the
    ///   rule here as well means a `Duration` can never be *computed* into a state
    ///   `parse_duration` would refuse to parse.
    ///
    /// # Examples
    ///
    /// ```
    /// use purrdf_xsd::XsdDatatype;
    /// use purrdf_xsd::numeric::parse_decimal;
    /// use purrdf_xsd::temporal::Duration;
    ///
    /// // 12 months, zero seconds: sign-coherent (seconds is zero, so only the
    /// // months sign matters), constructs fine.
    /// let d = Duration::new(12, parse_decimal("0").unwrap(), XsdDatatype::Duration).unwrap();
    /// assert_eq!(d.canonical_lexical(), "P1Y");
    ///
    /// // 1 month against -1 second: strictly opposite signs lie outside the
    /// // value space's lexical mapping, so construction is refused outright
    /// // rather than rendering a wrong string.
    /// let neg_one_sec = parse_decimal("-1").unwrap();
    /// assert!(Duration::new(1, neg_one_sec, XsdDatatype::Duration).is_err());
    /// ```
    pub fn new(months: i64, seconds: Decimal, datatype: XsdDatatype) -> Result<Self, XsdError> {
        let m_sign = i128::from(months.signum());
        let s_sign = seconds.mantissa().signum();
        if m_sign != 0 && s_sign != 0 && m_sign != s_sign {
            return Err(arith_overflow(
                datatype,
                "duration months and seconds components have differing signs",
            ));
        }
        match datatype {
            XsdDatatype::YearMonthDuration if !seconds.is_zero() => {
                return Err(arith_overflow(
                    datatype,
                    "yearMonthDuration must have a zero seconds component",
                ));
            }
            XsdDatatype::DayTimeDuration if months != 0 => {
                return Err(arith_overflow(
                    datatype,
                    "dayTimeDuration must have a zero months component",
                ));
            }
            _ => {}
        }
        Ok(Self {
            months,
            seconds,
            datatype,
        })
    }

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

    /// Classify which of `self`'s two components are nonzero. See [`Shape`].
    /// `Decimal::is_zero` is mantissa-only and scale-insensitive, so this
    /// classification is unaffected by which scale `self.seconds` happens to carry.
    fn shape(&self) -> Shape {
        match (self.months == 0, self.seconds.is_zero()) {
            (true, true) => Shape::Zero,
            (false, true) => Shape::Months,
            (true, false) => Shape::Seconds,
            (false, false) => Shape::Mixed,
        }
    }

    /// Canonical lexical form `[-]PnYnMnDTnHnMnS` (general duration grammar).
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        // `Duration::new` refuses to construct a mixed-sign (months, seconds) pair
        // (XSD 1.1 Part 2 §3.3.6 puts such pairs outside the lexical mapping's
        // range), so this function may assume sign coherence rather than needing to
        // guard against it — the invariant is load-bearing for the `neg` line below.
        debug_assert!(
            i128::from(self.months.signum()) * self.seconds.mantissa().signum() != -1,
            "Duration::new must enforce sign coherence: months and seconds may not have strictly opposite signs"
        );
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
        // The zero duration canonicalizes to "PT0S" — except for
        // `yearMonthDuration`, whose pattern facet ([^DT]*, XSD 1.1 Part 2
        // §3.4.27) forbids both 'D' and 'T', so its zero canonicalizes to "P0M".
        if out == "P" || out == "-P" {
            out.push_str(match self.datatype {
                XsdDatatype::YearMonthDuration => "0M",
                _ => "T0S",
            });
        }
        out
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

    // XSD 1.1 Part 2 pattern facets on the two named subtypes (§3.4.26–§3.4.27):
    // `yearMonthDuration` is `[^DT]*` — no day component and no time part at all;
    // `dayTimeDuration` is `[^YM]*(T.*)?` — no year/month component in the date
    // part (an `M` inside the *time* part denotes minutes and is unrestricted).
    // `xsd:duration` itself carries no pattern facet and accepts every component.
    match dt {
        XsdDatatype::YearMonthDuration if date_part.contains('D') || time_part.is_some() => {
            return Err(invalid(
                dt,
                s,
                "yearMonthDuration must not have a day or time component",
            ));
        }
        XsdDatatype::DayTimeDuration if date_part.contains('Y') || date_part.contains('M') => {
            return Err(invalid(
                dt,
                s,
                "dayTimeDuration must not have a year or month component",
            ));
        }
        _ => {}
    }

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
                'Y' => {
                    let added = n.checked_mul(12).ok_or_else(|| XsdError::OutOfRange {
                        datatype: dt,
                        lexical: s.to_string(),
                        reason: "duration months overflow",
                    })?;
                    months = months
                        .checked_add(added)
                        .ok_or_else(|| XsdError::OutOfRange {
                            datatype: dt,
                            lexical: s.to_string(),
                            reason: "duration months overflow",
                        })?;
                }
                'M' => {
                    months = months.checked_add(n).ok_or_else(|| XsdError::OutOfRange {
                        datatype: dt,
                        lexical: s.to_string(),
                        reason: "duration months overflow",
                    })?;
                }
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
        months = months.checked_neg().ok_or_else(|| XsdError::OutOfRange {
            datatype: dt,
            lexical: s.to_string(),
            reason: "duration months overflow",
        })?;
        let negated_mantissa =
            total_secs
                .mantissa()
                .checked_neg()
                .ok_or_else(|| XsdError::OutOfRange {
                    datatype: dt,
                    lexical: s.to_string(),
                    reason: "duration seconds overflow",
                })?;
        total_secs = Decimal::from_parts(negated_mantissa, total_secs.scale());
    }
    Duration::new(months, total_secs, dt)
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
///
/// ## Why `<`/`>` is indeterminate but `=` is total (see [`duration_equal`])
///
/// This function and [`duration_equal`] answer two genuinely different questions
/// over the same (months, seconds) pair, and the difference is not an oversight:
///
/// - XPath F&O defines `lt`/`gt` on durations **only for the two named
///   subtypes** (`dayTimeDuration`, `yearMonthDuration`), each of which is
///   totally ordered on its own. Comparing across incommensurable months/seconds
///   — `"P1M"` vs `"P30D"` — has no defined order, so this function's `None` is a
///   **spec-mandated outcome**, exactly like every other `None` this crate
///   returns from a `cmp_*` function (see the crate-level docs).
/// - XPath F&O's `op:duration-equal`, by contrast, is defined over the general
///   `xs:duration` and is **total**: it compares months and seconds
///   componentwise and always returns a boolean, never an error, even for
///   incommensurable pairs. `"P1M" = "P30D"` is `false`, not indeterminate.
///
/// So `cmp_duration(a, b) == Some(Ordering::Equal)` and `duration_equal(a, b)`
/// always agree (equality is the one relation both functions define the same
/// way), but a `None` from this function does **not** imply `duration_equal`
/// also has nothing to say — `duration_equal` always has an answer.
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

/// `xs:duration` value-space equality (XPath F&O `op:duration-equal`) — **total**,
/// unlike [`cmp_duration`]'s partial order. See that function's doc comment for
/// the full explanation of why `=` and `<`/`>` genuinely differ here.
///
/// Componentwise on `(months, seconds)`, with **no datatype-tag gate**:
/// `op:duration-equal`'s parameter type is the general `xs:duration`, so
/// cross-subtype pairs (e.g. a `yearMonthDuration` against a `dayTimeDuration`)
/// are in scope and compare by value, exactly like [`cmp_duration`]'s zero-pair
/// case. `"P1M" = "P30D"` is `false` — never an error.
///
/// Seconds equality goes through [`Decimal::cmp_exact`], never `==`: `Decimal`
/// deliberately derives no `PartialEq` because two different `(mantissa, scale)`
/// pairs (e.g. mantissa 10 scale 1, and mantissa 1 scale 0) denote the same
/// value, and `cmp_exact` is the only correct equality over that representation.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, value_cmp, value_equal};
///
/// let p1m = parse("P1M", XsdDatatype::YearMonthDuration)?;
/// let p30d = parse("P30D", XsdDatatype::DayTimeDuration)?;
///
/// // Total: always an answer, never an error.
/// assert_eq!(value_equal(&p1m, &p30d), Some(false));
/// // Partial: incommensurable components have no defined order.
/// assert_eq!(value_cmp(&p1m, &p30d), None);
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn duration_equal(a: &Duration, b: &Duration) -> bool {
    a.months == b.months && a.seconds.cmp_exact(&b.seconds).is_eq()
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
/// supply — that case is a hard `Indeterminate`, not a best-effort answer.
/// Both-timezoned and both-untimezoned pairs are always determinate.
fn instant_diff(datatype: XsdDatatype, a: &Instant, b: &Instant) -> Result<Duration, XsdError> {
    if a.tz.is_some() != b.tz.is_some() {
        return Err(XsdError::Indeterminate {
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

    Duration::new(
        0,
        Decimal::from_parts(diff, scale),
        XsdDatatype::DayTimeDuration,
    )
}

/// Require `dur`'s seconds component to be zero — the value-level condition XSD
/// 1.1 Part 2 §3.4.27's `[^DT]*` pattern facet guarantees for every
/// `yearMonthDuration`. Parse-time facet enforcement makes the declared tag and
/// the value's components coincide for the two named subtypes, so checking the
/// component rather than the tag is a strict relaxation of the old tag-only guard:
/// every `yearMonthDuration` still passes, and the general `xsd:duration` tag now
/// passes too whenever its seconds component happens to be zero.
fn require_year_month_duration(dur: &Duration) -> Result<(), XsdError> {
    if dur.seconds.is_zero() {
        Ok(())
    } else {
        Err(XsdError::TypeMismatch {
            reason: "operation requires a duration with a zero seconds component (yearMonthDuration-shaped)",
        })
    }
}

/// Require `dur`'s months component to be zero — the value-level condition XSD 1.1
/// Part 2 §3.4.26's `[^YM]*(T.*)?` pattern facet guarantees for every
/// `dayTimeDuration`. See [`require_year_month_duration`] for why checking the
/// component rather than the tag is a strict relaxation of the old guard.
fn require_day_time_duration(dur: &Duration) -> Result<(), XsdError> {
    if dur.months == 0 {
        Ok(())
    } else {
        Err(XsdError::TypeMismatch {
            reason: "operation requires a duration with a zero months component (dayTimeDuration-shaped)",
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

// ── Calendar-action classifier ────────────────────────────────────────────────────

/// How a temporal sort receives a duration's MONTHS component (XML Schema Appendix
/// E's addition table, generalized to the Gregorian family below it).
///
/// Durations form a partially ordered abelian group `D = Y ⊕ T`, the direct sum of
/// a months summand and a seconds summand meeting only at zero. Instants are a
/// genuine **torsor** under the seconds summand `T` — free, transitive, invertible,
/// which is why `dateTime − dateTime` always yields a determinate `dayTimeDuration`
/// — but only a **non-associative retraction** under the months summand `Y`:
/// `(2024-01-31 + P1M) + P1M ≠ 2024-01-31 + P2M`. That non-associativity is the
/// entire reason a duration must be applied months-first-then-seconds
/// (XML Schema Appendix E) rather than either order being equally valid, and it is
/// why the eight temporal sorts need a genuine classification of "how" a months
/// component is received rather than one free action shared by all of them.
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum MonthAction {
    /// The months component shifts the sort's absolute month count with no
    /// reference month to clamp against (`ℤ` acts freely) — `xsd:gYearMonth`.
    Free,
    /// The months component shifts year+month, then the day-of-month is
    /// clamped to the target month's last day: a real calendar year is
    /// known, so the clamp is always a real value, never a fabrication —
    /// `dateTime`/`date`, the two instant sorts, driven through [`drive`].
    Clamped,
    /// The months component shifts a month with no year attached
    /// (`gMonthDay`), so — unlike [`MonthAction::Clamped`] — there is no real
    /// calendar year to clamp the day-of-month against, and picking one to
    /// clamp against would silently fabricate a year the value never
    /// carried. Instead, the shift is accepted, with the day clamped to the
    /// target month's last day exactly as [`MonthAction::Clamped`] would,
    /// only when that clamped result is the same from **every** year the
    /// source `(month, day)` can be anchored at; a target month whose
    /// clamped result would depend on the anchor year is refused outright
    /// rather than guessed. Driven through [`drive_gregorian`], never
    /// through [`drive`] — no Gregorian sort has a `CalendarPoint` to clamp
    /// within.
    ClampIfYearIndependent,
    /// The months component acts through the quotient map `ℤ → ℤ/12`, not a
    /// dropped carry: XSD 1.1 Part 2 §3.3.13 makes `gMonth` a recurring month with
    /// no year field, so there is no year to carry into and no field is fabricated
    /// by reducing modulo 12 — `xsd:gMonth`.
    Cyclic12,
    /// The months component must lie in the subgroup `12ℤ` (a whole number of
    /// years) — `xsd:gYear` has no month field to receive a fractional-year shift.
    Divisible12,
    /// The months component is accepted unchanged only when doing so cannot alter
    /// the value's denotation — `xsd:gDay`, whose day exists in every month only
    /// for `day <= 28`; larger days depend on a month this sort does not carry.
    IdentityIfSafe,
    /// The sort has no months field at all — a nonzero months component is a type
    /// error (`xsd:time`).
    Absent,
}

/// How a temporal sort receives a duration's SECONDS component. See
/// [`MonthAction`]'s doc for the algebraic reason months and seconds need
/// independent classification rather than one shared action.
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum SecondAction {
    /// The seconds component shifts the full (date, time-of-day) point with no
    /// truncation — `xsd:dateTime`.
    Free,
    /// The seconds component is applied at midnight and only the resulting date is
    /// kept; a sub-day remainder never errors — `xsd:date`.
    MidnightTruncating,
    /// The seconds component shifts the time-of-day, wrapping across an implicit
    /// day boundary that is discarded afterward — `xsd:time`.
    CyclicDay,
    /// The seconds component shifts a day count that is accepted only when the
    /// shift's outcome does not depend on which year it starts in — `xsd:gMonthDay`.
    YearIndependentDays,
    /// The seconds component shifts a day count that is accepted only when the
    /// shift's outcome does not depend on which month it starts in — `xsd:gDay`.
    MonthIndependentDays,
    /// The sort has no seconds field at all — a nonzero seconds component is a
    /// type error (`xsd:gYearMonth`, `xsd:gYear`, `xsd:gMonth`).
    Absent,
}

/// Classify `datatype`'s reception of a duration's months and seconds components
/// (XML Schema Appendix E's addition table, generalized to the Gregorian family —
/// see [`MonthAction`]/[`SecondAction`]).
///
/// The match is exhaustive over **every** [`XsdDatatype`] variant, not just the
/// eight temporal sorts that receive a nonzero component: every non-temporal
/// datatype genuinely has no months field and no seconds field, so it classifies
/// as `(Absent, Absent)` — the same "no such field" answer `xsd:time` gives for
/// months. `XsdDatatype` is `#[non_exhaustive]` to downstream crates only; this
/// module is the defining crate, so the no-wildcard match is legal and total
/// coverage means adding a new `XsdDatatype` variant anywhere in the crate is a
/// compile error here until it is classified, not a silent gap.
const fn actions(datatype: XsdDatatype) -> (MonthAction, SecondAction) {
    match datatype {
        XsdDatatype::DateTime => (MonthAction::Clamped, SecondAction::Free),
        XsdDatatype::Date => (MonthAction::Clamped, SecondAction::MidnightTruncating),
        XsdDatatype::Time => (MonthAction::Absent, SecondAction::CyclicDay),
        XsdDatatype::GYearMonth => (MonthAction::Free, SecondAction::Absent),
        XsdDatatype::GYear => (MonthAction::Divisible12, SecondAction::Absent),
        XsdDatatype::GMonth => (MonthAction::Cyclic12, SecondAction::Absent),
        XsdDatatype::GMonthDay => (
            MonthAction::ClampIfYearIndependent,
            SecondAction::YearIndependentDays,
        ),
        XsdDatatype::GDay => (
            MonthAction::IdentityIfSafe,
            SecondAction::MonthIndependentDays,
        ),
        XsdDatatype::Integer
        | XsdDatatype::Long
        | XsdDatatype::Int
        | XsdDatatype::Short
        | XsdDatatype::Byte
        | XsdDatatype::UnsignedLong
        | XsdDatatype::UnsignedInt
        | XsdDatatype::UnsignedShort
        | XsdDatatype::UnsignedByte
        | XsdDatatype::NonNegativeInteger
        | XsdDatatype::PositiveInteger
        | XsdDatatype::NonPositiveInteger
        | XsdDatatype::NegativeInteger
        | XsdDatatype::Decimal
        | XsdDatatype::Float
        | XsdDatatype::Double
        | XsdDatatype::Boolean
        | XsdDatatype::String
        | XsdDatatype::Duration
        | XsdDatatype::DayTimeDuration
        | XsdDatatype::YearMonthDuration
        | XsdDatatype::HexBinary
        | XsdDatatype::Base64Binary => (MonthAction::Absent, SecondAction::Absent),
    }
}

/// Apply `dur` to `point`: the months component first (with the day-of-month
/// clamped against the target month, XML Schema Appendix E), then the seconds
/// component — the order is load-bearing, see [`MonthAction`]'s doc. A zero
/// component is skipped entirely regardless of its action, which is what makes
/// "zero duration → identity" uniform across every sort instead of a case worked
/// out per sort, and what spares a pure-seconds duration `shift_year_month`'s
/// multiplication for a no-op month step.
///
/// `month_action`/`second_action` come from [`actions`]. Only
/// [`MonthAction::Clamped`]/[`MonthAction::Absent`] and [`SecondAction::Free`]/
/// [`SecondAction::MidnightTruncating`]/[`SecondAction::CyclicDay`] are executed
/// here, for `dateTime`/`date`/`time`; the remaining actions — including
/// [`MonthAction::ClampIfYearIndependent`], `gMonthDay`'s own month-end action
/// — belong to the Gregorian family, whose values carry optional fields and no
/// time-of-day, so they are not shaped like a `CalendarPoint` and are not
/// driven through it; [`actions`] never pairs a `CalendarPoint` sort with
/// [`MonthAction::ClampIfYearIndependent`], so this function structurally
/// never receives it in practice, and the match arm below exists only to keep
/// the match total.
fn drive(
    datatype: XsdDatatype,
    month_action: MonthAction,
    second_action: SecondAction,
    point: CalendarPoint,
    dur: &Duration,
) -> Result<CalendarPoint, XsdError> {
    let point = if dur.months == 0 {
        point
    } else {
        match month_action {
            MonthAction::Clamped => {
                let (y, m) = shift_year_month(datatype, point.year, point.month, dur.months)?;
                let day = point.day.min(days_in_month(y, m));
                CalendarPoint {
                    year: y,
                    month: m,
                    day,
                    ..point
                }
            }
            MonthAction::Absent => {
                return Err(XsdError::TypeMismatch {
                    reason: "this temporal sort has no months field to receive a duration's months component",
                });
            }
            MonthAction::Free
            | MonthAction::Cyclic12
            | MonthAction::Divisible12
            | MonthAction::IdentityIfSafe
            | MonthAction::ClampIfYearIndependent => {
                return Err(XsdError::TypeMismatch {
                    reason: "this calendar action applies only to the Gregorian family, which this driver does not carry a CalendarPoint for",
                });
            }
        }
    };
    if dur.seconds.is_zero() {
        return Ok(point);
    }
    match second_action {
        SecondAction::Free | SecondAction::MidnightTruncating | SecondAction::CyclicDay => {
            let (y, m, d, h, mi, s) = add_seconds_decimal(datatype, &point, &dur.seconds)?;
            Ok(CalendarPoint {
                year: y,
                month: m,
                day: d,
                hour: h,
                minute: mi,
                second: s,
            })
        }
        SecondAction::Absent
        | SecondAction::YearIndependentDays
        | SecondAction::MonthIndependentDays => Err(XsdError::TypeMismatch {
            reason: "this temporal sort has no free-form seconds field for a duration's seconds component",
        }),
    }
}

/// Convert a duration's seconds component into a whole day count, or reject a
/// sub-day remainder. Universal precondition across **all five** Gregorian
/// sorts, checked before any [`SecondAction`]-specific dispatch: no Gregorian
/// type has an hour/minute/second field (XSD 1.1 Part 2 §3.3.9–§3.3.13), so —
/// unlike the reference implementation, which silently drops such precision —
/// this crate refuses to fabricate a day count that ignores it.
fn gregorian_whole_days(datatype: XsdDatatype, seconds: &Decimal) -> Result<i64, XsdError> {
    let day_unit = 10i128
        .pow(u32::from(seconds.scale()))
        .checked_mul(SECS_PER_DAY)
        .ok_or_else(|| arith_overflow(datatype, "gregorian day-count overflow"))?;
    let mantissa = seconds.mantissa();
    if mantissa.rem_euclid(day_unit) != 0 {
        return Err(XsdError::TypeMismatch {
            reason: "no Gregorian type has a time-of-day field: a duration's seconds component must be an exact whole number of days",
        });
    }
    let whole = mantissa.div_euclid(day_unit);
    i64::try_from(whole).map_err(|_| arith_overflow(datatype, "gregorian day-count overflow"))
}

/// A Gregorian value's `(year, month, day)` fields, the shape [`drive_gregorian`]
/// takes and returns — named so its signature does not trip
/// `clippy::type_complexity`.
type GregorianFields = (Option<i64>, Option<u8>, Option<u8>);

/// Apply `dur` to a Gregorian value's `(year, month, day)` fields through the
/// calendar-action classifier ([`actions`]). Companion to [`drive`], which
/// serves `dateTime`/`date`/`time`: Gregorian values carry optional fields and
/// no time-of-day, so they are not shaped like a [`CalendarPoint`] and are
/// driven separately here, over the same [`MonthAction`]/[`SecondAction`]
/// vocabulary — which is why [`drive`]'s own match has a permanent (never
/// executed from that function) arm for every action realized below, and this
/// function returns the mirror-image permanent `TypeMismatch` for the three
/// [`SecondAction`] variants ([`SecondAction::Free`],
/// [`SecondAction::MidnightTruncating`], [`SecondAction::CyclicDay`]) and the
/// two [`MonthAction`] variants ([`MonthAction::Absent`], and
/// [`MonthAction::Clamped`] — `dateTime`/`date`'s own instant-only clamp,
/// distinct from `gMonthDay`'s [`MonthAction::ClampIfYearIndependent`] below)
/// that no Gregorian sort's classification ever selects.
///
/// Months are applied first, then days — the order is load-bearing, see
/// [`MonthAction`]'s doc — and a zero component is skipped entirely, which is
/// what makes "zero duration → identity" uniform here too.
fn drive_gregorian(
    datatype: XsdDatatype,
    month_action: MonthAction,
    second_action: SecondAction,
    year: Option<i64>,
    month: Option<u8>,
    day: Option<u8>,
    dur: &Duration,
) -> Result<GregorianFields, XsdError> {
    let (year, month, day) = if dur.months == 0 {
        (year, month, day)
    } else {
        match month_action {
            MonthAction::Free => {
                // gYearMonth: `ℤ` acts freely on absolute months — exact,
                // checked arithmetic via the same `shift_year_month` the
                // CalendarPoint driver uses for `dateTime`/`date`.
                let (y, m) =
                    shift_year_month(datatype, year.unwrap_or(0), month.unwrap_or(1), dur.months)?;
                (Some(y), Some(m), day)
            }
            MonthAction::Divisible12 => {
                // gYear: the subgroup 12ℤ — a duration must be a whole number
                // of years, or gYear has no month field to receive the
                // remainder.
                if dur.months % 12 != 0 {
                    return Err(XsdError::TypeMismatch {
                        reason: "gYear has no month field: a duration's months component must be a whole number of years",
                    });
                }
                let y = year
                    .unwrap_or(0)
                    .checked_add(dur.months / 12)
                    .ok_or_else(|| arith_overflow(datatype, "gYear arithmetic overflow"))?;
                (Some(y), None, day)
            }
            MonthAction::Cyclic12 => {
                // gMonth: the quotient map ℤ → ℤ/12, not a dropped carry —
                // XSD 1.1 Part 2 §3.3.13 makes gMonth a recurring month with
                // no year field, so there is no year to carry into and no
                // field is fabricated by reducing modulo 12.
                //
                // Overflow-safety argument (style of `Decimal::cmp_exact`,
                // numeric.rs:73-88): reduce FIRST. `dur.months.rem_euclid(12)`
                // of any `i64` is in `0..=11`; `month - 1` (month in
                // `1..=12`) is in `0..=11`; their sum is therefore in
                // `0..=22`, strictly within `i64` range, so plain
                // (non-checked) arithmetic is provably safe here — there is
                // no overflow path to guard, for any `i64` months value.
                let m = month.unwrap_or(1);
                let m0 = i64::from(m) - 1 + dur.months.rem_euclid(12);
                (None, Some((m0 % 12 + 1) as u8), day)
            }
            MonthAction::IdentityIfSafe => {
                // gDay: day <= 28 exists in every month, so a months shift
                // cannot change what the value denotes; day >= 29 depends on
                // a month gDay does not carry.
                let d = day.unwrap_or(1);
                if d <= 28 {
                    (year, month, day)
                } else {
                    return Err(XsdError::TypeMismatch {
                        reason: "gDay day >= 29 cannot safely receive a duration's months component: the clamp would depend on the unknown month",
                    });
                }
            }
            MonthAction::ClampIfYearIndependent => {
                // gMonthDay. Recover the target month AND how many whole
                // years the shift carries into. `dur.months`'s own
                // div_euclid/rem_euclid split off the (unboundedly large but
                // overflow-safe, because dividing only shrinks a value)
                // year-carry component first, leaving a small `0..=22`
                // remainder sum to reduce a second time — the same
                // reduce-first trick `Cyclic12` above uses, extended one
                // step further so the exact year carry is recovered too,
                // without ever adding the raw (possibly `i64::MAX`-sized)
                // `dur.months` to anything unchecked.
                let m = month.unwrap_or(1);
                let d = day.unwrap_or(1);
                let months_div = dur.months.div_euclid(12);
                let months_rem = dur.months.rem_euclid(12); // 0..=11
                let combined = i64::from(m) - 1 + months_rem; // 0..=22, proven safe above
                let target_month = (combined % 12 + 1) as u8;
                let carry_years = months_div
                    .checked_add(combined / 12) // combined / 12 is 0 or 1
                    .ok_or_else(|| arith_overflow(datatype, "gMonthDay arithmetic overflow"))?;

                // XML Schema Appendix E clamps a day that does not exist in
                // the shifted month down to that month's actual length. The
                // question this rule answers is whether the CLAMPED result
                // is the same from every year the source (month, day) can be
                // anchored at — not, as the previous rule conflated, whether
                // the unclamped day exists in the target month at all.
                // `--03-31 + P1M` makes the difference concrete: April never
                // has a 31st, but April's length (30 days) never depends on
                // the anchor year, so the clamped result --04-30 is
                // identical from every anchor and the shift is genuinely
                // year-independent — unlike the previous rule's outright
                // refusal of it.
                //
                // A non-February target month's length is the same in every
                // year, so its clamp result never depends on the anchor:
                // accept unconditionally, clamping only if the source day
                // overshoots.
                //
                // A February target's length depends on the TARGET year's
                // leapness — `anchor_year + carry_years`, not the anchor
                // year by itself — so the question becomes whether that is
                // constant as the anchor ranges over every year the source
                // (month, day) is real.
                //
                // For every source other than `--02-29`, every year is a
                // real anchor (the day-shift arm's doc gives the same fact:
                // day <= 28 always exists, and a day of 29-31 outside
                // February exists regardless of leap status). So
                // `anchor_year + carry_years` ranges over every integer too
                // — both a leap and a non-leap target year are certainly
                // reachable whenever the clamp can even tell them apart,
                // i.e. whenever the source day is 29 or more. Refuse in that
                // case; a source day <= 28 needs no clamp and is unaffected
                // by which February it lands in, so accept.
                //
                // For a `--02-29` source, only leap years are real anchors,
                // and the reachable target-year leapness is no longer
                // "every pattern is reachable": the Gregorian calendar's
                // leap rule has an EXACT period of 400 years —
                // `leap(Y) == leap(Y + 400)` for every `Y`, because 400 is a
                // multiple of 4, of 100, and of 400 simultaneously — so the
                // decision reduces to a closed form instead of a handful of
                // concrete probe years the way the day-shift arm's
                // per-February decision does:
                // - `carry_years` a multiple of 400: the target year sits at
                //   the same point in the 400-year cycle as the (leap)
                //   anchor, so it is leap for every anchor — accept, day 29.
                // - `carry_years` not a multiple of 4: the anchor is a
                //   multiple of 4 (it is leap), so the target year never is
                //   — accept, day 28.
                // - `carry_years` a multiple of 4 but not of 400: the target
                //   year is a multiple of 4 for every anchor, so it is leap
                //   unless it also lands on one of the three non-leap
                //   century marks a 400-year window contains. The 97
                //   reachable leap-anchor residues mod 400 (every multiple
                //   of 4 except those three marks), shifted by a nonzero
                //   offset that is itself not a multiple of 400, always
                //   relocates at least one residue onto a mark while leaving
                //   most of the others off it — both a leap and a non-leap
                //   target year are reachable, so reject.
                //
                // The day-shift arm never needs any of this: a civil-day
                // walk moves through real calendar days and never clamps,
                // so its only year-dependence question is which February a
                // walk crosses, not whether an intermediate day exists.
                if target_month == 2 && d >= 29 {
                    let source_is_feb29 = m == 2 && d == 29;
                    if source_is_feb29 && carry_years % 400 == 0 {
                        (
                            None,
                            Some(target_month),
                            Some(days_in_month(LEAP_PROBE_YEAR, 2)),
                        )
                    } else if source_is_feb29 && carry_years % 4 != 0 {
                        (
                            None,
                            Some(target_month),
                            Some(days_in_month(NON_LEAP_PROBE_YEAR, 2)),
                        )
                    } else {
                        return Err(XsdError::TypeMismatch {
                            reason: "gMonthDay: the shifted (month, day) depends on which year it starts in",
                        });
                    }
                } else {
                    let clamped = d.min(days_in_month(NON_LEAP_PROBE_YEAR, target_month));
                    (None, Some(target_month), Some(clamped))
                }
            }
            MonthAction::Absent => {
                return Err(XsdError::TypeMismatch {
                    reason: "this Gregorian sort has no months field to receive a duration's months component",
                });
            }
            MonthAction::Clamped => {
                return Err(XsdError::TypeMismatch {
                    reason: "this calendar action applies only to dateTime/date, which this driver does not carry a CalendarPoint for",
                });
            }
        }
    };
    if dur.seconds.is_zero() {
        return Ok((year, month, day));
    }
    let whole_days = gregorian_whole_days(datatype, &dur.seconds)?;
    match second_action {
        SecondAction::YearIndependentDays => {
            // gMonthDay. A single non-leap probe cannot decide this (a
            // 1461-day walk agrees between a leap and a non-leap probe by
            // 4-year-cycle coincidence yet is genuinely century-dependent),
            // so a shift of 365 days or more is refused outright rather than
            // trusted to probe agreement.
            if !(-364..=364).contains(&whole_days) {
                return Err(XsdError::TypeMismatch {
                    reason: "gMonthDay: a duration spanning 365 days or more cannot be shown year-independent by a bounded probe",
                });
            }
            let m = month.unwrap_or(1);
            let d = day.unwrap_or(1);
            // Walking at most 364 civil days crosses at most one February —
            // crossing two would need a walk of a full year or more, which
            // the guard above already refuses — and that crossed February
            // belongs either to the walk's start year or to a year
            // immediately adjacent to it (start - 1 for a backward walk that
            // reaches past January into the previous year, start + 1 for a
            // forward walk that reaches past December into the next year).
            // A shift is year-independent exactly when it computes the same
            // result from *every* year in which the source (month, day) is a
            // real anchor, so probing a set of valid anchor years and
            // requiring unanimous agreement is both necessary (a genuinely
            // year-independent shift agrees everywhere) and sufficient,
            // provided the probe set realizes every leap/non-leap pattern
            // the crossed February can actually show — anything left
            // unprobed could hide a disagreement, but nothing further needs
            // checking once every reachable pattern is unanimous.
            //
            // For every (month, day) except `--02-29`, the source date
            // exists in every proleptic-Gregorian year: day <= 28 always
            // exists, and a day of 29-31 in a month other than February
            // exists regardless of leap status, since February is the only
            // variable-length month. So any year is a valid anchor.
            // Adjacent-year leapness has exactly three reachable patterns —
            // (leap, non-leap), (non-leap, leap), (non-leap, non-leap);
            // (leap, leap) is impossible in the Gregorian calendar, since two
            // leap years are never consecutive. [`LEAP_PROBE_YEAR`] (2000,
            // followed by non-leap 2001), [`THIRD_PROBE_YEAR`] (2003,
            // followed by leap 2004), and [`NON_LEAP_PROBE_YEAR`] (2001,
            // followed by non-leap 2002) realize all three. Their own
            // leapness (leap, non-leap, non-leap) and previous-year leapness
            // (1999 non-leap, 2000 leap, 2002 non-leap) additionally range
            // over the same three patterns, so a February crossed backward
            // into the start year itself, or into the year before it, is
            // equally covered — unanimity across these three probes decides
            // every crossing this walk can produce.
            //
            // `--02-29` only exists in leap years, so only a leap year is a
            // valid anchor for it. This is why the day is no longer clamped
            // to each probe year's own month length before walking (as this
            // code used to do): clamping is not a shortcut, it silently
            // substitutes a different question ("what does Mar 1 minus n
            // days look like" instead of "what does Feb 29 plus n days look
            // like"). Restricting to leap anchors also collapses the
            // reachable pattern set: two Gregorian leap years are never
            // consecutive, so *every* leap year's immediate neighbors are
            // non-leap, and the only reachable (previous-year, next-year)
            // pattern for a `--02-29` source is (non-leap, non-leap) — there
            // is no second pattern to unify against. [`FEB29_ALT_PROBE_YEAR`]
            // (2400, itself leap via the divisible-by-400 exception) is
            // probed alongside [`LEAP_PROBE_YEAR`] purely as a check that the
            // walk arithmetic agrees across a century-rule boundary.
            let probe_years: &[i64] = if (m, d) == (2, 29) {
                &[LEAP_PROBE_YEAR, FEB29_ALT_PROBE_YEAR]
            } else {
                &[LEAP_PROBE_YEAR, THIRD_PROBE_YEAR, NON_LEAP_PROBE_YEAR]
            };
            let probe = |probe_year: i64| -> Result<(u8, u8), XsdError> {
                let base = days_from_civil(probe_year, m, d);
                let shifted = base
                    .checked_add(whole_days)
                    .ok_or_else(|| arith_overflow(datatype, "gMonthDay arithmetic overflow"))?;
                let (_, nm, nd) = civil_from_days(shifted);
                Ok((nm, nd))
            };
            let mut agreed: Option<(u8, u8)> = None;
            for &probe_year in probe_years {
                let result = probe(probe_year)?;
                match agreed {
                    None => agreed = Some(result),
                    Some(first) if first != result => {
                        return Err(XsdError::TypeMismatch {
                            reason: "gMonthDay: the shifted (month, day) depends on which year it starts in",
                        });
                    }
                    Some(_) => {}
                }
            }
            let (nm, nd) = agreed.expect("probe_years is never empty");
            Ok((None, Some(nm), Some(nd)))
        }
        SecondAction::MonthIndependentDays => {
            // gDay: source and target day must both exist in every month —
            // the only sort whose modulus (28-31) is non-constant, so its
            // guard is month-independence rather than year-independence.
            let d = day.unwrap_or(1);
            if d > 28 {
                return Err(XsdError::TypeMismatch {
                    reason: "gDay: source day >= 29 does not exist in every month, so a days shift cannot be shown month-independent",
                });
            }
            match i64::from(d).checked_add(whole_days) {
                Some(t) if (1..=28).contains(&t) => Ok((None, None, Some(t as u8))),
                _ => Err(XsdError::TypeMismatch {
                    reason: "gDay: shifted day does not exist in every month",
                }),
            }
        }
        SecondAction::Absent => Err(XsdError::TypeMismatch {
            reason: "this Gregorian sort has no day field to receive a duration's seconds component",
        }),
        SecondAction::Free | SecondAction::MidnightTruncating | SecondAction::CyclicDay => {
            Err(XsdError::TypeMismatch {
                reason: "this calendar action applies only to dateTime/date/time, which this driver carries no fields for",
            })
        }
    }
}

/// Negate a `Duration`'s months and seconds components together (unary minus).
/// A `Duration` is always sign-coherent by construction (`Duration::new`), and
/// negating both components together preserves that — it can never produce a
/// mixed-sign pair, so this function has exactly one arm. Subtraction throughout
/// the general-duration primitives below, and [`subtract_durations`] itself, are
/// both expressed as negate-then-add rather than as their own driver passes.
///
/// **PurRDF extension:** XPath F&O's unary minus is numeric-only
/// (`op:numeric-unary-minus`, §4.2.8) — F&O defines no duration unary minus, and
/// unary plus stays numeric-only in PurRDF too (`+(?duration)` is a type error
/// while `-(?duration)` is not; that asymmetry is deliberate, not an oversight).
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{negate_duration, parse_duration};
///
/// let d = parse_duration(XsdDatatype::YearMonthDuration, "P1Y2M").unwrap();
/// let negated = negate_duration(&d).unwrap();
/// assert_eq!(negated.canonical_lexical(), "-P1Y2M");
/// assert_eq!(negate_duration(&negated).unwrap().canonical_lexical(), "P1Y2M");
/// ```
pub fn negate_duration(dur: &Duration) -> Result<Duration, XsdError> {
    let months = dur
        .months
        .checked_neg()
        .ok_or_else(|| arith_overflow(dur.datatype, "duration negation overflow"))?;
    let seconds = decimal_negate(dur.datatype, &dur.seconds)?;
    Duration::new(months, seconds, dur.datatype)
}

/// Add any `xsd:duration` value — regardless of its declared subtype — to a
/// `dateTime`, applying the months component first (clamping the day-of-month
/// against the target month, XML Schema Appendix E) and the seconds component
/// second. This is the general form of [`add_year_month_duration_to_datetime`] and
/// [`add_day_time_duration_to_datetime`] combined: those two remain the F&O-named
/// single-component entry points, but only this function (and its five siblings
/// below) can correctly order a duration whose months and seconds are both
/// nonzero.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{add_duration_to_datetime, parse_datetime, parse_duration};
///
/// // Months are applied before days: Jan 30 clamps to Feb 29 (2024 is a leap
/// // year), THEN one day is added, landing on Mar 1 — not Feb 29.
/// let dt = parse_datetime("2024-01-30T00:00:00").unwrap();
/// let dur = parse_duration(XsdDatatype::Duration, "P1M1D").unwrap();
/// let result = add_duration_to_datetime(&dt, &dur).unwrap();
/// assert_eq!(result.canonical_lexical(), "2024-03-01T00:00:00");
/// ```
pub fn add_duration_to_datetime(dt: &DateTime, dur: &Duration) -> Result<DateTime, XsdError> {
    let (month_action, second_action) = actions(XsdDatatype::DateTime);
    let point = CalendarPoint {
        year: dt.year,
        month: dt.month,
        day: dt.day,
        hour: dt.hour,
        minute: dt.minute,
        second: dt.second,
    };
    let point = drive(
        XsdDatatype::DateTime,
        month_action,
        second_action,
        point,
        dur,
    )?;
    Ok(DateTime {
        year: point.year,
        month: point.month,
        day: point.day,
        hour: point.hour,
        minute: point.minute,
        second: point.second,
        tz: dt.tz,
    })
}

/// Subtract any `xsd:duration` value from a `dateTime`. See
/// [`add_duration_to_datetime`] for the accepted shapes and the months-before-
/// seconds ordering; subtraction is negate-then-add.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{parse_datetime, parse_duration, subtract_duration_from_datetime};
///
/// let dt = parse_datetime("2024-03-01T00:00:00").unwrap();
/// let dur = parse_duration(XsdDatatype::Duration, "P1Y1D").unwrap();
/// let result = subtract_duration_from_datetime(&dt, &dur).unwrap();
/// assert_eq!(result.canonical_lexical(), "2023-02-28T00:00:00");
/// ```
pub fn subtract_duration_from_datetime(
    dt: &DateTime,
    dur: &Duration,
) -> Result<DateTime, XsdError> {
    add_duration_to_datetime(dt, &negate_duration(dur)?)
}

/// Add any `xsd:duration` value to a `date`. The seconds component is applied at
/// midnight and only the resulting date is kept (matching
/// [`add_day_time_duration_to_date`]) — a sub-day remainder never errors.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{add_duration_to_date, parse_date, parse_duration};
///
/// let d = parse_date("2024-01-31").unwrap();
/// // A sub-day remainder never errors and never changes the date.
/// let unchanged =
///     add_duration_to_date(&d, &parse_duration(XsdDatatype::Duration, "PT1H").unwrap()).unwrap();
/// assert_eq!(unchanged.canonical_lexical(), "2024-01-31");
/// // A whole day rolls the date forward.
/// let rolled =
///     add_duration_to_date(&d, &parse_duration(XsdDatatype::Duration, "P1D").unwrap()).unwrap();
/// assert_eq!(rolled.canonical_lexical(), "2024-02-01");
/// ```
pub fn add_duration_to_date(d: &Date, dur: &Duration) -> Result<Date, XsdError> {
    let (month_action, second_action) = actions(XsdDatatype::Date);
    let point = CalendarPoint::midnight(d.year, d.month, d.day);
    let point = drive(XsdDatatype::Date, month_action, second_action, point, dur)?;
    Ok(Date {
        year: point.year,
        month: point.month,
        day: point.day,
        tz: d.tz,
    })
}

/// Subtract any `xsd:duration` value from a `date`. See [`add_duration_to_date`]
/// for the midnight-truncation rule; subtraction is negate-then-add.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{parse_date, parse_duration, subtract_duration_from_date};
///
/// let d = parse_date("2024-02-01").unwrap();
/// let result =
///     subtract_duration_from_date(&d, &parse_duration(XsdDatatype::Duration, "P1D").unwrap())
///         .unwrap();
/// assert_eq!(result.canonical_lexical(), "2024-01-31");
/// ```
pub fn subtract_duration_from_date(d: &Date, dur: &Duration) -> Result<Date, XsdError> {
    add_duration_to_date(d, &negate_duration(dur)?)
}

/// Add any `xsd:duration` value to a `time`. The duration's months component must
/// be zero — `xsd:time` has no months field (`MonthAction::Absent`) — while the
/// seconds component wraps across an implicit day boundary that is then discarded,
/// leaving only the resulting time-of-day.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{add_duration_to_time, parse_duration, parse_time};
///
/// let t = parse_time("23:00:00").unwrap();
/// let result =
///     add_duration_to_time(&t, &parse_duration(XsdDatatype::Duration, "PT2H").unwrap()).unwrap();
/// assert_eq!(result.canonical_lexical(), "01:00:00");
///
/// // A duration with a nonzero months component is a type error: `time` has no
/// // months field to receive it.
/// let months = parse_duration(XsdDatatype::Duration, "P1M").unwrap();
/// assert!(add_duration_to_time(&t, &months).is_err());
/// ```
pub fn add_duration_to_time(t: &Time, dur: &Duration) -> Result<Time, XsdError> {
    let (month_action, second_action) = actions(XsdDatatype::Time);
    let point = CalendarPoint::on_reference_date(t.hour, t.minute, t.second);
    let point = drive(XsdDatatype::Time, month_action, second_action, point, dur)?;
    Ok(Time {
        hour: point.hour,
        minute: point.minute,
        second: point.second,
        tz: t.tz,
    })
}

/// Subtract any `xsd:duration` value from a `time`. See [`add_duration_to_time`]
/// for the accepted shapes; subtraction is negate-then-add.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{parse_duration, parse_time, subtract_duration_from_time};
///
/// let t = parse_time("01:00:00").unwrap();
/// let result =
///     subtract_duration_from_time(&t, &parse_duration(XsdDatatype::Duration, "PT2H").unwrap())
///         .unwrap();
/// assert_eq!(result.canonical_lexical(), "23:00:00");
/// ```
pub fn subtract_duration_from_time(t: &Time, dur: &Duration) -> Result<Time, XsdError> {
    add_duration_to_time(t, &negate_duration(dur)?)
}

/// Add any `xsd:duration` value to a Gregorian value (`gYearMonth`, `gYear`,
/// `gMonth`, `gMonthDay`, `gDay`), through the calendar-action classifier
/// (`actions`). Where the reference implementation would fabricate an
/// absent field to force an answer (substituting year 0, January, or day 1
/// via JAXP — e.g. `"2024-01"^^gYearMonth + P1D` returning `"2024-01"`
/// unchanged, or `"---31"^^gDay + P1M` returning `"---29"` clamped against a
/// fictitious leap year 0), this function returns a typed error instead: it
/// matches the reference implementation only where its answer is genuinely
/// year-independent. The sole information-losing accept is `gMonth`'s months
/// component, which acts through the quotient map `ℤ → ℤ/12` (XSD 1.1 Part 2
/// §3.3.13 — `gMonth` has no year field for a carry to be dropped from, so
/// none is fabricated by reducing modulo 12).
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{add_duration_to_gregorian, parse_duration, parse_gregorian};
///
/// // RDF4J's own pinned Gregorian case: year-independent, so it succeeds.
/// let g = parse_gregorian(XsdDatatype::GYearMonth, "2012-10").unwrap();
/// let dur = parse_duration(XsdDatatype::Duration, "P1Y1M").unwrap();
/// let result = add_duration_to_gregorian(&g, &dur).unwrap();
/// assert_eq!(result.canonical_lexical(), "2013-11");
///
/// // `gDay` day 31 has no year-independent answer for "+P1M" (the shift
/// // would depend on a month `gDay` does not carry) — a typed error, not
/// // RDF4J's fabricated `---29` clamped against a fictitious leap year 0.
/// let day = parse_gregorian(XsdDatatype::GDay, "---31").unwrap();
/// let one_month = parse_duration(XsdDatatype::Duration, "P1M").unwrap();
/// assert!(add_duration_to_gregorian(&day, &one_month).is_err());
/// ```
pub fn add_duration_to_gregorian(g: &Gregorian, dur: &Duration) -> Result<Gregorian, XsdError> {
    let (month_action, second_action) = actions(g.datatype);
    let (year, month, day) = drive_gregorian(
        g.datatype,
        month_action,
        second_action,
        g.year,
        g.month,
        g.day,
        dur,
    )?;
    Ok(Gregorian {
        year,
        month,
        day,
        tz: g.tz,
        datatype: g.datatype,
    })
}

/// Subtract any `xsd:duration` value from a Gregorian value. See
/// [`add_duration_to_gregorian`] for the accepted shapes and the
/// fabrication-refusal rule; subtraction is negate-then-add.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{
///     parse_duration, parse_gregorian, subtract_duration_from_gregorian,
/// };
///
/// let g = parse_gregorian(XsdDatatype::GYear, "2025").unwrap();
/// let dur = parse_duration(XsdDatatype::Duration, "P1Y").unwrap();
/// let result = subtract_duration_from_gregorian(&g, &dur).unwrap();
/// assert_eq!(result.canonical_lexical(), "2024");
/// ```
pub fn subtract_duration_from_gregorian(
    g: &Gregorian,
    dur: &Duration,
) -> Result<Gregorian, XsdError> {
    add_duration_to_gregorian(g, &negate_duration(dur)?)
}

// ── Duration ↔ calendar arithmetic (XPath F&O §9.7.5–9.7.14; XML Schema Appendix E) ─
//
// The ten functions below are the F&O-named single-component surface (each takes a
// duration statically declared `yearMonthDuration` or `dayTimeDuration`). Nothing
// inside this workspace calls them — the general `add_duration_to_*` /
// `subtract_duration_from_*` functions above are what any caller dispatching on a
// bare `xsd:duration` tag uses, because only they can order a duration whose
// months and seconds are both nonzero. These ten stay published as the crate's
// F&O-named public surface; they are not dead code.

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

// ── Duration arithmetic (XPath F&O §8.4, extended to the general xsd:duration) ───

/// The result datatype for `a OP b` over the duration group: syntactic on the
/// operands' own *declared* tags, never on their computed (months, seconds)
/// components — `dayTimeDuration` iff both operands declare it, `yearMonthDuration`
/// iff both do, else the general `xsd:duration`. This is a plain `match`, the same
/// idiom [`Shape`]'s doc requires of it: durations do not carry a total order over
/// tags for `Ord`/`max` to invent one from.
fn duration_result_datatype(a: XsdDatatype, b: XsdDatatype) -> XsdDatatype {
    match (a, b) {
        (XsdDatatype::YearMonthDuration, XsdDatatype::YearMonthDuration) => {
            XsdDatatype::YearMonthDuration
        }
        (XsdDatatype::DayTimeDuration, XsdDatatype::DayTimeDuration) => {
            XsdDatatype::DayTimeDuration
        }
        _ => XsdDatatype::Duration,
    }
}

/// `op:add-yearMonthDurations` / `op:add-dayTimeDurations`, extended to accept any
/// `xsd:duration` operand: PurRDF follows RDF4J's permissiveness here rather than
/// F&O's restriction to the two named subtypes (F&O's (months, seconds) pair for
/// the general tag is not a mixing of incompatible units — it is exactly this
/// module's own `D = Y ⊕ T` direct sum). Componentwise checked addition through
/// `Duration::new`, so a mixed-sign result is refused at construction regardless
/// of which operator produced it — see `Duration::new`'s own doc for why the
/// guard cannot live in either operator alone. See `duration_result_datatype` for
/// the result's tag.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{add_durations, parse_duration};
///
/// let a = parse_duration(XsdDatatype::YearMonthDuration, "P1Y").unwrap();
/// let b = parse_duration(XsdDatatype::DayTimeDuration, "PT1H").unwrap();
/// let sum = add_durations(&a, &b).unwrap();
/// assert_eq!(sum.canonical_lexical(), "P1YT1H");
/// assert_eq!(sum.datatype(), XsdDatatype::Duration);
/// ```
pub fn add_durations(a: &Duration, b: &Duration) -> Result<Duration, XsdError> {
    let datatype = duration_result_datatype(a.datatype, b.datatype);
    let months = a
        .months
        .checked_add(b.months)
        .ok_or_else(|| arith_overflow(datatype, "duration addition overflow (months)"))?;
    let seconds = decimal_add_exact(datatype, &a.seconds, &b.seconds)?;
    Duration::new(months, seconds, datatype)
}

/// `op:subtract-yearMonthDurations` / `op:subtract-dayTimeDurations`, extended the
/// same way as [`add_durations`]. Expressed as negate-then-add through
/// [`negate_duration`] rather than its own componentwise pass, matching this
/// module's negate-then-add idiom for calendar subtraction.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{subtract_durations, parse_duration};
///
/// let a = parse_duration(XsdDatatype::YearMonthDuration, "P1Y").unwrap();
/// let b = parse_duration(XsdDatatype::YearMonthDuration, "P1Y").unwrap();
/// let diff = subtract_durations(&a, &b).unwrap();
/// // Zero canonicalizes per the operand subtype ("P0M", not "PT0S").
/// assert_eq!(diff.canonical_lexical(), "P0M");
/// assert_eq!(diff.datatype(), XsdDatatype::YearMonthDuration);
/// ```
pub fn subtract_durations(a: &Duration, b: &Duration) -> Result<Duration, XsdError> {
    add_durations(a, &negate_duration(b)?)
}

/// `op:multiply-yearMonthDuration` / `op:multiply-dayTimeDuration`, extended with a
/// general-`xsd:duration` arm that scales BOTH components, preserving the
/// operand's declared tag. `factor` is a `Decimal` rather than F&O's `xs:double`:
/// this crate keeps its stored values exact (no floats), so the multiplication
/// stays exact too. Months results are always rounded to the nearest whole month,
/// ties toward positive infinity (`round_decimal_to_i64`, matching `fn:round`,
/// since months cannot be fractional) — this is a deliberate, documented
/// non-inverse: `(d ÷ 2) × 2` need not equal `d` for an odd month count.
pub fn multiply_duration(dur: &Duration, factor: &Decimal) -> Result<Duration, XsdError> {
    match dur.datatype {
        XsdDatatype::YearMonthDuration => {
            let months_dec = Decimal::from_parts(i128::from(dur.months), 0);
            let product = decimal_mul_raw(&months_dec, factor).map_err(|_| {
                arith_overflow(dur.datatype, "yearMonthDuration multiplication overflow")
            })?;
            let months = round_decimal_to_i64(dur.datatype, &product)?;
            Duration::new(months, Decimal::from_parts(0, 0), dur.datatype)
        }
        XsdDatatype::DayTimeDuration => {
            let seconds = decimal_mul_raw(&dur.seconds, factor).map_err(|_| {
                arith_overflow(dur.datatype, "dayTimeDuration multiplication overflow")
            })?;
            Duration::new(0, seconds, dur.datatype)
        }
        _ => {
            let months_dec = Decimal::from_parts(i128::from(dur.months), 0);
            let months_product = decimal_mul_raw(&months_dec, factor).map_err(|_| {
                arith_overflow(dur.datatype, "duration multiplication overflow (months)")
            })?;
            let months = round_decimal_to_i64(dur.datatype, &months_product)?;
            let seconds = decimal_mul_raw(&dur.seconds, factor).map_err(|_| {
                arith_overflow(dur.datatype, "duration multiplication overflow (seconds)")
            })?;
            Duration::new(months, seconds, dur.datatype)
        }
    }
}

/// `op:divide-yearMonthDuration` / `op:divide-dayTimeDuration`, extended with a
/// general-`xsd:duration` arm that scales BOTH components (see
/// [`multiply_duration`], whose months-rounding rule and non-inverse note apply
/// identically here). `divisor` is a `Decimal` for the same exactness reason as
/// [`multiply_duration`]; a zero divisor is `Err(DivisionByZero)`.
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
            Duration::new(months, Decimal::from_parts(0, 0), dur.datatype)
        }
        XsdDatatype::DayTimeDuration => {
            let seconds = decimal_div_raw(&dur.seconds, divisor)?;
            Duration::new(0, seconds, dur.datatype)
        }
        _ => {
            let months_dec = Decimal::from_parts(i128::from(dur.months), 0);
            let months_quotient = decimal_div_raw(&months_dec, divisor)?;
            let months = round_decimal_to_i64(dur.datatype, &months_quotient)?;
            let seconds = decimal_div_raw(&dur.seconds, divisor)?;
            Duration::new(months, seconds, dur.datatype)
        }
    }
}

/// Divide one `xsd:duration` by another, by VALUE commensurability rather than by
/// declared tag — the distinction [`divide_year_month_durations`] and
/// [`divide_day_time_durations`] do not need to make, since they each gate on a
/// single named subtype. Two durations are commensurable, and their ratio a plain
/// `xs:decimal`, iff they occupy the same summand of `D = Y ⊕ T` (`Shape`):
/// both `Shape::Months`-shaped, or both `Shape::Seconds`-shaped. A
/// `Shape::Zero` dividend is compatible with either summand — it is the point
/// where the two meet — so `0 ÷ nonzero` always succeeds with a ratio of `0`.
/// A `Shape::Zero` divisor is `Err(DivisionByZero)`, checked first so it takes
/// priority over a `Shape::Zero` dividend. An incommensurable pair (e.g. a
/// `Shape::Months` dividend against a `Shape::Seconds` divisor, or either operand
/// `Shape::Mixed`) reports `XsdError::Indeterminate` with the reason
/// `"incommensurable duration operands"` — the operand *types* are both
/// `xsd:duration`, so a type error would misclassify this; the under-
/// determination arises from the *values*, which is exactly what
/// `XsdError::Indeterminate` exists to name.
///
/// # Examples
///
/// ```
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::temporal::{divide_durations, parse_duration};
///
/// // Cross-tag but value-commensurable: both are purely seconds-shaped.
/// let thirty_days = parse_duration(XsdDatatype::Duration, "P30D").unwrap();
/// let one_day = parse_duration(XsdDatatype::DayTimeDuration, "P1D").unwrap();
/// assert_eq!(
///     divide_durations(&thirty_days, &one_day).unwrap().canonical_lexical(),
///     "30"
/// );
///
/// // Same tag but value-incommensurable: months vs. seconds.
/// let one_year = parse_duration(XsdDatatype::Duration, "P1Y").unwrap();
/// assert!(divide_durations(&one_year, &one_day).is_err());
/// ```
pub fn divide_durations(a: &Duration, b: &Duration) -> Result<Decimal, XsdError> {
    match (a.shape(), b.shape()) {
        (_, Shape::Zero) => Err(XsdError::DivisionByZero {
            datatype: a.datatype,
        }),
        (Shape::Zero, _) => Ok(Decimal::from_parts(0, 0)),
        (Shape::Months, Shape::Months) => decimal_div_raw(
            &Decimal::from_parts(i128::from(a.months), 0),
            &Decimal::from_parts(i128::from(b.months), 0),
        ),
        (Shape::Seconds, Shape::Seconds) => decimal_div_raw(&a.seconds, &b.seconds),
        (Shape::Months | Shape::Seconds | Shape::Mixed, _) => Err(XsdError::Indeterminate {
            reason: "incommensurable duration operands",
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

    /// `"P9223372036854775807Y"` (`i64::MAX` years) must overflow `checked_mul(12)`
    /// on the way in, and negating `i64::MAX` months (via a leading `-`) must
    /// overflow `checked_neg` — both are typed `OutOfRange`, never a silently
    /// wrapped value.
    #[test]
    fn duration_month_overflow_is_out_of_range() {
        assert!(matches!(
            parse_duration(XsdDatatype::Duration, "P9223372036854775807Y"),
            Err(XsdError::OutOfRange { .. })
        ));
        assert!(matches!(
            parse_duration(XsdDatatype::Duration, "-P9223372036854775807Y"),
            Err(XsdError::OutOfRange { .. })
        ));
    }

    /// XSD 1.1 Part 2's pattern facets on the two named duration subtypes:
    /// `yearMonthDuration` is `[^DT]*` (§3.4.27, no day component, no time part at
    /// all) and `dayTimeDuration` is `[^YM]*(T.*)?` (§3.4.26, no year/month
    /// component in the date part — an `M` inside the time part is minutes and is
    /// unrestricted). `xsd:duration` itself carries no pattern facet.
    #[test]
    fn duration_subtype_pattern_facets_are_enforced() {
        assert!(matches!(
            parse_duration(XsdDatatype::YearMonthDuration, "P1D"),
            Err(XsdError::InvalidLexical { .. })
        ));
        assert!(matches!(
            parse_duration(XsdDatatype::DayTimeDuration, "P1Y"),
            Err(XsdError::InvalidLexical { .. })
        ));
        // 'M' inside the TIME part is minutes, not months — unrestricted for
        // dayTimeDuration. Pin the parsed components, not just success.
        let minutes = parse_duration(XsdDatatype::DayTimeDuration, "PT1M").unwrap();
        assert_eq!(minutes.months(), 0);
        assert_eq!(minutes.seconds().canonical_lexical(), "60");
        // xsd:duration carries no pattern facet and accepts every component.
        let general = parse_duration(XsdDatatype::Duration, "P1Y1D").unwrap();
        assert_eq!(general.months(), 12);
        assert_eq!(general.seconds().canonical_lexical(), "86400");
    }

    /// A zero `yearMonthDuration` must canonicalize to `"P0M"` (the only lexical
    /// form its `[^DT]*` pattern facet permits), while a zero `dayTimeDuration`
    /// keeps `"PT0S"`; both must round-trip through `parse_duration`.
    #[test]
    fn duration_zero_lexical_is_subtype_correct() {
        let zero_ym = subtract_durations(
            &ymd(XsdDatatype::YearMonthDuration, "P1Y"),
            &ymd(XsdDatatype::YearMonthDuration, "P1Y"),
        )
        .unwrap();
        assert_eq!(zero_ym.canonical_lexical(), "P0M");
        assert_eq!(
            parse_duration(XsdDatatype::YearMonthDuration, &zero_ym.canonical_lexical())
                .unwrap()
                .canonical_lexical(),
            "P0M"
        );

        let zero_dt = subtract_durations(
            &ymd(XsdDatatype::DayTimeDuration, "PT1H"),
            &ymd(XsdDatatype::DayTimeDuration, "PT1H"),
        )
        .unwrap();
        assert_eq!(zero_dt.canonical_lexical(), "PT0S");
        assert_eq!(
            parse_duration(XsdDatatype::DayTimeDuration, &zero_dt.canonical_lexical())
                .unwrap()
                .canonical_lexical(),
            "PT0S"
        );
    }

    /// A `Duration` with strictly opposite-signed `(months, seconds)` components is
    /// outside the range of the lexical mapping (XSD 1.1 Part 2 §3.3.6) and must be
    /// unconstructible regardless of which arithmetic entry point would compute it.
    /// `Duration::new` is the single point every entry point funnels through, so it
    /// is exercised directly with both sign shapes; a companion assertion pins that
    /// `add_durations`/`subtract_durations` — now that the general `xsd:duration`
    /// tag is accepted and the old subtype gate is gone — reach that same
    /// `Duration::new` refusal, as `OutOfRange`, through BOTH public doors, rather
    /// than silently constructing a mixed-sign value.
    #[test]
    fn mixed_sign_duration_is_unconstructible() {
        // The guard this test protects is `Duration::new`'s sign coherence check —
        // the single point of construction every arithmetic entry point (addition
        // AND subtraction) funnels through. Exercise both sign directions
        // directly: (months: 12, seconds: -86400) is the shape `P1Y - P1D` would
        // reach; (months: -12, seconds: 86400) is the shape `-P1Y + P1D` would
        // reach. A guard placed in only one call site would leave one of these
        // constructible — pinning both proves it is not.
        assert!(matches!(
            Duration::new(12, Decimal::from_parts(-86_400, 0), XsdDatatype::Duration),
            Err(XsdError::OutOfRange { .. })
        ));
        assert!(matches!(
            Duration::new(-12, Decimal::from_parts(86_400, 0), XsdDatatype::Duration),
            Err(XsdError::OutOfRange { .. })
        ));

        // Both-public-doors form: the yearMonthDuration/dayTimeDuration subtype
        // gate that used to intercept a cross-subtype pair earlier, as a
        // TypeMismatch, is gone — the general `xsd:duration` tag is accepted, and
        // `Duration::new`'s sign guard is the one and only place left that can
        // still refuse the resulting mixed-sign pair.
        //
        // `add_durations`: a positive-months operand plus a negative-seconds
        // operand reaches (months: 12, seconds: -86400) directly.
        let ym = ymd(XsdDatatype::YearMonthDuration, "P1Y");
        let neg_day = ymd(XsdDatatype::DayTimeDuration, "-P1D");
        assert!(matches!(
            add_durations(&ym, &neg_day),
            Err(XsdError::OutOfRange { .. })
        ));
        // `subtract_durations`: two positive general-`xsd:duration` operands whose
        // difference is mixed-sign reaches the same guard via negate-then-add.
        let pos_year = ymd(XsdDatatype::Duration, "P1Y");
        let pos_day = ymd(XsdDatatype::Duration, "P1D");
        assert!(matches!(
            subtract_durations(&pos_year, &pos_day),
            Err(XsdError::OutOfRange { .. })
        ));
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

    // Brace kept on its own line (via `rustfmt::skip`) purely so this signature's
    // return type doesn't textually collide with `Duration::new`'s struct-literal
    // audit pattern (`rg 'Duration \{'`, used elsewhere to confirm every
    // construction site routes through the smart constructor) — this helper
    // already routes through `parse_duration`, not a struct literal.
    #[rustfmt::skip]
    fn ymd(dt: XsdDatatype, s: &str) -> Duration
    {
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

    // ── Calendar-action classifier ─────────────────────────────────────────────────

    #[test]
    fn calendar_action_driver_matches_the_classifier_table_for_all_eight_sorts() {
        assert_eq!(
            actions(XsdDatatype::DateTime),
            (MonthAction::Clamped, SecondAction::Free)
        );
        assert_eq!(
            actions(XsdDatatype::Date),
            (MonthAction::Clamped, SecondAction::MidnightTruncating)
        );
        assert_eq!(
            actions(XsdDatatype::Time),
            (MonthAction::Absent, SecondAction::CyclicDay)
        );
        assert_eq!(
            actions(XsdDatatype::GYearMonth),
            (MonthAction::Free, SecondAction::Absent)
        );
        assert_eq!(
            actions(XsdDatatype::GYear),
            (MonthAction::Divisible12, SecondAction::Absent)
        );
        assert_eq!(
            actions(XsdDatatype::GMonth),
            (MonthAction::Cyclic12, SecondAction::Absent)
        );
        assert_eq!(
            actions(XsdDatatype::GMonthDay),
            (
                MonthAction::ClampIfYearIndependent,
                SecondAction::YearIndependentDays
            )
        );
        assert_eq!(
            actions(XsdDatatype::GDay),
            (
                MonthAction::IdentityIfSafe,
                SecondAction::MonthIndependentDays
            )
        );
        // Every non-temporal datatype genuinely has no months field and no
        // seconds field, so it classifies the same way `time` does for months:
        // `Absent`/`Absent`.
        assert_eq!(
            actions(XsdDatatype::Integer),
            (MonthAction::Absent, SecondAction::Absent)
        );
        assert_eq!(
            actions(XsdDatatype::String),
            (MonthAction::Absent, SecondAction::Absent)
        );
        assert_eq!(
            actions(XsdDatatype::Duration),
            (MonthAction::Absent, SecondAction::Absent)
        );
    }

    /// THE load-bearing test for the two-component driver: XML Schema Appendix E
    /// requires months to be applied before seconds. Applying them in the other
    /// order produces a *different, wrong* result for this exact input, so this
    /// test pins both the correct answer and the incorrect alternative a
    /// days-first implementation would produce.
    #[test]
    fn component_order_is_months_before_seconds() {
        let dt = parse_datetime("2024-01-30T00:00:00").unwrap();
        let dur = ymd(XsdDatatype::Duration, "P1M1D");
        let result = add_duration_to_datetime(&dt, &dur).unwrap();
        // Months-first: Jan 30 clamps to Feb 29 (2024 is a leap year), then +1D
        // lands on Mar 1.
        assert_eq!(result.canonical_lexical(), "2024-03-01T00:00:00");
        // Days-first (wrong): Jan 30 + 1D = Jan 31, then +1M clamps to Feb 29.
        assert_ne!(result.canonical_lexical(), "2024-02-29T00:00:00");
    }

    /// A general `xsd:duration` with both components nonzero is accepted by the
    /// new primitive — impossible through either single-component function, both
    /// of which reject it (`duration_to_calendar_arithmetic_rejects_wrong_subtype`).
    #[test]
    fn general_duration_applies_to_instants() {
        let dt = parse_datetime("2024-01-01T00:00:00").unwrap();
        let dur = ymd(XsdDatatype::Duration, "P1Y1D");
        let result = add_duration_to_datetime(&dt, &dur).unwrap();
        assert_eq!(result.canonical_lexical(), "2025-01-02T00:00:00");
    }

    /// `date`'s `MidnightTruncating` second action: the duration is applied at
    /// midnight and only the resulting date is kept, so a sub-day remainder never
    /// errors — matching the shipped `add_day_time_duration_to_date`.
    #[test]
    fn date_plus_subday_duration_truncates_at_midnight() {
        let d = parse_date("2024-01-31").unwrap();
        let sub_day = ymd(XsdDatatype::Duration, "PT1H");
        let unchanged = add_duration_to_date(&d, &sub_day).unwrap();
        assert_eq!(unchanged.canonical_lexical(), "2024-01-31");

        let whole_day = ymd(XsdDatatype::Duration, "P1D");
        let rolled = add_duration_to_date(&d, &whole_day).unwrap();
        assert_eq!(rolled.canonical_lexical(), "2024-02-01");
    }

    /// `time` has no months field (`MonthAction::Absent`): a duration with a
    /// nonzero months component is a type error, while a pure-seconds duration
    /// still works.
    #[test]
    fn time_plus_months_is_a_type_error() {
        let t = parse_time("12:00:00").unwrap();
        let months = ymd(XsdDatatype::Duration, "P1M");
        assert!(add_duration_to_time(&t, &months).is_err());

        let hours = ymd(XsdDatatype::Duration, "PT2H");
        assert!(add_duration_to_time(&t, &hours).is_ok());
    }

    /// The relaxed guards on the ten F&O-named single-component functions: a
    /// general `xsd:duration` whose value happens to be pure-months or
    /// pure-seconds now passes, while a genuinely mixed general duration is still
    /// rejected by BOTH single-component forms (extending
    /// `duration_to_calendar_arithmetic_rejects_wrong_subtype`).
    #[test]
    fn relaxed_guards_accept_pure_component_general_durations() {
        let dt = parse_datetime("2024-01-01T00:00:00Z").unwrap();
        let pure_year = ymd(XsdDatatype::Duration, "P1Y");
        assert!(add_year_month_duration_to_datetime(&dt, &pure_year).is_ok());

        let pure_day = ymd(XsdDatatype::Duration, "P1D");
        assert!(add_day_time_duration_to_datetime(&dt, &pure_day).is_ok());
        let d = parse_date("2024-01-01").unwrap();
        assert!(add_day_time_duration_to_date(&d, &pure_day).is_ok());
        let t = parse_time("00:00:00").unwrap();
        assert!(add_day_time_duration_to_time(&t, &pure_day).is_ok());

        // A mixed general duration is still rejected by BOTH single-component
        // forms — the guard relaxation is strict, not a wholesale acceptance.
        let general = ymd(XsdDatatype::Duration, "P1Y1D");
        assert!(add_year_month_duration_to_datetime(&dt, &general).is_err());
        assert!(add_day_time_duration_to_datetime(&dt, &general).is_err());
    }

    // ── Gregorian ± duration through the classifier ────────────────────────────────

    /// The literal in-source table for `gregorian_duration_matrix`: one row per
    /// (Gregorian type × {months-accepted, months-rejected, days-accepted,
    /// days-rejected, sub-day, zero}) case that exists for that type — a cell
    /// whose action never accepts a component (e.g. `gYear`'s
    /// `SecondAction::Absent` never accepts a nonzero days shift, and `gMonth`'s
    /// `MonthAction::Cyclic12` never rejects a months shift) has no row, which is
    /// what "the 6-case grid per type collapses naturally" means.
    ///
    /// `(datatype, gregorian_lexical, duration_lexical,
    /// expected_canonical_lexical)` — `None` in the last slot means the row must
    /// be rejected as `XsdError::TypeMismatch`.
    #[test]
    fn gregorian_duration_matrix() {
        let rows: [(XsdDatatype, &str, &str, Option<&str>); 34] = [
            // ── gYearMonth (Free, Absent): 4 rows — Free never rejects ────────
            (XsdDatatype::GYearMonth, "2012-10", "P1Y1M", Some("2013-11")), // RDF4J's own pinned case
            (XsdDatatype::GYearMonth, "2024-01", "P1D", None),              // days-rejected: Absent
            (XsdDatatype::GYearMonth, "2024-01", "PT1H", None),             // sub-day
            (XsdDatatype::GYearMonth, "2024-01", "PT0S", Some("2024-01")),  // zero
            // ── gYear (Divisible12, Absent): 5 rows — no days-accepted ────────
            (XsdDatatype::GYear, "2024", "P1Y", Some("2025")), // months-accepted
            (XsdDatatype::GYear, "2024", "P1M", None),         // months-rejected: not a whole year
            (XsdDatatype::GYear, "2024", "P1D", None),         // days-rejected: Absent
            (XsdDatatype::GYear, "2024", "PT1H", None),        // sub-day
            (XsdDatatype::GYear, "2024", "PT0S", Some("2024")), // zero
            // ── gMonth (Cyclic12, Absent): 4 rows — Cyclic12 never rejects ────
            (XsdDatatype::GMonth, "--12", "P1M", Some("--01")), // months-accepted, wraps
            (XsdDatatype::GMonth, "--05", "P1D", None),         // days-rejected: Absent
            (XsdDatatype::GMonth, "--05", "PT1H", None),        // sub-day
            (XsdDatatype::GMonth, "--05", "PT0S", Some("--05")), // zero
            // ── gMonthDay (ClampIfYearIndependent, YearIndependentDays): 15 rows ──
            (XsdDatatype::GMonthDay, "--01-15", "P1M", Some("--02-15")), // months-accepted
            (XsdDatatype::GMonthDay, "--01-31", "P1M", None), // Feb 31 exists in no year — the clamped result varies by target-year leapness
            (XsdDatatype::GMonthDay, "--03-31", "P1M", Some("--04-30")), // April's length never depends on the anchor year, even though April 31 exists in no year
            (XsdDatatype::GMonthDay, "--08-31", "P2M", Some("--10-31")), // October always has 31 — the fix doesn't over-reject
            (XsdDatatype::GMonthDay, "--01-31", "P3M", Some("--04-30")), // April again, via a multi-month shift
            (XsdDatatype::GMonthDay, "--01-31", "P1D", Some("--02-01")), // January always has 31 days
            (XsdDatatype::GMonthDay, "--02-28", "P1D", None), // Feb 28 -> {Feb 29 | Mar 1}
            (XsdDatatype::GMonthDay, "--01-15", "PT1H", None), // sub-day
            (XsdDatatype::GMonthDay, "--01-15", "PT0S", Some("--01-15")), // zero
            // Every leap year's immediate next year is non-leap (leap years
            // are never consecutive), so a whole-year shift off Feb 29
            // always clamps to Feb 28 — the anchored oracle's own
            // adjudication of this case, which the previous day-existence
            // rule over-rejected.
            (XsdDatatype::GMonthDay, "--02-29", "P1Y", Some("--02-28")),
            // A 4-year carry is usually leap-to-leap, but not across a
            // century mark that is not itself divisible by 400 (2096 -> 2100
            // is the reachable example): both a leap and a non-leap target
            // year are real, so this is genuinely year-dependent.
            (XsdDatatype::GMonthDay, "--02-29", "P48M", None),
            // The unsound two-probe rule's counterexample: both 2000→2001 and
            // 2001→2002 see a non-leap February on the walk, so the old rule
            // accepted this and fabricated --03-01, but anchored at Dec 1,
            // 2003 the walk crosses leap February 2004 and lands on Feb 29 —
            // genuinely year-dependent.
            (XsdDatatype::GMonthDay, "--12-01", "P90D", None),
            // Another year-crossing walk that lands inside a February whose
            // leapness varies by anchor year.
            (XsdDatatype::GMonthDay, "--09-01", "P181D", None),
            // The longest walk this action still considers (364 days,
            // one under the 365-day magnitude guard) starting from a date
            // that crosses a year-dependent February.
            (XsdDatatype::GMonthDay, "--12-01", "P364D", None),
            // A genuine year-crossing walk that IS year-independent: January
            // 1 follows December 31 in every year, leap or not.
            (XsdDatatype::GMonthDay, "--12-31", "P1D", Some("--01-01")),
            // ── gDay (IdentityIfSafe, MonthIndependentDays): 6 rows ───────────
            (XsdDatatype::GDay, "---15", "P1M", Some("---15")), // months-accepted: day <= 28
            (XsdDatatype::GDay, "---31", "P1M", None),          // months-rejected: day >= 29
            (XsdDatatype::GDay, "---15", "P1D", Some("---16")), // days-accepted
            (XsdDatatype::GDay, "---28", "P1D", None),          // days-rejected: 29 not in 1..=28
            (XsdDatatype::GDay, "---15", "PT1H", None),         // sub-day
            (XsdDatatype::GDay, "---15", "PT0S", Some("---15")), // zero
        ];
        assert_eq!(rows.len(), 34);
        for (datatype, g_lex, dur_lex, expected) in rows {
            let g = parse_gregorian(datatype, g_lex).unwrap();
            let dur = ymd(XsdDatatype::Duration, dur_lex);
            let result = add_duration_to_gregorian(&g, &dur);
            match expected {
                Some(canon) => assert_eq!(
                    result.unwrap().canonical_lexical(),
                    canon,
                    "{datatype:?} {g_lex} + {dur_lex}"
                ),
                None => assert!(
                    matches!(result, Err(XsdError::TypeMismatch { .. })),
                    "{datatype:?} {g_lex} + {dur_lex} expected TypeMismatch, got {result:?}"
                ),
            }
        }
    }

    /// The `|whole_days| < 365` guard fires before probe unanimity is even
    /// checked: a 1461-day walk (four years plus a day, the classic "4-year
    /// cycle" magnitude) would agree across every probe year by 4-year-cycle
    /// coincidence yet is genuinely century-dependent (e.g. starting in 2100,
    /// a century non-leap year, it lands a day off) — exactly the unsound
    /// failure mode the magnitude guard exists to prevent without having to
    /// detect it directly.
    #[test]
    fn year_independence_rejects_the_century_leap_case() {
        let g = parse_gregorian(XsdDatatype::GMonthDay, "--01-15").unwrap();
        let dur = ymd(XsdDatatype::Duration, "P1461D");
        assert!(matches!(
            add_duration_to_gregorian(&g, &dur),
            Err(XsdError::TypeMismatch { .. })
        ));
    }

    /// `--02-29 + P1D` IS year-independent: the day after Feb 29 is always
    /// Mar 1, from every leap-year anchor there is. The probe-only-valid-years
    /// rule resolves this correctly without ever needing to clamp `--02-29`
    /// down to a non-leap probe's Feb 28 — see `gregorian_duration_matrix`'s
    /// `--02-28 + P1D` row for the genuinely ambiguous neighbor this is not.
    #[test]
    fn year_independence_accepts_the_day_after_leap_day() {
        let g = parse_gregorian(XsdDatatype::GMonthDay, "--02-29").unwrap();
        let dur = ymd(XsdDatatype::Duration, "P1D");
        let result = add_duration_to_gregorian(&g, &dur).unwrap();
        assert_eq!(result.canonical_lexical(), "--03-01");
    }

    /// The ground-truth oracle for `SecondAction::YearIndependentDays`:
    /// anchor a `(month, day)` source at every year in `ANCHOR_YEARS` where
    /// that date is real, walk `whole_days` civil days from each anchor, and
    /// collect the resulting `(month, day)` pairs. A shift is genuinely
    /// year-independent iff every anchor lands on the same pair.
    ///
    /// For every `(month, day)` valid in some year and every magnitude
    /// `SecondAction::YearIndependentDays` will ever consider
    /// (`|whole_days| <= 364`, both signs), this test asserts BOTH
    /// directions of agreement between the classifier and the oracle:
    ///
    /// - If the classifier **accepts**, the oracle set must be the singleton
    ///   `{result}` — soundness: the classifier never fabricates an answer
    ///   the true calendar disagrees with on some anchor. This is exactly
    ///   the property the reverted two-probe rule violated: it accepted
    ///   `--12-01 + P90D` and returned `--03-01`, while the oracle (anchored
    ///   at Dec 1, 2003) also contains `--02-29`.
    /// - If the classifier **rejects**, the oracle set must contain at least
    ///   two distinct pairs — completeness: the classifier never refuses a
    ///   shift that the calendar actually agrees on everywhere.
    ///
    /// `ANCHOR_YEARS` (1900..=2104) is a deliberately narrowed proleptic
    /// range, not the full `i64` year space: every disagreement this action
    /// can produce is a leap/non-leap difference between adjacent calendar
    /// years, and 1900..=2104 already contains that difference in its
    /// "ordinary" 4-year-cycle form many times over, plus the one kind of
    /// anchor year a wider range could add information about — a
    /// century year that is *not* divisible by 400 and so is non-leap
    /// despite being divisible by 4 (2100, included here).
    #[test]
    fn gmonthday_day_shift_agrees_with_the_anchored_oracle() {
        const ANCHOR_YEARS: std::ops::RangeInclusive<i64> = 1900..=2104;

        for month in 1u8..=12 {
            let max_day = days_in_month(LEAP_PROBE_YEAR, month);
            for day in 1u8..=max_day {
                // Every anchor year in range for which (month, day) is a real
                // date, expressed as a civil day count — computed once per
                // (month, day), then reused (incrementally, not re-walked)
                // across all 729 shift magnitudes below.
                let bases: Vec<i64> = ANCHOR_YEARS
                    .clone()
                    .filter(|&y| day <= days_in_month(y, month))
                    .map(|y| days_from_civil(y, month, day))
                    .collect();
                assert!(
                    !bases.is_empty(),
                    "--{month:02}-{day:02} has no anchor year in {ANCHOR_YEARS:?}"
                );

                let g_lex = format!("--{month:02}-{day:02}");
                let g = parse_gregorian(XsdDatatype::GMonthDay, &g_lex).unwrap();

                for whole_days in -364i64..=364 {
                    let dur = Duration::new(
                        0,
                        Decimal::from_parts(i128::from(whole_days) * 86_400, 0),
                        XsdDatatype::DayTimeDuration,
                    )
                    .unwrap();

                    // Oracle: walk every valid anchor; stop scanning as soon
                    // as two distinct results are seen — a rejection only
                    // needs to be shown genuinely ambiguous, not fully
                    // enumerated, and every accept path below still scans
                    // every anchor because it must confirm there is no
                    // disagreement anywhere.
                    let mut oracle_first: Option<(u8, u8)> = None;
                    let mut oracle_ambiguous = false;
                    for &base in &bases {
                        let (_, nm, nd) = civil_from_days(base + whole_days);
                        match oracle_first {
                            None => oracle_first = Some((nm, nd)),
                            Some(first) if first != (nm, nd) => {
                                oracle_ambiguous = true;
                                break;
                            }
                            Some(_) => {}
                        }
                    }

                    match add_duration_to_gregorian(&g, &dur) {
                        Ok(result) => {
                            assert!(
                                !oracle_ambiguous,
                                "{g_lex} + P{whole_days}D: classifier accepted but the oracle disagrees across anchors"
                            );
                            let (em, ed) = oracle_first.expect("bases is never empty");
                            assert_eq!(
                                result.canonical_lexical(),
                                format!("--{em:02}-{ed:02}"),
                                "{g_lex} + P{whole_days}D"
                            );
                        }
                        Err(XsdError::TypeMismatch { .. }) => {
                            assert!(
                                oracle_ambiguous,
                                "{g_lex} + P{whole_days}D: classifier rejected but every anchor agrees on {oracle_first:?}"
                            );
                        }
                        Err(other) => {
                            panic!("{g_lex} + P{whole_days}D: unexpected error {other:?}")
                        }
                    }
                }
            }
        }
    }

    /// The ground-truth oracle for `MonthAction::ClampIfYearIndependent`'s
    /// gMonthDay months-shift: anchor a `(month, day)` source at every year in
    /// `ANCHOR_YEARS` where that date is real, shift `months` (carrying years
    /// as needed), clamp the day down to the target month's actual length
    /// (XML Schema Appendix E), and collect the resulting `(month, day)`
    /// pairs. A shift is genuinely year-independent iff every anchor lands
    /// on the same clamped pair — exactly the same soundness/completeness
    /// contract as [`gmonthday_day_shift_agrees_with_the_anchored_oracle`],
    /// carried over to the month arm: an **accepted** shift's oracle set must
    /// be the singleton the classifier returned, and a **rejected** shift's
    /// oracle set must contain at least two distinct pairs.
    ///
    /// This is the property the previous day-existence rule violated in the
    /// opposite direction from the day arm's bug: instead of fabricating a
    /// wrong answer, it *over-rejected* `--03-31 + P1M`, refusing a shift
    /// whose clamped result (`--04-30`) is identical from every anchor
    /// because April's length never depends on the anchor year.
    ///
    /// `months` sweeps densely over `-48..=48` (covering every within-one-leap-cycle
    /// carry) plus a sparse set of larger magnitudes chosen to exercise
    /// deeper carries cheaply: an ordinary multi-decade carry (`1200` months
    /// = 100 years), a carry landing exactly on the century-mark alignment
    /// that makes a Feb-29 shift ambiguous (`2400` months = 200 years, a
    /// multiple of 4 but not of 400 years), and a carry landing exactly on
    /// one full 400-year Gregorian cycle (`4800` months), where the leap
    /// pattern repeats exactly and a Feb-29 shift is unambiguous again.
    #[test]
    fn gmonthday_month_shift_agrees_with_the_anchored_oracle() {
        const ANCHOR_YEARS: std::ops::RangeInclusive<i64> = 1900..=2104;
        let dense = (-48i64..=48).collect::<Vec<_>>();
        let sparse: Vec<i64> = vec![96, -96, 1200, -1200, 2400, -2400, 4800, -4800];

        for month in 1u8..=12 {
            let max_day = days_in_month(LEAP_PROBE_YEAR, month);
            for day in 1u8..=max_day {
                let anchors: Vec<i64> = ANCHOR_YEARS
                    .clone()
                    .filter(|&y| day <= days_in_month(y, month))
                    .collect();
                assert!(
                    !anchors.is_empty(),
                    "--{month:02}-{day:02} has no anchor year in {ANCHOR_YEARS:?}"
                );

                let g_lex = format!("--{month:02}-{day:02}");
                let g = parse_gregorian(XsdDatatype::GMonthDay, &g_lex).unwrap();

                for &months in dense.iter().chain(sparse.iter()) {
                    if months == 0 {
                        continue; // identity — no months arm is exercised at all
                    }
                    let dur = Duration::new(
                        months,
                        Decimal::from_parts(0, 0),
                        XsdDatatype::YearMonthDuration,
                    )
                    .unwrap();

                    // Oracle: shift-then-clamp from every valid anchor; stop
                    // scanning as soon as two distinct results are seen.
                    let mut oracle_first: Option<(u8, u8)> = None;
                    let mut oracle_ambiguous = false;
                    for &anchor_year in &anchors {
                        let total = i64::from(month) - 1 + months;
                        let target_year = anchor_year + total.div_euclid(12);
                        let target_month = (total.rem_euclid(12) + 1) as u8;
                        let clamped_day = day.min(days_in_month(target_year, target_month));
                        let candidate = (target_month, clamped_day);
                        match oracle_first {
                            None => oracle_first = Some(candidate),
                            Some(first) if first != candidate => {
                                oracle_ambiguous = true;
                                break;
                            }
                            Some(_) => {}
                        }
                    }

                    match add_duration_to_gregorian(&g, &dur) {
                        Ok(result) => {
                            assert!(
                                !oracle_ambiguous,
                                "{g_lex} + P{months}M: classifier accepted but the oracle disagrees across anchors"
                            );
                            let (em, ed) = oracle_first.expect("anchors is never empty");
                            assert_eq!(
                                result.canonical_lexical(),
                                format!("--{em:02}-{ed:02}"),
                                "{g_lex} + P{months}M"
                            );
                        }
                        Err(XsdError::TypeMismatch { .. }) => {
                            assert!(
                                oracle_ambiguous,
                                "{g_lex} + P{months}M: classifier rejected but every anchor agrees on {oracle_first:?}"
                            );
                        }
                        Err(other) => {
                            panic!("{g_lex} + P{months}M: unexpected error {other:?}")
                        }
                    }
                }
            }
        }
    }

    /// `gMonth`'s `Cyclic12` action is the quotient map `ℤ → ℤ/12`: it wraps
    /// correctly at the year boundary, and — because the reduce-first bound
    /// (`month - 1 + months.rem_euclid(12) <= 22`, proven in `drive_gregorian`'s
    /// doc) holds for every `i64`, not just small ones — a huge months value
    /// neither panics nor errors.
    #[test]
    fn gmonth_cycle_wraps_without_overflow() {
        let g = parse_gregorian(XsdDatatype::GMonth, "--12").unwrap();
        let dur = ymd(XsdDatatype::Duration, "P1M");
        let result = add_duration_to_gregorian(&g, &dur).unwrap();
        assert_eq!(result.canonical_lexical(), "--01");

        let huge = Duration::new(
            i64::MAX,
            Decimal::from_parts(0, 0),
            XsdDatatype::YearMonthDuration,
        )
        .unwrap();
        let g2 = parse_gregorian(XsdDatatype::GMonth, "--01").unwrap();
        assert!(add_duration_to_gregorian(&g2, &huge).is_ok());
    }

    /// `subtract_duration_from_gregorian` is negate-then-add, exercised
    /// independently of the addition matrix above.
    #[test]
    fn gregorian_subtraction_is_negate_then_add() {
        let g = parse_gregorian(XsdDatatype::GYear, "2025").unwrap();
        let dur = ymd(XsdDatatype::Duration, "P1Y");
        let result = subtract_duration_from_gregorian(&g, &dur).unwrap();
        assert_eq!(result.canonical_lexical(), "2024");
    }

    /// A zero duration is identity for every Gregorian type, including a
    /// timezone-carrying value — Appendix E step E[3] carries the timezone
    /// through unchanged.
    #[test]
    fn gregorian_zero_duration_preserves_timezone() {
        let g = parse_gregorian(XsdDatatype::GYear, "2024Z").unwrap();
        let dur = ymd(XsdDatatype::Duration, "PT0S");
        let result = add_duration_to_gregorian(&g, &dur).unwrap();
        assert_eq!(result.canonical_lexical(), "2024Z");
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

    /// `op:subtract-dates`'s timezone-indeterminacy rule (a hard `Indeterminate`
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

    /// Inverted from (and renamed from) a test that once pinned the opposite
    /// behavior: F&O restricts `+`/`-`/`*`//` to the two named subtypes, but
    /// PurRDF's RDF4J-permissiveness ruling accepts the general `xsd:duration`
    /// too, in both the mixed-subtype and the general-plus-general case. Both
    /// assertions pin the exact result value AND its exact result datatype — a
    /// bare `is_ok()` would not show the tag-join rule landed correctly.
    #[test]
    fn duration_add_mixes_subtypes_into_general_duration() {
        // yearMonthDuration + dayTimeDuration: neither operand declares the same
        // tag as the other, so the result is the general `xsd:duration`.
        let ym = ymd(XsdDatatype::YearMonthDuration, "P1Y");
        let dt = ymd(XsdDatatype::DayTimeDuration, "PT1H");
        let sum = add_durations(&ym, &dt).unwrap();
        assert_eq!(sum.canonical_lexical(), "P1YT1H");
        assert_eq!(sum.datatype(), XsdDatatype::Duration);

        // general + general: both operands already declare the general tag, and
        // an already-general tag stays general.
        let general_a = ymd(XsdDatatype::Duration, "P1Y1D");
        let general_b = ymd(XsdDatatype::Duration, "P1D");
        let sum = add_durations(&general_a, &general_b).unwrap();
        assert_eq!(sum.canonical_lexical(), "P1Y2D");
        assert_eq!(sum.datatype(), XsdDatatype::Duration);
    }

    /// Dual discriminator A: `P1Y - P1Y` computes the pair `(0, 0)`, which is indistinguishable
    /// from a pure `dayTimeDuration` zero by VALUE alone — only the declared-tag
    /// join rule (never a components-based one) keeps the result tagged
    /// `yearMonthDuration`, which is why its canonical lexical is `"P0M"` and not
    /// `"PT0S"`.
    #[test]
    fn zero_result_keeps_the_operand_subtype() {
        let a = ymd(XsdDatatype::YearMonthDuration, "P1Y");
        let b = ymd(XsdDatatype::YearMonthDuration, "P1Y");
        let diff = subtract_durations(&a, &b).unwrap();
        assert_eq!(diff.canonical_lexical(), "P0M");
        assert_eq!(diff.datatype(), XsdDatatype::YearMonthDuration);
    }

    /// Dual discriminator B: `P1M + PT0S` computes the pair `(1, 0)`, which looks
    /// exactly like a pure `yearMonthDuration` value by VALUE alone — only the
    /// declared-tag join rule (the operands' tags differ: `yearMonthDuration` vs.
    /// `dayTimeDuration`) keeps the result tagged as the general `xsd:duration`
    /// rather than `yearMonthDuration`. A+B together force the tag-join rule to be
    /// syntactic on tags, never a function of the computed components.
    #[test]
    fn mixed_subtype_sum_is_the_general_duration() {
        let a = ymd(XsdDatatype::YearMonthDuration, "P1M");
        let b = ymd(XsdDatatype::DayTimeDuration, "PT0S");
        let sum = add_durations(&a, &b).unwrap();
        assert_eq!(sum.canonical_lexical(), "P1M");
        assert_eq!(sum.datatype(), XsdDatatype::Duration);
    }

    /// The group inverse: negating both components together, and the round trip
    /// back through a second negation.
    #[test]
    fn unary_minus_negates_both_components() {
        let d = ymd(XsdDatatype::YearMonthDuration, "P1Y2M");
        let negated = negate_duration(&d).unwrap();
        assert_eq!(negated.canonical_lexical(), "-P1Y2M");
        assert_eq!(negated.datatype(), XsdDatatype::YearMonthDuration);

        let round_tripped = negate_duration(&negated).unwrap();
        assert_eq!(round_tripped.canonical_lexical(), d.canonical_lexical());
        assert_eq!(round_tripped.datatype(), d.datatype());
    }

    /// `multiply_duration`/`divide_duration` round the months component (ties
    /// toward positive infinity, `round_decimal_to_i64`) rather than truncating —
    /// and the documented consequence that rounding makes `(d ÷ 2) × 2` NOT an
    /// inverse for an odd month count.
    #[test]
    fn duration_scale_rounds_months_not_truncates() {
        let one_month = ymd(XsdDatatype::YearMonthDuration, "P1M");
        let half = parse_decimal("0.5").unwrap();
        // 1 month * 0.5 = 0.5, rounds to 1 (ties toward +infinity) => "P1M".
        assert_eq!(
            multiply_duration(&one_month, &half)
                .unwrap()
                .canonical_lexical(),
            "P1M"
        );
        let one_half = parse_decimal("1.5").unwrap();
        // 1 month * 1.5 = 1.5, rounds to 2 (ties toward +infinity) => "P2M".
        assert_eq!(
            multiply_duration(&one_month, &one_half)
                .unwrap()
                .canonical_lexical(),
            "P2M"
        );

        // Documented non-inverse: (P1M ÷ 2) × 2 rounds 1 month / 2 = 0.5 up to 1
        // month at the division step, so multiplying that back by 2 gives P2M, not
        // the original P1M.
        let two = Decimal::from_parts(2, 0);
        let halved = divide_duration(&one_month, &two).unwrap();
        let doubled = multiply_duration(&halved, &two).unwrap();
        assert_eq!(doubled.canonical_lexical(), "P2M");
        assert_ne!(doubled.canonical_lexical(), one_month.canonical_lexical());
    }

    /// `divide_durations` dispatches on VALUE commensurability, not on declared
    /// tags — three assertions, because a same-tag-only test cannot distinguish
    /// value-based dispatch from a wrong tag-gated implementation that happens to
    /// pass it verbatim.
    #[test]
    fn divide_durations_is_commensurability_not_tags() {
        // Same tag, commensurable (both purely months-shaped).
        let one_year = ymd(XsdDatatype::Duration, "P1Y");
        let one_month = ymd(XsdDatatype::Duration, "P1M");
        assert_eq!(
            divide_durations(&one_year, &one_month)
                .unwrap()
                .canonical_lexical(),
            "12"
        );

        // Cross tag, value-commensurable (both purely seconds-shaped) — must
        // succeed despite the differing declared tags.
        let thirty_days = ymd(XsdDatatype::Duration, "P30D");
        let one_day = ymd(XsdDatatype::DayTimeDuration, "P1D");
        assert_eq!(
            divide_durations(&thirty_days, &one_day)
                .unwrap()
                .canonical_lexical(),
            "30"
        );

        // Same tag, value-incommensurable (months vs. seconds) — must fail even
        // though the declared tags agree.
        let one_year = ymd(XsdDatatype::Duration, "P1Y");
        let one_day = ymd(XsdDatatype::Duration, "P1D");
        assert!(matches!(
            divide_durations(&one_year, &one_day),
            Err(XsdError::Indeterminate {
                reason: "incommensurable duration operands"
            })
        ));
    }

    /// The reachable overflow boundary `decimal_div_raw` guards with a typed
    /// `Err` rather than a wrapping/truncating `i128` multiply: scaling
    /// `86400` (one day, in seconds) up by `10^36` to align a divisor with 18
    /// fractional digits overflows `i128` (`86400 × 10^36 ≈ 8.64 × 10^40 >
    /// i128::MAX ≈ 1.7 × 10^38`). Documented here so the boundary is pinned,
    /// not discovered — it is a different case from the dead `shift_exp < 0`
    /// arm removed alongside this test, which was unreachable, not merely
    /// untested.
    #[test]
    fn decimal_div_scale_overflow_is_out_of_range() {
        let one_day = ymd(XsdDatatype::DayTimeDuration, "P1D");
        let attosecond = ymd(XsdDatatype::DayTimeDuration, "PT0.000000000000000001S");
        assert!(matches!(
            divide_durations(&one_day, &attosecond),
            Err(XsdError::OutOfRange {
                datatype: XsdDatatype::Decimal,
                ..
            })
        ));
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
