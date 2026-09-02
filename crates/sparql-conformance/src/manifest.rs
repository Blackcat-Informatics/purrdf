// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C `mf:` test-manifest parsing.
//!
//! A manifest `manifest.ttl` is itself an RDF graph: it is loaded with the native
//! Turtle codec and **queried with the native engine** (dog-fooding) to extract
//! its `mf:entries` list of test cases. File references in the manifest are
//! relative IRIs; they are parsed against a sentinel base and mapped back to
//! local paths under the manifest's directory.
//!
//! The DAWG manifest vocabulary also defines `mf:include`, an RDF collection of
//! further manifests an *aggregator* manifest pulls in. [`load`] follows it: see
//! [`load`]'s own documentation for the cycle, depth, fan-out and
//! aggregator-discovery rules, all of which are hard errors rather than
//! truncations.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use purrdf_core::{SparqlEngine, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::NativeSparqlEngine;

use crate::paths;

/// The ROOT of the sentinel base IRI space every manifest is parsed against.
///
/// Each manifest gets its OWN base below this root — see [`BaseResolver`] — so a
/// relative file reference `<agg01.rq>` resolves to `<base>agg01.rq` and the file
/// it names is recoverable from the IRI alone.
///
/// The root itself denotes the Cargo workspace root, so the sentinel IRI space
/// mirrors the workspace tree exactly. It is never a manifest's own base (except
/// for the degenerate case of a manifest sitting at the workspace root, which no
/// corpus does): using ONE constant base for every manifest is what made two
/// manifests that each declare the relative `@prefix : <manifest#>` mint
/// byte-identical case IRIs, and the [`crate::xfail`] ledger — which is global —
/// could then not tell the two apart.
pub(crate) const BASE_ROOT: &str = "http://purrdf.test/manifest/";

const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";

/// The SPARQL-1.1 update-test vocabulary (`ut:`). Update tests describe their
/// pre-state (`ut:data`/`ut:graphData`), the update request (`ut:request`), and
/// their expected post-state (an `mf:result` node carrying `ut:data`/
/// `ut:graphData`). A named graph is a blank node `[ ut:graph <file> ;
/// rdfs:label "graph-iri" ]` — the graph IRI is the `rdfs:label`, not the file.
const UT: &str = "http://www.w3.org/2009/sparql/tests/test-update#";

/// The RDF Schema namespace; `rdfs:label` carries the graph IRI of a
/// `ut:graphData` entry in an update test.
const RDFS_LABEL_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";

/// The SPARQL service-description namespace; `sd:entailmentRegime` on a query
/// test's action lists the entailment regimes under which its expected result
/// holds (an RDF list of `http://www.w3.org/ns/entailment/*` IRIs).
const SD_NS: &str = "http://www.w3.org/ns/sparql-service-description#";

/// This harness's own manifest-EXTENSION vocabulary, for fields the W3C `mf:`/`qt:`
/// vocabulary has no slot for. Test-INFRASTRUCTURE metadata that configures this
/// harness's own loader, under `example.org` exactly as `EXT_NS`/`REL_NS`/`LOSS_NS`
/// in `crate::run` are: caller-supplied harness configuration, never a vocabulary
/// PurRDF itself mints or ships.
const MF_EXT_NS: &str = "https://example.org/conformance-manifest#";

/// The kind of a discovered SPARQL test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKind {
    /// `mf:QueryEvaluationTest` (and result-format variants): run the query and
    /// diff the result.
    QueryEval,
    /// `mf:UpdateEvaluationTest`: apply the `ut:request` update to the pre-state
    /// dataset and diff the resulting dataset against the expected post-state.
    UpdateEval,
    /// `mf:PositiveSyntaxTest(11)`: the query must parse.
    PositiveSyntax,
    /// `mf:NegativeSyntaxTest(11)`: the query must fail to parse.
    NegativeSyntax,
    /// `mf:PositiveUpdateSyntaxTest`: the UPDATE request must parse.
    PositiveUpdateSyntax,
    /// `mf:NegativeUpdateSyntaxTest`: the UPDATE request must fail to parse.
    NegativeUpdateSyntax,
    /// A manifest entry whose `rdf:type` the harness does not model — recorded and
    /// surfaced (never silently skipped).
    Unknown,
}

/// What a [`TestKind::QueryEval`] case expects.
#[derive(Debug, Clone)]
pub enum ExpectedResult {
    /// SPARQL Results XML.
    Srx(PathBuf),
    /// SPARQL Results JSON.
    Srj(PathBuf),
    /// A graph (`CONSTRUCT`/`DESCRIBE`) — compared as canonical N-Quads.
    Graph(PathBuf),
    /// A Turtle-encoded `rs:ResultSet` description of a SELECT solution sequence
    /// (`rs:resultVariable`/`rs:solution`/`rs:binding`/`rs:variable`/`rs:value`) —
    /// compared as a solution multiset, not a graph.
    ResultSetTurtle(PathBuf),
    /// An UPDATE post-state: the expected default-graph data (`ut:data`) and
    /// named graphs (`ut:graphData`), compared to the mutated dataset as
    /// canonical N-Quads. Empty vectors denote an empty expected dataset.
    DatasetState {
        /// Expected default-graph Turtle files.
        data: Vec<PathBuf>,
        /// Expected named graphs as `(graph IRI, file)`.
        graph_data: Vec<(String, PathBuf)>,
    },
    /// A hard **evaluation failure**: the query must parse and then refuse to run,
    /// and the refusal must name the reason the file records.
    ///
    /// The W3C `mf:` vocabulary models a negative *syntax* test but no negative
    /// evaluation test, and this harness mints no vocabulary of its own. So the
    /// expectation rides the channel the harness already routes on — the
    /// `mf:result` file's extension (see `classify_result`) — exactly as `.srx`
    /// selects SPARQL Results XML and `.ttl` a graph. A `.err` file holds the text
    /// the diagnostic must contain, one expectation per line, all of which must
    /// appear; a case whose query SUCCEEDS fails, so the expectation can never be
    /// satisfied vacuously.
    ///
    /// First-party only: no vendored manifest carries a `.err` result.
    EvalError(PathBuf),
    /// Syntax tests carry no result.
    None,
    /// A result file whose extension the harness does not model.
    Unsupported(PathBuf),
}

