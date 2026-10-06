// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The mutable quad-store surface for the `purrdf` Python extension: the
//! SPARQL-capable `Store`, the canonicalization-capable `Dataset`, and the
//! `QuadIter` snapshot iterator they share.
//!
//! # Native backing
//!
//! `Store` wraps a copy-on-write [`MutableDataset`] over the
//! `purrdf-core` IR. Mutation (`add` / `remove`
//! / `load`) edits the COW delta; `query` freezes a snapshot and runs the native
//! `NativeSparqlEngine`; `update` runs the engine's COW-atomic UPDATE. The
//! `_store_capsule` hands `purrdf_shapes` / `purrdf_validate` a stable
//! `Arc<RdfDataset>` snapshot under the `c"purrdf-validation-dataset"` capsule name.

use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::ir::MutableDataset;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use super::canon::PyCanonicalizationAlgorithm;
use super::io::{
    PyRdfFormat, dataset_from_quads_verbatim, declare_loaded_graphs, parse_quads_and_prefixes,
    read_input,
};
use super::presentation;
use super::quad_store::PyQuadStore;
use super::query::{
    EngineConfig, build_engine, build_relations, collect_relations, engine_parser_options,
};
use super::term::{PyQuad, extract_graph_name, rdf_quad_to_values, rdf_quad_to_values_scoped};
use crate::py_jsonld::{PyCompiledJsonLdContext, options_from_inputs};
use crate::py_store::iri_value_error;
use crate::{
    BlankScope, DatasetMut, RdfQuad, RdfTerm, SerializeGraph, SerializeOptions, StatementLayer,
    serialize_dataset_with,
};

/// An in-memory RDF 1.2 quad store with SPARQL (`Store`).
///
/// The query, UPDATE, iteration and validation-capsule surface is the
/// [`PyQuadStore`] base class's, shared with `MutableDataset`.
/// The keyword-only `remember_empty_graphs=True` selects native remembered
/// graph slots; the 3.x default is false. Checkpoint and UPDATE retain the choice.
#[pyclass(name = "Store", extends = PyQuadStore)]
#[derive(Debug)]
pub struct PyStore {
    /// Monotonic per-load counter that isolates blank-node label scopes across
    /// separate [`load`](PyStore::load) calls (see [`load`](PyStore::load) for why).
    next_load_scope: AtomicU64,
}

#[pymethods]
impl PyStore {
    #[new]
    #[pyo3(signature = (*, remember_empty_graphs=false))]
    fn new(remember_empty_graphs: bool) -> PyResult<PyClassInitializer<Self>> {
        Ok(
            PyClassInitializer::from(PyQuadStore::empty(remember_empty_graphs)?).add_subclass(
                Self {
                    next_load_scope: AtomicU64::new(1),
                },
            ),
        )
    }

    /// Load RDF into the store. Either `input` (bytes/str data) or the keyword
    /// `path` (a file to read) must be given, together with `format`.
    ///
    /// Returns the document's prefix map, from the same parse: the `@prefix` /
    /// `PREFIX` bindings a Turtle or TriG document left in force at its end, as
    /// `(prefix, namespace)` pairs sorted by prefix, each namespace resolved. Empty for
    /// every other format.
    #[pyo3(signature = (input=None, format=None, *, path=None, base=None))]
    fn load(
        mut slf: PyRefMut<'_, Self>,
        py: Python<'_>,
        input: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        path: Option<String>,
        base: Option<String>,
    ) -> PyResult<Vec<(String, String)>> {
        let format = format.ok_or_else(|| PyValueError::new_err("load: format is required"))?;
        let data = read_input(input, path)?;
        // Parse natively into the flat quad stream, then insert into the COW set.
        //
        // Blank-node labels in a serialized document are document-local: two distinct
        // documents may reuse the same label (`_:b0`) for *different* nodes, and the same
        // store loaded from many files must keep those distinct: each load call gets a fresh
        // blank scope. The native codec preserves labels verbatim, so we provide that
        // isolation by tagging every parsed blank node's label with a per-load-call-unique `BlankScope` before insertion.
        // `parse` / `parse_quads` keep labels verbatim — that path round-trips a single
        // document, where verbatim labels are correct and canonicalization needs them.
        let scope = BlankScope(slf.next_load_scope() as u32);
        let inner = &mut slf.as_super().inner;
        // Parse + insert run detached (GIL released); the closure only touches
        // plain Rust data.
        py.detach(move || {
            let base_ref = base.as_deref();
            let (quads, prefixes, declared) =
                parse_quads_and_prefixes(&data, format.to_native(), base_ref)
                    .map_err(|e| PyValueError::new_err(format!("load error: {e}")))?;
            for quad in quads {
                inner
                    .insert(rdf_quad_to_values_scoped(&quad, scope))
                    .map_err(|e| iri_value_error(&e))?;
            }
            declare_loaded_graphs(inner, declared, scope)?;
            Ok(prefixes)
        })
    }

