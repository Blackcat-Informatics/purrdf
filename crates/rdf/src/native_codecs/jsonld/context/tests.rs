// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::{Value, json};

use super::*;

fn limits() -> JsonLdContextLimits {
    JsonLdContextLimits {
        context_bytes: 1_024,
        registry_documents: 8,
        registry_bytes: 4_096,
        terms: 64,
        nesting: 16,
        expansion_work: 1_024,
        definition_complexity: 1_024,
    }
}

#[test]
fn prefix_context_expands_compacts_and_canonicalizes_idempotently() {
    let compiled = CompiledJsonLdContext::from_prefixes([
        ("z", "https://z.example/"),
        ("ex", "https://example.org/ns#"),
    ])
    .expect("prefix context");

    assert_eq!(
        compiled.canonical_json(),
        r#"{"ex":{"@id":"https://example.org/ns#","@prefix":true},"z":{"@id":"https://z.example/","@prefix":true}}"#
    );
    assert_eq!(
        compiled
            .expand_iri("ex:Thing", true, false)
            .expect("expand"),
        Some("https://example.org/ns#Thing".to_owned())
    );
    assert_eq!(
        compiled
            .compact_iri("https://example.org/ns#Thing", true)
            .expect("compact"),
        "ex:Thing"
    );

    let recompiled = CompiledJsonLdContext::compile(compiled.canonical_context(), None)
        .expect("recompile canonical context");
    assert_eq!(recompiled, compiled);
}

#[test]
fn full_context_compiles_type_language_container_reverse_nest_and_scope() {
    let context = json!({
        "@base": "https://example.org/doc/",
        "@direction": "ltr",
        "@language": "EN",
        "@protected": true,
        "@version": 1.1,
        "@vocab": "https://example.org/vocab/",
        "schema": {"@id": "https://schema.org/", "@prefix": true},
        "id": "@id",
        "nest": "@nest",
        "name": {
            "@container": "@set",
            "@direction": "rtl",
            "@id": "schema:name",
            "@language": null
        },
        "link": {"@id": "schema:url", "@type": "@id"},
        "typed": {"@container": ["@set", "@type"], "@id": "schema:item"},
        "reverse": {"@container": "@set", "@reverse": "schema:parent"},
        "nested": {"@id": "schema:nested", "@nest": "nest"},
        "indexed": {
            "@container": "@index",
            "@id": "schema:indexed",
            "@index": "schema:position"
        },
        "scoped": {
            "@context": {"inner": "schema:inner"},
            "@id": "schema:scoped"
        },
        "payload": {"@id": "schema:payload", "@type": "@json"}
    });
    let compiled = CompiledJsonLdContext::compile(&context, None).expect("compile");

    assert_eq!(compiled.base_iri(), Some("https://example.org/doc/"));
    assert_eq!(compiled.vocab_mapping(), Some("https://example.org/vocab/"));
    assert_eq!(compiled.default_language(), Some("en"));
    assert_eq!(
        compiled.default_direction(),
        Some(JsonLdDirection::LeftToRight)
    );
    assert!(compiled.term("schema").expect("schema").is_prefix());
    assert_eq!(compiled.term("id").expect("id").iri_mapping(), Some("@id"));
    assert_eq!(
        compiled
            .expand_iri("id", false, false)
            .expect("keyword alias"),
        Some("@id".to_owned())
    );
    assert_eq!(
        compiled.compact_iri("_:b0", false).expect("blank node"),
        "_:b0"
    );

    let name = compiled.term("name").expect("name");
    assert_eq!(name.containers(), &BTreeSet::from([JsonLdContainer::Set]));
    assert_eq!(name.language_mapping(), Some(JsonLdNullable::Null));
    assert_eq!(
        name.direction_mapping(),
        Some(JsonLdNullable::Value(JsonLdDirection::RightToLeft))
    );
    assert!(name.is_protected());
    assert_eq!(
        compiled.term("link").expect("link").type_mapping(),
        Some(&JsonLdTypeMapping::Id)
    );
    assert!(
        compiled
            .term("reverse")
            .expect("reverse")
            .is_reverse_property()
    );
    assert_eq!(
        compiled.term("nested").expect("nested").nest(),
        Some("nest")
    );
    assert_eq!(
        compiled.term("indexed").expect("indexed").index_mapping(),
        Some("https://schema.org/position")
    );
    assert!(
        compiled
            .term("scoped")
            .expect("scoped")
            .scoped_context()
            .is_some()
    );
    assert_eq!(
        compiled.term("payload").expect("payload").type_mapping(),
        Some(&JsonLdTypeMapping::Json)
    );
}

