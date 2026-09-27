// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC-3986 URI + RFC-3987 IRI parser/validator.
//!
//! The component split follows the canonical RFC-3986 Appendix B regular
//! expression, hand-rolled (no `regex` dependency — this crate is wasm-clean and
//! zero-dep). Each component is then validated character-by-character against the
//! permitted grammar class for that component. We validate a **superset** in IRI
//! mode (RFC-3987 adds `ucschar`/`iprivate` to the URI character classes) and the
//! strict ASCII subset in URI mode.

use crate::error::{IriError, Result};
use crate::host::Mode;
use crate::scan::{ByteRun, byte_runs, count_runs, in_runs};
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

/// Validate `s` as an **IRI** and report whether it carries a scheme, without
/// building the owned [`Iri`].
///
/// Exactly `parse(s).map(|iri| iri.has_scheme())`, minus the copy of `s` that
/// answer would have been read off and then dropped. Same grammar, same errors,
/// same verdict — the internal `scan` is the single body both run, so this cannot come to
/// accept or reject anything [`parse`] does not.
///
/// This is for the caller that asks nothing but "is this acceptable, and is it
/// absolute". A query algebra's soundness walk is the motivating one: it asks that
/// of every IRI in the query, on every validation of the plan, and dropped a heap
/// `String` per IRI to read one bit.
///
/// # Errors
///
/// Whatever [`parse`] returns for the same input.
///
/// # Examples
///
/// ```rust
/// assert!(purrdf_iri::is_absolute("http://example.org/a")?);
/// assert!(!purrdf_iri::is_absolute("/a/b")?);
///
/// // Rejection is unchanged: the grammar is the one `parse` runs.
/// assert!(purrdf_iri::is_absolute("http://example.org/<bad>").is_err());
/// # Ok::<(), purrdf_iri::IriError>(())
/// ```
pub fn is_absolute(s: &str) -> Result<bool> {
    Ok(classify(s)? == IriForm::Absolute)
}

/// Whether a VALIDATED IRI reference carries a scheme — the one bit of [`Iri`] a
/// caller that only needs to accept-or-reject actually reads.
///
/// Returned by [`classify`], which runs the identical grammar [`parse`] runs and
/// stops before the owned [`Iri`] is built.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum IriForm {
    /// The reference has a scheme, so it IS an IRI (RFC-3986 §4.3).
    Absolute,
    /// The reference has no scheme and means nothing without a base (§4.2).
    Relative,
}

/// Validate `s` against the RFC-3987 IRI grammar and report only whether it is
/// absolute, WITHOUT materializing the parsed [`Iri`].
///
/// [`parse`] owns a copy of `s` so its component accessors can hand back slices; a
/// caller that asks nothing but "is this acceptable, and is it absolute" pays for
/// that copy and drops it unread. The store-once term tables of a dataset do exactly
/// that once per distinct IRI, which is once per IRI in a whole pack. Same grammar,
/// same errors — [`scan`] is the single body both entry points run.
pub(crate) fn classify(s: &str) -> Result<IriForm> {
    let spans = scan(s, Mode::Iri)?;
    Ok(if spans.scheme.is_some() {
        IriForm::Absolute
    } else {
        IriForm::Relative
    })
}

/// The component spans [`scan`] computes: [`Iri`] without its owned text.
struct Spans {
    scheme: Option<Range<usize>>,
    authority: Option<Range<usize>>,
    path: Range<usize>,
    query: Option<Range<usize>>,
    fragment: Option<Range<usize>>,
}

fn parse_inner(s: &str, mode: Mode) -> Result<Iri> {
    let spans = scan(s, mode)?;
    Ok(Iri {
        text: s.to_owned(),
        scheme: spans.scheme,
        authority: spans.authority,
        path: spans.path,
        query: spans.query,
        fragment: spans.fragment,
    })
}

