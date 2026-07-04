// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C SHACL conformance harness over the vendored `vectors/shacl` corpus
//! (w3c/data-shapes test suite, `core/` + `sparql/`).
//!
//! The harness walks the `mf:` (test-manifest) tree starting at
//! `vectors/shacl/manifest.ttl`: manifests `mf:include` sub-manifests and list
//! `mf:entries` of type `sht:Validate`, whose `mf:action` names a
//! `sht:shapesGraph` / `sht:dataGraph` (usually `<>` — the test file itself,
//! which then contains data, shapes, manifest entry, AND the expected report in
//! one graph, exactly as the upstream suite intends). Manifests are parsed with
//! the workspace's own Turtle codec (`purrdf::parse_dataset`, dogfooding);
//! relative IRIs resolve against each manifest's `file://` location.
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

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::data::{GraphFilter, IrDataGraph, ShaclDataGraph};
use purrdf_shapes::model::{rdf, sh};
use purrdf_shapes::term::{NamedNode, Term};

// ── Corpus location & vocabulary ──────────────────────────────────────────────

const VECTORS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/shacl");

/// Exact number of `sht:Validate` entries the manifest tree must discover.
/// Bump only when the vendored corpus itself changes (it is byte-frozen).
///
/// Note: the corpus ships 121 files with a `sht:Validate` entry, but upstream's
/// `sparql/component/manifest.ttl` never `mf:include`s `nodeValidator-001.ttl`,
/// so the manifest tree — the suite's own definition of membership — yields 120.
const TOTAL_TESTS: usize = 120;

mod mf {
    pub(crate) const INCLUDE: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#include";
    pub(crate) const ENTRIES: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#entries";
    pub(crate) const ACTION: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#action";
    pub(crate) const RESULT: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#result";
}

mod sht {
    pub(crate) const VALIDATE: &str = "http://www.w3.org/ns/shacl-test#Validate";
    pub(crate) const DATA_GRAPH: &str = "http://www.w3.org/ns/shacl-test#dataGraph";
    pub(crate) const SHAPES_GRAPH: &str = "http://www.w3.org/ns/shacl-test#shapesGraph";
    pub(crate) const FAILURE: &str = "http://www.w3.org/ns/shacl-test#Failure";
}

const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

// ── Xfail ledger ──────────────────────────────────────────────────────────────

/// Tests the engine currently fails, with the reason. `(test id, reason)` where
/// the id is the entry IRI relative to the corpus root (no `.ttl`).
///
/// A test listed here MUST fail; when engine work fixes it, the harness errors
/// with `XPASS` and the entry must be removed. This is the SHACL completion
/// roadmap — keep reasons precise.
const XFAIL: &[(&str, &str)] = &[
    // ── SHACL-SPARQL ──────────────────────────────────────────────────────────
    (
        "sparql/component/optional-001",
        "custom SPARQL constraint components (sh:ConstraintComponent with \
         sh:parameter and a sh:SPARQLAskValidator sh:ask validator) are \
         unsupported — only sh:sparql/sh:SPARQLConstraint SELECTs run",
    ),
    (
        "sparql/component/propertyValidator-select-001",
        "custom SPARQL constraint components (sh:ConstraintComponent with a \
         sh:propertyValidator SELECT validator and $PATH substitution) are \
         unsupported",
    ),
    (
        "sparql/component/validator-001",
        "custom SPARQL constraint components (sh:ConstraintComponent with a \
         sh:SPARQLAskValidator sh:ask validator over $value) are unsupported",
    ),
    (
        "sparql/pre-binding/pre-binding-002",
        "$this pre-binding is not visible inside FILTER-only UNION branches \
         (SPARQL pre-binding semantics); the query yields no solutions so no \
         violation is produced",
    ),
    (
        "sparql/pre-binding/pre-binding-005",
        "$this pre-binding is not visible inside a FILTER-only group \
         ({ FILTER(bound($this)) }); the query yields no solutions",
    ),
    (
        "sparql/pre-binding/shapesGraph-001",
        "$shapesGraph/$currentShape pre-bound variables are unsupported (the \
         SPARQL dataset carries no shapes graph), so the constraint never fires",
    ),
];

// ── Test-case model ───────────────────────────────────────────────────────────

/// Comparison tuple: `(focus, path, value, component, severity)` — see the
/// module header for the normalization rules.
type Tuple = (String, Option<String>, Option<String>, String, String);

/// Result multiset: tuple → occurrence count.
type Multiset = BTreeMap<Tuple, usize>;

enum Expected {
    /// `mf:result sht:Failure` — the validator must reject the input.
    Failure,
    /// A full expected `sh:ValidationReport`.
    Report { conforms: bool, results: Multiset },
}

struct TestCase {
    /// Entry IRI relative to the corpus root, e.g. `core/node/and-001`.
    id: String,
    /// Manifest section, e.g. `core/node`.
    section: String,
    shapes_path: PathBuf,
    data_path: PathBuf,
    expected: Expected,
}

