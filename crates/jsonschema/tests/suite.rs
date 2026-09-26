// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The official JSON-Schema-Test-Suite, draft 2020-12, one libtest case per
//! suite test.
//!
//! The vendored tree is `tests/suite/` (see `PROVENANCE.md` for the upstream
//! commit). Every file under `tests/draft2020-12/` is run — `optional/`
//! included, `optional/format/` not vendored because `format` is an
//! annotation under the 2020-12 meta-schema — and every file under
//! `output-tests/draft2020-12/content/`, whose tests validate this crate's
//! `basic` output against the schema the test supplies.
//!
//! The remotes are registered as the suite's README directs, each under
//! `http://localhost:1234/` plus its path below `remotes/`, except the
//! directories that belong to the draft-3/4/6/7 and `v1` test trees.
//!
//! One suite case is not run as written: `optional/cross-draft.json` asks for
//! a draft 2019-09 document to be evaluated under 2019-09 rules, and this
//! crate implements 2020-12 only and refuses every other dialect. That case is
//! marked ignored — the one ledgered case — and a separate case,
//! `dialect-refusal/optional/cross-draft.json/0`, asserts the typed refusal
//! it gets instead. `suite-inventory` pins every count below, so the suite
//! cannot shrink without failing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use purrdf_jsonschema::{OutputFormat, Registry, Schema, SchemaError};
use purrdf_testkit::harness::{self, Failed, Trial};
use serde_json::Value;

const SUITE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/suite");

/// Remote directories that belong to other drafts' test trees.
const FOREIGN_REMOTES: &[&str] = &["draft3", "draft4", "draft6", "draft7", "v1"];

/// The suite cases this crate refuses by design: `(file, group)`.
const DIALECT_REFUSALS: &[(&str, usize)] = &[("optional/cross-draft.json", 0)];

const EXPECTED_FILES: usize = 59;
const EXPECTED_GROUPS: usize = 434;
const EXPECTED_CASES: usize = 1463;
const EXPECTED_OUTPUT_CASES: usize = 4;
const EXPECTED_REMOTES: usize = 47;

