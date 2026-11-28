// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! PyO3 boundary for the Rust-owned GTS fold view.

use purrdf_columnar::{ColumnarProjection, PhysicalType, ProjectionCell, Repetition, Table};
use purrdf_gts::model::{Graph, Term, TermKind};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyList, PyTuple};

use crate::gts_view::{ALL_SCOPE, GtsFoldView, GtsFoldViewConfig, PublicValue, RelationalRows};
use crate::py_store::presentation::native_error;

type PyTermRow = (
    u8,
    Option<String>,
    Option<usize>,
    Option<String>,
    Option<String>,
    Option<usize>,
    Option<(usize, usize, usize)>,
);

/// One statement-layer reifier row as Python exchanges it: `(reifier_id, (s, p, o),
/// graph?)`. The graph slot is the term id of the named graph the declaration was
/// asserted in (`None` = default graph); the RDF 1.2 statement layer is keyed per
/// graph, so it is part of the row's identity.
type PyReifierRow = (usize, (usize, usize, usize), Option<usize>);
/// One statement-layer annotation row: `(reifier_id, predicate, value, graph?)`; the
/// graph slot is the one described on [`PyReifierRow`].
type PyAnnotationRow = (usize, usize, usize, Option<usize>);

/// Whole-read limits selected explicitly for hostile GTS input.
///
/// Omitting `read_limits` from a byte reader preserves the unbounded v4
/// contract. Pass `GtsReadLimits()` for hostile input, tune its ceilings for
/// large admitted datasets, and use `unbounded()` only for trusted artifacts.
#[pyclass(name = "GtsReadLimits", frozen)]
#[derive(Clone, Copy, Debug)]
pub struct PyGtsReadLimits {
    pub(crate) inner: purrdf_gts::reader::GtsReadLimits,
}

#[pymethods]
impl PyGtsReadLimits {
    #[new]
    #[pyo3(signature = (*, encoded_input_bytes=None, items=None, encoded_frame_bytes=None, decoded_frame_bytes=None, decoded_blob_bytes=None, total_decoded_bytes=None, terms=None, rows=None, metadata_entries=None, diagnostics=None, work=None))]
    #[allow(
        clippy::too_many_arguments,
        reason = "the binding mirrors the one native read-limit record"
    )]
    fn new(
        encoded_input_bytes: Option<usize>,
        items: Option<usize>,
        encoded_frame_bytes: Option<usize>,
        decoded_frame_bytes: Option<usize>,
        decoded_blob_bytes: Option<usize>,
        total_decoded_bytes: Option<usize>,
        terms: Option<usize>,
        rows: Option<usize>,
        metadata_entries: Option<usize>,
        diagnostics: Option<usize>,
        work: Option<usize>,
    ) -> Self {
        let defaults = purrdf_gts::reader::GtsReadLimits::DEFAULT;
        Self {
            inner: purrdf_gts::reader::GtsReadLimits {
                encoded_input_bytes: encoded_input_bytes.unwrap_or(defaults.encoded_input_bytes),
                items: items.unwrap_or(defaults.items),
                encoded_frame_bytes: encoded_frame_bytes.unwrap_or(defaults.encoded_frame_bytes),
                decoded_frame_bytes: decoded_frame_bytes.unwrap_or(defaults.decoded_frame_bytes),
                decoded_blob_bytes: decoded_blob_bytes.unwrap_or(defaults.decoded_blob_bytes),
                total_decoded_bytes: total_decoded_bytes.unwrap_or(defaults.total_decoded_bytes),
                terms: terms.unwrap_or(defaults.terms),
                rows: rows.unwrap_or(defaults.rows),
                metadata_entries: metadata_entries.unwrap_or(defaults.metadata_entries),
                diagnostics: diagnostics.unwrap_or(defaults.diagnostics),
                work: work.unwrap_or(defaults.work),
            },
        }
    }

    #[staticmethod]
    fn unbounded() -> Self {
        Self {
            inner: purrdf_gts::reader::GtsReadLimits::UNBOUNDED,
        }
    }

    #[getter]
    const fn encoded_input_bytes(&self) -> usize {
        self.inner.encoded_input_bytes
    }
    #[getter]
    const fn items(&self) -> usize {
        self.inner.items
    }
    #[getter]
    const fn encoded_frame_bytes(&self) -> usize {
        self.inner.encoded_frame_bytes
    }
    #[getter]
    const fn decoded_frame_bytes(&self) -> usize {
        self.inner.decoded_frame_bytes
    }
    #[getter]
    const fn decoded_blob_bytes(&self) -> usize {
        self.inner.decoded_blob_bytes
    }
    #[getter]
    const fn total_decoded_bytes(&self) -> usize {
        self.inner.total_decoded_bytes
    }
    #[getter]
    const fn terms(&self) -> usize {
        self.inner.terms
    }
    #[getter]
    const fn rows(&self) -> usize {
        self.inner.rows
    }
    #[getter]
    const fn metadata_entries(&self) -> usize {
        self.inner.metadata_entries
    }
    #[getter]
    const fn diagnostics(&self) -> usize {
        self.inner.diagnostics
    }
    #[getter]
    const fn work(&self) -> usize {
        self.inner.work
    }
}

