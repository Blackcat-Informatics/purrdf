// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Content-addressed blob store for the self-describing bundle.
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

use std::fmt;

use purrdf_hash::hex::Digest32;
use sha2::{Digest, Sha256};

use crate::FastMap;

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
pub struct ContentDigest(Digest32);

impl ContentDigest {
    /// Compute the digest of `bytes` directly (SHA-256).
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(Digest32::new(hasher.finalize().into()))
    }

    /// Wrap 32 raw digest bytes as a `ContentDigest` WITHOUT re-hashing.
    ///
    /// For callers that have already computed a SHA-256 digest themselves (e.g. a
    /// multi-section content fold) and want to carry the result as a `ContentDigest`.
    /// Unlike [`of`](Self::of), this does NOT hash — it adopts `raw` verbatim.
    pub fn from_raw(raw: [u8; 32]) -> Self {
        Self(Digest32::new(raw))
    }

    /// The 32 raw digest bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// The lowercase-hex rendering of the digest (64 chars).
    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }

    /// Parse a 64-char hex digest of either case
    /// ([`purrdf_hash::hex::decode_32`]). Returns `None` on any malformed
    /// input (wrong length or non-hex characters).
    pub fn from_hex(hex: &str) -> Option<Self> {
        purrdf_hash::hex::decode_32(hex).map(Self::from_raw)
    }
}

impl fmt::Debug for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentDigest({})", self.0)
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
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
    blobs: FastMap<ContentDigest, Bytes>,
}

impl ContentStore {
    /// A fresh, empty store.
    pub fn new() -> Self {
        Self {
            blobs: FastMap::default(),
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
        // The valid neighbour of both refusals: 64 digits, of either case.
        let d = ContentDigest::of(b"hello world");
        assert_eq!(ContentDigest::from_hex(&d.to_hex().to_uppercase()), Some(d));
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
}
