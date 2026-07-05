// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL Core validation engine.
//!
//! `validate` is the top-level entry point.  Resolves focus nodes for every
//! non-deactivated node shape, runs all constraints, and assembles a
//! deterministically-sorted [`ValidationReport`].

use std::sync::Arc;

use ::purrdf::{FastMap, FastSet, IdSet, RdfDataset, RdfDatasetBuilder, RdfTerm, TermId};

use crate::data::{quads_for_pattern_ids, resolve_id, GraphFilter, ShaclData};
use crate::model::{rdf, rdfs};
use crate::report::ValidationReport;
use crate::shapes::{Shape, Shapes, Target};
use crate::term::{term_id_to_native, NamedNode, Term};

// ── Target resolution helpers ─────────────────────────────────────────────────

/// Resolve a predicate IRI to its interned id in the Core dataset, if present.
#[inline]
fn resolve_pred(ds: &RdfDataset, pred: &NamedNode) -> Option<TermId> {
    resolve_id(ds, &Term::NamedNode(pred.clone()))
}

/// Collect distinct subjects of `(?, pred, ?)` across all graphs. Dedup is on the
/// interned [`TermId`] (`Copy`); only distinct subjects are resolved to a term.
fn subjects_of(ds: &RdfDataset, pred: &NamedNode) -> Vec<Term> {
    let Some(pid) = resolve_pred(ds, pred) else {
        return Vec::new();
    };
    let mut seen: IdSet = IdSet::default();
    let mut result = Vec::new();
    for q in quads_for_pattern_ids(ds, None, Some(pid), None, GraphFilter::AnyGraph) {
        if seen.insert(q.s) {
            result.push(term_id_to_native(ds, q.s));
        }
    }
    result
}

/// Collect distinct objects of `(?, pred, ?)` across all graphs. Dedup is on the
/// interned [`TermId`] (`Copy`); only distinct objects are resolved to a term.
fn objects_of(ds: &RdfDataset, pred: &NamedNode) -> Vec<Term> {
    let Some(pid) = resolve_pred(ds, pred) else {
        return Vec::new();
    };
    let mut seen: IdSet = IdSet::default();
    let mut result = Vec::new();
    for q in quads_for_pattern_ids(ds, None, Some(pid), None, GraphFilter::AnyGraph) {
        if seen.insert(q.o) {
            result.push(term_id_to_native(ds, q.o));
        }
    }
    result
}

/// The transitive closure of asserted `rdfs:subClassOf` at or below `class_iri`,
/// as interned [`TermId`]s: the set containing `class_iri` itself plus every class
/// that is a (transitive) subclass of it via `rdfs:subClassOf` triples **asserted
/// in the data graph**.
///
/// This implements SHACL class-membership semantics (§4.2.5), which honor the
/// subclass relationships present in the data. It is NOT OWL/RDFS inference: we
/// read `rdfs:subClassOf` triples that exist and materialize nothing. (The
/// "no-inference contract" means no reasoner is run, not that asserted subclass
/// edges are ignored.) See the issue tracker.
///
/// A class IRI not interned in `ds` (mentioned by no triple) yields an empty set:
/// nothing can be typed to it, so it has no SHACL instances.
pub(crate) fn subclass_closure(ds: &RdfDataset, class_iri: &NamedNode) -> IdSet {
    let mut closure: IdSet = IdSet::default();
    let Some(start) = resolve_id(ds, &Term::NamedNode(class_iri.clone())) else {
        return closure;
    };
    closure.insert(start);
    let Some(sco) = resolve_id(ds, &Term::NamedNode(NamedNode::from(rdfs::SUB_CLASS_OF))) else {
        // No `rdfs:subClassOf` edges in the data: the closure is just the class.
        return closure;
    };
    let mut frontier = vec![start];
    while let Some(superclass) = frontier.pop() {
        // Any X with `X rdfs:subClassOf superclass` is a subclass to descend into.
        for q in quads_for_pattern_ids(ds, None, Some(sco), Some(superclass), GraphFilter::AnyGraph)
        {
            if closure.insert(q.s) {
                frontier.push(q.s);
            }
        }
    }
    closure
}

