// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Host syntax: the three productions an authority's `host` is spelled in
//! (RFC 3986 §3.2.2, imported unchanged by RFC 3987 §2.2 except that
//! `ireg-name` admits `ucschar`).
//!
//! ```text
//! host        = IP-literal / IPv4address / reg-name
//! IP-literal  = "[" ( IPv6address / IPvFuture ) "]"
//! IPv4address = dec-octet "." dec-octet "." dec-octet "." dec-octet
//! dec-octet   = DIGIT / %x31-39 DIGIT / "1" 2DIGIT / "2" %x30-34 DIGIT / "25" %x30-35
//! reg-name    = *( unreserved / pct-encoded / sub-delims )
//! ```
//!
//! These predicates are the workspace's one implementation of each
//! production. [`parse`](crate::parse) and [`parse_uri`](crate::parse_uri)
//! validate every authority's host through them, and any other surface that
//! must decide the same question (a JSON Schema `ipv4` or `ipv6` format, an
//! address literal in a mail domain) asks here rather than carrying a second
//! address parser.
//!
//! Each is a pure function of its argument: no lookup, no socket, no clock.
//!
//! ```rust
//! use purrdf_iri::host::{Mode, is_ipv4_address, is_ipv6_address, is_reg_name};
//!
//! assert!(is_ipv4_address("192.0.2.1"));
//! assert!(!is_ipv4_address("192.0.2.01")); // dec-octet admits no leading zero
//! assert!(is_ipv6_address("2001:db8::ffff:192.0.2.1"));
//! assert!(!is_ipv6_address("fe80::1%eth0")); // no zone identifier
//! assert!(is_reg_name("例え.example", Mode::Iri));
//! assert!(!is_reg_name("例え.example", Mode::Uri));
//! ```

use crate::error::{IriError, Result};
use crate::parse::{Diagnostics, IriReadError, validate_component};

/// Which grammar a production is read under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// RFC 3987: `ucschar` is admitted beside the ASCII classes (and
    /// `iprivate` in a query).
    Iri,
    /// RFC 3986: ASCII only; anything else must be percent-encoded.
    Uri,
}

/// Whether `s` is an RFC 3986 `IPv4address`: four `dec-octet`s joined by
/// `.`, each a decimal value 0–255 written without a leading zero.
///
/// This is the same language as the dotted-quad of RFC 2673 §3.2.
#[must_use]
pub fn is_ipv4_address(s: &str) -> bool {
    let mut octets = 0_usize;
    for octet in s.split('.') {
        octets += 1;
        if octets > 4 || !is_dec_octet(octet.as_bytes()) {
            return false;
        }
    }
    octets == 4
}

/// RFC 3986 `dec-octet`: `0`–`9`, `10`–`99`, `100`–`199`, `200`–`249`,
/// `250`–`255`.
fn is_dec_octet(bytes: &[u8]) -> bool {
    match *bytes {
        [d] => d.is_ascii_digit(),
        [b'1'..=b'9', d] => d.is_ascii_digit(),
        [b'1', d1, d2] => d1.is_ascii_digit() && d2.is_ascii_digit(),
        [b'2', b'0'..=b'4', d] => d.is_ascii_digit(),
        [b'2', b'5', b'0'..=b'5'] => true,
        _ => false,
    }
}

/// Whether `s` is an RFC 3986 `IPv6address`: eight 16-bit groups of one to
/// four hexadecimal digits, at most one `::` standing for one or more zero
/// groups, and optionally the last two groups written as an `IPv4address`.
/// A zone identifier (`%eth0`, RFC 6874) is not part of the production.
#[must_use]
pub fn is_ipv6_address(s: &str) -> bool {
    // The standard parser is pure address syntax: no lookup, socket, clock or
    // other I/O, on native and wasm targets alike. Its language is exactly the
    // nine `IPv6address` alternatives: every compression, the embedded
    // `IPv4address` in the last 32 bits only and without leading zeros, and
    // no zone.
    s.parse::<core::net::Ipv6Addr>().is_ok()
}

