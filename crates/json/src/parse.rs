// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 8259 §§2, 4–7 grammar; Unicode scalar member names for RFC 6901 §3 paths.

use std::{borrow::Cow, fmt::Write as _, ops::Range};

use purrdf_core::terminals::find_first_json_string_special;

use crate::{Bounds, JsonError, Kind, Value};

pub(crate) fn parse(
    text: &str,
    bounds: Bounds,
) -> Result<(Vec<Value>, Vec<Range<usize>>), JsonError> {
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        at: 0,
        values: Vec::new(),
        bounds,
        pointer_bytes: 0,
    };
    parser.whitespace();
    parser.value(&mut String::new(), 0, None, 0)?;
    parser.whitespace();
    if parser.at != parser.bytes.len() {
        return Err(JsonError::Trailing { at: parser.at });
    }
    let mut runs = Vec::new();
    let mut at = 0;
    for value in parser.values.iter().filter(|value| value.kind.is_scalar()) {
        debug_assert!(value.span.start >= at, "scalar spans follow source order");
        if value.span.start > at {
            runs.push(at..value.span.start);
        }
        at = value.span.end;
    }
    if at < text.len() {
        runs.push(at..text.len());
    }
    Ok((parser.values, runs))
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
    values: Vec<Value>,
    bounds: Bounds,
    pointer_bytes: u64,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    // RFC 8259 §2 lists these four bytes, never a Unicode whitespace property.
    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn syntax(&self, expected: &'static str) -> JsonError {
        JsonError::Syntax {
            at: self.at,
            expected,
        }
    }

    fn expect(&mut self, byte: u8, expected: &'static str) -> Result<(), JsonError> {
        if self.peek() != Some(byte) {
            return Err(self.syntax(expected));
        }
        self.at += 1;
        Ok(())
    }

    fn value(
        &mut self,
        path: &mut String,
        depth: u16,
        parent: Option<usize>,
        ordinal: usize,
    ) -> Result<(), JsonError> {
        if self.values.len() as u64 >= u64::from(self.bounds.max_values) {
            return Err(JsonError::Limit {
                resource: "value occurrences",
                limit: u64::from(self.bounds.max_values),
            });
        }
        self.pointer_bytes += path.len() as u64;
        if self.pointer_bytes > self.bounds.max_pointer_bytes {
            return Err(JsonError::Limit {
                resource: "pointer bytes",
                limit: self.bounds.max_pointer_bytes,
            });
        }
        let kind = match self.peek() {
            Some(b'{') => Kind::Object,
            Some(b'[') => Kind::Array,
            Some(b'"') => Kind::String,
            Some(b't') => Kind::True,
            Some(b'f') => Kind::False,
            Some(b'n') => Kind::Null,
            Some(b'-' | b'0'..=b'9') => Kind::Number,
            _ => return Err(self.syntax("a JSON value")),
        };
        let occurrence = self.values.len();
        self.values.push(Value {
            span: self.at..self.at,
            kind,
            path: path.clone(),
            parent,
            ordinal,
            size: 0,
        });
        let start = self.at;
        let span = match kind {
            Kind::Object | Kind::Array => {
                if depth >= self.bounds.max_depth {
                    return Err(JsonError::Limit {
                        resource: "container depth",
                        limit: u64::from(self.bounds.max_depth),
                    });
                }
                self.container(path, depth + 1, occurrence, kind)?;
                start..self.at
            }
            Kind::String => self.string()?,
            Kind::Number => {
                self.number()?;
                start..self.at
            }
            Kind::True => {
                self.keyword(b"true")?;
                start..self.at
            }
            Kind::False => {
                self.keyword(b"false")?;
                start..self.at
            }
            Kind::Null => {
                self.keyword(b"null")?;
                start..self.at
            }
        };
        self.values[occurrence].span = span;
        Ok(())
    }

    fn container(
        &mut self,
        path: &mut String,
        depth: u16,
        parent: usize,
        kind: Kind,
    ) -> Result<(), JsonError> {
        let object = kind == Kind::Object;
        let close = if object { b'}' } else { b']' };
        self.at += 1;
        self.whitespace();
        if self.peek() == Some(close) {
            self.at += 1;
            return Ok(());
        }
        let mut ordinal = 0;
        loop {
            let restore = path.len();
            if object {
                let span = self.string()?;
                let key = unescape(&self.text[span.clone()], span.start)?;
                push_token(path, &key);
                self.whitespace();
                self.expect(b':', ":")?;
                self.whitespace();
            } else {
                // Writing to String is infallible and avoids a temporary index string.
                write!(path, "/{ordinal}").expect("writing into a String");
            }
            self.value(path, depth, Some(parent), ordinal)?;
            path.truncate(restore);
            ordinal += 1;
            self.values[parent].size = u32::try_from(ordinal)
                .expect("the profile bounds the total value count below u32::MAX");
            self.whitespace();
            if self.peek() == Some(close) {
                self.at += 1;
                return Ok(());
            }
            self.expect(b',', "a comma or closing container delimiter")?;
            self.whitespace();
        }
    }

    fn keyword(&mut self, word: &[u8]) -> Result<(), JsonError> {
        if !self.bytes[self.at..].starts_with(word) {
            return Err(self.syntax("true, false or null"));
        }
        self.at += word.len();
        Ok(())
    }

    fn number(&mut self) -> Result<(), JsonError> {
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(self.syntax("a digit")),
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if !self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                return Err(self.syntax("a fraction digit"));
            }
            self.digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if !self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                return Err(self.syntax("an exponent digit"));
            }
            self.digits();
        }
        Ok(())
    }

    fn digits(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
    }

    /// One string's body span, positioned at its opening quote.
    ///
    /// Each clean run — everything up to the next `"`, `\\` or C0 control, which
    /// RFC 8259 §7 calls `unescaped` — is crossed by one chunked scan of exactly
    /// that class ([`find_first_json_string_special`]); only the byte it stops at
    /// takes the per-byte grammar below. The text is a `&str`, so a run needs no
    /// UTF-8 check, and the scan's class holds only ASCII bytes, so it always
    /// stops on a char boundary.
    fn string(&mut self) -> Result<Range<usize>, JsonError> {
        self.expect(b'"', "a quoted string")?;
        let start = self.at;
        loop {
            let rest = &self.bytes[self.at..];
            self.at += find_first_json_string_special(rest).unwrap_or(rest.len());
            match self.peek() {
                None => return Err(self.syntax("a closing quote")),
                Some(b'"') => {
                    let end = self.at;
                    self.at += 1;
                    return Ok(start..end);
                }
                Some(b'\\') => {
                    self.at += 1;
                    match self.peek() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.at += 1;
                        }
                        Some(b'u') => {
                            self.at += 1;
                            self.hex4()?;
                        }
                        _ => return Err(self.syntax("a JSON escape")),
                    }
                }
                // The scan stops only at `"`, `\\` and the C0 controls.
                Some(_) => return Err(self.syntax("an escaped control character")),
            }
        }
    }

    /// The per-byte string scan [`string`](Self::string) replaced, kept verbatim
    /// as the oracle.
    #[cfg(test)]
    fn string_reference(&mut self) -> Result<Range<usize>, JsonError> {
        self.expect(b'"', "a quoted string")?;
        let start = self.at;
        loop {
            match self.peek() {
                None => return Err(self.syntax("a closing quote")),
                Some(b'"') => {
                    let end = self.at;
                    self.at += 1;
                    return Ok(start..end);
                }
                Some(b'\\') => {
                    self.at += 1;
                    match self.peek() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.at += 1;
                        }
                        Some(b'u') => {
                            self.at += 1;
                            self.hex4()?;
                        }
                        _ => return Err(self.syntax("a JSON escape")),
                    }
                }
                Some(byte) if byte < 0x20 => {
                    return Err(self.syntax("an escaped control character"));
                }
                Some(_) => self.at += 1,
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut result = 0;
        for _ in 0..4 {
            let digit = self
                .peek()
                .and_then(purrdf_hash::hex::nibble)
                .ok_or_else(|| self.syntax("four hexadecimal digits"))?;
            result = result * 16 + u32::from(digit);
            self.at += 1;
        }
        Ok(result)
    }
}

