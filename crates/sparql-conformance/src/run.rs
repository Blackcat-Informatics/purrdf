// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Running a discovered case: load data, parse + evaluate the query.

use std::sync::{Arc, OnceLock};

use purrdf::{SerializeGraph, serialize_dataset};
use purrdf_core::{
    GraphMatch, RdfDataset, RdfTextDirection, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_entail::{QNode, QTriple};
use purrdf_sparql_algebra::{
    BaseDirection, GraphPattern, Literal, NamedNodePattern, Query, SparqlParser, TermPattern,
    TriplePattern,
};
use purrdf_sparql_eval::{
    LossVocabulary, MemoryRelation, NativeSparqlEngine, ParserOptions, PropertyFunctionRegistry,
    QueryOptions, ServiceResolver, StandpointPredicates,
};

use crate::manifest::{BASE_ROOT, SparqlTestCase, TestKind};

/// The extension-function namespace the first-party suite fixtures spell their
/// calls under. PurRDF itself mints no vocabulary — the namespace is HARNESS
/// configuration (a neutral example.org name), exactly as a real deployment
/// supplies its own ontology namespace.
const EXT_NS: &str = "https://example.org/ext/";

/// The loss-declaration namespace used by the first-party loss-aware CONSTRUCT
/// cases. Like `EXT_NS`, this is harness configuration, not an engine constant.
const LOSS_NS: &str = "https://example.org/ext/loss/";

/// The **property-function** namespace the first-party relation fixtures spell
/// their calls under, and the prefix of every IRI `harness_relations` registers.
/// Like `EXT_NS`, this is HARNESS configuration: PurRDF recognizes a predicate as
/// a relation call only because this harness configured the namespace, and it
/// mints no such namespace of its own.
///
/// Public because the differential corpora in `tests/` re-evaluate the same suite
/// cases and must configure the parser identically to be comparing the same query.
pub const REL_NS: &str = "https://example.org/rel/";

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

/// The suite fixture graph the harness relation tables are read out of: an
/// `rdf:List` of `rdf:List`s per table, in the encoding
/// [`MemoryRelation::from_graph`] defines.
const RELATION_TABLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/suite/purrdf-property-functions/relations.ttl"
);

/// The namespace the relation TABLE HEADS in [`RELATION_TABLES`] are named under —
/// the same test namespace the tables' own values use, because a head is a node of
/// that fixture graph rather than a call site.
const TABLE_NS: &str = "http://purrdf.test/relations#";

/// The relation table the harness injects for every non-federated
/// `QueryEvaluationTest`, built once and shared.
///
/// These are HARNESS configuration in exactly the sense `EXT_NS` is: relations under
/// `REL_NS` a deployment injects as host code, so the rows do NOT come from the
/// queried dataset. They come from a **fixture graph** instead
/// (`suite/purrdf-property-functions/relations.ttl`), read through
/// [`MemoryRelation::from_graph`]: the mapping below still decides which predicate
/// IRI resolves to which table head and with what arity — that is configuration, and
/// it is code — but the tuples are data, and a reader checking a case's expected
/// result reads them in the same RDF the rest of the suite is written in.
///
/// Registering them for every case is harmless for the vendored W3C suites: no
/// vendored query spells a predicate under `REL_NS`, and a registered relation is
/// only ever reached through a call node the parser mints from that namespace.
///
/// Public for the same reason `REL_NS` is: a differential corpus that re-runs a
/// suite case has to inject the same relations, or it would be measuring a query
/// whose calls resolve to nothing.
///
/// # Panics
///
/// If the fixture graph is missing, unparseable, or does not hold every declared
/// table in the declared shape. A harness whose relations silently came up empty
/// would turn every relation case into a vacuous pass, so the failure is loud and
/// immediate rather than deferred to a confusing per-case mismatch.
#[must_use]
pub fn harness_relations() -> &'static PropertyFunctionRegistry {
    static RELATIONS: OnceLock<PropertyFunctionRegistry> = OnceLock::new();
    RELATIONS.get_or_init(|| {
        let tables = relation_tables();
        let mut registry = PropertyFunctionRegistry::new();
        // (call IRI local name, table head local name, subject arity, object arity).
        for (call, head, subject_arity, object_arity) in [
            ("memberOf", "memberOfTable", 1, 1),
            ("teamSite", "teamSiteTable", 1, 2),
            ("seeds", "seedsTable", 0, 1),
        ] {
            registry.register(
                format!("{REL_NS}{call}"),
                Arc::new(memory_table(&tables, head, subject_arity, object_arity)),
            );
        }
        // The MODE-RESTRICTED relation. Every table above declares the all-free mode,
        // which subsumes every access pattern — so without this one no suite case can
        // reach mode restriction, the subsumption rule, or the feasibility reorder
        // that exists to serve a relation that is not computable in every direction.
        registry.register(
            format!("{REL_NS}rank"),
            Arc::new(
                crate::mode_restricted::BoundSubjectLookup::from_graph(
                    &*tables,
                    &table_head("rankTable"),
                    GraphMatch::Default,
                )
                .unwrap_or_else(|e| panic!("harness relation table <{TABLE_NS}rankTable>: {e}")),
            ),
        );
        registry
    })
}

