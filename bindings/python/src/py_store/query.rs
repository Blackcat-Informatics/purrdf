// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL result model for the `purrdf` Python extension: the materialized
//! `QuerySolutions` / `QuerySolution` (SELECT), `QueryTriples` / `QueryQuads`
//! (CONSTRUCT/DESCRIBE), and `QueryBoolean` (ASK) pyclasses, plus the
//! `materialize_results` adapter the store seam uses to turn a native
//! [`SparqlResult`] into these objects.
//!
//! Native backing: solution cells are `purrdf_core::TermValue`,
//! CONSTRUCT triples are `RdfTriple` and CONSTRUCT quads are `RdfQuad`. The engine is
//! `NativeSparqlEngine`; the oxigraph `QueryResults` type is gone from this surface.
//!
//! # Two CONSTRUCT result types, chosen by what the result carries
//!
//! A quad-template CONSTRUCT (`CONSTRUCT { GRAPH ?g { … } }` — a first-party
//! extension, NOT defined by SPARQL 1.2) names a graph per statement, so one result
//! may span
//! several named graphs and may mix them with default-graph triples. `Triple` has no
//! graph slot, so a default-graph result stays a `QueryTriples` (unchanged, for every
//! SPARQL 1.1 CONSTRUCT and every DESCRIBE) while a graph-carrying one is a
//! `QueryQuads` of `Quad`s. Asking a `QueryQuads` for a single-graph syntax raises
//! rather than dropping the graphs — see [`refuse_uncarriable_named_graphs`].
//!
//! # The governed surface
//!
//! This module also owns the Python side of caller-supplied execution governors:
//! [`GovernorArgs`] (the ceilings a keyword argument carries), [`PyStopWatch`] (the
//! composed stop signal — the caller's token, the caller's deadline, and the
//! interpreter's own pending-signal flag), [`run_governed`] (which engages both while
//! the GIL is released), and the outcome objects `QueryOutcome` / `UpdateOutcome` /
//! `PartialAnswers` / `TrippedGovernor` / `GovernorEvidence` a governed call returns.
//!
//! **A tripped governor is returned, never raised.** It is neither a complete answer nor
//! a failure: reported as complete, a truncated answer is silently wrong; raised as an
//! exception, the rows the budget already paid for are thrown away and the caller is told
//! the engine misbehaved. So `Store.query_governed` returns a `QueryOutcome` on both
//! paths and reserves exceptions for what they mean everywhere else in this binding — a
//! malformed query, a broken snapshot, and the one Python-level event that genuinely is
//! an exception, a `KeyboardInterrupt` (see [`run_governed`]).

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use purrdf_core::{GraphMatch, ResourceVector};
use purrdf_sparql_eval::{
    AggregateRegistry, BudgetExhausted, CancellationFlag, GovernedOutcome, GovernedUpdateOutcome,
    GovernorEvidence, MemoryRelation, NativeSparqlEngine, ParserOptions, PartialAnswers,
    PropertyFunctionRegistry, QueryGovernors, ResourceDimension, StandpointPredicates, StopCause,
    StopSignal, TrippedGovernor, WallDeadline,
};
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyString};

use super::io::{PyRdfFormat, serialize_quads, serialize_triples};
use super::term::{PyQuad, PyTriple, PyVariable, extract_term_value, term_to_py};
use crate::{GovernedEntailment, RdfDataset, RdfQuad, RdfTerm, RdfTriple, SparqlResult, TermValue};

/// The optional per-call engine configuration the Python surface accepts, converted
/// out of Python before the GIL is released and moved into the detached region whole.
///
/// One struct rather than four positional `Option`s because three of the four are
/// `Option<Vec<String>>`: at a call site a swapped pair would still compile and would
/// silently reclassify every predicate under a namespace.
#[derive(Debug, Default)]
pub(super) struct EngineConfig {
    /// The extension-function namespace set threaded into [`ParserOptions`]. Unset
    /// means the engine default (**extensions off**): a call-position IRI is an
    /// ordinary custom function.
    pub(super) extension_namespaces: Option<Vec<String>>,
    /// The property-function namespace PREFIXES threaded into [`ParserOptions`]: a
    /// predicate IRI under one is lowered to a call node rather than to a triple
    /// pattern. Unset means the engine default (**prefix recognition off**), which is
    /// all a caller who registers relations needs — the engine derives EXACT-IRI
    /// recognition from the registry itself, so a registered relation is reachable
    /// without also reclassifying every other IRI under its namespace. Declaring a
    /// namespace is how a caller asks for the stricter reading, in which an
    /// unregistered IRI under it is a hard error instead of an empty scan.
    pub(super) property_fn_namespaces: Option<Vec<String>>,
    /// The `(according_to, sharpens)` domain predicate IRIs threaded into
    /// [`NativeSparqlEngine::with_standpoint_predicates`], read by `heldIn` and
    /// loss-aware `CONSTRUCT`. Unset means the engine default: evaluating `heldIn`
    /// is a hard error (PurRDF mints no vocabulary of its own).
    pub(super) standpoint_predicates: Option<(String, String)>,
}

/// Build the [`NativeSparqlEngine`] for one query/update call from the optional
/// Python-surface engine configuration (shared by `Store` and `MutableDataset`).
///
/// [`ParserOptions::property_fn_iris`] is left empty here on purpose: the exact-IRI
/// recognition set is derived by the engine from the registry the call is evaluated
/// under, one-to-one, so it cannot disagree with the relations actually injected.
pub(super) fn build_engine(config: EngineConfig) -> NativeSparqlEngine {
    let EngineConfig {
        extension_namespaces,
        property_fn_namespaces,
        standpoint_predicates,
    } = config;
    let mut engine = NativeSparqlEngine::new();
    if extension_namespaces.is_some() || property_fn_namespaces.is_some() {
        engine = engine.with_parser_options(ParserOptions {
            extension_fn_namespaces: extension_namespaces.unwrap_or_default(),
            property_fn_namespaces: property_fn_namespaces.unwrap_or_default(),
            property_fn_iris: Vec::new(),
        });
    }
    if let Some((according_to, sharpens)) = standpoint_predicates {
        engine =
            engine.with_standpoint_predicates(StandpointPredicates::new(according_to, sharpens));
    }
    engine
}

/// One caller-declared property-function relation, converted out of Python **before**
/// the GIL is released.
///
/// A relation the Python surface registers is pure data — a frozen table of
/// [`TermValue`]s, or the head of one written in the store's own dataset — never a
/// Python callable. That is what makes the seam GIL-free: nothing the engine invokes
/// while detached can re-enter the interpreter, so a relation is `Send + Sync` owned
/// data exactly as a Rust host's [`MemoryRelation`] is.
#[derive(Debug)]
pub(super) enum RelationSpec {
    /// Rows supplied as Python data: `(subject_arity, object_arity, rows)`.
    Rows {
        /// The declared number of subject-side arguments.
        subject_arity: usize,
        /// The declared number of object-side arguments.
        object_arity: usize,
        /// The table, in emission order; each row holds its values in flattened
        /// order (subject-side first, then object-side).
        rows: Vec<Vec<TermValue>>,
    },
    /// Rows read out of the store's own dataset: the head of an `rdf:List` of
    /// `rdf:List`s, one inner list per row.
    Graph {
        /// The term naming the table's head node.
        head: TermValue,
        /// The declared number of subject-side arguments.
        subject_arity: usize,
        /// The declared number of object-side arguments.
        object_arity: usize,
    },
}

/// Collect the `relations` / `relations_from_graph` keyword dicts into the ordered
/// `(IRI, spec)` list one call registers.
///
/// # A duplicate IRI is refused here, not at registration
///
/// [`PropertyFunctionRegistry::register`] **panics** on a duplicate, deliberately: a
/// shadowed relation silently changes which rows a graph pattern produces, and both
/// spellings of the call are identical. Python dict keys are unique, so the only way
/// to reach that panic from this surface is to name one IRI in both dicts — refused
/// here as a `ValueError`, because a host misconfiguration crossing a language
/// boundary is an exception, not an abort.
///
/// # Errors
///
/// `TypeError` if a key is not a `str` or a value is not the declared shape;
/// `ValueError` if an IRI is declared twice.
pub(super) fn collect_relations(
    relations: Option<&Bound<'_, PyDict>>,
    relations_from_graph: Option<&Bound<'_, PyDict>>,
) -> PyResult<Vec<(String, RelationSpec)>> {
    let mut specs: Vec<(String, RelationSpec)> = Vec::new();
    for (dict, from_graph) in [(relations, false), (relations_from_graph, true)] {
        let Some(dict) = dict else { continue };
        for (key, value) in dict {
            let iri = key.extract::<String>().map_err(|_| {
                PyTypeError::new_err("property-function relation keys must be IRI strings")
            })?;
            if specs.iter().any(|(seen, _)| *seen == iri) {
                return Err(PyValueError::new_err(format!(
                    "property function <{iri}> is declared twice; a relation may not be \
                     silently shadowed, because both spellings of the call are identical \
                     and the only observable difference is which rows the query returns"
                )));
            }
            let spec = if from_graph {
                graph_relation_spec(&iri, &value)?
            } else {
                rows_relation_spec(&iri, &value)?
            };
            specs.push((iri, spec));
        }
    }
    Ok(specs)
}

