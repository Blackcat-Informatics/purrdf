// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RFC-3986 URI + RFC-3987 IRI parser/validator.
//!
//! The component split follows the canonical RFC-3986 Appendix B regular
//! expression, hand-rolled (no `regex` dependency — this crate is wasm-clean and
//! zero-dep). Each component is then validated character-by-character against the
//! permitted grammar class for that component. We validate a **superset** in IRI
//! mode (RFC-3987 adds `ucschar`/`iprivate` to the URI character classes) and the
//! strict ASCII subset in URI mode.

use crate::error::{IriError, Result};
use core::ops::Range;

/// A parsed, validated IRI (or URI) with byte-offset spans for each component.
///
/// The original text is kept **verbatim** (Constitution C0.1 — the IR stores
/// literals/IRIs lexically). Component accessors return borrowed slices; nothing
/// is re-encoded at parse time. [`normalize`](crate::Iri::normalize) produces a
/// new `Iri` with RFC-3986 §6.2.2 syntax-based normalization applied.
///
/// # Examples
///
/// ```rust
/// let iri = purrdf_iri::parse("http://example.org/a/b?x=1#frag")?;
/// assert_eq!(iri.as_str(), "http://example.org/a/b?x=1#frag");
/// assert_eq!(iri.scheme(), Some("http"));
/// assert_eq!(iri.authority(), Some("example.org"));
/// assert_eq!(iri.path(), "/a/b");
/// assert_eq!(iri.query(), Some("x=1"));
/// assert_eq!(iri.fragment(), Some("frag"));
/// assert!(iri.has_scheme());
/// # Ok::<(), purrdf_iri::IriError>(())
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct Iri {
    pub(crate) text: String,
    pub(crate) scheme: Option<Range<usize>>,
    pub(crate) authority: Option<Range<usize>>,
    pub(crate) path: Range<usize>,
    pub(crate) query: Option<Range<usize>>,
    pub(crate) fragment: Option<Range<usize>>,
}

impl Iri {
    /// The full IRI text, verbatim.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The scheme (without the trailing `:`), if present.
    pub fn scheme(&self) -> Option<&str> {
        self.scheme.clone().map(|r| &self.text[r])
    }

    /// The authority (between `//` and the next `/?#`), if present. May be empty
    /// (e.g. `file:///path` has an empty authority — distinct from absent).
    pub fn authority(&self) -> Option<&str> {
        self.authority.clone().map(|r| &self.text[r])
    }

    /// The path component (always present; may be the empty string).
    pub fn path(&self) -> &str {
        &self.text[self.path.clone()]
    }

    /// The query (without the leading `?`), if present.
    pub fn query(&self) -> Option<&str> {
        self.query.clone().map(|r| &self.text[r])
    }

    /// The fragment (without the leading `#`), if present.
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.clone().map(|r| &self.text[r])
    }

    /// `true` iff the IRI is absolute (has a scheme). Note RFC-3986 reserves
    /// "absolute-URI" for a scheme-bearing reference *without* a fragment; here we
    /// use the looser, more useful "has a scheme" sense and treat the fragment
    /// separately via [`Iri::fragment`].
    pub fn has_scheme(&self) -> bool {
        self.scheme.is_some()
    }
}

impl core::fmt::Display for Iri {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.text)
    }
}

impl core::fmt::Debug for Iri {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Iri({:?})", self.text)
    }
}

/// Parse and validate an **IRI** (RFC-3987). Non-ASCII `ucschar`/`iprivate` code
/// points are permitted in the appropriate components.
///
/// # Examples
///
/// ```rust
/// // RFC-3987 permits non-ASCII code points directly.
/// let iri = purrdf_iri::parse("http://example.org/caf\u{e9}")?;
/// assert_eq!(iri.path(), "/caf\u{e9}");
///
/// // Malformed input is a typed hard error, never a degraded fallback.
/// assert!(purrdf_iri::parse("http://example.org/<bad>").is_err());
/// # Ok::<(), purrdf_iri::IriError>(())
/// ```
pub fn parse(s: &str) -> Result<Iri> {
    parse_inner(s, Mode::Iri)
}

/// Parse and validate a **URI** (RFC-3986). Non-ASCII code points are rejected
/// (they must be percent-encoded); everything else matches [`parse`].
///
/// # Examples
///
/// ```rust
/// // Strict ASCII: the percent-encoded spelling is accepted…
/// let uri = purrdf_iri::parse_uri("http://example.org/caf%C3%A9")?;
/// assert_eq!(uri.path(), "/caf%C3%A9");
///
/// // …but the raw non-ASCII code point is rejected in URI mode.
/// assert!(purrdf_iri::parse_uri("http://example.org/caf\u{e9}").is_err());
/// # Ok::<(), purrdf_iri::IriError>(())
/// ```
pub fn parse_uri(s: &str) -> Result<Iri> {
    parse_inner(s, Mode::Uri)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Iri,
    Uri,
}