fn push_token(path: &mut String, token: &str) {
    path.push('/');
    for character in token.chars() {
        match character {
            '~' => path.push_str("~0"),
            '/' => path.push_str("~1"),
            other => path.push(other),
        }
    }
}

// Source syntax has already validated each escape. Member names additionally
// require paired surrogate escapes for a representable RFC 6901 pointer. Keep a
// typed error here as well so parser disagreement cannot silently discard a key.
fn unescape(raw: &str, at: usize) -> Result<Cow<'_, str>, JsonError> {
    if !raw.contains('\\') {
        return Ok(Cow::Borrowed(raw));
    }
    let mut output = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        let next = chars.next().ok_or(JsonError::Syntax {
            at,
            expected: "a JSON escape",
        })?;
        match next {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            '/' => output.push('/'),
            'b' => output.push('\u{8}'),
            'f' => output.push('\u{c}'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            'u' => {
                let high = read_code_unit(&mut chars, at)?;
                let codepoint = if (0xD800..0xDC00).contains(&high) {
                    if chars.next() != Some('\\') || chars.next() != Some('u') {
                        return Err(JsonError::LoneSurrogate { at });
                    }
                    let low = read_code_unit(&mut chars, at)?;
                    if !(0xDC00..0xE000).contains(&low) {
                        return Err(JsonError::LoneSurrogate { at });
                    }
                    0x1_0000 + ((high - 0xD800) << 10) + low - 0xDC00
                } else {
                    high
                };
                output.push(char::from_u32(codepoint).ok_or(JsonError::LoneSurrogate { at })?);
            }
            _ => {
                return Err(JsonError::Syntax {
                    at,
                    expected: "a JSON escape",
                });
            }
        }
    }
    Ok(Cow::Owned(output))
}

