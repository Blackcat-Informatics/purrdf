// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared lexical scalar boundaries and named double-quote escapes.

pub(super) const fn blank(byte: Option<u8>) -> bool {
    matches!(byte, None | Some(b' ' | b'\t' | b'\n' | b'\r'))
}
pub(super) const fn flow_indicator(byte: Option<u8>) -> bool {
    matches!(byte, Some(b',' | b'[' | b']' | b'{' | b'}'))
}
pub(super) fn document_marker(bytes: &[u8]) -> bool {
    document_marker_before(bytes, bytes.get(3).copied())
}
pub(super) fn document_marker_before(bytes: &[u8], next: Option<u8>) -> bool {
    (bytes.starts_with(b"---") || bytes.starts_with(b"...")) && blank(next)
}
// These are exactly the reader's existing one-line stop conditions. Leading
// node/property classification remains the reader's surrounding grammar.
pub(super) fn plain_end(bytes: &[u8], at: usize, flow: bool) -> bool {
    plain_end_before(bytes, at, flow, bytes.get(at + 1).copied())
}
pub(super) fn plain_end_before(bytes: &[u8], at: usize, flow: bool, next: Option<u8>) -> bool {
    let byte = bytes.get(at).copied();
    match byte {
        None | Some(b'\n' | b'\r') => true,
        Some(b':') => blank(next) || flow && flow_indicator(next),
        Some(b'#') => at > 0 && matches!(bytes[at - 1], b' ' | b'\t'),
        _ => flow && flow_indicator(byte),
    }
}

// YAML 1.2.2 §5.1 c-printable. Quoted content can retain BOM; plain/block
// non-break content excludes it. LF/CR losslessness is checked by each style.
pub(super) const fn printable(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | ' '..='~' | '\u{85}' | '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
}

const SHORT_ESCAPES: &[(u8, char)] = &[
    (b'0', '\0'),
    (b'a', '\u{7}'),
    (b'b', '\u{8}'),
    (b't', '\t'),
    (b'\t', '\t'),
    (b'n', '\n'),
    (b'v', '\u{b}'),
    (b'f', '\u{c}'),
    (b'r', '\r'),
    (b'e', '\u{1b}'),
    (b' ', ' '),
    (b'"', '"'),
    (b'/', '/'),
    (b'\\', '\\'),
    (b'N', '\u{85}'),
    (b'_', '\u{a0}'),
    (b'L', '\u{2028}'),
    (b'P', '\u{2029}'),
];
pub(super) fn short_escape(name: u8) -> Option<char> {
    SHORT_ESCAPES
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| *value)
}
pub(super) fn preferred_escape(c: char) -> Option<u8> {
    SHORT_ESCAPES
        .iter()
        .find(|(_, value)| *value == c)
        .map(|(name, _)| *name)
}
pub(super) const fn hex_width(name: u8) -> Option<usize> {
    match name {
        b'x' => Some(2),
        b'u' => Some(4),
        b'U' => Some(8),
        _ => None,
    }
}

pub(super) const fn all_escape_bits() -> u128 {
    let mut bits = 0;
    let mut at = 0;
    while at < SHORT_ESCAPES.len() {
        bits |= 1_u128 << SHORT_ESCAPES[at].0;
        at += 1;
    }
    bits | (1_u128 << b'x') | (1_u128 << b'u') | (1_u128 << b'U')
}
