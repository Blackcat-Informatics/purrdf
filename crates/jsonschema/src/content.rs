// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The draft-07 content assertions (draft-07 Validation §8): whether a string
//! decodes under its `contentEncoding`, and whether the decoded content is a
//! document of its `contentMediaType`.
//!
//! Draft-07 leaves these assertions to the implementation. This crate makes
//! them for the encoding and the media types it can check completely —
//! `base64` (RFC 4648 §4) and JSON (`application/json` and every `+json`
//! structured-syntax type, RFC 8259) — and reads any other value as the
//! annotation the keyword otherwise is. In 2019-09 and 2020-12 the content
//! keywords are annotations only.

/// A `contentEncoding` this crate decodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Encoding {
    Base64,
}

impl Encoding {
    /// The encoding a `contentEncoding` value names (case-insensitively, as
    /// RFC 2045 §6.1 names them), if it is one this crate decodes.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        name.eq_ignore_ascii_case("base64").then_some(Self::Base64)
    }

    /// The decoded bytes, or `None` when `text` is not in the encoding.
    pub(crate) fn decode(self, text: &str) -> Option<Vec<u8>> {
        match self {
            Self::Base64 => base64(text),
        }
    }
}

/// A `contentMediaType` this crate parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MediaType {
    Json,
}

impl MediaType {
    /// The media type a `contentMediaType` value names — type and subtype
    /// compared case-insensitively, parameters ignored (RFC 2045 §5.1) — if it
    /// is one this crate parses.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        let essence = name.split(';').next().unwrap_or_default().trim();
        let (kind, subtype) = essence.split_once('/')?;
        let subtype = subtype.to_ascii_lowercase();
        (kind.eq_ignore_ascii_case("application")
            && (subtype == "json" || subtype.ends_with("+json")))
        .then_some(Self::Json)
    }

    /// Whether `bytes` is a document of this media type.
    pub(crate) fn check(self, bytes: &[u8]) -> bool {
        match self {
            Self::Json => std::str::from_utf8(bytes).is_ok_and(is_json_text),
        }
    }
}

fn base64_value(byte: u8) -> Option<u32> {
    Some(u32::from(match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    }))
}

/// RFC 4648 §4 base64: groups of four alphabet characters, the last group
/// padded with one or two `=`.
fn base64(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let (groups, _) = bytes.as_chunks::<4>();
    let mut out = Vec::with_capacity(groups.len() * 3);
    for (index, group) in groups.iter().enumerate() {
        let padding = group.iter().rev().take_while(|&&byte| byte == b'=').count();
        if padding > 2 || (padding > 0 && index + 1 != groups.len()) {
            return None;
        }
        let mut word = 0u32;
        for &byte in &group[..4 - padding] {
            word = (word << 6) | base64_value(byte)?;
        }
        word <<= 6 * padding as u32;
        let [_, first, second, third] = word.to_be_bytes();
        out.push(first);
        if padding < 2 {
            out.push(second);
        }
        if padding < 1 {
            out.push(third);
        }
    }
    Some(out)
}

/// Whether `text` is one RFC 8259 JSON text. Checked with an explicit stack,
/// so no nesting depth is too deep to answer.
pub(crate) fn is_json_text(text: &str) -> bool {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        at: 0,
    };
    reader.value() && {
        reader.whitespace();
        reader.at == reader.bytes.len()
    }
}

struct Reader<'t> {
    bytes: &'t [u8],
    at: usize,
}

