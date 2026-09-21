// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! W3C SHACL conformance harness over the vendored `vectors/shacl` corpus
//! (w3c/data-shapes test suite, `core/` + `sparql/`).
//!
//! Case discovery lives in [`shacl_corpora`], the one reader for this crate's two
//! SHACL corpora: it walks the `mf:` (test-manifest) tree starting at
//! `vectors/shacl/manifest.ttl`, where manifests `mf:include` sub-manifests and
//! list `mf:entries` of type `sht:Validate`, whose `mf:action` names a
//! `sht:shapesGraph` / `sht:dataGraph` (usually `<>` — the test file itself,
//! which then contains data, shapes, manifest entry, AND the expected report in
//! one graph, exactly as the upstream suite intends). This file is the GRADING
//! half: it runs each discovered case through the engine and decides whether the
//! produced report agrees with the manifest's.
//!
//! ## Comparison contract (caveats)
//!
//! The expected `sh:ValidationReport` graphs in the suite carry more detail
//! than the engine emits. The harness therefore compares on the SHARED tuple
//! subset, as a MULTISET:
//!
//!   `(focusNode, resultPath, value, sourceConstraintComponent, severity)`
//!
//! with these normalizations:
//!
//! - **blank nodes** (focus/value/path) normalize to the placeholder `_:` —
//!   expected reports use their own bnode labels which cannot match the
//!   engine's; complex `sh:resultPath` structures (inverse/sequence/alternative
//!   path bnodes) normalize the same way, so only a *simple* (IRI) result path
//!   is compared by identity;
//! - **`sh:sourceShape` is NOT compared** — many suite shapes are blank nodes;
//! - **`sh:resultMessage` and nested `sh:detail` are NOT compared** — the
//!   engine's message text is its own, and it does not emit `sh:detail`;
//! - `mf:result sht:Failure` means the validator must REJECT the test input
//!   (any engine `Err` passes; a successful validation fails).
//!
//! ## Xfail ledger
//!
//! Every currently-failing test is listed in [`XFAIL`] with a precise reason;
//! the ledger doubles as the SHACL completion roadmap. The harness asserts
//! that every non-xfail test passes, that every xfail test still fails (an
//! unexpectedly-passing xfail is an error — remove it from the ledger), and
//! the EXACT discovered-test/xfail counts so silent corpus drift fails fast.
//!
//! Run with `--nocapture` for the per-manifest-section scoreboard:
//! `cargo test -p purrdf-shapes --test w3c_conformance -- --nocapture`

mod shacl_corpora;

use std::collections::BTreeMap;
use std::fs;

use shacl_corpora::{Expected, Multiset, W3C_TOTAL_CASES, W3cCase, file_iri, norm, w3c_cases};

// ── Xfail ledger ──────────────────────────────────────────────────────────────

/// Tests the engine currently fails, with the reason. `(test id, reason)` where
/// the id is the entry IRI relative to the corpus root (no `.ttl`).
///
/// A test listed here MUST fail; when engine work fixes it, the harness errors
/// with `XPASS` and the entry must be removed. This is the SHACL completion
/// roadmap — keep reasons precise.
const XFAIL: &[(&str, &str)] = &[
    // All ledgered AF tests now pass; keep this ledger empty unless new
    // validation-only gaps are discovered.
];

// ── Running one case ──────────────────────────────────────────────────────────

/// Load graphs, run the engine. `Err` carries the parse/validation error.
fn validate_case(tc: &W3cCase) -> Result<purrdf_shapes::report::ValidationReport, String> {
    let shapes_text = fs::read_to_string(&tc.shapes_path)
        .map_err(|e| format!("cannot read shapes {}: {e}", tc.shapes_path.display()))?;
    let shapes_dataset = purrdf::parse_dataset(
        shapes_text.as_bytes(),
        "text/turtle",
        Some(&file_iri(&tc.shapes_path)),
    )
    .map_err(|e| format!("shapes graph parse error: {e}"))?;
    let doc_prefixes = purrdf_shapes::text_ingest::extract_prefixes(&shapes_text);
    let shapes_graph_iri = tc.shapes_graph_iri.as_deref();
    let shapes = purrdf_shapes::shapes::from_dataset_with_config_and_graph(
        &shapes_dataset,
        &doc_prefixes,
        None,
        shapes_graph_iri.map(ToOwned::to_owned),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;

    let data_dataset = if tc.data_path == tc.shapes_path {
        shapes_dataset
    } else {
        shacl_corpora::parse_turtle_file(&tc.data_path)
            .map_err(|e| format!("data graph parse error: {e}"))?
    };

    purrdf_shapes::engine::validate_dataset_with_shapes_graph(
        data_dataset.as_ref(),
        &shapes,
        shapes_graph_iri,
    )
    .map_err(|e| format!("validation error: {e}"))
}

/// Multiset of comparison tuples the engine produced.
fn produced_multiset(report: &purrdf_shapes::report::ValidationReport) -> Multiset {
    let mut multiset = Multiset::new();
    for r in &report.results {
        let focus = norm(&r.focus_node);
        let path = r.result_path.as_ref().map(norm);
        let value = r.value.as_ref().map(norm);
        let component = format!("<{}>", r.source_constraint_component.as_str());
        let severity = format!("<{}>", r.severity.iri());
        *multiset
            .entry((focus, path, value, component, severity))
            .or_insert(0) += 1;
    }
    multiset
}

/// [`validate_case`] hardened against engine panics: a panic is reported as a
/// failure string rather than aborting the whole harness. The engine's
/// SHACL-SPARQL path now rejects restricted queries at shape-load and surfaces
/// residual evaluation failures as `Err` (no known panicking case remains);
/// the guard stays as belt-and-braces so a regression reads as a FAIL with a
/// message instead of a harness abort.
fn validate_case_no_panic(tc: &W3cCase) -> Result<purrdf_shapes::report::ValidationReport, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| validate_case(tc))).unwrap_or_else(
        |payload| {
            let msg = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("<non-string panic payload>");
            Err(format!("engine panicked: {msg}"))
        },
    )
}