/// Parse [`RELATION_TABLES`] into the dataset every table is read out of.
///
/// Parsed against [`BASE_ROOT`] rather than any manifest's own base: this fixture
/// is harness configuration shared by EVERY case (built once into a `OnceLock`),
/// not a case's `qt:data`, so it belongs to no single manifest. Every IRI it
/// carries is written absolutely under [`TABLE_NS`], so the base is inert here —
/// there is no relative reference for it to resolve.
fn relation_tables() -> Arc<RdfDataset> {
    let bytes = std::fs::read(RELATION_TABLES)
        .unwrap_or_else(|e| panic!("read harness relation tables {RELATION_TABLES}: {e}"));
    purrdf::parse_dataset(&bytes, "text/turtle", Some(BASE_ROOT))
        .unwrap_or_else(|e| panic!("parse harness relation tables {RELATION_TABLES}: {e}"))
}

/// Read one table out of the fixture graph as a [`MemoryRelation`].
fn memory_table(
    tables: &RdfDataset,
    head: &str,
    subject_arity: usize,
    object_arity: usize,
) -> MemoryRelation {
    MemoryRelation::from_graph(
        tables,
        &table_head(head),
        GraphMatch::Default,
        subject_arity,
        object_arity,
    )
    .unwrap_or_else(|e| panic!("harness relation table <{TABLE_NS}{head}>: {e}"))
}

