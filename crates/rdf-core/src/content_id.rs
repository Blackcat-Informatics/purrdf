// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The BLAKE3 GTS content-id domain: `blake3:<hex>` term references.
//!
//! [`Blake3ContentId`] is a distinct 32-byte newtype from
//! [`ContentDigest`](crate::content_store::ContentDigest). Where `ContentDigest`
//! is the SHA-256 blob-store address computed by this crate (`of`/`from_raw`),
//! `Blake3ContentId` addresses the separate BLAKE3 domain used by the GTS
//! `blake3:<hex>` term encoding produced *outside* this crate. This type is
//! **decode-only**: it never hashes bytes and `purrdf-core` gains no `blake3`
//! dependency from it. Callers that need to mint a `Blake3ContentId` from raw
//! bytes must hash elsewhere and hand the crate the resulting hex or raw bytes.
//!
//! The hex-decode loop is shared with `ContentDigest::from_hex` via
//! [`decode_hex_32`](crate::content_store::decode_hex_32) so the two domains
//! never drift apart on parsing behavior; only the case-sensitivity policy
//! differs (this domain requires canonical lowercase hex).

use std::fmt;

use crate::content_store::decode_hex_32;

/// A content id in the BLAKE3 GTS domain (`blake3:<hex>` term references).
///
/// Distinct from [`ContentDigest`](crate::content_store::ContentDigest), which
/// is the SHA-256 blob-store domain. `Blake3ContentId` never hashes bytes —
/// it only decodes a pre-computed 64-char lowercase hex string (or wraps raw
/// bytes a caller already has) so this crate stays free of a `blake3` dependency.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Blake3ContentId([u8; 32]);

impl Blake3ContentId {
    /// Wrap 32 raw BLAKE3 digest bytes as a `Blake3ContentId` WITHOUT hashing.
    ///
    /// For callers that have already computed (or obtained) a BLAKE3 digest and
    /// want to carry the result as a `Blake3ContentId`.
    #[must_use]
    pub const fn from_raw(raw: [u8; 32]) -> Self {
        Self(raw)
    }

    /// The 32 raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The lowercase-hex rendering of the digest (64 chars).
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.0 {
            use fmt::Write as _;
            let _ = write!(s, "{b:02x}");
        }
        s
    }

    /// Parse a canonical 64-char **lowercase** hex digest. Returns `None` on
    /// any malformed input (wrong length, non-hex characters, or any
    /// uppercase `A`-`F`).
    ///
    /// Unlike [`ContentDigest::from_hex`](crate::content_store::ContentDigest::from_hex),
    /// this domain does not tolerate uppercase hex: the GTS `blake3:<hex>`
    /// encoding is always lowercase, so an uppercase input is treated as
    /// malformed rather than silently normalized.
    #[must_use]
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.bytes().any(|b| b.is_ascii_uppercase()) {
            return None;
        }
        decode_hex_32(hex).map(Self)
    }
}

impl fmt::Debug for Blake3ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blake3ContentId({})", self.to_hex())
    }
}

impl fmt::Display for Blake3ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::Blake3ContentId;
    use crate::content_store::ContentDigest;

    const KNOWN: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87, 0x78, 0x69, 0x5a, 0x4b, 0x3c, 0x2d,
        0x1e, 0xff,
    ];

    #[test]
    fn round_trips_through_hex() {
        let id = Blake3ContentId::from_raw(KNOWN);
        let hex = id.to_hex();
        assert_eq!(Blake3ContentId::from_hex(&hex), Some(id));
    }

    #[test]
    fn rejects_wrong_length() {
        let short = "a".repeat(63);
        let long = "a".repeat(65);
        assert_eq!(Blake3ContentId::from_hex(&short), None);
        assert_eq!(Blake3ContentId::from_hex(&long), None);
    }

    #[test]
    fn rejects_non_hex_char() {
        let mut s = "a".repeat(64);
        s.replace_range(0..1, "z");
        assert_eq!(Blake3ContentId::from_hex(&s), None);
    }

    #[test]
    fn rejects_uppercase_hex() {
        let upper = "AB".repeat(32);
        assert_eq!(Blake3ContentId::from_hex(&upper), None);
    }

    #[test]
    fn accepts_valid_lowercase_hex() {
        let hex = "ab".repeat(32);
        let id = Blake3ContentId::from_hex(&hex).expect("valid lowercase hex decodes");
        assert_eq!(id.as_bytes(), &[0xab; 32]);
    }

    #[test]
    fn is_a_distinct_type_from_content_digest() {
        let raw = [0x42u8; 32];
        let blake3_id = Blake3ContentId::from_raw(raw);
        let sha256_digest = ContentDigest::from_raw(raw);
        // Same underlying bytes, but distinct types: this only compiles because
        // they are not comparable to each other.
        assert_eq!(blake3_id.as_bytes(), sha256_digest.as_bytes());
    }
}
