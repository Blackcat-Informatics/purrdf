// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Instant order with explicitly declared civil precision.

use super::{
    Date, DateTime, TZ_BOUND_SECS, days_from_civil, days_from_civil_wide, days_in_month, is_leap,
};
use crate::{XsdDatatype, XsdValue};

const DAY: i128 = 86_400;

/// The field that ends the precision of a civil timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivilPrecision {
    /// The timestamp denotes an instant somewhere on its civil day.
    Day,
    /// The timestamp denotes an instant somewhere in its civil month.
    Month,
    /// The timestamp denotes an instant somewhere in its civil year.
    Year,
}

/// A proven answer to a strict ordering question about two instants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstantOrder {
    /// All represented pairs satisfy the order.
    Yes,
    /// No represented pair satisfies the order.
    No,
    /// The represented pairs do not all give the same answer.
    Undetermined,
}

/// A civil period that cannot describe the requested absolute precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrecisionError {
    /// The value has no anchored day, month, or year.
    Unanchored,
    /// Only one endpoint of a named period specifies a timezone.
    MixedTimezone,
    /// The exclusive end does not follow the start.
    NonPositivePeriod,
}

impl core::fmt::Display for PrecisionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Unanchored => "precision requires an anchored civil day, month, or year",
            Self::MixedTimezone => "both period endpoints must specify a timezone or both omit it",
            Self::NonPositivePeriod => "the exclusive period end must follow its start",
        })
    }
}

impl core::error::Error for PrecisionError {}

impl purrdf_hash::error::StructuredError for PrecisionError {
    purrdf_hash::error_visit! {
        Unanchored => "temporal-precision-unanchored" {},
        MixedTimezone => "temporal-precision-mixed-timezone" {},
        NonPositivePeriod => "temporal-precision-nonpositive-period" {},
    }
}

/// The possible instants of a civil period, on a half-open proleptic
/// Gregorian timeline. The represented instant is not the period's start.
/// A missing offset allows each timezone offset admitted by XSD, without
/// reading or assuming a host timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrecisionInterval {
    start: i128,
    end: i128,
}

impl PrecisionInterval {
    /// Interpret a timestamp with an explicitly declared coarse precision.
    /// Normalize end-of-day midnight before discarding finer fields.
    /// The timezone is retained.
    #[must_use]
    pub const fn from_datetime(value: &DateTime, precision: CivilPrecision) -> Self {
        let (year, month, day) = value.civil_date();
        Self::civil(year, month, day, value.tz, precision)
    }

    /// Interpret an anchored XSD date, year-month, or year as a period.
    ///
    /// # Errors
    ///
    /// Recurring or other datatypes do not supply an anchored civil period.
    pub fn from_calendar(value: &XsdValue) -> Result<Self, PrecisionError> {
        match value {
            XsdValue::Date(date) => Ok(Self::civil(
                i128::from(date.year),
                date.month,
                date.day,
                date.tz,
                CivilPrecision::Day,
            )),
            XsdValue::Gregorian(value) => match (value.datatype, value.year, value.month) {
                (XsdDatatype::GYear, Some(year), None) => Ok(Self::civil(
                    i128::from(year),
                    1,
                    1,
                    value.tz,
                    CivilPrecision::Year,
                )),
                (XsdDatatype::GYearMonth, Some(year), Some(month)) => Ok(Self::civil(
                    i128::from(year),
                    month,
                    1,
                    value.tz,
                    CivilPrecision::Month,
                )),
                _ => Err(PrecisionError::Unanchored),
            },
            _ => Err(PrecisionError::Unanchored),
        }
    }

    /// Interpret caller-defined period dates, such as the bounds of a season.
    /// Both dates name midnight and the end is excluded. Two dates without
    /// timezones share one unspecified offset within the period. No calendar
    /// definition for a named season is inferred.
    ///
    /// # Errors
    ///
    /// Refuses mixed explicit/unspecified timezones and nonpositive periods.
    pub const fn between_dates(start: &Date, end: &Date) -> Result<Self, PrecisionError> {
        let first = DAY * days_from_civil(start.year, start.month, start.day);
        let last = DAY * days_from_civil(end.year, end.month, end.day);
        let (first, last, offset_unknown) = match (start.tz, end.tz) {
            (Some(a), Some(b)) => (first - a as i128 * 60, last - b as i128 * 60, false),
            (None, None) => (first, last, true),
            _ => return Err(PrecisionError::MixedTimezone),
        };
        if first >= last {
            return Err(PrecisionError::NonPositivePeriod);
        }
        let margin = if offset_unknown { TZ_BOUND_SECS } else { 0 };
        Ok(Self {
            start: first - margin,
            end: last + margin,
        })
    }

