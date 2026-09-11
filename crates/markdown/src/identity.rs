// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Node identity: content-addressed IRIs for units and sections.
//!
//! Dialect-independent. Identity asks only for a kind, a source id, a
//! profile's **chunking** id, a byte span, and the digest of that span's
//! bytes, so a reader of another structured format mints under exactly
//! this law by handing it the same six facts.
//!
//! The profile id in the preimage is
//! [`Profile::chunking_id`](crate::Profile::chunking_id) and not
//! [`Profile::contract_id`](crate::Profile::contract_id). A node's
//! identity answers *what this is* — this span of these bytes, cut by
//! this law — and a vocabulary or a canon base changes what is said
//! about it, never what it is. Were the emission id the field, a
//! consumer who merely declared a canon base would re-mint every unit
//! and section of the corpus and orphan every reference it had stored,
//! and two graphs describing the same text could never merge.

use purrdf_core::ContentDigest;
use purrdf_core::embedding::ChunkingContractId;

use crate::profile::Vocabulary;

/// The identity of a unit, with the digest algorithm inside both the
/// preimage and the IRI. A consumer re-derives it from the bytes at a
/// span to prove a hit is bound to its source.
///
/// `contract` is
/// [`Profile::chunking_id`](crate::Profile::chunking_id) — the law that
/// cut this span — and never
/// [`Profile::contract_id`](crate::Profile::contract_id); see the module
/// documentation for why the narrower id is the one a node is addressed
/// under.
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

/// The identity of a section, over its heading line's bytes, under the
/// same chunking id [`unit_iri`] takes.
///
/// It is the formula [`render`](crate::render) mints every section IRI
/// under, offered to a consumer that holds the source and a recorded
/// heading span and wants to re-derive one for itself — exactly as
/// [`unit_iri`] is. The projection reaches the formula by the digest
/// form, because it has already taken the digest it needs; that is one
/// route and not two, since this function's whole body is the digest
/// form over `ContentDigest::of(heading_line)`, so the two agreeing
/// proves nothing about either.
///
/// What the formula *is* is proven elsewhere, and only elsewhere: this
/// module's own vectors mint from a preimage built by hand out of §5's
/// seven fields, and
/// `the_preimage_section_five_states_mints_the_very_ids_the_slicer_emits`
/// in `tests/slicer.rs` rebuilds that preimage from the specification's
/// words and holds it against the subject of every section and unit
/// claim a real document emits.
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