/// Whether `s` is a `reg-name` (RFC 3986) under [`Mode::Uri`], or an
/// `ireg-name` (RFC 3987) under [`Mode::Iri`]. The empty string is one: the
/// production is a repetition of zero or more characters.
///
/// Every `IPv4address` is also a `reg-name` spelling, which is why the
/// `host` rule's first-match reading tells the two apart and this predicate
/// does not.
#[must_use]
pub fn is_reg_name(s: &str, mode: Mode) -> bool {
    reg_name(s, 0, mode).is_ok()
}

/// Whether `s` is an RFC 5321 §4.1.3 `address-literal`, brackets included:
/// `"[" ( IPv4-address-literal / IPv6-address-literal ) "]"`, the form a mail
/// domain takes when it is an address rather than a name. (The third form,
/// `General-address-literal`, needs an IANA-registered tag, and the only one
/// registered is `IPv6`.)
///
/// This is not [`is_ipv4_address`] or [`is_ipv6_address`] in brackets: RFC
/// 5321's `Snum` admits leading zeros (`[127.000.0.1]`), its `IPv6` tag is an
/// ABNF string literal and so matches in any case, and an embedded IPv4 part
/// is read as `Snum`s too.
///
/// ```rust
/// use purrdf_iri::host::is_smtp_address_literal;
///
/// assert!(is_smtp_address_literal("[192.0.2.1]"));
/// assert!(is_smtp_address_literal("[127.000.0.1]"));
/// assert!(is_smtp_address_literal("[IPv6:2001:db8::1]"));
/// assert!(is_smtp_address_literal("[ipv6:::ffff:192.0.2.1]"));
/// assert!(!is_smtp_address_literal("192.0.2.1"), "no brackets");
/// assert!(!is_smtp_address_literal("[127.0.0.300]"));
/// assert!(!is_smtp_address_literal("[2001:db8::1]"), "no IPv6 tag");
/// ```
#[must_use]
pub fn is_smtp_address_literal(s: &str) -> bool {
    let Some(inner) = s.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) else {
        return false;
    };
    // The `IPv6` tag is an ABNF string literal, so it is case-insensitive.
    match inner.get(..5) {
        Some(tag) if tag.eq_ignore_ascii_case("IPv6:") => smtp_ipv6(&inner[5..]),
        _ => smtp_ipv4(inner),
    }
}

/// RFC 5321 `Snum = 1*3DIGIT`, "representing a decimal integer value in the
/// range 0 through 255": leading zeros are allowed.
fn is_snum(bytes: &[u8]) -> bool {
    (1..=3).contains(&bytes.len())
        && bytes.iter().all(u8::is_ascii_digit)
        && bytes
            .iter()
            .fold(0_u32, |value, digit| value * 10 + u32::from(digit - b'0'))
            <= 255
}

/// RFC 5321 `IPv4-address-literal = Snum 3("."  Snum)`.
fn smtp_ipv4(text: &str) -> bool {
    let mut parts = 0;
    for part in text.split('.') {
        parts += 1;
        if parts > 4 || !is_snum(part.as_bytes()) {
            return false;
        }
    }
    parts == 4
}