    const fn civil(
        year: i128,
        month: u8,
        day: u8,
        offset: Option<i32>,
        precision: CivilPrecision,
    ) -> Self {
        // The Gregorian leap rule repeats every 400 years.
        let cycle_year = (year % 400) as i64;
        let (month, day, length) = match precision {
            CivilPrecision::Day => (month, day, 1),
            CivilPrecision::Month => (month, 1, days_in_month(cycle_year, month) as i128),
            CivilPrecision::Year => (1, 1, if is_leap(cycle_year) { 366 } else { 365 }),
        };
        let first = DAY * days_from_civil_wide(year, month, day);
        let last = first + DAY * length;
        match offset {
            Some(minutes) => {
                let offset = minutes as i128 * 60;
                Self {
                    start: first - offset,
                    end: last - offset,
                }
            }
            None => Self {
                start: first - TZ_BOUND_SECS,
                end: last + TZ_BOUND_SECS,
            },
        }
    }

    /// Whether the instant represented by this period strictly precedes the
    /// instant represented by the other. Adjacent half-open periods do not
    /// overlap. Unspecified offsets of separate periods vary independently.
    #[must_use]
    pub const fn before(self, other: Self) -> InstantOrder {
        if self.end <= other.start {
            InstantOrder::Yes
        } else if self.start >= other.end {
            InstantOrder::No
        } else {
            InstantOrder::Undetermined
        }
    }

    /// Whether the represented instant strictly follows the other.
    #[must_use]
    pub const fn after(self, other: Self) -> InstantOrder {
        other.before(self)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{parse_date, parse_datetime};
    use super::*;

    fn period(datatype: XsdDatatype, text: &str) -> PrecisionInterval {
        PrecisionInterval::from_calendar(&crate::parse(text, datatype).unwrap()).unwrap()
    }

    #[test]
    fn coarse_precision_proves_only_orders_shared_by_all_possible_instants() {
        let year = period(XsdDatatype::GYear, "2024Z");
        let leap_day = period(XsdDatatype::Date, "2024-02-29Z");
        let next = period(XsdDatatype::GYearMonth, "2025-01Z");
        assert_eq!(year.before(leap_day), InstantOrder::Undetermined);
        assert_eq!(leap_day.before(year), InstantOrder::Undetermined);
        assert_eq!(year.before(next), InstantOrder::Yes);
        assert_eq!(next.before(year), InstantOrder::No);
        let noon = parse_datetime("2024-02-29T12:25:00Z").unwrap();
        let march = period(XsdDatatype::GYearMonth, "2024-03Z");
        assert_eq!(
            PrecisionInterval::from_datetime(&noon, CivilPrecision::Day),
            leap_day
        );
        assert_eq!(leap_day.before(march), InstantOrder::Yes);
        let unknown_zone = period(XsdDatatype::Date, "2024-02-29");
        assert_eq!(unknown_zone.before(march), InstantOrder::Undetermined);
        let next_day = period(XsdDatatype::Date, "2024-03-02Z");
        assert_eq!(unknown_zone.before(next_day), InstantOrder::Yes);
    }

    #[test]
    fn named_periods_have_caller_bounds_and_refuse_unanchored_inputs() {
        let start = parse_date("2024-09-01+02:00").unwrap();
        let end = parse_date("2024-12-01+02:00").unwrap();
        let fall = PrecisionInterval::between_dates(&start, &end).unwrap();
        let december = period(XsdDatatype::GYearMonth, "2024-12+02:00");
        assert_eq!(december.after(fall), InstantOrder::Yes);
        assert_eq!(
            PrecisionInterval::between_dates(&end, &start),
            Err(PrecisionError::NonPositivePeriod)
        );
        let floating_end = parse_date("2024-12-01").unwrap();
        assert_eq!(
            PrecisionInterval::between_dates(&start, &floating_end),
            Err(PrecisionError::MixedTimezone)
        );
        let recurring = crate::parse("--09-01", XsdDatatype::GMonthDay).unwrap();
        assert_eq!(
            PrecisionInterval::from_calendar(&recurring),
            Err(PrecisionError::Unanchored)
        );
    }

    #[test]
    fn end_of_day_is_the_next_civil_day_for_every_precision() {
        let previous = parse_datetime("2024-12-31T24:00:00Z").unwrap();
        let midnight = parse_datetime("2025-01-01T00:00:00Z").unwrap();
        for precision in [
            CivilPrecision::Day,
            CivilPrecision::Month,
            CivilPrecision::Year,
        ] {
            assert_eq!(
                PrecisionInterval::from_datetime(&previous, precision),
                PrecisionInterval::from_datetime(&midnight, precision)
            );
        }
        let last = parse_datetime("9223372036854775807-12-31T24:00:00Z").unwrap();
        let last_year = period(XsdDatatype::GYear, "9223372036854775807Z");
        assert_eq!(
            PrecisionInterval::from_datetime(&last, CivilPrecision::Year).after(last_year),
            InstantOrder::Yes
        );
    }
}