/// Parse one `relations` value: `(subject_arity, object_arity, rows)`.
fn rows_relation_spec(iri: &str, value: &Bound<'_, PyAny>) -> PyResult<RelationSpec> {
    let [subject_arity, object_arity, rows] =
        relation_triple(iri, value, "(subject_arity, object_arity, rows)")?;
    let subject_arity = arity(iri, &subject_arity)?;
    let object_arity = arity(iri, &object_arity)?;
    let mut table = Vec::new();
    for row in rows.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "property function <{iri}>: `rows` must be a sequence of rows"
        ))
    })? {
        let row = row?;
        let mut cells = Vec::new();
        for cell in row.try_iter().map_err(|_| {
            PyTypeError::new_err(format!(
                "property function <{iri}>: each row must be a sequence of terms"
            ))
        })? {
            cells.push(extract_term_value(&cell?)?);
        }
        table.push(cells);
    }
    Ok(RelationSpec::Rows {
        subject_arity,
        object_arity,
        rows: table,
    })
}

/// Parse one `relations_from_graph` value: `(head, subject_arity, object_arity)`.
fn graph_relation_spec(iri: &str, value: &Bound<'_, PyAny>) -> PyResult<RelationSpec> {
    let [head, subject_arity, object_arity] =
        relation_triple(iri, value, "(head, subject_arity, object_arity)")?;
    Ok(RelationSpec::Graph {
        head: extract_term_value(&head)?,
        subject_arity: arity(iri, &subject_arity)?,
        object_arity: arity(iri, &object_arity)?,
    })
}

/// Unpack a relation declaration into its three positions, naming the expected shape
/// in the error rather than reporting an anonymous extraction failure.
fn relation_triple<'py>(
    iri: &str,
    value: &Bound<'py, PyAny>,
    shape: &str,
) -> PyResult<[Bound<'py, PyAny>; 3]> {
    let shape_error = || {
        PyTypeError::new_err(format!(
            "property function <{iri}> must be declared as {shape}"
        ))
    };
    let items: Vec<Bound<'py, PyAny>> = value.extract().map_err(|_| shape_error())?;
    <[Bound<'py, PyAny>; 3]>::try_from(items).map_err(|_| shape_error())
}

/// Read one declared arity position as a non-negative integer.
fn arity(iri: &str, value: &Bound<'_, PyAny>) -> PyResult<usize> {
    value.extract::<usize>().map_err(|_| {
        PyTypeError::new_err(format!(
            "property function <{iri}>: an arity must be a non-negative integer"
        ))
    })
}

/// Build the registry one call evaluates under, from the already-converted `specs`
/// and the frozen snapshot the call runs against.
///
/// Runs **inside** the detached region: every value it reads is owned Rust data, and
/// [`RelationSpec::Graph`] needs the snapshot that only exists there. `None` — the
/// no-relations case — is the absence of a registry rather than an empty one, so a
/// caller who names no relation gets byte-for-byte the pre-existing evaluation.
///
/// # The graph a table head is read from
///
/// [`GraphMatch::Default`], matching the Rust harness: a relation table is
/// *configuration* written beside the data, and the default graph is the one a store
/// loaded without a graph name puts it in. Reading `Any` instead would let a table
/// silently gain rows from an unrelated named graph.
///
/// # Errors
///
/// `ValueError` carrying the kernel's own diagnostic for a ragged table
/// ([`MemoryRelation::new`]) or a torn, absent, or wrong-width list
/// ([`MemoryRelation::from_graph`]).
pub(super) fn build_relations(
    specs: Vec<(String, RelationSpec)>,
    dataset: &RdfDataset,
) -> PyResult<Option<PropertyFunctionRegistry>> {
    if specs.is_empty() {
        return Ok(None);
    }
    let mut registry = PropertyFunctionRegistry::new();
    for (iri, spec) in specs {
        let table = match spec {
            RelationSpec::Rows {
                subject_arity,
                object_arity,
                rows,
            } => MemoryRelation::new(subject_arity, object_arity, rows),
            RelationSpec::Graph {
                head,
                subject_arity,
                object_arity,
            } => MemoryRelation::from_graph(
                dataset,
                &head,
                GraphMatch::Default,
                subject_arity,
                object_arity,
            ),
        }
        .map_err(|e| PyValueError::new_err(format!("property function <{iri}>: {e}")))?;
        registry.register(iri, Arc::new(table));
    }
    Ok(Some(registry))
}

/// Build the [`AggregateRegistry`] for one query/update call from the caller's
/// `aggregate_namespace` keyword, or `None` when it is unset.
///
/// This is the ENTIRE Python surface for purrdf's first-party statistical aggregate set
/// (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
/// `FIRST`, `LAST`, `TOPK` — see `purrdf_sparql_eval::stat_agg`):
/// [`AggregateRegistry::register_statistical_aggregates`] takes only an IRI namespace
/// string, so it crosses the Python boundary exactly the way `property_fn_namespaces`
/// does, with no per-aggregate marshaling and no Python callable involved. The GENERAL
/// custom-aggregate seam (`purrdf_sparql_eval::agg_fn::AggregateRegistry::register`, an
/// arbitrary `init`/`step`/`combine`/`finish` closure) is Rust-host-only — a fold has no
/// data-only reduction the way a property-function relation does — and this binding
/// exposes no surface for it, not even a namespace-only one, because there is no
/// namespace-only ENTRY POINT for an arbitrary aggregate the way there is for the
/// closed statistical set.
///
/// `namespace` is caller configuration, never a purrdf-owned vocabulary: omitting the
/// keyword leaves every one of the ten names an ordinary unregistered custom-aggregate
/// IRI, refused at prepare time exactly as any other unregistered `AGG(<iri>, …)` call.
pub(super) fn build_aggregates(namespace: Option<String>) -> Option<AggregateRegistry> {
    let namespace = namespace?;
    let mut registry = AggregateRegistry::new();
    registry.register_statistical_aggregates(&namespace);
    Some(registry)
}

/// SELECT results, materialized. Mirrors the oxigraph Python `QuerySolutions`.
#[pyclass(name = "QuerySolutions")]
#[derive(Debug)]
pub struct PyQuerySolutions {
    variables: Arc<[String]>,
    rows: Vec<Vec<Option<RdfTerm>>>,
    pos: usize,
}

#[pymethods]
impl PyQuerySolutions {
    /// The bound variables, in projection order.
    #[getter]
    fn variables(&self, py: Python<'_>) -> PyResult<Vec<Py<PyVariable>>> {
        self.variables
            .iter()
            .map(|v| Py::new(py, PyVariable { inner: v.clone() }))
            .collect()
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(
        mut slf: PyRefMut<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Option<Py<PyQuerySolution>>> {
        if slf.pos >= slf.rows.len() {
            return Ok(None);
        }
        let pos = slf.pos;
        slf.pos += 1;
        let row = std::mem::take(&mut slf.rows[pos]);
        let variables = Arc::clone(&slf.variables);
        Ok(Some(Py::new(py, PyQuerySolution { variables, row })?))
    }

    fn __len__(&self) -> usize {
        self.rows.len()
    }
}

/// A single SELECT solution row. Mirrors the oxigraph Python `QuerySolution`.
#[pyclass(name = "QuerySolution")]
#[derive(Debug)]
pub struct PyQuerySolution {
    variables: Arc<[String]>,
    row: Vec<Option<RdfTerm>>,
}

#[pymethods]
impl PyQuerySolution {
    /// Look a binding up by variable name (`str`), `Variable`, or position
    /// (`int`). An unbound variable yields `None`; an unknown name is a
    /// `KeyError`, matching the oxigraph Python API.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
        let index = if let Ok(i) = key.extract::<usize>() {
            if i >= self.row.len() {
                return Err(PyKeyError::new_err(format!("no variable at position {i}")));
            }
            i
        } else {
            let name = if let Ok(var) = key.cast::<PyVariable>() {
                var.get().inner.clone()
            } else if let Ok(s) = key.cast::<PyString>() {
                s.to_str()?.to_owned()
            } else {
                return Err(PyTypeError::new_err(
                    "solution key must be a str, Variable, or int",
                ));
            };
            self.variables
                .iter()
                .position(|v| v.as_str() == name)
                .ok_or_else(|| PyKeyError::new_err(format!("no variable named `{name}`")))?
        };
        match &self.row[index] {
            Some(term) => Ok(Some(term_to_py(py, term)?)),
            None => Ok(None),
        }
    }
}

/// Default-graph CONSTRUCT/DESCRIBE results, materialized. Mirrors the oxigraph
/// Python `QueryTriples`.
///
/// This is the result object for a CONSTRUCT/DESCRIBE whose statements ALL land in
/// the default graph — the plain SPARQL 1.1 template, and every `DESCRIBE`. A
/// quad-template CONSTRUCT that writes a named graph yields [`PyQueryQuads`] instead,
/// because a triple has no slot to carry the graph name in.
#[pyclass(name = "QueryTriples")]
#[derive(Debug)]
pub struct PyQueryTriples {
    pub(crate) triples: Vec<RdfTriple>,
    pos: usize,
}

#[pymethods]
impl PyQueryTriples {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<Py<PyTriple>>> {
        if slf.pos >= slf.triples.len() {
            return Ok(None);
        }
        let triple = slf.triples[slf.pos].clone();
        slf.pos += 1;
        Ok(Some(Py::new(py, PyTriple { inner: triple })?))
    }

    fn __len__(&self) -> usize {
        self.triples.len()
    }

    /// Serialize the constructed triples to bytes in `format` (the N-Triples
    /// fast path the `sparql` seam uses for its rdflib hand-off).
    fn serialize<'py>(
        &self,
        py: Python<'py>,
        format: PyRdfFormat,
    ) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.serialize_bytes(py, format)?))
    }
}