#[test]
fn context_arrays_reset_and_protected_redefinition_is_rejected() {
    let context = json!([
        {"@vocab": "https://old.example/", "old": "https://old.example/value"},
        null,
        {"@vocab": "https://new.example/", "new": "value"}
    ]);
    let compiled = CompiledJsonLdContext::compile(&context, None).expect("reset context");
    assert!(compiled.term("old").is_none());
    assert_eq!(
        compiled.term("new").expect("new").iri_mapping(),
        Some("https://new.example/value")
    );

    let error = CompiledJsonLdContext::compile(
        &json!([
            {"@protected": true, "x": "https://example.org/x"},
            {"x": "https://example.org/changed"}
        ]),
        None,
    )
    .expect_err("protected redefinition");
    assert_eq!(error.code, "jsonld-context-invalid");
    assert!(error.message.contains("protected JSON-LD term `x`"));

    let error = CompiledJsonLdContext::compile(
        &json!([{"@protected": true, "x": "https://example.org/x"}, null]),
        None,
    )
    .expect_err("protected null reset");
    assert!(error.message.contains("erase protected term `x`"));
}

#[test]
fn offline_registry_resolves_context_iris_and_imports_without_network() {
    let registry = JsonLdContextRegistry::new([
        (
            "https://example.org/base",
            br#"{"@context":{"base":{"@id":"https://example.org/ns/","@prefix":true}}}"#.to_vec(),
        ),
        (
            "https://example.org/child",
            br#"{"@context":{"@import":"https://example.org/base","local":"base:local"}}"#.to_vec(),
        ),
    ])
    .expect("registry");
    let compiled =
        CompiledJsonLdContext::compile_registry_context("https://example.org/child", &registry)
            .expect("registry context");
    assert_eq!(registry.len(), 2);
    assert_eq!(
        compiled.term("local").expect("local").iri_mapping(),
        Some("https://example.org/ns/local")
    );
    assert_eq!(
        compiled.canonical_context(),
        &Value::String("https://example.org/child".to_owned())
    );
}

#[test]
fn registry_missing_documents_cycles_and_duplicate_members_fail_stably() {
    let missing = CompiledJsonLdContext::compile_registry_context(
        "https://example.org/missing",
        &JsonLdContextRegistry::default(),
    )
    .expect_err("missing document");
    assert_eq!(missing.code, "jsonld-context-invalid");
    assert!(missing.message.contains("no document"));

    let registry = JsonLdContextRegistry::new([
        (
            "https://example.org/a",
            br#"{"@context":"https://example.org/b"}"#.to_vec(),
        ),
        (
            "https://example.org/b",
            br#"{"@context":"https://example.org/a"}"#.to_vec(),
        ),
    ])
    .expect("cycle registry");
    let cycle = CompiledJsonLdContext::compile_registry_context("https://example.org/a", &registry)
        .expect_err("cycle");
    assert!(cycle.message.contains("offline context cycle"));

    let duplicate = CompiledJsonLdContext::compile_json(
        br#"{"x":"https://example.org/one","x":"https://example.org/two"}"#,
        None,
    )
    .expect_err("duplicate member");
    assert_eq!(duplicate.code, "jsonld-json-input");
    assert!(
        duplicate
            .message
            .contains("duplicate JSON object member `x`")
    );
}

