// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Duplicate-free, bounded JSON decoding shared by JSON-LD contexts and options.

use std::cell::Cell;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::RdfDiagnostic;

#[derive(Debug, Clone, Copy)]
pub(super) struct StrictJsonLimits {
    pub(super) bytes: usize,
    pub(super) depth: usize,
    pub(super) values: usize,
}

pub(super) fn parse_strict_json(
    bytes: &[u8],
    limits: StrictJsonLimits,
    description: &str,
) -> Result<Value, RdfDiagnostic> {
    if bytes.len() > limits.bytes {
        return Err(error(format!(
            "{description} is {} bytes; limit is {}",
            bytes.len(),
            limits.bytes
        )));
    }
    let remaining = Cell::new(limits.values);
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictJsonSeed {
        remaining: &remaining,
        depth: 0,
        max_depth: limits.depth,
    }
    .deserialize(&mut deserializer)
    .map_err(|source| error(format!("parse {description}: {source}")))?;
    deserializer
        .end()
        .map_err(|source| error(format!("parse {description}: {source}")))?;
    Ok(value)
}

#[derive(Clone, Copy)]
struct StrictJsonSeed<'a> {
    remaining: &'a Cell<usize>,
    depth: usize,
    max_depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictJsonSeed<'_> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.depth > self.max_depth {
            return Err(D::Error::custom(format!(
                "JSON nesting exceeds depth limit {}",
                self.max_depth
            )));
        }
        let remaining = self.remaining.get();
        if remaining == 0 {
            return Err(D::Error::custom(
                "JSON value count exceeds configured limit",
            ));
        }
        self.remaining.set(remaining - 1);
        deserializer.deserialize_any(StrictJsonVisitor(self))
    }
}

struct StrictJsonVisitor<'a>(StrictJsonSeed<'a>);

impl<'de> Visitor<'de> for StrictJsonVisitor<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a duplicate-free JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self.0.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let child = StrictJsonSeed {
            depth: self.0.depth + 1,
            ..self.0
        };
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(1_024));
        while let Some(value) = sequence.next_element_seed(child)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let child = StrictJsonSeed {
            depth: self.0.depth + 1,
            ..self.0
        };
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom(format!(
                    "duplicate JSON object member `{key}`"
                )));
            }
            values.insert(key, map.next_value_seed(child)?);
        }
        Ok(Value::Object(values))
    }
}

fn error(message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("jsonld-json-input", message)
}
