// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The HNSW **derived-index profile**: the one place the crate's PURREMB identity is
//! spelled out.
//!
//! PURREMB stores opaque index payloads and refuses to interpret them (see
//! `purrdf_core`'s `IndexGuardView`). Everything a reader needs to decide *which* opaque
//! payload is this crate's — and to reject one that claims to be but is not — is therefore
//! a contract this crate must state for itself. That contract is this module:
//!
//! * **Identity.** The [`IndexGuardContract::implementation`](purrdf_core::IndexGuardContract)
//!   artifact identity, whose stable identifier is [`IMPLEMENTATION_ID`],
//!   whose revision bytes carry the approximation evidence string, and whose digest binds
//!   the profile declaration.
//! * **Parameter encoding.** [`PARAMETER_ENCODING`], naming the canonical TLV block that
//!   [`parameters`] produces and [`parse_parameters`] reads.
//! * **Payload media type.** [`INDEX_MEDIA_TYPE`](crate::INDEX_MEDIA_TYPE).
//! * **Loss contract.** [`loss_contract`] — approximate, and no vector transform. The
//!   approximation is inherent: PURREMB's index loss contract always carries the
//!   `approximate = true` field (tag 1), and this profile sets `transforms_vectors` false
//!   because an HNSW graph stores no vectors at all.
//! * **Canonical payload layout.** The byte image [`crate::HnswIndex::canonical_image`]
//!   emits, documented below.
//!
//! # The approximation contract, stated honestly
//!
//! [`LOSS_EVIDENCE`] is the one sentence this profile publishes about its own quality:
//! *"approximate: recall measured against the exact oracle and pinned per fixture; an offer
//! of candidates is never a proof of absence"*. It said "recall unmeasured on realistic
//! corpora" until the corpora were fixed: the figure had been taken over uniform random
//! vectors, the one input class whose distances concentrate so hard that no index can score
//! well on it, and the resulting number was a statement about the generator. It is not a
//! disclaimer appended to documentation — it is carried **in the artifact**, as the
//! implementation identity's revision bytes, so the guard digest covers it and
//! [`crate::guard::validate_guard`] refuses a guard whose revision says anything else. A
//! host cannot bind an HNSW index without also binding the statement of what it does not
//! promise.
//!
//! # The parameter TLV schema
//!
//! Four `u64` fields in ascending tag order, little-endian, each padded to an 8-byte
//! boundary — the canonical form PURREMB's own TLV codec uses:
//!
//! | tag | field | wire type |
//! |----:|-------|-----------|
//! | 1 | `M` | `u64` |
//! | 2 | `M0` | `u64` |
//! | 3 | `ef_construction` | `u64` |
//! | 4 | `ef_search` | `u64` |
//!
//! The block is the guard's `parameters` field, and rdf-core commits its SHA-256 as the
//! guard's self-digest, so a changed parameter is a guard that fails verification rather
//! than an index that silently searches under a different identity.
//!
//! # The canonical payload layout
//!
//! The image [`parameters`] does not touch is the graph itself, emitted by
//! [`crate::HnswIndex::canonical_image`] and decoded by
//! [`crate::HnswIndex::decode`]. It is a contiguous, little-endian, 8-byte-aligned image
//! with no pointers and no host-endian fields, so it is the same bytes on every target:
//!
//! ```text
//! header:
//!   magic      [u8; 8]   IMAGE_MAGIC
//!   version    u32       IMAGE_VERSION
//!   kernel     u32       0 cosine, 1 negative-dot, 2 squared-euclidean
//!   M          u64
//!   M0         u64
//!   ef_c       u64
//!   ef_search  u64
//!   node_count u64
//!   max_level  u32
//!   reserved   u32       0
//!   entry      u64       row, or u64::MAX for an empty graph
//! node records, in ascending row order:
//!   row        u64       must equal the record's position
//!   level      u32
//!   reserved   u32       0
//!   layer records, ascending layer 0..=level:
//!     layer      u32     must equal the record's position
//!     reserved   u32     0
//!     count      u64
//!     neighbours, strictly ascending by neighbour row:
//!       row      u64
//!       distance u64     f64 bits
//! ```
//!
//! The in-memory graph orders a node's neighbours by `(distance, row)`; the image orders
//! them by neighbour row, because the row set is the identity and the distances are
//! derived. Decoding re-sorts by rank, so the two are views of one graph.

