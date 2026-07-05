// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL **Rules** (`sh:rule`) conformance harness over the first-party corpus
//! at `vectors/shacl/af/rules/`.
//!
//! Unlike the validation harness ([`w3c_conformance`](../w3c_conformance.rs)),
//! which asserts a `sh:ValidationReport`, a rules test asserts the **inferred
//! graph**: the triples [`purrdf_shapes::apply_rules`] derives from the data
//! under a shapes-with-rules graph.
//!
//! ## Fixture format
//!
//! Each case is a directory `vectors/shacl/af/rules/<case>/` holding:
//!
//! - `input.ttl` (or `input.trig`) — the data + shapes(+rules) graph the rules
//!   run over. The default graph carries the data and shape/rule definitions;
//!   `input.trig` may place data in a named graph (rules see the flattened
//!   default-graph projection, matching the validator).
//! - `expected-inferred.ttl` — the **derived triples only**: exactly the delta
//!   the rules add over `input.ttl`'s graph (NOT `base ∪ derived`). This is the
//!   convention the vendored DASH `dash:InferencingTestCase` corpus uses — its
//!   `dash:expectedResult` reified triples are precisely the inferred delta.
//!
//! An `err-*` case has NO `expected-inferred.ttl`; the harness asserts that
//! [`apply_rules`](purrdf_shapes::apply_rules) returns `Err` for it (a rule that
//! is malformed at firing time, or a rule set that diverges).
//!
//! ## Comparison contract
//!
//! For a non-error case the harness:
//!
//! 1. parses `input.ttl` once and projects it (`project_dataset`) to the base
//!    default graph exactly as [`apply_rules`](purrdf_shapes::apply_rules) sees it;
//! 2. runs `apply_rules`, yielding the full inferred graph `base ⊎ derived`;
//! 3. parses `expected-inferred.ttl` (the derived delta) and merges it onto the
//!    same projected base to reconstruct the expected full graph
//!    `base ⊎ expected-derived`;
//! 4. canonicalizes (RDFC-1.0) BOTH full graphs and compares the canonical
//!    N-Quads strings, so blank labels — including `freeze`'s standardize-apart
//!    relabeling and any freshly minted CONSTRUCT blanks — never cause a false
//!    mismatch (RDF isomorphism). Comparing full graphs (rather than a raw quad
//!    set-difference) is what makes the blank handling robust.
//!
//! ## Ledger and scoreboard
//!
//! The harness asserts an exact case count and maintains an [`XFAIL`] ledger
//! (target: empty — any entry must name a documented upstream spec ambiguity,
//! not a papered-over bug). It prints the scoreboard line `RULES: passed {n}
//! total {n}` (scraped by the conformance matrix).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf::{canonicalize, RdfDataset, RdfDatasetBuilder};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::shapes::from_dataset_with_prefixes;
use purrdf_shapes::{apply_rules, engine, text_ingest};

const RULES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/shacl/af/rules");

/// The EXACT number of case directories the corpus must hold, so a removed or
/// renamed case fails fast. Bump this deliberately when adding a case.
const TOTAL_CASES: usize = 17;

/// Cases the harness expects to fail, with a precise reason. An entry here MUST
/// document an upstream spec ambiguity, never a papered-over engine bug; a case
/// that unexpectedly passes is an error (`XPASS`).
const XFAIL: &[(&str, &str)] = &[];

// ── IRI / parse helpers ────────────────────────────────────────────────────────

fn file_iri(path: &Path) -> String {
    format!("file://{}", path.display())
}

