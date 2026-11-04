// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
const CONFIG: &str = r#"{"version":1,"name":"caller-yaml-source","profile_version":1,"vocabulary":"https://example.org/yaml#"}"#;

#[test]
fn explicit_profile_schema_preserves_defaults_and_portable_boundaries() {
    let selected = profile(CONFIG.as_bytes()).unwrap();
    assert_eq!(selected.bounds(), Bounds::standard());
    let native = Profile::new(
        "caller-yaml-source",
        1,
        Vocabulary::under("https://example.org/yaml#").unwrap(),
        Bounds::standard(),
    )
    .unwrap();
    assert_eq!(selected.identity(), native.identity());
    let mapped = profile(br#"{"version":1,"name":"mapping","profile_version":7,"vocabulary":"https://example.org/yaml#","bounds":{"syntax":{"max_input_bytes":11,"max_allocation_bytes":12,"max_occurrences":13,"max_depth":14,"aliases":false,"max_documents":15},"max_cover_spans":16,"max_partition_bytes":17,"max_statements":18,"max_payload_bytes":19,"max_projection_bytes":20,"max_decode_bytes":21}}"#).unwrap();
    let expected = Bounds {
        syntax: purrdf_lex::yaml::SyntaxStreamLimits {
            syntax: purrdf_lex::yaml::SyntaxLimits {
                max_input_bytes: 11,
                max_allocation_bytes: 12,
                max_occurrences: 13,
                max_depth: 14,
                aliases: false,
            },
            max_documents: 15,
        },
        max_cover_spans: 16,
        max_partition_bytes: 17,
        max_statements: 18,
        max_payload_bytes: 19,
        max_projection_bytes: 20,
        max_decode_bytes: 21,
    };
    assert_eq!(mapped.bounds(), expected);
    for name in ["max_input_bytes", "max_occurrences"] {
        for value in [
            "0",
            "4294967295",
            "4294967296",
            "-1",
            "1.0",
            "1e0",
            "null",
            "true",
        ] {
            let config = format!(
                "{},\"bounds\":{{\"syntax\":{{\"{name}\":{value}}}}}}}",
                &CONFIG[..CONFIG.len() - 1]
            );
            assert_eq!(
                profile(config.as_bytes()).is_ok(),
                matches!(value, "0" | "4294967295"),
                "{config}"
            );
        }
    }
    let maximum = profile(
        format!(
            "{},\"bounds\":{{\"syntax\":{{\"max_occurrences\":4294967295,\"aliases\":false}}}}}}",
            &CONFIG[..CONFIG.len() - 1]
        )
        .as_bytes(),
    )
    .unwrap();
    assert_eq!(
        maximum.bounds().syntax.syntax.max_occurrences,
        u64::from(u32::MAX)
    );
    assert!(!maximum.bounds().syntax.syntax.aliases);
    for config in [
        "{}".to_owned(),
        CONFIG.replace("\"version\":1", "\"version\":2"),
        CONFIG.replace("\"version\":1", "\"version\":1,\"version\":1"),
        CONFIG.replace("\"profile_version\":1", "\"profile_version\":0"),
        CONFIG.replace("caller-yaml-source", ""),
        CONFIG.replace("https://example.org/yaml#", "relative#"),
        CONFIG.replace("https://example.org/yaml#", "https://example.org/yaml"),
        format!("{},\"bounds\":null}}", &CONFIG[..CONFIG.len() - 1]),
        format!(
            "{},\"bounds\":{{\"syntax\":null}}}}",
            &CONFIG[..CONFIG.len() - 1]
        ),
        format!(
            "{},\"bounds\":{{\"syntax\":{{\"aliases\":null}}}}}}",
            &CONFIG[..CONFIG.len() - 1]
        ),
        format!(
            "{},\"bounds\":{{\"max_numeric_bytes\":1}}}}",
            &CONFIG[..CONFIG.len() - 1]
        ),
        format!("{},\"unknown\":true}}", &CONFIG[..CONFIG.len() - 1]),
    ] {
        assert!(profile(config.as_bytes()).is_err(), "{config}");
    }
    let Error::Configuration(diagnostic) = profile(
        CONFIG
            .replace("\"profile_version\":1", "\"profile_version\":-1")
            .as_bytes(),
    )
    .unwrap_err() else {
        panic!("record failure");
    };
    let original = crate::native_error::to_json(diagnostic.native_cause().unwrap()).unwrap();
    assert_eq!(
        original["fields"]["pointer"]["value"].as_str(),
        Some("/profile_version")
    );
    let Error::Configuration(diagnostic) = profile(b"{").unwrap_err() else {
        panic!("JSON failure");
    };
    assert_eq!(
        crate::native_error::to_json(diagnostic.native_cause().unwrap()).unwrap()["code"].as_str(),
        Some("json-document")
    );
    assert!(profile(&vec![b' '; MAX_PROFILE_BYTES + 1]).is_err());
    for (config, expected) in [
        (
            format!("{},\"unknown\":[[[[]]]]}}", &CONFIG[..CONFIG.len() - 1]),
            "json-depth-limit",
        ),
        (
            format!(
                "{},\"unknown\":[{}]}}",
                &CONFIG[..CONFIG.len() - 1],
                vec!["0"; 65].join(",")
            ),
            "json-value-limit",
        ),
    ] {
        let Error::Configuration(diagnostic) = profile(config.as_bytes()).unwrap_err() else {
            panic!("JSON limit refusal");
        };
        let original =
            purrdf_testkit::error_capture::Capture::read(diagnostic.native_cause().unwrap());
        assert_eq!(original.code, "json-document");
        assert_eq!(original.child("kind").code, expected);
    }
}

#[test]
fn native_source_roundtrip_preserves_roots_profiles_and_original_refusals() {
    let source = b"\xef\xbb\xbf# original\r\n---\r\na: &a [one, 'two']\r\nb: *a\r\n...\r\n---\r\n? [complex, key]\r\n: value\r\n";
    let selected = profile(CONFIG.as_bytes()).unwrap();
    let first = export("https://example.org/source.yaml", source, &selected).unwrap();
    let second = export("https://example.org/source.yaml", source, &selected).unwrap();
    assert_eq!(first.root, second.root);
    let native = purrdf_yaml::analyze(
        SourceDocument {
            id: "https://example.org/source.yaml",
            bytes: source,
        },
        &selected,
    )
    .unwrap();
    assert_eq!(first.root, native.id());
    assert_eq!(
        reconstruct(&first.dataset, &first.root, &selected)
            .unwrap()
            .as_bytes(),
        source
    );
    let serialize = |dataset: &RdfDataset| {
        purrdf_rdf::serialize_dataset(
            dataset,
            "application/n-quads",
            purrdf_rdf::SerializeGraph::Dataset,
        )
        .unwrap()
    };
    let text = serialize(&first.dataset);
    assert_eq!(text, serialize(&second.dataset));
    let parsed = purrdf_rdf::parse_dataset(text.as_bytes(), "application/n-quads", None).unwrap();
    assert_eq!(
        reconstruct(&parsed, &first.root, &selected)
            .unwrap()
            .as_bytes(),
        source
    );
    assert!(matches!(
        reconstruct(&parsed, "https://example.org/other", &selected),
        Err(Error::Native(YamlError::Metadata(_)))
    ));
    let wrong = profile(
        CONFIG
            .replace("caller-yaml-source", "other-profile")
            .as_bytes(),
    )
    .unwrap();
    assert!(reconstruct(&parsed, &first.root, &wrong).is_err());
    let forged = format!(
        "{text}<{}> <https://example.org/yaml#forged> \"extra\" .\n",
        first.root
    );
    let forged = purrdf_rdf::parse_dataset(forged.as_bytes(), "application/n-quads", None).unwrap();
    assert!(reconstruct(&forged, &first.root, &selected).is_err());
    let limited = profile(
        format!(
            "{},\"bounds\":{{\"max_partition_bytes\":0}}}}",
            &CONFIG[..CONFIG.len() - 1]
        )
        .as_bytes(),
    )
    .unwrap();
    assert!(matches!(
        export("https://example.org/source.yaml", source, &limited),
        Err(Error::Native(YamlError::Storage(_)))
    ));
    assert!(matches!(
        export("relative", source, &selected),
        Err(Error::Native(YamlError::Iri(_)))
    ));
    let Err(Error::Native(YamlError::Syntax(cause))) = export(
        "https://example.org/source.yaml",
        b"[unterminated",
        &selected,
    ) else {
        panic!("native syntax failure");
    };
    assert!(cause.line() > 0 && cause.column() > 0);
}