#[pyclass(name = "GtsFoldViewNative")]
#[derive(Debug)]
pub struct PyGtsFoldView {
    inner: GtsFoldView,
}

#[pymethods]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
impl PyGtsFoldView {
    /// `curie_prefixes` are the caller's `(prefix, namespace)` CURIE entries,
    /// highest priority first; the view builds in only the W3C namespaces.
    #[staticmethod]
    #[pyo3(signature = (data, curie_prefixes = Vec::new(), *, read_limits = None))]
    fn from_bytes(
        py: Python<'_>,
        data: &[u8],
        curie_prefixes: Vec<(String, String)>,
        read_limits: Option<PyRef<'_, PyGtsReadLimits>>,
    ) -> PyResult<Self> {
        let read_limits = read_limits.as_ref().map(|limits| limits.inner);
        // A fold diagnostic is an error here exactly as in every other byte consumer:
        // a damaged file never becomes an empty or partial view.
        let graph = py
            .detach(|| match read_limits {
                Some(limits) => purrdf::gts::read_graph_with_limits(data, true, limits),
                None => purrdf::gts::read_graph(data, true),
            })
            .map_err(|diagnostic| native_error(py, &diagnostic))?;
        py.detach(|| {
            Ok(Self {
                inner: fold_view(graph, curie_prefixes)?,
            })
        })
    }

    #[staticmethod]
    #[pyo3(signature = (terms, quads, reifiers, annotations, curie_prefixes = Vec::new()))]
    fn from_parts(
        py: Python<'_>,
        terms: Vec<PyTermRow>,
        quads: Vec<(usize, usize, usize, Option<usize>)>,
        reifiers: Vec<PyReifierRow>,
        annotations: Vec<PyAnnotationRow>,
        curie_prefixes: Vec<(String, String)>,
    ) -> PyResult<Self> {
        py.detach(|| {
            let graph = graph_from_parts(terms, quads, reifiers, annotations)?;
            Ok(Self {
                inner: fold_view(graph, curie_prefixes)?,
            })
        })
    }

    fn term_count(&self) -> usize {
        self.inner.graph().terms.len()
    }

    fn quad_count(&self) -> usize {
        self.inner.graph().quads.len()
    }

    fn reifier_count(&self) -> usize {
        self.inner.reifiers().len()
    }

    fn annotation_count(&self) -> usize {
        self.inner.annotations().len()
    }

    fn term_tuple(&self, tid: usize) -> PyResult<PyTermRow> {
        let term = self.term_ref(tid)?;
        Ok((
            term.kind.to_wire(),
            term.value.clone(),
            term.datatype,
            term.lang.clone(),
            term.direction.clone(),
            term.reifier,
            term.triple,
        ))
    }

    fn is_iri(&self, tid: usize) -> PyResult<bool> {
        self.ensure_tid(tid)?;
        Ok(self.inner.is_iri(tid))
    }

    fn is_bnode(&self, tid: usize) -> PyResult<bool> {
        self.ensure_tid(tid)?;
        Ok(self.inner.is_bnode(tid))
    }

    fn is_literal(&self, tid: usize) -> PyResult<bool> {
        self.ensure_tid(tid)?;
        Ok(self.inner.is_literal(tid))
    }

    fn iri(&self, tid: usize) -> PyResult<Option<String>> {
        self.ensure_tid(tid)?;
        Ok(self.inner.iri(tid).map(str::to_string))
    }

    fn lex(&self, tid: usize) -> PyResult<String> {
        self.ensure_tid(tid)?;
        Ok(self.inner.lex(tid).to_string())
    }

    fn lang(&self, tid: usize) -> PyResult<Option<String>> {
        self.ensure_tid(tid)?;
        Ok(self.inner.lang(tid).map(str::to_string))
    }