/// Collect subjects that are SHACL instances of `class_iri`: nodes with an
/// `rdf:type` to `class_iri` or to any asserted (transitive) subclass of it.
///
/// `closure_memo` is a per-`validate_with` call cache keyed by class IRI; the
/// subclass BFS is performed at most once per distinct class across all shapes.
fn instances_of_class(
    ds: &RdfDataset,
    class_iri: &NamedNode,
    closure_memo: &mut FastMap<NamedNode, IdSet>,
) -> Vec<Term> {
    let Some(rdf_type) = resolve_id(ds, &Term::NamedNode(NamedNode::from(rdf::TYPE))) else {
        return Vec::new();
    };
    // Compute the subclass closure at most once per class IRI; clone the key only
    // on a memo miss (insert requires ownership), never on a hit.
    if !closure_memo.contains_key(class_iri) {
        let closure = subclass_closure(ds, class_iri);
        closure_memo.insert(class_iri.clone(), closure);
    }
    let classes = &closure_memo[class_iri];
    let mut seen: IdSet = IdSet::default();
    let mut result = Vec::new();
    // Iterate the memoized id set by reference — never clone the whole set per call.
    for &class in classes {
        for q in quads_for_pattern_ids(ds, None, Some(rdf_type), Some(class), GraphFilter::AnyGraph)
        {
            if seen.insert(q.s) {
                result.push(term_id_to_native(ds, q.s));
            }
        }
    }
    result
}

/// Resolve the focus node set for a single shape from its target declarations.
///
/// Results are deduped by `Term` identity and sorted for a stable order.
///
/// `closure_memo` is threaded through to [`instances_of_class`] so the subclass
/// BFS is performed at most once per class IRI per `validate_with` call.
pub(crate) fn resolve_focus_nodes(
    data: &ShaclData,
    targets: &[Target],
    closure_memo: &mut FastMap<NamedNode, IdSet>,
) -> Result<Vec<Term>, String> {
    let ds = data.core();
    let mut seen: FastSet<Term> = FastSet::default();
    let mut nodes: Vec<Term> = Vec::new();

    for target in targets {
        let candidates: Vec<Term> = match target {
            Target::Class(class_iri) => instances_of_class(ds, class_iri, closure_memo),
            Target::SubjectsOf(pred) => subjects_of(ds, pred),
            Target::ObjectsOf(pred) => objects_of(ds, pred),
            Target::Node(t) => vec![t.clone()],
            Target::ImplicitClass(t) => {
                // Same semantics as Class: subjects of (?, rdf:type, t)
                if let Term::NamedNode(nn) = t {
                    instances_of_class(ds, nn, closure_memo)
                } else {
                    vec![]
                }
            }
            // sh:SPARQLTarget: execute the pre-parsed SELECT and collect ?this over
            // the native SPARQL engine, reading the IR dataset directly.
            // SELECT-form is enforced at shape-load (shapes.rs rejects
            // non-SELECT); a residual evaluation failure is surfaced as a hard
            // validation error rather than a panic.
            Target::Sparql {
                select,
                substitutions,
            } => crate::sparql::eval_target(data.sparql(), select, substitutions)
                .map_err(|e| format!("sh:target SPARQLTarget failed: {e}"))?,
        };
        for node in candidates {
            if seen.insert(node.clone()) {
                nodes.push(node);
            }
        }
    }

    // Sort for a stable, deterministic ordering across iterations.
    nodes.sort_by_cached_key(ToString::to_string);
    Ok(nodes)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Validate a [`ShaclData`] holder against `shapes`.
///
/// This is the single engine core; [`validate_dataset`] (the IR entry point)
/// bottoms out here.
///
/// For every non-deactivated node shape, the focus node set is resolved from the
/// shape's target declarations and each focus node is validated against the shape
/// via [`crate::constraints::validate_shape`]. Results are sorted by `(focus_node,
/// source_constraint_component, source_shape, result_path, value)` so reports are
/// identical across runs.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure: a SHACL-SPARQL target
/// or constraint query the engine cannot evaluate.
pub fn validate_with(data: &ShaclData, shapes: &Shapes) -> Result<ValidationReport, String> {
    validate_with_focus_filter(data, shapes, |_, _| true)
}

/// Validate with an explicit focus-node filter.
///
/// The filter is called after target resolution and before constraint evaluation.
/// It lets callers that already know only a bounded set of focus nodes changed
/// avoid rechecking the clean base graph, while still resolving targets against
/// the full data graph.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure (see [`validate_with`]).
pub fn validate_with_focus_filter<F>(
    data: &ShaclData,
    shapes: &Shapes,
    mut include_focus: F,
) -> Result<ValidationReport, String>
where
    F: FnMut(&Shape, &Term) -> bool,
{
    // Install the shapes graph's SHACL-AF function table (`sh:SPARQLFunction`) for
    // this whole validation pass. Held on the serial validation thread so every
    // `sh:sparql`/`sh:SPARQLTarget`/`sh:expression` query resolves declared user
    // functions; the guard restores the previous table on drop.
    let _function_scope = crate::sparql::enter_function_scope(Arc::clone(&shapes.functions));

    let mut all_results = Vec::new();
    // Per-call subclass-closure memo: keyed by class IRI, value is the full
    // transitive closure (interned ids) of asserted rdfs:subClassOf edges below
    // that class. Shared across all shapes in this validation run; each distinct
    // class IRI is BFS-walked AT MOST ONCE regardless of how many shapes target it.
    let mut closure_memo: FastMap<NamedNode, IdSet> = FastMap::default();

    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }

        let focus_nodes = resolve_focus_nodes(data, &shape.targets, &mut closure_memo)?;

        // Per-focus constraint evaluation stays SERIAL. A rayon `par_iter` over the
        // focus loop was measured on `shacl_validate/large_hierarchy` (3000 focus
        // nodes) and REGRESSED ~9% (15.71 ms → 16.43 ms), confirming that per-focus
        // work (~5 µs: an rdfs:subClassOf BFS-backed lookup + a `sh:pattern` regex)
        // is dwarfed by thread-pool dispatch and shared-`Store` read contention. The
        // frozen `RdfDataset` is `Sync`, so the seam stays ready, but the
        // parallel path waits on the re-entry condition: per-focus cost >50–100 µs,
        // i.e. once SHACL-SPARQL constraints are common or the IR-native backend runs
        // end-to-end (dropping the shared-`Store` contention). See the issue tracker (item 2).
        for focus in &focus_nodes {
            if !include_focus(shape, focus) {
                continue;
            }
            all_results.extend(crate::constraints::validate_shape_with(
                data,
                focus,
                shape,
                shapes.box_role_vocab.as_ref(),
            )?);
        }
    }

    // Deterministic sort key: (focus_node, component, source_shape, path, value,
    // message, severity). The message and severity tiebreakers make the ordering
    // TOTAL: two results that agree on the first five components (e.g. several
    // `sh:uniqueLang` violations on one focus, which differ only in their message
    // text) would otherwise keep their push order, which is a `FastMap`/`FastSet`
    // iteration order and thus not guaranteed stable across ahash versions or
    // targets. Sorting on the full serialized identity closes that leak so report
    // bytes are invariant under data-insertion order and platform.
    let sort_key = |r: &crate::report::ValidationResult| {
        (
            r.focus_node.to_string(),
            r.source_constraint_component.to_string(),
            r.source_shape.to_string(),
            r.result_path
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            r.value
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            r.message.clone().unwrap_or_default(),
            r.severity.clone(),
        )
    };
    all_results.sort_by_cached_key(sort_key);

    let conforms = all_results.is_empty();

    Ok(ValidationReport {
        conforms,
        results: all_results,
    })
}

