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
use crate::parse::validate_component;
use crate::terminals::hex_value;

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
    is_dotted_quad(s, is_dec_octet)
}

/// Whether `s` is exactly four `.`-joined parts each admitted by `part` —
/// the shape `IPv4address` (RFC 3986, `dec-octet`) and `IPv4-address-literal`
/// (RFC 5321, `Snum`) share, and differ only in the part.
fn is_dotted_quad(s: &str, part: fn(&[u8]) -> bool) -> bool {
    let mut parts = 0_usize;
    for piece in s.split('.') {
        parts += 1;
        if parts > 4 || !part(piece.as_bytes()) {
            return false;
        }
    }
    parts == 4
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

/// Whether `text` is an RFC 5321 §4.1.3 `address-literal` — the bracketed
/// address a mailbox's domain part may carry in place of a host name:
///
/// ```text
/// address-literal      = "[" ( IPv4-address-literal / IPv6-address-literal
///                              / General-address-literal ) "]"
/// IPv4-address-literal = Snum 3("."  Snum)
/// IPv6-address-literal = "IPv6:" IPv6-addr
/// Snum                 = 1*3DIGIT   ; a decimal value in the range 0 through 255
/// IPv6-addr            = IPv6-full / IPv6-comp / IPv6v4-full / IPv6v4-comp
/// IPv6-hex             = 1*4HEXDIG
/// IPv6-full            = IPv6-hex 7(":" IPv6-hex)
/// IPv6-comp            = [IPv6-hex *5(":" IPv6-hex)] "::" [IPv6-hex *5(":" IPv6-hex)]
///                        ; the "::" represents at least 2 16-bit groups of zeros;
///                        ; no more than 6 groups in addition to the "::" may be present
/// IPv6v4-full          = IPv6-hex 5(":" IPv6-hex) ":" IPv4-address-literal
/// IPv6v4-comp          = [IPv6-hex *3(":" IPv6-hex)] "::"
///                        [IPv6-hex *3(":" IPv6-hex) ":"] IPv4-address-literal
///                        ; the "::" represents at least 2 16-bit groups of zeros;
///                        ; no more than 4 groups in addition to the "::" and
///                        ; IPv4-address-literal may be present
/// ```
///
/// The `IPv6` tag is an ABNF string and so case-insensitive (`ipv6:` is
/// accepted). `General-address-literal` (`Standardized-tag ":" 1*dcontent`)
/// requires a tag registered in the IANA "Address Literal Tags" registry,
/// and `IPv6` is the only tag registered there, so the third alternative
/// admits no literal the second does not, and a bracketed literal with any
/// other tag — or no tag and no IPv4 shape — is refused.
///
/// # How this differs from the RFC 3986 productions
///
/// This is deliberately NOT [`is_ipv4_address`] or [`is_ipv6_address`]
/// inside brackets. SMTP and URI address syntax were written apart and
/// disagree at the edges, in both directions:
///
/// * **Leading zeros.** `Snum` is `1*3DIGIT` with a value bound, so
///   `[127.0.0.001]` and `[010.0.0.1]` are lawful literals; RFC 3986
///   `dec-octet` forbids a leading zero and [`is_ipv4_address`] refuses them.
///   The same `Snum` spells the IPv4 tail of an `IPv6v4-*` form, where RFC
///   3986 embeds `IPv4address` instead.
/// * **`::` stands for at least two groups.** `[IPv6:1:2:3:4:5:6:7::]` is
///   refused here (seven groups leave one for the `::`), while RFC 3986's
///   `IPv6address` and [`is_ipv6_address`] admit `1:2:3:4:5:6:7::`. With an
///   IPv4 tail the bound is four groups beside the `::`.
/// * **The tag.** An IPv6 literal must carry the `IPv6:` prefix; RFC 3986
///   writes the address bare inside the brackets.
/// * **What the brackets may hold.** RFC 3986 `IP-literal` admits
///   `IPvFuture` and never an IPv4 address; `address-literal` admits an
///   IPv4 address and has no `IPvFuture`.
///
/// Neither production admits a zone identifier.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::host::{is_ipv4_address, is_ipv6_address, is_smtp_address_literal};
///
/// assert!(is_smtp_address_literal("[192.0.2.1]"));
/// assert!(is_smtp_address_literal("[IPv6:2001:db8::1]"));
/// assert!(is_smtp_address_literal("[IPv6:::ffff:192.0.2.1]"));
/// // Snum admits a leading zero where dec-octet does not.
/// assert!(is_smtp_address_literal("[192.0.2.01]"));
/// assert!(!is_ipv4_address("192.0.2.01"));
/// // `::` must stand for at least two groups here, and for one in RFC 3986.
/// assert!(!is_smtp_address_literal("[IPv6:1:2:3:4:5:6:7::]"));
/// assert!(is_ipv6_address("1:2:3:4:5:6:7::"));
/// // The brackets and the tag are part of the production.
/// assert!(!is_smtp_address_literal("192.0.2.1"));
/// assert!(!is_smtp_address_literal("[2001:db8::1]"));
/// ```
#[must_use]
pub fn is_smtp_address_literal(text: &str) -> bool {
    let Some(inner) = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    match inner.get(..5) {
        Some(tag) if tag.eq_ignore_ascii_case("IPv6:") => is_smtp_ipv6_addr(&inner[5..]),
        _ => is_dotted_quad(inner, is_snum),
    }
}

/// RFC 5321 `Snum = 1*3DIGIT` with a value in `0..=255`: one to three ASCII
/// digits, leading zeros allowed.
fn is_snum(bytes: &[u8]) -> bool {
    if !(1..=3).contains(&bytes.len()) || !bytes.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let value = bytes
        .iter()
        .fold(0_u16, |value, &digit| value * 10 + u16::from(digit - b'0'));
    value <= 255
}

/// RFC 5321 `IPv6-hex = 1*4HEXDIG`.
fn is_ipv6_hex(piece: &str) -> bool {
    (1..=4).contains(&piece.len()) && piece.bytes().all(|b| hex_value(b).is_some())
}

/// How many `":"`-joined `IPv6-hex` groups `part` holds, or `None` when it
/// holds anything else. The empty string is zero groups — the absent
/// optional side of a `::`.
fn ipv6_hex_groups(part: &str) -> Option<usize> {
    if part.is_empty() {
        return Some(0);
    }
    let mut groups = 0;
    for piece in part.split(':') {
        if !is_ipv6_hex(piece) {
            return None;
        }
        groups += 1;
    }
    Some(groups)
}

/// RFC 5321 `IPv6-addr`, the text after the `IPv6:` tag.
fn is_smtp_ipv6_addr(text: &str) -> bool {
    // An `IPv4-address-literal` tail is the text after the last ':' when it
    // holds a '.'. Splitting it off leaves the hex part, keeping the ':'
    // before it only when that ':' is the second half of a `::`.
    let (hex, has_v4) = match text.rsplit_once(':') {
        Some((head, tail)) if tail.contains('.') => {
            if !is_dotted_quad(tail, is_snum) {
                return false;
            }
            let head_len = if head.ends_with(':') {
                head.len() + 1
            } else {
                head.len()
            };
            (&text[..head_len], true)
        }
        _ => (text, false),
    };
    // Eight groups make an address; an IPv4 tail is the last two.
    let full = if has_v4 { 6 } else { 8 };
    match hex.split_once("::") {
        None => ipv6_hex_groups(hex) == Some(full),
        Some((left, right)) => {
            // A second `::` leaves an empty piece in `right`, which
            // `ipv6_hex_groups` refuses. The `::` stands for at least two
            // groups, so at most `full - 2` may be written.
            let (Some(left), Some(right)) = (ipv6_hex_groups(left), ipv6_hex_groups(right)) else {
                return false;
            };
            left + right <= full - 2
        }
    }
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

/// [`is_reg_name`] with the typed diagnostic of the first refused byte,
/// offsets counted from `base_off`.
fn reg_name(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    // unreserved / pct-encoded / sub-delims (+ ucschar in IRI mode).
    validate_component(s, base_off, 0, false, mode)
}

/// Validate an authority's `host` (`s`, found at byte `base_off` of the whole
/// reference).
pub(crate) fn validate_host(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        // IP-literal = "[" ( IPv6address / IPvFuture ) "]". Character
        // membership alone does not establish either production.
        return validate_ip_literal(inner, base_off + 1);
    }
    // IPv4address / reg-name. Every IPv4address is a reg-name spelling, so
    // the reg-name check decides the union exactly.
    reg_name(s, base_off, mode)
}

/// Validate an IP-literal's contents (`inner`, found at byte `base_off`)
/// without normalizing its spelling.
fn validate_ip_literal(inner: &str, base_off: usize) -> Result<()> {
    if matches!(inner.as_bytes().first(), Some(b'v' | b'V')) {
        // IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" ).
        // ABNF string literals are case-insensitive.
        let (version, address) = inner[1..].split_once('.').ok_or_else(|| {
            IriError::BadAuthority(format!(
                "IPvFuture at byte {base_off} requires a version and '.'"
            ))
        })?;
        if version.is_empty() || address.is_empty() {
            return Err(IriError::BadAuthority(format!(
                "IPvFuture at byte {base_off} needs a nonempty version and address"
            )));
        }
        for (at, ch) in version.char_indices() {
            if !ch.is_ascii_hexdigit() {
                return Err(IriError::DisallowedChar(ch, base_off + 1 + at));
            }
        }
        let address_off = base_off + version.len() + 2;
        for (at, ch) in address.char_indices() {
            if !crate::terminals::is_ipvfuture_address_char(ch) {
                return Err(IriError::DisallowedChar(ch, address_off + at));
            }
        }
        return Ok(());
    }
    if is_ipv6_address(inner) {
        Ok(())
    } else {
        Err(IriError::BadAuthority(format!(
            "invalid IPv6 address {inner:?} at byte {base_off}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn snum_is_one_to_three_digits_up_to_255_with_leading_zeros() {
        for value in 0_u32..=999 {
            let plain = value.to_string();
            assert_eq!(is_snum(plain.as_bytes()), value <= 255, "Snum {plain}");
            for padded in [format!("0{value}"), format!("00{value}")] {
                let expected = padded.len() <= 3 && value <= 255;
                assert_eq!(is_snum(padded.as_bytes()), expected, "Snum {padded}");
            }
        }
        assert!(!is_snum(b""));
        assert!(!is_snum(b"+1"));
        assert!(!is_snum(b"-1"));
        assert!(!is_snum(b" 1"));
        assert!(!is_snum(b"0x1"));
        assert!(!is_snum("١".as_bytes()), "only ASCII DIGIT");
        assert!(is_snum(b"000") && is_snum(b"001") && is_snum(b"255"));
        assert!(!is_snum(b"256") && !is_snum(b"0255"));
    }

    #[test]
    fn smtp_ipv4_literal_accepts_what_dec_octet_refuses_and_nothing_wider() {
        for (literal, bare) in [
            ("[127.0.0.1]", "127.0.0.1"),
            ("[0.0.0.0]", "0.0.0.0"),
            ("[255.255.255.255]", "255.255.255.255"),
        ] {
            assert!(is_smtp_address_literal(literal), "{literal}");
            assert!(is_ipv4_address(bare), "{bare}");
        }
        // The difference, pinned from both sides: leading zeros.
        for (literal, bare) in [
            ("[127.0.0.01]", "127.0.0.01"),
            ("[010.0.0.1]", "010.0.0.1"),
            ("[1.2.3.004]", "1.2.3.004"),
        ] {
            assert!(is_smtp_address_literal(literal), "{literal}");
            assert!(!is_ipv4_address(bare), "{bare}");
        }
        // The refusals both share.
        for (invalid, valid) in [
            ("[256.0.0.1]", "[255.0.0.1]"),
            ("[1.2.3]", "[1.2.3.4]"),
            ("[1.2.3.4.5]", "[1.2.3.4]"),
            ("[1.2.3.4.]", "[1.2.3.4]"),
            ("[1..3.4]", "[1.0.3.4]"),
            ("[1.2.3.0004]", "[1.2.3.004]"),
            ("[ 1.2.3.4]", "[1.2.3.4]"),
            ("[1.2.3.4 ]", "[1.2.3.4]"),
            ("[]", "[0.0.0.0]"),
            ("1.2.3.4", "[1.2.3.4]"),
            ("[1.2.3.4", "[1.2.3.4]"),
            ("1.2.3.4]", "[1.2.3.4]"),
            ("[[1.2.3.4]]", "[1.2.3.4]"),
            ("[example.org]", "[1.2.3.4]"),
            ("[IPv4:1.2.3.4]", "[1.2.3.4]"),
        ] {
            assert!(!is_smtp_address_literal(invalid), "refuses {invalid:?}");
            assert!(is_smtp_address_literal(valid), "accepts {valid:?}");
        }
    }

    #[test]
    fn smtp_ipv6_literal_refusals_each_have_an_accepted_neighbour() {
        for (invalid, valid) in [
            // Eight groups, or a `::` standing for at least two.
            ("[IPv6:1:2:3:4:5:6:7]", "[IPv6:1:2:3:4:5:6:7:8]"),
            ("[IPv6:1:2:3:4:5:6:7:8:9]", "[IPv6:1:2:3:4:5:6:7:8]"),
            ("[IPv6:1:2:3:4:5:6:7::]", "[IPv6:1:2:3:4:5:6::]"),
            ("[IPv6:::1:2:3:4:5:6:7]", "[IPv6:::1:2:3:4:5:6]"),
            ("[IPv6:1:2:3::4:5:6:7]", "[IPv6:1:2:3::4:5:6]"),
            ("[IPv6:1::2::3]", "[IPv6:1::2:3]"),
            ("[IPv6:1:::2]", "[IPv6:1::2]"),
            ("[IPv6::1]", "[IPv6:::1]"),
            ("[IPv6:1:]", "[IPv6:1::]"),
            ("[IPv6:]", "[IPv6:::]"),
            // 1*4HEXDIG per group.
            ("[IPv6:12345::]", "[IPv6:1234::]"),
            ("[IPv6:g::]", "[IPv6:f::]"),
            ("[IPv6:1:2:3:4:5:6:7:0x8]", "[IPv6:1:2:3:4:5:6:7:8]"),
            ("[IPv6:1:2:3:4:5:6:7:+8]", "[IPv6:1:2:3:4:5:6:7:8]"),
            ("[IPv6:1:2:3:4:5:6:7: 8]", "[IPv6:1:2:3:4:5:6:7:8]"),
            // An IPv4 tail is the last two groups; `::` still stands for two.
            ("[IPv6:1:2:3:4:5:6:7:1.2.3.4]", "[IPv6:1:2:3:4:5:6:1.2.3.4]"),
            ("[IPv6:1:2:3:4:5:1.2.3.4]", "[IPv6:1:2:3:4:5:6:1.2.3.4]"),
            ("[IPv6:1:2:3:4:5::1.2.3.4]", "[IPv6:1:2:3:4::1.2.3.4]"),
            ("[IPv6:1:2:3::4:5:1.2.3.4]", "[IPv6:1:2:3::4:1.2.3.4]"),
            ("[IPv6:::1.2.3.256]", "[IPv6:::1.2.3.255]"),
            ("[IPv6:::1.2.3]", "[IPv6:::1.2.3.4]"),
            ("[IPv6:::1.2.3.4.5]", "[IPv6:::1.2.3.4]"),
            ("[IPv6:1.2.3.4]", "[IPv6:::1.2.3.4]"),
            ("[IPv6:1.2.3.4::]", "[IPv6:::1.2.3.4]"),
            ("[IPv6:::ffff:1.2.3.4:5]", "[IPv6:::ffff:1.2.3.4]"),
            ("[IPv6:1:2:3:4:5:6:1.2.3.4:]", "[IPv6:1:2:3:4:5:6:1.2.3.4]"),
            // No zone, no brackets inside, the tag spelled and closed.
            ("[IPv6:fe80::1%eth0]", "[IPv6:fe80::1]"),
            ("[IPv6:[::1]]", "[IPv6:::1]"),
            ("[IPv6::1", "[IPv6:::1]"),
            ("IPv6:::1", "[IPv6:::1]"),
            ("[IPv6::: 1]", "[IPv6:::1]"),
            ("[IPv6 ::1]", "[IPv6:::1]"),
            ("[IPv7:::1]", "[IPv6:::1]"),
            ("[IP:::1]", "[IPv6:::1]"),
            ("[::1]", "[IPv6:::1]"),
        ] {
            assert!(!is_smtp_address_literal(invalid), "refuses {invalid:?}");
            assert!(is_smtp_address_literal(valid), "accepts {valid:?}");
        }
    }

    #[test]
    fn smtp_ipv6_literal_pins_every_alternative_of_ipv6_addr() {
        // IPv6-full.
        assert!(is_smtp_address_literal("[IPv6:1:2:3:4:5:6:7:8]"));
        assert!(is_smtp_address_literal(
            "[IPv6:2001:0db8:0000:0000:0000:ff00:0042:8329]"
        ));
        assert!(is_smtp_address_literal(
            "[IPv6:ABCD:ef01:2345:6789:abcd:EF01:2345:6789]"
        ));
        // IPv6-comp, both sides optional, up to six groups in total.
        assert!(is_smtp_address_literal("[IPv6:::]"));
        assert!(is_smtp_address_literal("[IPv6:::1]"));
        assert!(is_smtp_address_literal("[IPv6:1::]"));
        assert!(is_smtp_address_literal("[IPv6:1::8]"));
        assert!(is_smtp_address_literal("[IPv6:1:2:3::6:7:8]"));
        assert!(is_smtp_address_literal("[IPv6:1:2:3:4:5:6::]"));
        assert!(is_smtp_address_literal("[IPv6:::3:4:5:6:7:8]"));
        // IPv6v4-full.
        assert!(is_smtp_address_literal("[IPv6:1:2:3:4:5:6:1.2.3.4]"));
        assert!(is_smtp_address_literal("[IPv6:0:0:0:0:0:ffff:192.0.2.001]"));
        // IPv6v4-comp, up to four groups in total.
        assert!(is_smtp_address_literal("[IPv6:::1.2.3.4]"));
        assert!(is_smtp_address_literal("[IPv6:::ffff:1.2.3.4]"));
        assert!(is_smtp_address_literal("[IPv6:1::1.2.3.4]"));
        assert!(is_smtp_address_literal("[IPv6:1:2::3:1.2.3.4]"));
        assert!(is_smtp_address_literal("[IPv6:1:2:3:4::1.2.3.4]"));
        assert!(is_smtp_address_literal("[IPv6:::1:2:3:4:1.2.3.4]"));
        // The tag is case-insensitive.
        for tag in ["IPv6", "ipv6", "IPV6", "Ipv6"] {
            assert!(is_smtp_address_literal(&format!("[{tag}:::1]")), "{tag}");
        }
    }

    #[test]
    fn smtp_and_rfc_3986_ipv6_agree_except_at_the_documented_edges() {
        // Where both accept.
        for address in [
            "::",
            "::1",
            "1::",
            "1:2:3:4:5:6:7:8",
            "1:2:3::6:7:8",
            "::ffff:192.0.2.1",
            "1:2:3:4:5:6:1.2.3.4",
            "2001:db8::1",
        ] {
            assert!(is_ipv6_address(address), "{address}");
            assert!(
                is_smtp_address_literal(&format!("[IPv6:{address}]")),
                "{address}"
            );
        }
        // RFC 3986 accepts, SMTP refuses: a `::` standing for one group.
        for address in ["1:2:3:4:5:6:7::", "::2:3:4:5:6:7:8", "1:2:3:4:5::1.2.3.4"] {
            assert!(is_ipv6_address(address), "{address}");
            assert!(
                !is_smtp_address_literal(&format!("[IPv6:{address}]")),
                "{address}"
            );
        }
        // SMTP accepts, RFC 3986 refuses: a leading zero in the IPv4 tail.
        for address in ["::ffff:192.168.0.01", "1:2:3:4:5:6:001.002.003.004"] {
            assert!(!is_ipv6_address(address), "{address}");
            assert!(
                is_smtp_address_literal(&format!("[IPv6:{address}]")),
                "{address}"
            );
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