/// What the parser expects next inside an open container.
#[derive(Clone, Copy)]
enum Open {
    Array,
    Object,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        self.whitespace();
        if self.peek() == Some(byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn literal(&mut self, word: &[u8]) -> bool {
        if self.bytes[self.at..].starts_with(word) {
            self.at += word.len();
            true
        } else {
            false
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
        self.at - start
    }

    fn number(&mut self) -> bool {
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return false,
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if self.digits() == 0 {
                return false;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return false;
            }
        }
        true
    }

    fn string(&mut self) -> bool {
        // The opening quote is consumed by the caller.
        loop {
            match self.peek() {
                None | Some(0x00..=0x1F) => return false,
                Some(b'"') => {
                    self.at += 1;
                    return true;
                }
                Some(b'\\') => {
                    self.at += 1;
                    match self.peek() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.at += 1;
                        }
                        Some(b'u') => {
                            self.at += 1;
                            for _ in 0..4 {
                                if !self.peek().is_some_and(|byte| byte.is_ascii_hexdigit()) {
                                    return false;
                                }
                                self.at += 1;
                            }
                        }
                        _ => return false,
                    }
                }
                Some(_) => self.at += 1,
            }
        }
    }

    /// One scalar, or the opening of a container pushed onto `stack`.
    fn start(&mut self, stack: &mut Vec<Open>) -> Result<bool, ()> {
        self.whitespace();
        let Some(byte) = self.peek() else {
            return Err(());
        };
        let scalar = match byte {
            b'{' => {
                self.at += 1;
                stack.push(Open::Object);
                return Ok(true);
            }
            b'[' => {
                self.at += 1;
                stack.push(Open::Array);
                return Ok(true);
            }
            b'"' => {
                self.at += 1;
                self.string()
            }
            b't' => self.literal(b"true"),
            b'f' => self.literal(b"false"),
            b'n' => self.literal(b"null"),
            _ => self.number(),
        };
        if scalar { Ok(false) } else { Err(()) }
    }

    /// An object member's key and colon.
    fn key(&mut self) -> bool {
        self.eat(b'"') && self.string() && self.eat(b':')
    }

    fn value(&mut self) -> bool {
        let mut stack = Vec::new();
        let Ok(opened) = self.start(&mut stack) else {
            return false;
        };
        // After opening a container: its first member or its immediate close.
        let mut fresh = opened;
        while let Some(&open) = stack.last() {
            let close = match open {
                Open::Array => b']',
                Open::Object => b'}',
            };
            if fresh {
                fresh = false;
                if self.eat(close) {
                    stack.pop();
                    continue;
                }
            } else if self.eat(close) {
                stack.pop();
                continue;
            } else if !self.eat(b',') {
                return false;
            }
            if matches!(open, Open::Object) && !self.key() {
                return false;
            }
            match self.start(&mut stack) {
                Ok(true) => fresh = true,
                Ok(false) => {}
                Err(()) => return false,
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{Encoding, MediaType, is_json_text};

    #[test]
    fn base64_decodes_padded_groups_and_refuses_the_rest() {
        let decode = |text| Encoding::Base64.decode(text);
        assert_eq!(
            decode("eyJmb28iOiAiYmFyIn0K").as_deref(),
            Some(&b"{\"foo\": \"bar\"}\n"[..])
        );
        assert_eq!(decode("ezp9Cg==").as_deref(), Some(&b"{:}\n"[..]));
        assert_eq!(decode("YQ==").as_deref(), Some(&b"a"[..]));
        assert_eq!(decode("YWI=").as_deref(), Some(&b"ab"[..]));
        assert_eq!(decode("").as_deref(), Some(&b""[..]));
        for invalid in [
            "eyJmb28iOi%iYmFyIn0K",
            "{}",
            "YQ=",
            "Y===",
            "YQ==YQ==",
            "YQ ==",
        ] {
            assert_eq!(decode(invalid), None, "{invalid:?}");
        }
    }

    #[test]
    fn media_types_are_read_by_essence() {
        for json in [
            "application/json",
            "Application/JSON",
            "application/json; charset=utf-8",
            "application/schema+json",
        ] {
            assert_eq!(MediaType::from_name(json), Some(MediaType::Json), "{json}");
        }
        for other in ["text/plain", "application/jsonx", "json", "image/png"] {
            assert_eq!(MediaType::from_name(other), None, "{other}");
        }
    }

    #[test]
    fn json_texts_are_recognised_at_any_depth() {
        for valid in [
            "{}",
            "[]",
            " 1 ",
            "-0.5e+3",
            "\"a\\u00e9\"",
            "{\"a\": [1, {\"b\": null}]}",
            "true",
            "[[], {}]",
        ] {
            assert!(is_json_text(valid), "{valid:?}");
        }
        for invalid in [
            "",
            "{\"a\"}",
            "[:]",
            "[1,]",
            "{\"a\" 1}",
            "01",
            "1.",
            "\"\u{1}\"",
            "[1] 2",
            "{\"a\":}",
            "tru",
            "[",
            "{\"a\":1,}",
        ] {
            assert!(!is_json_text(invalid), "{invalid:?}");
        }
        let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
        assert!(is_json_text(&deep));
        let unclosed = "[".repeat(100_000);
        assert!(!is_json_text(&unclosed));
    }
}
