// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exercise native Turtle parsing through its public extended entry point.

use purrdf_core::DatasetView;
use purrdf_rdf::{ParseOptions, parse_dataset_with};

fn main() -> Result<(), purrdf_rdf::RdfDiagnostic> {
    let source = b"@prefix ex: <https://example.org/> .\nex:s ex:p \"value\" .\n";
    let parsed = parse_dataset_with(
        source,
        "text/turtle",
        None,
        &ParseOptions {
            track_source_spans: true,
            yaml_ld: None,
        },
    )?;
    assert_eq!(parsed.dataset.quad_count(), 1);
    assert_eq!(parsed.document_prefixes.len(), 1);
    assert_eq!(
        parsed
            .spans
            .as_ref()
            .expect("requested source positions")
            .len(),
        1
    );
    let source_origins = parsed
        .spans
        .as_ref()
        .expect("requested source positions")
        .source_occurrences();
    let roles: Vec<_> = source_origins
        .iter()
        .map(purrdf_rdf::SourceOccurrence::role)
        .collect();
    assert_eq!(
        roles,
        [
            purrdf_rdf::SourceRole::Subject,
            purrdf_rdf::SourceRole::Predicate,
            purrdf_rdf::SourceRole::Object
        ]
    );
    for (occurrence, expected) in source_origins.iter().zip([
        b"ex:s".as_slice(),
        b"ex:p".as_slice(),
        b"\"value\"".as_slice(),
    ]) {
        let origin = source_origins
            .origin(occurrence.origin())
            .expect("native origin");
        let purrdf_rdf::SourceOriginKind::Written(range) = origin.kind() else {
            panic!("written source term")
        };
        assert_eq!(&source[range.clone()], expected);
        assert!(matches!(
            origin.semantic(),
            Some(purrdf_rdf::SourceSemanticReference::Term(_))
        ));
    }

    // The caller's typed operational failure needs no formatting bound.
    struct CallerFailure;
    let controlled = purrdf_rdf::parse_dataset_controlled(
        source,
        purrdf_rdf::NativeRdfFormat::Turtle,
        None,
        purrdf_lex::allocation::Resident,
        |_| Ok::<_, CallerFailure>(()),
    )
    .expect("controlled public parser");
    let view = controlled.dataset().expect("successful native dataset");
    assert!(view.quads().eq(parsed.dataset.quads()));
    assert_eq!(controlled.source().bytes(), source);
    assert_eq!(controlled.source().occurrences().count(), 3);
    assert_eq!(
        controlled.document_prefixes().collect::<Vec<_>>(),
        [("ex", "https://example.org/")]
    );
    Ok(())
}