    fn datatype(&self, tid: usize) -> PyResult<String> {
        self.ensure_tid(tid)?;
        Ok(self.inner.datatype(tid))
    }

    fn nq_token(&self, tid: usize) -> PyResult<String> {
        self.ensure_tid(tid)?;
        Ok(self.inner.nq_token(tid))
    }

    fn python_value(&self, py: Python<'_>, tid: usize) -> PyResult<Py<PyAny>> {
        self.ensure_tid(tid)?;
        match self.inner.public_value(tid) {
            PublicValue::Iri(value) | PublicValue::Blank(value) | PublicValue::String(value) => {
                Ok(value.into_pyobject(py)?.unbind().into())
            }
            PublicValue::Integer(value) => Ok(value.into_pyobject(py)?.unbind().into()),
            PublicValue::Float(value) => Ok(value.into_pyobject(py)?.unbind().into()),
            PublicValue::Boolean(value) => Ok(PyBool::new(py, value).to_owned().unbind().into()),
            PublicValue::LanguageString { value, lang } => {
                let d = PyDict::new(py);
                d.set_item("value", value)?;
                d.set_item("lang", lang)?;
                Ok(d.unbind().into())
            }
        }
    }

    fn tid_of_iri(&self, iri: &str) -> Option<usize> {
        self.inner.tid_of_iri(iri)
    }

    fn curie(&self, iri: &str) -> String {
        self.inner.curie(iri)
    }

    #[pyo3(signature = (scope=None))]
    fn quads(&self, scope: Option<String>) -> Vec<(usize, usize, usize, Option<usize>)> {
        self.inner.quads(scope.as_deref())
    }

    #[pyo3(signature = (class_iri, scope=None))]
    fn subjects_by_type(&self, class_iri: &str, scope: Option<String>) -> Vec<usize> {
        self.inner.subjects_by_type(class_iri, scope.as_deref())
    }

    #[pyo3(signature = (s_tid, p_iri, scope=None))]
    fn objects(&self, s_tid: usize, p_iri: &str, scope: Option<String>) -> Vec<usize> {
        self.inner.objects(s_tid, p_iri, scope.as_deref())
    }

    #[pyo3(signature = (s_tid, p_iri, scope=None))]
    fn value(&self, s_tid: usize, p_iri: &str, scope: Option<String>) -> Option<usize> {
        self.inner.value(s_tid, p_iri, scope.as_deref())
    }

    #[pyo3(signature = (s_tid, scope=None))]
    fn predicate_objects(&self, s_tid: usize, scope: Option<String>) -> Vec<(usize, usize)> {
        self.inner.predicate_objects(s_tid, scope.as_deref())
    }

    #[pyo3(signature = (s_tid, p_iri, o_tid, scope=None))]
    fn has(&self, s_tid: usize, p_iri: &str, o_tid: usize, scope: Option<String>) -> bool {
        self.inner.has(s_tid, p_iri, o_tid, scope.as_deref())
    }

    #[pyo3(signature = (head_tid, scope=None))]
    fn rdf_list(&self, head_tid: usize, scope: Option<String>) -> Vec<usize> {
        self.inner.rdf_list(head_tid, scope.as_deref())
    }

    /// `(reifier_id, (s, p, o), graph?)` — the graph slot is the term id of the named
    /// graph the declaration was asserted in (`None` = default graph). It is carried,
    /// not dropped: the RDF 1.2 statement layer is keyed per graph, so one reifier id
    /// may be declared in several graphs and those are distinct rows.
    fn reifiers(&self) -> Vec<PyReifierRow> {
        self.inner.reifiers().to_vec()
    }

    /// `(reifier_id, predicate, value, graph?)`; the graph slot mirrors
    /// [`Self::reifiers`].
    fn annotations(&self) -> Vec<PyAnnotationRow> {
        self.inner.annotations().to_vec()
    }

    fn tag_map(&self) -> BTreeMapString {
        BTreeMapString(self.inner.tag_map().clone())
    }

    fn available_languages(&self) -> Vec<String> {
        self.inner.available_languages().into_iter().collect()
    }

    #[pyo3(signature = (s_tid, p_iri, scope=None))]
    fn public_text(&self, s_tid: usize, p_iri: &str, scope: Option<String>) -> String {
        self.inner.public_text(s_tid, p_iri, scope.as_deref())
    }