impl PyQueryTriples {
    /// The serialization core, shared by the `serialize` method and the module-level
    /// `purrdf.serialize` function so the two can never diverge.
    pub(crate) fn serialize_bytes(&self, py: Python<'_>, format: PyRdfFormat) -> PyResult<Vec<u8>> {
        // The native serialization runs detached (GIL released).
        let triples = &self.triples;
        py.detach(|| serialize_triples(triples, format.to_native()))
            .map_err(|e| PyValueError::new_err(format!("serialize error: {e}")))
    }
}

/// Graph-carrying CONSTRUCT results, materialized: the quad stream of a template that
/// names at least one graph.
///
/// # Why a second result type rather than a graph slot on `QueryTriples`
///
/// A quad-template CONSTRUCT (`CONSTRUCT { GRAPH ?g { … } }` — a first-party
/// extension, NOT defined by SPARQL 1.2) carries a graph per STATEMENT: one template
/// may
/// write several named graphs, and may mix default-graph triples with named-graph
/// quads. `Triple` has no graph slot, so a `QueryTriples` cannot represent that result
/// — flattening it into one triple stream silently deletes exactly the graph names the
/// caller spelled out in the query. So a result that names a graph comes back as
/// `QueryQuads`, whose members are `Quad`s with a live `graph_name`, and
/// [`serialize`](Self::serialize) round-trips them through any quad-capable syntax.
///
/// A result whose statements are ALL default-graph is unchanged: it is still a
/// `QueryTriples` of `Triple`s, byte-identical on every format. Only a query that
/// actually asks for a named graph — which neither SPARQL 1.1 nor SPARQL 1.2 has any
/// syntax to ask for — can produce this type, so no pre-existing query changes shape.
#[pyclass(name = "QueryQuads")]
#[derive(Debug)]
pub struct PyQueryQuads {
    pub(crate) quads: Vec<RdfQuad>,
    pos: usize,
}

#[pymethods]
impl PyQueryQuads {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<Py<PyQuad>>> {
        if slf.pos >= slf.quads.len() {
            return Ok(None);
        }
        let quad = slf.quads[slf.pos].clone();
        slf.pos += 1;
        Ok(Some(Py::new(py, PyQuad { inner: quad })?))
    }

    fn __len__(&self) -> usize {
        self.quads.len()
    }

    /// Every distinct non-default graph name the result carries, in N-Triples term
    /// syntax, sorted lexicographically.
    ///
    /// Sorted rather than merely deduplicated so the list is a function of the RESULT
    /// and not of the evaluator's quad order — the same property the refusal message
    /// needs, from the same helper, so the two can never disagree.
    #[getter]
    fn graph_names(&self) -> Vec<String> {
        distinct_graph_names(&self.quads)
    }

    /// Serialize the constructed quads to bytes in `format`.
    ///
    /// # Errors
    ///
    /// Raises `ValueError` when `format` is a single-graph RDF syntax
    /// (`RdfFormat.TURTLE` / `RdfFormat.N_TRIPLES`): those have no named-graph
    /// construct, so serializing would DROP every graph-scoped statement and hand back
    /// a well-formed document missing exactly what the query asked for. See
    /// [`refuse_uncarriable_named_graphs`].
    fn serialize<'py>(
        &self,
        py: Python<'py>,
        format: PyRdfFormat,
    ) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.serialize_bytes(py, format)?))
    }
}

impl PyQueryQuads {
    /// The serialization core, shared by the `serialize` method and the module-level
    /// `purrdf.serialize` function so the refusal cannot be reachable from one entry
    /// point and not the other.
    pub(crate) fn serialize_bytes(&self, py: Python<'_>, format: PyRdfFormat) -> PyResult<Vec<u8>> {
        // Refused BEFORE the serializer runs: a result the requested syntax would
        // silently empty out never becomes bytes.
        refuse_uncarriable_named_graphs(&self.quads, format)?;
        // The native serialization runs detached (GIL released).
        let quads = &self.quads;
        py.detach(|| serialize_quads(quads, format.to_native()))
            .map_err(|e| PyValueError::new_err(format!("serialize error: {e}")))
    }
}

/// How many graph names a refusal spells out individually before it summarises the
/// rest as a count.
///
/// A CONSTRUCT template can name a graph per statement — and with a graph VARIABLE one
/// template writes as many graphs as the `WHERE` has distinct bindings — so the name
/// list is unbounded in principle. Eight is enough to identify the mistake in every
/// hand-written query and short enough that the message stays a message; the tail is
/// reported as "and N more" rather than truncated silently, so the count is always
/// exact even when the list is not complete. Matches the CLI refusal's limit.
const NAMED_GRAPH_SAMPLE_LIMIT: usize = 8;

/// The quad-capable `RdfFormat` members, in declaration order — the alternatives every
/// named-graph refusal points at.
const QUAD_CAPABLE_FORMATS: &str = "RdfFormat.N_QUADS/TRIG/TRIX/HEXTUPLES/JSON_LD/YAML_LD";

/// Refuse to serialize graph-carrying quads to a single-graph RDF syntax, naming the
/// graphs, the format, and what to use instead.
///
/// # Why this REFUSES rather than serializing what fits
///
/// The graph name is in the QUERY the caller wrote, one token at a time
/// (`CONSTRUCT { GRAPH ex:out { … } }`), so it is the single most explicit thing in the
/// request. Turtle and N-Triples have no named-graph construct and the single-graph
/// serializers DROP every graph-scoped row (they do not fold it into the default graph
/// — see `purrdf_core::loss`'s `named-graph-dropped` note). Serializing anyway would
/// return a well-formed document missing exactly the statements the caller asked for,
/// with no exception and no loss signal — the silent-wrong shape this binding refuses
/// everywhere else, and the reason a graph-carrying result is a `QueryQuads` at all.
///
/// A mixed template makes refusal the only honest answer: emitting the default-graph
/// half would report a partial answer as a complete one, which is worse than emitting
/// nothing. So ANY non-default graph refuses, exactly as the `purrdf query` lane does.
fn refuse_uncarriable_named_graphs(quads: &[RdfQuad], format: PyRdfFormat) -> PyResult<()> {
    if format.to_native().supports_datasets() {
        return Ok(());
    }
    let names = distinct_graph_names(quads);
    let count = names.len();
    if count == 0 {
        return Ok(());
    }
    let listed = if count > NAMED_GRAPH_SAMPLE_LIMIT {
        format!(
            "{}, and {} more",
            names[..NAMED_GRAPH_SAMPLE_LIMIT].join(", "),
            count - NAMED_GRAPH_SAMPLE_LIMIT
        )
    } else {
        names.join(", ")
    };
    let (graphs, them) = if count == 1 {
        ("named graph", "it")
    } else {
        ("named graphs", "them")
    };
    Err(PyValueError::new_err(format!(
        "a CONSTRUCT/DESCRIBE result carrying {count} {graphs} ({listed}) cannot be \
         serialized to the single-graph RDF syntax `{token}`: {token} has no named-graph \
         construct, so every statement in {them} would be DROPPED (not folded into the \
         default graph) and the output would silently omit what the query asked for. \
         Re-serialize with a quad-capable format ({QUAD_CAPABLE_FORMATS})",
        token = format.member_name()
    )))
}