// ── Graph helpers ─────────────────────────────────────────────────────────────

fn named(iri: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(iri))
}

/// All objects of `(subject, predicate, ?)`.
fn objects(g: &IrDataGraph, subject: &Term, predicate: &str) -> Vec<Term> {
    g.quads_for_pattern(
        Some(subject),
        Some(&named(predicate)),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|q| q.object)
    .collect()
}

/// The first object of `(subject, predicate, ?)`, if any.
fn object(g: &IrDataGraph, subject: &Term, predicate: &str) -> Option<Term> {
    objects(g, subject, predicate).into_iter().next()
}

/// Walk an RDF collection (`rdf:first`/`rdf:rest`) into a vec, in list order.
fn list_items(g: &IrDataGraph, head: &Term) -> Vec<Term> {
    let mut items = Vec::new();
    let mut node = head.clone();
    loop {
        if matches!(&node, Term::NamedNode(n) if n.as_str() == RDF_NIL) {
            break;
        }
        let Some(first) = object(g, &node, RDF_FIRST) else {
            break; // malformed list — stop rather than loop
        };
        items.push(first);
        match object(g, &node, RDF_REST) {
            Some(rest) => node = rest,
            None => break,
        }
    }
    items
}

// ── IRI ↔ path mapping ────────────────────────────────────────────────────────

fn file_iri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn iri_to_path(iri: &str) -> PathBuf {
    PathBuf::from(
        iri.strip_prefix("file://")
            .unwrap_or_else(|| panic!("expected a file:// IRI, got {iri}")),
    )
}

// ── Manifest walking ──────────────────────────────────────────────────────────

fn parse_turtle_file(path: &Path) -> Result<Arc<RdfDataset>, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    purrdf::parse_dataset(text.as_bytes(), "text/turtle", Some(&file_iri(path)))
        .map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

/// Recursively collect `sht:Validate` test cases from `manifest_path`.
fn collect_manifest(manifest_path: &Path, root: &Path, tests: &mut Vec<TestCase>) {
    let dataset =
        parse_turtle_file(manifest_path).unwrap_or_else(|e| panic!("manifest walk failed: {e}"));
    let g = IrDataGraph::new(dataset);

    // Sub-manifests: recurse in sorted order for a deterministic scoreboard.
    let mut includes: Vec<PathBuf> = g
        .quads_for_pattern(None, Some(&named(mf::INCLUDE)), None, GraphFilter::AnyGraph)
        .into_iter()
        .map(|q| match q.object {
            Term::NamedNode(n) => iri_to_path(n.as_str()),
            other => panic!(
                "{}: mf:include object must be an IRI, got {other}",
                manifest_path.display()
            ),
        })
        .collect();
    includes.sort();
    for include in includes {
        collect_manifest(&include, root, tests);
    }

    // Entries: an RDF list, in list (document) order.
    let entry_heads: Vec<Term> = g
        .quads_for_pattern(None, Some(&named(mf::ENTRIES)), None, GraphFilter::AnyGraph)
        .into_iter()
        .map(|q| q.object)
        .collect();
    for head in entry_heads {
        for entry in list_items(&g, &head) {
            if let Some(tc) = parse_entry(&g, &entry, manifest_path, root) {
                tests.push(tc);
            }
        }
    }
}

/// Parse one manifest entry into a [`TestCase`] (skipping non-`sht:Validate`).
fn parse_entry(
    g: &IrDataGraph,
    entry: &Term,
    manifest_path: &Path,
    root: &Path,
) -> Option<TestCase> {
    let is_validate = objects(g, entry, rdf::TYPE)
        .iter()
        .any(|t| matches!(t, Term::NamedNode(n) if n.as_str() == sht::VALIDATE));
    if !is_validate {
        return None;
    }

    let entry_iri = match entry {
        Term::NamedNode(n) => n.as_str().to_owned(),
        other => panic!(
            "{}: sht:Validate entry must be an IRI, got {other}",
            manifest_path.display()
        ),
    };
    let root_iri = format!("{}/", file_iri(root));
    let id = entry_iri
        .strip_prefix(&root_iri)
        .unwrap_or(&entry_iri)
        .to_owned();
    let section = id
        .rsplit_once('/')
        .map_or_else(String::new, |(dir, _)| dir.to_owned());

    let action = object(g, entry, mf::ACTION)
        .unwrap_or_else(|| panic!("{id}: sht:Validate entry has no mf:action"));
    let graph_path = |pred: &str, role: &str| -> PathBuf {
        match object(g, &action, pred) {
            Some(Term::NamedNode(n)) => iri_to_path(n.as_str()),
            other => panic!("{id}: mf:action has no IRI {role}, got {other:?}"),
        }
    };
    let shapes_path = graph_path(sht::SHAPES_GRAPH, "sht:shapesGraph");
    let data_path = graph_path(sht::DATA_GRAPH, "sht:dataGraph");

    let result = object(g, entry, mf::RESULT)
        .unwrap_or_else(|| panic!("{id}: sht:Validate entry has no mf:result"));
    let expected = match &result {
        Term::NamedNode(n) if n.as_str() == sht::FAILURE => Expected::Failure,
        report_node => Expected::Report {
            conforms: expected_conforms(g, report_node, &id),
            results: expected_multiset(g, report_node),
        },
    };

    Some(TestCase {
        id,
        section,
        shapes_path,
        data_path,
        expected,
    })
}