    #[pyo3(signature = (s_tid, p_iri, scope=None))]
    fn public_literal(
        &self,
        s_tid: usize,
        p_iri: &str,
        scope: Option<String>,
    ) -> (String, Option<String>) {
        self.inner.public_literal(s_tid, p_iri, scope.as_deref())
    }

    #[pyo3(signature = (s_tid, p_iri, requested, scope=None))]
    fn public_literal_with_fallback(
        &self,
        s_tid: usize,
        p_iri: &str,
        requested: Vec<String>,
        scope: Option<String>,
    ) -> (String, Option<String>, bool) {
        self.inner
            .public_literal_with_fallback(s_tid, p_iri, &requested, scope.as_deref())
    }

    #[pyo3(signature = (s_tid, p_iri, requested, scope=None))]
    fn public_text_with_fallback(
        &self,
        s_tid: usize,
        p_iri: &str,
        requested: Vec<String>,
        scope: Option<String>,
    ) -> (String, bool) {
        let (text, _lang, fallback) =
            self.inner
                .public_literal_with_fallback(s_tid, p_iri, &requested, scope.as_deref());
        (text, fallback)
    }

    #[pyo3(signature = (s_tid, p_iri, requested, scope=None))]
    fn public_texts(
        &self,
        s_tid: usize,
        p_iri: &str,
        requested: Vec<String>,
        scope: Option<String>,
    ) -> Vec<(String, Option<String>, bool)> {
        self.inner
            .public_texts(s_tid, p_iri, &requested, scope.as_deref())
    }

    fn relational_rows<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let view = &self.inner;
        let (rows, directions) = py
            .detach(|| {
                view.relational_rows()
                    .map(|rows| (rows, crate::gts_view::term_directions(view.graph())))
            })
            .map_err(crate::py_gts::native_gts_error)?;
        relational_rows_dict(py, rows, directions)
    }
}

impl PyGtsFoldView {
    fn term_ref(&self, tid: usize) -> PyResult<&Term> {
        self.inner
            .graph()
            .terms
            .get(tid)
            .ok_or_else(|| PyValueError::new_err(format!("term id out of range: {tid}")))
    }

    fn ensure_tid(&self, tid: usize) -> PyResult<()> {
        self.term_ref(tid).map(|_| ())
    }
}

#[pyfunction]
#[pyo3(signature = (data, *, read_limits = None))]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts optional Python borrows by value"
)]
fn gts_relational_rows_from_bytes<'py>(
    py: Python<'py>,
    data: &[u8],
    read_limits: Option<PyRef<'_, PyGtsReadLimits>>,
) -> PyResult<Bound<'py, PyDict>> {
    let read_limits = read_limits.as_ref().map(|limits| limits.inner);
    let graph = py
        .detach(|| match read_limits {
            Some(limits) => purrdf::gts::read_graph_with_limits(data, true, limits),
            None => purrdf::gts::read_graph(data, true),
        })
        .map_err(|diagnostic| native_error(py, &diagnostic))?;
    let (rows, directions) = py
        .detach(|| {
            crate::gts_view::relational_rows(&graph)
                .map(|rows| (rows, crate::gts_view::term_directions(&graph)))
        })
        .map_err(crate::py_gts::native_gts_error)?;
    relational_rows_dict(py, rows, directions)
}

pub(crate) fn columnar_rows_dict<'py>(
    py: Python<'py>,
    projection: &ColumnarProjection,
) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    let schemas = PyDict::new(py);
    for table in Table::ALL {
        let schema = table.schema();
        let columns: Vec<_> = schema
            .columns
            .iter()
            .map(|column| {
                (
                    column.name,
                    match column.physical_type {
                        PhysicalType::Int64 => "integer",
                        PhysicalType::ByteArray if column.utf8 => "text",
                        PhysicalType::ByteArray => "bytes",
                    },
                    column.repetition == Repetition::Optional,
                )
            })
            .collect();
        schemas.set_item(table.name(), columns)?;
        let rows = PyList::empty(py);
        for row in projection.rows(table) {
            let mut cells = Vec::with_capacity(row.len());
            for (cell, column) in row.into_iter().zip(schema.columns) {
                let value = match cell {
                    ProjectionCell::Null => py.None(),
                    ProjectionCell::Int64(value) => value.into_pyobject(py)?.into_any().unbind(),
                    ProjectionCell::Bytes(bytes) if column.utf8 => {
                        let value = std::str::from_utf8(bytes)
                            .map_err(|error| PyValueError::new_err(error.to_string()))?;
                        value.into_pyobject(py)?.into_any().unbind()
                    }
                    ProjectionCell::Bytes(bytes) => PyBytes::new(py, bytes).into_any().unbind(),
                };
                cells.push(value);
            }
            rows.append(PyTuple::new(py, cells)?)?;
        }
        out.set_item(table.name(), rows)?;
    }
    out.set_item("schema", schemas)?;
    Ok(out)
}