/// Every distinct non-default graph name in `quads`, rendered in N-Triples term syntax
/// and sorted lexicographically.
///
/// Sorted through a [`BTreeSet`], not merely deduplicated: the message must be
/// byte-identical across runs, and both the evaluator's quad order and any hash-map
/// iteration would make it a function of insertion order. The flat quad stream already
/// carries the RDF 1.2 statement layer as `rdf:reifies` / annotation rows WITH their
/// graph slot, so a graph named only by a reifier or annotation is listed too.
fn distinct_graph_names(quads: &[RdfQuad]) -> Vec<String> {
    let names: BTreeSet<String> = quads
        .iter()
        .filter_map(|quad| quad.graph_name.as_ref())
        .map(render_graph_name)
        .collect();
    names.into_iter().collect()
}

/// Render one graph-name term for a diagnostic, in N-Triples term syntax.
///
/// A CONSTRUCT template's graph slot only ever resolves to an IRI (a graph variable
/// bound to anything else skips the statement, per SPARQL §16.2), and the RDF 1.2
/// abstract syntax admits only an IRI or a blank node in the graph position — but the
/// match is total over [`RdfTerm`] rather than partial, because a diagnostic that
/// panics on a term it did not expect is worse than one that names it.
fn render_graph_name(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => format!("<{iri}>"),
        RdfTerm::BlankNode(label) => format!("_:{label}"),
        RdfTerm::Literal(literal) => format!("\"{}\"", literal.lexical_form),
        RdfTerm::Triple(_) => "<<( … )>>".to_owned(),
    }
}

/// An ASK result. Mirrors the oxigraph Python `QueryBoolean`.
#[pyclass(name = "QueryBoolean")]
#[derive(Debug)]
pub struct PyQueryBoolean {
    value: bool,
}

#[pymethods]
impl PyQueryBoolean {
    fn __bool__(&self) -> bool {
        self.value
    }

    fn __str__(&self) -> String {
        self.value.to_string()
    }

    fn __eq__(&self, other: bool) -> bool {
        self.value == other
    }

    fn __hash__(&self) -> u64 {
        u64::from(self.value)
    }
}

/// Convert a native [`SparqlResult`] into the materialized Python result object.
///
/// A SELECT becomes [`PyQuerySolutions`] (each cell a [`RdfTerm`]); an ASK becomes
/// [`PyQueryBoolean`]; a CONSTRUCT/DESCRIBE [`SparqlResult::Graph`] becomes
/// [`PyQueryTriples`] or [`PyQueryQuads`] according to what it carries — see
/// [`materialize_graph`].
///
/// This is the ONE adapter every result-bearing entry point routes through
/// (`Store.query`, `MutableDataset.query`, both governed lanes and the partial-answer
/// certificate), so the graph-carrying result type reaches all of them from here and
/// cannot be wired into some of them only.
pub(crate) fn materialize_results(py: Python<'_>, result: SparqlResult) -> PyResult<Py<PyAny>> {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => {
            let rows: Vec<Vec<Option<RdfTerm>>> = rows
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|cell| cell.map(term_value_to_rdf))
                        .collect()
                })
                .collect();
            Ok(Py::new(
                py,
                PyQuerySolutions {
                    variables: variables.into(),
                    rows,
                    pos: 0,
                },
            )?
            .into_any())
        }
        SparqlResult::Graph(graph) => materialize_graph(py, &graph),
        SparqlResult::Boolean(value) => Ok(Py::new(py, PyQueryBoolean { value })?.into_any()),
    }
}

/// Materialize a CONSTRUCT/DESCRIBE result dataset into the Python result object that
/// can actually hold it.
///
/// The dataset is first flattened to its source-faithful quad stream (the RDF 1.2
/// statement layer re-materialized as `rdf:reifies` / annotation rows, each keeping the
/// graph it was asserted in). Then:
///
/// * **every statement in the default graph** → [`PyQueryTriples`], the triple stream,
///   exactly as before. This is the plain SPARQL 1.1 CONSTRUCT, every DESCRIBE, and
///   every query written before the quad-template grammar existed, so their result
///   type, iteration and serialization are untouched;
/// * **any statement in a named graph** → [`PyQueryQuads`], the quad stream with the
///   graph slot live.
///
/// The discriminator is what the result CARRIES, not what the query's syntax looked
/// like: `CONSTRUCT { GRAPH ?g { … } }` whose `?g` never binds writes only default-graph
/// statements and correctly yields a `QueryTriples`.
///
/// The graph name is never dropped. A triple has no slot for one, so flattening a
/// graph-carrying result into `QueryTriples` would delete the most explicit part of the
/// caller's query with no exception and no loss signal.
fn materialize_graph(py: Python<'_>, graph: &RdfDataset) -> PyResult<Py<PyAny>> {
    let quads = crate::flat_rdf_quads_from_dataset(graph);
    if quads.iter().any(|quad| quad.graph_name.is_some()) {
        return Ok(Py::new(py, PyQueryQuads { quads, pos: 0 })?.into_any());
    }
    let triples: Vec<RdfTriple> = quads
        .into_iter()
        .map(|q| RdfTriple::new(q.subject, q.predicate, q.object))
        .collect();
    Ok(Py::new(py, PyQueryTriples { triples, pos: 0 })?.into_any())
}

