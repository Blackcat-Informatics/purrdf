// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The one reader for the two SHACL corpora this crate is graded against.**
//!
//! Two corpora exist, with two different discovery rules, and more than one
//! harness needs to walk each of them:
//!
//! * `vectors/shacl/` — the vendored W3C data-shapes suite plus the first-party
//!   `af/` seam. Discovery is MANIFEST-DRIVEN: nothing reads the directory, so a
//!   case file exists for a harness only if some `mf:include` chain reaches it.
//! * `crates/shapes/corpus/` — the first-party corpus. Discovery is by
//!   `read_dir` over numbered case directories, each carrying `data.nt`,
//!   `shapes.ttl` and `expected-report.nt`.
//!
//! This module is where both live, so that the harnesses grading those corpora
//! differ only in what they GRADE. The alternative — one walker per harness — is
//! how two harnesses end up measuring two different corpora while sharing a
//! directory name: the manifest walk in particular is fifty lines of RDF-list
//! chasing whose bugs are all silent, because every one of them makes the corpus
//! SMALLER and a smaller corpus is a greener harness.
//!
//! The exact case counts live here for the same reason. [`W3C_TOTAL_CASES`] and
//! [`FIRST_PARTY_TOTAL_CASES`] are asserted by the constructors below, so every
//! consumer inherits the drift guard rather than each restating a number that can
//! be updated in one place and forgotten in another.
//!
//! # What is discovery and what is grading
//!
//! Everything a MANIFEST says about a case is discovery and belongs here,
//! including the expected report: `mf:result` is manifest content, and reading it
//! needs the manifest dataset, which does not outlive the walk. What an ENGINE
//! does with a case — running it, normalizing the produced report, deciding
//! whether the two agree — is grading and belongs to the harness.
//!
//! Nothing here asserts a verdict. A consumer that discovered zero cases gets a
//! panic from the count assertions, not an empty vector.
//!
//! Two submodules extend this reader rather than copying it:
//!
//! * [`shacl12`] discovers the vendored W3C SHACL 1.2 suite
//!   (`vectors/shacl12/tests/`) through the same [`walk_manifest`] and the same
//!   `sht:Validate` entry parser, adding the 1.2 test types.
//! * [`report_grading`] is the one `sht:Validate` grader. It returns verdicts and
//!   asserts none, so both W3C harnesses grade a validation case identically.

#![allow(
    dead_code,
    reason = "one corpus reader serves several test binaries; each uses the part \
              of it that its own claim needs, and splitting the module per consumer \
              would reintroduce the walker duplication it exists to prevent"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf::RdfDataset;
use purrdf_shapes::ShapesImports;
use purrdf_shapes::data::{GraphFilter, native_quads};

pub(crate) mod report_grading;
pub(crate) mod shacl12;
use purrdf_shapes::model::{BoxRoleVocab, rdf, sh};
use purrdf_shapes::term::{NamedNode, Term};

// ── Corpus locations ──────────────────────────────────────────────────────────

/// The vendored W3C data-shapes suite plus the first-party `af/` seam.
pub(crate) const VECTORS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/shacl");

/// The first-party numbered-case corpus.
pub(crate) const CORPUS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/corpus");

/// Exact number of `sht:Validate` entries the manifest tree must discover.
/// Bump only when the vendored corpus itself changes (it is byte-frozen).
///
/// Note: the corpus ships 121 files with a `sht:Validate` entry in `core/` +
/// `sparql/`, but upstream's `sparql/component/manifest.ttl` never
/// `mf:include`s `nodeValidator-001.ttl`, so that subtree yields 120.
/// The SHACL-AF seam at `af/` adds 9 more `sht:Validate` entries — 6 vendored
/// from pySHACL's DASH tests and 3 first-party (no W3C SHACL-AF conformance
/// suite exists; see `vectors/shacl/af/README.md`).
pub(crate) const W3C_TOTAL_CASES: usize = 129;

/// Exact number of case directories under [`CORPUS_DIR`].
///
/// Asserted rather than merely non-empty so a removed or renamed corpus
/// directory fails fast instead of silently reducing coverage. Bump this when
/// adding a case.
pub(crate) const FIRST_PARTY_TOTAL_CASES: usize = 73;

// ── Vocabulary ────────────────────────────────────────────────────────────────

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

// ── The discovered case models ────────────────────────────────────────────────

