// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Hash domain separation: the one type every domain-separation string in the
//! workspace is spelt through.

/// A hash domain: the byte string that names one preimage family.
///
/// A digest is only as unambiguous as its preimage. When two different
/// structures are hashed with the same algorithm, a caller that prefixes each
/// preimage with its own domain guarantees that a digest of one can never be
/// read as a digest of the other. A `Domain` is that prefix (or the named
/// "domain" part of a framed preimage), and nothing else: it does not frame,
/// hash or version anything itself.
///
/// # Contract
///
/// * **One family per domain.** A `Domain` names exactly one preimage layout.
///   Two constructions never share a domain, even when their field lists
///   happen to coincide today.
/// * **Unique and prefix-free.** Across the whole workspace, no two registered
///   domains are equal, and no domain is a byte-prefix of another, so no
///   concatenation of a domain and a payload can begin another family's
///   preimage. `scripts/check-hash-domains.py` collects every
///   `Domain::new(…)` literal and holds this on every run; it also refuses a
///   domain-shaped literal handed to a hasher without going through a
///   `Domain`.
/// * **Existing spellings are never renamed.** A domain's bytes are part of
///   every digest computed under it, and those digests are published
///   identities: content addresses, cache keys, proof and artifact
///   identities, golden files. Renaming a domain silently moves every one of
///   them, so the bytes of a registered domain are frozen. A changed preimage
///   layout gets a *new* domain (a bumped version); an old spelling is never
///   reused for a different layout.
/// * **New domains** follow the convention the crate's README states,
///   `purrdf-<crate>/<purpose>/v<N>`.
///
/// `Domain` is a `const` newtype over a `'static` byte string, so every domain
/// is a compile-time constant living next to the construction it separates:
///
/// ```
/// use purrdf_hash::{Domain, blake3};
///
/// const EXAMPLE_DOMAIN: Domain = Domain::new(b"purrdf-example/record/v1");
///
/// let mut hasher = blake3::Hasher::new();
/// hasher.update(EXAMPLE_DOMAIN.as_bytes());
/// hasher.update(b"payload");
/// let digest = hasher.finalize();
/// assert_ne!(digest, blake3::hash(b"payload"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(&'static str);

impl Domain {
    /// The domain whose bytes are `bytes`, exactly: no terminator, separator
    /// or length prefix is added, so a registered spelling keeps the bytes it
    /// has always hashed.
    ///
    /// # Panics
    ///
    /// When `bytes` is not UTF-8. A domain is text — it is printed in
    /// specifications, carried as a CBOR text string and spelt into IRIs — so
    /// a `const` domain that is not UTF-8 fails to compile.
    #[must_use]
    pub const fn new(bytes: &'static [u8]) -> Self {
        match core::str::from_utf8(bytes) {
            Ok(text) => Self(text),
            Err(_) => panic!("a hash domain is UTF-8 text"),
        }
    }

    /// The domain's bytes, exactly as registered.
    #[must_use]
    pub const fn as_bytes(self) -> &'static [u8] {
        self.0.as_bytes()
    }

    /// The domain as text, for a construction that carries it as a string (a
    /// CBOR text string, an IRI segment). The same bytes as
    /// [`as_bytes`](Self::as_bytes).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// The number of bytes in the domain.
    #[must_use]
    pub const fn len(self) -> usize {
        self.0.len()
    }

    /// Whether the domain has no bytes. A registered domain never is: an empty
    /// domain separates nothing, and it is a prefix of every other one.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }
}

impl AsRef<[u8]> for Domain {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::Domain;

    const NUL_TERMINATED: Domain = Domain::new(b"example.org.v1.record\0");
    const SLASHED: Domain = Domain::new(b"purrdf-example/record/v1");

    /// The bytes come back exactly as written: no terminator, separator or
    /// length prefix is added or removed.
    #[test]
    fn the_bytes_are_returned_exactly_as_registered() {
        assert_eq!(NUL_TERMINATED.as_bytes(), b"example.org.v1.record\0");
        assert_eq!(NUL_TERMINATED.len(), 22);
        assert_eq!(SLASHED.as_bytes(), b"purrdf-example/record/v1");
        assert_eq!(SLASHED.as_ref(), SLASHED.as_bytes());
        assert!(!SLASHED.is_empty());
        assert_eq!(SLASHED.as_str(), "purrdf-example/record/v1");
        assert_eq!(
            NUL_TERMINATED.as_str().as_bytes(),
            NUL_TERMINATED.as_bytes()
        );
    }

    /// A domain is text: bytes that are not UTF-8 are refused.
    #[test]
    #[should_panic(expected = "a hash domain is UTF-8 text")]
    fn a_domain_that_is_not_utf8_is_refused() {
        let _ = Domain::new(b"purrdf-example/\xff/v1");
    }

    /// Its neighbour: control bytes, NUL and multi-byte UTF-8 are text and are
    /// accepted.
    #[test]
    fn a_domain_with_control_bytes_or_multibyte_text_is_accepted() {
        let domain = Domain::new("purrdf-example/\u{e9}\x1f\0".as_bytes());
        assert_eq!(domain.as_bytes(), b"purrdf-example/\xc3\xa9\x1f\0");
    }

    /// Hashing through the domain gives the digest of the raw bytes it
    /// replaced, so migrating a literal to a `Domain` moves no digest.
    #[test]
    fn hashing_through_a_domain_equals_hashing_its_raw_literal() {
        let mut through = crate::blake3::Hasher::new();
        through.update(SLASHED.as_bytes());
        through.update(b"payload");
        let mut raw = crate::blake3::Hasher::new();
        raw.update(b"purrdf-example/record/v1");
        raw.update(b"payload");
        assert_eq!(through.finalize(), raw.finalize());
    }

    /// Equality and order are the bytes'; two spellings that differ in one
    /// byte are two domains.
    #[test]
    fn domains_compare_by_their_bytes() {
        assert_eq!(SLASHED, Domain::new(b"purrdf-example/record/v1"));
        assert_ne!(SLASHED, Domain::new(b"purrdf-example/record/v2"));
        assert!(Domain::new(b"a") < Domain::new(b"b"));
    }
}