/// Validate a frozen [`::purrdf::RdfDataset`] against parsed SHACL shapes, IR-natively.
///
/// The generic engine reads pattern lookups DIRECTLY from a SHACL projection of
/// the IR — there is no whole-store oxigraph materialization on this path
/// (SHACL-SPARQL constraints, if any, lazily materialize a query store on demand
/// only). Named graphs are flattened so GTS bundle partitions behave like the
/// repository's Turtle source merge, which loads all inputs into one default graph.
///
/// # Errors
///
/// Returns an error string if the SHACL projection cannot be frozen into the IR.
pub fn validate_dataset(data: &RdfDataset, shapes: &Shapes) -> Result<ValidationReport, String> {
    let dataset = project_dataset(data)?;
    // The engine reads pattern lookups directly from the frozen IR; SHACL-SPARQL
    // paths run the native SPARQL engine over the same `Arc<RdfDataset>`.
    validate_projected_dataset(dataset, shapes)
}

/// Validate an already-SHACL-projected dataset.
///
/// Call [`project_dataset`] first when the same base graph is reused across many
/// overlays; this avoids flattening/reifier-projecting the base graph on every
/// validation pass.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure (see [`validate_with`]).
pub fn validate_projected_dataset(
    projected: Arc<RdfDataset>,
    shapes: &Shapes,
) -> Result<ValidationReport, String> {
    // Core lookups and the SHACL-SPARQL paths run over the same `Arc<RdfDataset>`.
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    validate_with(&data, shapes)
}

/// Validate an already-SHACL-projected dataset with a focus-node filter.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure (see [`validate_with`]).
pub fn validate_projected_dataset_with_focus_filter<F>(
    projected: Arc<RdfDataset>,
    shapes: &Shapes,
    include_focus: F,
) -> Result<ValidationReport, String>
where
    F: FnMut(&Shape, &Term) -> bool,
{
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    validate_with_focus_filter(&data, shapes, include_focus)
}