use purrdf_core::{ArtifactIdentity, ArtifactIdentityKind, ContentDigest, IndexLossContract};

use crate::error::{HnswError, Result};
use crate::params::Params;

/// The stable identifier of the HNSW derived-index implementation.
///
/// The same string as [`crate::IMPLEMENTATION_ID`]; re-exported through this module so a
/// consumer that reads the profile does not have to hop between modules.
pub use crate::IMPLEMENTATION_ID;

/// The stable identifier of the canonical parameter encoding.
pub const PARAMETER_ENCODING: &str =
    "application/vnd.blackcatinformatics.purrdf.hnsw.parameters+tlv-v1";

/// The media type of the implementation identity itself.
pub const IMPLEMENTATION_MEDIA_TYPE: &str =
    "application/vnd.blackcatinformatics.purrdf.hnsw.profile-v1";

/// The approximation evidence string, carried as the implementation identity's revision.
pub const LOSS_EVIDENCE: &str = "approximate: recall measured against the exact oracle and pinned per fixture; an offer \
     of candidates is never a proof of absence";

/// The parameter block's tag for `M`.
pub const PARAM_M: u16 = 1;
/// The parameter block's tag for `M0`.
pub const PARAM_M0: u16 = 2;
/// The parameter block's tag for `ef_construction`.
pub const PARAM_EF_CONSTRUCTION: u16 = 3;
/// The parameter block's tag for `ef_search`.
pub const PARAM_EF_SEARCH: u16 = 4;

/// The canonical image's magic marker, exposed for profile documentation and tooling.
pub const PAYLOAD_MAGIC: [u8; 8] = crate::graph::IMAGE_MAGIC;

/// The canonical image's format version.
pub const PAYLOAD_VERSION: u32 = crate::graph::IMAGE_VERSION;

/// The TLV value wire type for a little-endian `u64`.
const WIRE_U64: u8 = 4;
/// The critical-field flag bit.
const FLAG_CRITICAL: u8 = 1;

/// The implementation identity this profile binds into every guard.
///
/// The digest is a domain-separated SHA-256 over the profile declaration, and the revision
/// bytes are [`LOSS_EVIDENCE`] — so the guard digest commits the approximation statement
/// and not merely the algorithm name.
///
/// # Panics
///
/// Panics only if the static profile declaration is malformed, which the compile-time
/// constants rule out; callers cannot reach a panic.
#[must_use]
pub fn implementation() -> ArtifactIdentity {
    ArtifactIdentity::new(
        IMPLEMENTATION_ID,
        IMPLEMENTATION_MEDIA_TYPE,
        ContentDigest::of(profile_declaration().as_bytes()),
        Some(LOSS_EVIDENCE.as_bytes().to_vec()),
        ArtifactIdentityKind::Single,
    )
    .expect("the HNSW profile declaration is static and valid")
}

/// The exact bytes whose SHA-256 is the implementation identity's digest.
///
/// A stable, human-readable declaration rather than a serialized struct: it has no host
/// layout and no trailing version-dependent representation, so the digest is a function of
/// the profile's *meaning*.
#[must_use]
pub fn profile_declaration() -> String {
    format!(
        "{IMPLEMENTATION_ID}\n{PARAMETER_ENCODING}\n{}\n{}\n{LOSS_EVIDENCE}",
        crate::INDEX_MEDIA_TYPE,
        "approximate=true;transforms_vectors=false"
    )
}

/// The loss contract every HNSW guard carries.
///
/// Approximate (PURREMB's loss contract writes the `approximate = true` field itself) and
/// non-transforming: an HNSW graph stores no vectors, so no quantization contract applies
/// and `loss_encoding`/`loss_parameters` are absent, exactly as rdf-core requires for
/// `transforms_vectors == false`.
#[must_use]
pub const fn loss_contract() -> IndexLossContract {
    IndexLossContract {
        transforms_vectors: false,
        loss_encoding: None,
        loss_parameters: None,
    }
}

