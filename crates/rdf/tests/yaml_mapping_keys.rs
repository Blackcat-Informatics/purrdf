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
    YamlLdOptions, parse_yamlld_with_options, yamlld_to_jsonld, yamlld_to_jsonld_with_options,
};
use purrdf_rdf::native_codecs::parse_dataset;
use purrdf_rdf::{DatasetSink, OkfBundle, OkfConfig, lift_okf_bundle};

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