/// Lower a dataset-independent [`TermValue`] (the SPARQL egress cell type) into the
/// owned [`RdfTerm`] the Python term layer wraps.
pub(crate) fn term_value_to_rdf(value: TermValue) -> RdfTerm {
    match value {
        TermValue::Iri(iri) => RdfTerm::Iri(iri),
        TermValue::Blank { label, scope } => {
            RdfTerm::BlankNode(scope.qualify_label(&label).into_owned())
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => RdfTerm::Literal(crate::RdfLiteral {
            // The native IR carries the datatype IRI by value (always present); the
            // owned model keeps a plain `xsd:string` / lang `rdf:langString` literal
            // datatype-less, so collapse those back to `None` for term parity.
            datatype: collapse_synthetic_datatype(&datatype, language.as_ref()),
            lexical_form,
            language,
            direction,
        }),
        TermValue::Triple { s, p, o } => RdfTerm::triple(RdfTriple::new(
            term_value_to_rdf(*s),
            term_value_predicate(*p),
            term_value_to_rdf(*o),
        )),
    }
}

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Drop the `TermValue` synthetic datatype IRI when it is the one the owned model
/// leaves implicit: `xsd:string` for a plain literal, `rdf:langString` for a
/// language-tagged one. Any other datatype is kept verbatim.
fn collapse_synthetic_datatype(datatype: &str, language: Option<&String>) -> Option<String> {
    if language.is_some() {
        return (datatype != RDF_LANG_STRING).then(|| datatype.to_owned());
    }
    (datatype != XSD_STRING).then(|| datatype.to_owned())
}

/// A triple-term predicate `TermValue` must be an IRI; fall back to its lexical form
/// for any other (ill-formed) shape so the conversion is total.
fn term_value_predicate(value: TermValue) -> String {
    match value {
        TermValue::Iri(iri) => iri,
        other => term_value_to_rdf(other).to_string(),
    }
}

// ---------------------------------------------------------------------------
// Execution governors
// ---------------------------------------------------------------------------

/// A shareable cancellation bit a Python caller flips to stop a running governed call.
///
/// Hand the same token to as many governed calls as you like and keep your own handle:
/// [`cancel`](Self::cancel) stops every call running under it, from any thread, because a
/// governed call releases the GIL while the engine runs. Latching is by construction —
/// the bit only ever moves from clear to set, and nothing clears it — so build a fresh
/// token per operation rather than resetting one.
#[pyclass(name = "CancellationToken", frozen)]
#[derive(Debug, Default)]
pub struct PyCancellationToken {
    /// The shared monotone bit, handed to the engine inside a [`PyStopWatch`].
    flag: CancellationFlag,
}

#[pymethods]
impl PyCancellationToken {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Cancel every governed call running under this token. Idempotent, never reversible.
    fn cancel(&self) {
        self.flag.cancel();
    }

    /// Whether this token has been cancelled.
    #[getter]
    fn cancelled(&self) -> bool {
        self.flag.is_cancelled()
    }

    fn __repr__(&self) -> String {
        // Rendered Python-side (`True`/`False`), not Rust-side: this string is read in a
        // Python traceback, beside Python values.
        let cancelled = if self.flag.is_cancelled() {
            "True"
        } else {
            "False"
        };
        format!("<CancellationToken cancelled={cancelled}>")
    }
}

/// How long the interrupt watch waits between GIL re-acquisitions.
///
/// The check itself needs the GIL, and a governed call has deliberately released it, so
/// every check costs one re-acquisition. Doing that at each of the evaluator's stop polls
/// would put GIL traffic on a hot path and would serialize the governed call against
/// every other Python thread; doing it never would swallow Ctrl-C until the query
/// finished, which is the failure this watch exists to prevent. Twenty milliseconds
/// bounds the interrupt latency well below human perception while bounding the traffic at
/// fifty re-acquisitions a second regardless of how fast the evaluator polls.
const INTERRUPT_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// The interrupt watch's mutable half.
#[derive(Debug, Default)]
struct InterruptState {
    /// The exception `Python::check_signals` raised, held until the call re-raises it.
    ///
    /// `check_signals` *consumes* the interpreter's pending signal and hands back the
    /// exception the Python-level handler raised. Dropping it here would make a Ctrl-C
    /// vanish, so it is carried out of the detached region and raised by
    /// [`run_governed`].
    raised: Option<PyErr>,
    /// When the watch last re-acquired the GIL to check, or `None` if it never has.
    checked_at: Option<Instant>,
}

/// The composed [`StopSignal`] every governed Python call runs under.
///
/// Three sources, one signal, because [`QueryGovernors::with_stop_signal`] takes one and
/// composing them is the host's job:
///
/// * the caller's [`PyCancellationToken`], when one was supplied;
/// * the caller's wall deadline, when one was supplied;
/// * **always** the interpreter's own pending-signal flag, so Ctrl-C stops a long query
///   rather than being noticed only once it has finished.
///
/// # Latching
///
/// The trait's contract is that a fired signal stays fired, so the resolved cause is
/// written once into a [`OnceLock`] and every later poll returns it without consulting a
/// source again. A simultaneous fire resolves the way the kernel ranks it — a
/// cancellation (an explicit decision) ahead of a deadline (an elapsed measurement) —
/// which is the same order [`TrippedGovernor::precedence_rank`] pins for every tier.
#[derive(Debug)]
pub(super) struct PyStopWatch {
    /// The resolved cause, written once. See the latching note above.
    latched: OnceLock<StopCause>,
    /// The caller's cancellation flag, when one was supplied.
    cancel: Option<CancellationFlag>,
    /// The caller's wall deadline, when one was supplied.
    deadline: Option<WallDeadline>,
    /// Whether the interpreter's pending-signal flag has already fired, so the common
    /// path never takes the mutex.
    interrupted: AtomicBool,
    /// The rate limiter and the captured exception.
    interrupt: Mutex<InterruptState>,
}

impl PyStopWatch {
    /// A watch over the caller's `cancel` token and `deadline_ms` budget, if any.
    fn new(deadline_ms: Option<u64>, cancel: Option<&PyCancellationToken>) -> Self {
        Self {
            latched: OnceLock::new(),
            cancel: cancel.map(|token| token.flag.clone()),
            deadline: deadline_ms.map(|ms| WallDeadline::after(Duration::from_millis(ms))),
            interrupted: AtomicBool::new(false),
            interrupt: Mutex::new(InterruptState::default()),
        }
    }

    /// Take the `KeyboardInterrupt` (or whatever the interpreter's SIGINT handler raised)
    /// the watch captured while the GIL was released, if it captured one.
    fn take_interrupt(&self) -> Option<PyErr> {
        self.interrupt
            .lock()
            .expect("the interrupt state is only held for the length of a check")
            .raised
            .take()
    }

    /// Poll every source once and resolve a simultaneous fire by the kernel's precedence.
    fn observe(&self) -> Option<StopCause> {
        if self
            .cancel
            .as_ref()
            .is_some_and(CancellationFlag::is_cancelled)
            || self.poll_interrupt()
        {
            return Some(StopCause::Cancelled);
        }
        self.deadline.as_ref().and_then(StopSignal::poll)
    }

    /// Whether the interpreter has a signal pending, re-acquiring the GIL to ask at most
    /// once per [`INTERRUPT_POLL_INTERVAL`].
    ///
    /// The GIL is **never** acquired while the interrupt mutex is held: an evaluator
    /// worker blocking on the GIL with this lock in hand, while a thread that holds the
    /// GIL blocks on the lock, is a deadlock, and the two orders are only kept apart by
    /// releasing the lock before attaching.
    fn poll_interrupt(&self) -> bool {
        {
            let mut state = self
                .interrupt
                .lock()
                .expect("the interrupt state is only held for the length of a check");
            if state.raised.is_some() {
                return true;
            }
            let now = Instant::now();
            if state
                .checked_at
                .is_some_and(|last| now.duration_since(last) < INTERRUPT_POLL_INTERVAL)
            {
                return false;
            }
            state.checked_at = Some(now);
        }
        // Re-attach to the interpreter for exactly as long as the check takes. On a
        // non-main thread CPython answers "nothing pending" without running a handler, so
        // an evaluator worker asking is correct and cheap rather than merely harmless.
        let Some(raised) = Python::attach(|py| py.check_signals().err()) else {
            return false;
        };
        self.interrupt
            .lock()
            .expect("the interrupt state is only held for the length of a check")
            .raised = Some(raised);
        // Publish only after the exception is stored. A detached evaluator thread that
        // observes this flag may return immediately, and `run_governed` must then be able
        // to take and re-raise the exact Python exception rather than laundering it into
        // an ordinary cancellation outcome.
        self.interrupted.store(true, Ordering::Release);
        true
    }
}

impl StopSignal for PyStopWatch {
    fn poll(&self) -> Option<StopCause> {
        if let Some(&cause) = self.latched.get() {
            return Some(cause);
        }
        if self.interrupted.load(Ordering::Acquire) {
            return Some(*self.latched.get_or_init(|| StopCause::Cancelled));
        }
        let cause = self.observe()?;
        Some(*self.latched.get_or_init(|| cause))
    }
}

/// The ceilings one governed call's keyword arguments carry, before they are engaged.
///
/// `None` in a slot means the caller declined that ceiling — never zero, which is a
/// perfectly valid ceiling that trips on the first charged unit of work.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct GovernorArgs {
    /// Abstract execution steps.
    pub(super) fuel: Option<u64>,
    /// Wall-clock budget in milliseconds. Zero expires on the first poll.
    pub(super) deadline_ms: Option<u64>,
    /// Units committed to the answer sequence: solution rows for `SELECT`, output
    /// statements for `CONSTRUCT`/`DESCRIBE`. Inclusive, and nothing for `ASK`.
    pub(super) max_answers: Option<u64>,
    /// The largest intermediate bag, in cells (`rows * columns`).
    pub(super) max_intermediate_cells: Option<u64>,
    /// Bytes minted into the per-query scratch arena by value-constructing operations.
    pub(super) max_scratch_bytes: Option<u64>,
    /// Requests issued to remote or federated endpoints.
    pub(super) max_remote_requests: Option<u64>,
}

impl GovernorArgs {
    /// Engage these ceilings, plus `cancel` and the interpreter's signal flag, as one
    /// call's [`QueryGovernors`].
    ///
    /// # Why the base is `METERED` rather than `UNBOUNDED`
    ///
    /// Two reasons, and both are about what a governed call promises. First, every
    /// outcome — including a complete one — carries evidence a caller can size the next
    /// budget from; `UNBOUNDED` reports nothing, because it charges nothing. Second, the
    /// evaluator polls the stop signal every `STOP_POLL_FUEL` units of fuel *and* at each
    /// algebra node it enters; with fuel disengaged only the second of those runs, so a
    /// query spending a long time inside one operator would notice a cancellation or a
    /// Ctrl-C late. Metering costs a saturating add per charge point and buys prompt
    /// interruption on every query shape, which is the trade a caller who asked for
    /// governors has already chosen.
    fn engage(self, cancel: Option<&PyCancellationToken>) -> (QueryGovernors, Arc<PyStopWatch>) {
        let watch = Arc::new(PyStopWatch::new(self.deadline_ms, cancel));
        let mut governors = QueryGovernors::METERED;
        if let Some(fuel) = self.fuel {
            governors = governors.with_fuel(fuel);
        }
        if let Some(rows) = self.max_answers {
            governors = governors.with_max_answers(rows);
        }
        if let Some(cells) = self.max_intermediate_cells {
            governors = governors.with_max_intermediate_cells(cells);
        }
        if let Some(bytes) = self.max_scratch_bytes {
            governors = governors.with_max_scratch_bytes(bytes);
        }
        if let Some(requests) = self.max_remote_requests {
            governors = governors.with_max_remote_requests(requests);
        }
        let signal: Arc<dyn StopSignal> = Arc::<PyStopWatch>::clone(&watch);
        (governors.with_stop_signal(signal), watch)
    }
}