/// Encode `params` as the canonical parameter TLV block.
#[must_use]
pub fn parameters(params: Params) -> Vec<u8> {
    let mut out = Vec::new();
    put_u64(&mut out, PARAM_M, params.m() as u64);
    put_u64(&mut out, PARAM_M0, params.m0() as u64);
    put_u64(
        &mut out,
        PARAM_EF_CONSTRUCTION,
        params.ef_construction() as u64,
    );
    put_u64(&mut out, PARAM_EF_SEARCH, params.ef_search() as u64);
    out
}

/// Decode and validate a canonical parameter TLV block.
///
/// # Errors
///
/// [`HnswError::GuardProfile`] for a missing, out-of-order, over-long or wrong-wire-type
/// field, a non-zero padding byte, or a value that [`Params::new`] refuses (including a
/// negative parameter smuggled in as a two's-complement `u64`).
pub fn parse_parameters(bytes: &[u8]) -> Result<Params> {
    let mut reader = Reader::new(bytes);
    let mut m = None;
    let mut m0 = None;
    let mut ef_construction = None;
    let mut ef_search = None;
    let mut previous_tag = 0_u16;
    while !reader.is_empty() {
        let (tag, wire, value) = reader.entry()?;
        if tag <= previous_tag {
            return Err(profile_error(format!(
                "parameter tags must be strictly ascending, got {tag} after {previous_tag}"
            )));
        }
        previous_tag = tag;
        if wire != WIRE_U64 {
            return Err(profile_error(format!(
                "parameter {tag} has wire type {wire}, not a u64"
            )));
        }
        let value = u64::from_le_bytes(
            value
                .try_into()
                .map_err(|_| profile_error(format!("parameter {tag} is not eight bytes")))?,
        );
        match tag {
            PARAM_M => m = Some(value),
            PARAM_M0 => m0 = Some(value),
            PARAM_EF_CONSTRUCTION => ef_construction = Some(value),
            PARAM_EF_SEARCH => ef_search = Some(value),
            other => {
                return Err(profile_error(format!(
                    "unknown critical parameter tag {other}"
                )));
            }
        }
    }
    let missing = |field: &str| profile_error(format!("the parameter block is missing {field}"));
    let m = m.ok_or_else(|| missing("M"))?;
    let m0 = m0.ok_or_else(|| missing("M0"))?;
    let ef_construction = ef_construction.ok_or_else(|| missing("ef_construction"))?;
    let ef_search = ef_search.ok_or_else(|| missing("ef_search"))?;
    let widen = |name: &str, value: u64| {
        usize::try_from(value)
            .map_err(|_| profile_error(format!("parameter {name} = {value} does not fit usize")))
    };
    Params::new(
        widen("M", m)?,
        widen("M0", m0)?,
        widen("ef_construction", ef_construction)?,
        widen("ef_search", ef_search)?,
    )
}

/// A guard-profile failure, so every rejection reads as this profile's own.
fn profile_error(description: impl Into<String>) -> HnswError {
    HnswError::GuardProfile {
        description: description.into(),
    }
}

/// Append one canonical TLV entry (critical, padded to an 8-byte boundary).
fn put_u64(out: &mut Vec<u8>, tag: u16, value: u64) {
    put_entry(out, tag, WIRE_U64, &value.to_le_bytes());
}

/// Append one canonical TLV entry with the given wire type and value.
fn put_entry(out: &mut Vec<u8>, tag: u16, wire: u8, value: &[u8]) {
    out.extend_from_slice(&tag.to_le_bytes());
    out.push(wire);
    out.push(FLAG_CRITICAL);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
    let aligned = (out.len() + 7) & !7;
    out.resize(aligned, 0);
}