pub(crate) fn columnar_parquet_dict<'py>(
    py: Python<'py>,
    files: &purrdf_columnar::ParquetFiles,
) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    for (table, bytes) in files.iter() {
        out.set_item(table.file_name(), PyBytes::new(py, bytes))?;
    }
    Ok(out)
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGtsReadLimits>()?;
    m.add_class::<PyGtsFoldView>()?;
    m.add("GTS_ALL_SCOPE", ALL_SCOPE)?;
    m.add_function(wrap_pyfunction!(gts_relational_rows_from_bytes, m)?)?;
    crate::py_gts_export::register(m)?;
    Ok(())
}

/// Build the Rust-owned view, turning the fold's refusal into a `ValueError`.
///
/// A triple term that reaches itself states no triple, so every accessor that renders
/// a quoted triple (`nq_token`, `python_value`, the public-text family) terminates;
/// a literal datatype chain that reaches itself is refused, because an unbounded walk
/// overflows the stack — which in Rust aborts the process, killing the interpreter
/// outright rather than raising anything Python could catch. `from_parts` accepts a
/// term table straight from the caller, so this is the boundary where that is
/// stopped: term ids are range-checked in [`validate_terms`], and the shape they
/// describe is checked for termination here.
fn fold_view(graph: Graph, curie_prefixes: Vec<(String, String)>) -> PyResult<GtsFoldView> {
    let config = GtsFoldViewConfig {
        language_vocab: None,
        curie_prefixes,
    };
    GtsFoldView::with_config(graph, config)
        .map_err(|diagnostic| PyValueError::new_err(diagnostic.to_string()))
}

fn graph_from_parts(
    terms: Vec<PyTermRow>,
    quads: Vec<(usize, usize, usize, Option<usize>)>,
    reifiers: Vec<PyReifierRow>,
    annotations: Vec<PyAnnotationRow>,
) -> PyResult<Graph> {
    let term_count = terms.len();
    validate_terms(&terms, term_count)?;
    validate_rows(&quads, term_count, "quads", ["s", "p", "o", "g"])?;
    validate_reifiers(&reifiers, term_count)?;
    validate_rows(
        &annotations,
        term_count,
        "annotations",
        ["reifier", "predicate", "value", "g"],
    )?;
    Ok(Graph {
        terms: terms
            .into_iter()
            .map(
                |(kind, value, datatype, lang, direction, reifier, triple)| {
                    Ok(Term {
                        kind: term_kind(kind)?,
                        value,
                        datatype,
                        lang,
                        direction,
                        reifier,
                        triple,
                    })
                },
            )
            .collect::<PyResult<Vec<_>>>()?,
        quads,
        // The Python rows are already the 0.9.11 row-array shape, graph slot included:
        // the statement layer is keyed per graph, so the caller states the graph each
        // declaration and annotation was asserted in (`None` = default graph).
        reifiers,
        annotations,
        ..Graph::default()
    })
}

fn term_kind(kind: u8) -> PyResult<TermKind> {
    match kind {
        0 => Ok(TermKind::Iri),
        1 => Ok(TermKind::Literal),
        2 => Ok(TermKind::Bnode),
        3 => Ok(TermKind::Triple),
        _ => Err(PyValueError::new_err(format!(
            "unknown GTS term kind: {kind}"
        ))),
    }
}