/// A `qt:constructDataFile` action: the case's data is not a file on disk but the
/// **serialization of a CONSTRUCT query's result graph**.
///
/// ```turtle
/// [ qt:constructDataFile [ qt:query <c.rq> ; qt:format "text/turtle" ] ;
///   qt:query <ask.rq> ]
/// ```
///
/// The harness runs `query` against an empty dataset, writes the resulting graph
/// in `format`, and reads that serialization back as one of the case's source
/// documents — so the case grades a **round trip through a concrete syntax**, not
/// just the evaluator. The SEP-0009 `bnodes-export-*` cases use it to pin that a
/// blank node occurring both as a term and inside a `cdt:List` / `cdt:Map`
/// lexical form is written with ONE identifier in Turtle, N-Triples and RDF/XML
/// alike, so it still denotes one node after the round trip. A serializer that
/// spelled the two occurrences differently would fail them, and that failure is
/// the point of the action shape.
#[derive(Debug, Clone)]
pub struct ConstructDataFile {
    /// The CONSTRUCT query whose result graph becomes the case's data.
    pub query: PathBuf,
    /// The media type that graph is serialized to (and re-parsed from).
    pub format: String,
}

/// One discovered conformance test case.
#[derive(Debug, Clone)]
pub struct SparqlTestCase {
    /// The full test IRI (used for diagnostics and xfail matching).
    pub iri: String,
    /// The sentinel base IRI of the manifest that declared this case (see
    /// [`BaseResolver`]), ending in `/`.
    ///
    /// Carried on the case because the base is a PER-MANIFEST fact and every stage
    /// that resolves a relative IRI for this case must use the SAME one. A query
    /// like `GRAPH <exists02.ttl> { … }` names its graph relatively, and that
    /// reference has to land on the identical IRI the manifest's
    /// `qt:graphData <exists02.ttl>` produced — if the loader and the evaluator
    /// resolved against different bases the graph would simply not be found, and
    /// the case would fail with an empty result rather than a diagnosable error.
    pub base: String,
    /// The human-readable `mf:name`.
    pub name: String,
    /// The test kind.
    pub kind: TestKind,
    /// The query file (`.rq`).
    pub query: PathBuf,
    /// The default-graph data file(s) (`qt:data`).
    pub data: Vec<PathBuf>,
    /// Named-graph data files (`qt:graphData`); the graph IRI is the file IRI.
    pub graph_data: Vec<(String, PathBuf)>,
    /// `SERVICE` endpoint data: `(endpoint IRI, local file)` (`qt:serviceData`).
    pub service_data: Vec<(String, PathBuf)>,
    /// A `qt:constructDataFile` action, when the case declares one: its data is
    /// produced by running a CONSTRUCT query and serializing the result graph
    /// (see [`ConstructDataFile`]). Composes with `qt:data`/`qt:graphData` —
    /// the constructed document is merged in as one more source, standardized
    /// apart from the files exactly as they are from each other.
    pub construct_data: Option<ConstructDataFile>,
    /// The best-supported entailment regime for this case (`sd:entailmentRegime`),
    /// if any: the dataset is materialized under it before the query runs. `None`
    /// for a plain (Simple-entailment) test or one whose only regimes the native
    /// reasoner cannot materialize (OWL-Direct / D / RIF).
    pub regime: Option<purrdf_entail::Regime>,
    /// `purrdf:aggregateNamespace` on the test's `mf:action` (`MF_EXT_NS`,
    /// harness-loader vocabulary — see its doc comment): the IRI namespace this
    /// case's `AGG(<{NAMESPACE}NAME>, args…)` calls resolve against. `None` (the
    /// default, and every case but the ones that opt in) registers no statistical
    /// aggregate registry for this case, exactly as before this field existed —
    /// PurRDF mints no default namespace.
    pub aggregate_namespace: Option<String>,
    /// The expected result.
    pub expected: ExpectedResult,
}

/// The greatest `mf:include` nesting depth [`load`] will follow.
///
/// Justified by what aggregation the published corpora actually use: the DAWG
/// SPARQL manifests and the SEP-0009 CDT corpus both nest exactly ONE level (a
/// root aggregator over per-group manifests), and the deepest plausible shape —
/// spec-version → chapter → section → group → sub-group — is five. Eight leaves
/// that ample headroom while still bounding a pathological chain that the cycle
/// check cannot catch, because a long non-repeating chain of manifests is not a
/// cycle. Exceeding it is a hard error naming the bound and the chain, never a
/// truncation: a silently-truncated include tree runs fewer cases and reports
/// success, which is the exact failure this whole loader exists to refuse.
const MAX_INCLUDE_DEPTH: usize = 8;

/// The greatest number of manifests one `load` closure will visit.
///
/// The cycle check refuses a manifest that includes itself and the duplicate
/// check refuses one reached twice, so the remaining unbounded shape is FAN-OUT:
/// a tree of distinct manifests wide enough to exhaust memory or time. The whole
/// workspace carries well under a hundred manifests across every corpus, so 512
/// is roughly a five-fold headroom over the largest thing this repository could
/// legitimately grow into, and still small enough that hitting it means the
/// include graph is wrong rather than large.
const MAX_MANIFESTS_PER_CLOSURE: usize = 512;

