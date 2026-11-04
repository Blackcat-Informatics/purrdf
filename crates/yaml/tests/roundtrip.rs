// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native presentation, transport, closed metadata and phase-bound acceptance.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTerm};
use purrdf_yaml::{
    Bounds, Profile, SourceDocument, Vocabulary, YamlError, analyze, decode_stream, project,
};

const SOURCE: &str = "https://example.org/source.yaml";
fn profile(bounds: Bounds) -> Profile {
    Profile::new("source-test", 1, Vocabulary::standard().unwrap(), bounds).unwrap()
}
fn model<'a>(text: &'a str, profile: &Profile) -> purrdf_yaml::Document<'a> {
    analyze(
        SourceDocument {
            id: SOURCE,
            bytes: text.as_bytes(),
        },
        profile,
    )
    .unwrap()
}
fn specimen() -> String {
    concat!(
        "\u{feff}# original notice\n%YAML 1.2\n%TAG !e! tag:example.org,2026:\n---\n",
        "? [complex, key]\n: &a !e!record {text: 'it''s', flow: [é, .inf], empty:}\n",
        "copy: *a\nshadow: [&a old, *a, &a [&a inner, *a], *a]\n",
        "duplicate: first\nduplicate: second\nnonSpecific: ! plain\n",
        "literal: |2- # header\n  # body content\n  text\n",
        "folded: >+\n  one\n  two\n\n",
        "clip: |\n  final\nescaped: \"x\\n\\u00E9\\\n  joined\" # tail\n",
        "...\n---\n...\n---\n- one\n- two\n",
    )
    .replace('\n', "\r\n")
}

fn presentation_round_trips_through_rdf_with_deterministic_identities() {
    let profile = Profile::standard().unwrap();
    let specimen = specimen();
    for text in [
        specimen.as_str(),
        "",
        "# only comment\r\n",
        "---\r\n...\r\n",
    ] {
        let document = model(text, &profile);
        let graph = project(&document, &profile).unwrap();
        assert_eq!(
            decode_stream(&graph, document.id(), &profile).unwrap(),
            text
        );
        let transport = purrdf_rdf::serialize_dataset_to_format(
            &graph,
            purrdf_rdf::NativeRdfFormat::Turtle,
            None,
        )
        .unwrap();
        let decoded = purrdf_rdf::parse_dataset(&transport.bytes, "text/turtle", None).unwrap();
        assert_eq!(
            decode_stream(&decoded, document.id(), &profile).unwrap(),
            text
        );
        let again = model(text, &profile);
        assert_eq!(document.id(), again.id());
        let repeated = project(&again, &profile).unwrap();
        assert_eq!(
            purrdf_rdf::serialize_dataset(
                &graph,
                "application/n-triples",
                purrdf_rdf::SerializeGraph::Dataset
            )
            .unwrap(),
            purrdf_rdf::serialize_dataset(
                &repeated,
                "application/n-triples",
                purrdf_rdf::SerializeGraph::Dataset
            )
            .unwrap()
        );
        assert_eq!(
            document.source_digest(),
            purrdf_core::ContentDigest::of(text.as_bytes())
        );
        let mut frontier = 0;
        for span in document.cover_spans() {
            assert_eq!(span.start, frontier);
            assert!(span.end > span.start);
            frontier = span.end;
        }
        assert_eq!(frontier, text.len());
    }
    let document = model(&specimen, &profile);
    assert_eq!(document.syntax().documents().len(), 3);
    let aliases: Vec<_> = document
        .syntax()
        .nodes()
        .filter_map(|(_, node)| match node.kind {
            purrdf_yaml::syntax::NodeKind::Alias { target, .. } => Some(target),
            _ => None,
        })
        .collect();
    assert!(aliases.len() >= 4);
    assert_ne!(aliases[1], aliases[2]);
    assert_eq!(aliases[2], aliases[3]);
    let changed_text = specimen.replace("# original notice", "# changed notice");
    let changed = model(&changed_text, &profile);
    assert_ne!(document.id(), changed.id());
    let other_source = analyze(
        SourceDocument {
            id: "https://example.org/other.yaml",
            bytes: specimen.as_bytes(),
        },
        &profile,
    )
    .unwrap();
    assert_ne!(document.id(), other_source.id());
    let other_profile = Profile::new(
        "other",
        1,
        Vocabulary::standard().unwrap(),
        Bounds::standard(),
    )
    .unwrap();
    assert_ne!(document.id(), model(&specimen, &other_profile).id());
}

