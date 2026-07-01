// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RFC-3986 §6.2.2 syntax-based normalization.
//!
//! Three sub-steps, all syntax-based (no scheme-specific knowledge — §6.2.3
//! scheme-based normalization is deliberately out of scope and would be a separate,
//! per-scheme concern; we do not half-implement it):
//!
//! * §6.2.2.1 **Case** — scheme + host lower-cased; percent-encoding hex digits
//!   upper-cased.
//! * §6.2.2.2 **Percent-encoding** — `%XX` triplets that encode an *unreserved*
//!   character are decoded to that character.
//! * §6.2.2.3 **Path segment** — `remove_dot_segments` applied to the path.
//!
//! Normalization is idempotent: `n.normalize() == n.normalize().normalize()`.

use crate::parse::{parse, Iri};
use crate::resolve::remove_dot_segments;

impl Iri {
    /// Produce a syntax-normalized copy (RFC-3986 §6.2.2). The result is itself
    /// parsed/validated; normalization never yields an invalid IRI.
    pub fn normalize(&self) -> Iri {
        let mut out = String::with_capacity(self.as_str().len());

        if let Some(scheme) = self.scheme() {
            out.push_str(&scheme.to_ascii_lowercase());
            out.push(':');
        }
        if let Some(auth) = self.authority() {
            out.push_str("//");
            out.push_str(&normalize_authority(auth));
        }
        out.push_str(&remove_dot_segments(&pct_normalize(self.path())));
        if let Some(q) = self.query() {
            out.push('?');
            out.push_str(&pct_normalize(q));
        }
        if let Some(frag) = self.fragment() {
            out.push('#');
            out.push_str(&pct_normalize(frag));
        }

        // Re-parse: a normalized IRI is still a valid IRI by construction. The
        // panic path is unreachable for any input that parsed in the first place
        // (we only ever lower-case ASCII and decode unreserved chars), so a parse
        // failure here is a genuine bug, not user input — hence `expect`.
        parse(&out).expect("normalized IRI must re-parse")
    }
}

/// Case-normalize the host (lower-case) without disturbing a case-significant
/// userinfo, then percent-normalize. Userinfo (before `@`) keeps its case; the
/// host + port (after `@`) is lower-cased for the host portion.
fn normalize_authority(auth: &str) -> String {
    let (userinfo, host_port) = match auth.find('@') {
        Some(at) => (Some(&auth[..at]), &auth[at + 1..]),
        None => (None, auth),
    };
    let mut out = String::with_capacity(auth.len());
    if let Some(ui) = userinfo {
        out.push_str(&pct_normalize(ui));
        out.push('@');
    }
    // Lower-case the host (and the port, which is digits-only so case is moot).
    out.push_str(&pct_normalize(&host_port.to_ascii_lowercase()));
    out
}

/// RFC-3986 §6.2.2.1 + §6.2.2.2: upper-case percent-encoding hex digits and decode
/// any `%XX` that encodes an unreserved character.
fn pct_normalize(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = bytes[i + 1];
            let lo = bytes[i + 2];
            if hi.is_ascii_hexdigit() && lo.is_ascii_hexdigit() {
                let decoded = (hex_val(hi) << 4) | hex_val(lo);
                if is_unreserved_byte(decoded) {
                    out.push(decoded as char);
                } else {
                    out.push('%');
                    out.push(hi.to_ascii_uppercase() as char);
                    out.push(lo.to_ascii_uppercase() as char);
                }
                i += 3;
                continue;
            }
        }
        // Non-percent byte: copy the whole UTF-8 char verbatim.
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&s[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => unreachable!("guarded by is_ascii_hexdigit"),
    }
}

fn is_unreserved_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

/// Byte length of the UTF-8 sequence whose leading byte is `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}
