// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every W3C SHACL 1.2 `sht:EvalNodeExpr` entry, run through the BUILT `purrdf node-expr`
//! binary — the production entry point, not the library harness's internals.
//!
//! Discovery is the library harness's own corpus reader
//! (`crates/shapes/tests/shacl_corpora`, included here by path), with its drift guards:
//! the discovered total and per-type counts are asserted before anything runs. Grading is
//! its own `sht:EvalNodeExpr` grader (`shacl_corpora::node_expr_grading`): exact RDF 1.2
//! term equality, in order unless `sht:ignoreOrder true`, with the one upstream errata
//! table applied exactly as the library harness applies it.
//!
//! Each entry's expression is an anonymous `[ … ]` node (or a constant) that no caller
//! could label, so it is named the way the manifest itself names it: from the entry,
//! `mf:action`, then `sht:nodeExpr` — `--expr-at ENTRY --expr-via mf:action --expr-via
//! sht:nodeExpr`. The test file is both the shapes graph and the data graph, read under
//! its own `file://` IRI as the manifest walk read it. `sht:focusNode` is `--focus`,
//! every `sht:scope-NAME` is `--scope NAME=TERM`, and an entry with no focus node is
//! evaluated from the harness's absent focus node, a blank node the test graph is proved
//! never to mention.

#[path = "../../shapes/tests/shacl_corpora/mod.rs"]
mod shacl_corpora;

use std::process::Command;

use purrdf_shapes::data::{GraphFilter, native_quads};
use purrdf_shapes::free_expression::parse_term;
use purrdf_shapes::term::Term;

use shacl_corpora::file_iri;
use shacl_corpora::node_expr_grading::{
    ABSENT_FOCUS, NON_CANONICAL_EXPECTATIONS, NON_CANONICAL_EXPECTATIONS_COUNT,
    canonical_expectation, compare_outputs,
};
use shacl_corpora::shacl12::{Body, NodeExprCase, shacl12_cases, shacl12_root};

const MF_ACTION: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#action";
const SHT_NODE_EXPR: &str = "http://www.w3.org/ns/shacl-test#nodeExpr";

/// The discovered `sht:EvalNodeExpr` entries — pinned by the corpus reader's own
/// per-type table, restated here so this harness cannot pass by running fewer.
const NODE_EXPR_CASES: usize = 143;

/// The focus node an entry is evaluated from: its `sht:focusNode`, or the absent focus
/// node, which the test graph must not mention.
fn focus(tc: &NodeExprCase) -> Result<Term, String> {
    if let Some(focus) = &tc.focus {
        return Ok(focus.clone());
    }
    let absent = Term::blank(ABSENT_FOCUS);
    let dataset = tc.dataset.as_ref();
    let mentioned = !native_quads(dataset, Some(&absent), None, None, GraphFilter::AnyGraph)
        .is_empty()
        || !native_quads(dataset, None, None, Some(&absent), GraphFilter::AnyGraph).is_empty();
    if mentioned {
        return Err(format!("the test graph mentions the harness's {absent}"));
    }
    Ok(absent)
}

