// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `format` checks the Format-Assertion vocabulary turns on
//! (2020-12 Validation §7.3).
//!
//! Under the default 2020-12 meta-schema `format` is an annotation and none
//! of this runs. A meta-schema that declares the Format-Assertion vocabulary
//! makes it an assertion, and the specification then requires every format it
//! names to be checked completely or the schema refused. Each check below is
//! the grammar of the document the specification cites, implemented in full.
//! `hostname`, `idn-hostname` and `idn-email` are refused as assertions:
//! 2020-12 defines `hostname` to include the Punycode-produced labels of RFC
//! 5891, so a complete check decodes every `xn--` label and applies the
//! IDNA2008 rules to the result, and those need the derived-property, joining
//! type and bidirectional tables of RFC 5892 and RFC 5893. A format name the
//! specification does not define is refused the same way — never silently
//! passed. (The domain of an `email` is RFC 5321's `Domain`, an LDH syntax,
//! and is checked in full.)
//!
//! `ipv4` and `ipv6` are RFC 3986's `IPv4address` and `IPv6address`, which
//! are the languages of the RFC 2673 dotted quad and the RFC 4291 text form
//! the specification cites; they are decided by [`purrdf_iri::host`], the
//! workspace's one implementation of those productions. An `email` address
//! literal is RFC 5321's own grammar, which admits leading zeros in its
//! dotted part and lets `::` stand only for two or more zero groups; it is
//! checked here, over those same productions.

use purrdf_iri::host;

use crate::ecma;

/// A format this crate asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    DateTime,
    Date,
    Time,
    Duration,
    Email,
    Ipv4,
    Ipv6,
    Uri,
    UriReference,
    Iri,
    IriReference,
    Uuid,
    UriTemplate,
    JsonPointer,
    RelativeJsonPointer,
    Regex,
}

impl Format {
    /// The assertable format a name denotes.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "date-time" => Self::DateTime,
            "date" => Self::Date,
            "time" => Self::Time,
            "duration" => Self::Duration,
            "email" => Self::Email,
            "ipv4" => Self::Ipv4,
            "ipv6" => Self::Ipv6,
            "uri" => Self::Uri,
            "uri-reference" => Self::UriReference,
            "iri" => Self::Iri,
            "iri-reference" => Self::IriReference,
            "uuid" => Self::Uuid,
            "uri-template" => Self::UriTemplate,
            "json-pointer" => Self::JsonPointer,
            "relative-json-pointer" => Self::RelativeJsonPointer,
            "regex" => Self::Regex,
            _ => return None,
        })
    }

    /// Whether `text` is a valid instance of the format.
    pub(crate) fn check(self, text: &str) -> bool {
        match self {
            Self::DateTime => date_time(text),
            Self::Date => full_date(text.as_bytes()),
            Self::Time => full_time(text.as_bytes()),
            Self::Duration => duration(text.as_bytes()),
            Self::Email => email(text),
            Self::Ipv4 => host::is_ipv4_address(text),
            Self::Ipv6 => host::is_ipv6_address(text),
            Self::Uri => purrdf_iri::parse_uri(text).is_ok_and(|uri| uri.has_scheme()),
            Self::UriReference => text.is_empty() || purrdf_iri::parse_uri(text).is_ok(),
            Self::Iri => purrdf_iri::parse(text).is_ok_and(|iri| iri.has_scheme()),
            Self::IriReference => text.is_empty() || purrdf_iri::parse(text).is_ok(),
            Self::Uuid => uuid(text.as_bytes()),
            Self::UriTemplate => uri_template(text),
            Self::JsonPointer => json_pointer(text),
            Self::RelativeJsonPointer => relative_json_pointer(text),
            Self::Regex => ecma::is_valid_syntax(text),
        }
    }
}

fn digits(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(bytes.iter().fold(0_u32, |value, digit| {
        value
            .saturating_mul(10)
            .saturating_add(u32::from(digit - b'0'))
    }))
}

