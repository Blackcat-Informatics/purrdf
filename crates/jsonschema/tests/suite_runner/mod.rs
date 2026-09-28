// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The runner behind the three suite targets: one libtest case per test of
//! the official JSON-Schema-Test-Suite for one draft.
//!
//! The vendored tree is `tests/suite/` (see `PROVENANCE.md` for the upstream
//! commit). Every file under `tests/<draft>/` runs, `optional/` included.
//! `optional/format/` tests `format` as an assertion, so its files compile in
//! a registry with [`Registry::set_format_assertion`] on; everything else
//! runs with the default (annotation). Output tests, where the draft has
//! them, validate this crate's `basic` output against the schema each test
//! supplies.
//!
//! Every registry starts from one [`Metaschemas`] set holding the vendored
//! meta-schemas of all three drafts (`tests/metaschemas/`, supplied through
//! `purrdf_testkit::jsonschema_metaschemas`): this crate carries none. The
//! remotes are registered as the suite's README directs, each under
//! `http://localhost:1234/` plus its path below `remotes/` — the
//! draft-specific directories of the three supported drafts in their own
//! dialects, so a cross-draft `$ref` reaches a document of the right draft,
//! and the shared ones in the draft under test. The draft-3/4/6 and `v1`
//! directories belong to dialects this crate refuses and are not registered.
//!
//! The `suite-inventory` case pins every count, so the suite cannot shrink
//! without failing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use purrdf_jsonschema::{Dialect, Metaschemas, OutputFormat, Registry, Schema, SchemaError};
use purrdf_testkit::harness::{self, Failed, Trial};
use serde_json::Value;

const SUITE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/suite");

/// The remote directories of the supported drafts, and their dialects.
const DRAFT_REMOTES: &[(&str, Dialect)] = &[
    ("draft7", Dialect::Draft07),
    ("draft2019-09", Dialect::Draft2019_09),
    ("draft2020-12", Dialect::Draft2020_12),
];

/// Remote directories that belong to dialects this crate refuses.
const FOREIGN_REMOTES: &[&str] = &["draft3", "draft4", "draft6", "v1"];

/// Every count the run must reproduce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Inventory {
    /// Test files, `optional/format/` included.
    pub(crate) files: usize,
    /// Test groups (one schema each).
    pub(crate) groups: usize,
    /// Validation cases outside `optional/format/`.
    pub(crate) cases: usize,
    /// Validation cases in `optional/format/`.
    pub(crate) format_cases: usize,
    /// Output-format cases.
    pub(crate) output_cases: usize,
    /// Registered remote documents.
    pub(crate) remotes: usize,
}

/// One draft's suite.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Draft {
    /// The directory name under `tests/` and `output-tests/`.
    pub(crate) directory: &'static str,
    /// The dialect of a test schema without `$schema`.
    pub(crate) dialect: Dialect,
    /// Whether `output-tests/<directory>/` exists.
    pub(crate) output_tests: bool,
    /// The pinned counts.
    pub(crate) expected: Inventory,
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

/// The vendored meta-schemas of all three drafts, as one shared set.
pub(crate) fn metaschemas() -> Metaschemas {
    Metaschemas::new(
        purrdf_testkit::jsonschema_metaschemas::all().map(|(uri, text)| {
            let document: Value = serde_json::from_str(text).expect("meta-schema JSON");
            (uri, document)
        }),
    )
    .expect("the vendored meta-schemas form a complete set")
}

/// A registry holding the meta-schemas and every remote a suite of
/// `dialect` addresses, and the number of remotes registered.
pub(crate) fn remotes(metaschemas: &Metaschemas, dialect: Dialect) -> (Registry, usize) {
    let root = Path::new(SUITE).join("remotes");
    let mut registry = Registry::with_metaschemas(metaschemas);
    registry.set_default_dialect(dialect);
    let mut count = 0;
    for path in json_files(&root) {
        let name = relative(&path, &root);
        let directory = name.split_once('/').map(|(directory, _)| directory);
        if directory.is_some_and(|directory| FOREIGN_REMOTES.contains(&directory)) {
            continue;
        }
        let remote_dialect = DRAFT_REMOTES
            .iter()
            .find(|(draft, _)| directory == Some(draft))
            .map_or(dialect, |&(_, dialect)| dialect);
        registry
            .add_resource_with_dialect(
                &format!("http://localhost:1234/{name}"),
                read_json(&path),
                remote_dialect,
            )
            .unwrap_or_else(|error| panic!("remote {name}: {error}"));
        count += 1;
    }
    (registry, count)
}

fn compile_group(
    base: &Registry,
    uri: &str,
    schema: &Value,
    format_assertion: bool,
) -> Result<Schema, SchemaError> {
    let mut registry = base.clone();
    registry.set_format_assertion(format_assertion);
    registry.add_resource(uri, schema.clone())?;
    registry.compile(uri)
}

/// Run `draft`'s suite as a libtest-compatible target.
pub(crate) fn run(draft: Draft) -> ExitCode {
    let metaschemas = metaschemas();
    let (base, remotes) = remotes(&metaschemas, draft.dialect);
    let mut trials = Vec::new();
    let mut inventory = Inventory {
        files: 0,
        groups: 0,
        cases: 0,
        format_cases: 0,
        output_cases: 0,
        remotes,
    };
    let tests_root = Path::new(SUITE).join("tests").join(draft.directory);
    for path in json_files(&tests_root) {
        inventory.files += 1;
        let file = relative(&path, &tests_root);
        let format_assertion = file.starts_with("optional/format/");
        let Value::Array(file_groups) = read_json(&path) else {
            panic!("{file}: a suite file is an array of groups");
        };
        for (group_index, group) in file_groups.iter().enumerate() {
            inventory.groups += 1;
            let description = group["description"].as_str().unwrap_or_default().to_owned();
            let uri = format!(
                "https://json-schema.org/tests/suite/{}/{file}/{group_index}",
                draft.directory
            );
            let compiled = Arc::new(compile_group(
                &base,
                &uri,
                &group["schema"],
                format_assertion,
            ));
            let tests = group["tests"].as_array().cloned().unwrap_or_default();
            for (test_index, test) in tests.into_iter().enumerate() {
                if format_assertion {
                    inventory.format_cases += 1;
                } else {
                    inventory.cases += 1;
                }
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
                    let flag = schema.is_valid(&test["data"]).expect("evaluation");
                    let evaluated = schema
                        .evaluate(&test["data"])
                        .expect("evaluation")
                        .is_valid();
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
    if draft.output_tests {
        inventory.output_cases = output_trials(&base, draft.directory, &mut trials);
    }
    let expected = draft.expected;
    trials.push(Trial::test("suite-inventory", move || {
        if inventory == expected {
            Ok(())
        } else {
            Err(Failed::from(format!(
                "counted {inventory:?}, expected {expected:?}"
            )))
        }
    }));
    harness::main(trials)
}

/// The output tests: evaluate, write `basic`, and validate it against the
/// test's own schema (which references the suite's output meta-schema).
fn output_trials(base: &Registry, directory: &str, trials: &mut Vec<Trial>) -> usize {
    let root = Path::new(SUITE).join("output-tests").join(directory);
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
                let name = format!("output/{directory}/content/{file}/{group_index}/{test_index}");
                let retrieval = format!(
                    "https://json-schema.org/tests/suite/output/{directory}/{file}/{group_index}"
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
                        .expect("evaluation")
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
                    let verdict = checker.evaluate(&basic).expect("evaluation");
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
