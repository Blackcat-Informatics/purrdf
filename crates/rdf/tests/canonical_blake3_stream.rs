// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The streamed BLAKE3 digest of a flat canonical document is the identity a
//! GTS content label spells for that document's bytes.

use purrdf_rdf::{
    BlankScope, CanonHash, CanonicalizationControl, RDFC_V2_CORPUS_POLICY, RdfDatasetBuilder,
    RdfLiteral, try_canonicalize_flat_view, try_flat_blake3_view,
};

/// A caller that stores `blake3:<hex>` of the canonical bytes gets the same label
/// from the streamed digest as from the document it no longer has to build. The
/// fixture carries blanks, a reifier, annotations and a named graph, so the
/// document is neither trivial nor in the order the lines were added.
#[test]
fn the_streamed_digest_labels_the_same_identity_as_the_document_bytes() {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p").unwrap();
    let note = b.intern_iri("https://example.org/note").unwrap();
    let graph = b.intern_iri("https://example.org/g").unwrap();
    for i in (0..24).rev() {
        let s = b.intern_iri(&format!("https://example.org/s/{i}")).unwrap();
        let blank = b
            .intern_blank(&format!("b{i}"), BlankScope::DEFAULT)
            .unwrap();
        let o = b
            .intern_literal(RdfLiteral::language_tagged(format!("v {i} \"q\""), "en"))
            .unwrap();
        b.push_quad(s, p, blank, (i % 3 == 0).then_some(graph))
            .unwrap();
        let triple = b.intern_triple(s, p, o).unwrap();
        let reifier = b
            .intern_blank(&format!("r{i}"), BlankScope::DEFAULT)
            .unwrap();
        b.push_reifier_in_graph(reifier, triple, None).unwrap();
        b.push_annotation_in_graph(reifier, note, o, None).unwrap();
    }
    let dataset = b.freeze().unwrap();
    let control = || CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
    let document = try_canonicalize_flat_view(&*dataset, CanonHash::Sha256, &mut control())
        .expect("the fixture canonicalizes");
    let streamed = try_flat_blake3_view(&*dataset, CanonHash::Sha256, &mut control())
        .expect("the fixture streams");
    assert_eq!(
        purrdf_gts::wire::digest_label(&streamed),
        purrdf_gts::writer::digest_string(document.nquads().as_bytes())
    );
}
