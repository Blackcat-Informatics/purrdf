// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Replays `boon_differential_vectors.txt`: the verdicts boon 0.6.1 — the
//! validator this crate replaced — gave over the whole official suite and
//! over every (schema, instance) pair the workspace's former boon call sites
//! evaluated. Only boon's answers were kept, never its code (see
//! `PROVENANCE.md`).
//!
//! Every record must agree, with exactly two named exceptions: the suite
//! cases where boon's verdict contradicts the official suite, and the suite is
//! the authority — this crate must give the suite's answer.
//!
//! An exception that stops being needed (the record now agrees) fails too, so
//! the list cannot outlive its reason.

use std::fs;
use std::path::{Path, PathBuf};

use purrdf_jsonschema::{Registry, SchemaError};
use purrdf_testkit::vectors::{VectorFile, decode_str};
use serde_json::Value;

const VECTORS: &str = include_str!("boon_differential_vectors.txt");
const SUITE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/suite");
const FOREIGN_REMOTES: &[&str] = &["draft3", "draft4", "draft6", "draft7", "v1"];

/// Records where boon's verdict contradicts the official suite: `(source,
/// the suite's verdict)`.
const BOON_CONTRADICTS_SUITE: &[(&str, &str)] = &[
    // `1e308` is an integer, hence a multiple of 0.5; boon divided in binary
    // floating point, overflowed to infinity and answered invalid.
    ("suite:optional/float-overflow.json/0/0", "valid"),
    // A meta-schema that declares the Format-Assertion vocabulary (even as
    // `false`) makes `format` assert for an implementation that knows the
    // vocabulary; boon treated `format` as an annotation and answered valid.
    ("suite:optional/format-assertion.json/0/1", "invalid"),
];

fn json_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).expect("remotes directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn suite_registry() -> Registry {
    let root = Path::new(SUITE).join("remotes");
    let mut registry = Registry::new();
    for path in json_files(&root) {
        let name = path
            .strip_prefix(&root)
            .expect("under remotes")
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        if FOREIGN_REMOTES
            .iter()
            .any(|foreign| name.starts_with(&format!("{foreign}/")))
        {
            continue;
        }
        let document: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("remote")).expect("remote JSON");
        registry
            .add_resource(&format!("http://localhost:1234/{name}"), document)
            .expect("remote registers");
    }
    registry
}

fn verdict(
    base: &Registry,
    uri: &str,
    schema: Value,
    instance: &Value,
) -> Result<&'static str, SchemaError> {
    let mut registry = base.clone();
    registry.add_resource(uri, schema)?;
    let compiled = registry.compile(uri)?;
    Ok(if compiled.is_valid(instance) {
        "valid"
    } else {
        "invalid"
    })
}

#[test]
fn every_boon_verdict_is_reproduced_or_named() {
    let vectors = VectorFile::parse(VECTORS).expect("the vector file is intact");
    let suite = suite_registry();
    let plain = Registry::new();
    let mut disagreements = Vec::new();
    let mut exceptions_seen = Vec::new();
    for record in vectors.records() {
        let fields: Vec<String> = record
            .fields
            .iter()
            .map(|field| decode_str(field).expect("text field"))
            .collect();
        let [source, uri, schema, instance, boon] = fields.as_slice() else {
            panic!("line {}: a record has five fields", record.line);
        };
        let schema: Value = serde_json::from_str(schema).expect("schema JSON");
        let instance: Value = serde_json::from_str(instance).expect("instance JSON");
        let base = if source.starts_with("suite:") {
            &suite
        } else {
            &plain
        };
        let ours = verdict(base, uri, schema, &instance);
        let expected = if let Some(&(_, suite_verdict)) = BOON_CONTRADICTS_SUITE
            .iter()
            .find(|(name, _)| name == source)
        {
            exceptions_seen.push(source.clone());
            assert_ne!(
                boon, suite_verdict,
                "{source}: boon now agrees with the suite"
            );
            suite_verdict
        } else {
            boon.as_str()
        };
        match ours {
            Ok(got) if got == expected => {}
            other => disagreements.push(format!(
                "line {} {source}: expected {expected}, got {other:?}",
                record.line
            )),
        }
    }
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
    assert_eq!(
        exceptions_seen.len(),
        BOON_CONTRADICTS_SUITE.len(),
        "every named exception must still be recorded: {exceptions_seen:?}"
    );
    assert_eq!(vectors.records().len(), 1599);
}
