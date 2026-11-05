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
    CompiledJsonLdContext, JsonLdCompactionLimits, JsonLdCompactionPolicy, JsonLdContextFormat,
    JsonLdContextRegistry, JsonLdExpansionLimits, JsonLdFlattenLimits, YamlLdOptions,
    compact_expanded_with_context, compact_jsonld_with_context, compact_yamlld_with_context,
    expand_jsonld_with_context, expand_yamlld_with_context, flatten_jsonld_with_context,
    flatten_yamlld_with_context, parse_jsonld_with_context, parse_yamlld_with_context,
    parse_yamlld_with_options, yamlld_to_jsonld, yamlld_to_jsonld_with_options,
};
use purrdf_rdf::native_codecs::parse_dataset;
use purrdf_rdf::{DatasetSink, OkfBundle, OkfConfig, lift_okf_bundle};

#[test]
fn document_compaction_preserves_structure_and_native_terms() {
    use purrdf_lex::json::{self, Value};
    let expansion = JsonLdExpansionLimits::DEFAULT;
    let limits = JsonLdCompactionLimits::DEFAULT;
    let policy = JsonLdCompactionPolicy::DEFAULT;
    let empty = CompiledJsonLdContext::compile(&json::read("{}").unwrap(), None).unwrap();
    let check = |source: &str, context: &str, expected: &str, compact_arrays: bool| {
        let context_value = json::read(context).unwrap();
        let context = CompiledJsonLdContext::compile(&context_value, None).unwrap();
        let source = json::read(source).unwrap();
        let mut expected = json::read(expected).unwrap();
        if !context_value.is_null() {
            expected
                .as_object_mut()
                .unwrap()
                .insert("@context", context_value);
        }
        let actual = compact_expanded_with_context(
            &source,
            &context,
            &JsonLdCompactionPolicy {
                compact_arrays,
                ..policy
            },
            &limits,
        )
        .unwrap();
        assert_eq!(actual, expected, "expanded: {source:?}");
        actual
    };
    for (source, context, expected, arrays) in [
        ("null", "null", "{}", true),
        ("[]", "null", "{}", true),
        ("[]", "{}", "{}", false),
        (r#"[{"@id":"urn:a"}]"#, "{}", r#"{"@id":"urn:a"}"#, true),
        (
            r#"[{"@id":"urn:a"}]"#,
            r#"{"g":"@graph"}"#,
            r#"{"g":[{"@id":"urn:a"}]}"#,
            false,
        ),
        (
            r#"[{"@id":"urn:a"},{"@id":"urn:b"}]"#,
            r#"{"g":"@graph"}"#,
            r#"{"g":[{"@id":"urn:a"},{"@id":"urn:b"}]}"#,
            true,
        ),
        (
            r#"[{"@id":"urn:s","@type":["urn:T"],"urn:p":[{"@value":1.0}],"urn:q":[{"@id":"urn:o"}]}]"#,
            r#"{"T":"urn:T","p":"urn:p","q":{"@id":"urn:q","@type":"@id"}}"#,
            r#"{"@id":"urn:s","@type":"T","p":1.0,"q":"urn:o"}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@list":[{"@value":"a"},{"@value":"b"}]}]}]"#,
            r#"{"details":"@nest","p":{"@id":"urn:p","@container":"@list","@nest":"details"}}"#,
            r#"{"@graph":[{"details":{"p":["a","b"]}}]}"#,
            false,
        ),
        (
            r#"[{"urn:p":[{"@list":[{"@value":"one","@language":"en"},{"@value":"deux","@language":"fr"}]}]}]"#,
            r#"{"@language":"en","p":{"@id":"urn:p","@container":"@list","@language":"en"}}"#,
            r#"{"urn:p":{"@list":["one",{"@language":"fr","@value":"deux"}]}}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@value":"hello","@language":"en"},{"@value":"none"}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":"@language"}}"#,
            r#"{"@graph":[{"p":{"en":["hello"],"@none":["none"]}}]}"#,
            false,
        ),
        (
            r#"[{"urn:p":[{"@id":"urn:a","urn:q":[{"@value":"x"}]},{}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":"@id"}}"#,
            r#"{"p":{"urn:a":{"urn:q":"x"},"@none":{}}}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@id":"urn:x","@type":["urn:T"]}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":"@type","@type":"@id"},"T":"urn:T"}"#,
            r#"{"p":{"T":"urn:x"}}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@index":"row","@value":"x"},{"@value":"none"}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":"@index"}}"#,
            r#"{"p":{"row":"x","@none":"none"}}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@id":"urn:x","urn:key":[{"@value":"row"},{"@value":"residual"}],"urn:q":[{"@value":"x"}]}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":"@index","@index":"urn:key"}}"#,
            r#"{"p":{"row":{"@id":"urn:x","urn:key":"residual","urn:q":"x"}}}"#,
            true,
        ),
        (
            r#"[{"@reverse":{"urn:p":[{"@id":"urn:x"}],"urn:q":[{"@id":"urn:y"}]}}]"#,
            r#"{"back":{"@reverse":"urn:p","@type":"@id"}}"#,
            r#"{"@graph":[{"back":["urn:x"],"@reverse":{"urn:q":[{"@id":"urn:y"}]}}]}"#,
            false,
        ),
        (
            r#"[{"urn:p":[{"@id":"urn:x","urn:q":[{"@value":"scoped"}]}]}]"#,
            r#"{"p":{"@id":"urn:p","@context":{"q":"urn:q"}}}"#,
            r#"{"p":{"@id":"urn:x","q":"scoped"}}"#,
            true,
        ),
        (
            r#"[{"@type":["urn:T"],"urn:q":[{"@value":"local"}],"urn:p":[{"urn:q":[{"@value":"child"}]}]}]"#,
            r#"{"T":{"@id":"urn:T","@context":{"q":"urn:q"}},"p":"urn:p"}"#,
            r#"{"@type":"T","q":"local","p":{"urn:q":"child"}}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@graph":[{"@id":"urn:a"},{"@id":"urn:b"}]}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":"@graph"},"inc":"@included"}"#,
            r#"{"@graph":[{"p":[{"inc":[{"@id":"urn:a"},{"@id":"urn:b"}]}]}]}"#,
            false,
        ),
        (
            r#"[{"urn:p":[{"@id":"urn:g","@graph":[{"@id":"urn:a"},{"@id":"urn:b"}]}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":["@graph","@id"]}}"#,
            r#"{"@graph":[{"p":{"urn:g":[{"@id":"urn:a"},{"@id":"urn:b"}]}}]}"#,
            false,
        ),
        (
            r#"[{"urn:p":[{"@index":"row","@graph":[{"@id":"urn:a"},{"@id":"urn:b"}]}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":["@graph","@index"]}}"#,
            r#"{"@graph":[{"p":{"row":[{"@graph":[{"@id":"urn:a"},{"@id":"urn:b"}]}]}}]}"#,
            false,
        ),
        (
            r#"[{"urn:p":[{"@id":"urn:g","@graph":[{"@id":"urn:a"}],"urn:key":[{"@value":"row"},{"@value":"rest"}]}]}]"#,
            r#"{"p":{"@id":"urn:p","@container":["@graph","@index"],"@index":"urn:key"}}"#,
            r#"{"p":{"row":{"@id":"urn:g","@graph":[{"@id":"urn:a"}],"urn:key":"rest"}}}"#,
            true,
        ),
        (
            r#"[{"@included":[{"@id":"urn:i","urn:p":[{"@value":"included"}]}]}]"#,
            r#"{"inc":"@included","p":"urn:p"}"#,
            r#"{"inc":{"@id":"urn:i","p":"included"}}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@value":{"@bogus":[1.0,1e999999999999999999999]},"@type":"@json"}]}]"#,
            r#"{"p":{"@id":"urn:p","@type":"@json"}}"#,
            r#"{"p":{"@bogus":[1.0,1e999999999999999999999]}}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@value":null,"@type":"@json","@index":"row"}]}]"#,
            r#"{"p":{"@id":"urn:p","@type":"@json"}}"#,
            r#"{"urn:p":{"@value":null,"@type":"@json","@index":"row"}}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@value":[1.0],"@type":"@json"},{"@value":[2e0],"@type":"@json"}]}]"#,
            r#"{"p":{"@id":"urn:p","@type":"@json"}}"#,
            r#"{"urn:p":[{"@value":[1.0],"@type":"@json"},{"@value":[2e0],"@type":"@json"}]}"#,
            true,
        ),
        (
            r#"[{"urn:p":[{"@triple":{"@subject":{"@id":"urn:s"},"@predicate":"urn:q","@object":{"@value":1}},"@annotation":[{"@id":"urn:r","urn:note":[{"@value":"note"}]}]}],"@annotation":[{"urn:note":[{"@value":"detached"}]}]}]"#,
            r#"{"p":"urn:p","note":"urn:note"}"#,
            r#"{"@graph":[{"p":[{"@triple":{"@subject":{"@id":"urn:s"},"@predicate":"urn:q","@object":{"@value":1}},"@annotation":[{"@id":"urn:r","note":["note"]}]}],"@annotation":[{"note":["detached"]}]}]}"#,
            false,
        ),
    ] {
        let actual = check(source, context, expected, arrays);
        if let Some(opaque) = actual
            .get("p")
            .filter(|value| value.get("@bogus").is_some())
        {
            assert_eq!(
                json::write_compact(opaque),
                r#"{"@bogus":[1.0,1e999999999999999999999]}"#,
                "opaque numeric spelling"
            );
        }
    }

    let multiple_lists = r#"[{"urn:p":[{"@list":[{"@value":"a"}]},{"@list":[{"@value":"b"}]}]}]"#;
    check(
        multiple_lists,
        r#"{"p":{"@id":"urn:p","@container":"@list"}}"#,
        r#"{"p":["a"],"urn:p":{"@list":["b"]}}"#,
        true,
    );
    check(
        r#"[{"urn:p":[{"@list":[{"@value":"a","@language":"en"}]},{"@list":[{"@value":"b","@language":"fr"}]}]}]"#,
        r#"{"urn:p":{"@id":"urn:p","@container":"@list","@language":"en"},"fr":{"@id":"urn:p","@container":"@list","@language":"fr"}}"#,
        r#"{"urn:p":["a"],"fr":["b"]}"#,
        true,
    );
    check(
        r#"[{"urn:p":[{"@list":[{"@value":"a","@language":"en"}]},{"@list":[{"@value":"b","@language":"en"}]}]}]"#,
        r#"{"urn:p":{"@id":"urn:p","@container":"@list","@language":"en"},"en":{"@id":"urn:p","@container":"@list","@language":"en"}}"#,
        r#"{"en":["a"],"urn:p":["b"]}"#,
        true,
    );
    let shadowed = CompiledJsonLdContext::compile(
        &json::read(r#"{"urn:p":{"@id":"urn:p","@container":"@list"}}"#).unwrap(),
        None,
    )
    .unwrap();
    assert!(
        compact_expanded_with_context(
            &json::read(multiple_lists).unwrap(),
            &shadowed,
            &policy,
            &limits
        )
        .unwrap_err()
        .message
        .contains("no lossless property spelling")
    );
    assert!(
        compact_expanded_with_context(
            &json::read(r#"[{"urn:p":[{"@list":[{"@value":"a"}]}]}]"#).unwrap(),
            &shadowed,
            &policy,
            &limits
        )
        .is_ok()
    );

    let json_shadow = CompiledJsonLdContext::compile(
        &json::read(r#"{"urn:p":{"@id":"urn:p","@type":"@json"}}"#).unwrap(),
        None,
    )
    .unwrap();
    assert!(
        compact_expanded_with_context(
            &json::read(
                r#"[{"urn:p":[{"@type":"@json","@value":1},{"@type":"@json","@value":2}]}]"#
            )
            .unwrap(),
            &json_shadow,
            &policy,
            &limits
        )
        .unwrap_err()
        .message
        .contains("no lossless property spelling")
    );
    assert!(
        compact_expanded_with_context(
            &json::read(r#"[{"urn:p":[{"@type":"@json","@value":1}]}]"#).unwrap(),
            &json_shadow,
            &policy,
            &limits
        )
        .is_ok()
    );
    for (context, source) in [
        (
            r#"{"urn:p":{"@id":"urn:p","@container":"@index"}}"#,
            r#"[{"urn:p":[{"@value":"a","@index":"row"},{"@value":"b","@index":"@none"}]}]"#,
        ),
        (
            r#"{"urn:p":{"@id":"urn:p","@container":"@language"},"en":"@none"}"#,
            r#"[{"urn:p":[{"@value":"hello","@language":"en"}]}]"#,
        ),
    ] {
        let context = CompiledJsonLdContext::compile(&json::read(context).unwrap(), None).unwrap();
        let raw = json::read(source).unwrap();
        let compacted = compact_expanded_with_context(&raw, &context, &policy, &limits).unwrap();
        assert_eq!(
            compacted["urn:p"], raw[0]["urn:p"],
            "ordinary map shadows use explicit metadata objects"
        );
        assert_eq!(
            expand_jsonld_with_context(
                json::write_compact(&compacted).as_bytes(),
                &empty,
                &expansion
            )
            .unwrap(),
            raw
        );
    }
    check(
        r#"[{"urn:p":[{"@type":"@json","@value":1},{"@type":"@json","@value":2}]}]"#,
        r#"{"urn:p":{"@id":"urn:p","@type":"@json"},"safe":{"@id":"urn:p","@type":"@none"}}"#,
        r#"{"safe":[{"@type":"@json","@value":1},{"@type":"@json","@value":2}]}"#,
        true,
    );

    let metadata_list=json::read(r#"[{"urn:p":[{"@list":[{"@value":"a"}],"@index":"row","@annotation":[{"@id":"urn:r"}]}]}]"#).unwrap();
    let list_context = CompiledJsonLdContext::compile(
        &json::read(r#"{"urn:p":{"@id":"urn:p","@container":"@list"}}"#).unwrap(),
        None,
    )
    .unwrap();
    let list_policy = JsonLdCompactionPolicy {
        compact_arrays: false,
        ..policy
    };
    let compacted =
        compact_expanded_with_context(&metadata_list, &list_context, &list_policy, &limits)
            .unwrap();
    assert_eq!(
        compacted["@graph"][0]["urn:p"],
        json::read(r#"{"@list":["a"],"@index":"row","@annotation":[{"@id":"urn:r"}]}"#).unwrap()
    );
    assert_eq!(
        expand_jsonld_with_context(
            json::write_compact(&compacted).as_bytes(),
            &empty,
            &expansion
        )
        .unwrap(),
        metadata_list
    );

    let indexed_graph =
        json::read(r#"[{"urn:p":[{"@id":"urn:g","@index":"none","@graph":[{"@id":"urn:a","urn:q":[{"@value":"a"}]}]}]}]"#)
            .unwrap();
    let indexed_context = CompiledJsonLdContext::compile(
        &json::read(r#"{"urn:p":{"@id":"urn:p","@container":["@graph","@index"]},"none":"@none"}"#)
            .unwrap(),
        None,
    )
    .unwrap();
    let compacted =
        compact_expanded_with_context(&indexed_graph, &indexed_context, &policy, &limits).unwrap();
    assert_eq!(
        compacted["urn:p"]["none"],
        json::read(r#"{"@id":"urn:g","@index":"none","@graph":[{"@id":"urn:a","urn:q":"a"}]}"#)
            .unwrap()
    );
    assert_eq!(
        expand_jsonld_with_context(
            json::write_compact(&compacted).as_bytes(),
            &empty,
            &expansion
        )
        .unwrap(),
        indexed_graph
    );

    for key in ["@none", "none"] {
        let mut raw=json::read(r#"[{"urn:p":[{"@id":"urn:g","@graph":[{"@id":"urn:a","urn:q":[{"@value":"a"}]}],"urn:key":[{"@value":"placeholder"}]}]}]"#).unwrap();
        raw[0]["urn:p"][0]["urn:key"][0]["@value"] = Value::String(key.to_owned());
        let context=CompiledJsonLdContext::compile(&json::read(r#"{"p":{"@id":"urn:p","@container":["@graph","@index"],"@index":"urn:key"},"none":"@none"}"#).unwrap(),None).unwrap();
        let compacted = compact_expanded_with_context(&raw, &context, &policy, &limits).unwrap();
        assert_eq!(compacted["p"][key]["urn:key"], key);
        assert_eq!(compacted["p"][key]["@id"], "urn:g");
        let reopened = expand_jsonld_with_context(
            json::write_compact(&compacted).as_bytes(),
            &empty,
            &expansion,
        )
        .unwrap();
        assert_eq!(reopened, raw, "real key that aliases the none sentinel");
    }
    // RDF graph ownership must distinguish a real sentinel-looking key as well.
    let graph_context =
        json::read(r#"{"p":{"@id":"urn:p","@container":["@graph","@index"],"@index":"urn:key"},"raw":{"@id":"urn:raw","@type":"@json"}}"#)
            .unwrap();
    for key in ["@none", ""] {
        let source = format!(
            "<urn:s> <urn:p> _:g .\n_:g <urn:key> {} .\n<urn:a> <urn:q> \"inside\" _:g .\n<urn:s> <urn:raw> \"[0.25]\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON> .\n",
            json::write_compact(&Value::String(key.to_owned()))
        );
        let dataset = parse_dataset(source.as_bytes(), "application/n-quads", None).unwrap();
        let options = purrdf_rdf::native_codecs::jsonld::JsonLdSerializeOptions::context(
            &graph_context,
            None,
        )
        .unwrap();
        let text = purrdf_rdf::native_codecs::jsonld::serialize_dataset_to_jsonld_with_options(
            &dataset, &options,
        )
        .unwrap();
        assert_eq!(
            json::write_compact(&json::read(&text).unwrap()["@graph"][0]["raw"]),
            "[0.25]",
            "admitted numeric eligibility on the real RDF caller"
        );
        let reopened =
            purrdf_rdf::native_codecs::jsonld::parse_jsonld(text.as_bytes(), None).unwrap();
        assert!(
            purrdf_rdf::datasets_isomorphic(
                &dataset,
                &reopened,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap(),
            "{text}"
        );
    }

    let input =
        CompiledJsonLdContext::compile(&json::read(r#"{"old":"urn:p"}"#).unwrap(), None).unwrap();
    let output =
        CompiledJsonLdContext::compile(&json::read(r#"{"new":"urn:p"}"#).unwrap(), None).unwrap();
    let expected = json::read(r#"{"@context":{"new":"urn:p"},"new":7.0}"#).unwrap();
    assert_eq!(
        compact_jsonld_with_context(
            br#"{"old":7.0}"#,
            &input,
            &output,
            &policy,
            &expansion,
            &limits
        )
        .unwrap(),
        expected
    );
    assert_eq!(
        compact_yamlld_with_context(
            b"old: 7.0\n",
            &input,
            &output,
            &YamlLdOptions::DEFAULT,
            &policy,
            &expansion,
            &limits
        )
        .unwrap(),
        expected
    );
    let based = CompiledJsonLdContext::compile(
        &json::read(r#"{"@base":"https://example.org/dir/"}"#).unwrap(),
        None,
    )
    .unwrap();
    let expanded = json::read(r#"[{"@id":"https://example.org/dir/item"}]"#).unwrap();
    for (compact_to_relative, id) in [(true, "item"), (false, "https://example.org/dir/item")] {
        let actual = compact_expanded_with_context(
            &expanded,
            &based,
            &JsonLdCompactionPolicy {
                compact_to_relative,
                ..policy
            },
            &limits,
        )
        .unwrap();
        assert_eq!(actual["@id"], id);
        assert_eq!(actual["@context"], *based.canonical_context());
    }
    for malformed in [
        "[1]",
        r#"[["scalar"]]"#,
        r#"[{"urn:p":"scalar"}]"#,
        r#"[{"@id":1}]"#,
        r#"[{"@type":"urn:T"}]"#,
        r#"[{"@value":{},"@type":"urn:T"}]"#,
        r#"[{"@value":"x","@language":"en","@type":"urn:T"}]"#,
        r#"[{"@list":{}}]"#,
        r#"[{"@triple":{"@subject":{"@id":"urn:s"}}}]"#,
        r#"[{"@reverse":{"urn:p":[{"@value":"literal"}]}}]"#,
        r#"[{"@annotation":[1]}]"#,
        r#"[{"@id":"urn:a","@id":"urn:b"}]"#,
    ] {
        let invalid = json::read(malformed).unwrap();
        assert!(
            compact_expanded_with_context(&invalid, &empty, &policy, &limits).is_err(),
            "{malformed}"
        );
        assert!(
            compact_expanded_with_context(
                &json::read(r#"[{"@id":"urn:valid","urn:p":[{"@value":"neighbor"}]}]"#).unwrap(),
                &empty,
                &policy,
                &limits
            )
            .is_ok()
        );
    }
    let source = json::read(
        r#"[{"@id":"urn:valid","urn:p":[{"@value":{"opaque":[1.0]},"@type":"@json"}]}]"#,
    )
    .unwrap();
    assert!(
        compact_expanded_with_context(
            &source,
            &output,
            &policy,
            &JsonLdCompactionLimits {
                max_allocation_bytes: 0
            }
        )
        .is_err()
    );
    assert!(compact_expanded_with_context(&source, &output, &policy, &limits).is_ok());

    // Unchanged pinned local-context compaction and context-bearing flatten output.
    let root = purrdf_testkit::paths::workspace_root().join("vectors/yaml-ld/tests");
    let manifest =
        json::read(&std::fs::read_to_string(root.join("manifest.jsonld")).unwrap()).unwrap();
    for id in [
        "#compact-local-json-ld-context",
        "#compact-local-yaml-ld-context",
        "#flatten",
    ] {
        let entry = manifest["sequence"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["@id"] == id)
            .unwrap();
        let input_path = entry["input"].as_str().unwrap();
        let context_path = entry["context"].as_str().unwrap();
        let context_bytes = std::fs::read(root.join(context_path)).unwrap();
        let context_value = if context_path.ends_with(".jsonld") {
            json::read_slice(&context_bytes, json::Limits::DEFAULT).unwrap()
        } else {
            purrdf_lex::yaml::read(std::str::from_utf8(&context_bytes).unwrap()).unwrap()
        };
        let base = format!("{}{input_path}", manifest["baseIri"].as_str().unwrap());
        let input_context =
            CompiledJsonLdContext::compile(&json::read("{}").unwrap(), Some(&base)).unwrap();
        let output_context = CompiledJsonLdContext::compile(&context_value, Some(&base)).unwrap();
        let options = YamlLdOptions {
            extract_all_scripts: entry["option"]
                .get("extractAllScripts")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            ..YamlLdOptions::DEFAULT
        };
        let policy = JsonLdCompactionPolicy {
            compact_arrays: entry["option"]
                .get("compactArrays")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            ..policy
        };
        let bytes = std::fs::read(root.join(input_path)).unwrap();
        let expected = purrdf_lex::yaml::read(
            &std::fs::read_to_string(root.join(entry["expect"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        let actual = if id == "#flatten" {
            let flattened = flatten_yamlld_with_context(
                &bytes,
                &input_context,
                &options,
                &expansion,
                &JsonLdFlattenLimits::DEFAULT,
            )
            .unwrap();
            compact_expanded_with_context(&flattened, &output_context, &policy, &limits).unwrap()
        } else {
            let json_input = purrdf_lex::yaml::read(std::str::from_utf8(&bytes).unwrap()).unwrap();
            let from_json = compact_jsonld_with_context(
                json::write_compact(&json_input).as_bytes(),
                &input_context,
                &output_context,
                &policy,
                &expansion,
                &limits,
            )
            .unwrap();
            let from_yaml = compact_yamlld_with_context(
                &bytes,
                &input_context,
                &output_context,
                &options,
                &policy,
                &expansion,
                &limits,
            )
            .unwrap();
            assert_eq!(from_json, from_yaml, "{id}: shared JSON/YAML body");
            from_yaml
        };
        assert_eq!(actual, expected, "{id}: complete compacted document");
    }
}

#[test]
fn context_free_flattening_preserves_nodes_and_native_terms() {
    let context =
        CompiledJsonLdContext::compile(&purrdf_lex::json::read("{}").unwrap(), None).unwrap();
    let expansion = JsonLdExpansionLimits::DEFAULT;
    let limits = JsonLdFlattenLimits::DEFAULT;
    for (source, expected) in [
        (
            r#"[
          {"@id":"_:S","@type":["_:T"],"@index":"same",
           "@reverse":{"https://example.org/rev":[{"@id":"_:R","https://example.org/label":"reverse"}]},
           "@graph":[{"@id":"https://example.org/inside","https://example.org/label":"graph"}],
           "@included":[{"@id":"_:I","https://example.org/label":"included"}],
           "_:P":{"https://example.org/label":"anonymous"},
           "https://example.org/empty":[],
           "https://example.org/indexed":{"@value":"value","@index":"slot"},
           "https://example.org/list":[{"@list":["duplicate","duplicate"]},{"@list":["duplicate","duplicate"]}],
           "https://example.org/p":["same","same",{"@id":"_:I","https://example.org/label":"more"}],
           "https://example.org/raw":[{"@value":{"b":2,"a":1,"huge":1e9999999999999999999999},"@type":"@json"},{"@value":{"huge":0.1e10000000000000000000000,"a":1.0,"b":2e0},"@type":"@json"}],
           "https://example.org/text":"_:S"},
          {"@id":"_:S","@index":"same","https://example.org/p":["same","other"]},
          {"@id":"https://example.org/empty-graph","@graph":[]},
          {"@id":"_:only"}
        ]"#,
            r#"[
          {"@id":"_:b1","@type":["_:b0"],"@index":"same",
           "@graph":[{"@id":"https://example.org/inside","https://example.org/label":[{"@value":"graph"}]}],
           "_:b4":[{"@id":"_:b5"}],"https://example.org/empty":[],
           "https://example.org/indexed":[{"@value":"value","@index":"slot"}],
           "https://example.org/list":[{"@list":[{"@value":"duplicate"},{"@value":"duplicate"}]},{"@list":[{"@value":"duplicate"},{"@value":"duplicate"}]}],
           "https://example.org/p":[{"@value":"same"},{"@id":"_:b3"},{"@value":"other"}],
           "https://example.org/raw":[{"@value":{"b":2,"a":1,"huge":1e9999999999999999999999},"@type":"@json"}],
           "https://example.org/text":[{"@value":"_:S"}]},
          {"@id":"_:b2","https://example.org/label":[{"@value":"reverse"}],"https://example.org/rev":[{"@id":"_:b1"}]},
          {"@id":"_:b3","https://example.org/label":[{"@value":"included"},{"@value":"more"}]},
          {"@id":"_:b5","https://example.org/label":[{"@value":"anonymous"}]},
          {"@id":"https://example.org/empty-graph","@graph":[]}
        ]"#,
        ),
        (
            r#"{"@id":"https://example.org/s","https://example.org/p":[
          {"@triple":{"@subject":{"@id":"_:term","https://example.org/label":"term"},"@predicate":"https://example.org/q","@object":{"@triple":{"@subject":{"@id":"_:term"},"@predicate":"https://example.org/r","@object":{"@value":"_:term"}}}},
           "@annotation":[{"@id":"_:ann","https://example.org/note":{"@id":"_:child","https://example.org/label":"child"}}]},
          {"@id":"_:node","https://example.org/label":"nested","@annotation":[{"@id":"_:ann","https://example.org/note":"second"}]},
          {"@list":[{"@id":"_:term"},{"@value":"_:ann"}],"@annotation":[{"@id":"_:ann","https://example.org/note":"list"}]}
        ]}"#,
            r#"[
          {"@id":"_:b1","https://example.org/label":[{"@value":"child"}]},
          {"@id":"_:b2","https://example.org/label":[{"@value":"term"}]},
          {"@id":"_:b3","https://example.org/label":[{"@value":"nested"}]},
          {"@id":"https://example.org/s","https://example.org/p":[
            {"@triple":{"@subject":{"@id":"_:b2"},"@predicate":"https://example.org/q","@object":{"@triple":{"@subject":{"@id":"_:b2"},"@predicate":"https://example.org/r","@object":{"@value":"_:term"}}}},
             "@annotation":[{"@id":"_:b0","https://example.org/note":[{"@id":"_:b1"}]}]},
            {"@id":"_:b3","@annotation":[{"@id":"_:b0","https://example.org/note":[{"@value":"second"}]}]},
            {"@list":[{"@id":"_:b2"},{"@value":"_:ann"}],"@annotation":[{"@id":"_:b0","https://example.org/note":[{"@value":"list"}]}]}
          ]}
        ]"#,
        ),
        (
            r#"{"@id":"urn:a","urn:p":[{"@id":"urn:b","urn:q":[{"@id":"urn:c","@reverse":{"urn:p":[{"@id":"urn:a"}]}}]}]}"#,
            r#"[{"@id":"urn:a","urn:p":[{"@id":"urn:b"},{"@id":"urn:c"}]},{"@id":"urn:b","urn:q":[{"@id":"urn:c"}]}]"#,
        ),
    ] {
        let expected = purrdf_lex::json::read(expected).unwrap();
        let actual =
            flatten_jsonld_with_context(source.as_bytes(), &context, &expansion, &limits).unwrap();
        assert_eq!(actual, expected);
        if let Some(payload) = actual
            .as_array()
            .unwrap()
            .iter()
            .find_map(|node| node.get("https://example.org/raw"))
        {
            let payload = payload.as_array().unwrap()[0].get("@value").unwrap();
            // The existing strict decoder orders members before expansion;
            // node-map construction must retain its original number spellings.
            assert!(
                payload.same_text(
                    &purrdf_lex::json::read(r#"{"a":1,"b":2,"huge":1e9999999999999999999999}"#)
                        .unwrap()
                )
            );
        }
        let yaml = purrdf_lex::yaml::write(&purrdf_lex::json::read(source).unwrap());
        assert_eq!(
            flatten_yamlld_with_context(
                yaml.as_bytes(),
                &context,
                &YamlLdOptions::DEFAULT,
                &expansion,
                &limits
            )
            .unwrap(),
            expected
        );
        assert!(
            flatten_jsonld_with_context(
                source.as_bytes(),
                &context,
                &expansion,
                &JsonLdFlattenLimits {
                    max_allocation_bytes: 0
                }
            )
            .unwrap_err()
            .storage_error()
            .is_some()
        );
    }
    for source in [
        r#"{"@id":"https://example.org/root","https://example.org/p":"value","@annotation":[{"https://example.org/note":"unsupported-root"}]}"#,
        r#"{"@triple":{"@subject":{"@id":"https://example.org/s"},"@predicate":"https://example.org/p","@object":{"@value":"value"}}}"#,
    ] {
        assert!(
            flatten_jsonld_with_context(source.as_bytes(), &context, &expansion, &limits)
                .unwrap_err()
                .message
                .contains("graph roots")
        );
    }
    for indexed in [false, true] {
        let index = if indexed {
            r#", "@index":"semantic""#
        } else {
            ""
        };
        let source = format!(
            r#"{{"@id":"https://example.org/list-owner","https://example.org/list":{{"@list":[{{"@value":"value"}}]{index}}},"https://example.org/raw":{{"@value":{{"@index":"opaque"}},"@type":"@json"}}}}"#
        );
        let expected = purrdf_lex::json::read(r#"[{"@id":"https://example.org/list-owner","https://example.org/list":[{"@list":[{"@value":"value"}]}],"https://example.org/raw":[{"@value":{"@index":"opaque"},"@type":"@json"}]}]"#).unwrap();
        assert_eq!(
            flatten_jsonld_with_context(source.as_bytes(), &context, &expansion, &limits).unwrap(),
            expected
        );
        let rdf = parse_jsonld_with_context(source.as_bytes(), &context);
        if indexed {
            assert!(rdf.unwrap_err().message.contains("@index metadata"));
        } else {
            assert!(rdf.is_ok());
        }
    }
    for (numbers, lexical, datatype) in [
        ("1,1.0,1e0", "1", "integer"),
        ("1.0,1e0,1", "1", "integer"),
        ("1e0,1,1.0", "1", "integer"),
        (
            "999999999999999999999,999999999999999999999.0",
            "999999999999999999999",
            "integer",
        ),
        ("1e21,1000000000000000000000", "1.0E21", "double"),
        ("1000000000000000000000,1e21", "1.0E21", "double"),
    ] {
        let source = format!(
            r#"{{"@id":"https://example.org/numeric","https://example.org/p":[{numbers}]}}"#
        );
        let dataset = parse_jsonld_with_context(source.as_bytes(), &context).unwrap();
        let actual = purrdf_rdf::serialize_dataset(
            &dataset,
            "application/n-quads",
            purrdf_rdf::SerializeGraph::Dataset,
        )
        .unwrap();
        assert_eq!(actual, format!("<https://example.org/numeric> <https://example.org/p> \"{lexical}\"^^<http://www.w3.org/2001/XMLSchema#{datatype}> .\n").into_bytes());
    }
    for (second, succeeds) in [("same", true), ("different", false)] {
        let source = format!(
            r#"[{{"@id":"https://example.org/indexed","@index":"same"}},{{"@id":"https://example.org/indexed","@index":"{second}"}}]"#
        );
        let result = flatten_jsonld_with_context(source.as_bytes(), &context, &expansion, &limits);
        if succeeds {
            assert_eq!(
                result.unwrap(),
                purrdf_lex::json::read(
                    r#"[{"@id":"https://example.org/indexed","@index":"same"}]"#
                )
                .unwrap()
            );
        } else {
            assert!(result.unwrap_err().message.contains("conflicting indexes"));
        }
    }
}

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
    let retrieval_registry = JsonLdContextRegistry::new([(
        "https://doc.example/ctx",
        br#"{"@context":{"p":"https://example.org/p"}}"#.as_slice(),
    )])
    .unwrap();
    for (initial, subject, expected_subject) in [
        (
            r#"{"@base":"https://ids.example/"}"#,
            "subject",
            "https://ids.example/subject",
        ),
        (
            r#"{"@base":null}"#,
            "https://example.org/subject",
            "https://example.org/subject",
        ),
    ] {
        let context = CompiledJsonLdContext::compile_with_registry(
            &purrdf_lex::json::read(initial).unwrap(),
            Some("https://doc.example/file"),
            &retrieval_registry,
        )
        .unwrap();
        let source = format!(r#"{{"@context":"ctx","@id":"{subject}","p":"value"}}"#);
        let mut expected = purrdf_lex::json::read(&format!(
            r#"[{{"@id":"{expected_subject}","https://example.org/p":[{{"@value":"value"}}]}}]"#
        ))
        .unwrap();
        expected.sort_keys();
        assert_eq!(
            expand_jsonld_with_context(
                source.as_bytes(),
                &context,
                &JsonLdExpansionLimits::DEFAULT
            )
            .unwrap(),
            expected
        );
        // JSON is also valid YAML; both configured callers resolve the same URL.
        assert_eq!(
            expand_yamlld_with_context(
                source.as_bytes(),
                &context,
                &YamlLdOptions::DEFAULT,
                &JsonLdExpansionLimits::DEFAULT
            )
            .unwrap(),
            expected
        );
        let expected = parse_dataset(
            format!("<{expected_subject}> <https://example.org/p> \"value\" .\n").as_bytes(),
            "application/n-quads",
            None,
        )
        .unwrap();
        for actual in [
            parse_jsonld_with_context(source.as_bytes(), &context).unwrap(),
            parse_yamlld_with_context(source.as_bytes(), &context, &YamlLdOptions::DEFAULT)
                .unwrap(),
        ] {
            assert!(
                purrdf_rdf::datasets_isomorphic(
                    &actual,
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
    let structural = br#"{
      "@context": {"@vocab":"https://example.org/", "items":{"@container":"@list"},
        "indexed":{"@container":"@index"},"raw":{"@type":"@json"},
        "english":{"@context":{"@language":"en"}}},
      "@id":"same",
      "node":[{"@id":"repeat","label":"one"},{"@id":"repeat","label":"two"},{"label":"anonymous"}],
      "@graph":[{"@id":"inside","label":"graph"}],
      "@included":[{"@id":"included","label":"included"}],
      "@reverse":{"p":{"@id":"back"}},
      "items":[[1,2],[3,4]],"indexed":{"key":{"label":"index"}},
      "english":"text", "raw":{"@id":"literal","ordered":[true,null,1e400]}
    }"#;
    let mut expected = purrdf_lex::json::read(r#"[{
      "@id":"https://example.org/docs/same",
      "@graph":[{"@id":"https://example.org/docs/inside","https://example.org/label":[{"@value":"graph"}]}],
      "@included":[{"@id":"https://example.org/docs/included","https://example.org/label":[{"@value":"included"}]}],
      "@reverse":{"https://example.org/p":[{"@id":"https://example.org/docs/back"}]},
      "https://example.org/node":[{"@id":"https://example.org/docs/repeat","https://example.org/label":[{"@value":"one"}]},{"@id":"https://example.org/docs/repeat","https://example.org/label":[{"@value":"two"}]},{"https://example.org/label":[{"@value":"anonymous"}]}],
      "https://example.org/items":[{"@list":[{"@list":[{"@value":1},{"@value":2}]},{"@list":[{"@value":3},{"@value":4}]}]}],
      "https://example.org/indexed":[{"@index":"key","https://example.org/label":[{"@value":"index"}]}],
      "https://example.org/english":[{"@language":"en","@value":"text"}],
      "https://example.org/raw":[{"@type":"@json","@value":{"@id":"literal","ordered":[true,null,1e400]}}]
    }]"#).unwrap();
    expected.sort_keys();
    assert_eq!(
        expand_jsonld_with_context(structural, &context, &JsonLdExpansionLimits::DEFAULT).unwrap(),
        expected
    );
    assert!(
        expand_jsonld_with_context(
            structural,
            &context,
            &JsonLdExpansionLimits {
                max_allocation_bytes: 0
            }
        )
        .unwrap_err()
        .storage_error()
        .is_some()
    );
    let yaml_structure = purrdf_lex::yaml::write(
        &purrdf_lex::json::read_slice(structural, purrdf_lex::json::Limits::DEFAULT).unwrap(),
    );
    assert_eq!(
        expand_yamlld_with_context(
            yaml_structure.as_bytes(),
            &context,
            &YamlLdOptions::DEFAULT,
            &JsonLdExpansionLimits::DEFAULT
        )
        .unwrap(),
        expected
    );
    let pruned = format!(
        "[{}]",
        std::iter::repeat_n(r#"{"@id":"https://example.org/id"}"#, 512)
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(
        expand_jsonld_with_context(
            pruned.as_bytes(),
            &context,
            &JsonLdExpansionLimits {
                max_allocation_bytes: 16 * 1024
            }
        )
        .unwrap(),
        purrdf_lex::json::Value::Array(Vec::new())
    );
    let graphs = br#"[{"@graph":[{"@id":"https://example.org/s","https://example.org/p":"one"}]},{"@graph":[{"@id":"https://example.org/s","https://example.org/p":"two"}]}]"#;
    let graph_dataset = parse_dataset(graphs, "application/ld+json", None).unwrap();
    let graph_oracle = parse_dataset(b"<https://example.org/s> <https://example.org/p> \"one\" _:first .\n<https://example.org/s> <https://example.org/p> \"two\" _:second .\n", "application/n-quads", None).unwrap();
    assert!(
        purrdf_rdf::datasets_isomorphic(
            &graph_dataset,
            &graph_oracle,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None
            )
        )
        .unwrap()
    );
    for (input, expected) in [
        (
            br#"{"@graph":[{"@id":"https://example.org/s","https://example.org/p":"one"}]}"#
                .as_slice(),
            b"<https://example.org/s> <https://example.org/p> \"one\" .\n".as_slice(),
        ),
        (
            br#"[{"@graph":[{"@id":"https://example.org/s","https://example.org/p":"one"}]}]"#
                .as_slice(),
            b"<https://example.org/s> <https://example.org/p> \"one\" _:graph .\n".as_slice(),
        ),
    ] {
        let actual = parse_dataset(input, "application/ld+json", None).unwrap();
        let expected = parse_dataset(expected, "application/n-quads", None).unwrap();
        assert!(
            purrdf_rdf::datasets_isomorphic(
                &actual,
                &expected,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap()
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
            .unwrap()
            .quads()
            .count(),
        2,
        "an unmapped property is dropped by standard document expansion"
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

/// Three unchanged upstream samples compared as expanded document structures.
#[test]
fn pinned_w3c_structural_expansion_samples() {
    let root = purrdf_testkit::paths::workspace_root().join("vectors/yaml-ld/tests");
    let manifest = purrdf_lex::json::read_slice(
        &std::fs::read(root.join("manifest.jsonld")).unwrap(),
        purrdf_lex::json::Limits::DEFAULT,
    )
    .unwrap();
    for id in [
        "#cr-comments-1-positive",
        "#aa-cycles-1-positive",
        "#two-documents-from-stream",
    ] {
        let entry = manifest["sequence"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["@id"].as_str() == Some(id))
            .expect("pinned structural sample");
        let input = entry["input"].as_str().unwrap();
        let expected = entry["expect"].as_str().unwrap();
        let extract_all_scripts = entry.get("option").is_some_and(|options| {
            assert_eq!(options.as_object().unwrap().len(), 1, "{id}");
            options["extractAllScripts"]
                .as_bool()
                .expect("actual boolean selection")
        });
        let options = YamlLdOptions {
            extract_all_scripts,
            ..YamlLdOptions::DEFAULT
        };
        let base = format!("{}{input}", manifest["baseIri"].as_str().unwrap());
        let context = CompiledJsonLdContext::compile(
            &purrdf_lex::json::Value::Object(purrdf_lex::json::Object::new()),
            Some(&base),
        )
        .unwrap();
        let actual = expand_yamlld_with_context(
            &std::fs::read(root.join(input)).unwrap(),
            &context,
            &options,
            &JsonLdExpansionLimits::DEFAULT,
        )
        .unwrap_or_else(|error| panic!("{id}: {error}"));
        let mut expected =
            purrdf_lex::yaml::read(&std::fs::read_to_string(root.join(expected)).unwrap()).unwrap();
        expected.sort_keys();
        assert_eq!(actual, expected, "{id}: expanded document structure");
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
