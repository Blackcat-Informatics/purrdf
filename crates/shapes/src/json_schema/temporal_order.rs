// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Temporal order over XSD lexical forms, as JSON Schema can state it.
//!
//! A range bound over `xsd:dateTime`, `xsd:date` or `xsd:time` compares a value
//! of the same datatype on the XSD timeline (XSD 1.1 Part 2 §3.3.7–§3.3.9,
//! Appendix D.2): a value with a timezone is the instant it names, one without
//! is its local reading, and two of the same kind compare exactly. A value and
//! a bound of which exactly one has a timezone compare only when they differ by
//! more than the ±14:00 an absent timezone may be, and are incomparable
//! otherwise — which a range component reports as a violation (SHACL 1.2 Core
//! §4.4). Every other value, another datatype's included, compares with
//! nothing.
//!
//! The projection carries such a literal as a typed-literal object whose
//! `@value` is the lexical form, so a `pattern` judges it. The lexical forms of
//! one datatype are a fixed-field format, and every relation above is a
//! regular language over it:
//!
//! * without a timezone the order is lexicographic on the fields — the year by
//!   its signed value, then month, day, hour, minute and seconds — except that
//!   `24:00:00` is the next day's midnight;
//! * with one, the instant is the local reading less the offset, and an offset
//!   is at most 840 minutes: a local minute more than 840 minutes past the
//!   bound's is later under every offset, one more than 840 before it earlier
//!   under every offset, and each of the 1,681 minutes between is later exactly
//!   under the offsets below its distance from the bound — a bound on the
//!   offset field, which the pattern states minute by minute.
//!
//! [`order_pattern`] states the order on well-formed lexical forms;
//! [`lexical_patterns`] states well-formedness as the validator parses it —
//! the fields, the days of each month and the leap years, a year within the
//! signed 64-bit range and at most 18 fractional second digits — around the
//! `whiteSpace` `collapse` trim the three datatypes fix. A value meets a bound
//! exactly when its lexical form matches all of them.

use std::fmt::Write as _;

use super::numeric_order::{Decimal, Facet, Rel, Threshold, WS, frac_part, order_body};

/// The three temporal datatypes a range bound compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    DateTime,
    Date,
    Time,
}

impl Kind {
    /// The kind of an XSD local name.
    pub(super) fn from_local(local: &str) -> Option<Self> {
        match local {
            "dateTime" => Some(Self::DateTime),
            "date" => Some(Self::Date),
            "time" => Some(Self::Time),
            _ => None,
        }
    }

    /// The XSD local name.
    pub(super) const fn local(self) -> &'static str {
        match self {
            Self::DateTime => "dateTime",
            Self::Date => "date",
            Self::Time => "time",
        }
    }

    const fn has_date(self) -> bool {
        matches!(self, Self::DateTime | Self::Date)
    }

    const fn has_time(self) -> bool {
        matches!(self, Self::DateTime | Self::Time)
    }
}

/// Minutes in a day.
const DAY: i128 = 1440;
/// The greatest timezone offset, in minutes.
const ZONE: i128 = 840;

/// A point on the timeline: whole minutes since the proleptic epoch and the
/// seconds within the minute (whole seconds and fraction digits without
/// trailing zeros).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Point {
    minute: i128,
    second: u8,
    fraction: String,
}

impl Point {
    fn shifted(&self, minutes: i128) -> Self {
        Self {
            minute: self.minute + minutes,
            ..self.clone()
        }
    }

    const fn whole_minute(&self) -> bool {
        self.second == 0 && self.fraction.is_empty()
    }
}

/// A temporal range bound: its datatype, local reading and timezone.
#[derive(Clone, Debug)]
pub(super) struct Bound {
    kind: Kind,
    local: Point,
    zone: Option<i128>,
}

impl Bound {
    /// The bound a literal of `kind` states; `None` when the lexical form is
    /// not one of the datatype (the validator then compares it with nothing).
    pub(super) fn parse(kind: Kind, lexical: &str) -> Option<Self> {
        let lexical = lexical.trim_matches(['\t', '\n', '\r', ' ']);
        let iri = format!("http://www.w3.org/2001/XMLSchema#{}", kind.local());
        purrdf_xsd::parse_by_iri(lexical, &iri).ok()??;
        let (body, zone) = split_zone(lexical);
        let (days, time) = match kind {
            Kind::DateTime => {
                let (date, time) = body.split_once('T')?;
                (civil_days(date)?, Some(time))
            }
            Kind::Date => (civil_days(body)?, None),
            Kind::Time => (0, Some(body)),
        };
        let mut local = Point {
            minute: days * DAY,
            second: 0,
            fraction: String::new(),
        };
        if let Some(time) = time {
            let mut fields = time.splitn(3, ':');
            let hour: i128 = fields.next()?.parse().ok()?;
            let minute: i128 = fields.next()?.parse().ok()?;
            let seconds = fields.next()?;
            let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
            local.minute += hour * 60 + minute;
            local.second = whole.parse().ok()?;
            fraction
                .trim_end_matches('0')
                .clone_into(&mut local.fraction);
        }
        Some(Self { kind, local, zone })
    }