/// Parse an `input.ttl` / `input.trig` fixture into a frozen dataset.
fn parse_input(path: &Path, text: &str) -> Result<Arc<RdfDataset>, String> {
    let media = if path.extension().and_then(|e| e.to_str()) == Some("trig") {
        "application/trig"
    } else {
        "text/turtle"
    };
    purrdf::parse_dataset(text.as_bytes(), media, Some(&file_iri(path)))
        .map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

// ── Expected-graph reconstruction ──────────────────────────────────────────────

/// Merge the projected base with the parsed derived-delta graph into one frozen
/// dataset (`base ⊎ expected-derived`). `push_dataset` standardizes blank labels
/// apart per source, so the two graphs' blanks never collide.
fn merge(base: &RdfDataset, derived: &RdfDataset) -> Result<Arc<RdfDataset>, String> {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(base);
    builder.push_dataset(derived);
    builder.freeze().map_err(|e| e.to_string())
}

// ── One case ───────────────────────────────────────────────────────────────────

struct Case {
    name: String,
    dir: PathBuf,
    input_path: PathBuf,
    /// `true` for an `err-*` case (asserts `apply_rules` returns `Err`).
    expect_err: bool,
}

/// Build the shapes + projected base for a case's input.
fn load(case: &Case) -> Result<(purrdf_shapes::shapes::Shapes, Arc<RdfDataset>), String> {
    let text = fs::read_to_string(&case.input_path)
        .map_err(|e| format!("cannot read {}: {e}", case.input_path.display()))?;
    let input = parse_input(&case.input_path, &text)?;
    let doc_prefixes = text_ingest::extract_prefixes(&text);
    let shapes = from_dataset_with_prefixes(&input, &doc_prefixes)
        .map_err(|e| format!("shapes parse error: {e}"))?;
    let projected = engine::project_dataset(input.as_ref())?;
    Ok((shapes, projected))
}

/// Run one case to a pass (`Ok`) / fail-with-reason (`Err`) verdict.
fn run_case(case: &Case) -> Result<(), String> {
    let (shapes, projected) = load(case)?;
    let data = ShaclData::new(Arc::clone(&projected), Arc::clone(&projected), None);
    let outcome = apply_rules(&data, &shapes);

    if case.expect_err {
        return match outcome {
            Err(_) => Ok(()),
            Ok(_) => Err("expected apply_rules to return Err, but it succeeded".to_owned()),
        };
    }

    let produced = outcome.map_err(|e| format!("apply_rules failed: {e}"))?;

    // The expected inferred graph is `base ⊎ derived-delta`: parse the
    // derived-only expected file and merge it onto the same projected base
    // `apply_rules` saw, then compare full graphs by canonicalization. This is
    // robust to `freeze`'s blank-node relabeling, which a raw quad set-difference
    // is not.
    let expected_path = case.dir.join("expected-inferred.ttl");
    let expected_text = fs::read_to_string(&expected_path)
        .map_err(|e| format!("cannot read {}: {e}", expected_path.display()))?;
    let derived_ds = purrdf::parse_dataset(
        expected_text.as_bytes(),
        "text/turtle",
        Some(&file_iri(&expected_path)),
    )
    .map_err(|e| format!("cannot parse expected graph: {e}"))?;
    let expected_full = merge(projected.as_ref(), derived_ds.as_ref())?;

    let produced_canon = canonicalize(produced.as_ref()).nquads;
    let expected_canon = canonicalize(expected_full.as_ref()).nquads;
    if produced_canon != expected_canon {
        return Err(format!(
            "inferred-graph mismatch (RDFC-1.0):\n  --- PRODUCED (base ⊎ derived) ---\n{produced_canon}\n  --- EXPECTED (base ⊎ expected-derived) ---\n{expected_canon}"
        ));
    }
    Ok(())
}

// ── Discovery ──────────────────────────────────────────────────────────────────

fn discover() -> Vec<Case> {
    let root = Path::new(RULES_DIR);
    assert!(root.is_dir(), "rules corpus not found at {RULES_DIR}");
    let mut cases: Vec<Case> = fs::read_dir(root)
        .expect("read rules corpus dir")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if !path.is_dir() {
                return None;
            }
            let name = path.file_name()?.to_string_lossy().into_owned();
            let input_path = {
                let trig = path.join("input.trig");
                if trig.exists() {
                    trig
                } else {
                    path.join("input.ttl")
                }
            };
            assert!(
                input_path.exists(),
                "case {name}: no input.ttl or input.trig"
            );
            Some(Case {
                expect_err: name.starts_with("err-"),
                name,
                dir: path,
                input_path,
            })
        })
        .collect();
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    cases
}

// ── Harness ────────────────────────────────────────────────────────────────────

