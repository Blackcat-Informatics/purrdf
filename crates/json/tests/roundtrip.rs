// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Frozen first-party corpus and adversarial checks through production RDF codecs.

use std::sync::Arc;

use purrdf_core::{
    RdfLiteral, RdfQuad, RdfTerm,
    ir::{RdfDataset, RdfDatasetBuilder},
};
use purrdf_json::{
    Bounds, JsonError, Kind, Profile, SourceDocument, Vocabulary, analyze, decode_document, project,
};
use purrdf_rdf::{NativeRdfFormat, parse_dataset, serialize_dataset_to_format};

const SOURCE: &str = "https://example.org/doc.json";
const BASE: &str = "https://example.org/json#";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const CORPUS: [&str; 19] = [
    r#"["\uD800","\uDC00","\uD800\u0041","\uD800\uDC00"]"#,
    "null",
    "true",
    "false",
    "0",
    "-0.00e+007",
    "1E-1000000000",
    "1234567890123456789012345678901234567890",
    "\"\"",
    "[]",
    "{}",
    "[{},[],\"\",null,true,false,0]",
    "{\"a\":1,\"b\":2,\"a\":3}",
    " \r\n\t{ \"b\" : 2 , \"a\" : 1 }\t\r\n ",
    r#"{"a\/b":{"~x":["first","second"]},"a/b":[],"":{}}"#,
    r#"{"\u0061":1,"a":2,"\uD83D\uDE00":"\uD834\uDD1E"}"#,
    r#"["\"","\\","\/","\b","\f","\n","\r","\t","\u0000","\uFFFF"]"#,
    "{\"é\":\"café\",\"emoji\":\"🐈\",\"combining\":\"e\u{301}\"}",
    "[[[[{\"x\":[1,2,{\"y\":false}],\"x\":[3]}]]]]",
];

fn profile() -> Profile {
    Profile::new(
        "test-profile",
        1,
        Vocabulary::under(BASE).unwrap(),
        Bounds::standard(),
    )
    .unwrap()
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn profile_and_document_identities_match_independent_framed_sha256_vectors() {
    // Independently computed from SPEC.md's fixed-width preimage with Python's
    // struct.pack and hashlib, not by serializing the Rust model under test.
    let profile = profile();
    assert_eq!(
        profile.identity().to_hex(),
        "ad6ac631bf6b904a247cb44b80f5826c832cf1f0b61343178ca5bab3ef15607e"
    );
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: b"null",
        },
        &profile,
    )
    .unwrap();
    assert_eq!(
        model.id(),
        "https://example.org/json/document-9258280d2f9aa82db35fb3c6b2ea63b3fbcf0c6e686123890a90c0b2e5147ae3"
    );
}

fn encoded(text: &str) -> (Profile, String, Arc<RdfDataset>) {
    let profile = profile();
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: text.as_bytes(),
        },
        &profile,
    )
    .unwrap();
    let dataset = project(&model, &profile).unwrap();
    (profile, model.id().to_owned(), dataset)
}

fn mutate(dataset: &RdfDataset, mut edit: impl FnMut(&mut RdfQuad) -> bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (index, statement) in dataset.quads().enumerate() {
        let mut statement = dataset.to_owned_quad(index, statement);
        if edit(&mut statement) {
            builder.push_owned_quad(&statement);
        }
    }
    builder.freeze().unwrap()
}

fn target(statement: &RdfQuad, subject: &str, local: &str) -> bool {
    statement.subject == RdfTerm::Iri(subject.to_owned())
        && statement.predicate == format!("{BASE}{local}")
}