/// Run `run` under the governors `args` describes, with the GIL released.
///
/// The engine call is the whole of the detached region, so a governor is enforced exactly
/// where the work happens and another Python thread — the one holding the caller's
/// [`PyCancellationToken`] — makes progress while it does.
///
/// # A Ctrl-C is raised; a tripped governor is not
///
/// `Python::check_signals` *consumes* the interpreter's pending signal and hands back the
/// exception its handler raised, so swallowing that exception would make a Ctrl-C
/// disappear. It is therefore re-raised here, ahead of both the outcome and any
/// evaluation error, and it is the one stop cause that leaves this seam as an exception.
/// A cancellation token, a deadline, and every resource ceiling leave it as an outcome —
/// see this module's header for why.
///
/// # Errors
///
/// The captured `KeyboardInterrupt`, if the interpreter raised one during the call, and
/// otherwise whatever `run` returned.
pub(super) fn run_governed<T: Send>(
    py: Python<'_>,
    args: GovernorArgs,
    cancel: Option<&PyCancellationToken>,
    run: impl FnOnce(&QueryGovernors) -> PyResult<T> + Send,
) -> PyResult<T> {
    let (governors, watch) = args.engage(cancel);
    let outcome = py.detach(|| run(&governors));
    if let Some(raised) = watch.take_interrupt() {
        return Err(raised);
    }
    outcome
}

/// The governor that stopped one execution: which one, on which dimension, against which
/// ceiling.
#[pyclass(name = "TrippedGovernor", frozen)]
#[derive(Debug)]
pub struct PyTrippedGovernor {
    /// The kernel value this object renders.
    inner: TrippedGovernor,
}

#[pymethods]
impl PyTrippedGovernor {
    /// Which kind of governor stopped the execution: `"budget"` (a ceiling was reached),
    /// `"stopped"` (a stop signal fired), or `"refused"` (the planner's estimate already
    /// exceeded a ceiling, so nothing ran).
    ///
    /// # The wildcard arm, here and on every accessor below
    ///
    /// The kernel's `TrippedGovernor` is `#[non_exhaustive]`, so this crate — foreign to
    /// the one that defines it — must carry a wildcard even though the enum is exhaustive
    /// today. A governor a future kernel adds and this build cannot name therefore reads
    /// `"unknown"` here and `None` on every field accessor, rather than being silently
    /// folded into a kind it is not. [`label`](Self::label) and `str(...)` still describe
    /// it exactly, because both come from the kernel rather than from this match.
    #[getter]
    const fn kind(&self) -> &'static str {
        match self.inner {
            TrippedGovernor::Budget { .. } => "budget",
            TrippedGovernor::Stopped { .. } => "stopped",
            TrippedGovernor::Refused { .. } => "refused",
            _ => "unknown",
        }
    }

    /// The stable kebab-case discriminant, e.g. `"answer-cap-exhausted"`. A pinned
    /// contract: match on this rather than on the prose of `str(...)`.
    #[getter]
    const fn label(&self) -> &'static str {
        self.inner.label()
    }

    /// The governed dimension, e.g. `"fuel"` — `None` when a stop signal fired, which
    /// belongs to no dimension.
    #[getter]
    const fn dimension(&self) -> Option<&'static str> {
        match self.inner {
            TrippedGovernor::Budget { dimension, .. }
            | TrippedGovernor::Refused { dimension, .. } => Some(dimension.label()),
            TrippedGovernor::Stopped { .. } => None,
            _ => None,
        }
    }

    /// The inclusive ceiling in force, or `None` when a stop signal fired.
    #[getter]
    const fn limit(&self) -> Option<u64> {
        match self.inner {
            TrippedGovernor::Budget { limit, .. } | TrippedGovernor::Refused { limit, .. } => {
                Some(limit)
            }
            TrippedGovernor::Stopped { .. } => None,
            _ => None,
        }
    }

    /// Consumption charged before the refused work — a **measurement**, and present only
    /// on the `"budget"` kind.
    #[getter]
    const fn consumed(&self) -> Option<u64> {
        match self.inner {
            TrippedGovernor::Budget { consumed, .. } => Some(consumed),
            TrippedGovernor::Stopped { .. } | TrippedGovernor::Refused { .. } => None,
            _ => None,
        }
    }

    /// The planner's estimate that exceeded the ceiling — **not** a measurement, and
    /// present only on the `"refused"` kind, where nothing ran to measure.
    #[getter]
    const fn estimate(&self) -> Option<u64> {
        match self.inner {
            TrippedGovernor::Refused { estimate, .. } => Some(estimate),
            TrippedGovernor::Budget { .. } | TrippedGovernor::Stopped { .. } => None,
            _ => None,
        }
    }

    /// Which stop signal fired — `"cancelled"` or `"deadline-exceeded"` — or `None` when
    /// a ceiling rather than a signal stopped the execution.
    #[getter]
    const fn cause(&self) -> Option<&'static str> {
        match self.inner {
            TrippedGovernor::Stopped { .. } => Some(self.inner.label()),
            TrippedGovernor::Budget { .. } | TrippedGovernor::Refused { .. } => None,
            _ => None,
        }
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("<TrippedGovernor {}>", self.inner.label())
    }
}

/// One governed execution's receipt: what it was allowed, what it spent, and what stopped
/// it.
///
/// Returned on the complete path as well as the exhausted one — "completed, cost N fuel,
/// peak M cells" is how a caller sizes the next call's budget in the first place.
#[pyclass(name = "GovernorEvidence", frozen)]
#[derive(Debug)]
pub struct PyGovernorEvidence {
    /// The kernel value this object renders.
    inner: GovernorEvidence,
}

#[pymethods]
impl PyGovernorEvidence {
    /// Consumption charged per dimension, keyed by the dimension's stable label.
    ///
    /// A peak-tracked dimension (`intermediate-cells`, `udf-depth`) reports the largest
    /// single observation; every other dimension reports the running sum.
    #[getter]
    fn consumed<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        vector_to_dict(py, self.inner.consumed())
    }

    /// The inclusive ceilings in force, keyed by the dimension's stable label.
    ///
    /// A dimension the caller declined reads `2**64 - 1`; a governed call meters every
    /// caller-settable dimension, so an unset one reads `2**64 - 2` — engaged, at a
    /// ceiling no execution can reach.
    #[getter]
    fn limits<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        vector_to_dict(py, self.inner.limits())
    }

    /// The governor that stopped the execution, or `None` if it completed.
    #[getter]
    fn tripped(&self, py: Python<'_>) -> PyResult<Option<Py<PyTrippedGovernor>>> {
        self.inner
            .tripped()
            .map(|inner| Py::new(py, PyTrippedGovernor { inner }))
            .transpose()
    }

    /// Whether the execution completed with every governor intact.
    #[getter]
    const fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Consumption charged on one dimension, named by its stable label.
    fn consumed_in(&self, dimension: &str) -> PyResult<u64> {
        Ok(self.inner.consumed_in(dimension_from_label(dimension)?))
    }

    /// The inclusive ceiling in force on one dimension, named by its stable label.
    fn limit_for(&self, dimension: &str) -> PyResult<u64> {
        Ok(self.inner.limit_for(dimension_from_label(dimension)?))
    }

    fn __repr__(&self) -> String {
        match self.inner.tripped() {
            None => "<GovernorEvidence complete>".to_owned(),
            Some(tripped) => format!("<GovernorEvidence tripped={}>", tripped.label()),
        }
    }
}

/// What the rows a truncated execution reached bound, relative to the query's true answer.
///
/// A three-way interval, not a yes/no: `"certain"` rows are a certified **lower** bound
/// and are safe to admit as answers; `"at-most"` rows are a certified **upper** bound and
/// are sound only for the negative reading (a row absent from them is definitively not an
/// answer); `"unknown"` means neither bound survived, so **no row is handed over at all**
/// and [`barrier`](Self::barrier) names the operator that withheld them instead.
#[pyclass(name = "PartialAnswers", frozen)]
#[derive(Debug)]
pub struct PyPartialAnswers {
    /// `"certain"`, `"at-most"`, or `"unknown"`.
    certainty: &'static str,
    /// The materialized rows, absent on the `"unknown"` class.
    result: Option<Py<PyAny>>,
    /// Whether those rows are the true answer's first rows, in order.
    positional_prefix: Option<bool>,
    /// The operator that withheld the rows, on the `"unknown"` class.
    barrier: Option<String>,
}

