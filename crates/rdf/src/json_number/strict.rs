// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded container framing with typed serde scalar decoding. In particular,
//! arbitrary-precision numbers never pass through `deserialize_any`'s map
//! representation, so genuine object keys cannot impersonate a number.

use serde::de::{DeserializeOwned, Error as _};
use serde_json::{Map, Number, Value};

pub(crate) fn parse(bytes: &[u8], values: usize, depth: usize) -> Result<Value, serde_json::Error> {
    let mut parser = Parser {
        bytes,
        at: 0,
        remaining: values,
        max_depth: depth.min(128),
    };
    let value = parser.value(0)?;
    parser.whitespace();
    if parser.at != bytes.len() {
        return Err(parser.error("trailing JSON input"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    remaining: usize,
    max_depth: usize,
}

impl Parser<'_> {
    fn error(&self, message: impl std::fmt::Display) -> serde_json::Error {
        serde_json::Error::custom(format!("{message} at byte {}", self.at))
    }

    fn whitespace(&mut self) {
        while matches!(self.bytes.get(self.at), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.at += 1;
        }
    }

    fn take(&mut self, byte: u8) -> bool {
        self.whitespace();
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn scalar<T: DeserializeOwned>(&mut self) -> Result<T, serde_json::Error> {
        let mut stream =
            serde_json::Deserializer::from_slice(&self.bytes[self.at..]).into_iter::<T>();
        let value = stream
            .next()
            .ok_or_else(|| self.error("expected JSON scalar"))??;
        self.at += stream.byte_offset();
        Ok(value)
    }

    fn value(&mut self, depth: usize) -> Result<Value, serde_json::Error> {
        if depth > self.max_depth {
            return Err(self.error(format!(
                "JSON nesting exceeds depth limit {}",
                self.max_depth
            )));
        }
        if self.remaining == 0 {
            return Err(self.error("JSON value count exceeds configured limit"));
        }
        self.remaining -= 1;
        self.whitespace();
        match self.bytes.get(self.at) {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.scalar().map(Value::String),
            Some(b'-' | b'0'..=b'9') => self.scalar::<Number>().map(Value::Number),
            Some(b't' | b'f') => self.scalar().map(Value::Bool),
            Some(b'n') => self.scalar::<()>().map(|()| Value::Null),
            _ => Err(self.error("expected JSON value")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, serde_json::Error> {
        self.at += 1;
        let mut values = Map::new();
        if self.take(b'}') {
            return Ok(Value::Object(values));
        }
        loop {
            self.whitespace();
            let key: String = self.scalar()?;
            if values.contains_key(&key) {
                return Err(self.error(format!("duplicate JSON object member `{key}`")));
            }
            if !self.take(b':') {
                return Err(self.error("expected colon after JSON member name"));
            }
            values.insert(key, self.value(depth + 1)?);
            if self.take(b'}') {
                return Ok(Value::Object(values));
            }
            if !self.take(b',') {
                return Err(self.error("expected comma or closing JSON object"));
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, serde_json::Error> {
        self.at += 1;
        let mut values = Vec::new();
        if self.take(b']') {
            return Ok(Value::Array(values));
        }
        loop {
            values.push(self.value(depth + 1)?);
            if self.take(b']') {
                return Ok(Value::Array(values));
            }
            if !self.take(b',') {
                return Err(self.error("expected comma or closing JSON array"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn exact_numbers_are_single_values_and_marker_keys_stay_objects() {
        for lexical in ["0.25", "18446744073709551616", "1e+400", "-0.00001"] {
            let value = parse(lexical.as_bytes(), 1, 0).expect("one number");
            assert!(value.is_number(), "{lexical}");
            assert_eq!(value.to_string(), lexical);
        }
        let value =
            parse(br#"{"$serde_json::private::Number":"123"}"#, 2, 1).expect("ordinary object");
        assert_eq!(value["$serde_json::private::Number"].as_str(), Some("123"));
    }

    #[test]
    fn framing_limits_and_decoded_duplicate_names_are_enforced() {
        for bytes in [
            &b"[1,]"[..],
            &b"{\"a\":1,}"[..],
            &b"01"[..],
            &b"true false"[..],
            &b"[1 2]"[..],
            &b"{\"a\" 1}"[..],
            &b"\x0b1"[..],
            &br#"{"a":1,"\u0061":2}"#[..],
        ] {
            assert!(parse(bytes, 10, 8).is_err(), "{bytes:?}");
        }
        assert!(parse(b"[1,2]", 2, 8).is_err());
        assert!(parse(b"[[0]]", 3, 1).is_err());
        assert!(parse(b"[1,2]", 3, 1).is_ok());
        let deep = format!("{}0{}", "[".repeat(130), "]".repeat(130));
        assert!(parse(deep.as_bytes(), 1_000, usize::MAX).is_err());
    }
}
