// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Byte-determinism gate for content-addressed terms.
//!
//! Content addressing adds three DERIVED, non-serialized structures to a frozen
//! [`RdfDataset`]: the `content_ids` side table (interned-IRI → digest), the
//! resolved derivation-predicate `TermId`, and the lazy predecessor index. None
//! of them is part of the RDF term/quad/reifier/annotation content, so enabling
//! recognition MUST NOT perturb any serializer's output.
//!
//! This test builds the SAME logical dataset two ways — plain
//! ([`RdfDatasetBuilder::new`]) and content-addressing active
//! ([`RdfDatasetBuilder::with_content_addressing`]) — serializes both through the
//! canonical RDFC-1.0 N-Quads surface ([`canonical_flat_nquads`]), and asserts the
//! bytes are identical. To prove determinism DESPITE recognition being live (not
//! because it silently no-op'd), it also asserts the content-addressing dataset
//! actually recognized its `blake3:<64hex>` terms.

use std::sync::Arc;

use purrdf_core::ContentIdScheme;
use purrdf_rdf::gts_compose::SnapshotBuilder;
use purrdf_rdf::{
    CompositeDatasetView, CompositeSource, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId,
    ViewLimits, canonical_flat_nquads, parse_dataset,
};

/// The caller-supplied derivation-predicate IRI (no fabricated vocabulary: this is
/// configuration, spelled under `example.org` per the test-fixture rule).
const DERIVED_FROM: &str = "http://example.org/wasDerivedFrom";

/// A `blake3:`-scheme content-id IRI whose 64-hex tail is the two-hex-digit
/// `pair` repeated 32× (e.g. `"aa"` → `blake3:aaaa…aa`, 64 hex chars).
fn blake3_iri(pair: &str) -> String {
    format!("blake3:{}", pair.repeat(32))
}

/// The `TermId`s of the two content-addressed IRIs, captured at build time so the
/// caller can probe the frozen side table.
struct CaIds {
    subject: TermId,
    object: TermId,
}

/// Intern one fixed logical dataset into `builder` and return the two
/// content-addressed term ids. Identical push sequence for both builders, so any
/// serialized difference can only come from the content-addressing config itself.
///
/// Content exercised:
/// - a `blake3:<64hex>` IRI in SUBJECT position,
/// - a different `blake3:<64hex>` IRI in OBJECT position,
/// - an ordinary `example.org` quad, and
/// - a derivation annotation `(ca_subject, wasDerivedFrom, ca_object)`.
fn populate(builder: &mut RdfDatasetBuilder) -> CaIds {
    let ca_subject = builder.intern_iri(&blake3_iri("aa"));
    let ca_object = builder.intern_iri(&blake3_iri("bb"));
    let plain_subject = builder.intern_iri("http://example.org/thing");
    let predicate = builder.intern_iri("http://example.org/p");
    let derived_from = builder.intern_iri(DERIVED_FROM);
    let label = builder.intern_literal(RdfLiteral::simple("content-addressed"));

    // A content-addressed IRI as the subject of an asserted quad.
    builder.push_quad(ca_subject, predicate, label, None);
    // A content-addressed IRI as the object of an asserted quad.
    builder.push_quad(plain_subject, predicate, ca_object, None);
    // A derivation annotation linking the two content-addressed terms.
    builder.push_annotation(ca_subject, derived_from, ca_object);

    CaIds {
        subject: ca_subject,
        object: ca_object,
    }
}

