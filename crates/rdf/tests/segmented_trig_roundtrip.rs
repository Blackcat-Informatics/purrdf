// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native codec interoperability over real persistent dictionary guards and eviction.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, CanonHash, DatasetView, GlobalTermId, QuadIds, RdfTextDirection,
    SegmentedBuildLimits, SegmentedBuilder, SegmentedReadLimits, SegmentedSession, TermValue,
    try_canonicalize_flat_view,
};
use purrdf_rdf::{NativeRdfFormat, parse_dataset, serialize_dataset_to_format};

fn semantic_values() -> Vec<TermValue> {
    let blank = TermValue::Blank {
        label: "shared".to_owned(),
        scope: BlankScope(7),
    };
    let directional = TermValue::Literal {
        lexical_form: "مرحبا 漢字".to_owned(),
        datatype: purrdf_iri::vocab::rdf::DIR_LANG_STRING.to_owned(),
        language: Some("ar".to_owned()),
        direction: Some(RdfTextDirection::Rtl),
    };
    vec![
        TermValue::iri("https://example.org/s"),
        TermValue::iri("https://example.org/p"),
        blank.clone(),
        directional.clone(),
        TermValue::Triple {
            s: blank.into(),
            p: TermValue::iri("https://example.org/p").into(),
            o: directional.into(),
        },
        TermValue::iri("https://example.org/reifier"),
        TermValue::iri("https://example.org/graph"),
        TermValue::iri("https://example.org/empty"),
        TermValue::iri("https://example.org/note"),
    ]
}

#[test]
fn persistent_trig_roundtrip_preserves_rdf12_streams_and_empty_graphs() {
    let limits = SegmentedBuildLimits::new(4096, 200_000, 4096, 512, 4)
        .unwrap()
        .with_first_term_index((1_u64 << 53) + 1)
        .unwrap();
    let mut builder = SegmentedBuilder::new(limits);
    let mut ids: Vec<GlobalTermId> = Vec::new();
    for (index, value) in semantic_values().iter().enumerate() {
        builder
            .intern_batch(std::slice::from_ref(value), |_, id| ids.push(id))
            .unwrap();
        // Separate the referenced terms into distinct physical blocks so a two-slot
        // reader must reopen blocks while it writes nested terms and side tables.
        for filler in 0..30 {
            builder
                .intern_batch(
                    &[TermValue::iri(format!(
                        "https://example.org/unused/{index}/{filler:04}/中文"
                    ))],
                    |_, _| {},
                )
                .unwrap();
        }
    }
    builder
        .push_quad(QuadIds {
            s: ids[0],
            p: ids[1],
            o: ids[2],
            g: Some(ids[6]),
        })
        .unwrap();
    builder
        .push_quad(QuadIds {
            s: ids[2],
            p: ids[1],
            o: ids[4],
            g: None,
        })
        .unwrap();
    builder.push_reifier(ids[5], ids[4], Some(ids[6])).unwrap();
    builder
        .push_annotation(QuadIds {
            s: ids[5],
            p: ids[8],
            o: ids[3],
            g: Some(ids[6]),
        })
        .unwrap();
    builder.declare_named_graph(ids[7]).unwrap();
    let image = builder.seal().unwrap();
    let open = || {
        SegmentedSession::open(
            Arc::new(image.provider()),
            image.receipt(),
            SegmentedReadLimits::new(2_000_000, 2, 50_000, 100_000_000, 16),
        )
        .unwrap()
    };
    let streamed = open();
    let mut bytes = Vec::new();
    streamed
        .export_trig_lines(&mut purrdf_core::sink::WriterDrain(&mut bytes))
        .unwrap();
    assert!(streamed.evidence().evictions() > 0);
    let canonical = try_canonicalize_flat_view(&streamed, CanonHash::Sha256)
        .unwrap()
        .nquads;
    let parsed = parse_dataset(&bytes, "application/trig", None).unwrap();
    assert_eq!(parsed.quad_count(), 2);
    assert_eq!(parsed.reifier_quads().count(), 1);
    assert_eq!(parsed.annotation_quads().count(), 1);
    assert_eq!(parsed.named_graphs().count(), 2);
    let empty = parsed
        .as_ref()
        .term_id_by_value(&TermValue::iri("https://example.org/empty"))
        .unwrap();
    assert!(parsed.quads().all(|quad| quad.g != Some(empty)));
    assert_eq!(
        try_canonicalize_flat_view(parsed.as_ref(), CanonHash::Sha256)
            .unwrap()
            .nquads,
        canonical
    );
    let native = serialize_dataset_to_format(&open(), NativeRdfFormat::TriG, None).unwrap();
    let reparsed = parse_dataset(&native.bytes, "application/trig", None).unwrap();
    assert_eq!(reparsed.named_graphs().count(), 2);
    assert_eq!(
        try_canonicalize_flat_view(reparsed.as_ref(), CanonHash::Sha256)
            .unwrap()
            .nquads,
        canonical
    );
    let reexport =
        serialize_dataset_to_format(parsed.as_ref(), NativeRdfFormat::TriG, None).unwrap();
    let final_read = parse_dataset(&reexport.bytes, "application/trig", None).unwrap();
    assert_eq!(final_read.named_graphs().count(), 2);
    assert_eq!(
        try_canonicalize_flat_view(final_read.as_ref(), CanonHash::Sha256)
            .unwrap()
            .nquads,
        canonical
    );
}