#[test]
fn rules_conformance() {
    let cases = discover();
    assert_eq!(
        cases.len(),
        TOTAL_CASES,
        "rules corpus case count drifted — update TOTAL_CASES when adding/removing a case"
    );

    let xfail: std::collections::BTreeMap<&str, &str> = XFAIL.iter().copied().collect();
    assert_eq!(xfail.len(), XFAIL.len(), "duplicate XFAIL ledger entries");
    for (id, _) in XFAIL {
        assert!(
            cases.iter().any(|c| c.name == *id),
            "XFAIL ledger names unknown case {id} — stale entry?"
        );
    }

    let mut errors: Vec<String> = Vec::new();
    let mut passed = 0usize;
    let mut xfailed = 0usize;

    for case in &cases {
        let verdict = run_case(case);
        match (verdict, xfail.get(case.name.as_str())) {
            (Ok(()), None) => passed += 1,
            (Err(_), Some(_)) => xfailed += 1,
            (Ok(()), Some(reason)) => errors.push(format!(
                "XPASS [{}]: now passes — remove it from the XFAIL ledger (reason was: {reason})",
                case.name
            )),
            (Err(e), None) => errors.push(format!("FAIL [{}]: {e}", case.name)),
        }
    }

    // Scoreboard scraped by the conformance matrix (Task 6).
    println!("RULES: passed {passed} total {}", cases.len());

    assert!(
        errors.is_empty(),
        "rules_conformance: {} case(s) failed:\n{}",
        errors.len(),
        errors.join("\n\n")
    );
    assert_eq!(
        xfailed,
        XFAIL.len(),
        "xfail count must match the ledger exactly"
    );
    assert_eq!(
        passed + xfailed,
        TOTAL_CASES,
        "every discovered case must be a pass or a ledgered xfail"
    );
}

// ── Rust-level composition test ────────────────────────────────────────────────

/// `entail_dataset` is `apply_rules ∘ project_dataset`: entailing a dataset must
/// yield exactly what applying the rules to its projection yields. This pins the
/// public composition seam the corpus harness relies on.
#[test]
fn entail_dataset_composes_project_then_apply_rules() {
    let text = "\
        @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/ns#> .\n\
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
        ex:alice a ex:Person .\n\
        ex:S a sh:NodeShape ; sh:targetClass ex:Person ;\n\
          sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .";
    let input = purrdf::parse_dataset(text.as_bytes(), "text/turtle", None).expect("parse");
    let shapes =
        from_dataset_with_prefixes(&input, &text_ingest::extract_prefixes(text)).expect("shapes");

    let via_entail = purrdf_shapes::entail_dataset(input.as_ref(), &shapes).expect("entail");

    let projected = engine::project_dataset(input.as_ref()).expect("project");
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    let via_apply = apply_rules(&data, &shapes).expect("apply_rules");

    assert_eq!(
        canonicalize(via_entail.as_ref()).nquads,
        canonicalize(via_apply.as_ref()).nquads,
        "entail_dataset must equal apply_rules over the projected dataset"
    );
}

/// The `fp-order` corpus case fixes the sh:order VALUES; this test proves the
/// FINAL closure is order-INDEPENDENT: swapping the two rules' `sh:order` yields a
/// byte-identical entailment (a monotonic fixpoint does not depend on rule order).
#[test]
fn sh_order_is_order_independent_over_the_closure() {
    // Keep DATA and SHAPES in separate graphs so the entailed output carries only
    // the data + derived triples — the shapes graph (whose swapped `sh:order`
    // literals legitimately differ between the two runs) is not part of the
    // comparison.
    const PREFIXES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
                            @prefix ex: <http://example.org/ns#> .\n\
                            @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n";
    let data_text = format!("{PREFIXES}ex:alice a ex:Person .");
    let data =
        purrdf::parse_dataset(data_text.as_bytes(), "text/turtle", None).expect("data parse");

    let entail = |shapes_body: &str| {
        let shapes_text = format!("{PREFIXES}{shapes_body}");
        let shapes_ds = purrdf::parse_dataset(shapes_text.as_bytes(), "text/turtle", None)
            .expect("shapes parse");
        let shapes =
            from_dataset_with_prefixes(&shapes_ds, &text_ingest::extract_prefixes(&shapes_text))
                .expect("shapes");
        canonicalize(
            purrdf_shapes::entail_dataset(data.as_ref(), &shapes)
                .expect("entail")
                .as_ref(),
        )
        .nquads
    };

    let forward = entail(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;\n\
           sh:rule [ a sh:TripleRule ; sh:order 1 ; sh:subject sh:this ; sh:predicate ex:a ; sh:object ex:x ] ;\n\
           sh:rule [ a sh:TripleRule ; sh:order 2 ; sh:subject sh:this ; sh:predicate ex:b ; sh:object ex:y ] .",
    );
    let swapped = entail(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;\n\
           sh:rule [ a sh:TripleRule ; sh:order 2 ; sh:subject sh:this ; sh:predicate ex:a ; sh:object ex:x ] ;\n\
           sh:rule [ a sh:TripleRule ; sh:order 1 ; sh:subject sh:this ; sh:predicate ex:b ; sh:object ex:y ] .",
    );
    assert_eq!(
        forward, swapped,
        "swapping sh:order must not change the fixpoint closure"
    );
}
