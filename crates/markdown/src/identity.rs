// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Node identity: content-addressed IRIs for units and sections.
//!
//! Dialect-independent. Identity asks only for a kind, a source id, a
//! profile's contract id, a byte span, and the digest of that span's
//! bytes, so a reader of another structured format mints under exactly
//! this law by handing it the same six facts.

use purrdf_core::ContentDigest;
use purrdf_core::embedding::ChunkingContractId;

use crate::profile::Vocabulary;

/// The identity of a unit, with the digest algorithm inside both the
/// preimage and the IRI. A consumer re-derives it from the bytes at a
/// span to prove a hit is bound to its source.
#[must_use]
pub fn unit_iri(
    vocabulary: &Vocabulary,
    source_id: &str,
    contract: &ChunkingContractId,
    byte_start: u64,
    byte_end: u64,
    span: &[u8],
) -> String {
    node_iri(
        vocabulary, "unit", source_id, contract, byte_start, byte_end, span,
    )
}

/// The identity of a section, over its heading line's bytes.
#[must_use]
pub fn section_iri(
    vocabulary: &Vocabulary,
    source_id: &str,
    contract: &ChunkingContractId,
    byte_start: u64,
    byte_end: u64,
    heading_line: &[u8],
) -> String {
    node_iri(
        vocabulary,
        "section",
        source_id,
        contract,
        byte_start,
        byte_end,
        heading_line,
    )
}

fn node_iri(
    vocabulary: &Vocabulary,
    kind: &str,
    source_id: &str,
    contract: &ChunkingContractId,
    byte_start: u64,
    byte_end: u64,
    bytes: &[u8],
) -> String {
    node_iri_of_digest(
        vocabulary,
        kind,
        source_id,
        contract,
        byte_start,
        byte_end,
        &ContentDigest::of(bytes),
    )
}

/// The same identity over a digest that has already been taken.
///
/// A unit carries its content digest in the model
/// ([`Unit::digest`](crate::Unit::digest)), so the projection mints
/// from that rather than hashing the same span twice. The preimage is
/// the one [`unit_iri`] builds, byte for byte.
pub(crate) fn node_iri_of_digest(
    vocabulary: &Vocabulary,
    kind: &str,
    source_id: &str,
    contract: &ChunkingContractId,
    byte_start: u64,
    byte_end: u64,
    digest: &ContentDigest,
) -> String {
    let mut preimage = Vec::new();
    push_field(&mut preimage, kind.as_bytes());
    push_field(&mut preimage, source_id.as_bytes());
    push_field(&mut preimage, contract.as_bytes());
    push_field(&mut preimage, &byte_start.to_le_bytes());
    push_field(&mut preimage, &byte_end.to_le_bytes());
    push_field(&mut preimage, crate::DIGEST_ALGORITHM.as_bytes());
    push_field(&mut preimage, digest.as_bytes());
    format!(
        "{}{kind}:{}:{}",
        vocabulary.node_base,
        crate::DIGEST_ALGORITHM,
        ContentDigest::of(&preimage).to_hex()
    )
}

/// Length-prefixed field: no two field sequences share a preimage.
fn push_field(out: &mut Vec<u8>, field: &[u8]) {
    out.extend_from_slice(&(field.len() as u64).to_le_bytes());
    out.extend_from_slice(field);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minting_from_a_taken_digest_is_minting_from_the_bytes() {
        let v = Vocabulary::under("urn:test:").expect("a vocabulary");
        let contract = crate::Profile::new("t", 1, v.clone()).contract_id();
        let span = b"a span of bytes";
        assert_eq!(
            unit_iri(&v, "urn:test:doc", &contract, 3, 18, span),
            node_iri_of_digest(
                &v,
                "unit",
                "urn:test:doc",
                &contract,
                3,
                18,
                &ContentDigest::of(span)
            )
        );
    }
}
