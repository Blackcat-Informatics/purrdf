// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Offline conformance gate for the revision-pinned W3C CSVW corpus.
//!
//! Every manifest case runs. Expected failures, if any are introduced, must be
//! listed with a reason in [`XFAIL`]; an unexpected pass is itself a hard failure
//! so the ledger cannot become stale.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use purrdf_rdf::{
    CsvwAction, CsvwConfig, CsvwContext, CsvwInput, CsvwMode, CsvwVocabulary, ProjectionLimits,
    canonicalize, datasets_isomorphic, parse_dataset, read_csvw,
};
use serde_json::Value;

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/csvw-w3c");
const BASE: &str = "http://www.w3.org/2013/csvw/tests/";

/// Case IRI suffix and a mandatory reason for every expected failure.
const XFAIL: &[(&str, &str)] = &[];

#[test]
fn upstream_csvw_rdf_manifest() {
    let manifest: Value = serde_json::from_slice(
        &fs::read(Path::new(CORPUS).join("manifest-rdf.jsonld")).expect("manifest"),
    )
    .expect("manifest JSON");
    let mut failures = Vec::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut approvals = BTreeMap::<String, usize>::new();
    let mut observed_xfails = BTreeSet::new();
    let selected = std::env::var("CSVW_CASE").ok();
    for entry in manifest["entries"].as_array().expect("entries") {
        let id = entry["id"].as_str().expect("id");
        let kind = entry["type"].as_str().expect("type");
        let approval = entry["approval"].as_str().expect("approval");
        *counts.entry(kind.to_owned()).or_default() += 1;
        *approvals.entry(approval.to_owned()).or_default() += 1;
        if selected
            .as_deref()
            .is_some_and(|selected| !id.ends_with(selected))
        {
            continue;
        }
        record_case_result(id, run_case(entry), &mut failures, &mut observed_xfails);
    }
    assert_eq!(counts.get("csvt:NegativeRdfTest"), Some(&58));
    assert_eq!(counts.get("csvt:ToRdfTest"), Some(&76));
    assert_eq!(counts.get("csvt:ToRdfTestWithWarnings"), Some(&136));
    assert_eq!(
        approvals,
        BTreeMap::from([("rdft:Approved".to_owned(), 270)])
    );
    assert_xfail_inventory(&observed_xfails, selected.as_deref());
    assert!(
        failures.is_empty(),
        "{} CSVW RDF mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn upstream_csvw_validation_manifest() {
    let manifest: Value = serde_json::from_slice(
        &fs::read(Path::new(CORPUS).join("manifest-validation.jsonld")).expect("manifest"),
    )
    .expect("manifest JSON");
    let mut failures = Vec::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut approvals = BTreeMap::<String, usize>::new();
    let mut observed_xfails = BTreeSet::new();
    let selected = std::env::var("CSVW_CASE").ok();
    for entry in manifest["entries"].as_array().expect("entries") {
        let id = entry["id"].as_str().expect("id");
        let kind = entry["type"].as_str().expect("type");
        let approval = entry["approval"].as_str().expect("approval");
        *counts.entry(kind.to_owned()).or_default() += 1;
        *approvals.entry(approval.to_owned()).or_default() += 1;
        if selected
            .as_deref()
            .is_some_and(|selected| !id.ends_with(selected))
        {
            continue;
        }
        record_case_result(
            id,
            run_validation_case(entry),
            &mut failures,
            &mut observed_xfails,
        );
    }
    assert_eq!(counts.get("csvt:NegativeValidationTest"), Some(&145));
    assert_eq!(counts.get("csvt:PositiveValidationTest"), Some(&76));
    assert_eq!(counts.get("csvt:WarningValidationTest"), Some(&61));
    assert_eq!(
        approvals,
        BTreeMap::from([
            ("rdft:Approved".to_owned(), 281),
            ("rdft:Proposed".to_owned(), 1),
        ])
    );
    assert_xfail_inventory(&observed_xfails, selected.as_deref());
    assert!(
        failures.is_empty(),
        "{} CSVW validation mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn record_case_result(
    id: &str,
    result: Result<(), String>,
    failures: &mut Vec<String>,
    observed_xfails: &mut BTreeSet<&'static str>,
) {
    let expected = XFAIL.iter().find(|(suffix, _)| id.ends_with(suffix));
    match (result, expected) {
        (Ok(()), None) => {}
        (Ok(()), Some((suffix, reason))) => failures.push(format!(
            "{id}: unexpected pass for XFAIL `{suffix}` ({reason})"
        )),
        (Err(error), None) => failures.push(format!("{id}: {error}")),
        (Err(_), Some((suffix, _))) => {
            observed_xfails.insert(suffix);
        }
    }
}

fn assert_xfail_inventory(observed: &BTreeSet<&'static str>, selected: Option<&str>) {
    if selected.is_some() {
        return;
    }
    let declared = XFAIL
        .iter()
        .map(|(suffix, _)| *suffix)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        observed, &declared,
        "stale or undiscovered CSVW XFAIL entry"
    );
}

fn run_validation_case(entry: &Value) -> Result<(), String> {
    let action_path = entry["action"].as_str().ok_or("missing action")?;
    let action_iri = resolve(BASE, action_path)?;
    let implicit =
        entry
            .get("implicit")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            });
    let option_metadata = entry
        .get("option")
        .and_then(|value| value.get("metadata"))
        .and_then(Value::as_str);
    let linked_metadata = entry
        .get("httpLink")
        .and_then(Value::as_str)
        .and_then(|value| value.strip_prefix('<'))
        .and_then(|value| value.split_once('>'))
        .map(|(value, _)| value);
    let mut discovery_warning = false;
    let metadata_path = if let Some(path) = option_metadata {
        Some(path.to_owned())
    } else if is_json_path(action_path) {
        None
    } else {
        let linked_path = linked_metadata.map(|path| resolve_path(action_path, path));
        let mut candidates = linked_path.iter().cloned().collect::<Vec<_>>();
        candidates.extend(
            implicit
                .iter()
                .filter(|path| is_json_path(path))
                .filter(|path| Some(*path) != linked_path.as_ref())
                .cloned(),
        );
        let mut selected = None;
        for candidate in candidates {
            if metadata_describes(&candidate, &action_iri) {
                selected = Some(candidate);
                break;
            }
            discovery_warning = true;
        }
        selected
    };
    let mut paths = implicit;
    paths.push(action_path.to_owned());
    if let Some(path) = &metadata_path {
        paths.push(path.clone());
    }
    paths.sort();
    paths.dedup();
    let mut resources = BTreeMap::new();
    for path in paths {
        let physical_end = [path.find('?'), path.find('#')]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(path.len());
        let file = Path::new(CORPUS).join(&path[..physical_end]);
        if file.is_file() {
            resources.insert(
                resolve(BASE, &path)?,
                fs::read(&file).map_err(|error| error.to_string())?,
            );
        }
    }
    let action = if let Some(metadata) = option_metadata {
        CsvwAction::Metadata {
            metadata_iri: resolve(BASE, metadata)?,
        }
    } else if is_json_path(action_path) {
        CsvwAction::Metadata {
            metadata_iri: action_iri,
        }
    } else {
        CsvwAction::Table {
            table_iri: action_iri,
            metadata_iri: metadata_path
                .as_deref()
                .map(|path| resolve(BASE, path))
                .transpose()?,
        }
    };
    let config = config(CsvwMode::Standard);
    let input = CsvwInput::new(action, resources, config.limits()).map_err(|e| e.to_string())?;
    let outcome = read_csvw(&input, &config);
    let kind = entry["type"].as_str().ok_or("missing type")?;
    match kind {
        "csvt:PositiveValidationTest" => match outcome {
            Ok(outcome) if outcome.is_valid() => Ok(()),
            Ok(_) => Err("positive validation test reported invalid".to_owned()),
            Err(error) => Err(format!("positive validation test failed: {error}")),
        },
        "csvt:WarningValidationTest" => match outcome {
            Ok(outcome)
                if outcome.is_valid() && (!outcome.warnings.is_empty() || discovery_warning) =>
            {
                Ok(())
            }
            Ok(outcome) if !outcome.is_valid() => {
                Err("warning validation test reported invalid".to_owned())
            }
            Ok(_) => Err("warning validation test produced no warning".to_owned()),
            Err(error) => Err(format!("warning validation test failed: {error}")),
        },
        "csvt:NegativeValidationTest" => match outcome {
            Err(_) => Ok(()),
            Ok(outcome) if !outcome.is_valid() => Ok(()),
            Ok(_) => Err("negative validation test reported valid".to_owned()),
        },
        _ => Err(format!("unknown validation test type `{kind}`")),
    }
}

