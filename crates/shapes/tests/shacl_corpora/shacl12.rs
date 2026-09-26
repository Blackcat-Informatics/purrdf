// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **Discovery for the vendored W3C SHACL 1.2 test suite** at
//! `vectors/shacl12/tests/`.
//!
//! The suite has two manifest roots, and both are walked:
//!
//! * `manifest.ttl` — `mf:include`s `core/`, `sparql/`, `node-expr/` and
//!   `inference-rules/`, whose entries are `sht:Validate`, `sht:EvalNodeExpr` and
//!   `sht:Infer`.
//! * `sparql-rl/manifest-sparql-rl.ttl` — the SPARQL 1.2 RL suite, which the
//!   top-level manifest does NOT include. Its README names this file as the
//!   suite's own top-level manifest, and its `mf:include` is a single RDF LIST
//!   rather than one triple per sub-manifest.
//!
//! Discovery is total, and says so rather than hoping so:
//!
//! * an entry of a type this module does not know is a PANIC, never a skip;
//! * every `.ttl` under the suite root that lists `mf:entries` must be reached
//!   from a root ([`W3C12_UNREACHED_MANIFESTS`] is empty). Three vendored files
//!   carry approved entries that no upstream manifest includes; they are walked
//!   as extra roots, each with its reason, in [`W3C12_UNINCLUDED_MANIFESTS`];
//! * every node a reached manifest TYPES as a test must be one of its listed
//!   entries, so a test declared but never listed cannot vanish either — save
//!   the one evidenced stray in [`W3C12_UNLISTED_TYPED_NODES`];
//! * the discovered total must equal [`W3C12_TOTAL_CASES`], and test ids must be
//!   unique.
//!
//! As in the parent module, everything a manifest says is discovery and lives
//! here; running a case and deciding whether it passed is the harness's job.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::data::{GraphFilter, native_quads};
use purrdf_shapes::model::rdf;
use purrdf_shapes::term::Term;

use super::{
    W3cCase, file_iri, iri_to_path, list_items, manifest_includes, mf, object, objects,
    parse_entry, parse_turtle_file, sht, walk_manifest,
};

/// The vendored SHACL 1.2 suite root.
pub(crate) const SHACL12_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/shacl12/tests");

/// Exact number of test entries the manifest roots must discover.
///
/// The per-type split is pinned by [`W3C12_CASES_BY_TYPE`]; this is its sum.
/// Bump only on a deliberate re-vendor (the corpus is byte-frozen).
pub(crate) const W3C12_TOTAL_CASES: usize = 547;

/// The discovered count of each test type, pinned so a walk that lost one kind
/// of test while gaining another cannot keep the total.
pub(crate) const W3C12_CASES_BY_TYPE: &[(&str, usize)] = &[
    ("sht:Validate", 174),
    ("sht:EvalNodeExpr", 143),
    ("sht:Infer", 27),
    ("srlt:RulesPositiveSyntaxTest", 109),
    ("srlt:RulesNegativeSyntaxTest", 30),
    ("srlt:RulesPositiveWellFormednessTest", 4),
    ("srlt:RulesNegativeWellFormednessTest", 4),
    ("srlt:RulesPositiveStratificationTest", 5),
    ("srlt:RulesNegativeStratificationTest", 5),
    ("srlt:RulesEvalTest", 46),
];

/// Manifests on disk that neither root reaches. Must stay empty; it is a named
/// constant only so the assertion message can say what it expected.
pub(crate) const W3C12_UNREACHED_MANIFESTS: &[&str] = &[];

/// Nodes a reached manifest types as a test but lists in no `mf:entries`, each
/// with the evidence that it is not a runnable test: `(id, why)`. Anything else
/// typed-but-unlisted fails discovery.
pub(crate) const W3C12_UNLISTED_TYPED_NODES: &[(&str, &str)] = &[(
    "sparql-rl/eval2/eval-assign-01",
    "sparql-rl/eval2/manifest.ttl types a stray :eval-assign-01 that its mf:entries \
     omits and whose srlt:ruleset (eval-assign-01.srl), srlt:data (data-empty.ttl) and \
     mf:result (eval-assign-01-results.ttl) do not exist in eval2/; the listed, runnable \
     eval-assign-01 is sparql-rl/eval/eval-assign-01",
)];