/// Load and parse every case declared by `manifest_path` **and by the transitive
/// closure of its `mf:include` collection**.
///
/// # Manifest roles: aggregator versus group
///
/// The DAWG vocabulary lets one manifest declare `mf:entries` (a *group* — the
/// test cases themselves), `mf:include` (an *aggregator* — a collection of
/// further manifests), or both. This loader accepts all three shapes, with one
/// rule that keeps discovery and aggregation from colliding:
///
/// > **A manifest whose file name is `manifest.ttl` may not declare `mf:include`.**
///
/// The datatest harness in `tests/sparql_conformance.rs` discovers cases with the
/// glob `.*/manifest\.ttl$`. If an aggregator were itself named `manifest.ttl`
/// while its children were too, the glob would discover BOTH, and every child's
/// cases would run twice — once directly and once through the aggregator —
/// silently doubling the pass tally. Naming aggregators something the leaf glob
/// does not match (`manifest-all.ttl`, as the SEP-0009 corpus does) makes the two
/// roles disjoint by construction: the glob discovers group manifests only, and
/// an aggregator is only ever loaded because a `[[test]]` target names it. This
/// used to hold by accident of one file's name; it is now enforced, so a future
/// corpus cannot reintroduce the double count.
///
/// # Hard failures (never silent truncation)
///
/// * A manifest declaring NEITHER `mf:entries` NOR `mf:include` — it advertises
///   nothing and would load zero cases while reporting success.
/// * A manifest declaring an EMPTY `mf:entries ()` or `mf:include ()` collection.
///   These are diagnosed separately from the case above because an empty
///   `rdf:List` is `rdf:nil`, which has no `rdf:first`, so the collection walks
///   cannot tell "empty" from "absent" — and they are different mistakes.
/// * A closure whose transitive case count is ZERO. Given the three checks above
///   this is unreachable — every leaf either declares a non-empty `mf:entries`
///   (and the declared-vs-loaded check then forces at least one case) or is
///   refused — so it is a belt-and-braces assertion in the same spirit as the
///   executed-count check inside `load_one`: it survives a future refactor that
///   changes how a manifest can contribute cases.
/// * An include cycle, direct or transitive — the error names the whole chain.
/// * The same manifest reached twice in one closure (a diamond, not a cycle) —
///   its cases would be counted twice.
/// * Nesting deeper than [`MAX_INCLUDE_DEPTH`] or wider than
///   [`MAX_MANIFESTS_PER_CLOSURE`].
/// * Two manifests in one closure minting the same test-case IRI — the ledger in
///   [`crate::xfail`] could not tell them apart.
/// * Every per-manifest guarantee below (the declared-vs-loaded completeness
///   check) applied to each included manifest exactly as to the root, because
///   each is loaded through this same function.
///
/// # Errors
///
/// Returns a message on a read/parse failure, a malformed manifest, or any of the
/// hard failures above.
pub fn load(manifest_path: &Path) -> Result<Vec<SparqlTestCase>, String> {
    let mut walk = IncludeWalk::default();
    let cases = walk.load(manifest_path, 0)?;
    if cases.is_empty() {
        return Err(format!(
            "{}: the transitive mf:include/mf:entries closure yields ZERO test cases. A \
             manifest that declares tests and runs none reports success while measuring \
             nothing, so it is refused rather than counted green",
            manifest_path.display()
        ));
    }
    Ok(cases)
}

/// The state one [`load`] closure carries while following `mf:include`.
#[derive(Default)]
struct IncludeWalk {
    /// Canonical paths of the manifests currently being loaded, outermost first.
    /// A child already on this stack is a CYCLE, and the stack IS the chain the
    /// error names.
    stack: Vec<PathBuf>,
    /// Canonical paths of every manifest this closure has finished (or started).
    /// A repeat that is not on `stack` is a diamond: reached twice by two
    /// different parents, which would count its cases twice.
    visited: Vec<PathBuf>,
    /// Every case IRI minted so far, mapped to the manifest that minted it, so a
    /// cross-manifest collision names both sides.
    minted: BTreeMap<String, PathBuf>,
}

impl IncludeWalk {
    /// Load `manifest_path` and everything it includes, at nesting `depth`.
    fn load(&mut self, manifest_path: &Path, depth: usize) -> Result<Vec<SparqlTestCase>, String> {
        let canonical = manifest_path
            .canonicalize()
            .map_err(|e| format!("resolve manifest path {}: {e}", manifest_path.display()))?;

        if let Some(at) = self.stack.iter().position(|p| *p == canonical) {
            let mut chain: Vec<String> = self.stack[at..]
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            chain.push(canonical.display().to_string());
            return Err(format!(
                "mf:include cycle: {}. A manifest that includes itself, directly or \
                 transitively, has no finite closure; the cycle is refused rather than \
                 followed or quietly cut short",
                chain.join(" -> ")
            ));
        }
        if self.visited.contains(&canonical) {
            return Err(format!(
                "{} is reached twice in one mf:include closure (by two different parents, \
                 not a cycle). Loading it twice would count every one of its cases twice, \
                 inflating the pass tally; declare it in exactly one aggregator",
                canonical.display()
            ));
        }
        if depth > MAX_INCLUDE_DEPTH {
            return Err(format!(
                "{}: mf:include nesting exceeds the {MAX_INCLUDE_DEPTH}-level bound (chain: \
                 {}). The bound is refused, not truncated",
                canonical.display(),
                self.stack
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ));
        }
        if self.visited.len() >= MAX_MANIFESTS_PER_CLOSURE {
            return Err(format!(
                "{}: this mf:include closure would visit more than \
                 {MAX_MANIFESTS_PER_CLOSURE} manifests. The bound is refused, not truncated",
                canonical.display()
            ));
        }

        self.visited.push(canonical.clone());
        self.stack.push(canonical.clone());
        let result = self.load_within(&canonical, depth);
        self.stack.pop();
        result
    }

    /// The body of [`Self::load`], run with `canonical` already on the stack so an
    /// early return still unwinds it.
    fn load_within(
        &mut self,
        canonical: &Path,
        depth: usize,
    ) -> Result<Vec<SparqlTestCase>, String> {
        let Loaded {
            mut cases,
            includes,
        } = load_one(canonical)?;
        for case in &cases {
            if let Some(previous) = self.minted.get(&case.iri) {
                return Err(format!(
                    "case IRI {} is minted by BOTH {} and {}. The expected-failure ledger in \
                     crates/sparql-conformance/src/xfail.rs matches on the case IRI and is \
                     global, so one entry would silently govern two different tests",
                    case.iri,
                    previous.display(),
                    canonical.display()
                ));
            }
            self.minted
                .insert(case.iri.clone(), canonical.to_path_buf());
        }
        for child in includes {
            cases.extend(self.load(&child, depth + 1)?);
        }
        Ok(cases)
    }
}