#[test]
fn inverse_term_selection_obeys_shape_before_shortest_lexical_ties() {
    let context = json!({
        "a": {"@id": "https://example.org/p", "@type": "@id"},
        "b": "https://example.org/p",
        "languageName": {"@id": "https://example.org/p", "@language": "en"}
    });
    let compiled = CompiledJsonLdContext::compile(&context, None).expect("context");

    let id = JsonLdTermSelection::new(
        [Vec::<JsonLdContainer>::new()],
        JsonLdTermSelectionKind::Type,
        ["@id", "@none"],
    );
    assert_eq!(
        compiled
            .compact_iri_with_selection("https://example.org/p", true, Some(&id))
            .expect("id term"),
        "a"
    );

    let language = JsonLdTermSelection::new(
        [Vec::<JsonLdContainer>::new()],
        JsonLdTermSelectionKind::Language,
        ["en", "@none"],
    );
    assert_eq!(
        compiled
            .compact_iri_with_selection("https://example.org/p", true, Some(&language))
            .expect("language term"),
        "languageName"
    );

    let none = JsonLdTermSelection::new(
        [Vec::<JsonLdContainer>::new()],
        JsonLdTermSelectionKind::Language,
        ["@none"],
    );
    assert_eq!(
        compiled
            .compact_iri_with_selection("https://example.org/p", true, Some(&none))
            .expect("untyped term"),
        "b"
    );
}

#[test]
fn base_and_vocab_resolution_round_trip_exactly() {
    let compiled = CompiledJsonLdContext::compile(
        &json!({
            "@base": "https://example.org/a/b/",
            "@vocab": "../v/",
            "term": "item"
        }),
        None,
    )
    .expect("relative settings");
    assert_eq!(compiled.vocab_mapping(), Some("https://example.org/a/v/"));
    assert_eq!(
        compiled.term("term").expect("term").iri_mapping(),
        Some("https://example.org/a/v/item")
    );
    assert_eq!(
        compiled.expand_iri("c", false, true).expect("expand base"),
        Some("https://example.org/a/b/c".to_owned())
    );
    assert_eq!(
        compiled
            .compact_iri("https://example.org/a/b/c", false)
            .expect("remove base"),
        "c"
    );
}

#[test]
fn null_base_is_not_resurrected_from_the_document_url() {
    for relative_setting in [json!({"@base": "later"}), json!({"@vocab": "later/"})] {
        let error = CompiledJsonLdContext::compile(
            &json!([{"@base": null}, relative_setting]),
            Some("https://example.org/document"),
        )
        .expect_err("relative setting after null @base");
        assert!(error.message.contains("requires an absolute base IRI"));
    }

    let error = CompiledJsonLdContext::compile(
        &json!([{"@base": null}, {"path/item": {}}]),
        Some("https://example.org/document"),
    )
    .expect_err("relative term after null @base");
    assert!(error.message.contains("requires an absolute base IRI"));
}

#[test]
fn prefix_presence_and_mapped_index_rules_are_enforced() {
    for context in [
        json!({
            "ex": {"@id": "https://example.org/", "@prefix": true},
            "ex:item": {"@id": "https://example.org/item", "@prefix": false}
        }),
        json!({
            "path/item": {
                "@id": "https://example.org/path/item",
                "@prefix": false
            }
        }),
    ] {
        let error = CompiledJsonLdContext::compile(&context, Some("https://example.org/"))
            .expect_err("@prefix member on colon or slash term");
        assert!(error.message.contains("with @prefix must not contain"));
    }

    let error = CompiledJsonLdContext::compile(
        &json!({
            "indexed": {
                "@container": "@index",
                "@id": "https://example.org/indexed",
                "@index": "_:blank"
            }
        }),
        None,
    )
    .expect_err("blank-node mapped index");
    assert!(error.message.contains("index mapping"));
}

