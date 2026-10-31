// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Wire primitives: deterministic CBOR, BLAKE3 content-ids, and the id/prev
//! rule — mirror of `src/purrdf_tools/gts/wire.py`.
//!
//! A frame's `"id"` is BLAKE3-256 of the deterministic CBOR (RFC 8949 §4.2)
//! of its content — every key except `"id"` and `"sig"` (§6, §9.1). The
//! Header is hashed the same way, excluding only `"id"` (§5). `"prev"` names
//! the previous item's `"id"`; the first frame's `"prev"` is the Header's.

use purrdf_lex::cbor::{Limits, Value};

pub use purrdf_lex::cbor::{
    canonical, canonical_into as append_canonical, encode, encode_into, map_get,
};

/// CBOR self-describe tag (RFC 8949 §3.4.6); MAY prefix the Header item (§3).
pub const SELF_DESCRIBE_TAG: u64 = 55799;

/// Header magic string (`"GTS1"`) identifying a GTS file (§5).
pub const MAGIC: &str = "GTS1";
/// Wire-format major version, encoded in the header `"v"` field (§5).
pub const VERSION: u8 = 1;

/// A copy of `v` with every map ordered per RFC 8949 §4.2.1 (bytewise on
/// encoded keys), as [`Value::canonicalize`] orders it in place; [`encode`] of
/// the copy is [`canonical`] of `v`.
pub fn deterministic(v: &Value) -> Value {
    let mut ordered = v.clone();
    ordered.canonicalize();
    ordered
}

/// Input size at which native BLAKE3 subtrees use the shared Rayon pool.
const PARALLEL_HASH_MIN: usize = 128 * 1024;

struct RayonJoin;
impl purrdf_hash::blake3::Join for RayonJoin {
    fn join<A: Send, B: Send>(
        &self,
        left: impl FnOnce() -> A + Send,
        right: impl FnOnce() -> B + Send,
    ) -> (A, B) {
        rayon::join(left, right)
    }
}

/// The 32-byte BLAKE3-256 digest of `data`, without a digest allocation.
pub fn blake3_256(data: &[u8]) -> [u8; 32] {
    if data.len() < PARALLEL_HASH_MIN {
        return *purrdf_hash::blake3::hash(data).as_bytes();
    }
    let workers = rayon::current_num_threads();
    if workers == 1 {
        return *purrdf_hash::blake3::hash(data).as_bytes();
    }
    // A split threshold is twice its power-of-two leaf size. Aim to occupy
    // the caller's pool, with a 16 KiB minimum leaf to fill the widest SIMD
    // batch and a bounded grain for load balancing on large inputs. The paired
    // eight-worker measurements favor 32 KiB at 128 KiB, 256 KiB at 1 MiB,
    // and 1 MiB at 16 MiB; a fixed threshold leaves small trees underused.
    let grain = data
        .len()
        .div_ceil(workers)
        .next_power_of_two()
        .saturating_mul(2)
        .clamp(32 * 1024, 1024 * 1024);
    *purrdf_hash::blake3::hash_with_join(data, grain, &RayonJoin).as_bytes()
}

/// A `blake3:<hex>` content digest for inline blob addressing (§12): the
/// BLAKE3-256 of `data`, as [`digest_label`] spells it.
pub fn digest_str(data: &[u8]) -> String {
    digest_label(&blake3_256(data))
}

/// The `blake3:<hex>` spelling of a BLAKE3 digest already computed: the
/// scheme, then the digest's lowercase base16. The one place a GTS content
/// identifier is spelt.
pub fn digest_label(digest: &[u8]) -> String {
    format!("blake3:{}", purrdf_hash::hex::Lower(digest))
}

fn hash_map<E>(
    write: impl FnOnce(&mut purrdf_hash::blake3::Hasher) -> Result<(), E>,
) -> Result<Vec<u8>, E> {
    let mut writer = purrdf_hash::blake3::Hasher::new();
    write(&mut writer)?;
    Ok(writer.finalize().as_bytes().to_vec())
}
fn hash_excluding(entries: &[(Value, Value)], excluded: &[&str]) -> Vec<u8> {
    hash_map(|writer| purrdf_lex::cbor::write_canonical_map(entries, excluded, writer))
        .expect("hash writer cannot fail")
}
pub(crate) fn content_id_encoded(
    frame: purrdf_lex::cbor::BorrowedItem<'_>,
) -> Result<Vec<u8>, purrdf_lex::cbor::WriteError> {
    hash_map(|writer| purrdf_lex::cbor::write_canonical_borrowed_map(frame, &["id", "sig"], writer))
}

/// Compute a frame's `"id"` over its content (excluding `"id"`/`"sig"`).
pub fn content_id(frame: &[(Value, Value)]) -> Vec<u8> {
    hash_excluding(frame, &["id", "sig"])
}