/// Comparison tuple: `(focus, path, value, component, severity, source shape)` —
/// see [`norm`] for the normalization rules. A blank-node source shape compares as
/// `_:`, like every other blank node.
pub(crate) type Tuple = (
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
);

/// Result multiset: tuple → occurrence count.
pub(crate) type Multiset = BTreeMap<Tuple, usize>;

/// One expected result as the `sh:detail` grader reads it: its comparison tuple
/// and, when the expected report states any, its nested `sh:detail` results.
///
/// `details` is `None` when the expected result states no `sh:detail` — the
/// result's details are then not graded (see the `report_grading` module docs
/// for the SHACL 1.2 Core text that makes them optional) — and `Some` when it
/// states at least one, in which case the produced result's details must be
/// EXACTLY that multiset, recursively.
#[derive(Clone, Debug)]
pub(crate) struct ExpectedResult {
    pub(crate) tuple: Tuple,
    pub(crate) details: Option<Vec<Self>>,
}

/// What the manifest says a case's outcome must be.
pub(crate) enum Expected {
    /// `mf:result sht:Failure` — the validator must reject the input.
    Failure,
    /// A full expected `sh:ValidationReport`.
    Report { conforms: bool, results: Multiset },
}

/// One `sht:Validate` entry from the vendored manifest tree.
pub(crate) struct W3cCase {
    /// Entry IRI relative to the corpus root, e.g. `core/node/and-001`.
    pub(crate) id: String,
    /// Manifest section, e.g. `core/node`.
    pub(crate) section: String,
    pub(crate) shapes_path: PathBuf,
    pub(crate) data_path: PathBuf,
    /// IRI supplied by `sht:shapesGraph` (often `<>` resolving to the test file),
    /// used as the named graph for `$shapesGraph` pre-binding in SHACL-SPARQL.
    pub(crate) shapes_graph_iri: Option<String>,
    pub(crate) expected: Expected,
    /// The `sh:conformanceDisallows` IRIs of the EXPECTED report, which the suite
    /// uses as a validation parameter ("the test framework needs to use the
    /// values of sh:conformanceDisallows from the mf:result", W3C
    /// `core/validation-reports/conformance-disallows-001`). Empty when the
    /// expected report states none, which means the default set.
    pub(crate) conformance_disallows: Vec<String>,
    /// Every expected result that carries `sh:resultMessage`, with its messages
    /// as [`message_key`]s — compared EXACTLY, as a set. The suite asks a harness "to preserve all
    /// sh:resultMessage triples that are mentioned in the 'expected' results
    /// graph" (W3C `core/misc/message-001`), so these are graded beside the tuple
    /// multiset.
    pub(crate) expected_messages: Vec<(Tuple, BTreeSet<String>)>,
    /// Every expected result that carries a SHACL-SPARQL result annotation — a
    /// predicate outside `rdf:type` and the SHACL namespaces (SHACL 1.2 SPARQL
    /// Extensions, "Annotation Properties": the processor "copies the binding …
    /// into the validation result") — with its `(property, value)` pairs,
    /// compared EXACTLY as a set, the way messages are.
    pub(crate) expected_annotations: Vec<(Tuple, BTreeSet<(String, String)>)>,
    /// Every top-level expected result that states `sh:detail`, with its nested
    /// results — graded beside the tuple multiset: each must be carried by a
    /// distinct produced result with the same tuple whose details are exactly
    /// the stated ones (see [`ExpectedResult`]).
    pub(crate) expected_details: Vec<ExpectedResult>,
}

/// One numbered case directory from the first-party corpus.
pub(crate) struct FirstPartyCase {
    /// The directory name, e.g. `01-min-count`.
    pub(crate) name: String,
    pub(crate) shapes_path: PathBuf,
    pub(crate) data_path: PathBuf,
    pub(crate) expected_report_path: PathBuf,
}

/// The first-party corpus fixtures' caller-supplied box-role vocabulary: the
/// reifier-shape cases annotate shapes with `meta:graphBoxRole` terms under
/// `https://example.org/meta/`. PurRDF mints no vocabulary of its own, so every
/// harness over this corpus configures the vocab explicitly, exactly as a
/// consumer would.
pub(crate) fn first_party_box_role_vocab() -> BoxRoleVocab {
    BoxRoleVocab::for_namespace("https://example.org/meta/")
}

