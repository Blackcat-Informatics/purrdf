// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Non-string YAML mapping keys, per caller.
//!
//! The native YAML reader refuses a number, boolean or null mapping key unless the
//! caller opts in (`Limits::scalar_keys`). YAML-LD does not; OKF frontmatter does.
//! Each case is pinned with its neighbour:
//!
//! * **YAML-LD** refuses them because the specification requires it: "every mapping
//!   key MUST be a YAML scalar whose resolved node tag in the representation graph is
//!   `tag:yaml.org,2002:str`. Otherwise, a mapping-key-error MUST be detected, and
//!   processing aborted" (YAML-LD, "Mapping Key Types"). A quoted key, or one tagged
//!   `!!str`, is a string and is read.
//! * **OKF frontmatter** reads them as their source text, as the `serde_yaml` reader
//!   did (`1: a` is the key `1`), refuses a collection key, and checks uniqueness
//!   on the text (`1: a` with `"1": b` repeats a key).

use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdContextFormat, JsonLdContextRegistry, YamlLdOptions,
    parse_yamlld_with_context, parse_yamlld_with_options, yamlld_to_jsonld,
    yamlld_to_jsonld_with_options,
};
use purrdf_rdf::native_codecs::parse_dataset;
use purrdf_rdf::{DatasetSink, OkfBundle, OkfConfig, lift_okf_bundle};

#[test]
fn mixed_context_registry_reuses_yaml_admission_and_shared_resolution() {
    let main = concat!(
        "'@context':\n",
        "  '@import': shared.jsonld\n",
        "  child:\n",
        "    '@id': https://example.org/child\n",
        "    '@context': scoped.yamlld\n",
    );
    let imported = br#"{"@context":{"p":{"@id":"https://example.org/p","@type":"@id"}}}"#;
    let scoped = b"'@context': {q: 'https://example.org/q'}\n";
    let source = concat!(
        "'@context': contexts/main.yamlld\n",
        "'@id': subject\n",
        "p: object\n",
        "child: {'@id': child, q: inside}\n",
    );
    let expected = parse_dataset(
        concat!(
            "<https://example.org/docs/subject> <https://example.org/p> <https://example.org/docs/object> .\n",
            "<https://example.org/docs/subject> <https://example.org/child> <https://example.org/docs/child> .\n",
            "<https://example.org/docs/child> <https://example.org/q> \"inside\" .\n",
        )
        .as_bytes(),
        "application/n-quads",
        None,
    )
    .unwrap();
    for max_input_bytes in [main.len(), 0] {
        let mut options = YamlLdOptions::DEFAULT;
        options.syntax.syntax.max_input_bytes = max_input_bytes;
        let registry = JsonLdContextRegistry::new_with_formats([
            (
                "https://example.org/docs/contexts/main.yamlld",
                main.as_bytes(),
                JsonLdContextFormat::Yaml(options),
            ),
            (
                "https://example.org/docs/contexts/shared.jsonld",
                imported.as_slice(),
                JsonLdContextFormat::Json,
            ),
            (
                "https://example.org/docs/contexts/scoped.yamlld",
                scoped.as_slice(),
                JsonLdContextFormat::Yaml(YamlLdOptions::DEFAULT),
            ),
        ])
        .expect("mixed offline registry");
        let context = CompiledJsonLdContext::compile_with_registry(
            &purrdf_lex::json::Value::Null,
            Some("https://example.org/docs/source.yamlld"),
            &registry,
        )
        .unwrap();
        let actual =
            parse_yamlld_with_context(source.as_bytes(), &context, &YamlLdOptions::DEFAULT);
        if max_input_bytes == 0 {
            assert_eq!(actual.unwrap_err().code, "native-yaml-input-limit");
        } else {
            assert!(
                purrdf_rdf::datasets_isomorphic(
                    &actual.expect("relative YAML context, JSON import and YAML scoped context"),
                    &expected,
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None
                    )
                )
                .unwrap()
            );
        }
    }
}