/// The term a table's head IRI denotes in the fixture graph.
fn table_head(local: &str) -> TermValue {
    TermValue::iri(format!("{TABLE_NS}{local}"))
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
    use purrdf_entail::{Materialization, Regime};
    // A `qt:constructDataFile` action contributes one more SOURCE DOCUMENT — the
    // serialization of a CONSTRUCT result graph — merged alongside any `qt:data`
    // and standardized apart from it exactly as two files are (see
    // [`build_dataset`]).
    let mut sources = parse_sources(&case.base, &case.data, &case.graph_data)?;
    if let Some(construct) = &case.construct_data {
        sources.push((construct_data_document(case, construct)?, None));
    }
    let ds = freeze_sources(&sources, &case.graph_data)?;
    // For a rule-table lane, close the dataset before it is queried (the eval loop is
    // untouched — it queries an already-reasoned dataset). `OWL-Direct` and `RIF` are
    // handled by the CALLER (the `QueryEval` arm), which has the two inputs this
    // function does not: the query's basic graph pattern and the manifest's `.rif`
    // documents. They therefore pass through raw here, as does `D` (unwired).
    let plan = match case.regime {
        Some(Regime::Simple) => Materialization::Simple,
        Some(Regime::Rdf) => Materialization::Rdf,
        Some(Regime::Rdfs) => Materialization::Rdfs,
        Some(Regime::OwlRl) => Materialization::OwlRl,
        _ => return Ok(ds),
    };
    // The reasoning report is bound and dropped: a conformance case's verdict is
    // "did the engine return the manifest's expected result", which the report
    // cannot change. It is bound rather than avoided because there is no
    // report-free `materialize` — the evidence is always produced, and a caller
    // that has no use for it says so at the call site.
    purrdf_entail::materialize(&ds, plan)
        .map(|(closure, _report)| closure)
        .map_err(|e| format!("entailment ({:?}) for {}: {e}", plan.regime(), case.iri))
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
/// against: `<case base><file name>`, which is exactly what the manifest's OWN
/// relative reference (e.g. `<exists-graph-variable.ttl>`) resolved to — the
/// vendored suite never nests fixtures in subdirectories, so a bare file name
/// round-trips that reference.
///
/// `base` is the DECLARING MANIFEST's base (see
/// [`crate::manifest::SparqlTestCase::base`]), never a harness-wide constant.
/// Resolving here against a different base than the loader used would give the
/// fixture a graph name no query could refer to: a `GRAPH <exists02.ttl> { … }`
/// would name one IRI and `qt:graphData <exists02.ttl>` another, and the pattern
/// would match nothing while the case reported a plain empty-result mismatch.
///
/// Using the base ALONE for every file (as opposed to appending the file's own
/// name) would make a bare `<>` inside the Turtle content resolve to the same IRI
/// for every fixture, instead of self-referencing that fixture's own
/// `qt:data`/`qt:graphData` graph name — the exact self-reference some W3C
/// fixtures (e.g. `exists-graph-variable`) depend on.
fn file_base_iri(base: &str, path: &std::path::Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    format!("{base}{name}")
}

/// Build a dataset from default-graph Turtle files (`data`) and named-graph files
/// (`graph_data`, each `(graph IRI, file)`). Shared by the query pre-state loader
/// and the UPDATE pre-/post-state builders.
///
/// `base` is the declaring manifest's own sentinel base (see
/// [`crate::manifest::SparqlTestCase::base`]) — see [`file_base_iri`] for why it
/// must be that one and not a harness-wide constant.
///
/// # Every source file is a DOCUMENT, and documents are standardized apart
///
/// A case may declare several `qt:data` / `qt:graphData` files. Each is a separate
/// RDF document with its own blank-node scope, so combining them is an RDF 1.1
/// **merge** (§4.1): the blank nodes of the sources must first be *standardized
/// apart*, and only then unioned. Two files that both write `_:b` name two
/// DIFFERENT nodes.
///
/// This function used to serialize each file to N-Quads, concatenate the text, and
/// re-parse the concatenation as ONE document. That is a union of texts, not a
/// merge of graphs: every source's `_:b` collapsed onto a single node. So each
/// source is now parsed on its own and merged in under its own
/// [`BlankScope`](purrdf_core::BlankScope) — scopes `1, 2, 3, …` in declaration
/// order, with scope 0 ([`BlankScope::DEFAULT`](purrdf_core::BlankScope::DEFAULT))
/// deliberately left unused so no source of a merged dataset can ever alias a
/// blank node minted by an un-scoped `push_owned_*` (or by a SPARQL query, whose
/// own document scope is likewise distinct).
///
/// One scope per source is also what keeps a bare `_:b` and a `_:b` embedded in a
/// `cdt:List` / `cdt:Map` lexical form in the SAME file bound to the SAME node:
/// [`intern_owned_term_scoped`](purrdf_core::RdfDatasetBuilder::intern_owned_term_scoped)
/// binds both through that one scope (see [`purrdf_core::cdt_blank`]).
///
/// Merging over the OWNED model (rather than over serialized text) also carries
/// the RDF 1.2 statement layer — reifier bindings and annotations — which the old
/// N-Quads round trip could only express by flattening.
///
/// # Errors
///
/// Returns a message on any read, parse, or freeze failure (never silent).
pub fn build_dataset(
    base: &str,
    data: &[std::path::PathBuf],
    graph_data: &[(String, std::path::PathBuf)],
) -> Result<Arc<RdfDataset>, String> {
    let sources = parse_sources(base, data, graph_data)?;
    freeze_sources(&sources, graph_data)
}

/// One parsed source document plus the named graph its rows are retagged into
/// (`None` = the default graph).
type Source = (Arc<RdfDataset>, Option<purrdf_core::RdfTerm>);

/// Parse each `qt:data` / `qt:graphData` file into its OWN dataset, in
/// declaration order. Separate from [`freeze_sources`] so a caller with a further
/// source of its own — the `qt:constructDataFile` document in [`load_dataset`] —
/// can add it before the merge and have it scoped like any other document.
fn parse_sources(
    base: &str,
    data: &[std::path::PathBuf],
    graph_data: &[(String, std::path::PathBuf)],
) -> Result<Vec<Source>, String> {
    let mut sources: Vec<Source> = Vec::with_capacity(data.len() + graph_data.len());

    for path in data {
        let chunk = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let ds = purrdf::parse_dataset(
            &chunk,
            data_media_type(path),
            Some(&file_base_iri(base, path)),
        )
        .map_err(|e| format!("parse data {}: {e}", path.display()))?;
        sources.push((ds, None));
    }

    for (graph_iri, path) in graph_data {
        let chunk = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        // Parse against the graph's OWN resolved IRI (not the shared harness
        // `BASE`) so a bare `<>` inside the file self-references this named
        // graph, exactly like a real per-file base would.
        let ds = purrdf::parse_dataset(&chunk, data_media_type(path), Some(graph_iri))
            .map_err(|e| format!("parse graph data {}: {e}", path.display()))?;
        sources.push((ds, Some(purrdf_core::RdfTerm::Iri(graph_iri.clone()))));
    }

    Ok(sources)
}

/// Merge every parsed source under its OWN [`BlankScope`](purrdf_core::BlankScope)
/// and freeze — the standardize-apart half of the merge documented on
/// [`build_dataset`].
fn freeze_sources(
    sources: &[Source],
    graph_data: &[(String, std::path::PathBuf)],
) -> Result<Arc<RdfDataset>, String> {
    let mut builder = purrdf_core::RdfDatasetBuilder::new();
    for (index, (source, graph)) in sources.iter().enumerate() {
        let ordinal = u32::try_from(index + 1)
            .map_err(|_| format!("more than {} source documents in one case", u32::MAX))?;
        merge_source(
            &mut builder,
            source,
            purrdf_core::BlankScope(ordinal),
            graph.as_ref(),
        );
    }
    // A `qt:graphData` file that parses to ZERO quads (e.g. `empty.ttl`) leaves
    // nothing behind to imply its graph, so the graph is declared explicitly —
    // RDF 1.1 §4's "an RDF dataset MAY include an empty named graph". Declaring
    // one that DOES own quads is a no-op, so every graph is declared uniformly
    // rather than only the empty ones.
    for (graph_iri, _) in graph_data {
        let g = builder.intern_iri(graph_iri);
        builder.declare_named_graph(g);
    }

    builder
        .freeze()
        .map_err(|e| format!("freeze merged case dataset: {e}"))
}

/// Produce the source document a `qt:constructDataFile` action names: run its
/// CONSTRUCT query, serialize the result graph in the declared media type, and
/// parse that serialization back.
///
/// The round trip through real syntax is the whole point of the action, so it is
/// performed literally: the graph is WRITTEN and RE-READ rather than handed on as
/// an in-memory dataset. A blank node occurring both as a term and inside a
/// `cdt:List` / `cdt:Map` lexical form only survives it if the serializer spells
/// both occurrences with the SAME identifier and the parser binds both back to
/// one node.
///
/// The CONSTRUCT runs against an EMPTY dataset: the action supplies no data of its
/// own, and the graph it names is meant to be built by the query alone (the
/// SEP-0009 cases mint theirs with `BNODE()`). A query that needed data would name
/// it with `qt:data`, which composes.
///
/// # Errors
///
/// Returns a message if the query cannot be read, parsed, or evaluated, if it is
/// not a graph-producing form, or if the graph cannot be written to / read back
/// from the declared media type.
fn construct_data_document(
    case: &SparqlTestCase,
    construct: &crate::manifest::ConstructDataFile,
) -> Result<Arc<RdfDataset>, String> {
    let query_text = std::fs::read_to_string(&construct.query).map_err(|e| {
        format!(
            "read constructDataFile query {}: {e}",
            construct.query.display()
        )
    })?;
    let empty = purrdf_core::RdfDatasetBuilder::new()
        .freeze()
        .map_err(|e| format!("freeze the empty constructDataFile input dataset: {e}"))?;
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            &empty,
            SparqlRequest {
                query: &query_text,
                base_iri: Some(&case.base),
                substitutions: &[],
            },
        )
        .map_err(|e| {
            format!(
                "evaluate constructDataFile query {}: {e}",
                construct.query.display()
            )
        })?;
    let SparqlResult::Graph(graph) = result else {
        return Err(format!(
            "constructDataFile query {} is not a CONSTRUCT/DESCRIBE — a \
             qt:constructDataFile action names the query whose RESULT GRAPH becomes the \
             case's data, so a query form that produces no graph cannot serve it",
            construct.query.display()
        ));
    };
    let bytes =
        serialize_dataset(&graph, &construct.format, SerializeGraph::Dataset).map_err(|e| {
            format!(
                "serialize the {} result graph as {}: {e}",
                construct.query.display(),
                construct.format
            )
        })?;
    purrdf::parse_dataset(&bytes, &construct.format, Some(&case.base)).map_err(|e| {
        format!(
            "re-parse the {} result graph from {}: {e}",
            construct.query.display(),
            construct.format
        )
    })
}