/// Build the dataset exposed to SHACL-SPARQL paths.
///
/// Data quads stay in their original graphs (the projected default graph). When
/// a shapes-graph IRI is known, every quad from [`Shapes::shapes_dataset`] is
/// placed into a named graph with that IRI.
///
/// Returns the combined dataset and the shapes-graph IRI actually used, if any.
fn build_sparql_dataset(
    data: Arc<RdfDataset>,
    shapes: &Shapes,
    override_graph: Option<&str>,
) -> Result<(Arc<RdfDataset>, Option<String>), String> {
    let graph_iri = override_graph
        .map(ToOwned::to_owned)
        .or_else(|| shapes.shapes_graph.clone());
    let Some(ref graph_iri) = graph_iri else {
        return Ok((data, None));
    };
    if shapes.shapes_dataset.quad_count() == 0 {
        return Ok((data, None));
    }

    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(data.as_ref());

    let graph_term = RdfTerm::iri(graph_iri);
    for mut quad in shapes.shapes_dataset.owned_quads() {
        quad.graph_name = Some(graph_term.clone());
        builder.push_owned_quad(&quad);
    }

    builder
        .freeze()
        .map_err(|e| e.to_string())
        .map(|ds| (ds, Some(graph_iri.clone())))
}

/// Validate a frozen [`RdfDataset`] against parsed SHACL shapes, exposing the
/// shapes graph as a named graph to SHACL-SPARQL paths.
///
/// `shapes_graph_iri` overrides [`Shapes::shapes_graph`] when both are present.
pub fn validate_dataset_with_shapes_graph(
    data: &RdfDataset,
    shapes: &Shapes,
    shapes_graph_iri: Option<&str>,
) -> Result<ValidationReport, String> {
    let projected = project_dataset(data)?;
    validate_projected_dataset_with_shapes_graph(projected, shapes, shapes_graph_iri)
}

/// Validate an already-projected dataset with a shapes-graph overlay.
pub fn validate_projected_dataset_with_shapes_graph(
    projected: Arc<RdfDataset>,
    shapes: &Shapes,
    shapes_graph_iri: Option<&str>,
) -> Result<ValidationReport, String> {
    let (sparql_dataset, shapes_graph_iri) =
        build_sparql_dataset(Arc::clone(&projected), shapes, shapes_graph_iri)?;
    // Core lookups read the projected data graph (default graph only); the SPARQL
    // paths see the combined data(+shapes) dataset under `shapes_graph_iri`.
    let data = ShaclData::new(projected, sparql_dataset, shapes_graph_iri);
    validate_with(&data, shapes)
}

/// Build a SHACL-projection dataset from the source [`RdfDataset`], flattening
/// every quad into the default graph and materializing reifier bindings as
/// `rdf:reifies` triples and statement annotations as plain triples.
pub fn project_dataset(data: &RdfDataset) -> Result<Arc<RdfDataset>, String> {
    use ::purrdf::RdfDatasetBuilder;
    use purrdf::{RdfQuad, RdfTerm};

    let mut builder = RdfDatasetBuilder::new();

    for mut quad in data.owned_quads() {
        // FlattenToDefaultGraph: drop the source graph name.
        quad.graph_name = None;
        builder.push_owned_quad(&quad);
    }

    // Reifiers → `(reifier, rdf:reifies, <<triple>>)` triples.
    for reifier in data.owned_reifiers() {
        builder.push_owned_quad(&RdfQuad::new(
            reifier.reifier,
            RDF_REIFIES,
            RdfTerm::triple(reifier.statement),
        ));
    }

    // Annotations → `(reifier, predicate, object)` triples.
    for annotation in data.owned_annotations() {
        builder.push_owned_quad(&RdfQuad::new(
            annotation.reifier,
            annotation.predicate,
            annotation.object,
        ));
    }

    builder.freeze().map_err(|e| e.to_string())
}

#[cfg(test)]
pub(crate) fn shacl_dataset_from_dataset(data: &RdfDataset) -> Result<Arc<RdfDataset>, String> {
    project_dataset(data)
}

/// The `rdf:reifies` predicate IRI, used to project reifier bindings into the
/// quad table so SHACL's reifier-shape lookups can find them.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Parse a SHACL shapes graph from a Turtle string.
///
/// Creates an in-memory store, loads the shapes graph with prefix extraction,
/// and parses it into a reusable [`Shapes`] model. The parsed model can be
/// shared across multiple data graphs via `validate`, eliminating the cost of
/// re-parsing shapes for every validation phase.
///
/// # Errors
///
/// Returns an error string if the shapes Turtle fails to parse or contains
/// unsupported SHACL constructs.
pub fn parse_shapes(shapes_ttl: &str) -> Result<Shapes, String> {
    parse_shapes_with_config(shapes_ttl, None)
}