/// Run one test to a pass (`Ok`) / fail-with-reason (`Err`) verdict.
fn run_case(tc: &W3cCase) -> Result<(), String> {
    let outcome = validate_case_no_panic(tc);
    match (&tc.expected, outcome) {
        (Expected::Failure, Err(_)) => Ok(()),
        (Expected::Failure, Ok(_)) => {
            Err("suite expects sht:Failure but the engine validated successfully".to_owned())
        }
        (Expected::Report { .. }, Err(e)) => Err(e),
        (Expected::Report { conforms, results }, Ok(report)) => {
            if report.conforms != *conforms {
                return Err(format!(
                    "conforms mismatch: produced={}, expected={conforms}",
                    report.conforms
                ));
            }
            let produced = produced_multiset(&report);
            if &produced != results {
                return Err(multiset_diff(results, &produced));
            }
            Ok(())
        }
    }
}

/// Human-readable multiset diff for the failure message.
fn multiset_diff(expected: &Multiset, produced: &Multiset) -> String {
    let mut lines = vec!["result multiset mismatch:".to_owned()];
    for (tuple, n) in expected {
        let have = produced.get(tuple).copied().unwrap_or(0);
        if have != *n {
            lines.push(format!("  expected x{n}, produced x{have}: {tuple:?}"));
        }
    }
    for (tuple, n) in produced {
        if !expected.contains_key(tuple) {
            lines.push(format!("  expected x0, produced x{n}: {tuple:?}"));
        }
    }
    lines.join("\n")
}

// ── The harness ───────────────────────────────────────────────────────────────

#[test]
fn w3c_shacl_conformance() {
    let tests = w3c_cases();

    let xfail: BTreeMap<&str, &str> = XFAIL.iter().copied().collect();
    assert_eq!(
        xfail.len(),
        XFAIL.len(),
        "duplicate entries in the XFAIL ledger"
    );
    for (id, _) in XFAIL {
        assert!(
            tests.iter().any(|t| t.id == *id),
            "XFAIL ledger names unknown test {id} — stale entry?"
        );
    }

    let mut errors: Vec<String> = Vec::new();
    // (section, passed, xfailed) in discovery order.
    let mut sections: Vec<(String, usize, usize)> = Vec::new();
    let mut total_passed = 0usize;
    let mut total_xfailed = 0usize;

    // Silence the default panic hook while running cases: engine panics are
    // caught by `validate_case_no_panic` and reported as ledgered failures, so
    // their backtraces would only drown the scoreboard.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    for tc in &tests {
        if sections.last().is_none_or(|(s, _, _)| s != &tc.section) {
            sections.push((tc.section.clone(), 0, 0));
        }
        let slot = sections.last_mut().expect("section pushed above");

        let verdict = run_case(tc);
        match (verdict, xfail.get(tc.id.as_str())) {
            (Ok(()), None) => {
                slot.1 += 1;
                total_passed += 1;
            }
            (Err(_), Some(_)) => {
                slot.2 += 1;
                total_xfailed += 1;
            }
            (Ok(()), Some(reason)) => errors.push(format!(
                "XPASS [{id}]: now passes — remove it from the XFAIL ledger (reason was: {reason})",
                id = tc.id
            )),
            (Err(e), None) => errors.push(format!("FAIL [{id}]: {e}", id = tc.id)),
        }
    }

    std::panic::set_hook(default_hook);

    // Scoreboard: one line per manifest section.
    println!("W3C SHACL conformance scoreboard ({} tests):", tests.len());
    for (section, passed, xfailed) in &sections {
        println!("  {section:<28} passed {passed:>3}  xfailed {xfailed:>3}");
    }
    println!(
        "  TOTAL: passed {total_passed}, xfailed {total_xfailed}, ledger {}",
        XFAIL.len()
    );

    assert!(
        errors.is_empty(),
        "w3c_shacl_conformance: {} error(s):\n{}",
        errors.len(),
        errors.join("\n\n")
    );

    // Exact-count gates: every test is either a pass or a ledgered xfail.
    assert_eq!(
        total_xfailed,
        XFAIL.len(),
        "xfail count must match the ledger exactly"
    );
    assert_eq!(
        total_passed + total_xfailed,
        W3C_TOTAL_CASES,
        "every discovered test must be a pass or a ledgered xfail"
    );
}