/// Enabling content addressing must not change one canonical byte of serialized
/// output, because the recognition state is a derived side table — never content.
#[test]
fn content_addressing_does_not_perturb_serialized_bytes() {
    // (A) Plain builder — content addressing inactive.
    let mut plain = RdfDatasetBuilder::new();
    let _ = populate(&mut plain);
    let dataset_a = plain.freeze().expect("plain dataset freezes");

    // (B) Content-addressing active, with the derivation predicate configured.
    let scheme = ContentIdScheme::new("blake3:").expect("':' is not a hex digit");
    let mut addressed =
        RdfDatasetBuilder::with_content_addressing(scheme, Some(DERIVED_FROM.to_string()));
    let ca = populate(&mut addressed);
    let dataset_b = addressed
        .freeze()
        .expect("content-addressed dataset freezes");

    // Sanity: recognition was ACTIVE — the blake3 terms landed in the side table.
    // (Without this, the byte-equality below could pass trivially if recognition
    // had silently done nothing.)
    assert!(
        dataset_b.content_id(ca.subject).is_some(),
        "the blake3 subject IRI must be recognized as a content-id term"
    );
    assert!(
        dataset_b.content_id(ca.object).is_some(),
        "the blake3 object IRI must be recognized as a content-id term"
    );
    // The plain dataset, by contrast, has an empty side table.
    assert_eq!(
        dataset_a.content_ids().count(),
        0,
        "the plain dataset must not recognize any content-id term"
    );
    assert_eq!(
        dataset_b.content_ids().count(),
        2,
        "exactly the two blake3 IRIs are content-addressed"
    );

    // The load-bearing assertion: identical logical content → identical bytes,
    // regardless of the (non-serialized) content-addressing side tables.
    let bytes_a = canonical_flat_nquads(&dataset_a).expect("canonicalize plain");
    let bytes_b = canonical_flat_nquads(&dataset_b).expect("canonicalize addressed");
    assert_eq!(
        bytes_a.as_bytes(),
        bytes_b.as_bytes(),
        "content-addressing changed the serialized bytes:\n--- plain ---\n{bytes_a}\n--- addressed ---\n{bytes_b}"
    );
}

/// A single-source composite view over `dataset`, in an explicitly SHARED blank
/// identity space so the view reports the source's own `(label, scope)` pairs.
fn composite_over(dataset: &Arc<RdfDataset>) -> CompositeDatasetView {
    CompositeDatasetView::from_shared_sources(
        vec![CompositeSource::new(Arc::clone(dataset))],
        ViewLimits::default(),
    )
    .expect("a single retained source composes")
}

/// LITERAL NORMALIZATION IS A BYTE TRAP, and the two GTS ingestion surfaces must
/// fall into it identically.
///
/// A view's `TermRef::Literal` ALWAYS carries a datatype term id, while the flat
/// snapshot tables store a language-tagged literal with NO datatype and fold
/// `xsd:string` away entirely. So the view path has to DROP three datatype IRIs —
/// `xsd:string`, `rdf:langString`, `rdf:dirLangString` — and, because it drops
/// them, must never intern them either. Interning even one adds a dictionary row,
/// which re-ids every term after it and moves `snapshot_content_id`.
#[test]
fn the_view_ingestion_path_drops_the_same_datatype_iris_as_the_flat_path() {
    let source = parse_dataset(
        concat!(
            "<http://example.org/s> <http://example.org/a> \"bare\" .\n",
            "<http://example.org/s> <http://example.org/b> ",
            "\"x\"^^<http://www.w3.org/2001/XMLSchema#string> .\n",
            "<http://example.org/s> <http://example.org/c> \"cat\"@en .\n",
            "<http://example.org/s> <http://example.org/d> \"مرحبا\"@ar--rtl .\n",
            // A datatype that is NOT implicit must still be interned, or the test
            // would pass by dropping everything.
            "<http://example.org/s> <http://example.org/e> ",
            "\"7\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        )
        .as_bytes(),
        "application/n-triples",
        None,
    )
    .expect("the literal spread parses");

    let mut flat = SnapshotBuilder::new();
    flat.add_dataset(&source).expect("flat ingests");
    let mut view = SnapshotBuilder::new();
    let _ = view
        .add_view(&composite_over(&source))
        .expect("the view path ingests");

    assert_eq!(
        flat.snapshot_content_id(),
        view.snapshot_content_id(),
        "the two ingestion surfaces disagreed on the literal traps"
    );
    assert_eq!(flat.snapshot_payload(), view.snapshot_payload());

    let rendered = format!("{:?}", view.snapshot_payload());
    for dropped in [
        "XMLSchema#string",
        "22-rdf-syntax-ns#langString",
        "22-rdf-syntax-ns#dirLangString",
    ] {
        assert!(
            !rendered.contains(dropped),
            "the implicit datatype {dropped:?} must not reach the term table: {rendered}"
        );
    }
    // NON-VACUITY: an explicit, non-implicit datatype IS interned, so the
    // assertions above are about normalization rather than about a term table
    // that simply holds no datatypes.
    assert!(
        rendered.contains("XMLSchema#integer"),
        "an explicit datatype must still be interned: {rendered}"
    );
    assert!(
        rendered.contains("rtl"),
        "the directional literal keeps its base direction: {rendered}"
    );
}