/// RFC 5321 `IPv6-addr = IPv6-full / IPv6-comp / IPv6v4-full / IPv6v4-comp`:
/// eight (or six, before an IPv4 literal) groups of 1–4 hex digits, where a
/// `::` stands for at least two groups, so at most six (four) others may
/// appear.
fn smtp_ipv6(text: &str) -> bool {
    let (hex, v4) = match text.rsplit_once(':') {
        Some((head, tail)) if tail.contains('.') => {
            if !smtp_ipv4(tail) {
                return false;
            }
            // Keep the separating colon when it is half of a `::`.
            let head = if head.ends_with(':') {
                &text[..=head.len()]
            } else {
                head
            };
            (head, true)
        }
        _ => (text, false),
    };
    let full = if v4 { 6 } else { 8 };
    let group = |piece: &str| {
        (1..=4).contains(&piece.len()) && piece.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    let groups = |part: &str| -> Option<usize> {
        if part.is_empty() {
            return Some(0);
        }
        let pieces: Vec<&str> = part.split(':').collect();
        pieces
            .iter()
            .all(|piece| group(piece))
            .then_some(pieces.len())
    };
    match hex.split_once("::") {
        None => groups(hex) == Some(full),
        Some((head, tail)) => {
            let (Some(head), Some(tail)) = (groups(head), groups(tail)) else {
                return false;
            };
            head + tail <= full - 2
        }
    }
}

/// [`is_reg_name`] with the typed diagnostic of the first refused byte,
/// offsets counted from `base_off`.
fn reg_name(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    // unreserved / pct-encoded / sub-delims (+ ucschar in IRI mode).
    validate_component(s, base_off, 0, false, mode)
}

/// Validate an authority's `host` (`s`, found at byte `base_off` of the whole
/// reference).
pub(crate) fn validate_host_with(
    s: &str,
    base_off: usize,
    mode: Mode,
    diagnostics: Diagnostics<'_>,
) -> core::result::Result<(), IriReadError> {
    if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        // IP-literal = "[" ( IPv6address / IPvFuture ) "]". Character
        // membership alone does not establish either production.
        return validate_ip_literal_with(inner, base_off + 1, diagnostics);
    }
    // IPv4address / reg-name. Every IPv4address is a reg-name spelling, so
    // the reg-name check decides the union exactly.
    reg_name(s, base_off, mode).map_err(IriReadError::Lexical)
}