fn parse_inner(s: &str, mode: Mode) -> Result<Iri> {
    if s.is_empty() {
        return Err(IriError::Empty);
    }

    // ---- Component split (RFC-3986 Appendix B, hand-rolled) -------------------
    // scheme: leading run up to ':' that is a valid scheme AND the ':' is not
    // preceded by '/', '?' or '#' (those would make it part of path/authority).
    let bytes = s.as_bytes();
    let mut idx = 0usize;

    let mut scheme: Option<Range<usize>> = None;
    if let Some(colon) = find_scheme_colon(s) {
        validate_scheme(&s[..colon])?;
        scheme = Some(0..colon);
        idx = colon + 1; // skip ':'
    }

    // authority: only when the remainder starts with "//".
    let mut authority: Option<Range<usize>> = None;
    if bytes[idx..].starts_with(b"//") {
        let astart = idx + 2;
        // authority runs until the next '/', '?', '#' or end.
        let aend =
            astart + find_first_of(&bytes[astart..], *b"/?#").unwrap_or(bytes.len() - astart);
        validate_authority(&s[astart..aend], astart, mode)?;
        authority = Some(astart..aend);
        idx = aend;
    }

    // path: runs until '?' or '#' or end.
    let pstart = idx;
    let pend = pstart + find_first_of(&bytes[pstart..], *b"?#").unwrap_or(bytes.len() - pstart);
    validate_path(&s[pstart..pend], pstart, mode)?;
    // RFC-3986 §4.2: a relative reference with no scheme and no authority has a
    // `path-noscheme`, whose FIRST segment must not contain a ':' — otherwise the
    // reference would be ambiguous with a scheme. Reject it (hard-fail, not a
    // degraded accept). A ':' in a later segment (e.g. `foo/bar:baz`) is fine.
    if scheme.is_none() && authority.is_none() {
        let first_seg = &s[pstart..pend];
        let seg = first_seg.split('/').next().unwrap_or("");
        if let Some(rel) = seg.find(':') {
            return Err(IriError::DisallowedChar(':', pstart + rel));
        }
    }
    let path = pstart..pend;
    idx = pend;

    // query: '?' then until '#' or end.
    let mut query: Option<Range<usize>> = None;
    if idx < bytes.len() && bytes[idx] == b'?' {
        let qstart = idx + 1;
        let qend = qstart + find_first_byte(&bytes[qstart..], b'#').unwrap_or(bytes.len() - qstart);
        validate_query(&s[qstart..qend], qstart, mode)?;
        query = Some(qstart..qend);
        idx = qend;
    }

    // fragment: '#' then the rest.
    let fragment: Option<Range<usize>> = if idx < bytes.len() && bytes[idx] == b'#' {
        let fstart = idx + 1;
        validate_fragment(&s[fstart..], fstart, mode)?;
        Some(fstart..s.len())
    } else {
        None
    };

    Ok(Iri {
        text: s.to_owned(),
        scheme,
        authority,
        path,
        query,
        fragment,
    })
}

/// Locate the `:` that terminates a valid scheme, if any. Returns `None` when the
/// string is a relative reference (the first `:` — if any — is preceded by a
/// `/`, `?`, or `#`, or the leading run is not a valid scheme).
fn find_scheme_colon(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    // First char must be ALPHA for a scheme to exist at all.
    if b.is_empty() || !b[0].is_ascii_alphabetic() {
        return None;
    }
    match find_first_of(b, *b":/?#") {
        Some(i) if b[i] == b':' => Some(i),
        Some(_) | None => None,
    }
}

fn validate_scheme(s: &str) -> Result<()> {
    let b = s.as_bytes();
    if b.is_empty() {
        return Err(IriError::MissingScheme);
    }
    if !b[0].is_ascii_alphabetic() {
        return Err(IriError::BadScheme(s.to_owned()));
    }
    for &c in &b[1..] {
        // scheme tail = ALPHA / DIGIT / `+` `-` `.` (ASCII-only by grammar).
        if !ascii_class(c, SCHEME_TAIL) {
            return Err(IriError::BadScheme(s.to_owned()));
        }
    }
    Ok(())
}