/// The four hex digits of a `\uXXXX` escape as one UTF-16 code unit.
fn read_code_unit(chars: &mut core::str::Chars<'_>, at: usize) -> Result<u32, JsonError> {
    let mut result = 0;
    for _ in 0..4 {
        let digit = chars
            .next()
            .and_then(|character| u8::try_from(character).ok())
            .and_then(purrdf_hash::hex::nibble)
            .ok_or(JsonError::Syntax {
                at,
                expected: "four hexadecimal digits",
            })?;
        result = result * 16 + u32::from(digit);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::Bounds;

    fn parser(text: &str) -> Parser<'_> {
        Parser {
            text,
            bytes: text.as_bytes(),
            at: 0,
            values: Vec::new(),
            bounds: Bounds::standard(),
            pointer_bytes: 0,
        }
    }

    /// A fixed-seed generator (SplitMix64), so every run draws the same inputs.
    struct SplitMix(u64);

    impl SplitMix {
        const fn next(&mut self) -> u64 {
            purrdf_testkit::rng::splitmix64_next(&mut self.0)
        }

        fn below(&mut self, n: usize) -> usize {
            usize::try_from(self.next() % n as u64).expect("below n")
        }
    }

    /// The chunked string scan agrees with the per-byte one — span, end
    /// position, or the same typed refusal at the same offset — on fixed-seed
    /// bodies holding every special byte, whole and malformed escapes, DEL and
    /// the C1 block (lawful raw), and non-ASCII in every UTF-8 width, at lengths
    /// 0-70 and past several chunks.
    #[test]
    fn chunked_string_scan_agrees_with_the_per_byte_scan() {
        const PIECES: &[&str] = &[
            "\"",
            "\\",
            "\\\"",
            "\\\\",
            "\\/",
            "\\b",
            "\\n",
            "\\u0001",
            "\\uD83D\\uDE00",
            "\\u12",
            "\\x",
            "\u{0}",
            "\u{1}",
            "\t",
            "\n",
            "\u{1f}",
            "\u{7f}",
            " ",
            "\u{85}",
            "\u{e9}",
            "\u{4e2d}",
            "\u{1f600}",
        ];
        let mut rng = SplitMix(0x0150_05CA_9000_0001);
        let (mut ok, mut refused) = (0_usize, 0_usize);
        for len in (0..=70).chain([127, 128, 129, 1000, 4099]) {
            for round in 0..40 {
                let density = if round % 2 == 0 { 4 } else { 60 };
                let mut text = String::from("\"");
                for _ in 0..len {
                    if rng.below(density) == 0 {
                        text.push_str(PIECES[rng.below(PIECES.len())]);
                    } else {
                        text.push('s');
                    }
                }
                if round % 5 != 0 {
                    text.push('"');
                }
                text.push_str(", 1");
                let (mut fast, mut reference) = (parser(&text), parser(&text));
                let got = fast.string();
                let expected = reference.string_reference();
                assert_eq!(format!("{got:?}"), format!("{expected:?}"), "{text:?}");
                assert_eq!(fast.at, reference.at, "{text:?}");
                if got.is_ok() {
                    ok += 1;
                } else {
                    refused += 1;
                }
            }
        }
        assert!(ok > 0 && refused > 0, "{ok} {refused}");
    }
}