    /// Alias of [`load`] — bulk loading is not a different semantics, so the
    /// in-memory store path is identical.
    #[pyo3(signature = (input=None, format=None, *, path=None, base=None))]
    fn bulk_load(
        slf: PyRefMut<'_, Self>,
        py: Python<'_>,
        input: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        path: Option<String>,
        base: Option<String>,
    ) -> PyResult<Vec<(String, String)>> {
        Self::load(slf, py, input, format, path, base)
    }

    /// Add a single quad.
    ///
    /// Raises `ValueError` if the quad carries a relative IRI in any position — its
    /// own terms, a literal's datatype, or one nested in a quoted triple. A `Store`
    /// mutated this way is handed terms, never a document, so there is no base in
    /// scope to resolve a relative reference against and PurRDF invents none. Resolve
    /// it yourself before adding it. The message leads with the shared
    /// `iri-relative-no-base` diagnostic code.
    fn add(mut slf: PyRefMut<'_, Self>, quad: &PyQuad) -> PyResult<()> {
        slf.as_super()
            .inner
            .insert(rdf_quad_to_values(&quad.inner))
            .map_err(|e| iri_value_error(&e))?;
        Ok(())
    }

    /// Remove a single quad. No-op if the quad is absent (matches the RDFLib
    /// `Graph.remove` contract, which silently ignores misses).
    fn remove(mut slf: PyRefMut<'_, Self>, quad: &PyQuad) -> PyResult<()> {
        slf.as_super()
            .inner
            .remove(&rdf_quad_to_values(&quad.inner));
        Ok(())
    }