// ── Graph helpers ─────────────────────────────────────────────────────────────

pub(crate) fn named(iri: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(iri))
}

/// All objects of `(subject, predicate, ?)`.
pub(crate) fn objects(g: &RdfDataset, subject: &Term, predicate: &str) -> Vec<Term> {
    native_quads(
        g,
        Some(subject),
        Some(&named(predicate)),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(_, _, object)| object)
    .collect()
}

/// The first object of `(subject, predicate, ?)`, if any.
pub(crate) fn object(g: &RdfDataset, subject: &Term, predicate: &str) -> Option<Term> {
    objects(g, subject, predicate).into_iter().next()
}

/// Walk an RDF collection (`rdf:first`/`rdf:rest`) into a vec, in list order.
///
/// The corpora are frozen, so a malformed list — a cell with no `rdf:first`, no
/// `rdf:rest`, more than one of either, or a `rdf:rest` chain that revisits a
/// cell — is a discovery bug, not something to read around: stopping early would
/// hand every consumer a SHORTER list (fewer entries, fewer expected results),
/// and a shorter list is a greener harness. It panics instead.
pub(crate) fn list_items(g: &RdfDataset, head: &Term) -> Vec<Term> {
    let mut items = Vec::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut node = head.clone();
    loop {
        if matches!(&node, Term::NamedNode(n) if n.as_str() == RDF_NIL) {
            break;
        }
        assert!(
            visited.insert(node.to_string()),
            "malformed RDF list: the rdf:rest chain from {head} revisits {node}"
        );
        let firsts = objects(g, &node, RDF_FIRST);
        let rests = objects(g, &node, RDF_REST);
        let ([first], [rest]) = (firsts.as_slice(), rests.as_slice()) else {
            panic!(
                "malformed RDF list: cell {node} (from {head}) has {} rdf:first and {} \
                 rdf:rest values, exactly one of each is required",
                firsts.len(),
                rests.len()
            );
        };
        items.push(first.clone());
        node = rest.clone();
    }
    items
}

/// Normalize a term for comparison: blank nodes (incl. complex-path bnodes)
/// collapse to `_:`; everything else uses the engine's canonical rendering.
///
/// Expected reports in the vendored suite use their own blank-node labels, which
/// cannot match the engine's, so identity comparison is only possible for IRIs
/// and literals.
pub(crate) fn norm(t: &Term) -> String {
    match t {
        Term::BlankNode(_) => "_:".to_owned(),
        other => other.to_string(),
    }
}

// ── IRI ↔ path mapping ────────────────────────────────────────────────────────

// ── The vendored suites' owl:imports ─────────────────────────────────────────

/// DASH, the TopBraid vocabulary the vendored W3C `sparql/component/validator-001`
/// cases (SHACL 1.0 and SHACL 1.2) import.
pub(crate) const DASH: &str = "http://datashapes.org/dash";

/// The imports no document can be supplied for, so a case that imports one is an
/// EXPECTED REFUSAL rather than a validation.
///
/// DASH cannot be supplied as it is published, because it is not well-formed
/// SHACL: it gives `sh:validator` values that are `sh:JSValidator`s, where SHACL 1.2
/// SPARQL Extensions §4.2.3 says "The values of sh:validator must be ASK-based
/// validators", and `dash:uriTemplate` declares a parameter named `value`, which
/// §4.2.1 forbids. Its shapes would also change the case's verdict. A stand-in
/// document would be a fabricated ontology. PurRDF fetches nothing and refuses a
/// shapes graph whose imports closure is not in hand, so the honest grade of such a
/// case is that refusal, exactly.
pub(crate) const UNRESOLVABLE_IMPORTS: &[&str] = &[DASH];

/// The vendored W3C cases whose shapes graph imports an [`UNRESOLVABLE_IMPORTS`]
/// ontology: `(case id, the imports it must be refused for)`. The same id names the
/// case in the SHACL 1.0 and the SHACL 1.2 suite. Each is graded by
/// [`report_grading::grade_refused_import`] as an exact expected refusal and
/// reported as "refused: unresolvable import" — never as a pass.
pub(crate) const REFUSED_UNRESOLVABLE_IMPORT: &[(&str, &[&str])] =
    &[("sparql/component/validator-001", &[DASH])];