/// What one manifest FILE declares: its own cases plus the manifests it includes.
struct Loaded {
    /// The cases from this manifest's own `mf:entries`.
    cases: Vec<SparqlTestCase>,
    /// Local paths of the manifests in this manifest's `mf:include` collection,
    /// sorted so the closure is deterministic (a SPARQL solution sequence has no
    /// guaranteed order, and case identity does not depend on load order).
    includes: Vec<PathBuf>,
}

/// Load exactly ONE manifest file — no `mf:include` recursion.
fn load_one(manifest_path: &Path) -> Result<Loaded, String> {
    let resolver = BaseResolver::new(manifest_path)?;
    let bytes = std::fs::read(manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let dataset = purrdf::parse_dataset(&bytes, "text/turtle", Some(&resolver.base))
        .map_err(|e| format!("parse manifest {}: {e}", manifest_path.display()))?;

    // One row per (test × data × graphData × serviceData × result) combination;
    // grouped by ?test below. Property paths walk the rdf:List of entries.
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         PREFIX qt: <http://www.w3.org/2001/sw/DataAccess/tests/test-query#>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         PREFIX purrdf: <{MF_EXT_NS}>\n\
         SELECT ?test ?type ?name ?act ?query ?data ?graphData ?serviceEp ?serviceData ?result ?aggNs ?cdfQuery ?cdfFormat WHERE {{\n\
         ?mani mf:entries/rdf:rest*/rdf:first ?test .\n\
         ?test rdf:type ?type ; mf:name ?name ; mf:action ?act .\n\
         OPTIONAL {{ ?act qt:query ?query }}\n\
         OPTIONAL {{ ?act qt:data ?data }}\n\
         OPTIONAL {{ ?act qt:graphData ?graphData }}\n\
         OPTIONAL {{ ?act qt:serviceData ?sd . ?sd qt:endpoint ?serviceEp ; qt:data ?serviceData }}\n\
         OPTIONAL {{ ?act qt:constructDataFile ?cdf . ?cdf qt:query ?cdfQuery ; qt:format ?cdfFormat }}\n\
         OPTIONAL {{ ?act purrdf:aggregateNamespace ?aggNs }}\n\
         OPTIONAL {{ ?test mf:result ?result }}\n\
         }}"
    );

    let rows = query_rows(&dataset, &query)?;

    // Group rows by ?test IRI, accumulating the multi-valued columns.
    let mut by_test: BTreeMap<String, SparqlTestCase> = BTreeMap::new();
    for row in &rows {
        let test_iri = iri_of(row, "test").ok_or("manifest row without ?test IRI")?;
        let kind = classify(row.get("type"));
        let entry = by_test
            .entry(test_iri.clone())
            .or_insert_with(|| SparqlTestCase {
                iri: test_iri.clone(),
                base: resolver.base.clone(),
                name: lexical_of(row, "name").unwrap_or_default(),
                kind,
                query: PathBuf::new(),
                data: Vec::new(),
                graph_data: Vec::new(),
                service_data: Vec::new(),
                construct_data: None,
                regime: None,
                aggregate_namespace: None,
                expected: ExpectedResult::None,
            });
        // A test may carry several rdf:type values; prefer a recognized kind.
        if entry.kind == TestKind::Unknown && kind != TestKind::Unknown {
            entry.kind = kind;
        }

        // The query file: qt:query for eval tests, else mf:action itself (syntax).
        if let Some(q) = iri_of(row, "query") {
            entry.query = resolver.path(&q)?;
        } else if entry.query.as_os_str().is_empty()
            && let Some(act) = iri_of(row, "act")
        {
            entry.query = resolver.path(&act)?;
        }
        push_unique_path(
            &mut entry.data,
            resolve_opt(&resolver, iri_of(row, "data"))?,
        );
        if let Some(gd) = iri_of(row, "graphData") {
            let path = resolver.path(&gd)?;
            if !entry.graph_data.iter().any(|(_, p)| *p == path) {
                entry.graph_data.push((gd, path));
            }
        }
        if let (Some(ep), Some(sd)) = (iri_of(row, "serviceEp"), iri_of(row, "serviceData")) {
            let path = resolver.path(&sd)?;
            if !entry.service_data.iter().any(|(e, _)| *e == ep) {
                entry.service_data.push((ep, path));
            }
        }
        // `qt:constructDataFile` needs BOTH halves — the CONSTRUCT query and the
        // media type its graph is written in. Taking one without the other would
        // leave the harness guessing a serialization the manifest actually
        // states, so the pair binds together or not at all.
        if entry.construct_data.is_none()
            && let (Some(query), Some(format)) =
                (iri_of(row, "cdfQuery"), lexical_of(row, "cdfFormat"))
        {
            entry.construct_data = Some(ConstructDataFile {
                query: resolver.path(&query)?,
                format,
            });
        }
        if let Some(result) = iri_of(row, "result") {
            entry.expected = classify_result(&resolver.path(&result)?);
        }
        if entry.aggregate_namespace.is_none()
            && let Some(ns) = lexical_of(row, "aggNs")
        {
            entry.aggregate_namespace = Some(ns);
        }
    }

    // Completeness check: the row-grouping SELECT above requires ?type, ?name, AND
    // ?act to all bind (none are OPTIONAL), so an `mf:entries` member missing any one
    // of `rdf:type`/`mf:name`/`mf:action` produces NO row at all and would otherwise
    // vanish from `by_test` with no trace — a silent skip, not a modeled failure. List
    // every `mf:entries` member directly (no mandatory triple beyond list membership)
    // and fail loudly on any member that did not turn into a loaded case, naming it,
    // rather than letting the manifest quietly advertise fewer cases than it declares.
    //
    // The declared set is a SET, not a slot count: an `rdf:List` may name the same
    // test IRI in two cells (the vendored SEP-0009 `orderby` manifest lists
    // `:order-map-03` twice), and a repeated member denotes the SAME test. Running
    // it once is the only sound reading — running it per slot would inflate the
    // pass tally by re-counting one test — so the repeat collapses here and in
    // `by_test` alike, and the two counts still agree.
    let declared_list = list_entry_iris(&dataset)?;
    let declared: std::collections::BTreeSet<String> = declared_list.into_iter().collect();
    let missing: Vec<&String> = declared
        .iter()
        .filter(|t| !by_test.contains_key(*t))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "{}: {} of {} declared mf:entries member(s) produced no loaded test case \
             (missing rdf:type, mf:name, and/or mf:action — a silent-skip hole, not a \
             modeled result): {}",
            manifest_path.display(),
            missing.len(),
            declared.len(),
            missing
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut cases: Vec<SparqlTestCase> = by_test.into_values().collect();
    // Update tests carry their pre-state, request, and post-state under the `ut:`
    // vocabulary, which the query SELECT above does not read. Fill those fields in
    // with a dedicated pass so the `ut:` shape (nested graphData blank nodes) is
    // read explicitly rather than shoe-horned into the query SELECT.
    if cases.iter().any(|c| c.kind == TestKind::UpdateEval) {
        load_update_details(&dataset, &resolver, &mut cases)?;
    }
    // Entailment tests declare an `sd:entailmentRegime` list; select the regime
    // the native reasoner should materialize before the query runs.
    load_entailment_regimes(&dataset, &mut cases)?;

    // Belt-and-braces: the count that will actually be EXECUTED (`cases.len()`) must
    // equal what the manifest declares. This is the same fact the `missing` check
    // above already establishes (each declared IRI maps 1:1 into `by_test`, which
    // `cases` is built from), stated as an executed-count assertion so a future
    // refactor of the loader that changes the loading strategy — not just this
    // query's OPTIONAL/mandatory shape — still cannot silently drop a case without
    // this function failing.
    debug_assert_eq!(
        cases.len(),
        declared.len(),
        "loaded case count must equal declared mf:entries count"
    );
    if cases.len() != declared.len() {
        return Err(format!(
            "{}: loaded {} test case(s) but the manifest declares {} mf:entries member(s)",
            manifest_path.display(),
            cases.len(),
            declared.len()
        ));
    }

    let includes = load_includes(&dataset, manifest_path, &resolver)?;

    // An EMPTY `rdf:List` is `rdf:nil`, which carries no `rdf:first`, so the
    // `rdf:rest*/rdf:first` walk above cannot tell `mf:entries ()` from no
    // `mf:entries` at all. Both are refused, but with different diagnoses, because
    // they are different authoring mistakes: one manifest forgot to declare its
    // group, the other declared an empty one.
    let has_entries = declares_property(&dataset, "entries")?;
    if has_entries && declared.is_empty() {
        return Err(format!(
            "{}: declares an EMPTY mf:entries list. A group that names no test measures \
             nothing while still presenting as a loaded manifest; if the group is gone, \
             remove the manifest rather than emptying it",
            manifest_path.display()
        ));
    }
    if !has_entries && includes.is_empty() {
        return Err(format!(
            "{}: declares NEITHER mf:entries NOR mf:include, so it advertises a manifest and \
             contributes no test case. Such a manifest loads clean and reports success while \
             measuring nothing; it is refused rather than counted green",
            manifest_path.display()
        ));
    }

    Ok(Loaded { cases, includes })
}