#[test]
fn iri_compaction_rejects_prefix_confusion_and_shape_unsafe_fallbacks() {
    let confused = CompiledJsonLdContext::compile(
        &json!({"urn": {"@id": "https://example.org/", "@prefix": true}}),
        None,
    )
    .expect("prefix context");
    for vocab in [false, true] {
        let error = confused
            .compact_iri("urn:item", vocab)
            .expect_err("IRI confused with prefix");
        assert!(
            error
                .message
                .contains("confused with active JSON-LD prefix `urn`")
        );
    }
    assert_eq!(
        confused
            .compact_iri("urn://authority/path", true)
            .expect("authority disambiguates IRI"),
        "urn://authority/path"
    );

    let shaped = CompiledJsonLdContext::compile(
        &json!({
            "ex": {"@id": "https://example.org/", "@prefix": true},
            "ex:item": {"@id": "https://example.org/item", "@type": "@id"}
        }),
        None,
    )
    .expect("shaped compact-IRI term");
    let language = JsonLdTermSelection::new(
        [Vec::<JsonLdContainer>::new()],
        JsonLdTermSelectionKind::Language,
        ["fr"],
    );
    assert_eq!(
        shaped
            .compact_iri_with_selection("https://example.org/item", true, Some(&language),)
            .expect("shape-safe compaction"),
        "https://example.org/item"
    );
    assert_eq!(
        shaped
            .compact_iri("https://example.org/item", false)
            .expect("null-value prefix fallback"),
        "ex:item"
    );

    let vocab = CompiledJsonLdContext::compile(
        &json!({
            "@vocab": "https://example.org/",
            "item": {"@id": "https://example.org/item", "@type": "@id"}
        }),
        None,
    )
    .expect("vocabulary context");
    assert_eq!(
        vocab
            .compact_iri_with_selection("https://example.org/item", true, Some(&language),)
            .expect("existing term blocks vocabulary suffix"),
        "https://example.org/item"
    );
}