#[pymethods]
impl PyPartialAnswers {
    /// What these rows certify: `"certain"`, `"at-most"`, or `"unknown"`.
    #[getter]
    const fn certainty(&self) -> &'static str {
        self.certainty
    }

    /// Whether these rows are certified answers — i.e. whether they may be admitted.
    #[getter]
    fn is_certain(&self) -> bool {
        self.certainty == "certain"
    }

    /// The rows in hand, or `None` on the `"unknown"` class, where rows that bound the
    /// answer on neither side offer no sound use and one unsound one.
    #[getter]
    fn result(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.result.as_ref().map(|result| result.clone_ref(py))
    }

    /// Whether these rows are the true answer's **first** rows, in order. This licenses
    /// resumption by raising a deterministic ceiling; a wall-deadline rerun is fresh and
    /// may stop sooner. `None` on the `"unknown"` class.
    #[getter]
    const fn is_positional_prefix(&self) -> Option<bool> {
        self.positional_prefix
    }

    /// The algebra operator that withheld the rows, on the `"unknown"` class — which is
    /// what says whether a larger budget or a different query is the way forward.
    #[getter]
    fn barrier(&self) -> Option<&str> {
        self.barrier.as_deref()
    }

    fn __repr__(&self) -> String {
        match &self.barrier {
            Some(barrier) => format!("<PartialAnswers unknown barrier={barrier}>"),
            None => format!("<PartialAnswers {}>", self.certainty),
        }
    }
}

/// The outcome of one governed query: a complete answer, or an exhausted budget carrying
/// the partial answers the execution actually reached.
///
/// Exactly two shapes, and **neither is an exception**. A governor trip is not a failure:
/// raising it would throw away the rows the budget already paid for and tell the caller
/// the engine misbehaved. Check [`is_complete`](Self::is_complete), read
/// [`result`](Self::result) when it holds, and read [`tripped`](Self::tripped) with
/// [`partial`](Self::partial) when it does not.
#[pyclass(name = "QueryOutcome", frozen)]
#[derive(Debug)]
pub struct PyQueryOutcome {
    /// The complete result, present on the complete path only.
    result: Option<Py<PyAny>>,
    /// What the rows in hand bound, present on the exhausted path only.
    partial: Option<Py<PyPartialAnswers>>,
    /// The governor that stopped the execution, present on the exhausted path only.
    tripped: Option<Py<PyTrippedGovernor>>,
    /// This execution's consumption and ceilings, present on both paths.
    evidence: Py<PyGovernorEvidence>,
}

#[pymethods]
impl PyQueryOutcome {
    /// Whether every governor stayed intact and this is the query's complete answer.
    #[getter]
    const fn is_complete(&self) -> bool {
        self.tripped.is_none()
    }

    /// The **complete** result — `QuerySolutions`, `QueryTriples`, or `QueryBoolean` —
    /// or `None` when a governor stopped the execution.
    ///
    /// Deliberately never the partial rows: a caller that stopped reading the outcome one
    /// level too early receives nothing rather than a truncated answer wearing a complete
    /// answer's type. The rows a trip reached are on [`partial`](Self::partial), behind
    /// the certificate that says what they bound.
    #[getter]
    fn result(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.result.as_ref().map(|result| result.clone_ref(py))
    }

    /// What the rows the execution reached bound, or `None` when it completed.
    #[getter]
    fn partial(&self, py: Python<'_>) -> Option<Py<PyPartialAnswers>> {
        self.partial.as_ref().map(|partial| partial.clone_ref(py))
    }

    /// The governor that stopped the execution, or `None` when it completed.
    #[getter]
    fn tripped(&self, py: Python<'_>) -> Option<Py<PyTrippedGovernor>> {
        self.tripped.as_ref().map(|tripped| tripped.clone_ref(py))
    }

    /// This execution's consumption, ceilings, and trip — on both paths.
    #[getter]
    fn evidence(&self, py: Python<'_>) -> Py<PyGovernorEvidence> {
        self.evidence.clone_ref(py)
    }

    fn __repr__(&self) -> String {
        match &self.tripped {
            None => "<QueryOutcome complete>".to_owned(),
            Some(tripped) => format!("<QueryOutcome tripped={}>", tripped.get().label()),
        }
    }
}

/// The two-phase outcome of one governed entailment-aware query.
///
/// An answered run carries the ordinary [`PyQueryOutcome`] plus the byte-stable
/// reasoning report for the closure it queried. A closure stop carries neither: no query
/// ran and no closure exists to certify. [`tripped`](Self::tripped) spans both phases so a
/// host can make one retry/exit-code decision without erasing where the stop happened.
#[pyclass(name = "EntailmentQueryOutcome", frozen)]
#[derive(Debug)]
pub struct PyEntailmentQueryOutcome {
    phase: &'static str,
    outcome: Option<Py<PyQueryOutcome>>,
    report: Option<String>,
    tripped: Option<Py<PyTrippedGovernor>>,
}

#[pymethods]
impl PyEntailmentQueryOutcome {
    /// `"answered"` when closure completed, or `"closure-stopped"` when it did not.
    #[getter]
    const fn phase(&self) -> &'static str {
        self.phase
    }

    /// Whether both closure and query completed under every governor.
    #[getter]
    const fn is_complete(&self) -> bool {
        self.tripped.is_none()
    }

    /// Phase-two query outcome, absent when phase one stopped.
    #[getter]
    fn outcome(&self, py: Python<'_>) -> Option<Py<PyQueryOutcome>> {
        self.outcome.as_ref().map(|outcome| outcome.clone_ref(py))
    }

    /// Byte-stable reasoning report for the queried closure, absent when closure stopped.
    #[getter]
    fn report(&self) -> Option<&str> {
        self.report.as_deref()
    }

    /// The governor that stopped either phase, or `None` when both completed.
    #[getter]
    fn tripped(&self, py: Python<'_>) -> Option<Py<PyTrippedGovernor>> {
        self.tripped.as_ref().map(|tripped| tripped.clone_ref(py))
    }

    fn __repr__(&self) -> String {
        match &self.tripped {
            None => "<EntailmentQueryOutcome answered complete>".to_owned(),
            Some(tripped) => format!(
                "<EntailmentQueryOutcome {} tripped={}>",
                self.phase,
                tripped.get().label()
            ),
        }
    }
}

/// The outcome of one governed SPARQL UPDATE.
///
/// Deliberately not a [`PyQueryOutcome`] and deliberately without a partial arm: a
/// query's partial answer is a certifiable thing, a partial *mutation* is not. A tripped
/// request applied **nothing** — not "not all of it" — and left the store exactly as it
/// found it.
#[pyclass(name = "UpdateOutcome", frozen)]
#[derive(Debug)]
pub struct PyUpdateOutcome {
    /// The governor that stopped the request, present on the exhausted path only.
    tripped: Option<Py<PyTrippedGovernor>>,
    /// This request's consumption and ceilings, present on both paths.
    evidence: Py<PyGovernorEvidence>,
}

#[pymethods]
impl PyUpdateOutcome {
    /// Whether every operation of the request applied.
    ///
    /// `False` means **nothing** applied, never "not all of it applied".
    #[getter]
    const fn is_applied(&self) -> bool {
        self.tripped.is_none()
    }

    /// The governor that stopped the request, or `None` when it applied.
    #[getter]
    fn tripped(&self, py: Python<'_>) -> Option<Py<PyTrippedGovernor>> {
        self.tripped.as_ref().map(|tripped| tripped.clone_ref(py))
    }

    /// This request's consumption, ceilings, and trip — on both paths.
    #[getter]
    fn evidence(&self, py: Python<'_>) -> Py<PyGovernorEvidence> {
        self.evidence.clone_ref(py)
    }

    fn __repr__(&self) -> String {
        match &self.tripped {
            None => "<UpdateOutcome applied>".to_owned(),
            Some(tripped) => format!("<UpdateOutcome tripped={}>", tripped.get().label()),
        }
    }
}

/// The kernel dimension named by its stable kebab-case label.
fn dimension_from_label(label: &str) -> PyResult<ResourceDimension> {
    ResourceDimension::ALL
        .into_iter()
        .find(|dimension| dimension.label() == label)
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "unknown resource dimension `{label}`; expected one of {}",
                ResourceDimension::ALL
                    .map(ResourceDimension::label)
                    .join(", ")
            ))
        })
}