#[test]
fn configured_yamlld_context_reaches_shared_expansion() {
    let registry = JsonLdContextRegistry::new([(
        "https://example.org/context",
        br#"{"@context":{"p":{"@id":"https://example.org/p","@type":"@id"}}}"#.as_slice(),
    )])
    .expect("offline property context");
    let initial = purrdf_lex::json::read_slice(
        br#"{"label":"https://example.org/label"}"#,
        purrdf_lex::json::Limits::DEFAULT,
    )
    .unwrap();
    let context = CompiledJsonLdContext::compile_with_registry(
        &initial,
        Some("https://example.org/docs/source.yamlld"),
        &registry,
    )
    .expect("initial terms, document base and offline registry");
    let source = concat!(
        "--- {'@context': 'https://example.org/context', '@id': first, p: object, label: one}\n",
        "--- {'@context': 'https://example.org/context', '@id': second, p: other, label: two}\n",
    );
    let first = concat!(
        "<https://example.org/docs/first> <https://example.org/p> <https://example.org/docs/object> .\n",
        "<https://example.org/docs/first> <https://example.org/label> \"one\" .\n",
    );
    let all = format!(
        "{first}<https://example.org/docs/second> <https://example.org/p> <https://example.org/docs/other> .\n<https://example.org/docs/second> <https://example.org/label> \"two\" .\n"
    );
    for (extract_all_scripts, expected) in [(false, first), (true, all.as_str())] {
        let options = YamlLdOptions {
            extract_all_scripts,
            ..YamlLdOptions::DEFAULT
        };
        let actual = parse_yamlld_with_context(source.as_bytes(), &context, &options)
            .expect("configured YAML-LD construction");
        let expected = parse_dataset(expected.as_bytes(), "application/n-quads", None).unwrap();
        assert!(
            purrdf_rdf::datasets_isomorphic(
                &actual,
                &expected,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap(),
            "configured context with extract_all_scripts={extract_all_scripts}"
        );
    }
    let mut refused = YamlLdOptions::DEFAULT;
    refused.syntax.syntax.max_input_bytes = 0;
    assert_eq!(
        parse_yamlld_with_context(source.as_bytes(), &context, &refused)
            .unwrap_err()
            .code,
        "native-yaml-input-limit"
    );
}

#[test]
fn basic_selection_routes_default_and_options_through_one_jsonld_engine() {
    let all = YamlLdOptions {
        extract_all_scripts: true,
        ..YamlLdOptions::DEFAULT
    };
    let source = "--- {'@context': {'@vocab': 'https://a.example/'}, '@id': 'https://example.org/s', p: first}\n--- [null, [{'@context': {'@vocab': 'https://b.example/'}, '@id': 'https://example.org/s', p: second}]]\n--- null\n";
    let first = parse_dataset(source.as_bytes(), "application/ld+yaml", None).unwrap();
    assert_eq!(first.quads().count(), 1);
    let selected = parse_yamlld_with_options(source.as_bytes(), None, &all).unwrap();
    let json = yamlld_to_jsonld_with_options(source.as_bytes(), &all).unwrap();
    let expected = parse_dataset(json.as_bytes(), "application/ld+json", None).unwrap();
    assert_eq!(selected.quads().count(), 2);
    let triples = String::from_utf8(
        purrdf_rdf::serialize_dataset(
            &selected,
            "application/n-quads",
            purrdf_rdf::SerializeGraph::Dataset,
        )
        .unwrap(),
    )
    .unwrap();
    assert!(triples.contains("<https://a.example/p>"));
    assert!(triples.contains("<https://b.example/p>"));
    let isolated = format!("{source}--- {{'@id': 'https://example.org/t', p: undefined}}\n");
    assert_eq!(
        parse_yamlld_with_options(isolated.as_bytes(), None, &all)
            .unwrap_err()
            .code,
        "jsonld-context-invalid"
    );
    assert_eq!(
        purrdf_rdf::serialize_dataset(
            &selected,
            "application/n-quads",
            purrdf_rdf::SerializeGraph::Dataset
        )
        .unwrap(),
        purrdf_rdf::serialize_dataset(
            &expected,
            "application/n-quads",
            purrdf_rdf::SerializeGraph::Dataset
        )
        .unwrap()
    );
    let shared = "--- {'@id': '_:shared', 'https://example.org/p': same}\n--- {'@id': '_:shared', 'https://example.org/p': same}";
    assert_eq!(
        parse_yamlld_with_options(shared.as_bytes(), None, &all)
            .unwrap()
            .quads()
            .count(),
        1
    );
    let copies = "[&a {'https://example.org/p': same}, *a]";
    assert_eq!(
        parse_dataset(copies.as_bytes(), "application/ld+yaml", None)
            .unwrap()
            .quads()
            .count(),
        2
    );
    for source in ["", "# absent\n", "---\n", "null", "[null, [null]]"] {
        assert_eq!(
            parse_dataset(source.as_bytes(), "application/ld+yaml", None)
                .unwrap()
                .quads()
                .count(),
            0
        );
    }
    assert_eq!(yamlld_to_jsonld(b"---\n--- {later: .inf}").unwrap(), "null");
    assert_eq!(
        yamlld_to_jsonld_with_options(b"---\n--- null", &all).unwrap(),
        "[null,null]"
    );
    // Arrays inside a JSON literal stay literal, rather than becoming entries.
    let literal = b"{'@id': 'https://example.org/s', 'https://example.org/p': {'@value': [null, [1, 2]], '@type': '@json'}}";
    assert_eq!(
        parse_dataset(literal, "application/ld+yaml", None)
            .unwrap()
            .quads()
            .count(),
        1
    );
}

#[test]
fn basic_validates_later_documents_and_preserves_original_error_codes_and_locations() {
    for (source, code) in [
        ("--- {}\n--- {1: value}", "mapping-key-error"),
        (
            "--- {}\n--- {a: value, !!str a: other}",
            "loading-document-failed",
        ),
        ("--- {}\n--- string", "loading-document-failed"),
        ("--- {}\n--- &a [*a]", "loading-document-failed"),
        ("--- {}\n--- [broken", "loading-document-failed"),
    ] {
        let error = yamlld_to_jsonld(source.as_bytes()).unwrap_err();
        assert_eq!(error.code, code, "{source}");
        assert_eq!(error.location.as_ref().unwrap().line, Some(2));
        assert!(error.native_cause().is_some());
    }
    for source in ["--- {}\n--- {n: .inf}", "--- {}\n--- {n: !custom value}"] {
        assert_eq!(yamlld_to_jsonld(source.as_bytes()).unwrap(), "{}");
        let all = YamlLdOptions {
            extract_all_scripts: true,
            ..YamlLdOptions::DEFAULT
        };
        assert_eq!(
            yamlld_to_jsonld_with_options(source.as_bytes(), &all)
                .unwrap_err()
                .code,
            "loading-document-failed"
        );
    }
    let invalid = b"--- {}\n--- \xff";
    for error in [
        yamlld_to_jsonld(invalid).unwrap_err(),
        parse_dataset(invalid, "application/ld+yaml", None).unwrap_err(),
    ] {
        assert_eq!(error.code, "invalid-encoding");
        assert_eq!(error.location.as_ref().unwrap().line, Some(2));
        assert_eq!(error.location.as_ref().unwrap().column, Some(5));
    }
    let mut options = YamlLdOptions::DEFAULT;
    options.projection.max_numeric_bytes = 1;
    let error = yamlld_to_jsonld_with_options(b"{n: 12}", &options).unwrap_err();
    assert_eq!(error.code, "native-yaml-numeric-output-limit");
    assert!(error.native_cause().is_some());
    options.projection.max_numeric_bytes = 64;
    assert_eq!(
        yamlld_to_jsonld_with_options(
            b"{n: 0x100000000000000000000000000000001, date: 2026-11-04}",
            &options
        )
        .unwrap(),
        r#"{"n":340282366920938463463374607431768211457,"date":"2026-11-04"}"#
    );
    options.max_serialization_bytes = 1024;
    assert_eq!(
        yamlld_to_jsonld_with_options(b"{p: x}", &options).unwrap(),
        r#"{"p":"x"}"#
    );
    let escaped = format!("{{p: \"{}\"}}", "\\x01".repeat(512));
    let error = yamlld_to_jsonld_with_options(escaped.as_bytes(), &options).unwrap_err();
    assert_eq!(error.code, "native-yaml-output-limit");
    assert_eq!(
        error.storage_error(),
        Some(purrdf_lex::allocation::StorageError::AdmissionFailed)
    );
    options.max_serialization_bytes = 0;
    assert_eq!(
        parse_yamlld_with_options(b"{'https://example.org/p': x}", None, &options)
            .unwrap()
            .quads()
            .count(),
        1
    );
    options.syntax.syntax.max_input_bytes = invalid.len() - 1;
    assert_eq!(
        yamlld_to_jsonld_with_options(invalid, &options)
            .unwrap_err()
            .code,
        "native-yaml-input-limit"
    );
    options.syntax.syntax.max_input_bytes = invalid.len();
    assert_eq!(
        yamlld_to_jsonld_with_options(invalid, &options)
            .unwrap_err()
            .code,
        "invalid-encoding"
    );
}

fn yamlld(keys: &str) -> Result<usize, String> {
    let text = format!(
        "\"@context\":\n  \"1\": http://example.org/t1\n  \"true\": http://example.org/t2\n  \"null\": http://example.org/t3\n  \"~\": http://example.org/t4\n\"@id\": http://example.org/s\n\"http://example.org/p\": v\n{keys}\n"
    );
    parse_dataset(text.as_bytes(), "application/ld+yaml", None)
        .map(|dataset| dataset.quads().count())
        .map_err(|error| error.message)
}

#[test]
fn yamlld_refuses_a_number_boolean_or_null_key_and_reads_its_string_spellings() {
    for refused in [
        "1: a",
        "true: a",
        "null: a",
        "1.5: a",
        "~: a",
        "0x1F: a",
        "[x]: a",
        "? [x]\n: a",
        "{x: y}: a",
    ] {
        let error = yamlld(refused).expect_err(refused);
        assert!(
            error.contains("mapping key must be a string"),
            "{refused:?}: {error}"
        );
    }
    // The neighbours: the same keys as strings are read, each a defined term
    // that adds its triple to the document's one.
    for accepted in [
        "'1': a",
        "\"true\": a",
        "!!str 1: a",
        "'null': a",
        "!!str ~: a",
    ] {
        assert_eq!(yamlld(accepted), Ok(2), "{accepted:?}");
    }
    assert_eq!(yamlld("\"http://example.org/q\": a"), Ok(2));
}

fn config() -> OkfConfig {
    OkfConfig::new(
        "https://example.org/okf#",
        "https://example.org/doc/",
        ["type", "title", "producer", "true", "True", "null"],
    )
    .expect("valid caller profile")
}

fn lifted(frontmatter: &str) -> Result<(), String> {
    let bundle = OkfBundle::from_documents([(
        "doc.md",
        format!("---\ntype: Thing\n{frontmatter}\n---\nBody.\n"),
    )])
    .map_err(|error| error.to_string())?;
    let mut sink = DatasetSink::new();
    lift_okf_bundle(&bundle, &config(), &mut sink)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn okf_frontmatter_reads_scalar_keys_as_text_and_keeps_them_unique() {
    // Listed fields written as non-string scalars are those fields.
    for accepted in ["true: a", "null: a", "True: a", "'true': a", "? true\n: a"] {
        assert_eq!(lifted(accepted), Ok(()), "{accepted:?}");
    }
    // Nested keys of every scalar kind are read.
    for accepted in [
        "producer: {1: a}",
        "producer: {true: a, null: b, ~: c}",
        "producer: {1.5: a}",
    ] {
        assert_eq!(lifted(accepted), Ok(()), "{accepted:?}");
    }
    // An unlisted scalar key gets past YAML to the codec's own field rule.
    let unlisted = lifted("1: a").expect_err("`1` is not a listed field");
    assert!(
        unlisted.contains("unrecognized frontmatter key `1`"),
        "{unlisted}"
    );
    // Uniqueness holds on the text.
    for repeated in [
        "producer: {1: a, \"1\": b}",
        "true: a\n\"true\": b",
        "producer: {1: a, 1: b}",
    ] {
        let error = lifted(repeated).expect_err(repeated);
        assert!(error.contains("repeats a key"), "{repeated:?}: {error}");
    }
    // A collection is never a key.
    for refused in ["[x]: a", "? [x]\n: a", "{x: y}: a", "producer: {[x]: a}"] {
        let error = lifted(refused).expect_err(refused);
        assert!(error.contains("must be a string"), "{refused:?}: {error}");
    }
}

/// Every normative ToRDF entry in the publication-day W3C YAML-LD snapshot.
#[test]
fn pinned_w3c_normative_rdf_construction() {
    use purrdf_lex::json::{self, Limits};

    const SELECTED: &[&str] = &[
        "#aa-information-1-positive",
        "#shared-blank-node-from-stream",
        "#empty-document-from-stream-rdf",
        "#empty-document-rdf",
        "#empty-document-rdf-all",
        "#zero-documents-rdf",
        "#zero-documents-rdf-all",
        "#all-empty-rdf",
        "#all-empty-rdf-all",
        "#leading-empty-rdf",
        "#trailing-empty-rdf",
    ];
    const EXCLUDED: &[&str] = &[
        "#cir-scalar-core-2-positive",
        "#cir-scalar-other-2-positive",
    ];
    let root = purrdf_testkit::paths::workspace_root().join("vectors/yaml-ld/tests");
    let manifest = json::read_slice(
        &std::fs::read(root.join("manifest.jsonld")).expect("pinned YAML-LD manifest exists"),
        Limits::DEFAULT,
    )
    .expect("pinned YAML-LD manifest JSON");
    let entries = manifest["sequence"].as_array().expect("manifest sequence");
    assert_eq!(entries.len(), 97, "complete pinned manifest inventory");
    let base = manifest["baseIri"].as_str().expect("manifest base IRI");
    assert_eq!(base, "https://w3c.github.io/yaml-ld/tests/");
    let mut selected = Vec::new();
    let mut excluded = Vec::new();
    for entry in entries {
        let id = entry["@id"].as_str().expect("fixture ID");
        let types = entry["@type"].as_array().expect("fixture types");
        if !types
            .iter()
            .any(|kind| kind.as_str() == Some("jld:ToRDFTest"))
        {
            continue;
        }
        assert_eq!(types.len(), 2, "{id}: expected ToRDF evaluation type pair");
        assert!(
            types
                .iter()
                .any(|kind| kind.as_str() == Some("jld:PositiveEvaluationTest")),
            "{id}: positive evaluation"
        );
        let options = entry.get("option").map(|value| {
            value
                .as_object()
                .unwrap_or_else(|| panic!("{id}: options object"))
        });
        if let Some(options) = options
            && (options.contains_key("extendedYAML") || options.contains_key("normative"))
        {
            assert_eq!(options.len(), 2, "{id}: explicit Extended YAML exclusion");
            assert_eq!(options["extendedYAML"].as_bool(), Some(true), "{id}");
            assert_eq!(options["normative"].as_bool(), Some(false), "{id}");
            excluded.push(id);
            continue;
        }
        if let Some(options) = options {
            for key in options.keys() {
                assert_eq!(key, "extractAllScripts", "{id}: unexpected option");
            }
        }
        selected.push(entry);
    }
    assert_eq!(
        selected
            .iter()
            .map(|entry| entry["@id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        SELECTED,
        "every normative ToRDF ID is selected exactly once"
    );
    assert_eq!(
        excluded, EXCLUDED,
        "only the two informative ToRDF entries are excluded"
    );
    for entry in selected {
        let id = entry["@id"].as_str().unwrap();
        let input = entry["input"]
            .as_str()
            .unwrap_or_else(|| panic!("{id}: input path"));
        let expected = entry["expect"]
            .as_str()
            .unwrap_or_else(|| panic!("{id}: expected N-Quads path"));
        for path in [input, expected] {
            assert!(
                path.starts_with("cases/")
                    && std::path::Path::new(path)
                        .components()
                        .all(|part| matches!(part, std::path::Component::Normal(_))),
                "{id}: local pinned fixture path {path}"
            );
        }
        let extract_all_scripts = entry.get("option").is_some_and(|options| {
            options["extractAllScripts"]
                .as_bool()
                .unwrap_or_else(|| panic!("{id}: boolean extractAllScripts"))
        });
        let options = YamlLdOptions {
            extract_all_scripts,
            ..YamlLdOptions::DEFAULT
        };
        let source = std::fs::read(root.join(input))
            .unwrap_or_else(|error| panic!("{id}: {input}: {error}"));
        let actual = parse_yamlld_with_options(&source, Some(&format!("{base}{input}")), &options)
            .unwrap_or_else(|error| panic!("{id}: YAML-LD construction: {error}"));
        let oracle = std::fs::read(root.join(expected))
            .unwrap_or_else(|error| panic!("{id}: {expected}: {error}"));
        let expected = parse_dataset(&oracle, "application/n-quads", None)
            .unwrap_or_else(|error| panic!("{id}: pinned N-Quads: {error}"));
        assert!(
            purrdf_rdf::datasets_isomorphic(
                &actual,
                &expected,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap_or_else(|error| panic!("{id}: dataset comparison: {error}")),
            "{id}: constructed dataset differs from upstream N-Quads"
        );
    }
}