const fn is_leap(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// RFC 3339 `full-date`: `YYYY-MM-DD` with a real calendar day.
fn full_date(bytes: &[u8]) -> bool {
    let [y0, y1, y2, y3, b'-', m0, m1, b'-', d0, d1] = *bytes else {
        return false;
    };
    let (Some(year), Some(month), Some(day)) = (
        digits(&[y0, y1, y2, y3]),
        digits(&[m0, m1]),
        digits(&[d0, d1]),
    ) else {
        return false;
    };
    let last = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => return false,
    };
    (1..=last).contains(&day)
}

/// RFC 3339 `full-time`: `partial-time time-offset`, where a leap second is
/// only valid at 23:59:60 UTC.
fn full_time(bytes: &[u8]) -> bool {
    if bytes.len() < 9 {
        return false;
    }
    let [h0, h1, b':', m0, m1, b':', s0, s1, ref rest @ ..] = *bytes else {
        return false;
    };
    let (Some(hour), Some(minute), Some(second)) =
        (digits(&[h0, h1]), digits(&[m0, m1]), digits(&[s0, s1]))
    else {
        return false;
    };
    if hour > 23 || minute > 59 || second > 60 {
        return false;
    }
    let mut rest = rest;
    if let [b'.', fraction @ ..] = rest {
        let length = fraction
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if length == 0 {
            return false;
        }
        rest = &fraction[length..];
    }
    let offset_minutes: i64 = match *rest {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), oh0, oh1, b':', om0, om1] => {
            let (Some(offset_hour), Some(offset_minute)) =
                (digits(&[oh0, oh1]), digits(&[om0, om1]))
            else {
                return false;
            };
            if offset_hour > 23 || offset_minute > 59 {
                return false;
            }
            let magnitude = i64::from(offset_hour * 60 + offset_minute);
            if sign == b'+' { magnitude } else { -magnitude }
        }
        _ => return false,
    };
    if second == 60 {
        let utc = (i64::from(hour * 60 + minute) - offset_minutes).rem_euclid(24 * 60);
        return utc == 23 * 60 + 59;
    }
    true
}

/// RFC 3339 `date-time`: `full-date "T" full-time` (`T` in either case).
fn date_time(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() > 11
        && matches!(bytes[10], b'T' | b't')
        && full_date(&bytes[..10])
        && full_time(&bytes[11..])
}

/// RFC 3339 Appendix A `duration`.
fn duration(bytes: &[u8]) -> bool {
    let Some(rest) = bytes.strip_prefix(b"P") else {
        return false;
    };
    // dur-week = 1*DIGIT "W"
    if let Some(week) = rest.strip_suffix(b"W") {
        return digits(week).is_some();
    }
    let (date, time) = match rest.iter().position(|&byte| byte == b'T') {
        Some(split) => (&rest[..split], Some(&rest[split + 1..])),
        None => (rest, None),
    };
    // dur-date = (dur-day / dur-month / dur-year) [dur-time]; the designators
    // appear in order and each is followed only by the next smaller one.
    let date_ok = designators(date, b"YMD");
    let time_ok = time.is_none_or(|time| !time.is_empty() && designators(time, b"HMS"));
    let has_designator = !date.is_empty() || time.is_some();
    date_ok && time_ok && has_designator
}

/// A run of `1*DIGIT designator` groups whose designators are a contiguous
/// run of `order` (`Y M D`: `Y`, `YM`, `YMD`, `M`, `MD`, `D`).
fn designators(mut bytes: &[u8], order: &[u8]) -> bool {
    let mut expected: Option<usize> = None;
    while !bytes.is_empty() {
        let count = bytes
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if count == 0 || count == bytes.len() {
            return false;
        }
        let Some(position) = order
            .iter()
            .position(|&designator| designator == bytes[count])
        else {
            return false;
        };
        if expected.is_some_and(|next| next != position) {
            return false;
        }
        expected = Some(position + 1);
        bytes = &bytes[count + 1..];
    }
    true
}