fn strict_selection_rejects_forged_metadata_and_cover_claims() {
    let profile = Profile::standard().unwrap();
    let source = specimen();
    let document = model(&source, &profile);
    let graph = project(&document, &profile).unwrap();
    let base = profile.vocabulary().base();
    let mut builder = RdfDatasetBuilder::new();
    let mut removed = false;
    for (index, row) in graph.quads().enumerate() {
        let statement = graph.to_owned_quad(index, row);
        if !removed && statement.predicate == profile.vocabulary().term("verbatim").unwrap() {
            removed = true;
            continue;
        }
        builder.push_owned_quad(&statement).unwrap();
    }
    assert!(removed);
    assert!(matches!(
        decode_stream(&builder.freeze().unwrap(), document.id(), &profile),
        Err(YamlError::Cover(
            purrdf_core::cover::ReconstructError::UncoveredRange { .. }
        ))
    ));
    // Every selected class is checked through complete reprojected graph equality.
    for local in ["root", "key", "target", "tagName", "attachment", "decoded"] {
        let predicate = profile.vocabulary().term(local).unwrap();
        let mut removed = false;
        let mut builder = RdfDatasetBuilder::new();
        for (index, row) in graph.quads().enumerate() {
            let statement = graph.to_owned_quad(index, row);
            if !removed && statement.predicate == predicate {
                removed = true;
                continue;
            }
            builder.push_owned_quad(&statement).unwrap();
        }
        assert!(removed, "specimen must exercise {local}");
        assert!(
            decode_stream(&builder.freeze().unwrap(), document.id(), &profile).is_err(),
            "accepted missing {local}"
        );
    }
    for local in ["verbatim", "byteStart", "profile", "sourceDigest"] {
        let predicate = profile.vocabulary().term(local).unwrap();
        let mut edited = false;
        let mut builder = RdfDatasetBuilder::new();
        for (index, row) in graph.quads().enumerate() {
            let mut statement = graph.to_owned_quad(index, row);
            if !edited && statement.predicate == predicate {
                edited = true;
                statement.object = RdfTerm::Literal(RdfLiteral::simple("forged"));
            }
            builder.push_owned_quad(&statement).unwrap();
        }
        assert!(edited);
        assert!(
            decode_stream(&builder.freeze().unwrap(), document.id(), &profile).is_err(),
            "accepted wrongly typed {local}"
        );
    }
    // Namespace subjects stay selected even with no ownership edge.
    for subject in [
        format!("{}#forged", document.id()),
        "https://example.org/owned-forgery".to_owned(),
    ] {
        let mut builder = RdfDatasetBuilder::new();
        builder.push_dataset(&graph).unwrap();
        let s = builder.intern_iri(&subject).unwrap();
        let p = builder.intern_iri(&format!("{base}unknown")).unwrap();
        let o = builder
            .intern_literal(RdfLiteral::simple("forged"))
            .unwrap();
        builder.push_quad(s, p, o, None).unwrap();
        if !subject.starts_with(document.id()) {
            let owner = builder
                .intern_iri(profile.vocabulary().term("stream").unwrap())
                .unwrap();
            let root = builder.intern_iri(document.id()).unwrap();
            builder.push_quad(s, owner, root, None).unwrap();
        }
        assert!(decode_stream(&builder.freeze().unwrap(), document.id(), &profile).is_err());
    }
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&graph).unwrap();
    let named = builder
        .intern_iri(&format!("{}#hidden-graph", document.id()))
        .unwrap();
    builder.declare_named_graph(named).unwrap();
    assert!(decode_stream(&builder.freeze().unwrap(), document.id(), &profile).is_err());
    let other = model("other: source\n", &profile);
    let other_graph = project(&other, &profile).unwrap();
    let mut combined = RdfDatasetBuilder::new();
    combined.push_dataset(&graph).unwrap();
    combined.push_dataset(&other_graph).unwrap();
    let combined = combined.freeze().unwrap();
    assert_eq!(
        decode_stream(&combined, document.id(), &profile).unwrap(),
        source
    );
    assert_eq!(
        decode_stream(&combined, other.id(), &profile).unwrap(),
        "other: source\n"
    );
    assert!(decode_stream(&combined, "https://example.org/absent", &profile).is_err());
    let wrong_profile = Profile::new(
        "wrong",
        1,
        Vocabulary::standard().unwrap(),
        Bounds::standard(),
    )
    .unwrap();
    assert!(decode_stream(&graph, document.id(), &wrong_profile).is_err());
}