/// Compute the Header's genesis `"id"` (excluding only `"id"`) — §5.
pub fn header_id(header: &[(Value, Value)]) -> Vec<u8> {
    hash_excluding(header, &["id"])
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
    purrdf_lex::cbor::decode_sequence(data, Limits::DEFAULT)
}

/// Original decode failure at a GTS sequence item boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TornAppendError {
    /// Absolute byte offset where the refused item starts.
    pub offset: u64,
    /// Original decoder failure, with its item-relative location and I/O owner.
    pub cause: purrdf_lex::cbor::DecodeError,
}

impl core::fmt::Display for TornAppendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "torn at offset {}", self.offset)
    }
}

purrdf_lex::error_source!(TornAppendError, cause);

impl purrdf_hash::error::StructuredError for TornAppendError {
    purrdf_hash::error_visit! {
        @record self { offset, cause } => "gts-torn-append" {
            "offset" => Unsigned(u128::from(self.offset)), "cause" => Cause(&self.cause)
        }
    }
}

/// Decode through the same CBOR parser while retaining the original refusal.
pub fn iter_items_with_error(data: &[u8]) -> (Vec<(usize, Value)>, Option<TornAppendError>) {
    let decoded = purrdf_lex::cbor::decode_sequence_checked(data, Limits::DEFAULT);
    let failure = decoded.failure.map(|failure| TornAppendError {
        offset: failure.offset as u64,
        cause: failure.cause,
    });
    (decoded.items, failure)
}

/// Return the Header map, unwrapping the optional self-describe tag (§3).
pub fn unwrap_header(item: &Value) -> Result<&Vec<(Value, Value)>, HeaderError> {
    let inner = match item {
        Value::Tag(tag, inner) => {
            if *tag != SELF_DESCRIBE_TAG {
                return Err(HeaderError::Tag { actual: *tag });
            }
            inner.as_ref()
        }
        other => other,
    };
    match inner {
        Value::Map(entries) => Ok(entries),
        other => Err(HeaderError::NotMap(other.clone())),
    }
}

/// Consume a header after the shared interpretation check, retaining its map.
/// This preserves the original native refusal while avoiding a successful copy.
pub(crate) fn into_header(mut item: Value) -> Result<Vec<(Value, Value)>, HeaderError> {
    unwrap_header(&item)?;
    let inner = match &mut item {
        Value::Tag(_, inner) => inner.as_mut(),
        other => other,
    };
    let Value::Map(entries) = inner else {
        unreachable!("the shared header interpretation validated this map");
    };
    Ok(std::mem::take(entries))
}

/// A native refusal to interpret an item as a header map.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum HeaderError {
    /// A tag other than the optional self-describe tag.
    Tag {
        /// Original tag number.
        actual: u64,
    },
    /// The unwrapped original value is not a map.
    NotMap(Value),
}

impl core::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Tag { actual } => write!(f, "unexpected CBOR tag {actual} on the header item"),
            Self::NotMap(_) => f.write_str("header item is not a CBOR map"),
        }
    }
}

impl std::error::Error for HeaderError {}

impl purrdf_hash::error::StructuredError for HeaderError {
    purrdf_hash::error_visit! {
        Tag { actual } => "gts-header-tag" {
            "actual" => Unsigned(u128::from(*actual)),
            "expected" => Unsigned(u128::from(SELF_DESCRIBE_TAG))
        },
        NotMap(value) => "gts-header-not-map" { "value" => Record(value) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_hash_matches_frozen_answers() {
        // Frozen blake3 1.8.5 public-API answers; input byte i is i % 251.
        // Both sides of the scheduling threshold and a deeper tree are covered.
        let data: Vec<u8> = (0..1_048_576).map(|i| (i % 251) as u8).collect();
        let verify = || {
            for (len, expected) in [
                (
                    131_071,
                    "de433db299ce5940eb72f08f509f90fa93e8b8c38e26c927310b4f8b98e2f33c",
                ),
                (
                    131_072,
                    "306baba93b1a393cbd35172837c98b0f59a41f64e1b2682ae102d8b2534b9e1c",
                ),
                (
                    131_073,
                    "f837d4254d24ba3d50fe3743d46e4af6db5f5d6ab0469197d94e7ba1e906c4d8",
                ),
                (
                    1_048_576,
                    "74cb441fd087764ca9c3694da742ebe30cbeb3060a17009ca81825c7a8d10343",
                ),
            ] {
                assert_eq!(
                    purrdf_hash::hex::Lower(&blake3_256(&data[..len])).to_string(),
                    expected
                );
            }
        };
        verify();
        #[cfg(not(target_arch = "wasm32"))]
        for workers in [1, 2, 8] {
            rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .expect("test pool")
                .install(verify);
        }
    }

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