/// The expected refusal of `id`, if it is one.
pub(crate) fn refused_import(id: &str) -> Option<&'static [&'static str]> {
    REFUSED_UNRESOLVABLE_IMPORT
        .iter()
        .find(|(case, _)| *case == id)
        .map(|(_, iris)| *iris)
}

/// The import table a vendored W3C case's shapes graph loads with: for every
/// `owl:imports` the graph does not already resolve itself, the document the harness
/// supplies for it.
///
/// PurRDF refuses a shapes graph whose imports closure is not in hand. A harness is a
/// caller like any other, so it resolves imports the way a caller does: by supplying a
/// document. The prefix idiom the suites use (`owl:imports` of a node the case describes
/// with `sh:declare`) is resolved by the engine's own rule and needs nothing here. An
/// [`UNRESOLVABLE_IMPORTS`] ontology is left unresolved on purpose, so the load refuses
/// it. Any other import panics: a newly vendored case with an import has to be resolved
/// here on purpose, not skipped.
pub(crate) fn w3c_case_imports(dataset: &RdfDataset) -> ShapesImports {
    let imports = ShapesImports::new();
    for iri in imports.import_map().unresolved_imports(dataset) {
        assert!(
            UNRESOLVABLE_IMPORTS.contains(&iri.as_str()),
            "a vendored case owl:imports <{iri}>, which the harness does not supply a \
             document for; resolve it in `w3c_case_imports` or, if no document can be \
             supplied, list it in UNRESOLVABLE_IMPORTS"
        );
    }
    imports
}

pub(crate) fn file_iri(path: &Path) -> String {
    format!("file://{}", path.display())
}

pub(crate) fn iri_to_path(iri: &str) -> PathBuf {
    PathBuf::from(
        iri.strip_prefix("file://")
            .unwrap_or_else(|| panic!("expected a file:// IRI, got {iri}")),
    )
}

/// Parse a Turtle file with the workspace's own codec (dogfooding), resolving
/// relative IRIs against the file's own `file://` location.
pub(crate) fn parse_turtle_file(path: &Path) -> Result<Arc<RdfDataset>, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    purrdf::parse_dataset(text.as_bytes(), "text/turtle", Some(&file_iri(path)))
        .map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

// ── W3C manifest walking ──────────────────────────────────────────────────────

/// The vendored corpus root, canonicalized.
pub(crate) fn w3c_root() -> PathBuf {
    Path::new(VECTORS_DIR)
        .canonicalize()
        .expect("vectors/shacl corpus directory must exist")
}

/// Every `sht:Validate` case the manifest tree reaches, in manifest order.
///
/// Asserts [`W3C_TOTAL_CASES`] and, for the `af/` seam, that every case file on
/// disk is reachable from a manifest — see
/// [`af_case_files_are_all_reachable_from_a_manifest`].
pub(crate) fn w3c_cases() -> Vec<W3cCase> {
    let root = w3c_root();
    let mut cases: Vec<W3cCase> = Vec::new();
    collect_manifest(&root.join("manifest.ttl"), &root, &mut cases);

    // First-party AF (Advanced Features) seam: the vendored root manifest stays
    // pristine (no mf:include is added to it), so future upstream AF manifests
    // slot in at `af/manifest.ttl` and are discovered here without re-vendoring.
    // Today this adds 9 SHACL-AF validation tests from expression/, function/,
    // and target/ sub-manifests.
    let af = root.join("af/manifest.ttl");
    if af.exists() {
        collect_manifest(&af, &root, &mut cases);
        af_case_files_are_all_reachable_from_a_manifest(&root.join("af"));
    }

    assert_eq!(
        cases.len(),
        W3C_TOTAL_CASES,
        "discovered test count drifted — the vendored corpus is frozen, so this \
         means the manifest walk changed; update W3C_TOTAL_CASES only on a deliberate \
         corpus re-vendor"
    );
    cases
}

/// Recursively collect `sht:Validate` test cases from `manifest_path`.
fn collect_manifest(manifest_path: &Path, root: &Path, cases: &mut Vec<W3cCase>) {
    walk_manifest(manifest_path, &mut |g, entry, manifest| {
        if let Some(tc) = parse_entry(g, entry, manifest, root) {
            cases.push(tc);
        }
    });
}

