// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Running a discovered case: load data, parse + evaluate the query.

use std::sync::Arc;

use purrdf::{SerializeGraph, serialize_dataset};
use purrdf_core::{
    RdfDataset, RdfTextDirection, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_entail::{QNode, QTriple};
use purrdf_sparql_algebra::{
    BaseDirection, GraphPattern, Literal, NamedNodePattern, Query, SparqlParser, TermPattern,
    TriplePattern,
};
use purrdf_sparql_eval::{
    LossVocabulary, NativeSparqlEngine, ParserOptions, RemoteQuerySource, StandpointPredicates,
};

use crate::manifest::{SparqlTestCase, TestKind};

pub(crate) const BASE: &str = "http://purrdf.test/manifest/";

/// The extension-function namespace the first-party suite fixtures spell their
/// calls under. PurRDF itself mints no vocabulary — the namespace is HARNESS
/// configuration (a neutral example.org name), exactly as a real deployment
/// supplies its own ontology namespace.
const EXT_NS: &str = "https://example.org/ext/";

/// The loss-declaration namespace used by the first-party loss-aware CONSTRUCT
/// cases. Like `EXT_NS`, this is harness configuration, not an engine constant.
const LOSS_NS: &str = "https://example.org/ext/loss/";

/// The outcome of running a case (before comparison against the expected result).
#[derive(Debug)]
pub enum RunOutcome {
    /// A `QueryEvaluationTest` result.
    Eval {
        /// The engine's result.
        result: SparqlResult,
        /// Whether the query carries a **top-level** `ORDER BY` (§18.5): the row
        /// order of a `SELECT`'s solutions is then observable and the comparer
        /// must check it as an ordered sequence, not a multiset. `SparqlResult`
        /// carries no ordered flag, so it is derived here from the parsed query.
        ordered: bool,
    },
    /// An `UpdateEvaluationTest` post-state: the dataset after applying the update.
    Update(Arc<RdfDataset>),
    /// A syntax test: did the query parse?
    Syntax {
        /// `true` when the query text parsed without error.
        parsed_ok: bool,
    },
}

/// Load the case's `qt:data` and `qt:graphData` files into a combined dataset.
///
/// Default-graph data (`qt:data` Turtle files) is merged into the default graph.
/// Named-graph data (`qt:graphData`) is placed in the named graph identified by
/// its file IRI: each triple from the file is tagged with the graph IRI so it
/// appears in the named graph when queried with `GRAPH <iri> { … }`.
///
/// Both scoping axes are supported: named-graph worlds (queried via `GRAPH ?world
/// { … }`) and the standpoint poset (queried via `purrdf:heldIn` over the default-
/// graph reification layer). The combined-world case proves both axes with a JOIN:
/// a named-graph world triple joined against a default-graph standpoint-held
/// reifier.
///
/// # Errors
///
/// Returns a message on any read, parse, or serialize failure (never silent).
pub fn load_dataset(case: &SparqlTestCase) -> Result<Arc<RdfDataset>, String> {
    use purrdf_entail::Regime;
    let ds = build_dataset(&case.data, &case.graph_data)?;
    // For a forward-materializable entailment regime, close the dataset before it is
    // queried (the eval loop is untouched — it queries an already-reasoned dataset).
    // `OWL-Direct` is NOT forward-materializable: it is query-directed and handled by
    // the caller (the `QueryEval` arm) via `purrdf_entail::materialize_dl`, so the RAW
    // dataset is returned here. `RIF` (unwired) and `D` likewise pass through raw.
    match case.regime {
        Some(regime @ (Regime::Simple | Regime::Rdf | Regime::Rdfs | Regime::OwlRl)) => {
            purrdf_entail::materialize(&ds, regime)
                .map_err(|e| format!("entailment ({regime:?}) for {}: {e}", case.iri))
        }
        _ => Ok(ds),
    }
}

/// The native media type for a data file, by extension. Most fixtures are Turtle,
/// but the RDF-1.2 eval-triple-term tests carry `.trig` quad data (GRAPH blocks),
/// which the Turtle codec rejects.
fn data_media_type(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("trig") => "application/trig",
        Some("nq") => "application/n-quads",
        Some("nt") => "application/n-triples",
        Some("rdf") => "application/rdf+xml",
        _ => "text/turtle",
    }
}

