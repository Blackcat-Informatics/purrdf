// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Read-only assembly report aggregation against an actual producer receipt.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::process::ExitCode;

use purrdf_lex::json::{self, Value, record::Record};

#[derive(Clone)]
struct Request {
    directory: String,
    schema: u64,
    source: String,
    manifest: String,
    configurations: Vec<String>,
    sites: Vec<String>,
}
purrdf_lex::json_record!(Request as "assembly aggregation request" {
    "directory" => directory: required,
    "schema" => schema: required,
    "source" => source: required,
    "manifest" => manifest: required,
    "configurations" => configurations: required,
    "sites" => sites: required,
});

#[derive(Clone)]
struct Identity {
    schema: u64,
    source: String,
    manifest: String,
    compiler: String,
}
purrdf_lex::json_record!(Identity as "assembly measurement identity" {
    "schema" => schema: required,
    "source" => source: required,
    "manifest" => manifest: required,
    "compiler" => compiler: required,
});

fn expected(request: &Request, receipt: &str) -> Result<Identity, String> {
    for (name, value) in [("source", &request.source), ("manifest", &request.manifest)] {
        if purrdf_hash::hex::Digest32::from_hex(value).is_none() {
            return Err(format!("invalid expected {name} identity"));
        }
    }
    if request.schema != 1
        || !receipt.starts_with("rustc ")
        || ["commit-hash:", "host:", "LLVM version:"]
            .into_iter()
            .any(|field| {
                !receipt.lines().any(|line| {
                    line.strip_prefix(field)
                        .is_some_and(|value| !value.trim().is_empty())
                })
            })
    {
        return Err("missing or malformed producer compiler receipt/schema".to_owned());
    }
    Ok(Identity {
        schema: request.schema,
        source: request.source.clone(),
        manifest: request.manifest.clone(),
        compiler: receipt.to_owned(),
    })
}

fn merge(request: &Request, receipt: &str, reports: Vec<(String, Value)>) -> Result<Value, String> {
    let expected = expected(request, receipt)?;
    let configurations: BTreeSet<_> = request.configurations.iter().map(String::as_str).collect();
    let sites: BTreeSet<_> = request.sites.iter().map(String::as_str).collect();
    if configurations.len() != 7
        || request.configurations.len() != 7
        || sites.is_empty()
        || sites.len() != request.sites.len()
    {
        return Err(
            "aggregation requires seven unique configurations and unique nonempty sites".to_owned(),
        );
    }
    let mut merged = BTreeMap::new();
    for (path, report) in reports {
        let mut record =
            Record::new(&report, "assembly report").map_err(|error| format!("{path}: {error}"))?;
        let identity: Identity = record
            .required("identity")
            .map_err(|error| format!("{path}: {error}"))?;
        let status: String = record
            .required("status")
            .map_err(|error| format!("{path}: {error}"))?;
        let cells: Value = record
            .required("cells")
            .map_err(|error| format!("{path}: {error}"))?;
        for (field, agrees) in [
            ("schema", identity.schema == expected.schema),
            ("source", identity.source == expected.source),
            ("manifest", identity.manifest == expected.manifest),
            (
                "producer compiler receipt",
                identity.compiler == expected.compiler,
            ),
        ] {
            if !agrees {
                return Err(format!("{path}: assembly report {field} mismatch"));
            }
        }
        if status != "passed" {
            return Err(format!("{path}: assembly report is not passed"));
        }
        let columns = cells
            .as_object()
            .ok_or_else(|| format!("{path}: cells is not an object"))?;
        for (name, column) in columns.iter() {
            if !configurations.contains(name.as_str()) || merged.contains_key(name) {
                return Err(format!("{path}: unknown or repeated configuration {name}"));
            }
            let entries = column
                .as_object()
                .ok_or_else(|| format!("{path}: {name} is not a site column"))?;
            let found: BTreeSet<_> = entries.iter().map(|(site, _)| site.as_str()).collect();
            if found != sites || entries.len() != sites.len() {
                return Err(format!(
                    "{path}: incomplete, repeated or unknown sites in {name}"
                ));
            }
            merged.insert(name.clone(), column.clone());
        }
    }
    if merged.len() != configurations.len() {
        return Err("incomplete assembly configuration matrix".to_owned());
    }
    Ok(Value::object([("cells", Value::object(merged))]))
}

