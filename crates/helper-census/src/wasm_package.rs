// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact package host selections documented in `docs/WASM_TESTING.md`.

use std::process::{Command, Stdio};

use purrdf_lex::xml::Document;

const CASES: &[(&str, &[&str])] = &[
    (
        "reasoning-host-error.test.mjs",
        &["reasoning errors retain native handles across direct and query exports"],
    ),
    (
        "roundtrip.test.mjs",
        &[
            "literal_component_native_error_transport",
            "an unsupported format crosses the wasm boundary as a JavaScript Error",
        ],
    ),
    (
        "document-expansion-interface.test.mjs",
        &["document operation exports preserve byte transport and native error ownership"],
    ),
    (
        "yaml-source.test.mjs",
        &["YAML source pair preserves owned bytes and rejects before bridge copy"],
    ),
    (
        "jsonld-configured.test.mjs",
        &[
            "configured YAML input crosses wasm-bindgen with native error records",
            "configured Dataset serialization reuses compiled contexts",
        ],
    ),
    (
        "canonicalization-host-error.test.mjs",
        &["isomorphic refusal crosses the wasm boundary as a JavaScript Error"],
    ),
    (
        "shacl_tools.test.mjs",
        &[
            "wasm_shacl_apply_rules: maxStoredFacts reaches the evaluator and names the host control",
        ],
    ),
    (
        "cloudflare.test.mjs",
        &["CORS: preflight, echo and Vary with cors; no CORS header and a 405 OPTIONS without"],
    ),
    (
        "sync-coordinates.test.mjs",
        &[
            "synchronous coordinates cross every query export and package output",
            "synchronous positional query coordinates reject lossy host values",
            "synchronous omitted coordinates use fresh independent host defaults",
        ],
    ),
    (
        "async-coordinates.test.mjs",
        &[
            "async coordinates cross every query kind and output alias",
            "async coordinates reject lossy host inputs and unsupported routes",
            "async coordinate owners survive real suspension and interleaved jobs",
            "async jobs select omitted coordinates anew when validated options are reused",
            "Cloudflare query coordinates reach negotiated output",
        ],
    ),
    (
        "rule-coordinates.test.mjs",
        &[
            "rule coordinates cross synchronous async and low-level positional routes",
            "rule host coordinate refusals precede document parsing and preserve reuse",
            "rule coordinate owners survive actual JSPI yields and interleaving",
            "rule omitted coordinates retain independent host defaults",
            "rule supplied coordinates preserve signal cancellation and subsequent reuse",
        ],
    ),
    (
        "update-coordinates.test.mjs",
        &[
            "UPDATE coordinates cross every package raw and low-level host route",
            "UPDATE host coordinate errors precede mutation and release every claim",
            "UPDATE omitted coordinates reach independent host clock and entropy defaults",
            "UPDATE low-level coordinates survive suspension freed options and interleaved mutation",
            "UPDATE supplied coordinates preserve asynchronous abort atomicity and reuse",
            "Cloudflare UPDATE coordinates reach mutation and typed refusal responses",
        ],
    ),
    (
        "async.test.mjs",
        &[
            "dataset mutation during an async update is refused",
            "commitUpdate refuses a different dataset",
            "refusal pair: an update begun while another update of the dataset is in flight is refused; a sequential one applies",
        ],
    ),
    (
        "governors.test.mjs",
        &[
            "updateGoverned refuses maxAnswers rather than ignoring a ceiling the caller set",
            "a cancelled token stops an UPDATE as an outcome",
        ],
    ),
    (
        "async-concurrency.test.mjs",
        &[
            "a request as deep as the synchronous lane evaluates answers on a job's region",
            "a trap poisons every entry point of the instance, and jobs that fault without trapping poison nothing",
        ],
    ),
    (
        "panic-poison.test.mjs",
        &[
            "a panic in a synchronous call poisons every entry point, and typed errors poison nothing",
        ],
    ),
    (
        "trap-poison.test.mjs",
        &["a trap that is not a panic, in a synchronous call, poisons every entry point"],
    ),
];