/// A bounds-checked reader over a canonical TLV block.
///
/// Deliberately rejects the same shapes rdf-core's codec does — non-ascending tags, a
/// non-critical-flag bit, wrong wire types, non-zero padding, and trailing bytes — so a
/// parameter block this crate accepts is one a canonical reader would too.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    const fn is_empty(&self) -> bool {
        self.at == self.bytes.len()
    }

    /// One `(tag, wire, value)` triple, advancing past its padding.
    fn entry(&mut self) -> Result<(u16, u8, &'a [u8])> {
        let header_end = self.at + 8;
        let header = self
            .bytes
            .get(self.at..header_end)
            .ok_or_else(|| profile_error("a parameter entry is truncated"))?;
        let tag = u16::from_le_bytes([header[0], header[1]]);
        let wire = header[2];
        if header[3] & !FLAG_CRITICAL != 0 {
            return Err(profile_error("a parameter entry sets a reserved flag"));
        }
        if header[3] & FLAG_CRITICAL == 0 {
            return Err(profile_error("a parameter entry is not critical"));
        }
        let length = u32::from_le_bytes(header[4..8].try_into().expect("fixed slice"));
        let length = usize::try_from(length)
            .map_err(|_| profile_error("a parameter entry length does not fit usize"))?;
        let value_end = header_end
            .checked_add(length)
            .ok_or_else(|| profile_error("a parameter entry length overflows"))?;
        let value = self
            .bytes
            .get(header_end..value_end)
            .ok_or_else(|| profile_error("a parameter entry value is truncated"))?;
        let padded = (value_end + 7) & !7;
        let padding = self
            .bytes
            .get(value_end..padded)
            .ok_or_else(|| profile_error("a parameter entry padding is truncated"))?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(profile_error("a parameter entry has non-zero padding"));
        }
        self.at = padded;
        Ok((tag, wire, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_round_trip_through_the_canonical_block() {
        let params = Params::new(16, 32, 200, 64).expect("valid");
        let bytes = parameters(params);
        assert_eq!(bytes.len() % 8, 0, "the block is 8-byte aligned");
        assert_eq!(parse_parameters(&bytes).expect("round trips"), params);
    }

    #[test]
    fn a_non_ascending_tag_is_refused() {
        let mut bytes = Vec::new();
        // M0 before M.
        put_u64(&mut bytes, PARAM_M0, 32);
        put_u64(&mut bytes, PARAM_M, 16);
        assert!(matches!(
            parse_parameters(&bytes),
            Err(HnswError::GuardProfile { .. })
        ));
    }

    #[test]
    fn a_missing_parameter_is_refused() {
        let mut bytes = Vec::new();
        put_u64(&mut bytes, PARAM_M, 16);
        assert!(parse_parameters(&bytes).is_err());
    }

    #[test]
    fn non_zero_padding_is_refused() {
        // A four-`u64` block is already 8-byte aligned, so the padding case is exercised
        // on the reader directly: one aligned entry, then a one-byte value whose seven
        // padding bytes are non-zero.
        let mut block = Vec::new();
        put_u64(&mut block, PARAM_M, 16);
        block.extend_from_slice(&PARAM_M0.to_le_bytes());
        block.push(2); // the UTF-8 wire type, whose value here is one byte
        block.push(FLAG_CRITICAL);
        block.extend_from_slice(&1u32.to_le_bytes());
        block.push(b'x');
        block.extend_from_slice(&[9; 7]);
        let mut reader = Reader::new(&block);
        reader.entry().expect("the aligned entry parses");
        assert!(reader.entry().is_err(), "non-zero padding must be refused");
    }

    #[test]
    fn the_loss_contract_is_approximate_and_non_transforming() {
        let loss = loss_contract();
        assert!(!loss.transforms_vectors);
        assert!(loss.loss_encoding.is_none());
        assert!(loss.loss_parameters.is_none());
    }

    #[test]
    fn the_implementation_identity_carries_the_evidence() {
        let identity = implementation();
        assert_eq!(identity.identifier, IMPLEMENTATION_ID);
        assert_eq!(
            identity.revision.as_deref(),
            Some(LOSS_EVIDENCE.as_bytes()),
            "the evidence sentence is bound into the guard identity"
        );
    }
}