/// Validate an IP-literal's contents (`inner`, found at byte `base_off`)
/// without normalizing its spelling.
fn validate_ip_literal_with(
    inner: &str,
    base_off: usize,
    diagnostics: Diagnostics<'_>,
) -> core::result::Result<(), IriReadError> {
    if matches!(inner.as_bytes().first(), Some(b'v' | b'V')) {
        // IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" ).
        // ABNF string literals are case-insensitive.
        let Some((version, address)) = inner[1..].split_once('.') else {
            return Err(IriError::BadAuthority(diagnostics.message(format_args!(
                "IPvFuture at byte {base_off} requires a version and '.'"
            ))?)
            .into());
        };
        if version.is_empty() || address.is_empty() {
            return Err(IriError::BadAuthority(diagnostics.message(format_args!(
                "IPvFuture at byte {base_off} needs a nonempty version and address"
            ))?)
            .into());
        }
        for (at, ch) in version.char_indices() {
            if !ch.is_ascii_hexdigit() {
                return Err(IriError::DisallowedChar(ch, base_off + 1 + at).into());
            }
        }
        let address_off = base_off + version.len() + 2;
        for (at, ch) in address.char_indices() {
            if !crate::terminals::is_ipvfuture_address_char(ch) {
                return Err(IriError::DisallowedChar(ch, address_off + at).into());
            }
        }
        return Ok(());
    }
    if is_ipv6_address(inner) {
        Ok(())
    } else {
        Err(IriError::BadAuthority(diagnostics.message(format_args!(
            "invalid IPv6 address {inner:?} at byte {base_off}"
        ))?)
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smtp_address_literals_refuse_beside_accepting_neighbours() {
        let cases: &[(&str, &str)] = &[
            ("[127.0.0.300]", "[127.0.0.255]"),
            ("[127.0.0]", "[127.0.0.1]"),
            ("[127.0.0.1.2]", "[127.000.000.001]"),
            ("[1270.0.0.1]", "[127.0.0.1]"),
            ("127.0.0.1", "[127.0.0.1]"),
            ("[IPv6:1::2::3]", "[IPv6:1::2]"),
            ("[IPv6:1:2:3:4:5:6:7::]", "[IPv6:1:2:3:4:5:6::]"),
            ("[IPv6:1:2:3:4:5::1.2.3.4]", "[IPv6:1:2:3:4::1.2.3.4]"),
            ("[IPv6:1:2:3:4:5:6:7:8:9]", "[IPv6:1:2:3:4:5:6:7:8]"),
            ("[IPv6:12345::1]", "[IPv6:1234::1]"),
            ("[IPv6:::1.2.3.256]", "[IPv6:::1.2.3.4]"),
            ("[::1]", "[IPv6:::1]"),
            ("[IPv4:127.0.0.1]", "[ipv6:::1]"),
        ];
        for (invalid, valid) in cases {
            assert!(!is_smtp_address_literal(invalid), "{invalid}");
            assert!(is_smtp_address_literal(valid), "{valid}");
        }
    }

    #[test]
    fn dec_octet_is_exactly_zero_to_255_without_leading_zeros() {
        for value in 0_u32..=999 {
            let plain = value.to_string();
            assert_eq!(
                is_dec_octet(plain.as_bytes()),
                value <= 255,
                "dec-octet {plain}"
            );
            for padded in [format!("0{value}"), format!("00{value}")] {
                assert!(
                    !is_dec_octet(padded.as_bytes()),
                    "a leading zero is refused: {padded}"
                );
            }
        }
        assert!(!is_dec_octet(b""));
        assert!(!is_dec_octet(b"+1"));
        assert!(!is_dec_octet("١".as_bytes()), "only ASCII DIGIT");
    }

    #[test]
    fn ipv4_address_refusals_each_have_an_accepted_neighbour() {
        for (invalid, valid) in [
            ("127.0.0.01", "127.0.0.1"),
            ("256.0.0.1", "255.0.0.1"),
            ("1.2.3", "1.2.3.4"),
            ("1.2.3.4.5", "1.2.3.4"),
            ("1.2.3.4.", "1.2.3.4"),
            ("1..3.4", "1.0.3.4"),
            ("0x7f.0.0.1", "0.0.0.0"),
            (" 1.2.3.4", "1.2.3.4"),
            ("", "0.0.0.0"),
        ] {
            assert!(!is_ipv4_address(invalid), "refuses {invalid:?}");
            assert!(is_ipv4_address(valid), "accepts {valid:?}");
        }
    }

    #[test]
    fn ipv6_address_refusals_each_have_an_accepted_neighbour() {
        for (invalid, valid) in [
            ("1:2:3:4:5:6:7:8:9", "1:2:3:4:5:6:7:8"),
            ("1::2::3", "1::2:3"),
            ("fe80::1%eth0", "fe80::1"),
            ("::ffff:192.168.0.01", "::ffff:192.168.0.1"),
            ("1.2.3.4::", "::1.2.3.4"),
            ("12345::", "1234::"),
            ("1:2:3:4:5:6:7:8::", "1:2:3:4:5:6:7::"),
            ("::1:2:3:4:5:6:7:8", "::2:3:4:5:6:7:8"),
            ("1:2:3:4:5:6:1.2.3.4:5", "1:2:3:4:5:6:1.2.3.4"),
            (":1::", "1::"),
            ("", "::"),
        ] {
            assert!(!is_ipv6_address(invalid), "refuses {invalid:?}");
            assert!(is_ipv6_address(valid), "accepts {valid:?}");
        }
    }

    #[test]
    fn every_ipv4_address_is_a_reg_name() {
        for text in ["0.0.0.0", "255.255.255.255", "192.0.2.1", "10.20.30.40"] {
            assert!(is_ipv4_address(text));
            assert!(is_reg_name(text, Mode::Uri));
            assert!(is_reg_name(text, Mode::Iri));
        }
    }

    #[test]
    fn reg_name_mode_decides_non_ascii() {
        assert!(is_reg_name("", Mode::Uri), "reg-name is *( … )");
        assert!(is_reg_name("example.org", Mode::Uri));
        assert!(is_reg_name("ex%41mple.org", Mode::Uri));
        assert!(!is_reg_name("ex%4mple.org", Mode::Uri));
        assert!(is_reg_name("例え.jp", Mode::Iri));
        assert!(!is_reg_name("例え.jp", Mode::Uri));
        assert!(!is_reg_name("a:b", Mode::Iri), "`:` delimits the port");
        assert!(!is_reg_name("a@b", Mode::Iri), "`@` delimits userinfo");
        assert!(
            !is_reg_name("\u{E000}", Mode::Iri),
            "iprivate is query-only"
        );
    }
}
