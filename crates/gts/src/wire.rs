// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Wire primitives: deterministic CBOR, BLAKE3 content-ids, and the id/prev
//! rule — mirror of `src/purrdf_tools/gts/wire.py`.
//!
//! A frame's `"id"` is BLAKE3-256 of the deterministic CBOR (RFC 8949 §4.2)
//! of its content — every key except `"id"` and `"sig"` (§6, §9.1). The
//! Header is hashed the same way, excluding only `"id"` (§5). `"prev"` names
//! the previous item's `"id"`; the first frame's `"prev"` is the Header's.

use ciborium::value::Value;
use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq, SerializeTupleVariant, Serializer};

/// CBOR self-describe tag (RFC 8949 §3.4.6); MAY prefix the Header item (§3).
pub const SELF_DESCRIBE_TAG: u64 = 55799;

/// Header magic string (`"GTS1"`) identifying a GTS file (§5).
pub const MAGIC: &str = "GTS1";
/// Wire-format major version, encoded in the header `"v"` field (§5).
pub const VERSION: u8 = 1;

/// Encode a CBOR value as-is (definite lengths, shortest-form integers;
/// map entries in their current order).
pub fn encode(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::ser::into_writer(v, &mut out).expect("CBOR encoding to a Vec cannot fail");
    out
}

/// Append a CBOR value as-is to an existing buffer.
pub fn encode_into(v: &Value, out: &mut Vec<u8>) {
    ciborium::ser::into_writer(v, out).expect("CBOR encoding to a Vec cannot fail");
}

/// Recursively order map keys per RFC 8949 §4.2 (bytewise on encoded keys).
///
/// The spec mandates 8949 deterministic encoding, NOT a CBOR library's legacy
/// "canonical" (RFC 7049 length-first) mode — the two orderings diverge on
/// keys like `"x"` vs `"id"` (§4). Tags recurse into their value.
pub fn deterministic(v: &Value) -> Value {
    match v {
        Value::Tag(tag, inner) => Value::Tag(*tag, Box::new(deterministic(inner))),
        Value::Array(items) => Value::Array(items.iter().map(deterministic).collect()),
        Value::Map(entries) => {
            let mut keyed: Vec<(Vec<u8>, Value, Value)> = entries
                .iter()
                .map(|(k, val)| (encode(k), k.clone(), deterministic(val)))
                .collect();
            keyed.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Map(keyed.into_iter().map(|(_, k, val)| (k, val)).collect())
        }
        other => other.clone(),
    }
}

struct Canonical<'a>(&'a Value);

struct CanonicalMap<'a> {
    entries: &'a [(Value, Value)],
    excluded: &'a [&'a str],
}

fn sorted_entries<'a>(
    entries: &'a [(Value, Value)],
    excluded: &[&str],
) -> Vec<(Vec<u8>, &'a Value, &'a Value)> {
    let mut keyed: Vec<_> = entries
        .iter()
        .filter(|(key, _)| !matches!(key, Value::Text(text) if excluded.contains(&text.as_str())))
        .map(|(key, value)| (canonical(key), key, value))
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed
}

impl Serialize for Canonical<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Tag(tag, inner) => {
                let mut tagged =
                    serializer.serialize_tuple_variant("@@TAG@@", 0, "@@TAGGED@@", 2)?;
                tagged.serialize_field(tag)?;
                tagged.serialize_field(&Self(inner))?;
                tagged.end()
            }
            Value::Array(items) => {
                let mut sequence = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    sequence.serialize_element(&Self(item))?;
                }
                sequence.end()
            }
            Value::Map(entries) => CanonicalMap {
                entries,
                excluded: &[],
            }
            .serialize(serializer),
            // Delegate scalar spelling to ciborium so shortest integers/floats and
            // future scalar variants stay byte-identical to the library encoder.
            scalar => scalar.serialize(serializer),
        }
    }
}

impl Serialize for CanonicalMap<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entries = sorted_entries(self.entries, self.excluded);
        let mut map = serializer.serialize_map(Some(entries.len()))?;
        for (_, key, value) in entries {
            map.serialize_entry(&Canonical(key), &Canonical(value))?;
        }
        map.end()
    }
}

/// Encode an object as deterministic CBOR (RFC 8949 §4.2).
pub fn canonical(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::ser::into_writer(&Canonical(v), &mut out)
        .expect("CBOR encoding to a Vec cannot fail");
    out
}

/// Append deterministic CBOR to an existing buffer without allocating a
/// temporary encoded byte vector.
pub fn append_canonical(v: &Value, out: &mut Vec<u8>) {
    ciborium::ser::into_writer(&Canonical(v), out).expect("CBOR encoding to a Vec cannot fail");
}

/// Input size at which BLAKE3 switches to multi-threaded hashing.
///
/// Follows the blake3 crate's guidance: `update_rayon` only pays for its
/// fork-join overhead once the input spans enough chunks (~128 KiB). Below
/// the threshold the single-threaded fast path is used. Both paths compute
/// the same function, so every content id and blob digest is byte-identical
/// regardless of thread count (rayon runs inline-sequential on targets
/// without threads, e.g. wasm32).
const PARALLEL_HASH_MIN: usize = 128 * 1024;

