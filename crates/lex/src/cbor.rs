// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one CBOR codec (RFC 8949).
//!
//! Every CBOR byte PurRDF writes is spelled here, and every CBOR item it reads
//! is decoded here: the data-item [`head`] (major type and argument), the
//! [`Value`] tree, the encoders, and the decoders. The GTS container's content
//! identifiers are BLAKE3 digests of [`canonical`] bytes, so the encoding is a
//! published identity: one encoder is the only way to keep every writer's
//! bytes the same.
//!
//! # Contract
//!
//! * **Heads are always shortest.** Every encoder writes an argument in the
//!   fewest bytes that hold it (RFC 8949 §4.2.1): immediate below 24, then one,
//!   two, four or eight bytes. [`head::push_head`] and [`head::write_head`] are
//!   that rule for a caller streaming its own items.
//! * **Definite lengths only on output.** No encoder writes an indefinite
//!   length.
//! * **Floats are the shortest that preserve the value.** A float is written as
//!   binary16 when that is exact, else binary32 when that is exact, else
//!   binary64, comparing bit patterns, so `-0.0` stays negative and a NaN keeps
//!   its payload.
//! * **[`encode`] keeps map order; [`canonical`] sorts it.** [`encode`] writes
//!   map entries as the value holds them. [`canonical`] is RFC 8949 §4.2.1 core
//!   deterministic encoding: every map's entries in the bytewise order of their
//!   keys' own deterministic encodings (a stable sort, so a repeated key keeps
//!   its relative order). [`write_canonical_map`] is the same for a map whose
//!   named text keys are left out, streamed to any sink (a hasher), which is how
//!   a content identifier excludes its own `id`.
//! * **Integers span `-2^64 ..= 2^64 - 1`**, the range major types 0 and 1
//!   carry ([`Integer`]); no encoder writes a bignum tag.
//! * **Decoding is total and bounded.** [`decode`] reads any well-formed item
//!   (§4.1, indefinite lengths included) and refuses every malformed one with a
//!   typed [`DecodeError`] at a byte offset; [`decode_deterministic`] also
//!   refuses anything [`canonical`] would not have written.
//!   [`Limits::max_depth`] bounds nesting, and neither the decoders nor the
//!   encoders nor a [`Value`]'s drop, clone, comparison or `Debug` spend a
//!   machine-stack frame per level.
//! * **Nothing is lost on the way in.** A tag is kept as [`Value::Tag`] (tags 2
//!   and 3 included), and a simple value other than `false`, `true` and `null`
//!   as [`Value::Simple`] (`undefined` included).
//!
//! ```rust
//! use purrdf_lex::cbor::{self, Limits, Value};
//!
//! let map = Value::Map(vec![
//!     (Value::from("id"), Value::from(1_u8)),
//!     (Value::from("d"), Value::Bytes(vec![0xaa])),
//! ]);
//! assert_eq!(cbor::encode(&map), [0xa2, 0x62, b'i', b'd', 0x01, 0x61, b'd', 0x41, 0xaa]);
//! assert_eq!(cbor::canonical(&map), [0xa2, 0x61, b'd', 0x41, 0xaa, 0x62, b'i', b'd', 0x01]);
//! assert_eq!(cbor::decode(&cbor::encode(&map), Limits::DEFAULT).unwrap(), map);
//! ```

mod decode;
mod encode;
pub mod head;
mod value;

pub use decode::{
    DecodeError, DecodeErrorKind, Limits, decode, decode_deterministic, decode_prefix,
    decode_sequence, read_from,
};
pub use encode::{
    canonical, canonical_into, encode, encode_into, encoded_len, write, write_canonical,
    write_canonical_map,
};
pub use value::{Integer, IntegerRangeError, Value, map_get};

#[cfg(test)]
mod tests;