/// Whether any subject in the manifest carries the `mf:` property `local` at all.
///
/// Needed because the `rdf:rest*/rdf:first` collection walks cannot distinguish an
/// EMPTY collection (`()` is `rdf:nil`, which has no `rdf:first`) from an absent
/// property — and those two must be diagnosed differently.
fn declares_property(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    local: &str,
) -> Result<bool, String> {
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         SELECT ?list WHERE {{ ?mani mf:{local} ?list }}"
    );
    Ok(!query_rows(dataset, &query)?.is_empty())
}

/// Resolve this manifest's `mf:include` collection to local manifest paths.
///
/// Also enforces the aggregator-naming rule documented on [`load`]: a file named
/// `manifest.ttl` is what the datatest root glob discovers, so it may not itself
/// aggregate — otherwise its children (also `manifest.ttl`) would be run twice.
fn load_includes(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    manifest_path: &Path,
    resolver: &BaseResolver,
) -> Result<Vec<PathBuf>, String> {
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         SELECT ?inc WHERE {{\n\
         ?mani mf:include/rdf:rest*/rdf:first ?inc .\n\
         }}"
    );
    let rows = query_rows(dataset, &query)?;
    if rows.is_empty() {
        // Same `rdf:nil` blind spot as `mf:entries` (see `declares_property`): an
        // empty `mf:include ()` walks to nothing and must not be read as "this is
        // not an aggregator".
        if declares_property(dataset, "include")? {
            return Err(format!(
                "{}: declares an EMPTY mf:include list. An aggregator that aggregates nothing \
                 contributes no case while still presenting as a loaded manifest",
                manifest_path.display()
            ));
        }
        return Ok(Vec::new());
    }

    if manifest_path.file_name().and_then(|n| n.to_str()) == Some("manifest.ttl") {
        return Err(format!(
            "{}: a manifest named 'manifest.ttl' may not declare mf:include. The datatest root \
             glob in crates/sparql-conformance/tests/sparql_conformance.rs discovers every \
             '*/manifest.ttl', so an aggregator with that name would be discovered ALONGSIDE the \
             'manifest.ttl' files it includes and every one of their cases would run twice, \
             silently doubling the pass tally. Name an aggregator something the leaf glob does \
             not match (the vendored SEP-0009 corpus uses 'manifest-all.ttl')",
            manifest_path.display()
        ));
    }

    let mut includes: Vec<PathBuf> = Vec::with_capacity(rows.len());
    for row in &rows {
        let iri = iri_of(row, "inc").ok_or_else(|| {
            format!(
                "{}: an mf:include member is not an IRI; an included manifest must be named by \
                 the IRI of its file",
                manifest_path.display()
            )
        })?;
        includes.push(resolver.path(&iri)?);
    }
    // A SPARQL solution sequence has no guaranteed order and this SELECT is not
    // DISTINCT, so sort for determinism. A genuine repeat is NOT collapsed: unlike a
    // repeated `mf:entries` member (which denotes one test), a repeated include
    // denotes the same manifest's whole case set twice, and the walk refuses it by
    // name rather than quietly loading it once.
    includes.sort();
    Ok(includes)
}