/// The `mf:include` targets of one manifest, in sorted order.
///
/// Both spellings the W3C suites use are read: one `mf:include <m.ttl>` triple
/// per sub-manifest (the data-shapes suites), and ONE `mf:include ( <a> <b> )`
/// whose object is an RDF list (the SPARQL 1.2 RL suite). A list member, like a
/// single object, must be an IRI.
fn manifest_includes(g: &RdfDataset, manifest_path: &Path) -> Vec<PathBuf> {
    let mut includes: Vec<PathBuf> = Vec::new();
    for (_, _, object) in native_quads(
        g,
        None,
        Some(&named(mf::INCLUDE)),
        None,
        GraphFilter::AnyGraph,
    ) {
        let members = match &object {
            Term::NamedNode(_) => vec![object.clone()],
            Term::BlankNode(_) => list_items(g, &object),
            other => panic!(
                "{}: mf:include object must be an IRI or a list of IRIs, got {other}",
                manifest_path.display()
            ),
        };
        for member in members {
            match member {
                Term::NamedNode(n) => includes.push(iri_to_path(n.as_str())),
                other => panic!(
                    "{}: mf:include member must be an IRI, got {other}",
                    manifest_path.display()
                ),
            }
        }
    }
    includes.sort();
    includes
}

/// Walk the manifest tree rooted at `manifest_path`: every `mf:include` is
/// recursed into (sorted, for a deterministic scoreboard), then every member of
/// every `mf:entries` list is handed to `visit` in list (document) order,
/// together with the manifest dataset it lives in and that manifest's path.
///
/// The walker judges nothing about an entry — which test types a harness grades
/// is the visitor's decision — so the two SHACL harnesses share one list-chasing
/// implementation and differ only in what they keep.
pub(crate) fn walk_manifest(
    manifest_path: &Path,
    visit: &mut dyn FnMut(&Arc<RdfDataset>, &Term, &Path),
) {
    let g =
        parse_turtle_file(manifest_path).unwrap_or_else(|e| panic!("manifest walk failed: {e}"));

    for include in manifest_includes(&g, manifest_path) {
        walk_manifest(&include, visit);
    }

    // Entries: an RDF list, in list (document) order.
    let entry_heads: Vec<Term> = native_quads(
        &g,
        None,
        Some(&named(mf::ENTRIES)),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(_, _, object)| object)
    .collect();
    for head in entry_heads {
        for entry in list_items(&g, &head) {
            visit(&g, &entry, manifest_path);
        }
    }
}