/// Merge one parsed source document into `builder` under `scope`, optionally
/// retagging every row into the named graph `graph`.
///
/// `scope` is this document's own [`BlankScope`](purrdf_core::BlankScope): every
/// blank node it names — written as a term, or embedded in a `cdt:List` /
/// `cdt:Map` lexical form — is interned under it, which is what standardizes this
/// document apart from its siblings while keeping its own co-references intact.
///
/// The whole RDF 1.2 statement layer travels: base quads, reifier bindings and
/// annotations alike, each with its own graph slot retagged.
fn merge_source(
    builder: &mut purrdf_core::RdfDatasetBuilder,
    source: &RdfDataset,
    scope: purrdf_core::BlankScope,
    graph: Option<&purrdf_core::RdfTerm>,
) {
    for mut quad in source.owned_quads() {
        if let Some(g) = graph {
            quad.graph_name = Some(g.clone());
        }
        builder.push_owned_quad_scoped(&quad, scope);
    }
    for mut reifier in source.owned_reifiers() {
        if let Some(g) = graph {
            reifier.graph = Some(g.clone());
        }
        builder.push_owned_reifier_scoped(&reifier, scope);
    }
    for mut annotation in source.owned_annotations() {
        if let Some(g) = graph {
            annotation.graph = Some(g.clone());
        }
        builder.push_owned_annotation_scoped(&annotation, scope);
    }
    // A source being retagged into ONE named graph contributes no graph names of
    // its own — they have all been replaced by `graph`, which the caller declares.
    // A default-graph source (a `.trig` / `.nq` fixture) keeps its own.
    if graph.is_none() {
        for named in source.owned_named_graphs() {
            let g = builder.intern_owned_term_scoped(&named, scope);
            builder.declare_named_graph(g);
        }
    }
}