/// Every `mf:entries` list member's test IRI, with NO further requirement beyond
/// list membership — used only to detect a member the main loading query silently
/// dropped (see the completeness check in [`load`]).
fn list_entry_iris(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
) -> Result<Vec<String>, String> {
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         SELECT ?test WHERE {{\n\
         ?mani mf:entries/rdf:rest*/rdf:first ?test .\n\
         }}"
    );
    let rows = query_rows(dataset, &query)?;
    rows.iter()
        .map(|row| iri_of(row, "test").ok_or_else(|| "mf:entries member is not an IRI".to_string()))
        .collect()
}

/// Set `regime` for each test that declares an `sd:entailmentRegime` list,
/// choosing the regime the native reasoner should materialize under.
fn load_entailment_regimes(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    cases: &mut [SparqlTestCase],
) -> Result<(), String> {
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         PREFIX sd: <{SD_NS}>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         SELECT ?test ?regime WHERE {{\n\
         ?mani mf:entries/rdf:rest*/rdf:first ?test .\n\
         ?test mf:action ?act .\n\
         {{ ?act sd:entailmentRegime ?regime }}\n\
         UNION\n\
         {{ ?act sd:entailmentRegime/rdf:rest*/rdf:first ?regime }}\n\
         }}"
    );
    let rows = query_rows(dataset, &query)?;
    if rows.is_empty() {
        return Ok(()); // no entailment tests in this manifest
    }
    let mut by_test: BTreeMap<String, Vec<purrdf_entail::Regime>> = BTreeMap::new();
    for row in &rows {
        if let (Some(test), Some(reg)) = (iri_of(row, "test"), iri_of(row, "regime"))
            && let Some(r) = purrdf_entail::Regime::from_iri(&reg)
        {
            by_test.entry(test).or_default().push(r);
        }
    }
    for case in cases.iter_mut() {
        if let Some(regimes) = by_test.get(&case.iri) {
            case.regime = pick_regime(regimes);
        }
    }
    Ok(())
}

/// Choose the regime to materialize. `OWL-Direct` is preferred when declared: the
/// native DL reasoner answers it query-directed (`purrdf_entail::materialize_dl_reported`), which
/// is the strongest regime and subsumes the RDFS / OWL-RL answers for these cases. Else
/// prefer the weakest that still entails (RDFS), then OWL-RL, then the identity regimes.
/// Boundaries the native reasoner cannot materialize (D) yield `None` — the case runs
/// unmaterialized and, if it needs those entailments, is a typed `Entailment` xfail.
fn pick_regime(regimes: &[purrdf_entail::Regime]) -> Option<purrdf_entail::Regime> {
    use purrdf_entail::Regime::{OwlDirect, OwlRl, Rdf, Rdfs, Rif, Simple};
    // RIF-declared cases run through the RIF rule engine (wired in `run.rs`), which
    // needs the RAW dataset — so `Rif` is selected but `load_dataset` passes it
    // through unmaterialized, exactly like `OwlDirect`. The relative order among the
    // others is immaterial for the RIF cases (they declare only `ent:RIF`).
    [OwlDirect, Rdfs, OwlRl, Rdf, Simple, Rif]
        .into_iter()
        .find(|pref| regimes.contains(pref))
}

/// An accumulated expected post-state: `(default-graph files, named graphs)`.
type ExpectedState = (Vec<PathBuf>, Vec<(String, PathBuf)>);

/// Fill in the `ut:`-vocabulary fields for every [`TestKind::UpdateEval`] case:
/// the `ut:request` update file, the pre-state (`ut:data`/`ut:graphData`), and
/// the expected post-state (`mf:result` → `ut:data`/`ut:graphData`).
fn load_update_details(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    resolver: &BaseResolver,
    cases: &mut [SparqlTestCase],
) -> Result<(), String> {
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         PREFIX ut: <{UT}>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         PREFIX rdfs: <{RDFS_LABEL_NS}>\n\
         SELECT ?test ?request ?inData ?inGraph ?inLabel ?resData ?resGraph ?resLabel WHERE {{\n\
         ?mani mf:entries/rdf:rest*/rdf:first ?test .\n\
         ?test mf:action ?act .\n\
         OPTIONAL {{ ?act ut:request ?request }}\n\
         OPTIONAL {{ ?act ut:data ?inData }}\n\
         OPTIONAL {{ ?act ut:graphData ?ig . ?ig ut:graph ?inGraph . OPTIONAL {{ ?ig rdfs:label ?inLabel }} }}\n\
         OPTIONAL {{ ?test mf:result ?res .\n\
           OPTIONAL {{ ?res ut:data ?resData }}\n\
           OPTIONAL {{ ?res ut:graphData ?rg . ?rg ut:graph ?resGraph . OPTIONAL {{ ?rg rdfs:label ?resLabel }} }}\n\
         }}\n\
         }}"
    );
    let rows = query_rows(dataset, &query)?;

    // Accumulate the expected post-state per test IRI (built as we scan rows).
    let mut expected: BTreeMap<String, ExpectedState> = BTreeMap::new();

    let by_iri: BTreeMap<String, usize> = cases
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == TestKind::UpdateEval)
        .map(|(i, c)| (c.iri.clone(), i))
        .collect();

    for row in &rows {
        let Some(test_iri) = iri_of(row, "test") else {
            continue;
        };
        let Some(&idx) = by_iri.get(test_iri.as_str()) else {
            continue; // not an update test (or not modeled) — leave untouched
        };
        let case = &mut cases[idx];

        if let Some(req) = iri_of(row, "request") {
            case.query = resolver.path(&req)?;
        }
        push_unique_path(
            &mut case.data,
            resolve_opt(resolver, iri_of(row, "inData"))?,
        );
        if let Some(g) = iri_of(row, "inGraph") {
            let name = lexical_of(row, "inLabel").unwrap_or_else(|| g.clone());
            let path = resolver.path(&g)?;
            if !case.graph_data.iter().any(|(n, _)| *n == name) {
                case.graph_data.push((name, path));
            }
        }

        let acc = expected.entry(test_iri.clone()).or_default();
        push_unique_path(&mut acc.0, resolve_opt(resolver, iri_of(row, "resData"))?);
        if let Some(g) = iri_of(row, "resGraph") {
            let name = lexical_of(row, "resLabel").unwrap_or_else(|| g.clone());
            let path = resolver.path(&g)?;
            if !acc.1.iter().any(|(n, _)| *n == name) {
                acc.1.push((name, path));
            }
        }
    }

    for (iri, idx) in by_iri {
        let (data, graph_data) = expected.remove(&iri).unwrap_or_default();
        cases[idx].expected = ExpectedResult::DatasetState { data, graph_data };
    }
    Ok(())
}