/// Parse one manifest entry into a [`W3cCase`] (skipping non-`sht:Validate`).
pub(crate) fn parse_entry(
    g: &RdfDataset,
    entry: &Term,
    manifest_path: &Path,
    root: &Path,
) -> Option<W3cCase> {
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

    let shapes_graph_iri = object(g, &action, sht::SHAPES_GRAPH).and_then(|t| match t {
        Term::NamedNode(n) => Some(n.as_str().to_owned()),
        _ => None,
    });

    let result = object(g, entry, mf::RESULT)
        .unwrap_or_else(|| panic!("{id}: sht:Validate entry has no mf:result"));
    let (
        expected,
        conformance_disallows,
        expected_messages,
        expected_annotations,
        expected_details,
    ) = match &result {
        Term::NamedNode(n) if n.as_str() == sht::FAILURE => (
            Expected::Failure,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
        report_node => (
            Expected::Report {
                conforms: expected_conforms(g, report_node, &id),
                results: expected_multiset(g, report_node),
            },
            objects(g, report_node, sh::CONFORMANCE_DISALLOWS)
                .into_iter()
                .map(|level| match level {
                    Term::NamedNode(n) => n.as_str().to_owned(),
                    other => panic!("{id}: sh:conformanceDisallows value {other} is not an IRI"),
                })
                .collect(),
            expected_result_messages(g, report_node),
            expected_result_annotations(g, report_node),
            expected_result_details(g, report_node, &id),
        ),
    };

    Some(W3cCase {
        id,
        section,
        shapes_path,
        data_path,
        shapes_graph_iri,
        expected,
        conformance_disallows,
        expected_messages,
        expected_annotations,
        expected_details,
    })
}

/// Read the expected `sh:conforms` boolean off the expected-report node.
fn expected_conforms(g: &RdfDataset, report_node: &Term, id: &str) -> bool {
    match object(g, report_node, sh::CONFORMS) {
        Some(Term::Literal(l)) => match l.value() {
            "true" => true,
            "false" => false,
            other => panic!("{id}: unrecognized sh:conforms literal {other:?}"),
        },
        other => panic!("{id}: expected report has no sh:conforms literal, got {other:?}"),
    }
}

/// The comparison tuple of one expected result node.
fn expected_tuple(g: &RdfDataset, result: &Term) -> Tuple {
    let focus = object(g, result, sh::FOCUS_NODE).map_or_else(String::new, |t| norm(&t));
    let path = object(g, result, sh::RESULT_PATH).map(|t| norm(&t));
    let value = object(g, result, sh::VALUE).map(|t| norm(&t));
    let component =
        object(g, result, sh::SOURCE_CONSTRAINT_COMPONENT).map_or_else(String::new, |t| norm(&t));
    let severity = object(g, result, sh::RESULT_SEVERITY)
        .map_or_else(|| format!("<{}>", sh::VIOLATION), |t| norm(&t));
    // An expected result that states no source shape compares as the empty string,
    // which no produced result carries: every result the engine produces names its
    // shape, so such an expectation fails loudly instead of matching anything.
    let source_shape = object(g, result, sh::SOURCE_SHAPE).map_or_else(String::new, |t| norm(&t));
    (focus, path, value, component, severity, source_shape)
}

/// One message as the grader compares it: the literal's N-Triples rendering, so
/// the lexical form, the language tag, the base direction and the datatype all
/// take part.
pub(crate) fn message_key(message: &Term) -> String {
    message.to_string()
}

/// Every expected result carrying `sh:resultMessage`, with its messages as
/// [`message_key`]s.
fn expected_result_messages(g: &RdfDataset, report_node: &Term) -> Vec<(Tuple, BTreeSet<String>)> {
    objects(g, report_node, sh::RESULT)
        .into_iter()
        .filter_map(|result| {
            let messages: BTreeSet<String> = objects(g, &result, sh::RESULT_MESSAGE)
                .into_iter()
                .map(|message| message_key(&message))
                .collect();
            (!messages.is_empty()).then(|| (expected_tuple(g, &result), messages))
        })
        .collect()
}

/// Whether `predicate` is a result-annotation property on an expected result:
/// neither `rdf:type` nor a term of the SHACL or SHACL node-expression namespace.
pub(crate) fn is_annotation_property(predicate: &str) -> bool {
    predicate != "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
        && !predicate.starts_with("http://www.w3.org/ns/shacl#")
        && !predicate.starts_with("http://www.w3.org/ns/shacl-node-expr#")
}

/// Every expected result carrying result annotations, with its `(property,
/// value)` pairs, each value [`norm`]alized.
fn expected_result_annotations(
    g: &RdfDataset,
    report_node: &Term,
) -> Vec<(Tuple, BTreeSet<(String, String)>)> {
    objects(g, report_node, sh::RESULT)
        .into_iter()
        .filter_map(|result| {
            let pairs: BTreeSet<(String, String)> =
                native_quads(g, Some(&result), None, None, GraphFilter::AnyGraph)
                    .into_iter()
                    .filter(|(_, predicate, _)| is_annotation_property(predicate.as_str()))
                    .map(|(_, predicate, value)| {
                        (format!("<{}>", predicate.as_str()), norm(&value))
                    })
                    .collect();
            (!pairs.is_empty()).then(|| (expected_tuple(g, &result), pairs))
        })
        .collect()
}

/// Every top-level expected result that states `sh:detail`, as an
/// [`ExpectedResult`] tree.
fn expected_result_details(g: &RdfDataset, report_node: &Term, id: &str) -> Vec<ExpectedResult> {
    objects(g, report_node, sh::RESULT)
        .into_iter()
        .map(|result| expected_result(g, &result, id, &mut Vec::new()))
        .filter(|result| result.details.is_some())
        .collect()
}

/// One expected result and, recursively, the `sh:detail` results it states. A
/// detail chain that revisits a result is a malformed frozen corpus and panics,
/// as a malformed RDF list does.
fn expected_result(
    g: &RdfDataset,
    result: &Term,
    id: &str,
    path: &mut Vec<Term>,
) -> ExpectedResult {
    assert!(
        !path.contains(result),
        "{id}: the sh:detail chain revisits the expected result {result}"
    );
    path.push(result.clone());
    let details = objects(g, result, sh::DETAIL);
    let details = (!details.is_empty()).then(|| {
        details
            .iter()
            .map(|detail| expected_result(g, detail, id, path))
            .collect()
    });
    path.pop();
    ExpectedResult {
        tuple: expected_tuple(g, result),
        details,
    }
}

/// Build the expected result multiset from the expected-report node.
fn expected_multiset(g: &RdfDataset, report_node: &Term) -> Multiset {
    let mut multiset = Multiset::new();
    for result in objects(g, report_node, sh::RESULT) {
        *multiset.entry(expected_tuple(g, &result)).or_insert(0) += 1;
    }
    multiset
}

/// Prove every `.ttl` under `af/` is REACHED by a manifest, so the corpus on disk
/// and the corpus the harnesses run are the same corpus.
///
/// [`W3C_TOTAL_CASES`] alone cannot say this. Discovery under `af/` is by
/// `mf:include` only — nothing reads the directory — so a case file added without
/// a manifest entry runs nowhere, leaves the count untouched, and reddens
/// nothing. It would look exactly like a case that passes. (The sibling rules
/// harness has no such gap: it discovers by `read_dir`.) The byte-freeze manifest
/// is not a backstop either, because refreshing it is the documented step for
/// adding a case.
///
/// Both directions matter and both are checked: an unreferenced file, and a
/// manifest that names a file that does not exist.
pub(crate) fn af_case_files_are_all_reachable_from_a_manifest(af_root: &Path) {
    let mut on_disk: BTreeSet<PathBuf> = BTreeSet::new();
    let mut referenced: BTreeSet<PathBuf> = BTreeSet::new();
    let mut manifests: Vec<PathBuf> = vec![af_root.join("manifest.ttl")];
    let mut seen_manifests: BTreeSet<PathBuf> = BTreeSet::new();

    // Every `.ttl` beneath `af/`, manifests included — except `af/rules/`, which
    // is a different corpus with a different, already-tight discovery: the SHACL
    // Rules harness (`crates/shapes/tests/rules_conformance.rs`) walks it with
    // `read_dir` and asserts an exact `TOTAL_CASES`, so an unwired case there
    // already fails. It has no manifest and must not be measured against one.
    let mut dirs = vec![af_root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        if dir.file_name().is_some_and(|n| n == "rules") {
            continue;
        }
        for entry in fs::read_dir(&dir).expect("af corpus directory must be readable") {
            let path = entry.expect("af corpus entry must be readable").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "ttl") {
                on_disk.insert(path);
            }
        }
    }

    // Every `mf:include` target, transitively.
    while let Some(manifest) = manifests.pop() {
        if !seen_manifests.insert(manifest.clone()) {
            continue;
        }
        let text = fs::read_to_string(&manifest)
            .unwrap_or_else(|e| panic!("manifest {} must be readable: {e}", manifest.display()));
        let parent = manifest
            .parent()
            .expect("a manifest always has a parent directory")
            .to_path_buf();
        for line in text.lines() {
            let Some(rest) = line.split("mf:include").nth(1) else {
                continue;
            };
            let Some(open) = rest.find('<') else { continue };
            let Some(close) = rest[open + 1..].find('>') else {
                continue;
            };
            let target = parent.join(&rest[open + 1..open + 1 + close]);
            let target = target.canonicalize().unwrap_or_else(|e| {
                panic!(
                    "manifest {} includes {}, which does not exist: {e}",
                    manifest.display(),
                    target.display()
                )
            });
            referenced.insert(target.clone());
            if target.file_name().is_some_and(|n| n == "manifest.ttl") {
                manifests.push(target);
            }
        }
    }
    // A manifest is reached by being walked, not by being included.
    referenced.extend(seen_manifests.iter().cloned());

    let orphans: Vec<String> = on_disk
        .iter()
        .filter(|p| !referenced.contains(*p))
        .map(|p| p.display().to_string())
        .collect();
    assert!(
        orphans.is_empty(),
        "these af/ case files are on disk but no manifest includes them, so they run \
         nowhere and no gate would notice: {orphans:?}"
    );
}