/// Vendored files that carry an approved test entry but that NO upstream manifest
/// `mf:include`s, walked as extra roots so their entries are graded rather than
/// silently absent: `(path relative to the suite root, why it is walked)`.
///
/// The SHACL 1.0 harness documents its one such file (`nodeValidator-001.ttl`)
/// and leaves it out; here the files are run, because a test that exists and is
/// approved but runs nowhere is exactly the silent coverage loss the reachability
/// guard exists to stop. Running them is not counting them: their entries are
/// discovered with `listed == false`, and the harness grades and reports them in
/// a category of their own, never among the approved suite's passes.
pub(crate) const W3C12_UNINCLUDED_MANIFESTS: &[(&str, &str)] = &[
    (
        "core/node/xone-002.ttl",
        "approved sh:xone test (empty sh:xone list); core/node/manifest.ttl includes \
         only xone-001 and xone-duplicate",
    ),
    (
        "core/node/xone-003.ttl",
        "approved sh:xone test (shacl-shacl sh:minListLength warning on an empty \
         sh:xone); core/node/manifest.ttl includes only xone-001 and xone-duplicate",
    ),
    (
        "inference-rules/rdfs/rdfs1.ttl",
        "byte-identical copy of inference-rules/rectangle-condition.ttl left in rdfs/; \
         its entry resolves to a distinct IRI (inference-rules/rdfs/rectangle-condition) \
         and is graded like any other",
    ),
];

const SHT: &str = "http://www.w3.org/ns/shacl-test#";
const SRLT: &str = "http://www.w3.org/ns/sparql-rl-tests#";

mod sht12 {
    pub(crate) const EVAL_NODE_EXPR: &str = "http://www.w3.org/ns/shacl-test#EvalNodeExpr";
    pub(crate) const INFER: &str = "http://www.w3.org/ns/shacl-test#Infer";
    pub(crate) const NODE_EXPR: &str = "http://www.w3.org/ns/shacl-test#nodeExpr";
    pub(crate) const FOCUS_NODE: &str = "http://www.w3.org/ns/shacl-test#focusNode";
    pub(crate) const IGNORE_ORDER: &str = "http://www.w3.org/ns/shacl-test#ignoreOrder";
    pub(crate) const SCOPE_PREFIX: &str = "http://www.w3.org/ns/shacl-test#scope-";
}

mod srlt {
    pub(crate) const RULESET: &str = "http://www.w3.org/ns/sparql-rl-tests#ruleset";
    pub(crate) const DATA: &str = "http://www.w3.org/ns/sparql-rl-tests#data";
}

const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

// ── The case models ───────────────────────────────────────────────────────────

/// The seven SPARQL 1.2 RL test types (`srlt:` vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SrlKind {
    PositiveSyntax,
    NegativeSyntax,
    PositiveWellFormedness,
    NegativeWellFormedness,
    PositiveStratification,
    NegativeStratification,
    Eval,
}

impl SrlKind {
    const ALL: [(Self, &'static str); 7] = [
        (Self::PositiveSyntax, "RulesPositiveSyntaxTest"),
        (Self::NegativeSyntax, "RulesNegativeSyntaxTest"),
        (
            Self::PositiveWellFormedness,
            "RulesPositiveWellFormednessTest",
        ),
        (
            Self::NegativeWellFormedness,
            "RulesNegativeWellFormednessTest",
        ),
        (
            Self::PositiveStratification,
            "RulesPositiveStratificationTest",
        ),
        (
            Self::NegativeStratification,
            "RulesNegativeStratificationTest",
        ),
        (Self::Eval, "RulesEvalTest"),
    ];

    fn from_iri(iri: &str) -> Option<Self> {
        let local = iri.strip_prefix(SRLT)?;
        Self::ALL
            .iter()
            .find(|(_, name)| *name == local)
            .map(|(kind, _)| *kind)
    }

    /// The `srlt:` local name.
    pub(crate) fn local_name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(kind, _)| *kind == self)
            .map(|(_, name)| *name)
            .expect("every SrlKind has a name in ALL")
    }
}

