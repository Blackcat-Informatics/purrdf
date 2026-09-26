// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The runner behind the official JSON-Schema-Test-Suite targets: one libtest
//! case per suite test, for one draft's test tree.
//!
//! The vendored tree is `tests/suite/` (see `PROVENANCE.md` for the upstream
//! commit). Every file under `tests/<draft>/` is run — `optional/` included,
//! `optional/format/` not vendored because it tests `format` as an
//! assertion, which none of the three meta-schemas makes it — and, where the
//! draft has them, every file under `output-tests/<draft>/content/`, whose
//! tests validate this crate's `basic` output against the schema the test
//! supplies.
//!
//! The remotes are registered as the suite's README directs, each under
//! `http://localhost:1234/` plus its path below `remotes/`, except the
//! directories that belong to the draft-3/4/6 and `v1` test trees. A remote
//! that declares no `$schema` is read in the dialect of the directory it sits
//! in, or, outside a draft directory, in the dialect of the draft under test:
//! the suite's README makes the draft under test the default dialect, and a
//! test schema that declares no `$schema` is read in it too.
//!
//! Nothing is ignored and nothing is expected to fail. A `suite-inventory`
//! case pins the file, group, case, output-case and remote counts, so the
//! suite cannot shrink without failing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use purrdf_jsonschema::{
    DRAFT_07, DRAFT_2019_09, DRAFT_2020_12, OutputFormat, Registry, Schema, SchemaError,
};
use purrdf_testkit::harness::{self, Failed, Trial};
use serde_json::Value;

const SUITE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/suite");

/// Remote directories that belong to the test trees of dialects this crate
/// does not implement.
const FOREIGN_REMOTES: &[&str] = &["draft3", "draft4", "draft6", "v1"];

/// Remote directories and the dialect a remote in them is read in when it
/// declares none.
const REMOTE_DIALECTS: &[(&str, &str)] = &[
    ("draft2020-12", DRAFT_2020_12),
    ("draft2019-09", DRAFT_2019_09),
    ("draft7", DRAFT_07),
];

/// One draft's test tree, and the counts it must have.
pub(crate) struct Draft {
    /// The directory name under `tests/` (and `output-tests/`).
    pub(crate) directory: &'static str,
    /// The dialect's meta-schema URI: the default dialect of the run.
    pub(crate) metaschema: &'static str,
    /// Whether `output-tests/<directory>/` exists upstream.
    pub(crate) output_tests: bool,
    pub(crate) expected_files: usize,
    pub(crate) expected_groups: usize,
    pub(crate) expected_cases: usize,
    pub(crate) expected_output_cases: usize,
    pub(crate) expected_remotes: usize,
}

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

/// A registry holding every remote the suite addresses, with the draft under
/// test as the default dialect.
fn remotes(draft: &Draft) -> (Registry, usize) {
    let root = Path::new(SUITE).join("remotes");
    let mut registry = Registry::new();
    let mut count = 0;
    for path in json_files(&root) {
        let name = relative(&path, &root);
        let in_directory = |directory: &str| name.starts_with(&format!("{directory}/"));
        if FOREIGN_REMOTES.iter().any(|foreign| in_directory(foreign)) {
            continue;
        }
        let dialect = REMOTE_DIALECTS
            .iter()
            .find(|(directory, _)| in_directory(directory))
            .map_or(draft.metaschema, |&(_, dialect)| dialect);
        registry
            .set_default_dialect(dialect)
            .expect("an implemented dialect");
        registry
            .add_resource(&format!("http://localhost:1234/{name}"), read_json(&path))
            .unwrap_or_else(|error| panic!("remote {name}: {error}"));
        count += 1;
    }
    registry
        .set_default_dialect(draft.metaschema)
        .expect("an implemented dialect");
    (registry, count)
}

fn compile_group(base: &Registry, uri: &str, schema: &Value) -> Result<Schema, SchemaError> {
    let mut registry = base.clone();
    registry.add_resource(uri, schema.clone())?;
    registry.compile(uri)
}

/// Run every case of `draft`'s test tree.
pub(crate) fn run(draft: &'static Draft) -> ExitCode {
    let (base, remote_count) = remotes(draft);
    let mut trials = Vec::new();
    let mut files = 0;
    let mut groups = 0;
    let mut cases = 0;
    let tests_root = Path::new(SUITE).join("tests").join(draft.directory);
    for path in json_files(&tests_root) {
        files += 1;
        let file = relative(&path, &tests_root);
        let Value::Array(file_groups) = read_json(&path) else {
            panic!("{file}: a suite file is an array of groups");
        };
        for (group_index, group) in file_groups.iter().enumerate() {
            groups += 1;
            let description = group["description"].as_str().unwrap_or_default().to_owned();
            let uri = format!(
                "https://json-schema.org/tests/suite/{}/{file}/{group_index}",
                draft.directory
            );
            let compiled = Arc::new(compile_group(&base, &uri, &group["schema"]));
            let tests = group["tests"].as_array().cloned().unwrap_or_default();
            for (test_index, test) in tests.into_iter().enumerate() {
                cases += 1;
                let compiled = Arc::clone(&compiled);
                let label = format!(
                    "{description} / {}",
                    test["description"].as_str().unwrap_or_default()
                );
                let name = format!("{}/{file}/{group_index}/{test_index}", draft.directory);
                trials.push(Trial::test(name, move || {
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
                }));
            }
        }
    }
    let output_cases = if draft.output_tests {
        output_trials(draft, &base, &mut trials)
    } else {
        0
    };
    let counts = (files, groups, cases, output_cases, remote_count);
    trials.push(Trial::test("suite-inventory", move || {
        let expected = (
            draft.expected_files,
            draft.expected_groups,
            draft.expected_cases,
            draft.expected_output_cases,
            draft.expected_remotes,
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
fn output_trials(draft: &Draft, base: &Registry, trials: &mut Vec<Trial>) -> usize {
    let root = Path::new(SUITE).join("output-tests").join(draft.directory);
    let mut registry = base.clone();
    let output_schema = read_json(&root.join("output-schema.json"));
    let output_uri = output_schema["$id"]
        .as_str()
        .expect("the output schema has an $id")
        .to_owned();
    registry
        .add_resource(&output_uri, output_schema)
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
                let name = format!(
                    "output/{}/content/{file}/{group_index}/{test_index}",
                    draft.directory
                );
                let retrieval = format!(
                    "https://json-schema.org/tests/suite/output/{}/{file}/{group_index}",
                    draft.directory
                );
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