    /// The datatype.
    pub(super) const fn kind(&self) -> Kind {
        self.kind
    }
}

/// Split a trailing timezone (`Z` or `±hh:mm`, in minutes) off a lexical form.
fn split_zone(lexical: &str) -> (&str, Option<i128>) {
    if let Some(body) = lexical.strip_suffix('Z') {
        return (body, Some(0));
    }
    let bytes = lexical.as_bytes();
    if bytes.len() >= 6 {
        let tail = &lexical[lexical.len() - 6..];
        let sign = tail.as_bytes()[0];
        if matches!(sign, b'+' | b'-')
            && tail.as_bytes()[3] == b':'
            && let (Ok(hours), Ok(minutes)) =
                (tail[1..3].parse::<i128>(), tail[4..6].parse::<i128>())
        {
            let offset = hours * 60 + minutes;
            let offset = if sign == b'-' { -offset } else { offset };
            return (&lexical[..lexical.len() - 6], Some(offset));
        }
    }
    (lexical, None)
}

/// The day number of a `[-]YYYY-MM-DD` date.
fn civil_days(date: &str) -> Option<i128> {
    let (negative, body) = date
        .strip_prefix('-')
        .map_or((false, date), |body| (true, body));
    let mut fields = body.splitn(3, '-');
    let year: i128 = fields.next()?.parse().ok()?;
    let month: i128 = fields.next()?.parse().ok()?;
    let day: i128 = fields.next()?.parse().ok()?;
    Some(days_from_civil(
        if negative { -year } else { year },
        month,
        day,
    ))
}

/// Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant's
/// `days_from_civil`, over `i128`).
const fn days_from_civil(year: i128, month: i128, day: i128) -> i128 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The proleptic Gregorian date of a day number (H. Hinnant's
/// `civil_from_days`, over `i128`): `(year, month, day)`.
const fn civil_from_days(days: i128) -> (i128, i128, i128) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

/// The patterns a lexical form of `kind` must match to be one the validator
/// parses: the fields and, for a date, the leap-year rule for 29 February.
pub(super) fn lexical_patterns(kind: Kind) -> Vec<String> {
    let year = year_lexical();
    let month_day = "(?:(?:0[13578]|1[02])-(?:0[1-9]|[12][0-9]|3[01])|(?:0[469]|11)-(?:0[1-9]|[12][0-9]|30)|02-(?:0[1-9]|1[0-9]|2[0-9]))";
    let time =
        "(?:(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\\.[0-9]{1,18})?|24:00:00(?:\\.0{1,18})?)";
    let zone = "(?:Z|[+\\-](?:(?:0[0-9]|1[0-3]):[0-5][0-9]|14:00))";
    let body = match kind {
        Kind::DateTime => format!("{year}-{month_day}T{time}"),
        Kind::Date => format!("{year}-{month_day}"),
        Kind::Time => time.to_owned(),
    };
    let mut patterns = vec![format!("^{WS}{body}{zone}?{WS}$")];
    if kind.has_date() {
        // 29 February only in a leap year: the year is divisible by 4 and, a
        // century, by 400 — read on its last four digits (a year has at least
        // four).
        let leap =
            "(?:[0-9]{2}(?:0[48]|[2468][048]|[13579][26])|(?:0[048]|[2468][048]|[13579][26])00)";
        patterns.push(format!(
            "^{WS}-?(?:[0-9]*{leap}-02-29|[0-9]+-(?:0[013-9]|1[0-2])-|[0-9]+-02-(?:[01][0-9]|2[0-8]))"
        ));
    }
    patterns
}

/// A year the validator reads: at least four digits, no leading zero beyond
/// four, and a magnitude within the signed 64-bit range.
fn year_lexical() -> String {
    const MAX: &str = "9223372036854775807";
    let mut widest = Vec::new();
    for (index, digit) in MAX.bytes().enumerate() {
        let low = u8::from(index == 0);
        let high = digit - b'0';
        if high > low {
            widest.push(format!(
                "{}{}[0-9]{{{}}}",
                &MAX[..index],
                digit_class(low, high - 1),
                MAX.len() - index - 1
            ));
        }
    }
    widest.push(MAX.to_owned());
    format!("-?(?:0[0-9]{{3}}|[1-9][0-9]{{3,17}}|{})", widest.join("|"))
}

fn digit_class(low: u8, high: u8) -> String {
    if low == high {
        char::from(b'0' + low).to_string()
    } else {
        format!("[{}-{}]", char::from(b'0' + low), char::from(b'0' + high))
    }
}

/// The two-digit numerals `low..=high` (both within `0..=99`).
fn two_digits(low: i128, high: i128) -> Option<String> {
    if low > high {
        return None;
    }
    let digit = |value: i128| u8::try_from(value).expect("a decimal digit");
    let (low_tens, low_units) = (low / 10, low % 10);
    let (high_tens, high_units) = (high / 10, high % 10);
    if low_tens == high_tens {
        return Some(format!(
            "{low_tens}{}",
            digit_class(digit(low_units), digit(high_units))
        ));
    }
    let mut alternatives = vec![format!("{low_tens}{}", digit_class(digit(low_units), 9))];
    if high_tens > low_tens + 1 {
        alternatives.push(format!(
            "{}[0-9]",
            digit_class(digit(low_tens + 1), digit(high_tens - 1))
        ));
    }
    alternatives.push(format!("{high_tens}{}", digit_class(0, digit(high_units))));
    Some(format!("(?:{})", alternatives.join("|")))
}

/// The `hh:mm` of the minutes of the day `low..=high` (within `0..=1440`,
/// `24:00` being 1440).
fn hour_minutes(low: i128, high: i128) -> Option<String> {
    let (low, high) = (low.max(0), high.min(DAY));
    if low > high {
        return None;
    }
    let (low_hour, low_minute) = (low / 60, low % 60);
    let (high_hour, high_minute) = (high / 60, high % 60);
    let hour = |value: i128| format!("{value:02}");
    if low_hour == high_hour {
        return Some(format!(
            "{}:{}",
            hour(low_hour),
            two_digits(low_minute, high_minute)?
        ));
    }
    let mut alternatives = vec![format!(
        "{}:{}",
        hour(low_hour),
        two_digits(low_minute, 59)?
    )];
    if let Some(hours) = two_digits(low_hour + 1, high_hour - 1) {
        alternatives.push(format!("{hours}:[0-9]{{2}}"));
    }
    alternatives.push(format!(
        "{}:{}",
        hour(high_hour),
        two_digits(0, high_minute)?
    ));
    Some(format!("(?:{})", alternatives.join("|")))
}

/// The timezone lexical forms (`Z`, `±hh:mm`) whose offset is in
/// `low..=high` minutes.
fn zones(low: i128, high: i128) -> Option<String> {
    let (low, high) = (low.max(-ZONE), high.min(ZONE));
    if low > high {
        return None;
    }
    let magnitude = |low: i128, high: i128| -> Option<String> {
        let (low_hour, low_minute) = (low / 60, low % 60);
        let (high_hour, high_minute) = (high / 60, high % 60);
        if low_hour == high_hour {
            return Some(format!(
                "{low_hour:02}:{}",
                two_digits(low_minute, high_minute)?
            ));
        }
        let mut alternatives = vec![format!("{low_hour:02}:{}", two_digits(low_minute, 59)?)];
        if let Some(hours) = two_digits(low_hour + 1, high_hour - 1) {
            alternatives.push(format!("{hours}:[0-5][0-9]"));
        }
        alternatives.push(format!("{high_hour:02}:{}", two_digits(0, high_minute)?));
        Some(format!("(?:{})", alternatives.join("|")))
    };
    let mut alternatives = Vec::new();
    if low <= 0 && high >= 0 {
        alternatives.push("Z|[+\\-]00:00".to_owned());
    }
    if high > 0
        && let Some(positive) = magnitude(low.max(1), high)
    {
        alternatives.push(format!("\\+{positive}"));
    }
    if low < 0
        && let Some(negative) = magnitude((-high).max(1), -low)
    {
        alternatives.push(format!("-{negative}"));
    }
    Some(format!("(?:{})", alternatives.join("|")))
}

/// The year numerals (sign and leading zeros allowed) in relation `rel` to
/// `year`, or equal to it with `None`.
fn years(rel: Option<Rel>, year: i128) -> Option<String> {
    let Some(rel) = rel else {
        let digits = year.unsigned_abs();
        return Some(match year.signum() {
            0 => "-?0+".to_owned(),
            1 => format!("0*{digits}"),
            _ => format!("-0*{digits}"),
        });
    };
    let bound = Decimal::parse(&year.to_string(), true).expect("an integer numeral");
    order_body(&Threshold::Cmp(rel, bound), true).map(|body| format!("(?:{body})"))
}

/// The dates (`YEAR-MM-DD`, day numbers) in relation `rel` to `day`, or equal
/// to it with `None`.
fn dates(rel: Option<Rel>, day: i128) -> Option<String> {
    let (year, month, day) = civil_from_days(day);
    let exact = years(None, year)?;
    let Some(rel) = rel else {
        return Some(format!("{exact}-{month:02}-{day:02}"));
    };
    let (months, days) = match rel {
        Rel::Gt | Rel::Ge => (two_digits(month + 1, 12), two_digits(day + 1, 31)),
        Rel::Lt | Rel::Le => (two_digits(1, month - 1), two_digits(1, day - 1)),
    };
    let mut alternatives = Vec::new();
    if let Some(earlier_or_later) = years(Some(strict(rel)), year) {
        alternatives.push(format!("{earlier_or_later}-[0-9]{{2}}-[0-9]{{2}}"));
    }
    if let Some(months) = months {
        alternatives.push(format!("{exact}-{months}-[0-9]{{2}}"));
    }
    if let Some(days) = days {
        alternatives.push(format!("{exact}-{month:02}-{days}"));
    }
    (!alternatives.is_empty()).then(|| format!("(?:{})", alternatives.join("|")))
}

const fn strict(rel: Rel) -> Rel {
    match rel {
        Rel::Gt | Rel::Ge => Rel::Gt,
        Rel::Lt | Rel::Le => Rel::Lt,
    }
}

/// The lexical prefixes up to the minute (`YEAR-MM-DDThh:mm`, `hh:mm` or
/// `YEAR-MM-DD`) whose minute on the local timeline is in relation `rel` to
/// `minute`, or equal to it with `None`. `24:00` is the next day's first
/// minute.
fn minutes(kind: Kind, rel: Option<Rel>, minute: i128) -> Vec<String> {
    let day = minute.div_euclid(DAY);
    let of_day = minute.rem_euclid(DAY);
    let mut out = Vec::new();
    let mut push = |date: Option<String>, time: Option<String>| {
        if let (Some(date), Some(time)) = (date, time) {
            out.push(format!("{date}T{time}"));
        }
    };
    match (kind, rel) {
        (Kind::DateTime, None) => {
            push(dates(None, day), hour_minutes(of_day, of_day));
            if of_day == 0 {
                push(dates(None, day - 1), Some("24:00".to_owned()));
            }
        }
        (Kind::DateTime, Some(Rel::Gt | Rel::Ge)) => {
            push(dates(Some(Rel::Gt), day), hour_minutes(0, DAY));
            push(dates(None, day), hour_minutes(of_day + 1, DAY));
        }
        (Kind::DateTime, Some(Rel::Lt | Rel::Le)) => {
            push(dates(Some(Rel::Lt), day - 1), hour_minutes(0, DAY));
            let last = if of_day > 0 { DAY } else { DAY - 1 };
            push(dates(None, day - 1), hour_minutes(0, last));
            push(dates(None, day), hour_minutes(0, of_day - 1));
        }
        (Kind::Time, None) => out.extend(hour_minutes(minute, minute)),
        (Kind::Time, Some(Rel::Gt | Rel::Ge)) => out.extend(hour_minutes(minute + 1, DAY)),
        (Kind::Time, Some(Rel::Lt | Rel::Le)) => out.extend(hour_minutes(0, minute - 1)),
        (Kind::Date, None) => {
            if of_day == 0 {
                out.extend(dates(None, day));
            }
        }
        (Kind::Date, Some(Rel::Gt | Rel::Ge)) => out.extend(dates(Some(Rel::Gt), day)),
        (Kind::Date, Some(Rel::Lt | Rel::Le)) => {
            let below = if of_day == 0 { day } else { day + 1 };
            out.extend(dates(Some(Rel::Lt), below));
        }
    }
    out
}

/// The seconds fields (`ss(.s+)?`) in relation `rel` to the point's seconds.
fn seconds(rel: Rel, point: &Point) -> Option<String> {
    if point.whole_minute() {
        // Against zero seconds, loosely (the lexical patterns hold the field
        // to `ss(.s+)?`): a field is above zero exactly when it has a non-zero
        // digit.
        return match rel {
            Rel::Gt => Some("[0-9.]*[1-9][0-9.]*".to_owned()),
            Rel::Ge => Some(ANY_SECONDS.to_owned()),
            Rel::Lt => None,
            Rel::Le => Some("[0.]+".to_owned()),
        };
    }
    let whole = i128::from(point.second);
    let fraction = |rel: Option<Rel>| -> Option<String> {
        let alternatives: Vec<String> = frac_part(rel, &point.fraction)
            .into_iter()
            .map(|digits| {
                if digits.is_empty() {
                    String::new()
                } else {
                    format!("\\.(?:{digits})")
                }
            })
            .collect();
        (!alternatives.is_empty()).then(|| format!("(?:{})", alternatives.join("|")))
    };
    let mut alternatives = Vec::new();
    let (wider, strict_rel) = match rel {
        Rel::Gt | Rel::Ge => (two_digits(whole + 1, 59), Rel::Gt),
        Rel::Lt | Rel::Le => (two_digits(0, whole - 1), Rel::Lt),
    };
    if let Some(wider) = wider {
        alternatives.push(format!("{wider}(?:\\.[0-9]+)?"));
    }
    if let Some(fraction) = fraction(Some(strict_rel)) {
        alternatives.push(format!("{whole:02}{fraction}"));
    }
    if matches!(rel, Rel::Ge | Rel::Le)
        && let Some(fraction) = fraction(None)
    {
        alternatives.push(format!("{whole:02}{fraction}"));
    }
    (!alternatives.is_empty()).then(|| format!("(?:{})", alternatives.join("|")))
}

/// Any seconds field. Loose on its own (the lexical patterns hold the field
/// to `ss(.s+)?`): it is followed by a timezone or the end, neither of which
/// is a digit or a point.
const ANY_SECONDS: &str = "[0-9.]+";

/// Whether a value with no seconds field (a date: midnight, zero seconds)
/// stands in `rel` to the point's seconds when its minute equals the point's.
fn zero_seconds_meet(rel: Rel, point: &Point) -> bool {
    match rel {
        Rel::Gt => false,
        Rel::Ge => point.whole_minute(),
        Rel::Lt => !point.whole_minute(),
        Rel::Le => true,
    }
}

/// The lexical forms (without a timezone when `zone` is `None`, with one
/// otherwise — the regex for those zones) whose key, on the local timeline
/// less the offset, is in `rel` to `point`.
fn relation(kind: Kind, rel: Rel, point: &Point, zone: Option<&str>) -> Vec<String> {
    let tail = zone.map_or_else(String::new, ToOwned::to_owned);
    let mut out = Vec::new();
    let beyond = match rel {
        Rel::Gt | Rel::Ge => Some(Rel::Gt),
        Rel::Lt | Rel::Le => Some(Rel::Lt),
    };
    for prefix in minutes(kind, beyond, point.minute) {
        if kind.has_time() {
            out.push(format!("{prefix}:{ANY_SECONDS}{tail}"));
        } else {
            out.push(format!("{prefix}{tail}"));
        }
    }
    for prefix in minutes(kind, None, point.minute) {
        if kind.has_time() {
            if let Some(seconds) = seconds(rel, point) {
                out.push(format!("{prefix}:{seconds}{tail}"));
            }
        } else if zero_seconds_meet(rel, point) {
            out.push(format!("{prefix}{tail}"));
        }
    }
    out
}

/// The lexical forms with a timezone whose instant is in `rel` to `point`:
/// beyond 840 minutes of it under every offset, and within them minute by
/// minute under the offsets that place the instant on its side.
fn zoned(kind: Kind, rel: Rel, point: &Point) -> Vec<String> {
    let any = zones(-ZONE, ZONE).expect("every offset");
    let mut out = Vec::new();
    let beyond = match rel {
        Rel::Gt | Rel::Ge => minutes(kind, Some(Rel::Gt), point.minute + ZONE),
        Rel::Lt | Rel::Le => minutes(kind, Some(Rel::Lt), point.minute - ZONE),
    };
    for prefix in beyond {
        if kind.has_time() {
            out.push(format!("{prefix}:{ANY_SECONDS}{any}"));
        } else {
            out.push(format!("{prefix}{any}"));
        }
    }
    if !kind.has_time() {
        // A date's local minute is its midnight: at most two in the window.
        for local in point.minute - ZONE..=point.minute + ZONE {
            let prefixes = minutes(kind, None, local);
            if prefixes.is_empty() {
                continue;
            }
            let distance = local - point.minute;
            let mut offsets = Vec::new();
            offsets.extend(match rel {
                Rel::Gt | Rel::Ge => zones(-ZONE, distance - 1),
                Rel::Lt | Rel::Le => zones(distance + 1, ZONE),
            });
            if zero_seconds_meet(rel, point) {
                offsets.extend(zones(distance, distance));
            }
            if offsets.is_empty() {
                continue;
            }
            for prefix in prefixes {
                out.push(format!("{prefix}(?:{})", offsets.join("|")));
            }
        }
        return out;
    }
    // Within the window, group the local minutes by their hour: each shares
    // the offsets that clear every minute of the hour, and each minute adds
    // the offsets up to its own distance from the point (and the seconds
    // decide at exactly that distance).
    let mut hours: Vec<(String, i128, i128, i128)> = Vec::new();
    let mut add = |prefix: String, start: i128, minute: i128| match hours.last_mut() {
        Some((last, last_start, _, high)) if *last == prefix && *last_start == start => {
            *high = minute;
        }
        _ => hours.push((prefix, start, minute, minute)),
    };
    for local in point.minute - ZONE..=point.minute + ZONE {
        let of_day = local.rem_euclid(DAY);
        let (hour, minute) = (of_day / 60, of_day % 60);
        let start = local - minute;
        match kind {
            Kind::DateTime => {
                if let Some(date) = dates(None, local.div_euclid(DAY)) {
                    add(format!("{date}T{hour:02}"), start, minute);
                }
                if of_day == 0
                    && let Some(date) = dates(None, local.div_euclid(DAY) - 1)
                {
                    add(format!("{date}T24"), start, 0);
                }
            }
            Kind::Time => {
                if (0..DAY).contains(&local) {
                    add(format!("{hour:02}"), start, minute);
                } else if local == DAY {
                    add("24".to_owned(), start, 0);
                }
            }
            Kind::Date => unreachable!("a date has no time of day"),
        }
    }
    for (prefix, start, low, high) in hours {
        let distance = |minute: i128| start + minute - point.minute;
        let mut alternatives = Vec::new();
        let (shared, span) = match rel {
            Rel::Gt | Rel::Ge => (zones(-ZONE, distance(low) - 1), distance(low)),
            Rel::Lt | Rel::Le => (zones(distance(high) + 1, ZONE), distance(high)),
        };
        if let (Some(shared), Some(all)) = (shared, two_digits(low, high)) {
            alternatives.push(format!("{all}:{ANY_SECONDS}{shared}"));
        }
        // At or past a whole minute every seconds field meets the bound, so
        // the minute's own distance joins the offsets that clear it.
        let any_second = rel == Rel::Ge && point.whole_minute();
        let on_seconds = seconds(rel, point);
        for minute in low..=high {
            let here = distance(minute);
            let partial = match (rel, any_second) {
                (_, true) => zones(span, here),
                (Rel::Gt | Rel::Ge, false) => zones(span, here - 1),
                (Rel::Lt | Rel::Le, false) => zones(here + 1, span),
            };
            let mut tails = Vec::new();
            if let Some(partial) = partial {
                tails.push(format!("{ANY_SECONDS}{partial}"));
            }
            if !any_second
                && let (Some(on), Some(seconds)) = (zones(here, here), on_seconds.as_ref())
            {
                tails.push(format!("{seconds}{on}"));
            }
            if !tails.is_empty() {
                alternatives.push(format!("{minute:02}:(?:{})", tails.join("|")));
            }
        }
        if !alternatives.is_empty() {
            out.push(format!("{prefix}:(?:{})", alternatives.join("|")));
        }
    }
    out
}

/// The anchored pattern of the well-formed lexical forms of the bound's
/// datatype that meet `facet` against it, as the validator compares them; the
/// lexical form must also match [`lexical_patterns`].
pub(super) fn order_pattern(facet: Facet, bound: &Bound) -> String {
    let rel = match facet {
        Facet::MinInclusive => Rel::Ge,
        Facet::MinExclusive => Rel::Gt,
        Facet::MaxInclusive => Rel::Le,
        Facet::MaxExclusive => Rel::Lt,
    };
    let across = match rel {
        Rel::Gt | Rel::Ge => Rel::Gt,
        Rel::Lt | Rel::Le => Rel::Lt,
    };
    let shift = match rel {
        Rel::Gt | Rel::Ge => ZONE,
        Rel::Lt | Rel::Le => -ZONE,
    };
    let kind = bound.kind;
    // Against a bound with a timezone a zoned value compares exactly and a
    // local one only beyond ±14:00 of it (strictly: equality is never
    // determined); against a local bound the other way round.
    let (local, zoned_values) = match bound.zone {
        Some(offset) => {
            let instant = bound.local.shifted(-offset);
            (
                relation(kind, across, &instant.shifted(shift), None),
                zoned(kind, rel, &instant),
            )
        }
        None => (
            relation(kind, rel, &bound.local, None),
            zoned(kind, across, &bound.local.shifted(shift)),
        ),
    };
    let alternatives: Vec<String> = local.into_iter().chain(zoned_values).collect();
    if alternatives.is_empty() {
        return super::numeric_order::NOTHING.to_owned();
    }
    let mut body = String::new();
    for (index, alternative) in alternatives.iter().enumerate() {
        if index > 0 {
            body.push('|');
        }
        write!(body, "{alternative}").expect("String write");
    }
    format!("^{WS}(?:{body}){WS}$")
}

#[cfg(test)]
mod tests {
    use super::*;

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    fn compiled(patterns: &[String]) -> Vec<regex::Regex> {
        patterns
            .iter()
            .map(|pattern| {
                assert!(
                    purrdf_core::xsd_regex::ecma_262_rust_compatible(pattern),
                    "{}",
                    &pattern[..pattern.len().min(200)]
                );
                regex::RegexBuilder::new(pattern)
                    .size_limit(1 << 30)
                    .dfa_size_limit(1 << 30)
                    .build()
                    .expect("pattern compiles")
            })
            .collect()
    }

    /// The validator's verdict: the value parses (trimmed) and compares with
    /// the bound in the facet's direction.
    fn validator(kind: Kind, value: &str, bound: &str, facet: Facet) -> bool {
        let iri = format!("{XSD}{}", kind.local());
        let trim = |text: &str| text.trim_matches(['\t', '\n', '\r', ' ']).to_owned();
        let (Ok(Some(value)), Ok(Some(bound))) = (
            purrdf_xsd::parse_by_iri(&trim(value), &iri),
            purrdf_xsd::parse_by_iri(&trim(bound), &iri),
        ) else {
            return false;
        };
        let ordering = purrdf_xsd::value_cmp(&value, &bound);
        matches!(
            (facet, ordering),
            (
                Facet::MinInclusive,
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ) | (Facet::MinExclusive, Some(std::cmp::Ordering::Greater))
                | (
                    Facet::MaxInclusive,
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                )
                | (Facet::MaxExclusive, Some(std::cmp::Ordering::Less))
        )
    }

    const FACETS: [Facet; 4] = [
        Facet::MinInclusive,
        Facet::MinExclusive,
        Facet::MaxInclusive,
        Facet::MaxExclusive,
    ];

    const ZONES: [&str; 9] = [
        "", "Z", "+00:00", "-00:00", "+05:45", "-05:30", "+14:00", "-14:00", "+13:59",
    ];

    fn format_date(days: i128) -> String {
        let (year, month, day) = civil_from_days(days);
        let sign = if year < 0 { "-" } else { "" };
        format!("{sign}{:04}-{month:02}-{day:02}", year.unsigned_abs())
    }

    /// Local minutes around `center`: the ±14:00 window's edges and the bound's
    /// own minute, at every step a timezone could shift.
    fn minutes_around(center: i128) -> Vec<i128> {
        let mut out = Vec::new();
        for delta in [
            -2 * DAY,
            -DAY - 1,
            -DAY,
            -841,
            -840,
            -839,
            -601,
            -330,
            -61,
            -1,
            0,
            1,
            59,
            345,
            839,
            840,
            841,
            DAY,
            DAY + 1,
            2 * DAY,
        ] {
            out.push(center + delta);
        }
        out
    }

    fn check(kind: Kind, bound: &str, values: &[String]) {
        let parsed = Bound::parse(kind, bound).expect("bound parses");
        for facet in FACETS {
            let mut patterns = lexical_patterns(kind);
            patterns.push(order_pattern(facet, &parsed));
            let regexes = compiled(&patterns);
            for value in values {
                let by_pattern = regexes.iter().all(|regex| regex.is_match(value));
                assert_eq!(
                    by_pattern,
                    validator(kind, value, bound, facet),
                    "{kind:?} {facet:?} {bound:?}: value {value:?}"
                );
            }
        }
    }

    fn date_time_values(center: i128) -> Vec<String> {
        let mut values = Vec::new();
        for minute in minutes_around(center) {
            let day = minute.div_euclid(DAY);
            let of_day = minute.rem_euclid(DAY);
            for seconds in ["00", "00.0", "00.001", "29.5", "30", "30.25", "59.999"] {
                for zone in ZONES {
                    values.push(format!(
                        "{}T{:02}:{:02}:{seconds}{zone}",
                        format_date(day),
                        of_day / 60,
                        of_day % 60
                    ));
                }
            }
            if of_day == 0 {
                for zone in ZONES {
                    values.push(format!("{}T24:00:00{zone}", format_date(day - 1)));
                    values.push(format!(" {}T24:00:00.000{zone}\n", format_date(day - 1)));
                }
            }
        }
        values.extend(
            [
                "2021-02-29T00:00:00",
                "2020-02-30T00:00:00",
                "2020-13-01T00:00:00",
                "+2020-01-01T00:00:00",
                "02020-01-01T00:00:00",
                "2020-01-01T24:00:01",
                "2020-01-01T12:00:5",
                "2020-01-01T12:00:00+14:01",
                "2020-01-01T12:00:00+15:00",
                "2020-01-01T12:00:00.1234567890123456789",
                "2020-01-01 T12:00:00",
                "not a date",
                "",
            ]
            .map(ToOwned::to_owned),
        );
        values
    }

    #[test]
    fn date_time_patterns_agree_with_the_validator() {
        for bound in [
            "2020-03-01T00:00:00",
            "2020-03-01T00:00:00Z",
            "2020-03-01T12:34:56.5",
            "2020-03-01T12:34:56.5-05:30",
            "2020-02-29T23:59:59.999+14:00",
            " 2020-03-01T24:00:00 ",
            "-0001-12-31T23:30:00+01:00",
            "0000-01-01T00:00:00",
        ] {
            let parsed = Bound::parse(Kind::DateTime, bound).expect("bound");
            let values = date_time_values(parsed.local.minute);
            check(Kind::DateTime, bound, &values);
        }
    }

    #[test]
    fn date_patterns_agree_with_the_validator() {
        let mut values = Vec::new();
        let center = days_from_civil(2000, 2, 28);
        for day in center - 3..=center + 3 {
            for zone in ZONES {
                values.push(format!("{}{zone}", format_date(day)));
            }
        }
        for day in [
            days_from_civil(-1, 12, 31),
            days_from_civil(0, 2, 29),
            days_from_civil(1900, 2, 28),
            days_from_civil(10_000, 1, 1),
        ] {
            values.push(format_date(day));
            values.push(format!("{}Z", format_date(day)));
        }
        values.extend(
            [
                "-0000-01-01",
                "1900-02-29",
                "2100-02-29",
                "2400-02-29",
                "-0004-02-29",
                "-0003-02-29",
                "12345-01-01",
                "9223372036854775807-01-01",
                "9223372036854775808-01-01",
                "2000-02-28+14:00",
                "2000-02-28 ",
            ]
            .map(ToOwned::to_owned),
        );
        for bound in [
            "2000-02-28",
            "2000-02-28Z",
            "2000-02-28-14:00",
            "2000-02-29+05:45",
            "-0001-12-31",
        ] {
            check(Kind::Date, bound, &values);
        }
    }

    #[test]
    fn time_patterns_agree_with_the_validator() {
        let mut values = Vec::new();
        for minute in (0..=DAY)
            .step_by(29)
            .chain([0, 1, 719, 720, 721, 1439, 1440])
        {
            for seconds in ["00", "00.000", "00.5", "30.5", "59.9"] {
                for zone in ZONES {
                    if minute == DAY {
                        values.push(format!("24:00:00{zone}"));
                    } else {
                        values.push(format!(
                            "{:02}:{:02}:{seconds}{zone}",
                            minute / 60,
                            minute % 60
                        ));
                    }
                }
            }
        }
        for bound in [
            "12:00:00",
            "12:00:00Z",
            "00:00:00+14:00",
            "23:59:59.5-14:00",
            "24:00:00",
            "06:30:30.25+05:45",
        ] {
            check(Kind::Time, bound, &values);
        }
    }

    #[test]
    fn calendar_arithmetic_round_trips() {
        for days in (-800_000..800_000).step_by(997) {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(days_from_civil(year, month, day), days);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }
}
