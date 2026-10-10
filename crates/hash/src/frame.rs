// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Length framing: the one way a variable-length field enters a preimage or a
//! wire encoding.
//!
//! A framed field is the field's byte length as eight little-endian bytes,
//! followed by the field's bytes:
//!
//! ```text
//! frame(field) = u64_le(len(field)) ‖ field
//! ```
//!
//! A sequence of framed fields is injective: no two different field sequences
//! concatenate to the same bytes, whatever bytes the fields hold. Text in RDF is
//! arbitrary UTF-8 — a lexical form may contain NUL, a label may contain any
//! separator a caller could pick — so no byte value is free to serve as a
//! delimiter, and a fixed-width count is the only framing that is oblivious to
//! the payload. The width is fixed at eight bytes and the order at little-endian
//! so that the encoding is byte-identical on every target, the 32-bit
//! `wasm32-unknown-unknown` build included, where `usize` is narrower.
//!
//! [`frame_le`] appends a framed field to a byte buffer; [`frame_le_into`]
//! absorbs one into a streaming [`Digest`], giving the digest of exactly the
//! bytes [`frame_le`] would have appended, without materialising them.
//!
//! [`frame_be_labelled`] is the one other framing a published identity uses: a
//! label and a value, each behind its length as eight **big-endian** bytes. It
//! is the preimage part of the SPARQL registry identities, the JSON Schema
//! compilation key and the HNSW space generation, all of which were published
//! in that byte order; it is not a second spelling of [`frame_le`], and a new
//! construction frames with [`frame_le`].
//!
//! These bytes are part of published identities — proof encodings, contract and
//! plan hashes, content-addressed node IRIs, artifact identities — so the
//! construction is frozen: `purrdf-hash-conformance`'s
//! `tests/vectors/frame_le_vectors.txt` holds its answers over a structured
//! corpus, and every target replays them.

use crate::Digest;

/// The length prefix of a `len`-byte field: `len` as eight little-endian bytes.
///
/// `usize` is at most 64 bits wide on every target Rust supports, so the
/// widening never truncates.
#[inline]
const fn prefix(len: usize) -> [u8; 8] {
    (len as u64).to_le_bytes()
}

/// Append `bytes` to `out` as one framed field: its length as eight
/// little-endian bytes, then the bytes themselves.
///
/// ```
/// let mut out = Vec::new();
/// purrdf_hash::frame::frame_le(&mut out, b"ab");
/// purrdf_hash::frame::frame_le(&mut out, b"");
/// assert_eq!(out, b"\x02\0\0\0\0\0\0\0ab\0\0\0\0\0\0\0\0");
/// ```
#[inline]
pub fn frame_le(out: &mut Vec<u8>, bytes: &[u8]) {
    out.reserve(8 + bytes.len());
    out.extend_from_slice(&prefix(bytes.len()));
    out.extend_from_slice(bytes);
}

/// The checked exact destination layout of one published labelled frame.
/// Returns `None` if the combined frame cannot fit in addressable storage.
#[must_use]
pub fn frame_be_labelled_len(label: &str, value: &[u8]) -> Option<usize> {
    size_of::<u64>()
        .checked_add(label.len())?
        .checked_add(size_of::<u64>())?
        .checked_add(value.len())
}

/// Append `label` and then `value` to `out`, each as its length in eight
/// big-endian bytes followed by its bytes:
/// `u64_be(len(label)) ‖ label ‖ u64_be(len(value)) ‖ value`.
///
/// The labelled part of the SPARQL registry identities, the JSON Schema
/// compilation key and the HNSW space generation — see the
/// [module documentation](self). Their published values fix the byte order.
///
/// ```
/// let mut out = Vec::new();
/// purrdf_hash::frame::frame_be_labelled(&mut out, "k", b"vv");
/// assert_eq!(out, b"\0\0\0\0\0\0\0\x01k\0\0\0\0\0\0\0\x02vv");
/// ```
#[inline]
pub fn frame_be_labelled(out: &mut Vec<u8>, label: &str, value: &[u8]) {
    out.reserve(frame_be_labelled_len(label, value).expect("labelled frame size overflow"));
    out.extend_from_slice(&(label.len() as u64).to_be_bytes());
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value);
}