    /// Fold everything mutated so far into this store's BASE, leaving the
    /// copy-on-write delta empty. The store's contents are unchanged.
    ///
    /// This is what makes "what did my last change break?" a question with an
    /// answer. A `Store` records mutations as a delta over a frozen base, and a
    /// freshly constructed store has an EMPTY base — so without this, the delta of
    /// a store you loaded a million triples into is those million triples, and
    /// `purrdf.shapes.PreparedShapes.validate_store_changes` would dutifully
    /// re-validate the whole graph. Checkpoint after loading, mutate, and the delta
    /// is exactly the mutation.
    ///
    /// Cheap to call once after a bulk load and expensive to call in a tight
    /// mutation loop: it is a real compaction (the COW base is rebuilt), which is
    /// why it is an explicit act rather than something `add` does behind your back.
    ///
    /// Raises `ValueError` if the store cannot be frozen.
    fn checkpoint(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<()> {
        let inner = &mut slf.as_super().inner;
        // A real compaction over the whole base — run it detached (GIL released).
        py.detach(move || {
            let base = inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store checkpoint failed: {e}")))?;
            *inner = MutableDataset::new_with_graph_existence(base, inner.graph_existence());
            Ok(())
        })
    }

    /// The number of quads this store has ADDED since the last
    /// [`checkpoint`](Self::checkpoint), and the number it has REMOVED, as a pair.
    ///
    /// The size of the change `validate_store_changes` expands, so a caller can see
    /// whether a checkpoint is due (or whether the mutation they believe they made
    /// actually landed) without validating anything.
    fn change_size(slf: &Bound<'_, Self>) -> (usize, usize) {
        let guard = slf.as_super().borrow();
        let inner = &guard.inner;
        (inner.added_len(), inner.suppressed_len())
    }

    /// Prepare a SPARQL query once, to be bound and run many times.
    ///
    /// `Store.query` parses and admits its text on every call. A caller running one
    /// query per row therefore pays that cost per row to be handed back the same
    /// plan — and a caller who instead splices the row's value into the text pays a
    /// real re-parse, because a spliced query is a different query. A prepared query
    /// is the alternative: the text is parsed and admitted here, and each run binds
    /// parameters and evaluates.
    ///
    /// `parameters` names the variables `run` will bind, without the `?`/`$` sigil.
    /// Each is pre-bound exactly as a `substitutions` entry is on
    /// [`query`](Self::query), so a parameter stays projectable and reaches inside
    /// `OPTIONAL`, `MINUS`, `EXISTS` and sub-`SELECT`s by ordinary correlation. A
    /// parameter the query never mentions could bind nothing, so it raises `ValueError`
    /// naming it (`message_id` `sparql-prepared-parameter-unmentioned`) — see
    /// `purrdf_sparql_eval::PreparedExecution::check_parameters_mentioned` for what
    /// counts as a mention.
    ///
    /// What is prepared here is the PLAN, not the data: the returned object holds a
    /// reference to THIS store and re-reads its current contents on every
    /// [`run`](super::prepared::PyPreparedQuery::run), rather than freezing a
    /// snapshot once now. A mutation made after `prepare` — including one made
    /// between two `run` calls on the SAME handle — is visible to the next run. This
    /// is what a rule fixpoint or an incremental SHACL revalidation needs: both
    /// mutate their store every round and re-run the same plan against what the
    /// mutation just produced, so a handle that answered over a one-time snapshot
    /// would make that use case impossible — refusing staleness by refusing the
    /// feature.
    ///
    /// A prepared query must not be shared between threads: a run borrows it
    /// uniquely, because a query body can re-enter the evaluator and a handle
    /// reachable twice while in flight is one two evaluations can disagree about.
    ///
    /// Engine configuration and relation/aggregate registration behave exactly as on
    /// [`query`](Self::query) — `extension_namespaces`, `property_fn_namespaces`,
    /// `standpoint_predicates`, `relations`, `relations_from_graph`, `path_relations`
    /// and `aggregate_namespace` all admit the plan and are then CARRIED by the
    /// returned object, so [`PreparedQuery::run`](super::prepared::PyPreparedQuery::run)
    /// evaluates under the SAME registries the plan was admitted under. `xpath_regex`
    /// (see [`query`](Self::query)) is carried the same way: every run's `REGEX` and
    /// `REPLACE` evaluate under the dated law selected here. Nothing here
    /// widens what a registered relation reaches — running under a different registry
    /// than the one a plan was prepared against is refused, not silently answered
    /// short, exactly as [`query`](Self::query) would refuse it if asked to.
    ///
    /// Two of those axes read the store's OWN GRAPH rather than a caller's constant:
    /// `relations_from_graph` reads an `rdf:List` of `rdf:List`s written in the store,
    /// and `path_relations` traverses the store's own edges. Their tables are
    /// therefore rebuilt from each run's dataset and the plan re-admitted under the
    /// rebuilt registry — see
    /// [`GraphDerivedRelations`](super::prepared::GraphDerivedRelations) — so that
    /// "the data is re-read on every run" holds for a relation's rows exactly as it
    /// holds for an ordinary triple pattern, and one answer is never assembled from
    /// two points in time. The other axes are caller-supplied constants and are
    /// carried unchanged: re-deriving a constant is how a constant stops being one.
    ///
    /// `substitutions` has no seat here, deliberately: a prepared query's whole point
    /// is that the values that change between runs arrive per-run through
    /// [`run`](super::prepared::PyPreparedQuery::run)'s parameter bindings, and a
    /// second, prepare-time door onto the same values would be redundant at best and,
    /// since only `run`'s bindings are actually honoured, silently ignored at worst.
    #[pyo3(signature = (
        query,
        *,
        parameters=None,
        extension_namespaces=None,
        property_fn_namespaces=None,
        standpoint_predicates=None,
        relations=None,
        relations_from_graph=None,
        path_relations=None,
        aggregate_namespace=None,
        xpath_regex=None,
    ))]
    #[allow(
        clippy::too_many_arguments,
        reason = "each engine-configuration axis is named explicitly at the call site, exactly \
                  as on `query`"
    )]
    fn prepare(
        slf: &Bound<'_, Self>,
        query: &str,
        parameters: Option<Vec<String>>,
        extension_namespaces: Option<Vec<String>>,
        property_fn_namespaces: Option<Vec<String>>,
        standpoint_predicates: Option<(String, String)>,
        relations: Option<&Bound<'_, PyDict>>,
        relations_from_graph: Option<&Bound<'_, PyDict>>,
        path_relations: Option<&Bound<'_, PyDict>>,
        aggregate_namespace: Option<String>,
        xpath_regex: Option<&str>,
    ) -> PyResult<super::prepared::PyPreparedQuery> {
        presentation::settled(move || {
            let py = slf.py();
            let specs = collect_relations(relations, relations_from_graph, path_relations)?;
            // Carried forward to the returned object for `run` to rebuild an engine
            // under (see [`super::prepared::PyPreparedQuery`]) — `config` below moves
            // `standpoint_predicates` into the engine this call admits the plan with, so
            // the value itself has to be cloned before that move.
            let standpoint_for_run = standpoint_predicates.clone();
            let config = EngineConfig {
                extension_namespaces,
                property_fn_namespaces,
                standpoint_predicates,
                xpath_regex: crate::xpath_regex::selection(xpath_regex)?,
            };
            // Carried forward exactly as `standpoint_for_run` is: the law is read at
            // EVALUATION time, off the engine `run` builds, so the handle keeps it.
            let xpath_regex_for_run = config.xpath_regex;
            let parameters = parameters.unwrap_or_default();
            // Read off `config` BEFORE `build_engine` consumes it, exactly as `query`
            // does: the namespace declarations are parse configuration, they belong to
            // the extension environment rather than to the engine, and the environment
            // this call admits the plan against is the one every later `run` evaluates
            // under — so it is derived once, here, and carried.
            let parser_options = engine_parser_options(&config);
            // The relations whose ROWS come out of the store's own graph, kept as their
            // SOURCE configuration for `run` to re-derive from the dataset it answers
            // over. `None` when the caller registered none of them, which is the common
            // case and rebuilds nothing. See `super::prepared::GraphDerivedRelations` for
            // why a table read out of a graph cannot be built once and kept on a handle
            // that re-reads its store every run.
            let graph_derived = super::prepared::GraphDerivedRelations::for_specs(
                &specs,
                query,
                &parameters,
                &parser_options,
                aggregate_namespace.as_ref(),
            );
            // A cheap owning handle to THIS store, taken under the GIL, for `run` to
            // re-read on every call (see `Self::prepare`'s doc comment and
            // `super::prepared::PyPreparedQuery::store`) — distinct from `guard` below,
            // which only borrows `inner` for the ADMISSION-TIME freeze this call itself
            // needs (building the relation registry and parsing/admitting the plan
            // against a snapshot of what the store holds right now).
            let store_handle: Py<Self> = slf.clone().unbind();
            let guard = slf.as_super().borrow();
            let inner = &guard.inner;
            // Snapshot + engine build + admission run detached (GIL released), exactly as
            // `query` does: this does the same freeze, registry build and parse/admit
            // work `query` does on every call, just once instead of per run.
            let result = py.detach(move || {
                let dataset = inner
                    .freeze()
                    .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))?;
                let registry = build_relations(specs, &dataset)?;
                let aggregates =
                    purrdf_validate::query::statistical_aggregates(aggregate_namespace.as_deref());
                let engine = build_engine(config);
                super::prepared::prepare(
                    store_handle,
                    &engine,
                    query,
                    &parameters,
                    parser_options,
                    registry.as_ref(),
                    aggregates.as_ref(),
                    standpoint_for_run,
                    xpath_regex_for_run,
                    graph_derived,
                )
            });
            drop(guard);
            result
        })
    }

    /// Dump the whole store (or one graph, via `from_graph`) in `format`. When `output` (a file-like
    /// with `.write`) is given the bytes are written to it and `None` is returned; otherwise the bytes are
    /// returned directly.
    ///
    /// `base` is the document base the output is written under — the egress MIRROR of
    /// [`load`](Self::load)'s, which this surface lacked. A syntax that can express a
    /// base writes it and relativizes against it; one that cannot emits absolute IRIs.
    /// A base that is not an absolute IRI is a hard failure whatever the format.
    ///
    /// The statement layer is [`StatementLayer::Emit`], which is what this dump already
    /// did before it carried a base. A dump round-trips a user's own store, so its RDF
    /// 1.2 reifier and annotation rows must survive: `Project` would silently thin the
    /// data on the way out, and a format with no surface for them fails closed instead.
    #[pyo3(signature = (output=None, format=None, *, from_graph=None, jsonld_options=None, jsonld_context=None, yaml_schema_url=None, base=None))]
    #[allow(
        clippy::too_many_arguments,
        reason = "Python dump names graph selection, the document base, and JSON-LD configuration explicitly"
    )]
    fn dump(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        output: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        from_graph: Option<&Bound<'_, PyAny>>,
        jsonld_options: Option<&str>,
        jsonld_context: Option<&PyCompiledJsonLdContext>,
        yaml_schema_url: Option<&str>,
        base: Option<String>,
    ) -> PyResult<Option<Py<PyBytes>>> {
        let format = format.ok_or_else(|| PyValueError::new_err("dump: format is required"))?;
        let native = format.to_native();
        // Resolve the Python-side graph selection BEFORE releasing the GIL; the
        // quad materialization + native serialization run detached.
        let graph_projection: Option<Option<RdfTerm>> =
            if native.supports_datasets() && from_graph.is_none() {
                None
            } else {
                // `from_graph` selects one graph (a NamedNode/BlankNode → that graph; an
                // explicit DefaultGraph, or no `from_graph` on a non-dataset format → the
                // default graph). Project its triples into the default graph.
                Some(extract_graph_name(from_graph)?)
            };
        let configured =
            if jsonld_options.is_some() || jsonld_context.is_some() || yaml_schema_url.is_some() {
                Some(options_from_inputs(
                    jsonld_options,
                    jsonld_context,
                    yaml_schema_url,
                )?)
            } else {
                None
            };
        let guard = slf.as_super().borrow();
        let store: &PyQuadStore = &guard;
        let buf: Vec<u8> = py.detach(move || {
            // Serialize natively: materialize the store's quads into the IR
            // verbatim (preserving literal lexical forms) and dispatch to the codec.
            let (quads, declared, selection) = match &graph_projection {
                None => (
                    store.collect_all_quads(),
                    store.declared_graphs(),
                    SerializeGraph::Dataset,
                ),
                Some(graph) => (
                    store.collect_graph_quads(graph.as_ref()),
                    Vec::new(),
                    SerializeGraph::DefaultGraph,
                ),
            };
            let dataset = dataset_from_quads_verbatim(&quads, &declared)
                .map_err(|e| PyValueError::new_err(format!("dump error: {e}")))?;
            // ONE serialization call, not a configured/unconfigured pair: the JSON-LD
            // options are an axis of the same options value the base and the graph
            // selection travel on, so a base cannot reach one arm and miss the other.
            serialize_dataset_with(
                &dataset,
                native,
                base.as_deref(),
                &SerializeOptions {
                    selection,
                    statement_layer: StatementLayer::Emit,
                    jsonld_options: configured.as_ref(),
                },
            )
            .map(|outcome| outcome.bytes)
            .map_err(|e| PyValueError::new_err(format!("dump error: {e}")))
        })?;
        match output {
            Some(output) => {
                output.call_method1("write", (PyBytes::new(py, &buf),))?;
                Ok(None)
            }
            None => Ok(Some(PyBytes::new(py, &buf).unbind())),
        }
    }
}