#[test]
fn options_decoder_is_versioned_closed_and_mode_explicit() {
    let expanded = JsonLdSerializeOptions::from_json(br#"{"mode":"expanded","version":1}"#)
        .expect("expanded options");
    assert!(matches!(expanded.mode(), JsonLdSerializeMode::Expanded));

    let context = JsonLdSerializeOptions::from_json(
        br#"{"mode":"context","prefixes":{"ex":"https://example.org/"},"version":1,"yaml_schema_url":"schema.json"}"#,
    )
    .expect("prefix options");
    let JsonLdSerializeMode::Context(compiled) = context.mode() else {
        panic!("context mode");
    };
    assert_eq!(
        compiled.expand_iri("ex:item", true, false).expect("expand"),
        Some("https://example.org/item".to_owned())
    );
    assert_eq!(context.yaml_schema_url(), Some("schema.json"));

    for invalid in [
        br#"{"mode":"expanded","mode":"derived","version":1}"#.as_slice(),
        br#"{"mode":"expanded","prefixes":{},"version":1}"#.as_slice(),
        br#"{"mode":"expanded","unknown":true,"version":1}"#.as_slice(),
        br#"{"mode":"expanded","version":2}"#.as_slice(),
    ] {
        assert!(JsonLdSerializeOptions::from_json(invalid).is_err());
    }
    assert_eq!(
        JsonLdSerializeOptions::json_schema()["properties"]["version"]["const"],
        JSON_LD_SERIALIZE_OPTIONS_VERSION
    );
}

#[test]
fn options_schema_and_decoder_have_identical_mode_field_constraints() {
    fn schema_accepts(instance: &Value) -> bool {
        let location = "mem:///jsonld-options.schema.json";
        let mut schemas = boon::Schemas::new();
        let mut compiler = boon::Compiler::new();
        compiler
            .add_resource(location, JsonLdSerializeOptions::json_schema())
            .expect("register options schema");
        let compiled = compiler
            .compile(location, &mut schemas)
            .expect("compile options schema");
        schemas.validate(instance, compiled).is_ok()
    }

    let cases = [
        (json!({"mode": "expanded", "version": 1}), true),
        (json!({"mode": "derived", "version": 1}), true),
        (
            json!({"context": {}, "mode": "context", "version": 1}),
            true,
        ),
        (
            json!({
                "context": {},
                "document_iri": "https://example.org/document",
                "mode": "context",
                "registry": {
                    "https://example.org/context": {"@context": {}}
                },
                "version": 1
            }),
            true,
        ),
        (
            json!({
                "mode": "context",
                "prefixes": {"ex": "https://example.org/"},
                "version": 1
            }),
            true,
        ),
        (
            json!({"context": {}, "mode": "expanded", "version": 1}),
            false,
        ),
        (
            json!({
                "document_iri": "https://example.org/",
                "mode": "derived",
                "version": 1
            }),
            false,
        ),
        (json!({"mode": "context", "version": 1}), false),
        (
            json!({
                "context": {},
                "mode": "context",
                "prefixes": {"ex": "https://example.org/"},
                "version": 1
            }),
            false,
        ),
        (
            json!({
                "document_iri": "https://example.org/",
                "mode": "context",
                "prefixes": {"ex": "https://example.org/"},
                "version": 1
            }),
            false,
        ),
        (
            json!({
                "mode": "context",
                "prefixes": {"ex": "https://example.org/"},
                "registry": {},
                "version": 1
            }),
            false,
        ),
    ];
    for (instance, expected) in cases {
        let bytes = serde_json::to_vec(&instance).expect("encode options case");
        assert_eq!(
            JsonLdSerializeOptions::from_json(&bytes).is_ok(),
            expected,
            "decoder parity for {instance}"
        );
        assert_eq!(
            schema_accepts(&instance),
            expected,
            "schema parity for {instance}"
        );
    }
}

#[test]
fn options_input_can_carry_more_than_one_context_ceiling_of_registry_data() {
    let padding = "x".repeat(600_000);
    let options = json!({
        "context": {},
        "mode": "context",
        "registry": {
            "https://example.org/first": {"@context": null, "padding": padding},
            "https://example.org/second": {"@context": null, "padding": padding}
        },
        "version": 1
    });
    let bytes = serde_json::to_vec(&options).expect("encode registry-bearing options");
    let limits = JsonLdContextLimits::default();
    assert!(bytes.len() > limits.max_context_bytes());
    assert!(bytes.len() < limits.max_options_bytes());
    JsonLdSerializeOptions::from_json(&bytes).expect("decode aggregate registry options");
}

#[test]
fn fixed_limits_accept_exact_boundaries_and_reject_one_over() {
    let exact_registry_limits = JsonLdContextLimits {
        context_bytes: 4,
        registry_documents: 1,
        registry_bytes: 4,
        ..limits()
    };
    let registry = JsonLdContextRegistry::new_with_limits(
        [("https://example.org/context", b"null".to_vec())],
        exact_registry_limits,
    )
    .expect("exact registry byte/count limits");
    assert_eq!(registry.total_bytes(), 4);
    let byte_error = JsonLdContextRegistry::new_with_limits(
        [("https://example.org/context", b"null ".to_vec())],
        exact_registry_limits,
    )
    .expect_err("one byte over per-document limit");
    assert_eq!(byte_error.code, "jsonld-context-limit");
    let count_error = JsonLdContextRegistry::new_with_limits(
        [
            ("https://example.org/first", b"null".to_vec()),
            ("https://example.org/second", b"null".to_vec()),
        ],
        exact_registry_limits,
    )
    .expect_err("one document over registry limit");
    assert_eq!(count_error.code, "jsonld-context-limit");
    assert!(
        JsonLdContextRegistry::new_with_limits(
            [
                ("https://example.org/one", b"null".to_vec()),
                ("https://example.org/two", b"null".to_vec()),
            ],
            exact_registry_limits,
        )
        .is_err()
    );

    let exact_terms = JsonLdContextLimits {
        terms: 2,
        ..limits()
    };
    assert!(
        compiler::compile(
            &json!({"a": "https://example.org/a", "b": "https://example.org/b"}),
            None,
            &JsonLdContextRegistry::default(),
            exact_terms,
        )
        .is_ok()
    );
    let one_over = compiler::compile(
        &json!({
            "a": "https://example.org/a",
            "b": "https://example.org/b",
            "c": "https://example.org/c"
        }),
        None,
        &JsonLdContextRegistry::default(),
        exact_terms,
    )
    .expect_err("term one-over");
    assert_eq!(one_over.code, "jsonld-context-limit");

    let exact_work = JsonLdContextLimits {
        expansion_work: 2,
        definition_complexity: 0,
        ..limits()
    };
    assert!(
        compiler::compile(
            &Value::Null,
            None,
            &JsonLdContextRegistry::default(),
            exact_work,
        )
        .is_ok()
    );
    let one_over_work = compiler::compile(
        &Value::Null,
        None,
        &JsonLdContextRegistry::default(),
        JsonLdContextLimits {
            expansion_work: 1,
            ..exact_work
        },
    )
    .expect_err("work one-over");
    assert_eq!(one_over_work.code, "jsonld-context-limit");

    let strict = StrictJsonLimits {
        bytes: ByteLimit::new(4),
        depth: 1,
        values: 1,
    };
    assert_eq!(
        parse_strict_json(b"null", strict, "boundary").expect("exact strict limits"),
        Value::Null
    );
    assert!(parse_strict_json(b"null ", strict, "boundary").is_err());
}

#[test]
fn protected_terms_retain_identical_definitions_and_scopes_may_override() {
    let compiled = CompiledJsonLdContext::compile(
        &json!([
            {"@protected": true, "x": "https://example.org/x"},
            {"x": {"@id": "https://example.org/x", "@protected": false}}
        ]),
        None,
    )
    .expect("identical protected redefinition");
    assert!(compiled.term("x").expect("x").is_protected());

    let scoped = CompiledJsonLdContext::compile(
        &json!({
            "@protected": true,
            "x": "https://example.org/x",
            "scope": {
                "@id": "https://example.org/scope",
                "@context": {"x": "https://example.org/scoped-x"}
            }
        }),
        Some("https://example.org/context/document"),
    )
    .expect("scoped context validates with protected override");
    assert_eq!(
        scoped.term("x").expect("outer x").iri_mapping(),
        Some("https://example.org/x")
    );
    assert_eq!(
        scoped
            .term("scope")
            .expect("scope")
            .scoped_context_base_iri(),
        Some("https://example.org/context/document")
    );
}

#[test]
fn type_keyword_and_type_containers_follow_json_ld_11_rules() {
    let compiled = CompiledJsonLdContext::compile(
        &json!({
            "@type": {"@container": "@set", "@protected": true},
            "typed": {"@container": "@type", "@id": "https://example.org/typed"},
            "vocabTyped": {
                "@container": ["@set", "@type"],
                "@id": "https://example.org/vocab-typed",
                "@type": "@vocab"
            }
        }),
        None,
    )
    .expect("type definitions");
    let keyword = compiled.term("@type").expect("@type definition");
    assert_eq!(keyword.iri_mapping(), Some("@type"));
    assert!(keyword.is_protected());
    assert_eq!(
        keyword.containers(),
        &BTreeSet::from([JsonLdContainer::Set])
    );
    assert_eq!(
        compiled.term("typed").expect("typed").type_mapping(),
        Some(&JsonLdTypeMapping::Id)
    );
    assert_eq!(
        compiled
            .term("vocabTyped")
            .expect("vocab typed")
            .type_mapping(),
        Some(&JsonLdTypeMapping::Vocab)
    );

    for invalid in [
        json!({"@type": "https://example.org/type"}),
        json!({"@type": {"@container": "@index"}}),
        json!({
            "bad": {
                "@container": "@type",
                "@id": "https://example.org/bad",
                "@type": "https://example.org/datatype"
            }
        }),
    ] {
        assert!(CompiledJsonLdContext::compile(&invalid, None).is_err());
    }
}

#[test]
fn explicit_self_mappings_compact_iri_consistency_and_relative_terms_are_checked() {
    let self_mapped = CompiledJsonLdContext::compile(
        &json!({"@vocab": "https://example.org/vocab/", "x": {"@id": "x"}}),
        None,
    )
    .expect("self mapping");
    assert_eq!(
        self_mapped.term("x").expect("x").iri_mapping(),
        Some("https://example.org/vocab/x")
    );

    let relative = CompiledJsonLdContext::compile(
        &json!({"path/item": {}}),
        Some("https://example.org/base/document"),
    )
    .expect("relative IRI term");
    assert_eq!(
        relative
            .term("path/item")
            .expect("relative term")
            .iri_mapping(),
        Some("https://example.org/base/path/item")
    );

    let mismatch = CompiledJsonLdContext::compile(
        &json!({
            "ex": {"@id": "https://example.org/", "@prefix": true},
            "ex:item": {"@id": "https://other.example/item"}
        }),
        None,
    )
    .expect_err("compact IRI term must agree with explicit mapping");
    assert!(mismatch.message.contains("term itself expands"));
}

#[test]
fn reverse_definitions_accept_null_containers_and_stop_after_reverse() {
    let compiled = CompiledJsonLdContext::compile(
        &json!({
            "reverse": {
                "@container": null,
                "@direction": "rtl",
                "@language": "en",
                "@reverse": "https://example.org/reverse",
                "@type": "@id"
            }
        }),
        None,
    )
    .expect("reverse definition");
    let reverse = compiled.term("reverse").expect("reverse");
    assert!(reverse.is_reverse_property());
    assert!(reverse.containers().is_empty());
    assert_eq!(reverse.type_mapping(), Some(&JsonLdTypeMapping::Id));
    assert_eq!(reverse.language_mapping(), None);
    assert_eq!(reverse.direction_mapping(), None);

    assert!(
        CompiledJsonLdContext::compile(
            &json!({"bad": {"@reverse": "https://example.org/p", "@nest": "nest"}}),
            None,
        )
        .is_err()
    );
}

#[test]
fn inverse_context_uses_exact_language_direction_and_any_keys() {
    let iri = "https://example.org/p";
    let compiled = CompiledJsonLdContext::compile(
        &json!({
            "bothNull": {"@direction": null, "@id": iri, "@language": null},
            "direction": {"@direction": "rtl", "@id": iri},
            "directionNull": {"@direction": null, "@id": iri},
            "language": {"@id": iri, "@language": "EN"},
            "untyped": {"@id": iri, "@type": "@none"}
        }),
        None,
    )
    .expect("inverse context");

    let cases = [
        (JsonLdTermSelectionKind::Language, "@null", "bothNull"),
        (JsonLdTermSelectionKind::Language, "_rtl", "direction"),
        (JsonLdTermSelectionKind::Language, "@none", "directionNull"),
        (JsonLdTermSelectionKind::Language, "en", "language"),
        (JsonLdTermSelectionKind::Language, "@any", "untyped"),
        (JsonLdTermSelectionKind::Type, "@any", "untyped"),
    ];
    for (kind, preferred, expected) in cases {
        let selection =
            JsonLdTermSelection::new([Vec::<JsonLdContainer>::new()], kind, [preferred]);
        assert_eq!(
            compiled
                .compact_iri_with_selection(iri, true, Some(&selection))
                .expect("inverse selection"),
            expected
        );
    }
}

#[test]
fn any_inverse_selection_ignores_coercion_preferences() {
    let iri = "https://example.org/p";
    let compiled = CompiledJsonLdContext::compile(
        &json!({
            "plain": iri,
            "typed": {"@id": iri, "@type": "@id"}
        }),
        None,
    )
    .expect("inverse context");
    let selection = JsonLdTermSelection::new(
        [Vec::<JsonLdContainer>::new()],
        JsonLdTermSelectionKind::Any,
        ["@id"],
    );
    assert_eq!(
        compiled
            .compact_iri_with_selection(iri, true, Some(&selection))
            .expect("fallback selection"),
        "plain"
    );
}

#[test]
fn null_local_context_resets_to_original_document_base() {
    let compiled = CompiledJsonLdContext::compile(
        &json!({"@base": "nested/"}),
        Some("https://example.org/root/document"),
    )
    .expect("mutated active base");
    assert_eq!(
        compiled.base_iri(),
        Some("https://example.org/root/nested/")
    );

    let reset = compiled
        .apply_local_context(&Value::Null)
        .expect("reset local context");
    assert_eq!(
        reset
            .expand_iri("item", false, true)
            .expect("expand against reset base"),
        Some("https://example.org/root/item".to_owned())
    );
}

#[test]
fn propagation_import_merge_cache_and_keyword_form_behavior_are_bounded() {
    let propagated = CompiledJsonLdContext::compile(
        &json!({
            "@propagate": false,
            "x": "https://example.org/x",
            "y": "https://example.org/y"
        }),
        None,
    )
    .expect("non-propagating context");
    assert!(!propagated.propagates());
    assert!(propagated.has_previous_context());
    let array = CompiledJsonLdContext::compile(
        &json!([{"@propagate": false, "x": "https://example.org/x"}]),
        None,
    )
    .expect("array item validates @propagate without changing the call default");
    assert!(array.propagates());
    assert!(!array.has_previous_context());

    let imported = br#"{"@context":{"path/item":{"@id":"https://example.org/dir/path/item"}}}"#;
    let repeated = br#"{"@context":{"cached":"https://example.org/cached"}}"#;
    let registry = JsonLdContextRegistry::new([
        ("https://example.org/imported", imported.to_vec()),
        ("https://example.org/repeated", repeated.to_vec()),
    ])
    .expect("registry");
    let merged = CompiledJsonLdContext::compile_with_registry(
        &json!({"@import": "https://example.org/imported"}),
        Some("https://example.org/dir/document"),
        &registry,
    )
    .expect("reverse-merged import");
    assert_eq!(
        merged
            .term("path/item")
            .expect("imported term")
            .iri_mapping(),
        Some("https://example.org/dir/path/item")
    );

    let cache_limits = JsonLdContextLimits {
        context_bytes: 256,
        registry_bytes: repeated.len(),
        ..limits()
    };
    compiler::compile(
        &json!([
            "https://example.org/repeated",
            "https://example.org/repeated"
        ]),
        None,
        &registry,
        cache_limits,
    )
    .expect("cached remote context is charged once");

    let ignored = CompiledJsonLdContext::compile(
        &json!({"@Unknown": "ignored", "x": "https://example.org/x"}),
        None,
    )
    .expect("unknown keyword form is ignored");
    assert!(ignored.term("@Unknown").is_none());
    assert!(
        CompiledJsonLdContext::compile(&json!({"@triple": "https://example.org/forbidden"}), None,)
            .is_err()
    );
}

#[test]
fn canonical_context_byte_limit_accepts_exactly_and_rejects_one_over() {
    let context = json!({"x": "https://example.org/x"});
    let canonical_bytes = serde_json::to_vec(&canonicalize(&context)).expect("canonical bytes");
    let exact = JsonLdContextLimits {
        context_bytes: canonical_bytes.len(),
        ..limits()
    };
    compiler::compile(&context, None, &JsonLdContextRegistry::default(), exact)
        .expect("exact context byte limit");
    let error = compiler::compile(
        &context,
        None,
        &JsonLdContextRegistry::default(),
        JsonLdContextLimits {
            context_bytes: canonical_bytes.len() - 1,
            ..exact
        },
    )
    .expect_err("one byte over context limit");
    assert_eq!(error.code, "jsonld-context-limit");
}

#[test]
fn compiled_context_is_safe_to_reuse_across_threads_and_options() {
    let compiled = Arc::new(
        CompiledJsonLdContext::from_prefixes([("ex", "https://example.org/")]).expect("context"),
    );
    let options = JsonLdSerializeOptions::compiled(Arc::clone(&compiled));
    let handle = std::thread::spawn(move || {
        let JsonLdSerializeMode::Context(context) = options.mode() else {
            panic!("context mode");
        };
        context
            .expand_iri("ex:item", true, false)
            .expect("thread expansion")
    });
    assert_eq!(
        handle.join().expect("thread"),
        Some("https://example.org/item".to_owned())
    );
}