/// Run one entry through the binary and grade its output.
fn run_case(id: &str, entry_iri: &str, tc: &NodeExprCase) -> Result<(), String> {
    let file = tc.file.to_str().ok_or("a non-UTF-8 corpus path")?;
    let base = file_iri(&tc.file);
    let focus = focus(tc)?.to_string();
    let mut command = Command::new(env!("CARGO_BIN_EXE_purrdf"));
    command.args([
        "node-expr",
        "--shapes",
        file,
        "--shapes-base",
        &base,
        "--expr-at",
        entry_iri,
        "--expr-via",
        MF_ACTION,
        "--expr-via",
        SHT_NODE_EXPR,
        "--focus",
        &focus,
        "--base",
        &base,
    ]);
    for (name, value) in &tc.scope {
        command.arg("--scope").arg(format!("{name}={value}"));
    }
    command.arg(file);
    let out = command.output().map_err(|e| format!("spawn purrdf: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return Err(format!("exit {:?}: {stderr}", out.status.code()));
    }
    let stdout = String::from_utf8(out.stdout).map_err(|e| format!("stdout: {e}"))?;
    let produced = stdout
        .lines()
        .map(|line| parse_term(line).map_err(|e| format!("output line {line:?}: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
    let reported = format!("node-expr outputs {}\n", produced.len());
    if !stderr.contains(&reported) {
        return Err(format!("stderr does not report {reported:?}: {stderr}"));
    }
    let expected = canonical_expectation(id, &tc.expected)?;
    compare_outputs(&produced, &expected, tc.ignore_order)
}

#[test]
fn every_w3c_node_expression_runs_through_the_cli() {
    let root_iri = file_iri(&shacl12_root());
    let cases = shacl12_cases();
    let node_expr: Vec<(&str, &NodeExprCase)> = cases
        .iter()
        .filter_map(|case| match &case.body {
            Body::NodeExpr(tc) => Some((case.id.as_str(), tc)),
            _ => None,
        })
        .collect();
    assert_eq!(
        node_expr.len(),
        NODE_EXPR_CASES,
        "discovered sht:EvalNodeExpr entries"
    );
    assert!(
        cases
            .iter()
            .filter(|case| matches!(case.body, Body::NodeExpr(_)))
            .all(|case| case.listed),
        "every sht:EvalNodeExpr entry is one an upstream manifest lists"
    );

    let mut passed = 0usize;
    let mut errata = 0usize;
    let mut errors: Vec<String> = Vec::new();
    for (id, tc) in &node_expr {
        let entry_iri = format!("{root_iri}/{id}");
        match run_case(id, &entry_iri, tc) {
            Ok(()) if NON_CANONICAL_EXPECTATIONS.iter().any(|(e, ..)| e == id) => errata += 1,
            Ok(()) => passed += 1,
            Err(e) => errors.push(format!("FAIL [{id}]: {e}")),
        }
    }
    println!(
        "W3C SHACL 1.2 sht:EvalNodeExpr through `purrdf node-expr`: passed {passed}, \
         upstream-errata {errata}, failed {}",
        errors.len()
    );
    assert!(
        errors.is_empty(),
        "{} entr(ies) failed through the CLI:\n{}",
        errors.len(),
        errors.join("\n\n")
    );
    assert_eq!(
        errata, NON_CANONICAL_EXPECTATIONS_COUNT,
        "every upstream erratum is reported as one, and nothing else is"
    );
    assert_eq!(
        passed + errata,
        NODE_EXPR_CASES,
        "every entry is a pass or an upstream erratum"
    );
}

/// The selector's refusal through the binary, beside its valid neighbour: the entry's
/// own walk evaluates, and one step further — a predicate the expression does not carry
/// — reaches no value and is refused (exit 1), naming the step and the count.
#[test]
fn an_entry_walk_that_reaches_nothing_is_refused() {
    let root = shacl12_root();
    let file = root.join("node-expr/shnex/count.ttl");
    let base = file_iri(&file);
    let file = file.to_str().expect("UTF-8 corpus path");
    let cases = shacl12_cases();
    let (id, _) = cases
        .iter()
        .find_map(|case| match &case.body {
            Body::NodeExpr(tc) if tc.file.ends_with("node-expr/shnex/count.ttl") => {
                Some((case.id.as_str(), tc))
            }
            _ => None,
        })
        .expect("a count entry");
    let entry = format!("{}/{id}", file_iri(&root));
    let walk = |via: &[&str]| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_purrdf"));
        command.args([
            "node-expr",
            "--shapes",
            file,
            "--shapes-base",
            &base,
            "--expr-at",
            &entry,
            "--focus",
            "http://example.org/ns#nowhere",
            "--base",
            &base,
        ]);
        for predicate in via {
            command.arg("--expr-via").arg(predicate);
        }
        command.arg(file);
        command.output().expect("spawn purrdf")
    };
    let valid = walk(&[MF_ACTION, SHT_NODE_EXPR]);
    assert_eq!(
        valid.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let refused = walk(&[MF_ACTION, SHT_NODE_EXPR, SHT_NODE_EXPR]);
    assert_eq!(refused.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("step 3") && stderr.contains("reaches 0 values"),
        "{stderr}"
    );
}