// ── First-party corpus walking ────────────────────────────────────────────────

/// Every numbered case directory under [`CORPUS_DIR`], in sorted (case-name)
/// order. Asserts [`FIRST_PARTY_TOTAL_CASES`].
pub(crate) fn first_party_cases() -> Vec<FirstPartyCase> {
    let corpus_path = Path::new(CORPUS_DIR);
    assert!(
        corpus_path.exists(),
        "corpus directory not found at {CORPUS_DIR}"
    );

    let mut dirs: Vec<PathBuf> = fs::read_dir(corpus_path)
        .expect("failed to read corpus dir")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_dir() { Some(path) } else { None }
        })
        .collect();
    dirs.sort();

    assert_eq!(
        dirs.len(),
        FIRST_PARTY_TOTAL_CASES,
        "unexpected corpus case count — update FIRST_PARTY_TOTAL_CASES when \
         adding/removing a corpus case"
    );

    dirs.into_iter()
        .map(|dir| FirstPartyCase {
            name: dir
                .file_name()
                .expect("a corpus case directory always has a name")
                .to_string_lossy()
                .into_owned(),
            shapes_path: dir.join("shapes.ttl"),
            data_path: dir.join("data.nt"),
            expected_report_path: dir.join("expected-report.nt"),
        })
        .collect()
}