/// Read the expected `sh:conforms` boolean off the expected-report node.
fn expected_conforms(g: &IrDataGraph, report_node: &Term, id: &str) -> bool {
    match object(g, report_node, sh::CONFORMS) {
        Some(Term::Literal(l)) => match l.value() {
            "true" => true,
            "false" => false,
            other => panic!("{id}: unrecognized sh:conforms literal {other:?}"),
        },
        other => panic!("{id}: expected report has no sh:conforms literal, got {other:?}"),
    }
}

/// Build the expected result multiset from the expected-report node.
fn expected_multiset(g: &IrDataGraph, report_node: &Term) -> Multiset {
    let mut multiset = Multiset::new();
    for result in objects(g, report_node, sh::RESULT) {
        let focus = object(g, &result, sh::FOCUS_NODE).map_or_else(String::new, |t| norm(&t));
        let path = object(g, &result, sh::RESULT_PATH).map(|t| norm(&t));
        let value = object(g, &result, sh::VALUE).map(|t| norm(&t));
        let component = object(g, &result, sh::SOURCE_CONSTRAINT_COMPONENT)
            .map_or_else(String::new, |t| norm(&t));
        let severity = object(g, &result, sh::RESULT_SEVERITY)
            .map_or_else(|| format!("<{}>", sh::VIOLATION), |t| norm(&t));
        *multiset
            .entry((focus, path, value, component, severity))
            .or_insert(0) += 1;
    }
    multiset
}

/// Normalize a term for comparison: blank nodes (incl. complex-path bnodes)
/// collapse to `_:`; everything else uses the engine's canonical rendering.
fn norm(t: &Term) -> String {
    match t {
        Term::BlankNode(_) => "_:".to_owned(),
        other => other.to_string(),
    }
}

// ── Running one case ──────────────────────────────────────────────────────────

/// Load graphs, run the engine. `Err` carries the parse/validation error.
fn validate_case(tc: &TestCase) -> Result<purrdf_shapes::report::ValidationReport, String> {
    let shapes_text = fs::read_to_string(&tc.shapes_path)
        .map_err(|e| format!("cannot read shapes {}: {e}", tc.shapes_path.display()))?;
    let shapes_dataset = purrdf::parse_dataset(
        shapes_text.as_bytes(),
        "text/turtle",
        Some(&file_iri(&tc.shapes_path)),
    )
    .map_err(|e| format!("shapes graph parse error: {e}"))?;
    let doc_prefixes = purrdf_shapes::text_ingest::extract_prefixes(&shapes_text);
    let shapes = purrdf_shapes::shapes::from_dataset_with_prefixes(&shapes_dataset, &doc_prefixes)
        .map_err(|e| format!("shapes parse error: {e}"))?;

    let data_dataset = if tc.data_path == tc.shapes_path {
        shapes_dataset
    } else {
        parse_turtle_file(&tc.data_path).map_err(|e| format!("data graph parse error: {e}"))?
    };

    purrdf_shapes::engine::validate_dataset(data_dataset.as_ref(), &shapes)
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
fn validate_case_no_panic(
    tc: &TestCase,
) -> Result<purrdf_shapes::report::ValidationReport, String> {
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
fn run_case(tc: &TestCase) -> Result<(), String> {
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
    let root = Path::new(VECTORS_DIR)
        .canonicalize()
        .expect("vectors/shacl corpus directory must exist");

    let mut tests: Vec<TestCase> = Vec::new();
    collect_manifest(&root.join("manifest.ttl"), &root, &mut tests);

    // First-party AF (Advanced Features) seam: the vendored root manifest stays
    // pristine (no mf:include is added to it), so future upstream AF manifests
    // slot in at `af/manifest.ttl` and are discovered here without re-vendoring.
    // The placeholder ships zero entries today, so this discovers 0 tests and
    // TOTAL_TESTS stays 120.
    let af = root.join("af/manifest.ttl");
    if af.exists() {
        collect_manifest(&af, &root, &mut tests);
    }

    assert_eq!(
        tests.len(),
        TOTAL_TESTS,
        "discovered test count drifted — the vendored corpus is frozen, so this \
         means the manifest walk changed; update TOTAL_TESTS only on a deliberate \
         corpus re-vendor"
    );

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
        TOTAL_TESTS,
        "every discovered test must be a pass or a ledgered xfail"
    );
}