/// One `sht:EvalNodeExpr` entry.
///
/// The test file is both the manifest and the focus graph, and the node
/// expression is a node (usually blank) of that very dataset, so the dataset the
/// manifest was read from is carried: re-parsing the file would mint different
/// blank nodes and `expr` would name nothing in them.
pub(crate) struct NodeExprCase {
    pub(crate) file: PathBuf,
    pub(crate) dataset: Arc<RdfDataset>,
    /// `sht:nodeExpr`.
    pub(crate) expr: Term,
    /// `sht:focusNode`, when the entry gives one.
    pub(crate) focus: Option<Term>,
    /// Every `sht:scope-NAME value`, as `(NAME, value)`, in NAME order.
    pub(crate) scope: Vec<(String, Term)>,
    /// The `mf:result` list, in list order.
    pub(crate) expected: Vec<Term>,
    /// `sht:ignoreOrder true`.
    pub(crate) ignore_order: bool,
}

/// What an `sht:Infer` entry expects.
pub(crate) enum InferExpected {
    /// `mf:result sht:Failure` — loading or running the rules must fail.
    Failure,
    /// An inline list of `( s p o )` triples (possibly `rdf:nil`, i.e. none).
    Triples(Vec<[Term; 3]>),
    /// A separate Turtle file holding exactly the inferred triples.
    File(PathBuf),
}

/// One `sht:Infer` entry.
pub(crate) struct InferCase {
    pub(crate) shapes_path: PathBuf,
    pub(crate) data_path: PathBuf,
    pub(crate) shapes_graph_iri: String,
    pub(crate) expected: InferExpected,
}

/// One SPARQL 1.2 RL entry.
pub(crate) struct SrlCase {
    pub(crate) kind: SrlKind,
    /// The `.srl` rule set under test.
    pub(crate) ruleset: PathBuf,
    /// `srlt:data` of an evaluation test.
    pub(crate) data: Option<PathBuf>,
    /// `mf:result` of an evaluation test — the expected inference graph.
    pub(crate) result: Option<PathBuf>,
}

/// The body of a discovered case, by test type.
pub(crate) enum Body {
    Validate(W3cCase),
    NodeExpr(NodeExprCase),
    Infer(InferCase),
    Srl(SrlCase),
}

/// One discovered SHACL 1.2 test entry.
pub(crate) struct Case12 {
    /// Entry id relative to the suite root, e.g. `core/node/in-003`,
    /// `node-expr/shnex/var-bound`, `sparql-rl/eval/eval-basic-01`.
    pub(crate) id: String,
    /// The id's directory, e.g. `core/node`.
    pub(crate) section: String,
    pub(crate) body: Body,
    /// Whether an upstream manifest lists the entry. `false` for the entries of
    /// [`W3C12_UNINCLUDED_MANIFESTS`], which no upstream manifest includes: they
    /// are graded, but they are not the approved suite, and a harness reports
    /// them apart from it.
    pub(crate) listed: bool,
}

impl Case12 {
    /// The manifest's test type, as a prefixed name.
    pub(crate) fn type_label(&self) -> String {
        match &self.body {
            Body::Validate(_) => "sht:Validate".to_owned(),
            Body::NodeExpr(_) => "sht:EvalNodeExpr".to_owned(),
            Body::Infer(_) => "sht:Infer".to_owned(),
            Body::Srl(srl) => format!("srlt:{}", srl.kind.local_name()),
        }
    }
}