fn phase_bounds_and_native_refusals_are_explicit() {
    let text = "a: [one, two]\n";
    let original = Profile::standard().unwrap();
    let document = model(text, &original);
    let mut bounds = Bounds::standard();
    bounds.max_cover_spans = document.cover_spans().len();
    assert!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes()
            },
            &profile(bounds)
        )
        .is_ok()
    );
    bounds.max_cover_spans -= 1;
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes()
            },
            &profile(bounds)
        ),
        Err(YamlError::Limit {
            resource: "cover spans",
            ..
        })
    ));
    let mut bounds = Bounds::standard();
    bounds.syntax.syntax.max_input_bytes = text.len() - 1;
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes()
            },
            &profile(bounds)
        ),
        Err(YamlError::Limit {
            resource: "source bytes",
            ..
        })
    ));
    let mut bounds = Bounds::standard();
    bounds.syntax.syntax.max_occurrences = 1;
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes()
            },
            &profile(bounds)
        ),
        Err(YamlError::Syntax(_))
    ));
    let mut bounds = Bounds::standard();
    bounds.syntax.syntax.max_depth = 0;
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes()
            },
            &profile(bounds)
        ),
        Err(YamlError::Syntax(_))
    ));
    let mut bounds = Bounds::standard();
    bounds.max_partition_bytes = 1;
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes()
            },
            &profile(bounds)
        ),
        Err(YamlError::Storage(_))
    ));
    for (resource, configure) in [
        ("storage", 0_u8),
        ("RDF statements", 1),
        ("metadata payload bytes", 2),
    ] {
        let mut bounds = Bounds::standard();
        match configure {
            0 => bounds.max_projection_bytes = 1,
            1 => bounds.max_statements = 1,
            _ => bounds.max_payload_bytes = 1,
        }
        let profile = profile(bounds);
        let document = model(text, &profile);
        let error = project(&document, &profile).unwrap_err();
        assert!(
            matches!(error, YamlError::Storage(_))
                || matches!(error, YamlError::Limit { resource: actual, .. } if actual == resource)
        );
    }
    let mut bounds = Bounds::standard();
    bounds.max_decode_bytes = 1;
    let profile = profile(bounds);
    let document = model(text, &profile);
    let graph = project(&document, &profile).unwrap();
    assert!(matches!(
        decode_stream(&graph, document.id(), &profile),
        Err(YamlError::Storage(_))
    ));
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: b"\xff"
            },
            &original
        ),
        Err(YamlError::Utf8 { at: 0 })
    ));
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: b"&a [*a]"
            },
            &original
        ),
        Err(YamlError::Syntax(_))
    ));
}

fn presentation_metadata_does_not_change_the_yaml_ld_dataset() {
    let first = "'@id': https://example.org/s\nhttps://example.org/p: value\nhttps://example.org/q: value\n";
    let second = "# authored presentation\n---\n'@id': 'https://example.org/s'\nhttps://example.org/p: &value value\nhttps://example.org/q: *value\n...\n";
    let profile = Profile::standard().unwrap();
    assert_ne!(model(first, &profile).id(), model(second, &profile).id());
    let first = purrdf_rdf::parse_dataset(first.as_bytes(), "application/ld+yaml", None).unwrap();
    let second = purrdf_rdf::parse_dataset(second.as_bytes(), "application/ld+yaml", None).unwrap();
    assert_eq!(
        purrdf_rdf::serialize_dataset(
            &first,
            "application/n-quads",
            purrdf_rdf::SerializeGraph::Dataset
        )
        .unwrap(),
        purrdf_rdf::serialize_dataset(
            &second,
            "application/n-quads",
            purrdf_rdf::SerializeGraph::Dataset
        )
        .unwrap()
    );
}

purrdf_testkit::harness_main!(
    presentation_round_trips_through_rdf_with_deterministic_identities,
    strict_selection_rejects_forged_metadata_and_cover_claims,
    phase_bounds_and_native_refusals_are_explicit,
    presentation_metadata_does_not_change_the_yaml_ld_dataset,
);