fn validate_terms(terms: &[PyTermRow], term_count: usize) -> PyResult<()> {
    for (idx, (_, _value, datatype, lang, _direction, reifier, triple)) in terms.iter().enumerate()
    {
        // `from_parts` builds a `Graph` WITHOUT going through `reader::read`, so
        // the reader's language-tag gate (`purrdf_gts::reader`'s `h_terms`) never
        // sees this row — and `GtsFoldView::nq_token` renders `Term::lang`
        // straight into an N-Quads `LANGTAG` token. Ask the same grammar, on the
        // same profile, that the byte path asks. This surface is a constructor,
        // not a byte reader, so it refuses hard rather than degrading: every
        // other malformed field in this row already raises here.
        if let Some(tag) = lang
            && let Some(code) = purrdf_gts::model::language_tag_refusal(tag)
        {
            return Err(PyValueError::new_err(format!(
                "terms[{idx}].lang: {tag:?} is not a language tag the RDF concrete-syntax \
                 grammar accepts ({code})"
            )));
        }
        validate_optional_term_id(*datatype, term_count, &format!("terms[{idx}].datatype"))?;
        validate_optional_term_id(*reifier, term_count, &format!("terms[{idx}].reifier"))?;
        // A self-describing quoted triple names its own components; every id is
        // validated exactly like the reifier slot, so no silent acceptance. Range is
        // all this pass can say: whether the SHAPE those ids describe terminates is a
        // property of the whole table, checked once in `fold_view`.
        if let Some((s, p, o)) = *triple {
            validate_term_id(s, term_count, &format!("terms[{idx}].triple.s"))?;
            validate_term_id(p, term_count, &format!("terms[{idx}].triple.p"))?;
            validate_term_id(o, term_count, &format!("terms[{idx}].triple.o"))?;
        }
    }
    Ok(())
}

fn validate_reifiers(reifiers: &[PyReifierRow], term_count: usize) -> PyResult<()> {
    for (idx, (r, (s, p, o), g)) in reifiers.iter().enumerate() {
        validate_term_id(*r, term_count, &format!("reifiers[{idx}].reifier"))?;
        validate_term_id(*s, term_count, &format!("reifiers[{idx}].s"))?;
        validate_term_id(*p, term_count, &format!("reifiers[{idx}].p"))?;
        validate_term_id(*o, term_count, &format!("reifiers[{idx}].o"))?;
        validate_optional_term_id(*g, term_count, &format!("reifiers[{idx}].g"))?;
    }
    Ok(())
}

/// Bounds-check every row of a four-column id table — three required term ids and
/// an optional graph id — naming a failure `table[row].column`. The one checker the
/// quad and annotation tables share: both are `(id, id, id, Option<id>)` rows and
/// differ only in their column names.
fn validate_rows(
    rows: &[(usize, usize, usize, Option<usize>)],
    term_count: usize,
    table: &str,
    [first, second, third, graph]: [&str; 4],
) -> PyResult<()> {
    for (idx, (a, b, c, g)) in rows.iter().enumerate() {
        validate_term_id(*a, term_count, &format!("{table}[{idx}].{first}"))?;
        validate_term_id(*b, term_count, &format!("{table}[{idx}].{second}"))?;
        validate_term_id(*c, term_count, &format!("{table}[{idx}].{third}"))?;
        validate_optional_term_id(*g, term_count, &format!("{table}[{idx}].{graph}"))?;
    }
    Ok(())
}

fn validate_optional_term_id(tid: Option<usize>, term_count: usize, label: &str) -> PyResult<()> {
    if let Some(tid) = tid {
        validate_term_id(tid, term_count, label)?;
    }
    Ok(())
}

fn validate_term_id(tid: usize, term_count: usize, label: &str) -> PyResult<()> {
    if tid < term_count {
        return Ok(());
    }
    Err(PyValueError::new_err(format!(
        "{label} term id out of range: {tid} >= {term_count}"
    )))
}

/// Build the projection dict.
///
/// `directions` is a SEPARATE key positionally parallel to `terms`, not an eighth
/// column inside each term row. Python callers unpack those rows positionally, so
/// widening them breaks at runtime with no type boundary to catch it; a new key
/// breaks nothing that does not already ask for it.
fn relational_rows_dict(
    py: Python<'_>,
    rows: RelationalRows,
    directions: Vec<Option<String>>,
) -> PyResult<Bound<'_, PyDict>> {
    let out = PyDict::new(py);
    out.set_item("terms", rows.terms)?;
    out.set_item("directions", directions)?;
    out.set_item("quads", rows.quads)?;
    out.set_item("reifiers", rows.reifiers)?;
    out.set_item("annotations", rows.annotations)?;
    let blobs = PyList::empty(py);
    for (digest, bytes) in rows.blobs {
        blobs.append((digest, PyBytes::new(py, &bytes)))?;
    }
    out.set_item("blobs", blobs)?;
    Ok(out)
}

struct BTreeMapString(std::collections::BTreeMap<String, String>);

impl<'py> IntoPyObject<'py> for BTreeMapString {
    type Target = PyDict;
    type Output = Bound<'py, PyDict>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let out = PyDict::new(py);
        for (key, value) in self.0 {
            out.set_item(key, value)?;
        }
        Ok(out)
    }
}