/// Split and validate `s`'s components, returning their spans. The whole grammar
/// lives here; [`parse_inner`] adds the owned text and nothing else.
fn scan(s: &str, mode: Mode) -> Result<Spans> {
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

    Ok(Spans {
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
pub(crate) fn validate_component(
    s: &str,
    base_off: usize,
    extra_mask: u8,
    allow_iprivate: bool,
    mode: Mode,
) -> Result<()> {
    debug_assert!(
        extra_mask & !COMPONENT_EXTRAS == 0,
        "a component admits only the single-byte extra classes"
    );
    let bytes = s.as_bytes();
    let allowed = UNRESERVED | SUB_DELIMS | extra_mask;
    let mut i = 0usize;
    while i < bytes.len() {
        // Clean-run precheck: skip every leading byte of the next eight that the
        // per-byte arm below would accept as an ASCII class member, all at once.
        // It stops at the first byte that needs that arm — a `%`, a non-ASCII
        // lead, or a refused byte — which the arm then handles exactly as
        // before, so acceptance, errors and offsets are unchanged.
        if let Some(word) = bytes[i..].first_chunk::<CLEAN_WORD>() {
            let run = clean_prefix_len(word, extra_mask);
            i += run;
            if run == CLEAN_WORD {
                continue;
            }
        }
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

/// The width of the [`validate_component`] clean-run precheck: one `u64` of bytes.
const CLEAN_WORD: usize = 8;

/// The extra classes a component may admit beyond `unreserved` / `sub-delims`,
/// each of which is a single byte.
const COMPONENT_EXTRAS: u8 = COLON | AT | SLASH | QUESTION;

/// The [`CLASS`] members of `mask`, over all 256 byte values (non-ASCII bytes
/// carry no ASCII class).
const fn class_members(mask: u8) -> [u8; 256] {
    let mut table = [0_u8; 256];
    let mut i = 0;
    while i < CLASS.len() {
        if CLASS[i] & mask != 0 {
            table[i] = 1;
        }
        i += 1;
    }
    table
}

/// The one byte a single-byte class holds, found in [`CLASS`] so the class
/// table stays the only place the class is spelled.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the index is below CLASS.len() == 128; u8::try_from is not const-callable"
)]
const fn sole_member(mask: u8) -> u8 {
    let mut found = None;
    let mut i = 0;
    while i < CLASS.len() {
        if CLASS[i] & mask != 0 {
            assert!(
                found.is_none(),
                "a single-byte class has exactly one member"
            );
            found = Some(i as u8);
        }
        i += 1;
    }
    match found {
        Some(b) => b,
        None => panic!("a single-byte class has exactly one member"),
    }
}

/// `unreserved` / `sub-delims`, the classes every component admits, as byte runs
/// derived from [`CLASS`] at compile time.
const COMPONENT_BASE_TABLE: [u8; 256] = class_members(UNRESERVED | SUB_DELIMS);
const COMPONENT_BASE_RUNS: [ByteRun; count_runs(&COMPONENT_BASE_TABLE)] =
    byte_runs(&COMPONENT_BASE_TABLE);
const COLON_BYTE: u8 = sole_member(COLON);
const AT_BYTE: u8 = sole_member(AT);
const SLASH_BYTE: u8 = sole_member(SLASH);
const QUESTION_BYTE: u8 = sole_member(QUESTION);

/// How many leading bytes of `word` are ASCII members of
/// `UNRESERVED | SUB_DELIMS | extra_mask`, as comparisons only.
///
/// Each lane is the class test written as run comparisons plus one equality
/// per admitted extra byte, OR-ed without a branch, and stored as a `0x00` /
/// `0xFF` byte. The eight lanes read as one little-endian `u64`, and its
/// trailing ones over 8 are the clean prefix. Independent byte lanes with no
/// cross-lane dependence are the shape the compiler lowers to packed byte
/// compares where the target has them.
#[allow(
    clippy::inline_always,
    reason = "the precheck must be inlined into the component loop so its lane compares \
              vectorize there rather than behind a call per eight bytes"
)]
#[allow(
    clippy::needless_bitwise_bool,
    reason = "every lane is evaluated without a branch; a lazy `||` would reintroduce \
              the per-byte branch the precheck exists to remove"
)]
#[inline(always)]
fn clean_prefix_len(word: &[u8; CLEAN_WORD], extra_mask: u8) -> usize {
    let colon = extra_mask & COLON != 0;
    let at = extra_mask & AT != 0;
    let slash = extra_mask & SLASH != 0;
    let question = extra_mask & QUESTION != 0;
    let mut lanes = [0_u8; CLEAN_WORD];
    for (lane, &b) in lanes.iter_mut().zip(word) {
        let clean = in_runs(b, &COMPONENT_BASE_RUNS)
            | (colon & (b == COLON_BYTE))
            | (at & (b == AT_BYTE))
            | (slash & (b == SLASH_BYTE))
            | (question & (b == QUESTION_BYTE));
        *lane = u8::from(clean).wrapping_neg();
    }
    (u64::from_le_bytes(lanes).trailing_ones() / u8::BITS) as usize
}