/// The identity of one **citation edge**: what a concordance row lifted
/// onto one unit.
///
/// A row states one edge per unit it lifted onto, and the edge is the
/// node that carries the row's own sources beside the row's own anchors.
/// Two rows covering one verse must therefore mint two nodes, and one
/// row covering two verses must mint two more — so the pair is what is
/// addressed, not the row alone.
///
/// It is the same discipline as [`unit_iri`], asked of a different
/// thing. The position is the row's own line span, so an identical row
/// written twice in one table is two edges; the content is a
/// length-prefixed preimage of the row's line, the unit's IRI **and the
/// triple terms the node reifies**, so the same row lifting onto two
/// verses is two edges, and a unit that re-mints (its text edited, its
/// span moved) re-mints every edge that named it. Nothing else is
/// reachable from a citation, which is why it carries no span of its own
/// into the graph.
///
/// # Why what it reifies is in the preimage
///
/// A citation node is a **reifier of one lifting**: it identifies one
/// row's lifting onto one unit, and it states, as `rdf:reifies` objects,
/// every cited edge that lifting produced — none, one, or many. So two
/// liftings that reify different *sets* of terms must be two nodes.
/// Were they one, two graphs whose citation nodes carried that one IRI
/// would assert different reified edges under it, and merging them would
/// silently produce a node reifying the union — asserting edges no
/// single document ever stated of it.
///
/// The row's line does not settle that, because what the row writes is
/// not what the graph states. Under a declared canon base the object is
/// the minted IRI `base ++ anchor`; under none it is the anchor as a
/// literal typed with the anchor datatype; and the predicate is whatever
/// the vocabulary's `cites` field names. One row read under two profiles
/// can therefore state two different *sets* of triples while every byte
/// of the row stands still.
///
/// `reified` closes that: it is the whole list of triple terms the node
/// reifies, each rendered exactly as the emission law writes it — the
/// whole of `<<( <unit> <cites> <anchor> )>>`, subject, predicate and
/// object — in the row's own order, so the digest distinguishes both
/// which edges were reified and the order the row named them in. A row
/// that lifted no anchor passes an empty slice, reifies nothing, and is
/// still exactly one node.
// Eight fields, because the preimage has eight fields. A parameter
// struct would hide behind one name the very list a producer in another
// language has to reproduce term for term, and the list is the law
// here: the identity formulas in
// [`Profile::emission_bytes`](crate::Profile::emission_bytes) spell out
// these same fields in these same positions.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn citation_iri(
    vocabulary: &Vocabulary,
    source_id: &str,
    contract: &ChunkingContractId,
    row_start: u64,
    row_end: u64,
    row_line: &[u8],
    unit_node: &str,
    reified: &[String],
) -> String {
    let mut content = Vec::new();
    push_field(&mut content, row_line);
    push_field(&mut content, unit_node.as_bytes());
    for term in reified {
        push_field(&mut content, term.as_bytes());
    }
    node_iri_of_digest(
        vocabulary,
        "citation",
        source_id,
        contract,
        row_start,
        row_end,
        &ContentDigest::of(&content),
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

    /// The node base these vectors mint under, written out rather than
    /// read off a [`Vocabulary`], for the same reason the preimage is.
    const BASE: &str = "urn:test:";
    const SOURCE: &str = "urn:test:doc";

    /// §5's node identity, rebuilt here **by hand**: seven fields, each
    /// prefixed with its length as eight bytes little-endian, in the
    /// order §5 lists them — the kind, the source id, the chunking id,
    /// the span's start and end as eight bytes little-endian each, the
    /// digest algorithm tag, and the digest of the content — digested,
    /// and written `<node base><kind>:<alg>:<hex>`.
    ///
    /// Nothing under test is called. [`push_field`] could lose its
    /// prefix, [`node_iri_of_digest`] could reorder its fields or drop
    /// the algorithm tag, and these vectors would fail — which is the
    /// whole reason to write the preimage out twice. Holding
    /// [`unit_iri`] against [`node_iri_of_digest`] instead would prove
    /// nothing at all: `unit_iri`'s entire body is that call.
    ///
    /// The wider claim — that a real document *fills* these fields the
    /// way §5 says it does — is a different fact, and it is held in
    /// `the_preimage_section_five_states_mints_the_very_ids_the_slicer_emits`
    /// in `tests/slicer.rs`, over the subject of every claim the guide
    /// emits.
    fn by_hand(
        kind: &str,
        contract: &ChunkingContractId,
        byte_start: u64,
        byte_end: u64,
        content: &[u8],
    ) -> String {
        let (start, end) = (byte_start.to_le_bytes(), byte_end.to_le_bytes());
        let content_digest = ContentDigest::of(content);
        let mut preimage = Vec::new();
        for field in [
            kind.as_bytes(),
            SOURCE.as_bytes(),
            contract.as_bytes(),
            &start[..],
            &end[..],
            crate::DIGEST_ALGORITHM.as_bytes(),
            content_digest.as_bytes(),
        ] {
            preimage.extend_from_slice(&(field.len() as u64).to_le_bytes());
            preimage.extend_from_slice(field);
        }
        format!(
            "{BASE}{kind}:{}:{}",
            crate::DIGEST_ALGORITHM,
            ContentDigest::of(&preimage).to_hex()
        )
    }

    #[test]
    fn a_units_identity_is_section_fives_preimage_over_its_own_bytes() {
        let v = Vocabulary::under(BASE).expect("a vocabulary");
        let contract = crate::Profile::new("t", 1, v.clone()).chunking_id();
        let span = b"a span of bytes";
        assert_eq!(
            unit_iri(&v, SOURCE, &contract, 3, 18, span),
            by_hand("unit", &contract, 3, 18, span)
        );
        // The span is a field, so the same bytes read at another offset
        // are another node.
        assert_ne!(
            unit_iri(&v, SOURCE, &contract, 3, 18, span),
            unit_iri(&v, SOURCE, &contract, 4, 19, span)
        );
    }

    /// The same preimage for a section, over its *heading* span — the
    /// path the projection actually takes, since it holds the heading
    /// line and digests those bytes itself.
    #[test]
    fn a_sections_identity_is_the_same_preimage_over_its_heading_line() {
        let v = Vocabulary::under(BASE).expect("a vocabulary");
        let contract = crate::Profile::new("t", 1, v.clone()).chunking_id();
        let heading = b"## The Marrow Shelf";
        assert_eq!(
            section_iri(&v, SOURCE, &contract, 64, 83, heading),
            by_hand("section", &contract, 64, 83, heading)
        );
        // And the kind is a field, so the same span of the same bytes is
        // a different node under a different kind.
        assert_ne!(
            section_iri(&v, SOURCE, &contract, 64, 83, heading),
            unit_iri(&v, SOURCE, &contract, 64, 83, heading)
        );
    }

    /// The digest form the projection reaches for is the *same* formula,
    /// asked of a digest already taken — and asked here against the
    /// hand-built preimage rather than against [`unit_iri`], because
    /// `unit_iri` is a call to it and the two cannot disagree.
    #[test]
    fn the_digest_form_mints_section_fives_preimage_too() {
        let v = Vocabulary::under(BASE).expect("a vocabulary");
        let contract = crate::Profile::new("t", 1, v.clone()).chunking_id();
        let span = b"a span of bytes";
        assert_eq!(
            node_iri_of_digest(
                &v,
                "unit",
                SOURCE,
                &contract,
                3,
                18,
                &ContentDigest::of(span)
            ),
            by_hand("unit", &contract, 3, 18, span)
        );
    }

    /// The field the reifier's identity rests on: what a row *reifies*
    /// is inside its node, so one row read under two canon bases — or
    /// under two spellings of the citing predicate — is two nodes and
    /// never one node reifying two triples.
    #[test]
    fn a_citations_identity_carries_the_triple_terms_it_reifies() {
        let v = Vocabulary::under("urn:test:").expect("a vocabulary");
        let contract = crate::Profile::new("t", 1, v.clone()).chunking_id();
        let row = b"| 1 | `atlas/a.ttl` | `anchor-one` |";
        let unit = "urn:test:unit:sha256:00";
        let mint = |terms: &[&str]| {
            let terms: Vec<String> = terms.iter().map(|t| (*t).to_owned()).collect();
            citation_iri(&v, "urn:test:doc", &contract, 0, 36, row, unit, &terms)
        };
        let term =
            |predicate: &str, object: &str| format!("<<( <{unit}> <{predicate}> {object} )>>");
        let literal = term("urn:test:cites", "\"anchor-one\"^^<urn:test:anchor>");
        let based = term("urn:test:cites", "<https://example.org/canon#anchor-one>");
        let otherwise = term("urn:test:cites", "<https://other.example/atlas/anchor-one>");
        // The same anchor, cited through a renamed predicate: a
        // different triple, and so a different reifier.
        let renamed = term("urn:test:quotes", "\"anchor-one\"^^<urn:test:anchor>");
        let minted = [
            mint(&[&literal]),
            mint(&[&based]),
            mint(&[&otherwise]),
            mint(&[&renamed]),
            mint(&[]),
        ];
        for (i, left) in minted.iter().enumerate() {
            for right in &minted[i + 1..] {
                assert_ne!(left, right, "a different term is a different reifier");
            }
        }
        // And it is still a function of what it is handed: the same
        // terms in the same order mint the same node.
        assert_eq!(mint(&[&literal]), minted[0]);
        // Order is a fact of the row, so it is a fact of the identity.
        assert_ne!(
            mint(&[&literal, &based]),
            mint(&[&based, &literal]),
            "the row wrote its anchors in an order"
        );
    }
}
