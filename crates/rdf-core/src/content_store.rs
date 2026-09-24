// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Content-addressed blob store for the self-describing bundle (S3).
//!
//! [`ContentStore`] is the **one** place bytes live. The RDF IR, the
//! [`ArtifactRecord`](crate::bundle::ArtifactRecord), and every quad hold a
//! [`ContentDigest`] *reference* — never the payload bytes themselves
//! (blob-by-reference doctrine). A multi-gigabyte blob is addressed by its
//! 32-byte SHA-256 digest; only that fixed-size id flows through the dataset and
//! records, so no large payload is ever copied into a quad or record.
//!
//! The digest is the content id: `insert` hashes the bytes, and `load`-time
//! validation re-hashes every stored blob and **hard-fails** on any mismatch
//! (no silent repair).

use std::collections::HashMap;
use std::fmt;

use sha2::{Digest, Sha256};

/// Owned blob payload bytes. A thin alias so the by-reference doctrine reads
/// clearly at call sites: only the kernel's [`ContentStore`] ever owns a `Bytes`;
/// everything else holds a [`ContentDigest`].
pub type Bytes = Vec<u8>;

/// A content id: the SHA-256 digest of a blob's bytes.
///
/// This is the only thing that flows through the dataset and the artifact index
/// in place of the bytes. It is a fixed 32 bytes regardless of payload size, so a
/// multi-terabyte blob still costs one `ContentDigest` to reference.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentDigest([u8; 32]);

impl ContentDigest {
    /// Compute the digest of `bytes` directly (SHA-256).
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let out = hasher.finalize();
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&out);
        Self(buf)
    }

    /// Wrap 32 raw digest bytes as a `ContentDigest` WITHOUT re-hashing.
    ///
    /// For callers that have already computed a SHA-256 digest themselves (e.g. a
    /// multi-section content fold) and want to carry the result as a `ContentDigest`.
    /// Unlike [`of`](Self::of), this does NOT hash — it adopts `raw` verbatim.
    pub fn from_raw(raw: [u8; 32]) -> Self {
        Self(raw)
    }

    /// The 32 raw digest bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The lowercase-hex rendering of the digest (64 chars).
    pub fn to_hex(&self) -> String {
        crate::hex::lower(&self.0)
    }

    /// Parse a 64-char hex digest. Returns `None` on any malformed input
    /// (wrong length or non-hex characters).
    pub fn from_hex(hex: &str) -> Option<Self> {
        decode_hex_32(hex).map(Self)
    }
}

/// Decode a 64-char hex string into 32 raw bytes. Returns `None` on any
/// malformed input (wrong length or non-hex characters). Case-insensitive
/// (accepts both `0-9a-f` and `0-9A-F`).
///
/// Shared by [`ContentDigest::from_hex`] and
/// [`crate::content_id::Blake3ContentId::from_hex`] (through
/// [`decode_hex_32_lower`]) so the two content-id domains (SHA-256 vs BLAKE3)
/// do not duplicate the decode loop.
pub(crate) fn decode_hex_32(hex: &str) -> Option<[u8; 32]> {
    decode_hex_32_as::<true>(hex)
}

/// [`decode_hex_32`] for canonical lowercase hex: `0-9a-f` only, so an
/// uppercase digit is malformed like any other non-hex byte.
pub(crate) fn decode_hex_32_lower(hex: &str) -> Option<[u8; 32]> {
    decode_hex_32_as::<false>(hex)
}

/// Digits per chunk of the hex classification: one 128-bit vector of bytes.
const HEX_CHUNK: usize = 16;