fn literal(value: &str) -> RdfTerm {
    RdfTerm::Literal(RdfLiteral::simple(value))
}
fn integer(value: &str) -> RdfTerm {
    RdfTerm::Literal(RdfLiteral::typed(value, XSD_INTEGER))
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn frozen_corpus_round_trips_through_real_rdf_serialization_and_reparse() {
    assert_eq!(CORPUS.len(), 19);
    let mut corpus_bytes = Vec::new();
    for text in CORPUS {
        let (profile, document, dataset) = encoded(text);
        for (format, base) in [
            (NativeRdfFormat::Turtle, None),
            (NativeRdfFormat::Turtle, Some(document.as_str())),
            (NativeRdfFormat::NTriples, None),
            (NativeRdfFormat::JsonLd, None),
        ] {
            let serialized = serialize_dataset_to_format(&*dataset, format, base).unwrap();
            corpus_bytes.extend_from_slice(&(serialized.bytes.len() as u64).to_le_bytes());
            corpus_bytes.extend_from_slice(&serialized.bytes);
            let parsed = parse_dataset(&serialized.bytes, format.media_type(), None).unwrap();
            assert_eq!(
                decode_document(&parsed, &document, &profile)
                    .unwrap()
                    .as_bytes(),
                text.as_bytes(),
                "{format:?}: {text}"
            );
            assert_eq!(
                serialize_dataset_to_format(&*dataset, format, base)
                    .unwrap()
                    .bytes,
                serialized.bytes
            );
        }
    }
    assert_eq!(
        purrdf_core::ContentDigest::of(&corpus_bytes).to_hex(),
        "580633f8ca53cbdca3045ec708a9b9acf9631ff007457fb8878ab93454eee0fc",
        "production RDF bytes must match the same native and wasm corpus golden"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn duplicate_member_occurrences_have_distinct_identity_and_parentage() {
    let profile = profile();
    let text = r#"{"x":{"a":1},"x":{"a":2},"x":[],"":{}}"#;
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: text.as_bytes(),
        },
        &profile,
    )
    .unwrap();
    let paths: Vec<_> = model
        .values()
        .iter()
        .map(purrdf_json::Value::path)
        .collect();
    assert_eq!(paths, ["", "/x", "/x/a", "/x", "/x/a", "/x", "/"]);
    assert_eq!(model.values()[2].parent(), Some(1));
    assert_eq!(model.values()[4].parent(), Some(3));
    assert_eq!(model.values()[5].kind(), Kind::Array);
    assert_eq!(model.values()[6].kind(), Kind::Object);
    assert_eq!(model.values()[5].ordinal(), 2);
    assert_eq!(
        model.text().as_ptr(),
        text.as_ptr(),
        "the source remains borrowed"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn pointers_decode_keys_and_escape_each_reference_token() {
    let profile = profile();
    let text = r#"{"a\u002Fb":{"~1":[{"\uD83D\uDE00":"value"}]},"\u0000":1}"#;
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: text.as_bytes(),
        },
        &profile,
    )
    .unwrap();
    assert_eq!(model.values()[4].path(), "/a~1b/~01/0/😀");
    assert_eq!(model.values()[5].path(), "/\0");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn lexical_strings_and_number_spellings_are_authoritative() {
    let profile = profile();
    let text = r#"["\u0061","a",-0.00e+07,0,1E9999999]"#;
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: text.as_bytes(),
        },
        &profile,
    )
    .unwrap();
    let lexicals: Vec<_> = model
        .values()
        .iter()
        .filter(|value| value.kind().is_scalar())
        .map(|value| &model.text()[value.span()])
        .collect();
    assert_eq!(lexicals, [r"\u0061", "a", "-0.00e+07", "0", "1E9999999"]);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn rejects_json_grammar_extensions_and_unrepresentable_member_names() {
    let profile = profile();
    for text in [
        "",
        " ",
        "01",
        "-01",
        "+1",
        "1.",
        ".1",
        "1e",
        "1e+",
        "NaN",
        "Infinity",
        "[1,]",
        "{\"x\":1,}",
        "{x: 1, y: 2}",
        "[/*x*/1]",
        "true false",
        "\u{FEFF}null",
        "\u{a0}null",
        "\"raw\nline\"",
        r#""\v""#,
        r#""\u123""#,
        r#"{"\uD800":0}"#,
    ] {
        assert!(
            analyze(
                SourceDocument {
                    id: SOURCE,
                    bytes: text.as_bytes()
                },
                &profile
            )
            .is_err(),
            "accepted {text:?}"
        );
    }
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: b"\xff"
            },
            &profile
        ),
        Err(JsonError::InvalidUtf8 { .. })
    ));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn source_and_vocabulary_require_real_absolute_iris() {
    let profile = profile();
    for id in [
        "relative",
        "https://exa mple.org/a",
        "http://example.org/%ZZ",
        "http://[unterminated/",
    ] {
        assert!(
            matches!(
                analyze(SourceDocument { id, bytes: b"null" }, &profile),
                Err(JsonError::Iri(_))
            ),
            "{id}"
        );
    }
    assert!(Vocabulary::under("relative#").is_err());
    assert!(Vocabulary::under("https://example.org/json").is_err());
    assert!(profile.vocabulary().term("madeUp").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn every_resource_bound_is_enforced_and_changes_identity() {
    let standard = Bounds::standard();
    let configurations = [
        (
            Bounds {
                max_source_bytes: 3,
                ..standard
            },
            "null",
        ),
        (
            Bounds {
                max_values: 2,
                ..standard
            },
            "[1,2]",
        ),
        (
            Bounds {
                max_depth: 1,
                ..standard
            },
            "[[]]",
        ),
        (
            Bounds {
                max_pointer_bytes: 2,
                ..standard
            },
            r#"{"abc":1}"#,
        ),
    ];
    let base = profile();
    for (bounds, text) in configurations {
        let limited =
            Profile::new("test-profile", 1, Vocabulary::under(BASE).unwrap(), bounds).unwrap();
        assert_ne!(base.identity(), limited.identity());
        assert!(matches!(
            analyze(
                SourceDocument {
                    id: SOURCE,
                    bytes: text.as_bytes()
                },
                &limited
            ),
            Err(JsonError::Limit { .. })
        ));
    }
    let depth = "[".repeat(128) + "0" + &"]".repeat(128);
    assert!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: depth.as_bytes()
            },
            &base
        )
        .is_ok()
    );
    let too_deep = format!("[{depth}]");
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: too_deep.as_bytes()
            },
            &base
        ),
        Err(JsonError::Limit { .. })
    ));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn impossible_profile_bounds_refuse_at_construction() {
    for bounds in [
        Bounds {
            max_depth: 129,
            ..Bounds::standard()
        },
        Bounds {
            max_source_bytes: u64::MAX,
            ..Bounds::standard()
        },
        Bounds {
            max_values: 0,
            ..Bounds::standard()
        },
        Bounds {
            max_pointer_bytes: 0,
            ..Bounds::standard()
        },
    ] {
        assert!(Profile::new("test", 1, Vocabulary::under(BASE).unwrap(), bounds).is_err());
    }
    assert!(Profile::new("", 1, Vocabulary::under(BASE).unwrap(), Bounds::standard()).is_err());
    assert!(
        Profile::new(
            "test",
            0,
            Vocabulary::under(BASE).unwrap(),
            Bounds::standard()
        )
        .is_err()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn profile_mismatch_fails_before_projection_or_decode() {
    let (profile, document, dataset) = encoded("null");
    let other = Profile::new(
        "different",
        1,
        profile.vocabulary().clone(),
        profile.bounds(),
    )
    .unwrap();
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: b"null",
        },
        &profile,
    )
    .unwrap();
    assert!(matches!(
        project(&model, &other),
        Err(JsonError::ProfileMismatch)
    ));
    assert!(matches!(
        decode_document(&dataset, &document, &other),
        Err(JsonError::ProfileMismatch)
    ));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn explicit_selection_isolates_source_revisions_in_a_merged_dataset() {
    let (profile, first, dataset) = encoded(r#"{"x":1}"#);
    let second = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: b"[2]",
        },
        &profile,
    )
    .unwrap();
    let mut builder = RdfDatasetBuilder::new();
    assert_ne!(first, second.id());
    builder.push_dataset(&dataset);
    builder.push_dataset(&project(&second, &profile).unwrap());
    let merged = builder.freeze().unwrap();
    assert_eq!(
        decode_document(&merged, &first, &profile).unwrap(),
        r#"{"x":1}"#
    );
    assert_eq!(
        decode_document(&merged, second.id(), &profile).unwrap(),
        "[2]"
    );
    assert!(decode_document(&merged, "https://example.org/missing", &profile).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn every_required_metadata_fact_is_required() {
    let (profile, document, dataset) = encoded(r#"{"x":[1,"",{}]}"#);
    for (removed, _) in dataset.quads().enumerate() {
        let mut index = 0;
        let damaged = mutate(&dataset, |_| {
            let keep = index != removed;
            index += 1;
            keep
        });
        assert!(
            decode_document(&damaged, &document, &profile).is_err(),
            "removed statement {removed} was accepted"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn cover_success_cannot_launder_fabricated_structural_metadata() {
    let (profile, document, dataset) = encoded(r#"{"x":[1,2]}"#);
    let leaf = format!("{document}#v2");
    for (field, replacement) in [
        ("path", literal("/invented")),
        ("kind", literal("string")),
        ("ordinal", integer("1")),
        ("occurrence", integer("999")),
        ("parent", RdfTerm::Iri(document.clone())),
    ] {
        let damaged = mutate(&dataset, |statement| {
            if target(statement, &leaf, field) {
                statement.object = replacement.clone();
            }
            true
        });
        assert!(
            decode_document(&damaged, &document, &profile).is_err(),
            "accepted fake {field}"
        );
    }
    let container = format!("{document}#v1");
    let damaged = mutate(&dataset, |statement| {
        if target(statement, &container, "byteEnd") {
            statement.object = integer("0");
        }
        true
    });
    assert!(decode_document(&damaged, &document, &profile).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn rejects_malformed_offsets_wrong_datatypes_and_language_tags() {
    let (profile, document, dataset) = encoded("[1]");
    let leaf = format!("{document}#v1");
    for replacement in [
        integer("-1"),
        integer("+1"),
        integer("01"),
        integer("1.0"),
        integer("18446744073709551616"),
        literal("1"),
        RdfTerm::Literal(RdfLiteral::language_tagged("1", "en")),
    ] {
        let damaged = mutate(&dataset, |statement| {
            if target(statement, &leaf, "byteStart") {
                statement.object = replacement.clone();
            }
            true
        });
        assert!(decode_document(&damaged, &document, &profile).is_err());
    }
    let damaged = mutate(&dataset, |statement| {
        if target(statement, &leaf, "text") {
            statement.object = RdfTerm::Literal(RdfLiteral::language_tagged("1", "en"));
        }
        true
    });
    assert!(decode_document(&damaged, &document, &profile).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn conflicting_metadata_never_uses_first_or_last_value() {
    let (profile, document, dataset) = encoded("[1]");
    let leaf = format!("{document}#v1");
    for replacement in ["2", "0"] {
        let mut builder = RdfDatasetBuilder::new();
        builder.push_dataset(&dataset);
        builder.push_owned_quad(&RdfQuad::new(
            RdfTerm::Iri(leaf.clone()),
            format!("{BASE}byteStart"),
            integer(replacement),
        ));
        assert!(decode_document(&builder.freeze().unwrap(), &document, &profile).is_err());
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn rejects_changed_ownership_named_graphs_and_unexpected_predicates() {
    let (profile, document, dataset) = encoded("[{}]");
    let empty = format!("{document}#v1");
    let owner = mutate(&dataset, |statement| {
        if target(statement, &empty, "document") {
            statement.object = RdfTerm::Iri("https://example.org/other".to_owned());
        }
        true
    });
    assert!(decode_document(&owner, &document, &profile).is_err());
    let named = mutate(&dataset, |statement| {
        statement.graph_name = Some(RdfTerm::Iri("https://example.org/graph".to_owned()));
        true
    });
    assert!(decode_document(&named, &document, &profile).is_err());
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&dataset);
    builder.push_owned_quad(&RdfQuad::new(
        RdfTerm::Iri(empty),
        "https://example.org/unexpected",
        literal("extra"),
    ));
    assert!(decode_document(&builder.freeze().unwrap(), &document, &profile).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn digest_and_profile_are_mandatory_typed_identity_metadata() {
    let (profile, document, dataset) = encoded("null");
    for (field, replacement) in [
        ("sourceDigest", literal("sha256:invalid")),
        ("profile", literal("wrong")),
        (
            "source",
            RdfTerm::Iri("https://example.org/elsewhere".to_owned()),
        ),
        ("byteLength", integer("999999999999")),
    ] {
        let damaged = mutate(&dataset, |statement| {
            if target(statement, &document, field) {
                statement.object = replacement.clone();
            }
            true
        });
        assert!(
            decode_document(&damaged, &document, &profile).is_err(),
            "accepted fake {field}"
        );
    }
    let leaf = format!("{document}#v0");
    let damaged = mutate(&dataset, |statement| {
        if target(statement, &leaf, "text") {
            statement.object = literal("true");
        }
        true
    });
    assert!(matches!(
        decode_document(&damaged, &document, &profile),
        Err(JsonError::Cover(_))
    ));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn rdf12_side_tables_cannot_hide_conflicting_metadata_or_ownership() {
    let (profile, document, dataset) = encoded("[1]");
    let leaf = format!("{document}#v1");
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&dataset);
    builder.push_owned_annotation(&purrdf_core::RdfAnnotation::new(
        RdfTerm::Iri(leaf.clone()),
        format!("{BASE}path"),
        literal("/invented"),
    ));
    assert!(decode_document(&builder.freeze().unwrap(), &document, &profile).is_err());

    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&dataset);
    builder.push_owned_annotation(&purrdf_core::RdfAnnotation::new(
        RdfTerm::Iri("https://example.org/hidden-owner".to_owned()),
        format!("{BASE}document"),
        RdfTerm::Iri(document.clone()),
    ));
    assert!(decode_document(&builder.freeze().unwrap(), &document, &profile).is_err());

    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&dataset);
    builder.push_owned_reifier(&purrdf_core::RdfReifier::new(
        RdfTerm::Iri(leaf),
        purrdf_core::RdfTriple::new(
            RdfTerm::Iri("https://example.org/s".to_owned()),
            "https://example.org/p",
            literal("o"),
        ),
    ));
    assert!(decode_document(&builder.freeze().unwrap(), &document, &profile).is_err());

    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&dataset);
    let graph = builder.intern_iri(&document);
    builder.declare_named_graph(graph);
    assert!(decode_document(&builder.freeze().unwrap(), &document, &profile).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn equivalent_rdf12_metadata_and_unrelated_statement_layers_coexist() {
    let (profile, document, dataset) = encoded("[1]");
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&dataset);
    // An identical assertion in a side table is the same RDF fact.
    builder.push_owned_annotation(&purrdf_core::RdfAnnotation::new(
        RdfTerm::Iri(format!("{document}#v1")),
        format!("{BASE}path"),
        literal("/0"),
    ));
    let unrelated = RdfTerm::Iri("https://example.org/unrelated".to_owned());
    builder.push_owned_reifier(&purrdf_core::RdfReifier::new(
        unrelated.clone(),
        purrdf_core::RdfTriple::new(
            RdfTerm::Iri(document.clone()),
            "https://example.org/quoted",
            literal("unasserted"),
        ),
    ));
    builder.push_owned_annotation(&purrdf_core::RdfAnnotation::new(
        unrelated,
        "https://example.org/note",
        literal("external"),
    ));
    assert_eq!(
        decode_document(&builder.freeze().unwrap(), &document, &profile).unwrap(),
        "[1]"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn document_profile_and_length_refuse_before_owned_subgraph_collection() {
    let (profile, document, dataset) = encoded("null");
    for (field, replacement) in [
        ("profile", literal("wrong")),
        ("byteLength", integer("999999999999")),
    ] {
        let damaged = mutate(&dataset, |statement| {
            if target(statement, &document, field) {
                statement.object = replacement.clone();
            }
            true
        });
        let mut builder = RdfDatasetBuilder::new();
        builder.push_dataset(&damaged);
        // This invalid ownership subject would fail if the decoder traversed the
        // owned subgraph before checking the document's bounded admission facts.
        builder.push_owned_quad(&RdfQuad::new(
            RdfTerm::BlankNode("invalid-owner".to_owned()),
            format!("{BASE}document"),
            RdfTerm::Iri(document.clone()),
        ));
        let error = decode_document(&builder.freeze().unwrap(), &document, &profile).unwrap_err();
        if field == "profile" {
            assert!(matches!(error, JsonError::ProfileMismatch));
        } else {
            assert!(matches!(
                error,
                JsonError::Limit {
                    resource: "source bytes",
                    ..
                }
            ));
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn sparql_traverses_parent_paths_sizes_and_distinct_raw_values() {
    use purrdf_core::{SparqlResult, TermValue};
    use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

    let text = r#"{"items":[{"a":"\u0061","a":"a"}],"empty":[]}"#;
    let (profile, document, dataset) = encoded(text);
    let serialized = serialize_dataset_to_format(&*dataset, NativeRdfFormat::Turtle, None).unwrap();
    let dataset = parse_dataset(&serialized.bytes, "text/turtle", None).unwrap();
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(
            r#"PREFIX j: <https://example.org/json#>
           SELECT ?path ?text ?size WHERE {
             ?array j:path "/items"; j:kind "array"; j:size 1 .
             ?object j:parent ?array; j:path "/items/0"; j:size ?size .
             ?value j:parent ?object; j:path ?path; j:text ?text; j:ordinal ?ordinal .
             ?empty j:path "/empty"; j:size 0 .
           } ORDER BY ?ordinal"#,
            None,
        )
        .unwrap();
    let SparqlResult::Solutions {
        variables, rows, ..
    } = engine
        .query_prepared_view(&*dataset, &prepared, &[], QueryOptions::EMPTY)
        .unwrap()
    else {
        panic!("SELECT returns solutions")
    };
    assert_eq!(variables, ["path", "text", "size"]);
    let row = |text| {
        vec![
            Some(TermValue::simple_literal("/items/0/a")),
            Some(TermValue::simple_literal(text)),
            Some(TermValue::typed_literal("2", XSD_INTEGER)),
        ]
    };
    // Query text is explicitly raw JSON lexical content. Decoding JSON escapes
    // for text search would be a different operation from this byte-cover fact.
    assert_eq!(rows, vec![row(r"\u0061"), row("a")]);
    assert_eq!(
        decode_document(&dataset, &document, &profile).unwrap(),
        text
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn fabricated_container_size_and_extra_scalar_size_are_refused() {
    let (profile, document, dataset) = encoded(r#"{"a":1,"a":2,"empty":[]}"#);
    let root_value = format!("{document}#v0");
    let corrupt = mutate(&dataset, |statement| {
        if target(statement, &root_value, "size") {
            statement.object = integer("2");
        }
        true
    });
    assert!(decode_document(&corrupt, &document, &profile).is_err());
    let scalar = format!("{document}#v1");
    let corrupt = mutate(&dataset, |statement| {
        if target(statement, &scalar, "text") {
            statement.predicate = format!("{BASE}size");
            statement.object = integer("0");
        }
        true
    });
    assert!(decode_document(&corrupt, &document, &profile).is_err());
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: br#"{"a":1,"a":2,"empty":[]}"#,
        },
        &profile,
    )
    .unwrap();
    assert_eq!(
        model
            .values()
            .iter()
            .map(purrdf_json::Value::size)
            .collect::<Vec<_>>(),
        [Some(3), None, None, Some(0)]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn document_bases_factor_full_identities_for_every_namespace_shape() {
    for namespace in [
        "https://example.org/json#",
        "https://example.org/json/",
        "https://example.org/json#terms/",
        "https://example.org/json?q=terms#",
        "urn:example:json#",
    ] {
        let profile = Profile::new(
            "namespace-shapes",
            1,
            Vocabulary::under(namespace).unwrap(),
            Bounds::standard(),
        )
        .unwrap();
        let model = analyze(
            SourceDocument {
                id: SOURCE,
                bytes: br#"{"a":"value","a":[]}"#,
            },
            &profile,
        )
        .unwrap();
        assert!(!model.id().contains('#'));
        let dataset = project(&model, &profile).unwrap();
        let absolute =
            serialize_dataset_to_format(&*dataset, NativeRdfFormat::Turtle, None).unwrap();
        let relative =
            serialize_dataset_to_format(&*dataset, NativeRdfFormat::Turtle, Some(model.id()))
                .unwrap();
        assert!(relative.bytes.len() < absolute.bytes.len(), "{namespace}");
        let serialized = std::str::from_utf8(&relative.bytes).unwrap();
        assert!(serialized.contains("<#v0>"), "{namespace}: {serialized}");
        let reparsed = parse_dataset(&relative.bytes, "text/turtle", None).unwrap();
        assert_eq!(
            decode_document(&reparsed, model.id(), &profile)
                .unwrap()
                .as_bytes(),
            model.source().bytes
        );
    }
}