/// Run `case`, optionally resolving `SERVICE` clauses through `remote`.
///
/// # Errors
///
/// Returns a message on a read/parse/evaluation failure (the harness decides
/// whether that is an expected failure).
pub fn run(
    case: &SparqlTestCase,
    remote: Option<&(dyn ServiceResolver + Sync)>,
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
                .with_base_iri(file_base_iri(&case.base, &case.query))
                .parse_query(&query_text)
                .is_ok();
            Ok(RunOutcome::Syntax { parsed_ok })
        }
        TestKind::PositiveUpdateSyntax | TestKind::NegativeUpdateSyntax => {
            let parsed_ok = SparqlParser::new()
                .with_base_iri(file_base_iri(&case.base, &case.query))
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
                let bgp = collect_query_bgp(&case.base, &query_text);
                dataset = purrdf_entail::materialize_dl_reported(&dataset, &bgp)
                    .map_err(|e| format!("OWL-Direct entailment for {}: {e}", case.iri))?
                    .0;
            }
            // RIF entailment: the qt:data graph references one or more `.rif`
            // documents via `rif:usedWithProfile`; parse each (plus its RDF
            // imports) into a Horn rule set, forward-chain it over the RAW dataset,
            // then hand the materialized dataset to the UNMODIFIED engine.
            if case.regime == Some(purrdf_entail::Regime::Rif) {
                let ruleset = build_rif_ruleset(case, &dataset)?;
                dataset = purrdf_entail::materialize_rif(&dataset, &ruleset)
                    .map_err(|e| format!("RIF entailment for {}: {e}", case.iri))?
                    .0;
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
                property_fn_namespaces: vec![REL_NS.to_owned()],
                property_fn_iris: Vec::new(),
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
                base_iri: Some(&case.base),
                substitutions: &[],
            };
            // `purrdf:aggregateNamespace` (see `crate::manifest::SparqlTestCase`) is
            // PER-CASE, unlike `EXT_NS`/`REL_NS`: unlike the property-function/
            // extension-function namespaces (recognized only when the query text
            // actually calls one), `AggregateRegistry::register_statistical_aggregates`
            // registers ten IRIs unconditionally under its namespace, so registering it
            // harness-wide would change the answer of any OTHER case (including a
            // vendored W3C fixture) whose query happens to call `AGG(<iri>, …)` under
            // that same namespace. Opt-in per case keeps every other case byte-for-byte
            // unaffected — `None` here is the untouched behavior from before this field
            // existed.
            let aggregates = case.aggregate_namespace.as_ref().map(|namespace| {
                let mut registry = purrdf_sparql_eval::AggregateRegistry::new();
                registry.register_statistical_aggregates(namespace);
                registry
            });
            // A federated case resolves `SERVICE` through the injected source; every case
            // (federated or not) carries the harness relation table for its OUTER
            // pattern — a call node inside a `SERVICE` body is refused at forwarding
            // regardless, so handing `query_with_source` the registry only extends what
            // the query's own top-level patterns can reach. The vendored federated
            // fixtures happen to spell no `REL_NS` predicate today, but the registry
            // costs nothing to carry and keeps the two branches from silently
            // disagreeing about which predicates are calls.
            let empty_aggregates = purrdf_sparql_eval::AggregateRegistry::EMPTY;
            let options = QueryOptions {
                property_functions: harness_relations(),
                aggregates: aggregates.as_ref().unwrap_or(&empty_aggregates),
                ..QueryOptions::EMPTY
            };
            let result = match remote {
                Some(source) => engine.query_with_source(&dataset, request, source, options),
                None => engine.query_with_options_view(&*dataset, request, options),
            }
            .map_err(|e| format!("evaluate {}: {e}", case.iri))?;
            let ordered = query_is_top_level_ordered(&query_text, &parser_options);
            Ok(RunOutcome::Eval { result, ordered })
        }
        TestKind::UpdateEval => {
            // Apply the `ut:request` update to the pre-state dataset; the mutated
            // dataset is diffed against the expected post-state in `compare`.
            let mut dataset = build_dataset(&case.base, &case.data, &case.graph_data)?;
            let engine = NativeSparqlEngine::new().with_parser_options(ParserOptions {
                extension_fn_namespaces: vec![EXT_NS.to_owned()],
                property_fn_namespaces: vec![REL_NS.to_owned()],
                property_fn_iris: Vec::new(),
            });
            let request = SparqlRequest {
                query: &query_text,
                base_iri: Some(&case.base),
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
fn collect_query_bgp(base: &str, query_text: &str) -> Vec<QTriple> {
    let Ok(query) = SparqlParser::new()
        .with_base_iri(base)
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
        // `UNFOLD` expands a composite value the solution already carries and
        // matches no triple in any graph, so it is transparent to this walk.
        | GraphPattern::Unfold { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => collect_bgp(inner, out),
        // Leaves that hold no triple pattern. A property-function call matches no
        // triple in any graph — its rows come from the injected relation table — so
        // it scaffolds no class expression for the OWL-Direct augmentation, exactly
        // as a path or an inline `VALUES` scaffolds none.
        GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_) => {}
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