/// The one decode behind [`decode_hex_32`] (`UPPER = true`) and
/// [`decode_hex_32_lower`] (`UPPER = false`).
///
/// Two fixed-length passes with no data-dependent branch inside either:
///
/// 1. The 64 digits are classified and valued sixteen at a time, by
///    comparisons only. `b - b'0' < 10` is a decimal digit and `b - b'a' < 6` a
///    lowercase letter (with `UPPER`, `b | 0x20` folds `A-F` onto `a-f` and
///    nothing else onto them), each test as a `0x00`/`0xFF` byte lane; the
///    nibble is selected by masking, and a byte that is neither sets its lane
///    of a sixteen-lane rejection accumulator, OR-ed across the chunks. The
///    accumulator is folded to one byte once, after the last chunk.
/// 2. Only when no byte was rejected, each pair of nibbles is packed into its
///    byte.
///
/// A byte at or above `0x80` is neither a digit nor a letter, so non-ASCII
/// input is rejected exactly as `char::to_digit(16)` rejected it.
#[inline]
fn decode_hex_32_as<const UPPER: bool>(hex: &str) -> Option<[u8; 32]> {
    let digits: &[u8; 64] = hex.as_bytes().try_into().ok()?;
    let (chunks, _) = digits.as_chunks::<HEX_CHUNK>();
    let mut nibbles = [[0_u8; HEX_CHUNK]; 64 / HEX_CHUNK];
    let mut rejected = [0_u8; HEX_CHUNK];
    for (values, chunk) in nibbles.iter_mut().zip(chunks) {
        for ((value, reject), &b) in values.iter_mut().zip(&mut rejected).zip(chunk) {
            let decimal = b.wrapping_sub(b'0');
            let letter = (if UPPER { b | 0x20 } else { b }).wrapping_sub(b'a');
            let is_decimal = u8::from(decimal < 10).wrapping_neg();
            let is_letter = u8::from(letter < 6).wrapping_neg();
            *value = (decimal & is_decimal) | (letter.wrapping_add(10) & is_letter);
            *reject |= !(is_decimal | is_letter);
        }
    }
    if rejected.iter().fold(0, |any, &lane| any | lane) != 0 {
        return None;
    }
    let mut buf = [0_u8; 32];
    let pairs = nibbles.as_flattened().as_chunks::<2>().0;
    for (byte, pair) in buf.iter_mut().zip(pairs) {
        *byte = (pair[0] << 4) | pair[1];
    }
    Some(buf)
}

impl fmt::Debug for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentDigest({})", self.to_hex())
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// An error raised while validating or accessing a content-addressed blob.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentStoreError {
    /// A stored blob's bytes do not hash to the id they are filed under. This is
    /// always a hard error — the store never silently re-files or repairs it.
    DigestMismatch {
        /// The id the bytes were filed under.
        stored: ContentDigest,
        /// The id the bytes actually hash to.
        actual: ContentDigest,
    },
}

impl fmt::Display for ContentStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DigestMismatch { stored, actual } => write!(
                f,
                "content digest mismatch: bytes filed under {stored} hash to {actual}"
            ),
        }
    }
}

impl std::error::Error for ContentStoreError {}

/// A content-addressed blob store: bytes keyed by their SHA-256 [`ContentDigest`].
///
/// Insertion is idempotent — equal bytes always yield the same id and are stored
/// once. The store is the single owner of blob payloads in an
/// [`RdfBundle`](crate::bundle::RdfBundle); the dataset and artifact index hold
/// only the digest reference.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentStore {
    blobs: HashMap<ContentDigest, Bytes>,
}

impl ContentStore {
    /// A fresh, empty store.
    pub fn new() -> Self {
        Self {
            blobs: HashMap::new(),
        }
    }

    /// Insert `bytes`, returning the content id they are addressed by.
    ///
    /// Idempotent: re-inserting equal bytes returns the same id without storing a
    /// second copy. The bytes are *moved* into the store — the only place a
    /// payload is ever owned.
    pub fn insert(&mut self, bytes: Bytes) -> ContentDigest {
        let id = ContentDigest::of(&bytes);
        self.blobs.entry(id).or_insert(bytes);
        id
    }