/// Read original reports without modifying their bytes or invoking a compiler.
pub(crate) fn run() -> Result<ExitCode, String> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let request: Request = json::record::from_slice(&bytes)
        .map_err(|error| format!("assembly aggregation request: {error}"))?;
    let receipt_path = std::path::Path::new(&request.directory).join("compiler-receipt.txt");
    let receipt = std::fs::read_to_string(&receipt_path).map_err(|error| {
        format!(
            "actual producer compiler receipt {}: {error}",
            receipt_path.display()
        )
    })?;
    let mut paths = std::fs::read_dir(&request.directory)
        .map_err(|error| format!("{}: {error}", request.directory))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    });
    paths.sort();
    let mut reports = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let value = json::read(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        reports.push((path.display().to_string(), value));
    }
    println!(
        "{}",
        json::write_compact(&merge(&request, &receipt, reports)?)
    );
    Ok(ExitCode::SUCCESS)
}

/// Independent admission/refusal fixtures for compiler and matrix provenance.
pub(crate) fn self_test_cases() -> Vec<(&'static str, bool)> {
    let receipt = "rustc fixture\ncommit-hash: original\nhost: fixture\nLLVM version: fixture\n";
    let request = Request {
        directory: String::new(),
        schema: 1,
        source: "a".repeat(64),
        manifest: "b".repeat(64),
        configurations: (0..7)
            .map(|index| format!("configuration-{index}"))
            .collect(),
        sites: vec!["first".to_owned(), "second".to_owned()],
    };
    let identity = Value::object([
        ("schema", Value::from(1_u64)),
        ("source", Value::from(request.source.clone())),
        ("manifest", Value::from(request.manifest.clone())),
        ("compiler", Value::from(receipt)),
    ]);
    let reports: Vec<_> = request
        .configurations
        .iter()
        .map(|name| {
            (
                name.clone(),
                Value::object([
                    ("identity", identity.clone()),
                    ("status", Value::from("passed")),
                    (
                        "cells",
                        Value::object([(
                            name.clone(),
                            Value::object([
                                ("first", Value::from("first measurement")),
                                ("second", Value::from("second measurement")),
                            ]),
                        )]),
                    ),
                ]),
            )
        })
        .collect();
    let mut cases = vec![(
        "assembly aggregation accepts seven original reports bound to the producer receipt",
        merge(&request, receipt, reports.clone()).is_ok(),
    )];
    for field in ["schema", "source", "manifest", "compiler"] {
        let mut changed = reports.clone();
        changed[0].1["identity"][field] = match field {
            "schema" => Value::from(2_u64),
            "source" | "manifest" => Value::from("c".repeat(64)),
            _ => Value::from(receipt.replace("original", "different")),
        };
        cases.push((
            match field {
                "schema" => "assembly aggregation refuses a changed report schema",
                "source" => "assembly aggregation refuses stale source",
                "manifest" => "assembly aggregation refuses a changed manifest",
                _ => "assembly aggregation refuses mixed measured compilers",
            },
            merge(&request, receipt, changed).is_err(),
        ));
    }
    let mut changed = reports.clone();
    changed[0].1["status"] = Value::from("incomplete");
    cases.push((
        "assembly aggregation refuses incomplete evidence",
        merge(&request, receipt, changed).is_err(),
    ));
    cases.push((
        "assembly aggregation refuses a missing configuration",
        merge(&request, receipt, reports[..6].to_vec()).is_err(),
    ));
    let mut changed = reports.clone();
    changed.push(reports[0].clone());
    cases.push((
        "assembly aggregation refuses a repeated configuration",
        merge(&request, receipt, changed).is_err(),
    ));
    let mut changed = reports.clone();
    changed[0].1["cells"][request.configurations[0].as_str()] =
        Value::object([("first", "measurement")]);
    cases.push((
        "assembly aggregation refuses missing sites",
        merge(&request, receipt, changed).is_err(),
    ));
    cases.push((
        "assembly aggregation refuses an absent producer receipt",
        merge(&request, "", reports.clone()).is_err(),
    ));
    cases.push((
        "assembly aggregation refuses the aggregator compiler substituted for the producer",
        merge(
            &request,
            &receipt.replace("original", "aggregator"),
            reports.clone(),
        )
        .is_err(),
    ));
    let mut changed = reports;
    changed[0].1["cells"] = Value::object([(
        "unknown",
        changed[0].1["cells"][request.configurations[0].as_str()].clone(),
    )]);
    cases.push((
        "assembly aggregation refuses an unknown configuration",
        merge(&request, receipt, changed).is_err(),
    ));
    cases
}