fn run_case(entry: &Value) -> Result<(), String> {
    let action_path = entry["action"].as_str().ok_or("missing action")?;
    let action_iri = resolve(BASE, action_path)?;
    let implicit =
        entry
            .get("implicit")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            });
    let option_metadata = entry
        .get("option")
        .and_then(|value| value.get("metadata"))
        .and_then(Value::as_str);
    let linked_metadata = entry
        .get("httpLink")
        .and_then(Value::as_str)
        .and_then(|value| value.strip_prefix('<'))
        .and_then(|value| value.split_once('>'))
        .map(|(value, _)| value);
    let mut discovery_warning = false;
    let metadata_path = if let Some(path) = option_metadata {
        Some(path.to_owned())
    } else if is_json_path(action_path) {
        None
    } else {
        let linked_path = linked_metadata.map(|path| resolve_path(action_path, path));
        let mut candidates = linked_path.iter().cloned().collect::<Vec<_>>();
        candidates.extend(
            implicit
                .iter()
                .filter(|path| is_json_path(path))
                .filter(|path| Some(*path) != linked_path.as_ref())
                .cloned(),
        );
        let mut selected = None;
        for candidate in candidates {
            if metadata_describes(&candidate, &action_iri) {
                selected = Some(candidate);
                break;
            }
            discovery_warning = true;
        }
        selected
    };
    let mut paths = implicit;
    paths.push(action_path.to_owned());
    if let Some(path) = &metadata_path {
        paths.push(path.clone());
    }
    paths.sort();
    paths.dedup();
    let mut resources = BTreeMap::new();
    for path in paths {
        let physical_end = [path.find('?'), path.find('#')]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(path.len());
        let file = Path::new(CORPUS).join(&path[..physical_end]);
        if file.is_file() {
            resources.insert(
                resolve(BASE, &path)?,
                fs::read(&file).map_err(|e| e.to_string())?,
            );
        }
    }
    let action = if let Some(metadata) = option_metadata {
        CsvwAction::Metadata {
            metadata_iri: resolve(BASE, metadata)?,
        }
    } else if is_json_path(action_path) {
        CsvwAction::Metadata {
            metadata_iri: action_iri,
        }
    } else {
        CsvwAction::Table {
            table_iri: action_iri,
            metadata_iri: metadata_path
                .as_deref()
                .map(|path| resolve(BASE, path))
                .transpose()?,
        }
    };
    let mode = if entry
        .get("option")
        .and_then(|value| value.get("minimal"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        CsvwMode::Minimal
    } else {
        CsvwMode::Standard
    };
    let config = config(mode);
    let input = CsvwInput::new(action, resources, config.limits()).map_err(|e| e.to_string())?;
    let actual = read_csvw(&input, &config);
    let kind = entry["type"].as_str().ok_or("missing type")?;
    if kind == "csvt:NegativeRdfTest" {
        return if actual.is_err() {
            Ok(())
        } else {
            Err("negative test unexpectedly succeeded".to_owned())
        };
    }
    let actual = actual.map_err(|error| error.to_string())?;
    if kind == "csvt:ToRdfTestWithWarnings" && actual.warnings.is_empty() && !discovery_warning {
        return Err("warning test produced no warning".to_owned());
    }
    let result_path = entry["result"].as_str().ok_or("missing result")?;
    let expected_iri = resolve(BASE, result_path)?;
    let expected = parse_dataset(
        &fs::read(Path::new(CORPUS).join(result_path)).map_err(|e| e.to_string())?,
        "text/turtle",
        Some(&expected_iri),
    )
    .map_err(|error| error.to_string())?;
    if datasets_isomorphic(&actual.dataset, &expected) {
        Ok(())
    } else {
        let summary = format!(
            "RDF mismatch ({} warnings, {} rows)",
            actual.warnings.len(),
            actual
                .group
                .tables
                .iter()
                .map(|table| table.rows.len())
                .sum::<usize>(),
        );
        if std::env::var_os("CSVW_CASE").is_some() {
            Err(format!(
                "{summary}\nACTUAL:\n{}EXPECTED:\n{}",
                canonicalize(&actual.dataset).nquads,
                canonicalize(&expected).nquads,
            ))
        } else {
            Err(summary)
        }
    }
}