/// Run only the documented package host cases and require each to pass once.
pub(super) fn run(args: &[String]) -> Result<(), String> {
    if args != ["--wasm-package-hosts"] {
        return Err("usage: helper-census --wasm-package-hosts".to_owned());
    }
    let root = purrdf_testkit::paths::workspace_root();
    let inventory = std::fs::read_to_string(root.join("docs/WASM_TESTING.md"))
        .map_err(|error| format!("cannot read WASM execution inventory: {error}"))?;
    let inventory = inventory.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut total = 0;
    for &(file, names) in CASES {
        for name in names {
            if !inventory.contains(&format!("`{name}`")) {
                return Err(format!("{file}: `{name}` has no named WASM obligation"));
            }
        }
        let pattern = names
            .iter()
            .map(|name| regex::escape(name))
            .collect::<Vec<_>>()
            .join("|");
        let output = Command::new("node")
            .current_dir(root.join("crates/rdf-wasm/js"))
            .args(["--test", "--test-reporter=junit"])
            .arg(format!("--test-name-pattern=^(?:{pattern})$"))
            .arg(format!("tests/{file}"))
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| format!("cannot run Node host cases in {file}: {error}"))?;
        let report = String::from_utf8(output.stdout)
            .map_err(|error| format!("{file}: Node JUnit report is not UTF-8: {error}"))?;
        if !output.status.success() {
            return Err(format!("{file}: Node failed ({})\n{report}", output.status));
        }
        check_report(&report, names).map_err(|error| format!("{file}: {error}\n{report}"))?;
        println!("{file}: {} exact host cases passed", names.len());
        total += names.len();
    }
    println!(
        "WASM package host cases: {total} passed across {} targets",
        CASES.len()
    );
    Ok(())
}

fn check_report(report: &str, wanted: &[&str]) -> Result<(), String> {
    let document = Document::parse(report).map_err(|error| error.to_string())?;
    if !document.root_element().has_tag_name("testsuites") {
        return Err("Node did not return a JUnit testsuites document".to_owned());
    }
    let mut seen = vec![false; wanted.len()];
    for case in document
        .descendants()
        .filter(|node| node.has_tag_name("testcase"))
    {
        let name = case.attribute("name").ok_or("JUnit testcase has no name")?;
        let skipped = case.descendants().any(|node| node.has_tag_name("skipped"));
        if case
            .descendants()
            .any(|node| node.has_tag_name("failure") || node.has_tag_name("error"))
        {
            return Err(format!("host case failed: {name}"));
        }
        if let Some(index) = wanted.iter().position(|expected| *expected == name) {
            if skipped {
                return Err(format!("requested host case was skipped: {name}"));
            }
            if seen[index] {
                return Err(format!(
                    "requested host case executed more than once: {name}"
                ));
            }
            seen[index] = true;
        } else if !skipped {
            return Err(format!("unexpected host case executed: {name}"));
        }
    }
    for (name, passed) in wanted.iter().zip(seen) {
        if !passed {
            return Err(format!("requested host case did not execute: {name}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_report;

    /// A release gate must prove exact successful execution, including on Node
    /// versions that report filtered cases as skipped rather than omitting them.
    #[test]
    fn host_qualification_requires_each_requested_case_once_without_skips() {
        let passing = "<testsuites><testcase name='host'/><testcase name='other'><skipped/></testcase></testsuites>";
        assert!(check_report(passing, &["host"]).is_ok());
        for refused in [
            "<testsuites/>",
            "<testsuites><testcase name='host'><skipped/></testcase></testsuites>",
            "<testsuites><testcase name='host'/><testcase name='host'/></testsuites>",
            "<testsuites><testcase name='host'/><testcase name='other'/></testsuites>",
            "<testsuites><testcase name='host'><failure/></testcase></testsuites>",
            "<testsuites><testcase name='host'/>",
        ] {
            assert!(check_report(refused, &["host"]).is_err(), "{refused}");
        }
    }
}
