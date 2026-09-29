// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded container framing with typed serde scalar decoding. In particular,
//! arbitrary-precision numbers never pass through `deserialize_any`'s map
//! representation, so genuine object keys cannot impersonate a number.

use serde::de::{DeserializeOwned, Error as _};
use serde_json::{Map, Number, Value};

/// A structural resource limit or invalid JSON input.
#[derive(Debug)]
pub enum Error {
    /// A caller-selected bound or the parser's safety depth was exceeded.
    Limit(String),
    /// Invalid JSON syntax or a duplicate object member.
    Syntax(serde_json::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Limit(message) => formatter.write_str(message),
            Self::Syntax(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Syntax(error)
    }
}

/// Resource bounds for decoding one JSON value. Object keys are not value nodes.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum input bytes.
    pub bytes: usize,
    /// Maximum value nodes, including the root.
    pub values: usize,
    /// Maximum child depth; the root has depth zero. Also capped at 128.
    pub depth: usize,
    /// Maximum UTF-8 bytes in one decoded string or object key.
    pub string_bytes: usize,
}

/// Decode exact JSON numbers with bounded structure and duplicate-free objects.
pub fn parse(bytes: &[u8], limits: Limits) -> Result<Value, Error> {
    if bytes.len() > limits.bytes {
        return Err(Error::Limit("JSON input exceeds byte limit".to_owned()));
    }
    let mut parser = Parser {
        bytes,
        at: 0,
        remaining: limits.values,
        max_values: limits.values,
        max_depth: limits.depth.min(128),
        string_bytes: limits.string_bytes,
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
    max_values: usize,
    string_bytes: usize,
}

impl Parser<'_> {
    fn error(&self, message: impl std::fmt::Display) -> Error {
        Error::Syntax(serde_json::Error::custom(format!(
            "{message} at byte {}",
            self.at
        )))
    }

    fn limit(&self, message: impl std::fmt::Display) -> Error {
        Error::Limit(format!("{message} at byte {}", self.at))
    }

    /// Skip JSON `ws = *( %x20 / %x09 / %x0A / %x0D )`.
    fn whitespace(&mut self) {
        self.at = purrdf_iri::terminals::skip_ws(self.bytes, self.at);
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

    fn scalar<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let mut stream =
            serde_json::Deserializer::from_slice(&self.bytes[self.at..]).into_iter::<T>();
        let value = stream
            .next()
            .ok_or_else(|| self.error("expected JSON scalar"))??;
        self.at += stream.byte_offset();
        Ok(value)
    }

    fn string(&mut self) -> Result<String, Error> {
        let value: String = self.scalar()?;
        if value.len() > self.string_bytes {
            return Err(self.limit(format!(
                "JSON contains a {}-byte string; limit is {}",
                value.len(),
                self.string_bytes
            )));
        }
        Ok(value)
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > self.max_depth {
            return Err(self.limit(format!(
                "JSON nesting limit {}: input exceeds depth limit",
                self.max_depth
            )));
        }
        if self.remaining == 0 {
            return Err(self.limit(format!(
                "JSON value limit exceeded: more than {} JSON nodes",
                self.max_values
            )));
        }
        self.remaining -= 1;
        self.whitespace();
        match self.bytes.get(self.at) {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Value::String),
            Some(b'-' | b'0'..=b'9') => self.scalar::<Number>().map(Value::Number),
            Some(b't' | b'f') => self.scalar().map(Value::Bool),
            Some(b'n') => self.scalar::<()>().map(|()| Value::Null),
            _ => Err(self.error("expected JSON value")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, Error> {
        self.at += 1;
        let mut values = Map::new();
        if self.take(b'}') {
            return Ok(Value::Object(values));
        }
        loop {
            self.whitespace();
            let key = self.string()?;
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

    fn array(&mut self, depth: usize) -> Result<Value, Error> {
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
    fn parse(bytes: &[u8], values: usize, depth: usize) -> Result<serde_json::Value, super::Error> {
        super::parse(
            bytes,
            super::Limits {
                bytes: usize::MAX,
                values,
                depth,
                string_bytes: usize::MAX,
            },
        )
    }

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

/// Serialize a JSON tree through primitive integer and binary64 serializer calls.
///
/// Integers in the i64/u64 domain remain integers; other numbers round to finite
/// binary64. This avoids arbitrary-precision serde wrapper objects in non-JSON
/// formats. A lossless carrier must compare the decoded result before accepting
/// output; this adapter deliberately names its numeric conversion.
#[derive(Debug)]
pub struct Binary64<'a>(pub &'a Value);

impl serde::Serialize for Binary64<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{Error as _, SerializeMap, SerializeSeq};
        match self.0 {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::String(value) => serializer.serialize_str(value),
            Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    serializer.serialize_i64(value)
                } else if let Some(value) = value.as_u64() {
                    serializer.serialize_u64(value)
                } else {
                    let _scope = purrdf_xsd::ieee::Binary64Scope::enter();
                    let value = value
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| S::Error::custom("JSON number exceeds finite binary64"))?;
                    serializer.serialize_f64(value)
                }
            }
            Value::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(&Self(value))?;
                }
                sequence.end()
            }
            Value::Object(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, &Self(value))?;
                }
                map.end()
            }
        }
    }
}