impl PyStore {
    /// The next per-load blank scope ordinal (monotonic, wrapping past 1).
    fn next_load_scope(&self) -> u64 {
        self.next_load_scope.fetch_add(1, Ordering::Relaxed)
    }
}

/// An in-memory quad set supporting RDFC-1.0 canonicalization (`Dataset`).
///
/// # Absoluteness holds here too
///
/// A `Dataset` is a plain quad list rather than a store, so it has no term table whose
/// interner would enforce the IR-boundary absoluteness invariant for it. It enforces the
/// invariant itself, at both ingresses ([`add`](PyDataset::add) and the constructor),
/// through [`QuadValues::check_absolute_iris`] — the SAME
/// `purrdf_core::ir::absolute::check_absolute` every other ingress reaches, not a second
/// spelling of the rule.
///
/// That is deliberate rather than incidental. `NamedNode("foo")` is constructible — the
/// term constructors are string carriers and do not resolve anything — so without this
/// check a relative IRI could reach `canonicalize`, which would hash it and hand back a
/// stable RDFC-1.0 label for a term whose identity is unknowable. "Nothing invalid
/// escapes because there is no serializer here" is not the invariant; being
/// unrepresentable from every ingress is.
#[pyclass(name = "Dataset")]
#[derive(Debug)]
pub struct PyDataset {
    /// The accumulated quads, deduplicated by content (set semantics).
    quads: Vec<RdfQuad>,
}

