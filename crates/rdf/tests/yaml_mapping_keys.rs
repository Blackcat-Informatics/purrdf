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

use purrdf_rdf::native_codecs::parse_dataset;
use purrdf_rdf::{DatasetSink, OkfBundle, OkfConfig, lift_okf_bundle};

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