// ── Discovery ─────────────────────────────────────────────────────────────────

/// The suite root, canonicalized.
pub(crate) fn shacl12_root() -> PathBuf {
    Path::new(SHACL12_DIR)
        .canonicalize()
        .expect("vectors/shacl12/tests corpus directory must exist")
}

/// The manifest roots, in walk order, each with whether it is one of the suite's
/// own: the two top-level manifests (`true`), then [`W3C12_UNINCLUDED_MANIFESTS`]
/// (`false`).
fn manifest_roots(root: &Path) -> Vec<(PathBuf, bool)> {
    let mut roots = vec![
        (root.join("manifest.ttl"), true),
        (root.join("sparql-rl/manifest-sparql-rl.ttl"), true),
    ];
    for (relative, _) in W3C12_UNINCLUDED_MANIFESTS {
        let path = root.join(relative);
        assert!(path.is_file(), "{relative} is not a vendored file");
        roots.push((path, false));
    }
    roots
}

/// Every test entry the manifest roots reach, in manifest order.
///
/// Asserts [`W3C12_TOTAL_CASES`], [`W3C12_CASES_BY_TYPE`], id uniqueness, and the
/// reachability and listed-ness guards in the module docs.
pub(crate) fn shacl12_cases() -> Vec<Case12> {
    let root = shacl12_root();
    let mut cases: Vec<Case12> = Vec::new();
    for (manifest, listed) in manifest_roots(&root) {
        walk_manifest(&manifest, &mut |g, entry, manifest_path| {
            let mut case = parse_case(g, entry, manifest_path, &root);
            case.listed = listed;
            cases.push(case);
        });
    }

    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for case in &cases {
        assert!(
            ids.insert(case.id.as_str()),
            "two SHACL 1.2 entries share the id {}",
            case.id
        );
    }

    every_manifest_and_every_test_is_reached(&root, &ids);

    let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
    for case in &cases {
        *by_type.entry(case.type_label()).or_insert(0) += 1;
    }
    let pinned: BTreeMap<String, usize> = W3C12_CASES_BY_TYPE
        .iter()
        .map(|(label, n)| ((*label).to_owned(), *n))
        .collect();
    let discovered: BTreeMap<String, usize> = pinned
        .keys()
        .map(|label| (label.clone(), by_type.get(label).copied().unwrap_or(0)))
        .chain(by_type.iter().map(|(label, n)| (label.clone(), *n)))
        .collect();
    assert_eq!(
        discovered, pinned,
        "the per-type SHACL 1.2 test counts drifted — the vendored corpus is frozen, so \
         this means the manifest walk changed"
    );
    assert_eq!(
        cases.len(),
        W3C12_TOTAL_CASES,
        "discovered SHACL 1.2 test count drifted — update W3C12_TOTAL_CASES only on a \
         deliberate corpus re-vendor"
    );
    cases
}

/// The id of an entry: its IRI relative to the suite root.
///
/// The SPARQL 1.2 RL evaluation manifests name their entries under an assumed
/// test base (`https://w3c.github.io/rdf-tests/shacl/shacl12/`) shared by BOTH
/// `eval/` and `eval2/` — each of which has an `eval-assign-01` — so such an
/// entry is identified by its manifest's directory plus its local name instead.
fn entry_id(entry_iri: &str, manifest_path: &Path, root: &Path) -> String {
    let root_iri = format!("{}/", file_iri(root));
    if let Some(relative) = entry_iri.strip_prefix(&root_iri) {
        return relative.to_owned();
    }
    let dir = manifest_path
        .parent()
        .and_then(|p| p.strip_prefix(root).ok())
        .unwrap_or_else(|| panic!("manifest {} is outside the suite", manifest_path.display()));
    let local = entry_iri
        .rsplit(['/', '#'])
        .next()
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| panic!("entry IRI {entry_iri} has no local name"));
    format!("{}/{local}", dir.display())
}