// ---- Character classes (RFC-3986 §2 / RFC-3987 §2.2) -----------------------
//
// The ASCII character classes are precomputed into a const 128-entry bitmap
// (`CLASS`, one byte per ASCII code point), so a per-character grammar check is a
// single table load + mask instead of a chain of `matches!` comparisons. The
// non-ASCII path (`ucschar`/`iprivate`) is unaffected — those code points are
// validated by range test as before.

/// `unreserved` = ALPHA / DIGIT / `-` `.` `_` `~`.
const UNRESERVED: u8 = 1 << 0;
/// `sub-delims` = `!` `$` `&` `'` `(` `)` `*` `+` `,` `;` `=`.
const SUB_DELIMS: u8 = 1 << 1;
/// The literal `:` (a `pchar`/userinfo extra).
const COLON: u8 = 1 << 2;
/// The literal `@` (a `pchar` extra).
const AT: u8 = 1 << 3;
/// The literal `/` (path segment separator; also a query/fragment extra).
const SLASH: u8 = 1 << 4;
/// The literal `?` (a query/fragment extra).
const QUESTION: u8 = 1 << 5;
/// `scheme` tail = ALPHA / DIGIT / `+` `-` `.` (after the mandatory leading ALPHA).
const SCHEME_TAIL: u8 = 1 << 6;

/// Precomputed class bitmap for the ASCII range (`0x00..=0x7F`). Built once at
/// compile time; each entry ORs together every [`UNRESERVED`]/[`SUB_DELIMS`]/…
/// class that its code point belongs to.
const CLASS: [u8; 128] = build_class_table();

const fn build_class_table() -> [u8; 128] {
    let mut table = [0u8; 128];
    let mut i = 0usize;
    while i < 128 {
        let b = i as u8;
        let mut cls = 0u8;
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            cls |= UNRESERVED;
        }
        if matches!(
            b,
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
        ) {
            cls |= SUB_DELIMS;
        }
        if b == b':' {
            cls |= COLON;
        }
        if b == b'@' {
            cls |= AT;
        }
        if b == b'/' {
            cls |= SLASH;
        }
        if b == b'?' {
            cls |= QUESTION;
        }
        if b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.') {
            cls |= SCHEME_TAIL;
        }
        table[i] = cls;
        i += 1;
    }
    table
}

/// `true` iff the ASCII byte belongs to ANY class in `mask`. Non-ASCII bytes carry
/// no ASCII class and return `false`.
#[inline]
fn ascii_class(byte: u8, mask: u8) -> bool {
    byte < 0x80 && CLASS[byte as usize] & mask != 0
}

fn is_unreserved(c: char) -> bool {
    c.is_ascii() && CLASS[c as usize] & UNRESERVED != 0
}

fn is_sub_delims(c: char) -> bool {
    c.is_ascii() && CLASS[c as usize] & SUB_DELIMS != 0
}

/// RFC-3987 §2.2 `ucschar` — the Unicode ranges IRIs add over URIs.
fn is_ucschar(c: char) -> bool {
    let u = c as u32;
    (0xA0..=0xD7FF).contains(&u)
        || (0xF900..=0xFDCF).contains(&u)
        || (0xFDF0..=0xFFEF).contains(&u)
        || (0x1_0000..=0x1_FFFD).contains(&u)
        || (0x2_0000..=0x2_FFFD).contains(&u)
        || (0x3_0000..=0x3_FFFD).contains(&u)
        || (0x4_0000..=0x4_FFFD).contains(&u)
        || (0x5_0000..=0x5_FFFD).contains(&u)
        || (0x6_0000..=0x6_FFFD).contains(&u)
        || (0x7_0000..=0x7_FFFD).contains(&u)
        || (0x8_0000..=0x8_FFFD).contains(&u)
        || (0x9_0000..=0x9_FFFD).contains(&u)
        || (0xA_0000..=0xA_FFFD).contains(&u)
        || (0xB_0000..=0xB_FFFD).contains(&u)
        || (0xC_0000..=0xC_FFFD).contains(&u)
        || (0xD_0000..=0xD_FFFD).contains(&u)
        || (0xE_1000..=0xE_FFFD).contains(&u)
}

/// RFC-3987 §2.2 `iprivate` — permitted only in the query component.
fn is_iprivate(c: char) -> bool {
    let u = c as u32;
    (0xE000..=0xF8FF).contains(&u)
        || (0xF_0000..=0xF_FFFD).contains(&u)
        || (0x10_0000..=0x10_FFFD).contains(&u)
}

/// Extra (beyond ASCII) chars allowed in IRI mode for a given component.
fn iri_extra_ok(c: char, allow_iprivate: bool, mode: Mode) -> bool {
    if mode == Mode::Uri {
        return false; // URIs are ASCII-only; non-ASCII must be percent-encoded.
    }
    is_ucschar(c) || (allow_iprivate && is_iprivate(c))
}