fn metadata_describes(path: &str, action_iri: &str) -> bool {
    let Ok(bytes) = fs::read(Path::new(CORPUS).join(path)) else {
        return false;
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return false;
    };
    let base = resolve(BASE, path).unwrap_or_else(|_| BASE.to_owned());
    let urls: Vec<&str> = value.get("tables").and_then(Value::as_array).map_or_else(
        || {
            value
                .get("url")
                .and_then(Value::as_str)
                .into_iter()
                .collect()
        },
        |tables| {
            tables
                .iter()
                .filter_map(|table| table.get("url").and_then(Value::as_str))
                .collect()
        },
    );
    urls.into_iter()
        .filter_map(|url| resolve(&base, url).ok())
        .any(|url| url == action_iri)
}

fn is_json_path(path: &str) -> bool {
    let physical_end = [path.find('?'), path.find('#')]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(path.len());
    Path::new(&path[..physical_end])
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

fn resolve(base: &str, reference: &str) -> Result<String, String> {
    purrdf_iri::parse(base)
        .map_err(|error| error.to_string())?
        .resolve(reference)
        .map(|iri| iri.as_str().to_owned())
        .map_err(|error| error.to_string())
}

fn resolve_path(action: &str, reference: &str) -> String {
    let mut path = PathBuf::from(action);
    path.pop();
    path.push(reference);
    path.to_string_lossy().into_owned()
}

fn config(mode: CsvwMode) -> CsvwConfig {
    let prefixes = BTreeMap::from([
        ("csvw".to_owned(), "http://www.w3.org/ns/csvw#".to_owned()),
        ("dc".to_owned(), "http://purl.org/dc/terms/".to_owned()),
        (
            "dc11".to_owned(),
            "http://purl.org/dc/elements/1.1/".to_owned(),
        ),
        ("dcat".to_owned(), "http://www.w3.org/ns/dcat#".to_owned()),
        ("dcterms".to_owned(), "http://purl.org/dc/terms/".to_owned()),
        ("foaf".to_owned(), "http://xmlns.com/foaf/0.1/".to_owned()),
        ("oa".to_owned(), "http://www.w3.org/ns/oa#".to_owned()),
        ("org".to_owned(), "http://www.w3.org/ns/org#".to_owned()),
        (
            "owl".to_owned(),
            "http://www.w3.org/2002/07/owl#".to_owned(),
        ),
        (
            "rdf".to_owned(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_owned(),
        ),
        (
            "rdfs".to_owned(),
            "http://www.w3.org/2000/01/rdf-schema#".to_owned(),
        ),
        ("schema".to_owned(), "http://schema.org/".to_owned()),
        (
            "skos".to_owned(),
            "http://www.w3.org/2004/02/skos/core#".to_owned(),
        ),
        (
            "xsd".to_owned(),
            "http://www.w3.org/2001/XMLSchema#".to_owned(),
        ),
    ]);
    CsvwConfig::new(
        BASE,
        CsvwContext::new("http://www.w3.org/ns/csvw", prefixes).expect("context"),
        "http://example.org/purrdf/csvw-test-group",
        CsvwVocabulary::new(
            "http://www.w3.org/ns/csvw#",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
            "http://www.w3.org/2000/01/rdf-schema#",
            "http://www.w3.org/2001/XMLSchema#",
        )
        .expect("vocabulary"),
        mode,
        ProjectionLimits::new(128, 8_000_000, 24_000_000, 32_000_000, 16).expect("limits"),
        100_000,
    )
    .expect("config")
}