    /// Insert pre-digested bytes, hard-failing if the supplied digest does not
    /// match the bytes. Used by `load` to validate every blob it deserializes.
    ///
    /// # Errors
    ///
    /// [`ContentStoreError::DigestMismatch`] if `stored` != `SHA-256(bytes)`.
    pub fn insert_checked(
        &mut self,
        stored: ContentDigest,
        bytes: Bytes,
    ) -> Result<ContentDigest, ContentStoreError> {
        let actual = ContentDigest::of(&bytes);
        if actual != stored {
            return Err(ContentStoreError::DigestMismatch { stored, actual });
        }
        self.blobs.entry(stored).or_insert(bytes);
        Ok(stored)
    }

    /// Borrow the bytes for `digest`, or `None` if not present.
    pub fn get(&self, digest: &ContentDigest) -> Option<&Bytes> {
        self.blobs.get(digest)
    }

    /// True if `digest` is present.
    pub fn contains(&self, digest: &ContentDigest) -> bool {
        self.blobs.contains_key(digest)
    }

    /// The number of distinct blobs stored.
    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    /// True when the store holds no blobs.
    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }

    /// Iterate `(digest, bytes)` over every stored blob.
    pub fn iter(&self) -> impl Iterator<Item = (&ContentDigest, &Bytes)> {
        self.blobs.iter()
    }

    /// Re-hash every stored blob and hard-fail on the first mismatch. The bundle
    /// loader calls this so a corrupted store can never be observed as valid.
    ///
    /// # Errors
    ///
    /// [`ContentStoreError::DigestMismatch`] for the first blob whose bytes do
    /// not hash to their key.
    pub fn verify_all(&self) -> Result<(), ContentStoreError> {
        for (stored, bytes) in &self.blobs {
            let actual = ContentDigest::of(bytes);
            if &actual != stored {
                return Err(ContentStoreError::DigestMismatch {
                    stored: *stored,
                    actual,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_of_is_stable_and_hex_round_trips() {
        let d = ContentDigest::of(b"hello world");
        let hex = d.to_hex();
        assert_eq!(hex.len(), 64);
        assert_eq!(ContentDigest::from_hex(&hex), Some(d));
    }

    #[test]
    fn from_hex_rejects_bad_input() {
        assert_eq!(ContentDigest::from_hex("short"), None);
        assert_eq!(ContentDigest::from_hex(&"z".repeat(64)), None);
    }

    #[test]
    fn insert_is_idempotent_and_content_addressed() {
        let mut store = ContentStore::new();
        let id1 = store.insert(b"payload".to_vec());
        let id2 = store.insert(b"payload".to_vec());
        assert_eq!(id1, id2, "equal bytes -> equal id");
        assert_eq!(store.len(), 1, "stored once");
        assert_eq!(store.get(&id1).map(Vec::as_slice), Some(&b"payload"[..]));
    }

    #[test]
    fn insert_checked_accepts_matching_digest() {
        let mut store = ContentStore::new();
        let bytes = b"abc".to_vec();
        let id = ContentDigest::of(&bytes);
        assert_eq!(store.insert_checked(id, bytes), Ok(id));
    }

    #[test]
    fn insert_checked_rejects_mismatched_digest() {
        let mut store = ContentStore::new();
        let wrong = ContentDigest::of(b"not the bytes");
        let err = store
            .insert_checked(wrong, b"the real bytes".to_vec())
            .unwrap_err();
        assert!(matches!(err, ContentStoreError::DigestMismatch { .. }));
    }

    #[test]
    fn verify_all_passes_for_clean_store() {
        let mut store = ContentStore::new();
        store.insert(b"a".to_vec());
        store.insert(b"b".to_vec());
        assert!(store.verify_all().is_ok());
    }

    /// The per-character decode [`decode_hex_32`] replaced: its oracle.
    fn decode_hex_32_per_char(hex: &str) -> Option<[u8; 32]> {
        if hex.len() != 64 {
            return None;
        }
        let mut buf = [0u8; 32];
        for (i, byte) in buf.iter_mut().enumerate() {
            let hi = (hex.as_bytes()[i * 2] as char).to_digit(16)?;
            let lo = (hex.as_bytes()[i * 2 + 1] as char).to_digit(16)?;
            *byte = (hi * 16 + lo) as u8;
        }
        Some(buf)
    }

    /// The uppercase refusal plus per-character decode that
    /// `Blake3ContentId::from_hex` ran before [`decode_hex_32_lower`]: its
    /// oracle.
    fn decode_hex_32_lower_per_char(hex: &str) -> Option<[u8; 32]> {
        if hex.bytes().any(|b| b.is_ascii_uppercase()) {
            return None;
        }
        decode_hex_32_per_char(hex)
    }

    fn assert_decoders_agree(hex: &str) {
        assert_eq!(decode_hex_32(hex), decode_hex_32_per_char(hex), "{hex:?}");
        assert_eq!(
            decode_hex_32_lower(hex),
            decode_hex_32_lower_per_char(hex),
            "{hex:?} (lowercase domain)"
        );
    }

    /// Every ASCII byte at every one of the 64 positions of an otherwise valid
    /// digest: that covers each invalid class (controls, the bytes below `0`,
    /// between `9` and `A`, `G-Z`, between `Z` and `a`, `g-z`, above `z`, and
    /// DELETE) and the uppercase `A-F` that only the lowercase domain refuses.
    #[test]
    fn hex_decode_matches_per_char_for_every_ascii_byte_at_every_position() {
        // A mixed-case base, valid case-insensitively, and a lowercase base,
        // valid in both domains, so a substituted byte is judged against a
        // digest each decoder accepts.
        let mixed = "0123456789abcdefABCDEF".repeat(3)[..64].to_owned();
        let lower = "fedcba9876543210".repeat(4);
        assert!(decode_hex_32(&mixed).is_some() && decode_hex_32_lower(&mixed).is_none());
        assert!(decode_hex_32(&lower).is_some() && decode_hex_32_lower(&lower).is_some());
        for base in [&mixed, &lower] {
            for position in 0..64 {
                for byte in 0..0x80_u8 {
                    let mut bytes = base.clone().into_bytes();
                    bytes[position] = byte;
                    let hex = String::from_utf8(bytes).expect("ASCII is UTF-8");
                    assert_decoders_agree(&hex);
                }
            }
        }
    }

    /// Every non-ASCII byte, as the lead or a continuation byte of a UTF-8
    /// character standing in for digits, at every position it fits; and every
    /// length other than 64.
    #[test]
    fn hex_decode_matches_per_char_for_non_ascii_and_wrong_lengths() {
        let lower = "0123456789abcdef".repeat(4);
        // Two-, three- and four-byte characters: between them their bytes
        // cover the leads 0xC2-0xDF, 0xE0-0xEF and 0xF0-0xF4 and continuation
        // bytes across 0x80-0xBF.
        let mut chars: Vec<char> = Vec::new();
        chars.extend(('\u{80}'..='\u{7ff}').step_by(7));
        chars.extend(('\u{800}'..='\u{ffff}').step_by(509));
        chars.extend(('\u{10000}'..='\u{10ffff}').step_by(40_009));
        let mut seen = [false; 256];
        for c in chars {
            let width = c.len_utf8();
            let mut encoded = [0_u8; 4];
            c.encode_utf8(&mut encoded);
            for &b in &encoded[..width] {
                seen[usize::from(b)] = true;
            }
            for position in 0..=64 - width {
                let hex = format!("{}{c}{}", &lower[..position], &lower[position + width..]);
                assert_eq!(hex.len(), 64);
                assert_decoders_agree(&hex);
            }
        }
        assert!(
            (0x80..=0xBF).chain(0xC2..=0xF4).all(|b| seen[b]),
            "every continuation and lead byte is exercised"
        );
        for len in [0, 1, 2, 31, 32, 62, 63, 65, 66, 128] {
            let hex: String = "a".repeat(len);
            assert_decoders_agree(&hex);
        }
    }
}