/// The per-file base IRI a `qt:data`/`qt:graphData` Turtle file is parsed
/// against: `<BASE><file name>`, matching how the manifest's OWN relative
/// reference (e.g. `<exists-graph-variable.ttl>`) resolves against the harness's
/// sentinel [`BASE`] — the vendored suite never nests fixtures in
/// subdirectories, so a bare file name round-trips the manifest's relative
/// reference exactly. Using the SHARED harness-wide [`BASE`] for every file
/// instead (as opposed to the file's own resolved IRI) would make a bare `<>`
/// inside the Turtle content resolve to the same IRI for every fixture, instead
/// of self-referencing that fixture's own `qt:data`/`qt:graphData` graph name —
/// the exact self-reference some W3C fixtures (e.g. `exists-graph-variable`)
/// depend on.
fn file_base_iri(path: &std::path::Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    format!("{BASE}{name}")
}

/// Build a dataset from default-graph Turtle files (`data`) and named-graph files
/// (`graph_data`, each `(graph IRI, file)`). Shared by the query pre-state loader
/// and the UPDATE pre-/post-state builders.
///
/// # Errors
///
/// Returns a message on any read, parse, or serialize failure (never silent).
pub fn build_dataset(
    data: &[std::path::PathBuf],
    graph_data: &[(String, std::path::PathBuf)],
) -> Result<Arc<RdfDataset>, String> {
    // Serialize each qt:data Turtle file to N-Quads (default graph — no graph tag).
    let mut combined_nq: Vec<u8> = Vec::new();
    for data in data {
        let chunk = std::fs::read(data).map_err(|e| format!("read {}: {e}", data.display()))?;
        let ds = purrdf::parse_dataset(&chunk, data_media_type(data), Some(&file_base_iri(data)))
            .map_err(|e| format!("parse data {}: {e}", data.display()))?;
        let nq = serialize_dataset(&ds, "application/n-quads", SerializeGraph::Dataset)
            .map_err(|e| format!("serialize {}: {e}", data.display()))?;
        combined_nq.extend_from_slice(&nq);
        if combined_nq.last() != Some(&b'\n') {
            combined_nq.push(b'\n');
        }
    }

    // Serialize each qt:graphData Turtle file to N-Quads, then tag every triple line
    // with the named-graph IRI so it is placed in that named graph. A file that
    // parses to ZERO quads (e.g. `empty.ttl`) leaves no trace in N-Quads text — the
    // format has no syntax for "this named graph exists but is empty" — so its IRI
    // is separately remembered in `empty_graphs` and explicitly declared on the
    // final builder below (RdfDataset's `named_graphs`, RDF 1.1 §3's "an RDF
    // dataset MAY include an empty named graph").
    let mut empty_graphs: Vec<&str> = Vec::new();
    for (graph_iri, path) in graph_data {
        let chunk = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        // Parse against the graph's OWN resolved IRI (not the shared harness
        // `BASE`) so a bare `<>` inside the file self-references this named
        // graph, exactly like a real per-file base would.
        let ds = purrdf::parse_dataset(&chunk, data_media_type(path), Some(graph_iri))
            .map_err(|e| format!("parse graph data {}: {e}", path.display()))?;
        if ds.quad_count() == 0 {
            empty_graphs.push(graph_iri.as_str());
            continue;
        }
        let nq = serialize_dataset(&ds, "application/n-quads", SerializeGraph::Dataset)
            .map_err(|e| format!("serialize graph data {}: {e}", path.display()))?;
        let nq_text = std::str::from_utf8(&nq)
            .map_err(|e| format!("utf-8 in serialized nquads for {}: {e}", path.display()))?;

        // Tag each triple line (lines ending with ` .`) with the named-graph IRI.
        // Comment lines and blank lines are passed through unchanged.
        // Lines that already carry a graph term (four-element quads) are also passed through.
        for line in nq_text.lines() {
            let trimmed = line.trim_end();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                combined_nq.extend_from_slice(trimmed.as_bytes());
            } else if let Some(body) = trimmed.strip_suffix(" .") {
                // Strip the trailing ` .`, insert the graph IRI, re-append ` .`
                combined_nq.extend_from_slice(body.as_bytes());
                combined_nq.extend_from_slice(b" <");
                combined_nq.extend_from_slice(graph_iri.as_bytes());
                combined_nq.extend_from_slice(b"> .");
            } else {
                combined_nq.extend_from_slice(trimmed.as_bytes());
            }
            combined_nq.push(b'\n');
        }
    }

    let parsed = purrdf::parse_dataset(&combined_nq, "application/n-quads", Some(BASE))
        .map_err(|e| format!("parse combined n-quads: {e}"))?;
    if empty_graphs.is_empty() {
        return Ok(parsed);
    }

    // Re-intern the parsed quads into a fresh builder (standard `push_dataset`
    // merge) so the empty graph IRIs can be declared alongside them before freeze.
    let mut builder = purrdf_core::RdfDatasetBuilder::new();
    builder.push_dataset(&parsed);
    for graph_iri in empty_graphs {
        let g = builder.intern_iri(graph_iri);
        builder.declare_named_graph(g);
    }
    builder
        .freeze()
        .map_err(|e| format!("freeze dataset with declared empty graphs: {e}"))
}