/// RFC 5321 §4.1.2 `Domain`: dot-separated labels of 1..=63 letters, digits
/// and hyphens, neither starting nor ending with a hyphen, 253 octets at most
/// (RFC 1123 §2.1 host-name syntax, which `sub-domain` restates).
fn domain(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 253
        && text.split('.').all(|label| {
            let bytes = label.as_bytes();
            (1..=63).contains(&bytes.len())
                && bytes
                    .iter()
                    .all(|&byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && bytes.first() != Some(&b'-')
                && bytes.last() != Some(&b'-')
        })
}

/// RFC 4122 §3 `UUID`: 8-4-4-4-12 hex digits.
fn uuid(bytes: &[u8]) -> bool {
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, &byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

/// RFC 5321 §4.1.2 `Mailbox`: `Local-part "@" ( Domain / address-literal )`.
fn email(text: &str) -> bool {
    let Some(at) = text.rfind('@') else {
        return false;
    };
    let (local, domain) = (&text[..at], &text[at + 1..]);
    local_part(local.as_bytes()) && (self::domain(domain) || address_literal(domain))
}

/// `Dot-string / Quoted-string`.
fn local_part(bytes: &[u8]) -> bool {
    if let [b'"', inner @ .., b'"'] = bytes {
        let mut index = 0;
        while index < inner.len() {
            match inner[index] {
                b'\\' => {
                    if !inner
                        .get(index + 1)
                        .is_some_and(|byte| (32..=126).contains(byte))
                    {
                        return false;
                    }
                    index += 2;
                }
                32..=33 | 35..=91 | 93..=126 => index += 1,
                _ => return false,
            }
        }
        return true;
    }
    !bytes.is_empty()
        && bytes.split(|&byte| byte == b'.').all(|atom| {
            !atom.is_empty()
                && atom.iter().all(|&byte| {
                    byte.is_ascii_alphanumeric() || b"!#$%&'*+-/=?^_`{|}~".contains(&byte)
                })
        })
}

/// `"[" ( IPv4-address-literal / IPv6-address-literal ) "]"`.
fn address_literal(text: &str) -> bool {
    let Some(inner) = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    // The `IPv6` tag is an ABNF string literal, so it is case-insensitive.
    match inner.get(..5) {
        Some(tag) if tag.eq_ignore_ascii_case("IPv6:") => address_literal_v6(&inner[5..]),
        _ => address_literal_v4(inner),
    }
}

/// RFC 5321 §4.1.3 `IPv4-address-literal = Snum 3("." Snum)`, where `Snum =
/// 1*3DIGIT` represents 0 through 255. Unlike RFC 3986's `dec-octet`, `Snum`
/// admits leading zeros.
fn address_literal_v4(text: &str) -> bool {
    let mut parts = 0_usize;
    text.split('.').all(|part| {
        parts += 1;
        (1..=3).contains(&part.len()) && digits(part.as_bytes()).is_some_and(|value| value <= 255)
    }) && parts == 4
}

/// RFC 5321 §4.1.3 `IPv6-addr`: RFC 3986's `IPv6address`, except that its
/// last 32 bits, when dotted, are an [`address_literal_v4`], and that a
/// `::` stands for at least two zero groups ("No more than 6 groups in
/// addition to the `::` may be present", the dotted part counting as two).
fn address_literal_v6(text: &str) -> bool {
    let (hex, dotted) = match text.rsplit_once(':') {
        Some((hex, last)) if last.contains('.') => (hex, Some(last)),
        _ => (text, None),
    };
    let shape_ok = match dotted {
        // The dotted part is checked as RFC 5321 spells it; the rest keeps
        // RFC 3986's shape with that part standing as two groups.
        Some(dotted) => {
            address_literal_v4(dotted) && host::is_ipv6_address(&format!("{hex}:0.0.0.0"))
        }
        None => host::is_ipv6_address(text),
    };
    if !text.contains("::") {
        return shape_ok;
    }
    let explicit = text
        .split(':')
        .filter(|group| !group.is_empty())
        .map(|group| if group.contains('.') { 2 } else { 1 })
        .sum::<usize>();
    shape_ok && explicit <= 6
}

/// RFC 6901 JSON Pointer.
fn json_pointer(text: &str) -> bool {
    crate::pointer::tokens(text).is_some()
}

/// Relative JSON Pointer (draft-bhutton-relative-json-pointer-00):
/// `non-negative-integer [index-manipulation] ( "#" / json-pointer )`.
fn relative_json_pointer(text: &str) -> bool {
    let bytes = text.as_bytes();
    let count = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if count == 0 || (count > 1 && bytes[0] == b'0') {
        return false;
    }
    let mut rest = &text[count..];
    if let Some(after) = rest.strip_prefix(['+', '-']) {
        let manipulation = after.bytes().take_while(u8::is_ascii_digit).count();
        if manipulation == 0 || (manipulation > 1 && after.starts_with('0')) {
            return false;
        }
        rest = &after[manipulation..];
    }
    rest == "#" || json_pointer(rest)
}

/// RFC 6570 §2 URI Template syntax.
fn uri_template(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                let Some(length) = bytes[index + 1..].iter().position(|&byte| byte == b'}') else {
                    return false;
                };
                if !expression(&text[index + 1..index + 1 + length]) {
                    return false;
                }
                index += length + 2;
            }
            b'%' => {
                if !pct_encoded(&bytes[index..]) {
                    return false;
                }
                index += 3;
            }
            // `literals` (§2.1) excludes CTL, SP, DQUOTE, "'", "<", ">", "\",
            // "^", "`", "|" and "}"; non-ASCII (ucschar / iprivate) is admitted.
            0x00..=0x20 | 0x7F | b'"' | b'\'' | b'<' | b'>' | b'\\' | b'^' | b'`' | b'|' | b'}' => {
                return false;
            }
            _ => index += 1,
        }
    }
    true
}