/// The test-type IRIs of an entry.
fn entry_types(g: &RdfDataset, entry: &Term) -> Vec<String> {
    objects(g, entry, rdf::TYPE)
        .into_iter()
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n.as_str().to_owned()),
            _ => None,
        })
        .collect()
}

/// Parse one entry; an entry of any type this module does not know PANICS.
fn parse_case(g: &Arc<RdfDataset>, entry: &Term, manifest_path: &Path, root: &Path) -> Case12 {
    let Term::NamedNode(entry_iri) = entry else {
        panic!(
            "{}: test entry must be an IRI, got {entry}",
            manifest_path.display()
        );
    };
    let id = entry_id(entry_iri.as_str(), manifest_path, root);
    let section = id
        .rsplit_once('/')
        .map_or_else(String::new, |(dir, _)| dir.to_owned());
    let types = entry_types(g, entry);
    let [ty] = types.as_slice() else {
        panic!("{id}: a test entry must carry exactly one rdf:type, got {types:?}");
    };

    let body = if ty == sht::VALIDATE {
        let mut tc = parse_entry(g, entry, manifest_path, root)
            .unwrap_or_else(|| panic!("{id}: sht:Validate entry was not parsed"));
        tc.id.clone_from(&id);
        tc.section.clone_from(&section);
        Body::Validate(tc)
    } else if ty == sht12::EVAL_NODE_EXPR {
        Body::NodeExpr(parse_node_expr_case(g, entry, manifest_path, &id))
    } else if ty == sht12::INFER {
        Body::Infer(parse_infer_case(g, entry, &id))
    } else if let Some(kind) = SrlKind::from_iri(ty) {
        Body::Srl(parse_srl_case(g, entry, kind, &id))
    } else {
        panic!("{id}: unknown SHACL 1.2 test type <{ty}> — grade it, do not skip it");
    };
    Case12 {
        id,
        section,
        body,
        listed: true,
    }
}

/// The single object of `(subject, predicate, ?)`, panicking on none or several.
fn exactly_one(g: &RdfDataset, subject: &Term, predicate: &str, id: &str) -> Term {
    let values = objects(g, subject, predicate);
    let [value] = values.as_slice() else {
        panic!(
            "{id}: expected exactly one <{predicate}> on {subject}, got {}",
            values.len()
        );
    };
    value.clone()
}

/// An IRI object read as a vendored file path that must exist.
fn existing_file(term: &Term, id: &str, role: &str) -> PathBuf {
    let Term::NamedNode(n) = term else {
        panic!("{id}: {role} must be a file IRI, got {term}");
    };
    let path = iri_to_path(n.as_str());
    assert!(
        path.is_file(),
        "{id}: {role} names {}, which is not a vendored file",
        path.display()
    );
    path
}