/// Run `case`, optionally resolving `SERVICE` clauses through `remote`.
///
/// # Errors
///
/// Returns a message on a read/parse/evaluation failure (the harness decides
/// whether that is an expected failure).
pub fn run(
    case: &SparqlTestCase,
    remote: Option<&(dyn RemoteQuerySource + Sync)>,
) -> Result<RunOutcome, String> {
    let query_text = std::fs::read_to_string(&case.query)
        .map_err(|e| format!("read query {}: {e}", case.query.display()))?;

    match case.kind {
        // W3C syntax tests are parsed against the test file's own IRI as the
        // in-scope BASE (§4.1.1.1), so relative IRI references in the query
        // (e.g. `<x>`, `FROM <file>`) resolve to absolute term-position IRIs
        // rather than being (correctly) rejected as scheme-less. The harness's
        // per-file sentinel base mirrors how the manifest's own relative
        // `mf:action <file.rq>` resolves against [`BASE`].
        TestKind::PositiveSyntax | TestKind::NegativeSyntax => {
            let parsed_ok = SparqlParser::new()
                .with_base_iri(file_base_iri(&case.query))
                .parse_query(&query_text)
                .is_ok();
            Ok(RunOutcome::Syntax { parsed_ok })
        }
        TestKind::PositiveUpdateSyntax | TestKind::NegativeUpdateSyntax => {
            let parsed_ok = SparqlParser::new()
                .with_base_iri(file_base_iri(&case.query))
                .parse_update(&query_text)
                .is_ok();
            Ok(RunOutcome::Syntax { parsed_ok })
        }
        TestKind::QueryEval => {
            let mut dataset = load_dataset(case)?;
            // OWL-Direct is query-directed: augment the RAW dataset with the DL
            // entailments its basic graph pattern needs, then hand the augmented
            // dataset to the UNMODIFIED engine (whose simple-entailment answers then
            // coincide with the OWL Direct-Semantics certain answers).
            if case.regime == Some(purrdf_entail::Regime::OwlDirect) {
                let bgp = collect_query_bgp(&query_text);
                dataset = purrdf_entail::materialize_dl(&dataset, &bgp)
                    .map_err(|e| format!("OWL-Direct entailment for {}: {e}", case.iri))?;
            }
            // RIF entailment: the qt:data graph references one or more `.rif`
            // documents via `rif:usedWithProfile`; parse each (plus its RDF
            // imports) into a Horn rule set, forward-chain it over the RAW dataset,
            // then hand the materialized dataset to the UNMODIFIED engine.
            if case.regime == Some(purrdf_entail::Regime::Rif) {
                let ruleset = build_rif_ruleset(case, &dataset)?;
                dataset = purrdf_entail::materialize_rif(&dataset, &ruleset)
                    .map_err(|e| format!("RIF entailment for {}: {e}", case.iri))?;
            }
            // Both the extension-function namespace and the standpoint predicate
            // table are CALLER configuration (the engine has no defaults): the
            // purrdf-extend suite's standpoint cases exercise `ext:heldIn` and the
            // purrdf-list-functions suite the `ext:list*` functions, all spelled
            // under the harness-configured example.org/ext/ namespace, against
            // fixture data written in the same namespace — so the harness supplies
            // that namespace plus its accordingTo/sharpens table here. (A gmeow
            // deployment would supply its own gmeow IRIs instead — everything
            // flows through configuration, not constants.) Harmless for the W3C
            // suites, which never call the extension functions.
            let parser_options = ParserOptions {
                extension_fn_namespaces: vec![EXT_NS.to_owned()],
            };
            let engine = NativeSparqlEngine::new()
                .with_parser_options(parser_options.clone())
                .with_standpoint_predicates(StandpointPredicates::new(
                    format!("{EXT_NS}accordingTo"),
                    format!("{EXT_NS}sharpens"),
                ))
                .with_loss_vocabulary(LossVocabulary::new(
                    format!("{LOSS_NS}ProjectionLoss"),
                    format!("{LOSS_NS}lossCode"),
                    format!("{LOSS_NS}lostReifies"),
                ));
            let request = SparqlRequest {
                query: &query_text,
                base_iri: Some(BASE),
                substitutions: &[],
            };
            let result = match remote {
                Some(source) => engine.query_with_source(&dataset, request, source),
                None => engine.query(&dataset, request),
            }
            .map_err(|e| format!("evaluate {}: {e}", case.iri))?;
            let ordered = query_is_top_level_ordered(&query_text, &parser_options);
            Ok(RunOutcome::Eval { result, ordered })
        }
        TestKind::UpdateEval => {
            // Apply the `ut:request` update to the pre-state dataset; the mutated
            // dataset is diffed against the expected post-state in `compare`.
            let mut dataset = build_dataset(&case.data, &case.graph_data)?;
            let engine = NativeSparqlEngine::new().with_parser_options(ParserOptions {
                extension_fn_namespaces: vec![EXT_NS.to_owned()],
            });
            let request = SparqlRequest {
                query: &query_text,
                base_iri: Some(BASE),
                substitutions: &[],
            };
            engine
                .update(&mut dataset, request)
                .map_err(|e| format!("apply update {}: {e}", case.iri))?;
            Ok(RunOutcome::Update(dataset))
        }
        TestKind::Unknown => Err(format!("unmodeled test type for {}", case.iri)),
    }
}

