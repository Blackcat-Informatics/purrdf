// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Length-prefixed ("framed") byte fields, for digests and encodings that
//! combine several variable-length fields.
//!
//! A frame is the field's length as a `u64` in little-endian byte order,
//! followed by the field's bytes. Framing every variable-length field before
//! it is hashed means no concatenation of fields can be read as a different
//! split of the same bytes: `("ab", "c")` and `("a", "bc")` frame
//! differently, so they hash differently.
//!
//! ```
//! use purrdf_hash::blake3::{self, RecordHasher};
//! use purrdf_hash::frame::{frame_le, frame_le_into, Framed};
//!
//! let mut bytes = Vec::new();
//! frame_le(&mut bytes, b"abc");
//! assert_eq!(bytes, [3, 0, 0, 0, 0, 0, 0, 0, b'a', b'b', b'c']);
//!
//! // The same frame absorbed straight into a hasher, no buffer in between.
//! let mut hasher = RecordHasher::new();
//! frame_le_into(&mut hasher, b"abc");
//! assert_eq!(hasher.finalize(), blake3::hash(&bytes));
//!
//! // Or field by field through a framing wrapper.
//! let mut identity = Framed(RecordHasher::new());
//! identity.field(b"abc");
//! assert_eq!(identity.finish().finalize(), blake3::hash(&bytes));
//! ```
//!
//! # The framing law
//!
//! The prefix is **eight bytes, little-endian, always**. Published digests
//! — proof and derivation identities, cache keys, witness digests — are
//! computed over framed fields, so this encoding is part of their definition
//! and must never change: not the width, not the byte order. No big-endian
//! variant is offered, and none may be added, because two framings would let
//! two identities exist for one value.

use crate::Digest;

/// Something bytes can be absorbed into in order: a growing buffer or a
/// streaming hasher.
///
/// `Vec<u8>` appends; every hasher in this crate (through [`Digest`], so
/// `dyn Digest` too) updates its state.
pub trait FrameSink {
    /// Absorb `bytes` after everything absorbed before.
    fn absorb(&mut self, bytes: &[u8]);
}

impl FrameSink for Vec<u8> {
    #[inline]
    fn absorb(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}

impl<D: Digest + ?Sized> FrameSink for D {
    #[inline]
    fn absorb(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }
}

/// Appends the frame of `bytes` — its length as eight little-endian bytes,
/// then the bytes — to `out`.
#[inline]
pub fn frame_le(out: &mut Vec<u8>, bytes: &[u8]) {
    frame_le_into(out, bytes);
}

/// Absorbs the frame of `bytes` — its length as eight little-endian bytes,
/// then the bytes — into `sink`.
///
/// Into a `Vec<u8>` this is [`frame_le`]; into a hasher it is the digest of
/// the same bytes without materializing them.
#[inline]
pub fn frame_le_into<W: FrameSink + ?Sized>(sink: &mut W, bytes: &[u8]) {
    sink.absorb(&(bytes.len() as u64).to_le_bytes());
    sink.absorb(bytes);
}

/// A sink that frames every field written to it.
///
/// Wrap a `Vec<u8>` to build framed bytes, or a hasher to compute the digest
/// of framed fields directly; [`finish`](Self::finish) hands the sink back
/// (the bytes, or the hasher to `finalize`).
#[derive(Clone, Debug, Default)]
pub struct Framed<D: FrameSink>(pub D);

impl<D: FrameSink> Framed<D> {
    /// Frames into `sink`.
    #[inline]
    pub const fn new(sink: D) -> Self {
        Self(sink)
    }

    /// Absorbs one framed field.
    #[inline]
    pub fn field(&mut self, bytes: &[u8]) -> &mut Self {
        frame_le_into(&mut self.0, bytes);
        self
    }

    /// The sink with every field framed into it.
    #[inline]
    pub fn finish(self) -> D {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameSink, Framed, frame_le, frame_le_into};
    use crate::blake3::{self, Hasher, RecordHasher};
    use crate::crc32::Crc32;
    use crate::md5::Md5;
    use crate::sha1::Sha1;
    use crate::sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};
    use crate::{Digest, MAX_OUTPUT_LEN};

    #[test]
    fn frame_le_is_the_little_endian_length_then_the_bytes() {
        let mut out = Vec::new();
        frame_le(&mut out, b"");
        assert_eq!(out, [0; 8]);
        frame_le(&mut out, b"abc");
        assert_eq!(
            out,
            [
                0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, b'a', b'b', b'c'
            ]
        );
        let mut long = Vec::new();
        frame_le(&mut long, &[0xaa; 0x0102]);
        assert_eq!(&long[..8], [0x02, 0x01, 0, 0, 0, 0, 0, 0]);
        assert_eq!(long.len(), 8 + 0x0102);
        assert!(long[8..].iter().all(|&b| b == 0xaa));
    }