/// Validate that a component string only contains the allowed ASCII set
/// (`unreserved` / `sub-delims` plus every class in `extra_mask`), valid
/// percent-encoding, plus IRI Unicode where permitted.
///
/// The ASCII grammar check is a single [`CLASS`] table lookup + mask; only a
/// non-ASCII byte (a UTF-8 lead byte, always a char boundary here) is decoded to a
/// `char` and routed through the `ucschar`/`iprivate` range test.
fn validate_component(
    s: &str,
    base_off: usize,
    extra_mask: u8,
    allow_iprivate: bool,
    mode: Mode,
) -> Result<()> {
    let bytes = s.as_bytes();
    let allowed = UNRESERVED | SUB_DELIMS | extra_mask;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' {
            // Require exactly two following hex digits.
            if i + 3 > bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return Err(IriError::BadPercentEncoding(base_off + i));
            }
            i += 3;
            continue;
        }
        if b < 0x80 {
            if ascii_class(b, allowed) {
                i += 1;
                continue;
            }
            return Err(IriError::DisallowedChar(b as char, base_off + i));
        }
        // Non-ASCII: `b` is a UTF-8 lead byte at a char boundary (the ASCII bytes
        // before it are single-byte). Decode and apply the IRI Unicode test.
        let c = s[i..].chars().next().expect("non-ASCII byte begins a char");
        if iri_extra_ok(c, allow_iprivate, mode) {
            i += c.len_utf8();
            continue;
        }
        return Err(IriError::DisallowedChar(c, base_off + i));
    }
    Ok(())
}

fn validate_authority(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    // authority = [ userinfo "@" ] host [ ":" port ]
    // Split off optional userinfo (last '@' before host — userinfo cannot contain
    // an unescaped '@', so the first '@' delimits it).
    let (userinfo, rest, host_off) = match find_first_byte(s.as_bytes(), b'@') {
        Some(at) => (Some(&s[..at]), &s[at + 1..], base_off + at + 1),
        None => (None, s, base_off),
    };
    if let Some(ui) = userinfo {
        // userinfo: unreserved / pct / sub-delims / ":"
        validate_component(ui, base_off, COLON, false, mode)?;
    }

    // Split host and optional port. An IP-literal host is bracketed `[...]`.
    let (host, port_off, port) = if rest.starts_with('[') {
        match rest.find(']') {
            Some(close) => {
                let host = &rest[..=close];
                let after = &rest[close + 1..];
                if after.is_empty() {
                    (host, None, None)
                } else if let Some(stripped) = after.strip_prefix(':') {
                    (host, Some(host_off + close + 2), Some(stripped))
                } else {
                    return Err(IriError::BadAuthority(
                        "trailing characters after IP-literal".to_owned(),
                    ));
                }
            }
            None => {
                return Err(IriError::BadAuthority(
                    "unterminated IP-literal '['".to_owned(),
                ));
            }
        }
    } else {
        match rest.rfind(':') {
            Some(colon) => (
                &rest[..colon],
                Some(host_off + colon + 1),
                Some(&rest[colon + 1..]),
            ),
            None => (rest, None, None),
        }
    };

    validate_host(host, host_off, mode)?;
    if let (Some(p), Some(poff)) = (port, port_off) {
        for (k, c) in p.char_indices() {
            if !c.is_ascii_digit() {
                return Err(IriError::DisallowedChar(c, poff + k));
            }
        }
        // A port is a TCP/UDP port number: it must fit in a u16. An empty port
        // (`host:`) is permitted by the grammar (`port = *DIGIT`); a numeric
        // overflow is not — reject it rather than silently accept garbage.
        if !p.is_empty() && p.parse::<u16>().is_err() {
            return Err(IriError::BadAuthority(format!("port out of range: {p}")));
        }
    }
    Ok(())
}

fn validate_host(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        // IP-literal: IPv6address / IPvFuture. Light validation: hex, ':', '.',
        // and (for IPvFuture) 'v', unreserved, sub-delims. Reject empties.
        if inner.is_empty() {
            return Err(IriError::BadAuthority("empty IP-literal".to_owned()));
        }
        for (k, c) in inner.char_indices() {
            let ok = c.is_ascii_hexdigit()
                || matches!(c, ':' | '.' | 'v' | 'V')
                || is_unreserved(c)
                || is_sub_delims(c);
            if !ok {
                return Err(IriError::DisallowedChar(c, base_off + 1 + k));
            }
        }
        return Ok(());
    }
    // reg-name / IPv4: unreserved / pct / sub-delims (+ ucschar in IRI mode).
    validate_component(s, base_off, 0, false, mode)
}