fn read_json(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn json_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("{}: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
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

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .expect("under the root")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// A registry holding every remote the 2020-12 suite addresses.
fn remotes() -> (Registry, usize) {
    let root = Path::new(SUITE).join("remotes");
    let mut registry = Registry::new();
    let mut count = 0;
    for path in json_files(&root) {
        let name = relative(&path, &root);
        if FOREIGN_REMOTES
            .iter()
            .any(|foreign| name.starts_with(&format!("{foreign}/")))
        {
            continue;
        }
        registry
            .add_resource(&format!("http://localhost:1234/{name}"), read_json(&path))
            .unwrap_or_else(|error| panic!("remote {name}: {error}"));
        count += 1;
    }
    (registry, count)
}

/// The retrieval URI a suite schema is registered under.
fn retrieval_uri(file: &str, group: usize) -> String {
    format!("https://json-schema.org/tests/suite/draft2020-12/{file}/{group}")
}

fn compile_group(
    base: &Registry,
    file: &str,
    group: usize,
    schema: &Value,
) -> Result<Schema, SchemaError> {
    let mut registry = base.clone();
    let uri = retrieval_uri(file, group);
    registry.add_resource(&uri, schema.clone())?;
    registry.compile(&uri)
}

fn main() -> ExitCode {
    let (base, remote_count) = remotes();
    let mut trials = Vec::new();
    let mut files = 0;
    let mut groups = 0;
    let mut cases = 0;
    let tests_root = Path::new(SUITE).join("tests/draft2020-12");
    for path in json_files(&tests_root) {
        files += 1;
        let file = relative(&path, &tests_root);
        let Value::Array(file_groups) = read_json(&path) else {
            panic!("{file}: a suite file is an array of groups");
        };
        for (group_index, group) in file_groups.iter().enumerate() {
            groups += 1;
            let description = group["description"].as_str().unwrap_or_default().to_owned();
            let refused = DIALECT_REFUSALS.contains(&(file.as_str(), group_index));
            let compiled = Arc::new(compile_group(&base, &file, group_index, &group["schema"]));
            if refused {
                let compiled = Arc::clone(&compiled);
                trials.push(Trial::test(
                    format!("dialect-refusal/{file}/{group_index}"),
                    move || match compiled.as_ref() {
                        Err(SchemaError::UnsupportedDialect { dialect, .. })
                            if dialect == "https://json-schema.org/draft/2019-09/schema" =>
                        {
                            Ok(())
                        }
                        other => Err(Failed::from(format!(
                            "expected the typed 2019-09 dialect refusal, got {other:?}"
                        ))),
                    },
                ));
            }
            let tests = group["tests"].as_array().cloned().unwrap_or_default();
            for (test_index, test) in tests.into_iter().enumerate() {
                cases += 1;
                let compiled = Arc::clone(&compiled);
                let label = format!(
                    "{description} / {}",
                    test["description"].as_str().unwrap_or_default()
                );
                let name = format!("draft2020-12/{file}/{group_index}/{test_index}");
                let trial = Trial::test(name, move || {
                    let expected = test["valid"].as_bool().ok_or("the test has no `valid`")?;
                    let schema = compiled
                        .as_ref()
                        .as_ref()
                        .map_err(|error| format!("{label}: the schema did not compile: {error}"))?;
                    let flag = schema.is_valid(&test["data"]);
                    let evaluated = schema.evaluate(&test["data"]).is_valid();
                    if flag != evaluated {
                        return Err(Failed::from(format!(
                            "{label}: is_valid says {flag} but evaluate says {evaluated}"
                        )));
                    }
                    if flag == expected {
                        Ok(())
                    } else {
                        Err(Failed::from(format!(
                            "{label}: expected valid={expected}, got {flag}"
                        )))
                    }
                });
                trials.push(trial.with_ignored_flag(refused));
            }
        }
    }
    let output_cases = output_trials(&base, &mut trials);
    let counts = (files, groups, cases, output_cases, remote_count);
    trials.push(Trial::test("suite-inventory", move || {
        let expected = (
            EXPECTED_FILES,
            EXPECTED_GROUPS,
            EXPECTED_CASES,
            EXPECTED_OUTPUT_CASES,
            EXPECTED_REMOTES,
        );
        if counts == expected {
            Ok(())
        } else {
            Err(Failed::from(format!(
                "(files, groups, cases, output cases, remotes) = {counts:?}, expected {expected:?}"
            )))
        }
    }));
    harness::main(trials)
}

/// The output tests: evaluate, write `basic`, and validate it against the
/// test's own schema (which references the suite's output meta-schema).
fn output_trials(base: &Registry, trials: &mut Vec<Trial>) -> usize {
    let root = Path::new(SUITE).join("output-tests/draft2020-12");
    let mut registry = base.clone();
    registry
        .add_resource(
            "https://json-schema.org/draft/2020-12/output/schema",
            read_json(&root.join("output-schema.json")),
        )
        .expect("the output schema registers");
    let registry = Arc::new(registry);
    let content = root.join("content");
    let mut count = 0;
    for path in json_files(&content) {
        let file = relative(&path, &content);
        let Value::Array(groups) = read_json(&path) else {
            panic!("{file}: an output test file is an array of groups");
        };
        for (group_index, group) in groups.into_iter().enumerate() {
            let tests = group["tests"].as_array().cloned().unwrap_or_default();
            for (test_index, test) in tests.into_iter().enumerate() {
                count += 1;
                let registry = Arc::clone(&registry);
                let schema = group["schema"].clone();
                let name = format!("output/draft2020-12/content/{file}/{group_index}/{test_index}");
                let retrieval =
                    format!("https://json-schema.org/tests/suite/output/{file}/{group_index}");
                trials.push(Trial::test(name, move || {
                    let mut registry = registry.as_ref().clone();
                    registry
                        .add_resource(&retrieval, schema)
                        .map_err(|error| format!("schema: {error}"))?;
                    let compiled = registry
                        .compile(&retrieval)
                        .map_err(|error| format!("schema: {error}"))?;
                    let basic = compiled
                        .evaluate(&test["data"])
                        .to_json(OutputFormat::Basic);
                    let Some(checker) = test["output"].get("basic") else {
                        return Err(Failed::from("the test has no basic output schema"));
                    };
                    let checker_uri = checker["$id"]
                        .as_str()
                        .ok_or("the output schema has no $id")?
                        .to_owned();
                    registry
                        .add_resource(&checker_uri, checker.clone())
                        .map_err(|error| format!("output schema: {error}"))?;
                    let checker = registry
                        .compile(&checker_uri)
                        .map_err(|error| format!("output schema: {error}"))?;
                    let verdict = checker.evaluate(&basic);
                    if verdict.is_valid() {
                        Ok(())
                    } else {
                        Err(Failed::from(format!(
                            "basic output {basic} fails its output schema: {}",
                            verdict.to_json(OutputFormat::Basic)
                        )))
                    }
                }));
            }
        }
    }
    count
}