/// Whether `query_text` is a `SELECT` with a **top-level** `ORDER BY`, i.e. one
/// whose sort determines the observable row order of the whole result.
///
/// A `SELECT`'s modifier chain wraps the ordered pattern outermost-to-innermost
/// as `Slice → Distinct/Reduced → Project → OrderBy → …` (see the algebra
/// parser's query-form construction), so a top-level `ORDER BY` is found by
/// descending through exactly those solution-modifier wrappers and checking for
/// an [`GraphPattern::OrderBy`] before any other node. An `ORDER BY` buried
/// inside a sub-`SELECT` (below a join, `GRAPH`, etc.) does NOT surface here —
/// only the sub-query's own slice is observable, not its sort — which is
/// exactly the W3C rule (§18.5: order is only defined for a top-level sort).
///
/// A parse failure (or a non-`SELECT` form) yields `false`: an unordered
/// comparison is the conservative default, and a query the harness could not
/// parse would already have failed evaluation before reaching the comparer.
fn query_is_top_level_ordered(query_text: &str, options: &ParserOptions) -> bool {
    let Ok(Query::Select { pattern, .. }) =
        SparqlParser::new().parse_query_with(query_text, options)
    else {
        return false;
    };
    let mut node = &pattern;
    loop {
        match node {
            GraphPattern::OrderBy { .. } => return true,
            GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => node = inner,
            _ => return false,
        }
    }
}

/// The RIF vocabulary predicate a `qt:data` graph uses to reference the `.rif`
/// document(s) whose rules govern the case.
const RIF_USED_WITH_PROFILE: &str = "http://www.w3.org/2007/rif#usedWithProfile";

/// Build the combined RIF [`RuleSet`](purrdf_entail::RuleSet) for a `Rif`-regime
/// case by scanning `dataset` for `?doc rif:usedWithProfile ?profile` triples,
/// resolving each `?doc` to a local `.rif` fixture beside the case's `qt:data`
/// file, and parsing it (with its RDF imports) into a rule set.
///
/// # Errors
///
/// Returns a message if the case has no `qt:data` file (so no fixture directory),
/// if no `.rif` reference is found, or if any referenced `.rif` fails to parse.
fn build_rif_ruleset(
    case: &SparqlTestCase,
    dataset: &RdfDataset,
) -> Result<purrdf_entail::RuleSet, String> {
    let dir = case
        .data
        .first()
        .and_then(|p| p.parent())
        .ok_or_else(|| format!("RIF case {} has no qt:data fixture directory", case.iri))?;

    // Collect the referenced `.rif` basenames in first-seen dataset order (dedup),
    // so the combined rule set is deterministic regardless of triple iteration.
    let mut basenames: Vec<String> = Vec::new();
    for q in dataset.quads() {
        if q.g.is_some() {
            continue;
        }
        if !matches!(dataset.term_value(q.p), TermValue::Iri(p) if p == RIF_USED_WITH_PROFILE) {
            continue;
        }
        if let TermValue::Iri(doc) = dataset.term_value(q.s)
            && let Some(name) = doc.rsplit(['/', '#']).next().filter(|s| !s.is_empty())
        {
            let name = name.to_owned();
            if !basenames.contains(&name) {
                basenames.push(name);
            }
        }
    }
    if basenames.is_empty() {
        return Err(format!(
            "RIF case {} references no rif:usedWithProfile document",
            case.iri
        ));
    }

    let mut ruleset = purrdf_entail::RuleSet::new();
    for name in basenames {
        let rif_path = dir.join(&name);
        ruleset.extend(crate::rif_xml::load_ruleset(&rif_path)?);
    }
    Ok(ruleset)
}