/// Run `query` against `dataset` and return its solution rows as variable→value
/// maps (unbound cells omitted).
///
/// `pub(crate)` because [`crate::rs_resultset`] reuses the exact same
/// dog-fooded query-and-decode path to read the `rs:ResultSet` Turtle result
/// encoding, rather than duplicating a second ad hoc SPARQL runner.
pub(crate) fn query_rows(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    query: &str,
) -> Result<Vec<BTreeMap<String, TermValue>>, String> {
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            dataset,
            SparqlRequest {
                query,
                // The base for the QUERY TEXT, which is a different thing from the
                // manifest's own base: every IRI in every query this module and
                // `crate::rs_resultset` build is written out absolutely, so no
                // relative reference is ever resolved against it and it can stay the
                // shared root. It must NOT be mistaken for the per-manifest base —
                // that one is computed by `manifest_base` and threaded through
                // `local_path`.
                base_iri: Some(BASE_ROOT),
                substitutions: &[],
            },
        )
        .map_err(|e| format!("manifest query failed: {e}"))?;
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => Ok(rows
            .into_iter()
            .map(|row| {
                variables
                    .iter()
                    .zip(row)
                    .filter_map(|(v, cell)| cell.map(|t| (v.clone(), t)))
                    .collect()
            })
            .collect()),
        other => Err(format!(
            "manifest query did not return solutions: {other:?}"
        )),
    }
}

/// The IRI string of a bound variable, if it is an IRI.
fn iri_of(row: &BTreeMap<String, TermValue>, var: &str) -> Option<String> {
    match row.get(var) {
        Some(TermValue::Iri(i)) => Some(i.clone()),
        _ => None,
    }
}

/// The lexical form of a bound literal variable.
fn lexical_of(row: &BTreeMap<String, TermValue>, var: &str) -> Option<String> {
    match row.get(var) {
        Some(TermValue::Literal { lexical_form, .. }) => Some(lexical_form.clone()),
        _ => None,
    }
}

/// One manifest's sentinel IRI space and the disk directory it denotes.
///
/// The base is [`BASE_ROOT`] followed by the manifest's directory path **relative
/// to the Cargo workspace root**, and [`BASE_ROOT`] itself denotes that workspace
/// root. So the sentinel IRI space is an exact mirror of the workspace tree, and
/// mapping a file IRI back to a file is one strip and one join — for a reference
/// pointing down (`<agg01.rq>`) and equally for one pointing up (`<../x.ttl>`),
/// which a base-relative strip alone could not resolve.
///
/// # Why the base is per-manifest
///
/// Deriving the base from the manifest's own location is what makes a case IRI
/// globally unique. A single constant base gave every manifest that declares the
/// relative `@prefix : <manifest#>` the identical namespace `<BASE_ROOT>manifest#`,
/// so two sibling group manifests minted byte-identical case IRIs for every local
/// name they share — and the global [`crate::xfail`] ledger, which matches on the
/// case IRI, could then not tell one group's `get-01` from another's.
///
/// The anchor is the workspace root, not the caller's working directory and not
/// the absolute path, so the base — and every IRI resolved against it — is
/// byte-identical in every checkout on every machine, whether the caller passes a
/// relative or an absolute manifest path. A corpus outside the workspace has no
/// such stable identity and is refused rather than given a machine-specific one.
///
/// Every manifest wired into the live suite today declares an ABSOLUTE `@prefix`,
/// so their case IRIs are unaffected: an absolute prefix ignores the base
/// entirely. Only a manifest using a relative prefix — which is exactly the shape
/// that collided — sees its IRIs change.
struct BaseResolver {
    /// This manifest's own base IRI, ending in `/`.
    base: String,
    /// The Cargo workspace root, which [`BASE_ROOT`] denotes.
    workspace_root: PathBuf,
}

