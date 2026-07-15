// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Serializer parity across `DatasetView` backends: the native RDF serializer
//! ([`serialize_dataset_to_format`]) is generic over any [`DatasetView`], so the SAME
//! rich fixture serialized through the production [`RdfDataset`] and through a
//! [`PackView`] opened over that dataset's pack bytes must be BYTE-IDENTICAL for every
//! [`NativeRdfFormat`] — and report the same star-layer drop count.
//!
//! This is the egress twin of the query-side parity guard in
//! `crates/sparql-eval/tests/pack_query_e2e.rs`: it proves a `PackView` (or any
//! `DatasetView`) serializes straight to RDF text with zero materialization and no
//! behavioral divergence from the source dataset. Byte-identity is the strong claim —
//! it certifies that the pack codec preserves the quad/term/side-table iteration order
//! the serializer folds into its deterministic output.

use purrdf_core::{DatasetView, PackBuilder, PackView};
use purrdf_rdf::{NativeRdfFormat, serialize_dataset_to_format};

mod common;
use common::build_fixture;

/// Every native RDF format the serializer targets.
const ALL_FORMATS: &[NativeRdfFormat] = &[
    NativeRdfFormat::Turtle,
    NativeRdfFormat::TriG,
    NativeRdfFormat::NTriples,
    NativeRdfFormat::NQuads,
    NativeRdfFormat::RdfXml,
    NativeRdfFormat::TriX,
    NativeRdfFormat::HexTuples,
    NativeRdfFormat::JsonLd,
    NativeRdfFormat::YamlLd,
];

/// Whether `fmt` is a classic star-incapable quad syntax with NO RDF-1.2 triple-term
/// surface at all: it cannot serialize the fixture's triple-term-as-object base quad
/// (`meta statesFact <<( alice knows bob )>>`) and, by deliberate design, HARD-errors
/// on it rather than dropping it silently (see `native_codecs::trix` /
/// `native_codecs::hextuples`). Every other format either carries the star layer or,
/// like RDF/XML, has a triple-term surface for the base quad while dropping only the
/// reifier/annotation statement layer.
fn is_classic_no_triple_term_surface(fmt: NativeRdfFormat) -> bool {
    matches!(fmt, NativeRdfFormat::TriX | NativeRdfFormat::HexTuples)
}

#[test]
fn pack_view_serializes_identically_to_source_dataset() {
    let ds = build_fixture();
    let bytes = PackBuilder::build_bytes(&ds).expect("pack build");
    let view = PackView::from_bytes(&bytes).expect("pack opens");

    // Sanity: the fixture actually carries a star layer, so the parity check genuinely
    // exercises the reifier/annotation side tables (a check over star-free data would
    // be vacuous for the star-drop accounting).
    assert!(view.reifier_quads().count() >= 1, "fixture has a reifier");
    assert!(
        view.annotation_quads().count() >= 2,
        "fixture has two annotations"
    );

    // At least one format must succeed with real bytes, so the whole matrix is never a
    // vacuous "every backend errored identically" pass.
    let mut succeeded = 0usize;

    for &fmt in ALL_FORMATS {
        let from_source = serialize_dataset_to_format(&*ds, fmt, None);
        let from_pack = serialize_dataset_to_format(&view, fmt, None);

        match (from_source, from_pack) {
            (Ok(source), Ok(pack)) => {
                // The `PackView` (id space `PackId`) and the source `RdfDataset` (id
                // space `TermId`) intern terms in different orders, so byte-identity
                // here certifies the serializer's canonical, value-based ordering is
                // fully backend-independent.
                assert!(
                    !source.bytes.is_empty(),
                    "{fmt:?}: source serialization must be non-vacuous"
                );
                assert!(
                    !pack.bytes.is_empty(),
                    "{fmt:?}: pack serialization must be non-vacuous"
                );
                assert_eq!(
                    source.bytes, pack.bytes,
                    "{fmt:?}: PackView serialization must be BYTE-IDENTICAL to the source RdfDataset"
                );
                assert_eq!(
                    source.statement_rows_dropped, pack.statement_rows_dropped,
                    "{fmt:?}: star-layer drop count must match across backends"
                );
                assert!(
                    !is_classic_no_triple_term_surface(fmt),
                    "{fmt:?}: a classic no-triple-term format must NOT serialize the \
                     triple-term-as-object fixture — it is expected to hard-error"
                );
                succeeded += 1;
            }
            (Err(source), Err(pack)) => {
                // Parity holds on the error path too: a format that cannot represent the
                // fixture must fail IDENTICALLY on both backends (same code + message),
                // never diverge (one erroring while the other emits partial output).
                assert!(
                    is_classic_no_triple_term_surface(fmt),
                    "{fmt:?}: unexpected serialize error on both backends: {}",
                    source.message
                );
                assert_eq!(
                    source.code, pack.code,
                    "{fmt:?}: error code must match across backends"
                );
                assert_eq!(
                    source.message, pack.message,
                    "{fmt:?}: error message must match across backends"
                );
            }
            (source, pack) => panic!(
                "{fmt:?}: PackView and source diverge (one Ok, one Err): source={source:?} pack={pack:?}"
            ),
        }
    }

    assert!(
        succeeded >= 7,
        "expected the star-capable + RDF/XML formats to serialize; only {succeeded} did"
    );
}
