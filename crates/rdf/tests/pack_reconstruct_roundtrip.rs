// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reconstruction round-trip: the public, `DatasetView`-generic reconstructor
//! [`dataset_from_view`] materializes a concrete [`Arc<RdfDataset>`] from a mmap'd
//! [`PackView`], and that reconstruction is IDENTICAL to the source dataset the pack
//! was built from — isomorphic by RDFC-1.0 digest AND byte-identical when serialized
//! to a star-capable format.
//!
//! This is the ingress twin of the serializer parity guard in
//! `serialize_pack_parity.rs` and the query parity guard in
//! `crates/sparql-eval/tests/pack_query_e2e.rs`: it proves a caller holding only a
//! read-only pack projection can recover a full `RdfDataset` (to feed a transform
//! that needs one, e.g. the reasoner) with zero loss — every base quad, every RDF-1.2
//! reifier binding, and every default- and graph-scoped annotation survive the round
//! trip. The same rich fixture (`example.org` only) exercises every seam the pack
//! codec claims to unify.

use purrdf_core::{PackBuilder, PackView, dataset_from_view, datasets_isomorphic, restore_pack};
use purrdf_rdf::{NativeRdfFormat, serialize_dataset_to_format};

mod common;
use common::build_fixture;

/// The headline: reconstruct the rich fixture straight off its `PackView` and prove the
/// result is the same dataset — isomorphic by RDFC-1.0 digest AND byte-identical in a
/// star-capable serialization.
#[test]
fn dataset_from_view_roundtrips_the_full_fixture() {
    let ds = build_fixture();

    // Build the pack, open a read-only view over it, and reconstruct a concrete dataset.
    let bytes = PackBuilder::build_bytes(&ds).expect("pack build");
    let view = PackView::from_bytes(&bytes).expect("pack opens");
    let rebuilt = dataset_from_view(&view).expect("reconstruct from view");

    // 1) Direct dataset isomorphism (RDFC-1.0 blank-node canonicalization under the hood).
    assert!(
        datasets_isomorphic(&rebuilt, &ds), // deref-coerces Arc<RdfDataset> -> &RdfDataset
        "reconstruction must be isomorphic to the source dataset"
    );

    // 2) Isomorphism by RDFC-1.0 content digest: build a pack from the reconstruction and
    //    compare its stored RDFC-1.0 digest to the source pack's. Equal digests certify
    //    the two datasets canonicalize identically (base quads + reifier + annotations).
    let rebuilt_bytes = PackBuilder::build_bytes(&rebuilt).expect("pack build from reconstruction");
    let rebuilt_view = PackView::from_bytes(&rebuilt_bytes).expect("rebuilt pack opens");
    assert_eq!(
        view.rdfc_digest(),
        rebuilt_view.rdfc_digest(),
        "reconstruction's RDFC-1.0 digest must equal the source pack's"
    );

    // 3) Byte-identical N-Quads (a star-capable format: the triple-term-as-object base
    //    quad and the reifier/annotation side-tables all serialize losslessly).
    let rebuilt_nq = serialize_dataset_to_format(&*rebuilt, NativeRdfFormat::NQuads, None)
        .expect("serialize reconstruction to NQuads")
        .bytes;
    let source_nq = serialize_dataset_to_format(&*ds, NativeRdfFormat::NQuads, None)
        .expect("serialize source to NQuads")
        .bytes;
    assert_eq!(
        rebuilt_nq, source_nq,
        "reconstruction must serialize byte-identically to the source in NQuads"
    );
    assert!(
        !source_nq.is_empty(),
        "non-vacuous: the fixture is non-empty"
    );
}

/// The byte-oriented convenience surface used by persistent caches must restore
/// the same complete RDF 1.2 dataset as the generic view reconstruction.
#[test]
fn restore_pack_roundtrips_the_full_fixture() {
    let source = build_fixture();
    let bytes = PackBuilder::build_bytes(&source).expect("pack build");
    let restored = restore_pack(&bytes).expect("pack restores");

    assert!(
        datasets_isomorphic(&restored, &source),
        "restore_pack must preserve the source dataset"
    );
    assert_eq!(restored.reifiers().count(), source.reifiers().count());
    assert_eq!(restored.annotations().count(), source.annotations().count());
}