/// Absorb `bytes` into `digest` as one framed field: exactly the bytes
/// [`frame_le`] appends, streamed, without materialising them.
///
/// A preimage absorbs its [`Domain`](crate::Domain) once, at its opening, in
/// whatever form its specification states: as its own first framed field
/// (`frame_le_into(digest, DOMAIN.as_bytes())`), or as raw leading bytes
/// (`digest.update(DOMAIN.as_bytes())`).
///
/// ```
/// use purrdf_hash::frame::{frame_le, frame_le_into};
/// use purrdf_hash::{Digest, Domain, sha1::Sha1};
///
/// const RECORD: Domain = Domain::new(b"purrdf-example/record/v1");
///
/// let mut streamed = Sha1::new();
/// frame_le_into(&mut streamed, RECORD.as_bytes());
/// frame_le_into(&mut streamed, b"payload");
///
/// let mut buffer = Vec::new();
/// frame_le(&mut buffer, RECORD.as_bytes());
/// frame_le(&mut buffer, b"payload");
/// assert_eq!(streamed.finalize(), Sha1::digest(&buffer));
/// ```
#[inline]
pub fn frame_le_into<D: Digest + ?Sized>(digest: &mut D, bytes: &[u8]) {
    digest.update(&prefix(bytes.len()));
    digest.update(bytes);
}

#[cfg(test)]
mod tests {
    use super::{frame_be_labelled, frame_le, frame_le_into};
    use crate::sha1::Sha1;
    use crate::{Digest, Domain};

    const RECORD: Domain = Domain::new(b"purrdf-example/record/v1");

    /// The prefix is the length as eight little-endian bytes, then the bytes.
    #[test]
    fn a_frame_is_the_little_endian_length_then_the_bytes() {
        let mut out = Vec::new();
        frame_le(&mut out, b"abc");
        assert_eq!(out, [3, 0, 0, 0, 0, 0, 0, 0, b'a', b'b', b'c']);
        let long = vec![0x5a; 0x1_0203];
        let mut out = Vec::new();
        frame_le(&mut out, &long);
        assert_eq!(&out[..8], &[0x03, 0x02, 0x01, 0, 0, 0, 0, 0]);
        assert_eq!(&out[8..], &long[..]);
    }

    /// An empty field is still framed: eight zero bytes.
    #[test]
    fn an_empty_field_is_eight_zero_bytes() {
        let mut out = vec![0xff];
        frame_le(&mut out, b"");
        assert_eq!(out, [0xff, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    /// Two splits of the same bytes frame differently.
    #[test]
    fn framing_separates_every_split_of_the_same_bytes() {
        let mut left = Vec::new();
        frame_le(&mut left, b"ab");
        frame_le(&mut left, b"c");
        let mut right = Vec::new();
        frame_le(&mut right, b"a");
        frame_le(&mut right, b"bc");
        assert_ne!(left, right);
    }

    /// Streaming a frame into a digest is digesting the appended frame.
    #[test]
    fn a_streamed_frame_digests_the_appended_bytes() {
        let mut buffer = Vec::new();
        let mut streamed = Sha1::new();
        for field in [&b""[..], b"x", RECORD.as_bytes(), &[0u8; 300]] {
            frame_le(&mut buffer, field);
            frame_le_into(&mut streamed, field);
        }
        assert_eq!(streamed.finalize(), Sha1::digest(&buffer));
        let mut through_dyn = Sha1::new();
        frame_le_into(&mut through_dyn as &mut dyn Digest, b"x");
        let mut appended = Vec::new();
        frame_le(&mut appended, b"x");
        assert_eq!(through_dyn.finalize(), Sha1::digest(&appended));
    }

    /// The labelled framing is big-endian: its lengths read the other way
    /// round from the little-endian frame of the same bytes.
    #[test]
    fn the_labelled_framing_is_big_endian() {
        let mut out = Vec::new();
        frame_be_labelled(&mut out, "ab", &[0xff; 0x0102]);
        assert_eq!(&out[..8], &[0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(&out[8..10], b"ab");
        assert_eq!(&out[10..18], &[0, 0, 0, 0, 0, 0, 1, 2]);
        assert_eq!(out.len(), 18 + 0x0102);
        let mut empty = Vec::new();
        frame_be_labelled(&mut empty, "", b"");
        assert_eq!(empty, [0; 16]);
    }
}