/// The 32-byte BLAKE3-256 digest of `data`.
pub fn blake3_256(data: &[u8]) -> Vec<u8> {
    if data.len() >= PARALLEL_HASH_MIN {
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(data);
        return hasher.finalize().as_bytes().to_vec();
    }
    blake3::hash(data).as_bytes().to_vec()
}

/// Lowercase hex of a byte string.
pub fn hex(data: &[u8]) -> String {
    use std::fmt::Write as _;
    data.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// A `blake3:<hex>` content digest for inline blob addressing (§12).
pub fn digest_str(data: &[u8]) -> String {
    format!("blake3:{}", hex(&blake3_256(data)))
}

/// Get a map entry by text key (first match, like Python `dict.get`).
pub fn map_get<'a>(entries: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find(|(k, _)| matches!(k, Value::Text(t) if t == key))
        .map(|(_, v)| v)
}

fn hash_excluding(entries: &[(Value, Value)], excluded: &[&str]) -> Vec<u8> {
    struct HashWriter(blake3::Hasher);

    impl std::io::Write for HashWriter {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0.update(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut writer = HashWriter(blake3::Hasher::new());
    ciborium::ser::into_writer(&CanonicalMap { entries, excluded }, &mut writer)
        .expect("hash writer cannot fail");
    writer.0.finalize().as_bytes().to_vec()
}

/// Compute a frame's `"id"` over its content (excluding `"id"`/`"sig"`).
pub fn content_id(frame: &[(Value, Value)]) -> Vec<u8> {
    hash_excluding(frame, &["id", "sig"])
}

/// Compute the Header's genesis `"id"` (excluding only `"id"`) — §5.
pub fn header_id(header: &[(Value, Value)]) -> Vec<u8> {
    hash_excluding(header, &["id"])
}

struct Counting<'a> {
    data: &'a [u8],
    pos: usize,
}

impl std::io::Read for Counting<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min(self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// Decode a CBOR Sequence into `(byte_offset, item)` pairs plus a torn marker.
///
/// Detects a torn append (a partial trailing item) by position: at an item
/// boundary the offset is either end-of-data (clean end) or the start of a
/// complete item; a decode failure there is a torn append (§3). Survivors are
/// returned regardless, so a reader can fold the intact prefix. The second
/// element is `None` for a clean end, or the byte offset of the incomplete
/// trailing item.
pub fn iter_items(data: &[u8]) -> (Vec<(usize, Value)>, Option<usize>) {
    let mut reader = Counting { data, pos: 0 };
    let mut out = Vec::new();
    let mut torn = None;
    loop {
        let start = reader.pos;
        if start == data.len() {
            break;
        }
        match ciborium::de::from_reader::<Value, _>(&mut reader) {
            Ok(item) => out.push((start, item)),
            Err(_) => {
                // partial (EOF) or corrupt trailing item
                torn = Some(start);
                break;
            }
        }
    }
    (out, torn)
}

/// Return the Header map, unwrapping the optional self-describe tag (§3).
pub fn unwrap_header(item: &Value) -> Result<&Vec<(Value, Value)>, String> {
    let inner = match item {
        Value::Tag(tag, inner) => {
            if *tag != SELF_DESCRIBE_TAG {
                return Err(format!("unexpected CBOR tag {tag} on the header item"));
            }
            inner.as_ref()
        }
        other => other,
    };
    match inner {
        Value::Map(entries) => Ok(entries),
        _ => Err("header item is not a CBOR map".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nested_value() -> Value {
        Value::Tag(
            SELF_DESCRIBE_TAG,
            Box::new(Value::Map(vec![
                (
                    Value::Text("z".to_owned()),
                    Value::Array(vec![
                        Value::Integer(24.into()),
                        Value::Float(1.5),
                        Value::Bool(true),
                        Value::Null,
                    ]),
                ),
                (
                    Value::Text("a".to_owned()),
                    Value::Map(vec![
                        (
                            Value::Integer((-1).into()),
                            Value::Text("negative".to_owned()),
                        ),
                        (Value::Integer(1.into()), Value::Bytes(vec![0, 1, 2, 255])),
                    ]),
                ),
            ])),
        )
    }

    #[test]
    fn borrowed_canonical_encoder_is_byte_identical_to_recursive_oracle() {
        let value = nested_value();
        let expected = encode(&deterministic(&value));
        assert_eq!(canonical(&value), expected);

        let mut appended = vec![0xaa];
        append_canonical(&value, &mut appended);
        assert_eq!(&appended[1..], expected);
    }

    #[test]
    fn streaming_excluded_map_hash_matches_recursive_oracle() {
        let entries = vec![
            (Value::Text("sig".to_owned()), Value::Bytes(vec![9; 64])),
            (Value::Text("d".to_owned()), nested_value()),
            (Value::Text("id".to_owned()), Value::Bytes(vec![8; 32])),
            (
                Value::Text("t".to_owned()),
                Value::Text("snapshot".to_owned()),
            ),
        ];
        let legacy = |excluded: &[&str]| {
            let content: Vec<_> = entries
                .iter()
                .filter(|(key, _)| {
                    !matches!(key, Value::Text(text) if excluded.contains(&text.as_str()))
                })
                .cloned()
                .collect();
            blake3_256(&encode(&deterministic(&Value::Map(content))))
        };

        assert_eq!(content_id(&entries), legacy(&["id", "sig"]));
        assert_eq!(header_id(&entries), legacy(&["id"]));
    }
}