fn parse_node_expr_case(
    g: &Arc<RdfDataset>,
    entry: &Term,
    manifest_path: &Path,
    id: &str,
) -> NodeExprCase {
    let action = exactly_one(g, entry, mf::ACTION, id);
    let mut focus: Option<Term> = None;
    let mut expr: Option<Term> = None;
    let mut ignore_order = false;
    let mut scope: Vec<(String, Term)> = Vec::new();
    for (_, predicate, value) in native_quads(g, Some(&action), None, None, GraphFilter::AnyGraph) {
        let predicate = predicate.as_str();
        if predicate == sht12::NODE_EXPR {
            assert!(expr.replace(value).is_none(), "{id}: two sht:nodeExpr");
        } else if predicate == sht12::FOCUS_NODE {
            assert!(focus.replace(value).is_none(), "{id}: two sht:focusNode");
        } else if predicate == sht12::IGNORE_ORDER {
            ignore_order = match &value {
                Term::Literal(l) if l.datatype_str() == XSD_BOOLEAN => {
                    match purrdf_xsd::parse(l.value(), purrdf_xsd::XsdDatatype::Boolean) {
                        Ok(purrdf_xsd::XsdValue::Boolean(flag)) => flag,
                        other => panic!("{id}: sht:ignoreOrder is not an xsd:boolean: {other:?}"),
                    }
                }
                other => panic!("{id}: sht:ignoreOrder must be an xsd:boolean, got {other}"),
            };
        } else if let Some(name) = predicate.strip_prefix(sht12::SCOPE_PREFIX) {
            assert!(
                !scope.iter().any(|(bound, _)| bound == name),
                "{id}: scope variable {name} bound twice"
            );
            scope.push((name.to_owned(), value));
        } else {
            panic!("{id}: unknown sht:EvalNodeExpr action property <{predicate}>");
        }
    }
    scope.sort_by(|a, b| a.0.cmp(&b.0));
    let expr = expr.unwrap_or_else(|| panic!("{id}: no sht:nodeExpr"));

    let result = exactly_one(g, entry, mf::RESULT, id);
    let is_list = matches!(&result, Term::BlankNode(_))
        || matches!(&result, Term::NamedNode(n) if n.as_str() == RDF_NIL);
    assert!(is_list, "{id}: mf:result must be an RDF list, got {result}");
    let expected = list_items(g, &result);

    NodeExprCase {
        file: manifest_path.to_path_buf(),
        dataset: Arc::clone(g),
        expr,
        focus,
        scope,
        expected,
        ignore_order,
    }
}

fn parse_infer_case(g: &RdfDataset, entry: &Term, id: &str) -> InferCase {
    let action = exactly_one(g, entry, mf::ACTION, id);
    for (_, predicate, _) in native_quads(g, Some(&action), None, None, GraphFilter::AnyGraph) {
        let predicate = predicate.as_str();
        assert!(
            predicate == sht::DATA_GRAPH || predicate == sht::SHAPES_GRAPH,
            "{id}: unknown sht:Infer action property <{predicate}>"
        );
    }
    let shapes_term = exactly_one(g, &action, sht::SHAPES_GRAPH, id);
    let shapes_path = existing_file(&shapes_term, id, "sht:shapesGraph");
    let data_path = existing_file(
        &exactly_one(g, &action, sht::DATA_GRAPH, id),
        id,
        "sht:dataGraph",
    );
    let shapes_graph_iri = match &shapes_term {
        Term::NamedNode(n) => n.as_str().to_owned(),
        _ => unreachable!("existing_file accepted only an IRI"),
    };

    let result = exactly_one(g, entry, mf::RESULT, id);
    let expected = match &result {
        Term::NamedNode(n) if n.as_str() == sht::FAILURE => InferExpected::Failure,
        Term::NamedNode(n) if n.as_str() == RDF_NIL => InferExpected::Triples(Vec::new()),
        Term::NamedNode(_) => InferExpected::File(existing_file(&result, id, "mf:result")),
        Term::BlankNode(_) => InferExpected::Triples(
            list_items(g, &result)
                .iter()
                .map(|row| {
                    let terms = list_items(g, row);
                    let Ok(triple) = <[Term; 3]>::try_from(terms) else {
                        panic!("{id}: an expected triple must be a three-member list");
                    };
                    triple
                })
                .collect(),
        ),
        other => panic!("{id}: unrecognised sht:Infer mf:result {other}"),
    };
    InferCase {
        shapes_path,
        data_path,
        shapes_graph_iri,
        expected,
    }
}

fn parse_srl_case(g: &RdfDataset, entry: &Term, kind: SrlKind, id: &str) -> SrlCase {
    let action = exactly_one(g, entry, mf::ACTION, id);
    if kind == SrlKind::Eval {
        let ruleset = existing_file(
            &exactly_one(g, &action, srlt::RULESET, id),
            id,
            "srlt:ruleset",
        );
        let data = existing_file(&exactly_one(g, &action, srlt::DATA, id), id, "srlt:data");
        let result = existing_file(&exactly_one(g, entry, mf::RESULT, id), id, "mf:result");
        SrlCase {
            kind,
            ruleset,
            data: Some(data),
            result: Some(result),
        }
    } else {
        assert!(
            object(g, entry, mf::RESULT).is_none(),
            "{id}: a syntax/well-formedness/stratification test carries no mf:result"
        );
        SrlCase {
            kind,
            ruleset: existing_file(&action, id, "mf:action"),
            data: None,
            result: None,
        }
    }
}