/// Render a resource vector as a `{dimension label: value}` dict, in the kernel's
/// declaration order so the mapping is deterministic across calls and builds.
fn vector_to_dict(py: Python<'_>, vector: ResourceVector) -> PyResult<Bound<'_, PyDict>> {
    let dict = PyDict::new(py);
    for dimension in ResourceDimension::ALL {
        dict.set_item(dimension.label(), vector.get(dimension))?;
    }
    Ok(dict)
}

/// Convert a native [`GovernedOutcome`] into the Python `QueryOutcome` object.
pub(crate) fn materialize_outcome(
    py: Python<'_>,
    outcome: GovernedOutcome,
) -> PyResult<Py<PyQueryOutcome>> {
    match outcome {
        GovernedOutcome::Complete {
            result, evidence, ..
        } => Py::new(
            py,
            PyQueryOutcome {
                result: Some(materialize_results(py, result)?),
                partial: None,
                tripped: None,
                evidence: Py::new(py, PyGovernorEvidence { inner: evidence })?,
            },
        ),
        GovernedOutcome::BudgetExhausted(BudgetExhausted {
            tripped,
            evidence,
            partial,
            ..
        }) => Py::new(
            py,
            PyQueryOutcome {
                result: None,
                partial: Some(materialize_partial(py, partial)?),
                tripped: Some(Py::new(py, PyTrippedGovernor { inner: tripped })?),
                evidence: Py::new(py, PyGovernorEvidence { inner: evidence })?,
            },
        ),
    }
}

/// Convert the native two-phase entailment carrier without dropping either phase's
/// evidence.
pub(crate) fn materialize_entailment_outcome(
    py: Python<'_>,
    outcome: GovernedEntailment,
) -> PyResult<Py<PyEntailmentQueryOutcome>> {
    match outcome {
        GovernedEntailment::Answered { outcome, report } => {
            let tripped = outcome
                .tripped()
                .map(|inner| Py::new(py, PyTrippedGovernor { inner }))
                .transpose()?;
            Py::new(
                py,
                PyEntailmentQueryOutcome {
                    phase: "answered",
                    outcome: Some(materialize_outcome(py, outcome)?),
                    report: Some(purrdf_validate::render_reasoning_report(&report)),
                    tripped,
                },
            )
        }
        GovernedEntailment::ClosureStopped { tripped } => Py::new(
            py,
            PyEntailmentQueryOutcome {
                phase: "closure-stopped",
                outcome: None,
                report: None,
                tripped: Some(Py::new(py, PyTrippedGovernor { inner: tripped })?),
            },
        ),
        _ => Err(PyRuntimeError::new_err(
            "unsupported governed entailment outcome",
        )),
    }
}

/// Convert a native [`GovernedUpdateOutcome`] into the Python `UpdateOutcome` object.
pub(crate) fn materialize_update_outcome(
    py: Python<'_>,
    outcome: &GovernedUpdateOutcome,
) -> PyResult<Py<PyUpdateOutcome>> {
    let tripped = outcome
        .tripped()
        .map(|inner| Py::new(py, PyTrippedGovernor { inner }))
        .transpose()?;
    Py::new(
        py,
        PyUpdateOutcome {
            tripped,
            evidence: Py::new(
                py,
                PyGovernorEvidence {
                    inner: outcome.evidence().clone(),
                },
            )?,
        },
    )
}

/// Convert the native certificate into the Python `PartialAnswers` object.
fn materialize_partial(py: Python<'_>, partial: PartialAnswers) -> PyResult<Py<PyPartialAnswers>> {
    let certainty = match partial {
        PartialAnswers::Certain(_) => "certain",
        PartialAnswers::AtMost(_) => "at-most",
        PartialAnswers::Unknown(_) => "unknown",
    };
    let barrier = partial
        .barrier()
        .map(|barrier| barrier.operator().to_owned());
    let (result, positional_prefix) = match partial.into_result() {
        Some(rows) => {
            let positional_prefix = rows.is_positional_prefix();
            (
                Some(materialize_results(py, rows.into_result())?),
                Some(positional_prefix),
            )
        }
        None => (None, None),
    };
    Py::new(
        py,
        PyPartialAnswers {
            certainty,
            result,
            positional_prefix,
            barrier,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SparqlEngine, SparqlRequest, parse_dataset};

    /// An `ASK` whose filter calls an IRI in call position under a would-be
    /// extension namespace.
    const EXT_CALL_ASK: &str = "ASK { FILTER(<https://ex.example/fn/nope>(1)) }";

    fn ask(engine: &NativeSparqlEngine) -> Result<SparqlResult, crate::RdfDiagnostic> {
        let dataset = parse_dataset(b"", "application/n-triples", None).expect("empty dataset");
        engine.query(
            &dataset,
            SparqlRequest {
                query: EXT_CALL_ASK,
                base_iri: None,
                substitutions: &[],
            },
        )
    }

    #[test]
    fn default_engine_leaves_extension_functions_off() {
        // Unset = engine defaults: the call-position IRI is an ordinary custom
        // function, so the query PARSES (any failure is an evaluation error,
        // never the extension seam's parse-time unknown-local-name hard-fail).
        if let Err(diag) = ask(&build_engine(EngineConfig::default())) {
            assert!(
                !diag.to_string().contains("parse"),
                "extensions-off must not fail at parse time: {diag}"
            );
        }
    }

    #[test]
    fn extension_namespaces_thread_into_parser_options() {
        // With the namespace configured, the UNKNOWN local name `nope` is a
        // parse-time hard error — proving the kwarg reached ParserOptions.
        let engine = build_engine(EngineConfig {
            extension_namespaces: Some(vec!["https://ex.example/fn/".to_owned()]),
            ..EngineConfig::default()
        });
        let diag = ask(&engine).expect_err("unknown extension local name must hard-fail");
        assert_eq!(diag.code, "native-sparql-query-parse", "{diag}");
    }

    #[test]
    fn standpoint_predicates_thread_into_the_engine() {
        let engine = build_engine(EngineConfig {
            standpoint_predicates: Some((
                "https://ex.example/accordingTo".to_owned(),
                "https://ex.example/sharpens".to_owned(),
            )),
            ..EngineConfig::default()
        });
        // The engine Debug surface reports the configured table (the engine's own
        // crate tests cover `heldIn` evaluation semantics end-to-end).
        assert!(
            format!("{engine:?}").contains("accordingTo"),
            "standpoint predicate table must be installed"
        );
    }

    /// A property-function namespace configured with NO registry: a predicate under
    /// it is lowered to a call node, and the call is refused as unregistered rather
    /// than quietly scanning a dataset that holds no such triple.
    #[test]
    fn property_fn_namespaces_thread_into_parser_options() {
        let engine = build_engine(EngineConfig {
            property_fn_namespaces: Some(vec!["https://ex.example/rel/".to_owned()]),
            ..EngineConfig::default()
        });
        let dataset = parse_dataset(b"", "application/n-triples", None).expect("empty dataset");
        let diag = engine
            .query(
                &dataset,
                SparqlRequest {
                    query: "ASK { ?s <https://ex.example/rel/nope> ?o }",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect_err("a call under a declared namespace with no relation must hard-fail");
        assert!(
            diag.to_string()
                .contains("no property function is registered"),
            "{diag}"
        );
    }

    /// The registry the Python surface builds is the kernel's own, so a ragged table
    /// is refused by [`MemoryRelation::new`] and surfaces as a Python `ValueError`.
    #[test]
    fn a_ragged_tuple_relation_is_refused() {
        let specs = vec![(
            "https://ex.example/rel/pairs".to_owned(),
            RelationSpec::Rows {
                subject_arity: 1,
                object_arity: 1,
                rows: vec![vec![TermValue::iri("https://ex.example/a")]],
            },
        )];
        let dataset = parse_dataset(b"", "application/n-triples", None).expect("empty dataset");
        let error = build_relations(specs, &dataset).expect_err("a one-cell row is not two wide");
        assert!(
            format!("{error}").contains("https://ex.example/rel/pairs"),
            "the refusal must name the relation: {error}"
        );
    }

    /// No declared relation is the ABSENCE of a registry, not an empty one — the
    /// configuration every pre-existing call keeps.
    #[test]
    fn no_declared_relation_attaches_no_registry() {
        let dataset = parse_dataset(b"", "application/n-triples", None).expect("empty dataset");
        assert!(
            build_relations(Vec::new(), &dataset)
                .expect("no relations is not an error")
                .is_none()
        );
    }
}