fn pct_encoded(bytes: &[u8]) -> bool {
    matches!(bytes, [b'%', high, low, ..] if high.is_ascii_hexdigit() && low.is_ascii_hexdigit())
}

/// `[ operator ] variable-list`.
fn expression(body: &str) -> bool {
    let body = body
        .strip_prefix(['+', '#', '.', '/', ';', '?', '&', '=', ',', '!', '@', '|'])
        .unwrap_or(body);
    !body.is_empty() && body.split(',').all(varspec)
}

/// `varname [ modifier-level4 ]`.
fn varspec(spec: &str) -> bool {
    let (name, valid_modifier) = if let Some(name) = spec.strip_suffix('*') {
        (name, true)
    } else if let Some((name, length)) = spec.split_once(':') {
        let bytes = length.as_bytes();
        let ok = (1..=4).contains(&bytes.len())
            && bytes[0] != b'0'
            && bytes.iter().all(u8::is_ascii_digit);
        (name, ok)
    } else {
        (spec, true)
    };
    valid_modifier && varname(name.as_bytes())
}

/// `varchar *( ["."] varchar )`, `varchar = ALPHA / DIGIT / "_" / pct-encoded`.
fn varname(bytes: &[u8]) -> bool {
    let mut index = 0;
    let mut previous_dot = true;
    while index < bytes.len() {
        match bytes[index] {
            b'.' if !previous_dot => {
                previous_dot = true;
                index += 1;
            }
            b'%' if pct_encoded(&bytes[index..]) => {
                previous_dot = false;
                index += 3;
            }
            byte if byte.is_ascii_alphanumeric() || byte == b'_' => {
                previous_dot = false;
                index += 1;
            }
            _ => return false,
        }
    }
    !bytes.is_empty() && !previous_dot
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(name: &str, text: &str) -> bool {
        Format::from_name(name).expect("known format").check(text)
    }

    #[test]
    // RFC 6570 templates spell their expressions `{name}`, which is not a
    // format string here.
    #[allow(clippy::literal_string_with_formatting_args)]
    fn each_format_refuses_its_invalid_case_and_accepts_the_neighbour() {
        for (format, invalid, valid) in [
            (
                "date-time",
                "1963-06-19T08:30:06.28123+01:00Z",
                "1963-06-19T08:30:06.283185Z",
            ),
            (
                "date-time",
                "1998-12-31T23:59:60+01:00",
                "1998-12-31T23:59:60Z",
            ),
            ("date", "2021-02-29", "2020-02-29"),
            ("time", "08:30:06", "08:30:06Z"),
            ("time", "22:59:60Z", "23:59:60Z"),
            ("time", "15:59:60-08:01", "15:59:60-08:00"),
            ("duration", "P1Y2D", "P1Y2M3D"),
            ("duration", "PT1H2S", "PT1H2M3S"),
            ("duration", "P", "P4W"),
            ("duration", "PT", "PT36H"),
            ("email", "2962", "joe.bloggs@example.com"),
            ("email", "te..st@example.com", "\"te..st\"@example.com"),
            ("email", "joe@[127.0.0.300]", "joe@[IPv6:::1]"),
            ("email", "joe@-a.example", "joe@a-b.example"),
            ("email", "joe@a..example", "joe@[ipv6:::1]"),
            // RFC 5321's `Snum` admits leading zeros, unlike RFC 3986's
            // `dec-octet`; its value is still at most 255.
            ("email", "joe@[127.0.0.256]", "joe@[127.0.0.001]"),
            ("email", "joe@[127.0.0.0001]", "joe@[IPv6:::127.0.0.001]"),
            // RFC 5321's `::` stands for at least two zero groups.
            (
                "email",
                "joe@[IPv6:1:2:3:4:5:6:7::]",
                "joe@[IPv6:1:2:3:4:5:6::]",
            ),
            (
                "email",
                "joe@[IPv6:1:2:3:4:5::1.2.3.4]",
                "joe@[IPv6:1:2:3:4::1.2.3.4]",
            ),
            (
                "email",
                "joe@[IPv6:fe80::1%eth0]",
                "joe@[IPv6:1:2:3:4:5:6:7:8]",
            ),
            (
                "email",
                "joe@[IPv6:1:2:3:4:5:6:7:8:9]",
                "joe@[IPv6:1:2:3:4:5:6:1.2.3.4]",
            ),
            ("ipv4", "127.0.0.01", "127.0.0.1"),
            ("ipv4", "256.0.0.1", "255.0.0.1"),
            ("ipv6", "1:2:3:4:5:6:7:8:9", "1:2:3:4:5:6:7:8"),
            ("ipv6", "1::2::3", "::ffff:192.168.0.1"),
            ("ipv6", "fe80::1%eth0", "fe80::1"),
            ("uri", "//example.org/relative", "https://example.org/a?b#c"),
            ("uri-reference", "\\\\WINDOWS\\fileshare", "../a"),
            ("iri", "/relative", "https://例え.jp/"),
            ("iri-reference", "http://example.org/<x>", "#frag"),
            (
                "uuid",
                "2eb8aa08-aa98-11ea-b4aa-73b441d1638",
                "2EB8AA08-AA98-11EA-B4AA-73B441D16380",
            ),
            (
                "uri-template",
                "http://example.com/dictionary/{term:1}/{term",
                "http://example.com/{term:1}/{term}",
            ),
            ("uri-template", "a'b", "a%27b"),
            ("json-pointer", "/foo/bar~", "/foo/bar~0"),
            ("relative-json-pointer", "01/a", "0#"),
            ("relative-json-pointer", "0+", "1-1/a"),
            ("regex", "^(abc]", "^(abc)$"),
        ] {
            assert!(!check(format, invalid), "{format} must refuse {invalid:?}");
            assert!(check(format, valid), "{format} must accept {valid:?}");
        }
    }

    #[test]
    fn unassertable_formats_have_no_check() {
        assert_eq!(Format::from_name("idn-hostname"), None);
        assert_eq!(Format::from_name("idn-email"), None);
        assert_eq!(Format::from_name("hostname"), None);
        assert_eq!(Format::from_name("color"), None);
        assert_eq!(Format::from_name("ipv4"), Some(Format::Ipv4));
    }
}