#[pymethods]
impl PyDataset {
    /// Build a dataset, optionally seeding it from an iterable of `Quad`.
    ///
    /// Raises `ValueError` on the first seed quad carrying a relative IRI, for the same
    /// reason `add` does. The dataset is not partially built: the constructor fails and
    /// no object is returned.
    #[new]
    #[pyo3(signature = (quads=None))]
    fn new(quads: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut out = Self { quads: Vec::new() };
        if let Some(quads) = quads {
            for item in quads.try_iter()? {
                let item = item?;
                let quad = item
                    .cast::<PyQuad>()
                    .map_err(|_| PyTypeError::new_err("Dataset accepts an iterable of Quad"))?;
                out.checked_insert(quad.get().inner.clone())?;
            }
        }
        Ok(out)
    }

    /// Add a single quad.
    ///
    /// Raises `ValueError` if the quad carries a relative IRI in any position — its own
    /// terms, a literal's datatype, or one nested in a quoted triple — with the shared
    /// `iri-relative-no-base` diagnostic code leading the message, exactly as `Store.add`
    /// does. A `Dataset` is handed terms, never a document, so there is no base in scope
    /// to resolve a relative reference against and PurRDF invents none.
    ///
    /// The refused quad does not land: the dataset is unchanged and still usable.
    fn add(&mut self, quad: &PyQuad) -> PyResult<()> {
        self.checked_insert(quad.inner.clone())
    }