fn validate_path(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    // pchar = unreserved / pct / sub-delims / ":" / "@"; plus "/" segment sep.
    validate_component(s, base_off, COLON | AT | SLASH, false, mode)
}

fn validate_query(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    // query = *( pchar / "/" / "?" ); IRIs additionally allow `iprivate`.
    validate_component(s, base_off, COLON | AT | SLASH | QUESTION, true, mode)
}

fn validate_fragment(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    // fragment = *( pchar / "/" / "?" )
    validate_component(s, base_off, COLON | AT | SLASH | QUESTION, false, mode)
}

/// SWAR word width: one `u64` scans eight bytes per iteration.
const SCAN_WORD: usize = 8;

/// Broadcast a byte to all eight lanes of a `u64`.
const LANES_LO: u64 = 0x0101_0101_0101_0101;
/// High bit of each byte lane.
const LANES_HI: u64 = 0x8080_8080_8080_8080;

#[inline]
fn find_first_byte(bytes: &[u8], needle: u8) -> Option<usize> {
    find_first_of(bytes, [needle])
}

/// Set the high bit of every byte lane in `v` that is zero (classic SWAR
/// zero-byte test); all other lanes report clear.
#[inline]
fn zero_byte_lanes(v: u64) -> u64 {
    v.wrapping_sub(LANES_LO) & !v & LANES_HI
}

/// Find the first ASCII delimiter byte in a dense byte slice.
///
/// IRI component splitting scans long ASCII-heavy strings for a very small set
/// of delimiters. A branch-light SWAR scan over `u64` words (stable Rust, no
/// dependencies — this crate is a zero-dep leaf) keeps the hot path wide
/// without changing the UTF-8 semantics: all delimiter bytes are ASCII and
/// therefore cannot be confused with a non-ASCII continuation byte. Lane
/// order is fixed by `from_le_bytes`, so the result is platform-independent.
#[inline]
fn find_first_of<const N: usize>(bytes: &[u8], needles: [u8; N]) -> Option<usize> {
    if N == 0 {
        return None;
    }

    let mut offset = 0usize;
    while offset + SCAN_WORD <= bytes.len() {
        let word = u64::from_le_bytes(
            bytes[offset..offset + SCAN_WORD]
                .try_into()
                .expect("slice is exactly SCAN_WORD bytes"),
        );
        let mut mask = 0u64;
        for &needle in &needles {
            mask |= zero_byte_lanes(word ^ (u64::from(needle) * LANES_LO));
        }
        if mask != 0 {
            // The first set bit sits in the high bit of the first matching
            // little-endian lane: bit index / 8 = byte index within the word.
            return Some(offset + (mask.trailing_zeros() as usize) / 8);
        }
        offset += SCAN_WORD;
    }

    bytes[offset..]
        .iter()
        .position(|b| needles.contains(b))
        .map(|i| offset + i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swar_delimiter_scan_matches_first_scalar_hit() {
        let haystack = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa?tail#later";
        assert_eq!(find_first_of(haystack, *b"?#"), Some(32));
        assert_eq!(find_first_byte(haystack, b'#'), Some(37));
        assert_eq!(find_first_of(haystack, *b":/"), None);
    }

    /// The const `CLASS` bitmap must reproduce the original per-character grammar
    /// predicates EXACTLY over the whole ASCII range — a one-bit table error would
    /// silently widen or narrow IRI acceptance, which the W3C suite might not pinpoint.
    #[test]
    fn class_table_matches_scalar_predicates() {
        for b in 0u8..128 {
            let c = b as char;
            let cls = CLASS[b as usize];
            assert_eq!(
                cls & UNRESERVED != 0,
                c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'),
                "UNRESERVED mismatch at 0x{b:02X}"
            );
            assert_eq!(
                cls & SUB_DELIMS != 0,
                matches!(
                    c,
                    '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
                ),
                "SUB_DELIMS mismatch at 0x{b:02X}"
            );
            assert_eq!(cls & COLON != 0, c == ':', "COLON mismatch at 0x{b:02X}");
            assert_eq!(cls & AT != 0, c == '@', "AT mismatch at 0x{b:02X}");
            assert_eq!(cls & SLASH != 0, c == '/', "SLASH mismatch at 0x{b:02X}");
            assert_eq!(
                cls & QUESTION != 0,
                c == '?',
                "QUESTION mismatch at 0x{b:02X}"
            );
            assert_eq!(
                cls & SCHEME_TAIL != 0,
                c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'),
                "SCHEME_TAIL mismatch at 0x{b:02X}"
            );
        }
    }
}