/// [`parse_shapes`] with the caller-supplied [`BoxRoleVocab`](crate::model::BoxRoleVocab)
/// (`crate::model::BoxRoleVocab`) threaded through.
///
/// PurRDF mints no vocabulary IRIs, so the box-role annotation feature has no
/// default vocabulary: with `box_role_vocab = None` it is INACTIVE (shapes
/// parse fine, but no role annotations are collected or stamped).
///
/// # Errors
///
/// Returns an error string if the shapes Turtle fails to parse or contains
/// unsupported SHACL constructs.
pub fn parse_shapes_with_config(
    shapes_ttl: &str,
    box_role_vocab: Option<crate::model::BoxRoleVocab>,
) -> Result<Shapes, String> {
    // Parse the shapes graph via the native purrdf codecs — no
    // the oxigraph `io` parser. The native codec drops document prefixes once it folds to
    // the IR, so we recover the `@prefix`/SPARQL `PREFIX` map by scanning the
    // source text: SHACL-AF sh:select queries (and pySHACL) rely on prefixed
    // names. See the issue tracker. A syntax error is reported per-statement so
    // a SHACL author sees the full list in one pass (item 4), not the
    // fix-one-rerun-find-the-next loop.
    let shapes_dataset = crate::text_ingest::parse_turtle_to_dataset(shapes_ttl)
        .map_err(|errors| errors.join("\n"))?;
    let doc_prefixes = crate::text_ingest::extract_prefixes(shapes_ttl);

    crate::shapes::from_dataset_with_config(&shapes_dataset, &doc_prefixes, box_role_vocab)
}

/// Validate data (N-Triples) against shapes (Turtle), returning a [`ValidationReport`].
///
/// Creates an in-memory data store, loads the data graph, parses shapes once
/// via [`parse_shapes`], and delegates to `validate`.
///
/// The data graph is loaded with the **lenient** RDF parser. A validator must be
/// able to ingest the data graph before it can validate any shapes against it,
/// and RDF lexical well-formedness is a separate concern from SHACL conformance.
/// The purrdf ontology carries private-use `@x-purrdf-*` language tags whose
/// subtag exceeds BCP-47's 8-char limit (e.g. `@x-purrdf-afrikaans`); the strict
/// parser rejects the entire file on these, which would make the real ontology
/// un-validatable. Lenient parsing skips that check so the data ingests. See the issue tracker.
///
/// # Errors
///
/// Returns an error string if either graph fails to parse.
pub fn validate_graphs(data_nt: &str, shapes_ttl: &str) -> Result<ValidationReport, String> {
    validate_graphs_with_config(data_nt, shapes_ttl, None)
}

/// [`validate_graphs`] with the caller-supplied [`BoxRoleVocab`](crate::model::BoxRoleVocab)
/// (`crate::model::BoxRoleVocab`) threaded through to shape parsing and
/// validation. `None` leaves the box-role feature inactive.
///
/// # Errors
///
/// Returns an error string if either graph fails to parse.
pub fn validate_graphs_with_config(
    data_nt: &str,
    shapes_ttl: &str,
    box_role_vocab: Option<crate::model::BoxRoleVocab>,
) -> Result<ValidationReport, String> {
    // Parse the data graph via the native codecs. Every malformed
    // N-Triples line is reported in one pass — same multi-error contract as
    // `parse_shapes`. See the issue tracker (item 4).
    let data = crate::text_ingest::parse_ntriples_to_dataset(data_nt)
        .map_err(|errors| errors.join("\n"))?;

    let shapes = parse_shapes_with_config(shapes_ttl, box_role_vocab)?;
    validate_dataset(data.as_ref(), &shapes)
}