// ── The corpus relation ───────────────────────────────────────────────────────

/// The ONE relation IRI the first-party corpus may reach, registered by every
/// harness that grades the corpus.
///
/// Its own namespace, distinct from the `example.org/ns#` the cases use for data, so
/// no case can name it by accident and every case that names it means to.
pub(crate) const CORPUS_REL: &str = "http://example.org/corpus/rel/flagged";

/// The one row the corpus relation answers with: `ex:flagged` is flagged.
///
/// This is the only place that fact exists. No case's `data.nt` mentions
/// [`CORPUS_REL`], so a case whose expected report depends on this row is a case that
/// can only pass if the call really resolved — which is what makes a relation case
/// gradeable in a frozen corpus at all. A body whose call was lowered to an ordinary
/// triple pattern would match nothing and score every focus node the same.
#[derive(Debug)]
pub(crate) struct CorpusRelation {
    modes: [purrdf_sparql_eval::BindingPattern; 1],
    opens: Arc<AtomicU64>,
}

#[derive(Debug)]
struct CorpusCursor {
    rows: std::vec::IntoIter<purrdf_sparql_eval::PfRow>,
}

impl purrdf_sparql_eval::PfCursor for CorpusCursor {
    fn next(&mut self) -> Result<Option<purrdf_sparql_eval::PfRow>, purrdf_sparql_eval::EvalError> {
        Ok(self.rows.next())
    }
}

impl purrdf_sparql_eval::PropertyFunction for CorpusRelation {
    fn volatility(&self) -> purrdf_sparql_eval::Volatility {
        purrdf_sparql_eval::Volatility::Stable
    }

    fn arity(&self) -> purrdf_sparql_eval::PfArity {
        purrdf_sparql_eval::PfArity::new(1, 1)
    }

    fn modes(&self) -> &[purrdf_sparql_eval::BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: purrdf_sparql_eval::BindingPattern) -> u64 {
        1
    }

    fn open(
        &self,
        _args: &purrdf_sparql_eval::PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn purrdf_sparql_eval::PfCursor>, purrdf_sparql_eval::EvalError> {
        self.opens.fetch_add(1, Ordering::Relaxed);
        let rows = vec![vec![
            purrdf_core::TermValue::iri("http://example.org/ns#flagged"),
            purrdf_core::TermValue::iri("http://example.org/ns#yes"),
        ]];
        Ok(Box::new(CorpusCursor {
            rows: rows.into_iter(),
        }))
    }
}

/// The corpus relation registry, plus the counter that says whether it was reached.
///
/// Every harness grading the corpus installs this, so the three of them cannot be
/// validating the same cases under different environments — which they were, before a
/// case existed that could tell.
pub(crate) fn corpus_relations() -> (
    Arc<purrdf_sparql_eval::PropertyFunctionRegistry>,
    Arc<AtomicU64>,
) {
    let opens = Arc::new(AtomicU64::new(0));
    let mut registry = purrdf_sparql_eval::PropertyFunctionRegistry::new();
    registry.register(
        CORPUS_REL.to_owned(),
        Arc::new(CorpusRelation {
            modes: [purrdf_sparql_eval::BindingPattern::from_code("ff")],
            opens: Arc::clone(&opens),
        }),
    );
    (Arc::new(registry), opens)
}