/// Parse `query_text` and collect every basic-graph-pattern triple, translated into
/// the neutral [`QTriple`] representation the OWL-Direct reasoner consumes.
///
/// A parse failure yields an empty pattern: the reasoner then augments only the
/// data's own vocabulary, and the engine (which will also fail to parse) reports the
/// error. RDF-1.2 quoted-triple term positions (absent from the entailment fixtures)
/// are skipped — they are never a class-expression scaffold.
fn collect_query_bgp(query_text: &str) -> Vec<QTriple> {
    let Ok(query) = SparqlParser::new()
        .with_base_iri(BASE)
        .parse_query(query_text)
    else {
        return Vec::new();
    };
    let pattern = match &query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => pattern,
    };
    let mut triples: Vec<&TriplePattern> = Vec::new();
    collect_bgp(pattern, &mut triples);
    triples
        .into_iter()
        .filter_map(|tp| {
            Some(QTriple {
                s: term_to_qnode(&tp.subject)?,
                p: named_node_pattern_to_qnode(&tp.predicate),
                o: term_to_qnode(&tp.object)?,
            })
        })
        .collect()
}

/// Recursively gather every [`TriplePattern`] out of `p` (from `Bgp` nodes, descending
/// through every join / filter / graph / optional / union / modifier wrapper).
fn collect_bgp<'a>(p: &'a GraphPattern, out: &mut Vec<&'a TriplePattern>) {
    match p {
        GraphPattern::Bgp { patterns } => out.extend(patterns.iter()),
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::LeftJoin { left, right, .. } => {
            collect_bgp(left, out);
            collect_bgp(right, out);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => collect_bgp(inner, out),
        GraphPattern::Path { .. } | GraphPattern::Values { .. } => {}
    }
}

/// Translate a subject/object [`TermPattern`] into a [`QNode`] (`None` for an RDF-1.2
/// quoted-triple term, which cannot scaffold a class expression).
fn term_to_qnode(t: &TermPattern) -> Option<QNode> {
    Some(match t {
        TermPattern::Variable(v) => QNode::Var(v.as_str().to_owned()),
        TermPattern::NamedNode(n) => QNode::Term(TermValue::iri(n.as_str())),
        TermPattern::BlankNode(b) => QNode::Term(TermValue::blank(b.as_str())),
        TermPattern::Literal(l) => QNode::Term(literal_to_term_value(l)),
        TermPattern::Triple(_) => return None,
    })
}

/// Translate a predicate [`NamedNodePattern`] into a [`QNode`].
fn named_node_pattern_to_qnode(p: &NamedNodePattern) -> QNode {
    match p {
        NamedNodePattern::NamedNode(n) => QNode::Term(TermValue::iri(n.as_str())),
        NamedNodePattern::Variable(v) => QNode::Var(v.as_str().to_owned()),
    }
}

/// Translate an algebra [`Literal`] into a [`TermValue`] (language lowercased per C0.1).
fn literal_to_term_value(l: &Literal) -> TermValue {
    match l.language() {
        Some(lang) => TermValue::Literal {
            lexical_form: l.value().to_owned(),
            datatype: l.datatype().as_str().to_owned(),
            language: Some(lang.to_ascii_lowercase()),
            direction: l.direction().map(|d| match d {
                BaseDirection::Ltr => RdfTextDirection::Ltr,
                BaseDirection::Rtl => RdfTextDirection::Rtl,
            }),
        },
        None => TermValue::typed_literal(l.value(), l.datatype().as_str()),
    }
}
