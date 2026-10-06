// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 3339 date-time text for the files and tar profiles' `modified` stamps.
//!
//! The grammar and the canonical writer are `purrdf_xsd::rfc3339`; this module
//! names the profiles' reading of it: `full-date` and `full-time` separated by
//! `T`, `t` or a single space (the §5.6 note permits a space for readability),
//! and a leap second only at 23:59:60 UTC on the last day of a month (§5.7,
//! Appendix D). [`format`] writes the one canonical UTC spelling the profiles
//! emit, `YYYY-MM-DDTHH:MM:SS[.f]Z`.

use purrdf_xsd::rfc3339::{LeapSecond, ParseError, Separator};

pub(crate) use purrdf_xsd::rfc3339::format;

/// Parses a `modified` stamp into Unix seconds and nanoseconds in UTC.
pub(crate) fn parse(text: &str) -> Result<(i64, u32), ParseError> {
    purrdf_xsd::rfc3339::parse(text, Separator::GtsSpaceAllowed, LeapSecond::MonthEnd)
}

#[cfg(test)]
mod tests {
    use super::{format, parse};

    /// The profiles' reading: a space separator is accepted, and a leap second
    /// only at a month end.
    #[test]
    fn a_space_separated_month_end_leap_second_is_a_modified_stamp() {
        assert_eq!(parse("1990-12-31 23:59:60Z"), Ok((662_687_999, 0)));
        assert!(parse("1990-12-30 23:59:60Z").is_err());
        assert_eq!(
            format(662_687_999, 0).as_deref(),
            Ok("1990-12-31T23:59:59Z")
        );
    }
}
