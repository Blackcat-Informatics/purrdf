// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared whole-import fixture, allocation window and RDF 1.2 oracle.

use purrdf_alloc_probe::{CurrentThreadWindow, Measurement};
use purrdf_gts::writer::Writer;
use purrdf_lex::cbor::Value;
use purrdf_rdf::{GtsBundle, SerializeGraph, import_gts_events, serialize_dataset};

pub(super) fn source(repeats: usize, snapshot: bool) -> Vec<u8> {
    let mut writer = Writer::new("rdf12");
    writer.add_frame(
        "terms",
        Some(Value::Array(vec![Value::Map(vec![
            ("k".into(), 0.into()),
            ("v".into(), "https://example.org/prefix".into()),
        ])])),
        None,
        None,
        None,
    );
    // A snapshot restarts its dictionary at zero; ordinary rows use the outer
    // dictionary, which already contains the prefix term above.
    let base = usize::from(!snapshot);
    let id = |local: usize| Value::from(local + base);
    let mut terms: Vec<_> = ["s", "p", "o", "g", "datatype"]
        .into_iter()
        .map(|name| {
            Value::Map(vec![
                ("k".into(), 0.into()),
                ("v".into(), format!("https://example.org/{name}").into()),
            ])
        })
        .collect();
    terms.extend([
        Value::Map(vec![
            ("k".into(), 1.into()),
            ("v".into(), "typed".into()),
            ("dt".into(), id(4)),
        ]),
        Value::Map(vec![
            ("k".into(), 3.into()),
            ("rf".into(), id(6)),
            ("tt".into(), Value::Array(vec![id(0), id(1), id(5)])),
        ]),
        Value::Map(vec![("k".into(), 2.into()), ("v".into(), "b".into())]),
        Value::Map(vec![
            ("k".into(), 1.into()),
            ("v".into(), "directional".into()),
            ("l".into(), "en".into()),
            ("dir".into(), "rtl".into()),
        ]),
    ]);
    let rows = [
        Value::Array(vec![id(7), id(1), id(6), id(3)]),
        Value::Array(vec![id(0), id(1), id(8)]),
        Value::Array(vec![id(0), id(1), id(5)]),
    ];
    let payloads = vec![
        ("terms", Value::Array(terms)),
        (
            "quads",
            Value::Array((0..repeats).flat_map(|_| rows.clone()).collect()),
        ),
        (
            "reifies",
            Value::Array(vec![Value::Array(vec![id(7), id(0), id(1), id(5), id(3)])]),
        ),
        (
            "annot",
            Value::Array(vec![Value::Array(vec![id(7), id(1), id(8), id(3)])]),
        ),
    ];
    if snapshot {
        writer.add_frame(
            "snapshot",
            Some(Value::Map(
                payloads
                    .into_iter()
                    .map(|(key, value)| (key.into(), value))
                    .collect(),
            )),
            None,
            None,
            None,
        );
    } else {
        for (kind, payload) in payloads {
            writer.add_frame(kind, Some(payload), None, None, None);
        }
    }
    writer.into_bytes()
}

/// Source construction precedes this call; the complete returned RDF bundle is
/// alive when the current-thread window closes. The reader/resolver/builder
/// path exercised here is sequential. This is allocator demand, not process RSS.
pub(super) fn observe(bytes: &[u8]) -> (GtsBundle, Measurement) {
    let window = CurrentThreadWindow::open();
    let bundle = import_gts_events(bytes).expect("valid RDF 1.2 row frame");
    let measured = window.close();
    (bundle, measured)
}

/// Check the complete emitted RDF outside the allocation window, including
/// rebased triple/datatype/graph references and both statement-layer tables.
pub(super) fn check_content(bundle: &GtsBundle, expected: &mut Option<String>) {
    assert_eq!(bundle.dataset.quad_count(), 3);
    assert_eq!(bundle.dataset.owned_reifiers().count(), 1);
    assert_eq!(bundle.dataset.owned_annotations().count(), 1);
    let rendered = String::from_utf8(
        serialize_dataset(
            &*bundle.dataset,
            "application/n-quads",
            SerializeGraph::Dataset,
        )
        .expect("render the imported RDF 1.2 content"),
    )
    .expect("N-Quads is UTF-8");
    assert!(rendered.contains("<<( <https://example.org/s> <https://example.org/p> \"typed\"^^<https://example.org/datatype> )>>"));
    assert!(rendered.contains("\"directional\"@en--rtl"));
    assert!(rendered.contains("<https://example.org/g>"));
    if let Some(expected) = expected.as_ref() {
        assert_eq!(
            &rendered, expected,
            "row repetition and snapshot rebasing preserve all RDF content"
        );
    } else {
        *expected = Some(rendered);
    }
}