/// The frozen JSON string-body decoding vectors, replayed against the member-name unescaper.
#[cfg(test)]
mod json_string_frozen_vectors {
    use purrdf_testkit::vectors::{VectorFile, answer_digest, decode_str, encode_str};

    const STRINGS: &str = include_str!("../../lex/tests/vectors/json_string_vectors.txt");
    const UNITS: &str = include_str!("../../lex/tests/vectors/json_unit_vectors.txt");

    /// `body` (the text between the quotes) decoded, or `None` when refused.
    fn decode(body: &str) -> Option<String> {
        super::unescape(body, 0)
            .ok()
            .map(std::borrow::Cow::into_owned)
    }

    fn answer(body: &str) -> String {
        decode(body).map_or_else(|| "-".to_owned(), |text| encode_str(&format!("={text}")))
    }

    /// Whether this copy is known to answer `body` differently from `expected`.
    fn skipped(body: &str, expected: &str) -> bool {
        let _ = (body, expected);
        false
    }

    #[test]
    fn string_bodies_replay_the_frozen_vectors() {
        let file = VectorFile::parse(STRINGS).expect("json_string_vectors.txt");
        let mut replayed = 0;
        for record in file.records() {
            let body = decode_str(record.fields[0]).expect("an encoded body");
            let expected = decode_str(record.fields[1]).expect("an encoded answer");
            if skipped(&body, &expected) {
                continue;
            }
            assert_eq!(answer(&body), record.fields[1], "{body:?}");
            replayed += 1;
        }
        assert!(replayed > 1000, "{replayed}");
    }

    #[test]
    fn every_unit_and_pair_replays_the_frozen_digests() {
        let file = VectorFile::parse(UNITS).expect("json_unit_vectors.txt");
        for record in file.records() {
            let upper = record.fields[1] == "upper";
            let unit = |u: u32| {
                if upper {
                    format!("\\u{u:04X}")
                } else {
                    format!("\\u{u:04x}")
                }
            };
            let first = u32::from_str_radix(record.fields[2], 16).expect("hex");
            let last = u32::from_str_radix(record.fields[3], 16).expect("hex");
            let answers = (first..=last).map(|value| {
                if record.fields[0] == "unit" {
                    answer(&unit(value))
                } else {
                    let offset = value - 0x1_0000;
                    answer(&format!(
                        "{}{}",
                        unit(0xD800 + (offset >> 10)),
                        unit(0xDC00 + (offset & 0x3FF))
                    ))
                }
            });
            assert_eq!(
                answer_digest(answers),
                record.fields[4],
                "{:?}",
                record.fields
            );
        }
    }
}

/// The frozen JSON Pointer vectors, replayed against the value-path writer.
#[cfg(test)]
mod json_pointer_frozen_vectors {
    use purrdf_testkit::vectors::{VectorFile, answer_digest, decode_str, encode_str};

    const POINTERS: &str = include_str!("../../lex/tests/vectors/json_pointer_vectors.txt");

    fn escape(token: &str) -> String {
        let mut path = String::new();
        super::push_token(&mut path, token);
        path.split_off(1)
    }

    #[test]
    fn pointers_replay_the_frozen_vectors() {
        let file = VectorFile::parse(POINTERS).expect("json_pointer_vectors.txt");
        let mut replayed = 0;
        for record in file.records() {
            let fields = &record.fields;
            match fields[0] {
                "escape" => {
                    let token = decode_str(fields[1]).expect("a token");
                    assert_eq!(encode_str(&escape(&token)), fields[2], "{token:?}");
                    replayed += 1;
                }
                "plane" => {
                    let plane = u32::from_str_radix(fields[1], 16).expect("hex");
                    let answers = ((plane << 16)..=((plane << 16) | 0xFFFF))
                        .filter_map(char::from_u32)
                        .map(|c| encode_str(&escape(&format!("{c}~{c}/"))));
                    assert_eq!(answer_digest(answers), fields[2], "plane {plane:X}");
                    replayed += 1;
                }
                _ => {}
            }
        }
        assert!(replayed > 100, "{replayed}");
    }
}