/// Validate a frozen [`::purrdf::RdfDataset`] against a Turtle SHACL shapes graph.
///
/// # Errors
///
/// Returns an error string if the shapes graph fails to parse or if the SHACL
/// projection cannot be frozen.
pub fn validate_dataset_graphs(
    data: &RdfDataset,
    shapes_ttl: &str,
) -> Result<ValidationReport, String> {
    let shapes = parse_shapes(shapes_ttl)?;
    validate_dataset(data, &shapes)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Severity;
    use crate::shapes::Shapes;

    const PREFIXES: &str = r"
        @prefix sh:   <http://www.w3.org/ns/shacl#> .
        @prefix ex:   <http://example.org/ns#> .
        @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
    ";

    fn load_data_nt(nt: &str) -> Arc<RdfDataset> {
        crate::text_ingest::parse_ntriples_to_dataset(nt).expect("data N-Triples must parse")
    }

    fn load_shapes_ttl(ttl: &str) -> Shapes {
        let dataset =
            crate::text_ingest::parse_turtle_to_dataset(ttl).expect("shapes Turtle must parse");
        crate::shapes::from_dataset(&dataset).expect("shapes parse must succeed")
    }

    /// Validate native: the in-crate tests historically called `validate(&store, …)`;
    /// route them through the IR entrypoint, which is the only engine path now.
    fn validate(data: &Arc<RdfDataset>, shapes: &Shapes) -> ValidationReport {
        validate_dataset(data.as_ref(), shapes).expect("validate_dataset must succeed")
    }

    // ── Multi-error syntax reporting (item 4) ─────────────────────────────

    #[test]
    fn parse_shapes_reports_all_syntax_errors() {
        // Two independently-malformed Turtle STATEMENTS, separated by a valid one.
        // oxttl recovers at statement granularity (resync on the `.` terminator),
        // so BOTH errors must surface in one report — proving the accumulator is
        // real, not a one-element surface. (A lexer-level break such as an
        // unterminated string literal instead consumes to EOF and yields a single
        // error; that is correct, not a regression. The recoverable case below is
        // what proves multi-error reporting works.) If this regresses to a single
        // error on recoverable input, item 4's premise has broken.
        let bad = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "ex:a ex:p .\n",                // missing object → recoverable error
            "ex:b ex:q ex:c .\n",           // valid, between the two errors
            "ex:d ex:r ex:s ex:t ex:u .\n", // too many terms → recoverable error
        );
        let err = parse_shapes(bad).expect_err("malformed Turtle must error");
        let n = err.matches("Turtle parse error").count();
        assert!(
            n >= 2,
            "expected >=2 accumulated Turtle errors, got {n}:\n{err}"
        );
    }

    #[test]
    fn validate_graphs_reports_all_data_syntax_errors() {
        // Multiple malformed N-Triples lines must all be reported in one pass
        // rather than short-circuiting on the first (the single-error
        // load-into-store behavior item 4 replaced).
        let bad_data = concat!(
            "this is not a triple\n",
            "<http://example.org/s> <http://example.org/p> .\n",
            "neither is this\n",
        );
        let err = validate_graphs(bad_data, "").expect_err("malformed N-Triples must error");
        let n = err.matches("N-Triples parse error").count();
        assert!(
            n >= 2,
            "expected >=2 accumulated N-Triples errors, got {n}:\n{err}"
        );
    }

    #[test]
    fn parse_shapes_clean_input_still_succeeds() {
        // The accumulator must not turn a well-formed document into a failure.
        let ok = format!("{PREFIXES}\nex:Shape a sh:NodeShape ; sh:targetClass ex:Thing .\n");
        parse_shapes(&ok).expect("well-formed shapes must parse");
    }

    // ── Pre-existing tests ─────────────────────────────────────────────────────

    #[test]
    fn empty_inputs_return_conforming_report() {
        let report = validate_graphs("", "").expect("empty inputs must not error");
        assert!(report.conforms, "empty report must conform");
        assert!(
            report.results.is_empty(),
            "empty report must have no results"
        );
    }

    #[test]
    fn dataset_entrypoint_validates_gts_backed_graph() {
        use ::purrdf::RdfDatasetBuilder;
        let mut builder = RdfDatasetBuilder::new();
        let ids: Vec<_> = [
            "http://example.org/ns#a",
            "http://example.org/ns#p",
            "http://example.org/ns#b",
        ]
        .into_iter()
        .map(|value| builder.intern_iri(value))
        .collect();
        builder.push_quad(ids[0], ids[1], ids[2], None);
        let dataset = builder.freeze().expect("valid test dataset");

        let shapes_ttl = format!(
            "{PREFIXES}
            ex:Shape a sh:NodeShape ;
                sh:targetNode ex:a ;
                sh:property [
                    sh:path ex:missing ;
                    sh:minCount 1 ;
                ] ."
        );
        let report = validate_dataset_graphs(dataset.as_ref(), &shapes_ttl)
            .expect("GTS-backed store should validate");
        assert!(!report.conforms, "missing property must violate the shape");
        assert_eq!(report.results.len(), 1);
    }

    #[test]
    fn validate_stub_always_conforms() {
        let data = load_data_nt("");
        let shapes = Shapes::default();
        let report = validate(&data, &shapes);
        assert!(report.conforms);
        assert!(report.results.is_empty());
    }

    // ── Task 4 tests ───────────────────────────────────────────────────────────

    // Test 1: targetClass + minCount — violating case (no ex:name on ex:alice)
    #[test]
    fn target_class_min_count_violating() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:alice is a Person but has no ex:name
        let data_nt = "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(!report.conforms, "must NOT conform: alice has no ex:name");
        assert_eq!(report.results.len(), 1, "exactly one result expected");
        let r = &report.results[0];
        assert!(
            r.source_constraint_component.as_str().contains("MinCount"),
            "component must be MinCountConstraintComponent, got {}",
            r.source_constraint_component.as_str()
        );
        assert_eq!(
            r.focus_node.to_string(),
            "<http://example.org/ns#alice>",
            "focus node must be ex:alice"
        );
    }

    // Test 2: conforming case — adding ex:name makes it pass
    #[test]
    fn target_class_min_count_conforming() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        let data_nt = concat!(
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n"
        );

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(report.conforms, "must conform: alice now has ex:name");
        assert!(report.results.is_empty(), "zero results expected");
    }

    // Test 3a: targetSubjectsOf — shape targets subjects of ex:knows
    #[test]
    fn target_subjects_of() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:KnowerShape a sh:NodeShape ;
                sh:targetSubjectsOf ex:knows ;
                sh:property [
                    sh:path ex:label ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:alice knows ex:bob, but alice has no ex:label
        let data_nt = "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#bob> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            !report.conforms,
            "alice (subject of knows) must be a focus node and fail"
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node.to_string(),
            "<http://example.org/ns#alice>"
        );
    }

    // Test 3b: targetObjectsOf — shape targets objects of ex:knows
    #[test]
    fn target_objects_of() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:KnownShape a sh:NodeShape ;
                sh:targetObjectsOf ex:knows ;
                sh:property [
                    sh:path ex:label ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:alice knows ex:bob, bob has no ex:label
        let data_nt = "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#bob> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            !report.conforms,
            "bob (object of knows) must be a focus node and fail"
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node.to_string(),
            "<http://example.org/ns#bob>"
        );
    }

    // Test 4: sh:targetClass honors ASSERTED rdfs:subClassOf (SHACL §4.2.5).
    // This is NOT OWL inference — the subclass edge is asserted in the data; we
    // read it, materialize nothing. (Inverted from the former no-subclass
    // contract; see the issue tracker.)
    #[test]
    fn target_class_honors_asserted_subclass() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:bob is typed ex:Employee, and ex:Employee rdfs:subClassOf ex:Person
        // is ASSERTED → bob is a SHACL instance of ex:Person → it is a focus node
        // and, lacking ex:name, violates sh:minCount.
        let data_nt = concat!(
            "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Employee> .\n",
            "<http://example.org/ns#Employee> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
        );

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            !report.conforms,
            "ex:bob IS a focus node via asserted Employee ⊑ Person; report: {report:?}"
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node.to_string(),
            "<http://example.org/ns#bob>"
        );
    }

    // Test 4b: a class with NO asserted subClassOf edge is not reached — we
    // honor asserted edges only, inventing none.
    #[test]
    fn target_class_unasserted_subclass_not_reached() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [ sh:path ex:name ; sh:minCount 1 ; ] .
            "
        );
        // ex:carol is an ex:Robot; no ex:Robot rdfs:subClassOf ex:Person triple
        // exists → carol is not a Person-instance → conforms.
        let data_nt = "<http://example.org/ns#carol> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Robot> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            report.conforms,
            "carol must NOT be reached without an asserted subClassOf edge; report: {report:?}"
        );
    }

    // Test 5: deactivated shape produces no results even with violating data
    #[test]
    fn deactivated_shape_produces_no_results() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:deactivated true ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // alice is a Person with no ex:name — would fail if shape were active
        let data_nt = "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            report.conforms,
            "deactivated shape must produce no results; report: {report:?}"
        );
        assert!(report.results.is_empty());
    }

    // Test 6: determinism — two runs on the same input yield identical results
    #[test]
    fn determinism_same_results_twice() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // Two persons, both missing ex:name, to get multiple results
        let data_nt = concat!(
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        );

        let data1 = load_data_nt(data_nt);
        let shapes1 = load_shapes_ttl(&shapes_ttl);
        let report1 = validate(&data1, &shapes1);

        let data2 = load_data_nt(data_nt);
        let shapes2 = load_shapes_ttl(&shapes_ttl);
        let report2 = validate(&data2, &shapes2);

        assert_eq!(report1.conforms, report2.conforms);
        assert_eq!(report1.results.len(), report2.results.len());

        // Compare result tuples in order (not just as a set) to confirm stable sort.
        let tuples1: Vec<_> = report1
            .results
            .iter()
            .map(|r| {
                (
                    r.focus_node.to_string(),
                    r.source_constraint_component.to_string(),
                    r.source_shape.to_string(),
                    r.result_path.as_ref().map(ToString::to_string),
                    r.value.as_ref().map(ToString::to_string),
                    r.severity.clone(),
                )
            })
            .collect();
        let tuples2: Vec<_> = report2
            .results
            .iter()
            .map(|r| {
                (
                    r.focus_node.to_string(),
                    r.source_constraint_component.to_string(),
                    r.source_shape.to_string(),
                    r.result_path.as_ref().map(ToString::to_string),
                    r.value.as_ref().map(ToString::to_string),
                    r.severity.clone(),
                )
            })
            .collect();

        assert_eq!(
            tuples1, tuples2,
            "result ordering must be identical across runs"
        );

        // Also verify to_ntriples() is identical
        assert_eq!(
            report1.to_ntriples(),
            report2.to_ntriples(),
            "N-Triples output must be identical across runs"
        );
    }

    // Bonus: targetNode explicit
    #[test]
    fn target_node_explicit() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:AliceShape a sh:NodeShape ;
                sh:targetNode ex:alice ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:alice explicitly targeted; no ex:name triple
        let data_nt = "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(!report.conforms, "ex:alice has no ex:name → must fail");
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node.to_string(),
            "<http://example.org/ns#alice>"
        );
    }

    // Severity-independence: a Warning result makes conforms=false
    #[test]
    fn warning_result_makes_report_non_conforming() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:WarnShape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:severity sh:Warning ;
                sh:property [
                    sh:path ex:label ;
                    sh:minCount 1 ;
                    sh:severity sh:Warning ;
                ] .
            "
        );
        let data_nt = "<http://example.org/ns#x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Thing> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        // SHACL: conforms is false if ANY result exists, regardless of severity
        assert!(
            !report.conforms,
            "Warning results must still make conforms=false"
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].severity, Severity::Warning);
    }

    // Determinism under permuted DATA-INSERTION order.
    //
    // Interning assigns each term a TermId in first-appearance order, so
    // permuting the N-Triples lines assigns different ids to the same terms and
    // reorders every id-keyed FastSet/FastMap and path frontier. The report must
    // still be byte-identical, proving no id/hash iteration order reaches output.
    // The fixture deliberately includes TWO sh:uniqueLang violations on one focus
    // (duplicate `@en` and `@fr`), which share every sort component except the
    // message — the case the message/severity tiebreaker in the engine sort
    // exists to make total.
    #[test]
    fn determinism_under_permuted_insertion_order() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:uniqueLang true ; ] .
            "
        );
        let ty = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
        let name = "<http://example.org/ns#name>";
        let person = "<http://example.org/ns#Person>";
        // alice: two duplicated languages (@en, @fr) → two uniqueLang violations
        // with identical sort keys differing only in message. bob/carol: plain
        // members exercising focus ordering. dave: no name → minCount violation.
        let lines = vec![
            format!("<http://example.org/ns#alice> {ty} {person} .\n"),
            format!("<http://example.org/ns#alice> {name} \"a\"@en .\n"),
            format!("<http://example.org/ns#alice> {name} \"b\"@en .\n"),
            format!("<http://example.org/ns#alice> {name} \"c\"@fr .\n"),
            format!("<http://example.org/ns#alice> {name} \"d\"@fr .\n"),
            format!("<http://example.org/ns#bob> {ty} {person} .\n"),
            format!("<http://example.org/ns#bob> {name} \"Bob\" .\n"),
            format!("<http://example.org/ns#carol> {ty} {person} .\n"),
            format!("<http://example.org/ns#carol> {name} \"Carol\" .\n"),
            format!("<http://example.org/ns#dave> {ty} {person} .\n"),
        ];

        let render = |ordered: &[String]| {
            let data_nt: String = ordered.concat();
            let data = load_data_nt(&data_nt);
            let shapes = load_shapes_ttl(&shapes_ttl);
            validate(&data, &shapes).to_ntriples()
        };

        let forward = render(&lines);

        let mut reversed = lines.clone();
        reversed.reverse();
        assert_eq!(
            forward,
            render(&reversed),
            "report must be byte-identical under reversed insertion order"
        );

        // A rotation (different id assignment again) must also match.
        let mut rotated = lines.clone();
        rotated.rotate_left(3);
        assert_eq!(
            forward,
            render(&rotated),
            "report must be byte-identical under rotated insertion order"
        );

        // Sanity: the fixture actually produced the equal-sort-key uniqueLang pair
        // plus other violations, so the tiebreaker is genuinely exercised.
        let data = load_data_nt(&lines.concat());
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);
        let unique_lang = report
            .results
            .iter()
            .filter(|r| {
                r.source_constraint_component
                    .as_str()
                    .contains("UniqueLang")
            })
            .count();
        assert_eq!(
            unique_lang, 2,
            "fixture must yield two uniqueLang violations (dup @en and @fr)"
        );
    }
}