// ── Reachability ──────────────────────────────────────────────────────────────

/// Every manifest reachable from `manifest` by `mf:include`, with its dataset.
fn reachable_manifests(manifest: &Path, out: &mut BTreeMap<PathBuf, Arc<RdfDataset>>) {
    if out.contains_key(manifest) {
        return;
    }
    let g = parse_turtle_file(manifest).unwrap_or_else(|e| panic!("manifest walk failed: {e}"));
    let includes = manifest_includes(&g, manifest);
    out.insert(manifest.to_path_buf(), g);
    for include in includes {
        reachable_manifests(&include, out);
    }
}

/// Prove the corpus on disk and the corpus discovered are the same corpus: every
/// `.ttl` listing `mf:entries` is a reached manifest, and every node a reached
/// manifest types as a SHACL 1.2 / SPARQL 1.2 RL test is a discovered entry.
fn every_manifest_and_every_test_is_reached(root: &Path, discovered_ids: &BTreeSet<&str>) {
    let mut reached: BTreeMap<PathBuf, Arc<RdfDataset>> = BTreeMap::new();
    for (manifest, _) in manifest_roots(root) {
        reachable_manifests(&manifest, &mut reached);
    }

    let mut on_disk: Vec<PathBuf> = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).expect("suite directory must be readable") {
            let path = entry.expect("suite entry must be readable").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "ttl")
                && fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()))
                    .contains("mf:entries")
            {
                on_disk.push(path);
            }
        }
    }
    let unreached: Vec<String> = on_disk
        .iter()
        .filter(|p| !reached.contains_key(*p))
        .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string())
        .collect();
    let expected_unreached: Vec<String> = W3C12_UNREACHED_MANIFESTS
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_eq!(
        unreached, expected_unreached,
        "manifests on disk that no manifest root reaches run nowhere, and no gate would \
         notice"
    );

    let mut unlisted: Vec<String> = Vec::new();
    for (manifest, g) in &reached {
        for (subject, _, ty) in native_quads(
            g,
            None,
            Some(&super::named(rdf::TYPE)),
            None,
            GraphFilter::AnyGraph,
        ) {
            let Term::NamedNode(ty) = ty else { continue };
            let ty = ty.as_str();
            if !(ty.starts_with(SHT) || ty.starts_with(SRLT)) {
                continue;
            }
            let Term::NamedNode(subject) = subject else {
                panic!(
                    "{}: a node typed <{ty}> is not an IRI: {subject}",
                    manifest.display()
                );
            };
            let id = entry_id(subject.as_str(), manifest, root);
            if !discovered_ids.contains(id.as_str()) {
                unlisted.push(id);
            }
        }
    }
    unlisted.sort();
    let mut expected_unlisted: Vec<String> = W3C12_UNLISTED_TYPED_NODES
        .iter()
        .map(|(id, _)| (*id).to_owned())
        .collect();
    expected_unlisted.sort();
    assert_eq!(
        unlisted, expected_unlisted,
        "nodes typed as tests but listed in no mf:entries run nowhere; only the pinned, \
         evidenced W3C12_UNLISTED_TYPED_NODES may"
    );
    for (id, _) in W3C12_UNLISTED_TYPED_NODES {
        let dir = root.join(id.rsplit_once('/').map_or("", |(d, _)| d));
        let local = id.rsplit('/').next().unwrap_or(id);
        assert!(
            !dir.join(format!("{local}.srl")).exists(),
            "{id}: pinned as not runnable, but its rule set now exists"
        );
    }
}
