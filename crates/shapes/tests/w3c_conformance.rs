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
//! Each expected `sh:ValidationReport` is compared, as a MULTISET, on the tuple
//!
//!   `(focusNode, resultPath, value, sourceConstraintComponent, severity,
//!   sourceShape)`
//!
//! with these normalizations:
//!
//! - **blank nodes** (focus/value/path/source shape) normalize to the
//!   placeholder `_:` — expected reports use their own bnode labels which cannot
//!   match the engine's; complex `sh:resultPath` structures
//!   (inverse/sequence/alternative path bnodes) normalize the same way, so only a
//!   *simple* (IRI) result path is compared by identity, and a blank-node source
//!   shape is compared as "a blank node";
//! - **`sh:resultMessage` is compared only where the expected report states
//!   one** — the suite asks a harness "to preserve all sh:resultMessage triples
//!   that are mentioned in the 'expected' results graph", so a produced result
//!   with the same tuple must carry EXACTLY that message set (language tags,
//!   directions and datatypes included); an engine-generated message on a result
//!   the suite states none for is the engine's own;
//! - **nested `sh:detail` is compared where the expected report states it** —
//!   such a result's produced details must be exactly the stated ones, as a
//!   multiset of the same tuple, recursively; a result the expected report
//!   states no details for is not graded on details, since SHACL 1.2 Core makes
//!   them optional and processor-dependent (quoted in `report_grading`);
//! - the expected report's `sh:conformanceDisallows` values are the validation
//!   parameter the case runs under, and the report must echo them;
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

use shacl_corpora::report_grading::run_validate_case;
use shacl_corpora::{W3C_TOTAL_CASES, w3c_cases};

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
//
// Grading lives in `shacl_corpora::report_grading`, shared with the SHACL 1.2
// harness so both suites mean the same thing by "agrees with the manifest".

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
    // caught by `report_grading::no_panic` and reported as ledgered failures, so
    // their backtraces would only drown the scoreboard.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    for tc in &tests {
        if sections.last().is_none_or(|(s, _, _)| s != &tc.section) {
            sections.push((tc.section.clone(), 0, 0));
        }
        let slot = sections.last_mut().expect("section pushed above");

        let verdict = run_validate_case(tc);
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