fn validate_authority(s: &str, base_off: usize, mode: Mode) -> Result<()> {
    // authority = [ userinfo "@" ] host [ ":" port ]
    // Userinfo cannot contain an unescaped '@', so the first '@' delimits it.
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

    crate::host::validate_host(host, host_off, mode)?;
    if let (Some(p), Some(poff)) = (port, port_off) {
        for (k, c) in p.char_indices() {
            if !c.is_ascii_digit() {
                return Err(IriError::DisallowedChar(c, poff + k));
            }
        }
        // RFC 3986 §3.2.3 defines the generic syntax as `port = *DIGIT`.
        // Transport ranges belong to a scheme's connection policy, not IRI
        // identity. Empty, long, and leading-zero digit strings remain lexical.
    }
    Ok(())
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

    /// **`is_absolute` is `parse(..).map(has_scheme)`, for every input.**
    ///
    /// The whole claim the non-owning entry point rests on is that it decides
    /// EXACTLY what the owning one decides — same acceptance, same rejection, same
    /// verdict — so it is asserted as that equivalence over a corpus that reaches
    /// both answers and both failure modes, rather than as a list of expected
    /// values a later grammar change could drift away from on one side only.
    ///
    /// The corpus deliberately pairs each rejected spelling with the accepted
    /// neighbour it differs from by one construct: a scan that started refusing at
    /// the wrong place would break the pair, not just the refusal.
    #[test]
    fn is_absolute_decides_exactly_what_parse_decides() {
        const CORPUS: &[&str] = &[
            // Absolute, and the relative neighbour of each.
            "http://example.org/a/b?x=1#frag",
            "//example.org/a/b?x=1#frag",
            "urn:example:thing",
            "example:thing",
            "http://example.org/caf\u{e9}",
            "/caf\u{e9}",
            "file:///path",
            "///path",
            // Relative references that must stay relative rather than be read as
            // schemes.
            "a/b",
            "./a:b",
            "#frag",
            "?q",
            "",
            // Rejected by the grammar on BOTH sides of the scheme question: a scan
            // that stopped once it had seen a scheme would admit the first of
            // these, and one that stopped at the first segment would admit the
            // last.
            "http://example.org/<bad>",
            "<bad>",
            "a:b/c",
            "http://exa mple.org/",
        ];
        for input in CORPUS {
            let owned = parse(input).map(|iri| iri.has_scheme());
            let borrowed = is_absolute(input);
            match (&owned, &borrowed) {
                (Ok(owned), Ok(borrowed)) => assert_eq!(
                    owned, borrowed,
                    "{input:?}: `parse` says has_scheme = {owned}, `is_absolute` says {borrowed}"
                ),
                (Err(owned), Err(borrowed)) => assert_eq!(
                    owned.to_string(),
                    borrowed.to_string(),
                    "{input:?}: the two entry points must fail with the SAME error, or one of \
                     them is running a different grammar"
                ),
                _ => panic!(
                    "{input:?}: one entry point accepted and the other refused — \
                     parse: {owned:?}, is_absolute: {borrowed:?}"
                ),
            }
        }
        // Non-vacuity: the corpus must actually reach all three verdicts, or the
        // equivalence above is asserted over a corpus that proves one of them.
        let verdicts: Vec<Option<bool>> = CORPUS.iter().map(|s| is_absolute(s).ok()).collect();
        for (wanted, label) in [
            (Some(true), "an accepted ABSOLUTE reference"),
            (Some(false), "an accepted RELATIVE reference"),
            (None, "a REFUSED reference"),
        ] {
            assert!(
                verdicts.contains(&wanted),
                "the corpus never produces {label}, so the equivalence is untested there"
            );
        }
    }

    /// `validate_component` as it was before the clean-run precheck: the
    /// per-byte loop alone, kept as the oracle the precheck must not move.
    fn validate_component_reference(
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
            let c = s[i..].chars().next().expect("non-ASCII byte begins a char");
            if iri_extra_ok(c, allow_iprivate, mode) {
                i += c.len_utf8();
                continue;
            }
            return Err(IriError::DisallowedChar(c, base_off + i));
        }
        Ok(())
    }

    /// Every combination of the extra classes a component may admit.
    fn extra_masks() -> impl Iterator<Item = u8> {
        (0_u8..16).map(|bits| {
            [COLON, AT, SLASH, QUESTION]
                .iter()
                .enumerate()
                .filter(|&(k, _)| bits & (1 << k) != 0)
                .fold(0, |mask, (_, &class)| mask | class)
        })
    }

    /// The comparison form of the precheck agrees with the class table on every
    /// byte value, for every extra mask, in every lane position.
    #[test]
    fn clean_prefix_compares_match_the_class_table() {
        for extra in extra_masks() {
            let allowed = UNRESERVED | SUB_DELIMS | extra;
            for b in 0..=u8::MAX {
                let member = ascii_class(b, allowed);
                for lane in 0..CLEAN_WORD {
                    let mut word = [b'a'; CLEAN_WORD];
                    word[lane] = b;
                    let expected = if member { CLEAN_WORD } else { lane };
                    assert_eq!(
                        clean_prefix_len(&word, extra),
                        expected,
                        "byte {b:#04X} in lane {lane}, extra {extra:#04X}"
                    );
                }
            }
        }
    }

    /// Fixed-seed random components: the precheck path returns the same verdict,
    /// error and offset as the per-byte path, for every extra mask, both modes and
    /// both `iprivate` settings.
    #[test]
    fn validate_component_matches_the_per_byte_reference() {
        // Every byte class boundary in ASCII, a percent in each shape, and
        // scalars on both sides of the `ucschar`/`iprivate` edges.
        const PIECES: &[&str] = &[
            "a",
            "Z",
            "0",
            "9",
            "-",
            ".",
            "_",
            "~",
            "!",
            "$",
            "&",
            "'",
            "(",
            ")",
            "*",
            "+",
            ",",
            ";",
            "=",
            ":",
            "@",
            "/",
            "?",
            "#",
            "[",
            "]",
            " ",
            "\"",
            "<",
            ">",
            "\\",
            "^",
            "`",
            "{",
            "|",
            "}",
            "\u{7f}",
            "\u{0}",
            "\u{1f}",
            "%",
            "%4",
            "%41",
            "%4g",
            "%zz",
            "\u{a0}",
            "\u{9f}",
            "\u{e9}",
            "\u{d7ff}",
            "\u{e000}",
            "\u{f8ff}",
            "\u{f900}",
            "\u{fdd0}",
            "\u{fffd}",
            "\u{fffe}",
            "\u{10000}",
            "\u{1fffe}",
            "\u{e0fff}",
            "\u{e1000}",
            "\u{f0000}",
            "\u{10fffd}",
        ];
        let mut state = 0x01B1_C0DE_5EED_u64;
        let mut next = move || {
            usize::try_from(crate::test_rng::splitmix64_next(&mut state) % 1_000_003)
                .expect("small")
        };
        let mut verdicts = [0_usize; 3];
        for len in (0..=70).chain([128, 400]) {
            for _ in 0..30 {
                let mut s = String::new();
                while s.len() < len {
                    // Mostly clean bytes, so the refusals land at every offset.
                    if next() % 5 == 0 {
                        s.push_str(PIECES[next() % PIECES.len()]);
                    } else {
                        s.push(
                            "abcXYZ019-._~!$&'()*+,;="[next() % 24..]
                                .chars()
                                .next()
                                .expect("ascii"),
                        );
                    }
                }
                for extra in extra_masks() {
                    for allow_iprivate in [false, true] {
                        for mode in [Mode::Iri, Mode::Uri] {
                            let got = validate_component(&s, 7, extra, allow_iprivate, mode);
                            let expected =
                                validate_component_reference(&s, 7, extra, allow_iprivate, mode);
                            assert_eq!(
                                got.as_ref().map_err(ToString::to_string),
                                expected.as_ref().map_err(ToString::to_string),
                                "{s:?} extra {extra:#04X} iprivate {allow_iprivate}"
                            );
                            verdicts[match &got {
                                Ok(()) => 0,
                                Err(IriError::DisallowedChar(..)) => 1,
                                Err(_) => 2,
                            }] += 1;
                        }
                    }
                }
            }
        }
        // Non-vacuity: the corpus accepts, refuses a character, and refuses a
        // percent-encoding.
        assert!(verdicts.iter().all(|&n| n > 0), "{verdicts:?}");
    }

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