    /// Frames the fields into fresh bytes.
    fn framed(fields: &[&[u8]]) -> Vec<u8> {
        let mut framed = Framed::new(Vec::new());
        for field in fields {
            framed.field(field);
        }
        framed.finish()
    }

    #[test]
    fn framed_fields_distinguish_every_split() {
        let ab_c = framed(&[b"ab", b"c"]);
        let a_bc = framed(&[b"a", b"bc"]);
        assert_eq!(
            ab_c,
            [
                2, 0, 0, 0, 0, 0, 0, 0, b'a', b'b', 1, 0, 0, 0, 0, 0, 0, 0, b'c'
            ]
        );
        assert_eq!(
            a_bc,
            [
                1, 0, 0, 0, 0, 0, 0, 0, b'a', 2, 0, 0, 0, 0, 0, 0, 0, b'b', b'c'
            ]
        );
        assert_ne!(ab_c, a_bc);
        let abc = framed(&[b"abc"]);
        let abc_empty = framed(&[b"abc", b""]);
        assert_eq!(abc.len() + 8, abc_empty.len());
        assert_ne!(abc, abc_empty, "an empty trailing field is still a field");
        // Chained calls on one wrapper frame the same bytes.
        let mut chained = Framed::<Vec<u8>>::default();
        chained.field(b"ab").field(b"c");
        assert_eq!(chained.finish(), ab_c);
        assert_ne!(
            blake3::hash(&ab_c),
            blake3::hash(&a_bc),
            "the digests differ because the frames differ"
        );
    }

    /// Framing into a hasher is the digest of the framed bytes, for every
    /// hasher in the crate, on any split of the fields.
    #[test]
    fn every_hasher_absorbs_frames_like_the_byte_buffer() {
        let fields: [&[u8]; 4] = [b"", b"a", b"bc", &[0u8; 1500]];
        let mut bytes = Vec::new();
        for field in fields {
            frame_le(&mut bytes, field);
        }

        fn through<D: Digest + Default>(fields: &[&[u8]]) -> Vec<u8> {
            let mut framed = Framed::new(D::default());
            for field in fields {
                framed.field(field);
            }
            let mut hasher = framed.finish();
            let mut out = [0u8; MAX_OUTPUT_LEN];
            let len = hasher.finalize_reset(&mut out);
            out[..len].to_vec()
        }
        assert_eq!(through::<Md5>(&fields), Md5::digest(&bytes));
        assert_eq!(through::<Sha1>(&fields), Sha1::digest(&bytes));
        assert_eq!(through::<Sha3_224>(&fields), Sha3_224::digest(&bytes));
        assert_eq!(through::<Sha3_256>(&fields), Sha3_256::digest(&bytes));
        assert_eq!(through::<Sha3_384>(&fields), Sha3_384::digest(&bytes));
        assert_eq!(through::<Sha3_512>(&fields), Sha3_512::digest(&bytes));
        assert_eq!(
            through::<Crc32>(&fields),
            Crc32::checksum(&bytes).to_be_bytes()
        );
        assert_eq!(through::<Hasher>(&fields), blake3::hash(&bytes).as_bytes());
        assert_eq!(
            through::<RecordHasher>(&fields),
            blake3::hash(&bytes).as_bytes()
        );

        // The free function and the wrapper agree, and so does `dyn Digest`.
        let mut direct = RecordHasher::new();
        for field in fields {
            frame_le_into(&mut direct, field);
        }
        assert_eq!(direct.finalize(), blake3::hash(&bytes));
        let mut erased = Sha1::new();
        let sink: &mut dyn Digest = &mut erased;
        for field in fields {
            frame_le_into(sink, field);
        }
        let mut out = [0u8; MAX_OUTPUT_LEN];
        let len = sink.finalize_reset(&mut out);
        assert_eq!(out[..len], Sha1::digest(&bytes));
    }

    #[test]
    fn vec_sink_appends_and_finish_returns_it() {
        let mut sink = vec![0xff];
        sink.absorb(b"xy");
        assert_eq!(sink, [0xff, b'x', b'y']);
        let framed = Framed::<Vec<u8>>::default();
        assert_eq!(framed.0, Vec::<u8>::new());
        let mut framed = Framed(sink);
        framed.field(b"z");
        assert_eq!(
            framed.finish(),
            [0xff, b'x', b'y', 1, 0, 0, 0, 0, 0, 0, 0, b'z']
        );
    }
}