    /// Canonicalize blank-node labels in place under `algorithm` (native RDFC-1.0).
    ///
    /// Raises `ValueError` if this dataset's content is refused canonicalization (a
    /// reserved-vocabulary IRI, or an n-degree search that exhausts its budget) — this
    /// dataset's content is wholly caller-supplied, so the refusal comes back as an
    /// ordinary Python exception via the typed [`purrdf_core::try_canonicalize`] path,
    /// never a process abort. The dataset is left unchanged when it raises.
    fn canonicalize(
        &mut self,
        py: Python<'_>,
        algorithm: PyCanonicalizationAlgorithm,
    ) -> PyResult<()> {
        // RDFC-1.0 hashing is the heavy path — run it detached (GIL released).
        let quads = &self.quads;
        let canonicalized = py
            .detach(|| super::canon::canonicalize_quads(quads, algorithm))
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        self.quads = canonicalized;
        Ok(())
    }

    fn __len__(&self) -> usize {
        self.quads.len()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyQuadIter>> {
        let quads = self.quads.clone();
        Py::new(py, PyQuadIter { quads, pos: 0 })
    }
}

impl PyDataset {
    /// Enforce the absoluteness invariant, then insert with set semantics.
    ///
    /// The check runs BEFORE the push, so a refusal leaves the dataset byte-identical to
    /// what it was — `Store.add`'s contract, kept here.
    fn checked_insert(&mut self, quad: RdfQuad) -> PyResult<()> {
        rdf_quad_to_values(&quad)
            .check_absolute_iris()
            .map_err(|e| iri_value_error(&e))?;
        self.insert(quad);
        Ok(())
    }

    /// Insert a quad with set semantics (no duplicate content).
    fn insert(&mut self, quad: RdfQuad) {
        if !self.quads.contains(&quad) {
            self.quads.push(quad);
        }
    }
}

/// Iterator over a [`PyDataset`]'s / [`PyStore`]'s quads (snapshot at iteration time).
#[pyclass(name = "QuadIter")]
#[derive(Debug)]
pub struct PyQuadIter {
    pub(crate) quads: Vec<RdfQuad>,
    pub(crate) pos: usize,
}

#[pymethods]
impl PyQuadIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<Py<PyQuad>>> {
        let slf = &mut *slf;
        next_quad(py, &slf.quads, &mut slf.pos)
    }
}

/// Advance a snapshot iterator over `quads`: the quad at `*pos` as a fresh Python
/// `Quad`, or `None` once exhausted. The one `__next__` body every quad-snapshot
/// iterator (`QuadIter`, `QueryQuads`) shares.
pub(crate) fn next_quad(
    py: Python<'_>,
    quads: &[RdfQuad],
    pos: &mut usize,
) -> PyResult<Option<Py<PyQuad>>> {
    let Some(quad) = quads.get(*pos) else {
        return Ok(None);
    };
    *pos += 1;
    Ok(Some(Py::new(
        py,
        PyQuad {
            inner: quad.clone(),
        },
    )?))
}