impl BaseResolver {
    /// Derive the base for `manifest_path`.
    fn new(manifest_path: &Path) -> Result<Self, String> {
        let dir = manifest_path.parent().ok_or_else(|| {
            format!(
                "{}: manifest path has no parent directory",
                manifest_path.display()
            )
        })?;
        let dir = dir
            .canonicalize()
            .map_err(|e| format!("resolve manifest directory {}: {e}", dir.display()))?;
        let workspace_root = workspace_root(&dir).ok_or_else(|| {
            format!(
                "{}: no ancestor directory carries a Cargo.toml with a [workspace] table, so no \
                 workspace-relative manifest identity can be derived. The conformance corpora \
                 must live inside this workspace; a base derived from an absolute path would \
                 differ between checkouts and put machine-specific strings into every case IRI",
                manifest_path.display()
            )
        })?;
        let relative = dir.strip_prefix(&workspace_root).map_err(|_| {
            format!(
                "{}: manifest directory is not under the workspace root {}",
                manifest_path.display(),
                workspace_root.display()
            )
        })?;

        let mut base = String::from(BASE_ROOT);
        for component in relative.components() {
            let std::path::Component::Normal(name) = component else {
                return Err(format!(
                    "{}: workspace-relative path carries a non-ordinary component \
                     ({component:?}); it cannot be turned into a stable manifest base",
                    manifest_path.display()
                ));
            };
            let name = name.to_str().ok_or_else(|| {
                format!(
                    "{}: path component is not valid UTF-8, so it cannot become an IRI segment",
                    manifest_path.display()
                )
            })?;
            if !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '~'))
            {
                return Err(format!(
                    "{}: path component {name:?} carries a character outside the IRI unreserved \
                     set. Percent-encoding it here would let two different directories normalize \
                     onto one base and re-open the collision this per-manifest base exists to \
                     close, so the path is refused instead: name corpus directories with ASCII \
                     letters, digits, '.', '-', '_' or '~'",
                    manifest_path.display()
                ));
            }
            base.push_str(name);
            base.push('/');
        }
        Ok(Self {
            base,
            workspace_root,
        })
    }

    /// Map a sentinel-space file IRI back to the file it denotes.
    ///
    /// # Errors
    ///
    /// Refuses an IRI outside the sentinel space — an absolute reference to some
    /// other authority, or one whose `..` segments escaped [`BASE_ROOT`] entirely.
    /// Such an IRI names no file in this workspace; silently falling back to
    /// joining it onto the manifest directory (as this used to) produced a path
    /// that could never open, so the failure surfaced later as an unreadable
    /// fixture instead of here as the unresolvable reference it is.
    fn path(&self, iri: &str) -> Result<PathBuf, String> {
        let relative = iri.strip_prefix(BASE_ROOT).ok_or_else(|| {
            format!(
                "manifest based at {} references <{iri}>, which is outside the sentinel space \
                 {BASE_ROOT} and therefore names no file in this workspace",
                self.base
            )
        })?;
        Ok(paths::resolve(&self.workspace_root, relative))
    }
}

/// The nearest ancestor of `start` (inclusive) whose `Cargo.toml` declares a
/// `[workspace]` table.
///
/// Member crates carry `[package]` and `workspace = true` VALUES but never a
/// `[workspace]` section header, so the first ancestor that matches is the true
/// workspace root and not an intervening member.
fn workspace_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .is_ok_and(|text| text.lines().any(|line| line.trim() == "[workspace]"))
        })
        .map(Path::to_path_buf)
}

/// Resolve an OPTIONAL file IRI, keeping "no such column bound" (`None`) distinct
/// from "bound to an IRI that names no file here" (an error).
fn resolve_opt(resolver: &BaseResolver, iri: Option<String>) -> Result<Option<PathBuf>, String> {
    iri.map(|i| resolver.path(&i)).transpose()
}

/// Push `path` into `dst` if present and not already there.
fn push_unique_path(dst: &mut Vec<PathBuf>, path: Option<PathBuf>) {
    if let Some(p) = path
        && !dst.contains(&p)
    {
        dst.push(p);
    }
}

/// Classify a manifest entry's `rdf:type` IRI into a [`TestKind`].
fn classify(type_term: Option<&TermValue>) -> TestKind {
    let Some(TermValue::Iri(t)) = type_term else {
        return TestKind::Unknown;
    };
    let local = t.strip_prefix(MF).unwrap_or(t);
    match local {
        "QueryEvaluationTest" | "CSVResultFormatTest" => TestKind::QueryEval,
        "UpdateEvaluationTest" => TestKind::UpdateEval,
        "PositiveSyntaxTest" | "PositiveSyntaxTest11" => TestKind::PositiveSyntax,
        "NegativeSyntaxTest" | "NegativeSyntaxTest11" => TestKind::NegativeSyntax,
        "PositiveUpdateSyntaxTest" | "PositiveUpdateSyntaxTest11" => TestKind::PositiveUpdateSyntax,
        "NegativeUpdateSyntaxTest" | "NegativeUpdateSyntaxTest11" => TestKind::NegativeUpdateSyntax,
        _ => TestKind::Unknown,
    }
}

/// The `rs:` (SPARQL result-set) vocabulary namespace: a Turtle file describing
/// an `rs:ResultSet` encodes a SELECT solution sequence, not a graph, so it
/// must be routed to [`ExpectedResult::ResultSetTurtle`] rather than
/// [`ExpectedResult::Graph`]. See [`crate::rs_resultset`].
const RS_NS: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#";

/// Classify a result file by extension; a `.ttl` file is additionally content-
/// sniffed for the `rs:ResultSet` encoding (a plain substring check — the real
/// parse in [`crate::rs_resultset`] validates the shape and errors loudly on a
/// false positive, so this is a routing hint, not the correctness boundary).
fn classify_result(path: &Path) -> ExpectedResult {
    match path.extension().and_then(|e| e.to_str()) {
        Some("srx") => ExpectedResult::Srx(path.to_path_buf()),
        Some("srj") => ExpectedResult::Srj(path.to_path_buf()),
        Some("err") => ExpectedResult::EvalError(path.to_path_buf()),
        Some("ttl") if is_rs_resultset_turtle(path) => {
            ExpectedResult::ResultSetTurtle(path.to_path_buf())
        }
        Some("ttl" | "nt" | "nq" | "rdf") => ExpectedResult::Graph(path.to_path_buf()),
        _ => ExpectedResult::Unsupported(path.to_path_buf()),
    }
}

/// Whether `path` textually mentions the `rs:ResultSet` type IRI. Cheap and
/// content-based (not extension-based) because the W3C suite ships `.ttl`
/// result files in both shapes (plain CONSTRUCT graphs and `rs:ResultSet`
/// solution descriptions) under the same extension.
fn is_rs_resultset_turtle(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.contains(RS_NS) && text.contains("ResultSet")
}
