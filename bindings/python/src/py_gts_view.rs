// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 boundary for the Rust-owned GTS fold view.

use purrdf_gts::model::{Graph, Term, TermKind};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyList};

use crate::gts_view::{GtsFoldView, PublicValue, RelationalRows, ALL_SCOPE};

type PyTermRow = (
    u8,
    Option<String>,
    Option<usize>,
    Option<String>,
    Option<String>,
    Option<usize>,
);

#[pyclass(name = "GtsFoldViewNative")]
#[derive(Debug)]
pub struct PyGtsFoldView {
    inner: GtsFoldView,
}

#[pymethods]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
impl PyGtsFoldView {
    #[staticmethod]
    fn from_bytes(py: Python<'_>, data: &[u8]) -> Self {
        py.detach(|| {
            let graph = purrdf_gts::reader::read(data, true, None);
            Self {
                inner: GtsFoldView::new(graph),
            }
        })
    }

    #[staticmethod]
    fn from_parts(
        py: Python<'_>,
        terms: Vec<PyTermRow>,
        quads: Vec<(usize, usize, usize, Option<usize>)>,
        reifiers: Vec<(usize, (usize, usize, usize))>,
        annotations: Vec<(usize, usize, usize)>,
    ) -> PyResult<Self> {
        py.detach(|| {
            let graph = graph_from_parts(terms, quads, reifiers, annotations)?;
            Ok(Self {
                inner: GtsFoldView::new(graph),
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
            term_kind_int(term.kind),
            term.value.clone(),
            term.datatype,
            term.lang.clone(),
            term.direction.clone(),
            term.reifier,
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

    fn quads(&self, scope: Option<String>) -> Vec<(usize, usize, usize, Option<usize>)> {
        self.inner.quads(scope.as_deref())
    }

    fn subjects_by_type(&self, class_iri: &str, scope: Option<String>) -> Vec<usize> {
        self.inner.subjects_by_type(class_iri, scope.as_deref())
    }

    fn objects(&self, s_tid: usize, p_iri: &str, scope: Option<String>) -> Vec<usize> {
        self.inner.objects(s_tid, p_iri, scope.as_deref())
    }

    fn value(&self, s_tid: usize, p_iri: &str, scope: Option<String>) -> Option<usize> {
        self.inner.value(s_tid, p_iri, scope.as_deref())
    }

    fn predicate_objects(&self, s_tid: usize, scope: Option<String>) -> Vec<(usize, usize)> {
        self.inner.predicate_objects(s_tid, scope.as_deref())
    }

    fn has(&self, s_tid: usize, p_iri: &str, o_tid: usize, scope: Option<String>) -> bool {
        self.inner.has(s_tid, p_iri, o_tid, scope.as_deref())
    }

    fn rdf_list(&self, head_tid: usize, scope: Option<String>) -> Vec<usize> {
        self.inner.rdf_list(head_tid, scope.as_deref())
    }

    fn reifiers(&self) -> Vec<(usize, (usize, usize, usize))> {
        // The Python projection carries no graph axis (purrdf reification is
        // standpoint-scoped); drop the always-`None` 0.9.11 graph slot.
        self.inner
            .reifiers()
            .iter()
            .map(|&(rid, spo, _graph)| (rid, spo))
            .collect()
    }

    fn annotations(&self) -> Vec<(usize, usize, usize)> {
        self.inner
            .annotations()
            .iter()
            .map(|&(r, p, o, _graph)| (r, p, o))
            .collect()
    }

    fn tag_map(&self) -> BTreeMapString {
        BTreeMapString(self.inner.tag_map().clone())
    }

    fn available_languages(&self) -> Vec<String> {
        self.inner.available_languages().into_iter().collect()
    }

    fn public_text(&self, s_tid: usize, p_iri: &str, scope: Option<String>) -> String {
        self.inner.public_text(s_tid, p_iri, scope.as_deref())
    }

    fn public_literal(
        &self,
        s_tid: usize,
        p_iri: &str,
        scope: Option<String>,
    ) -> (String, Option<String>) {
        self.inner.public_literal(s_tid, p_iri, scope.as_deref())
    }

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
        let rows = py
            .detach(|| view.relational_rows())
            .map_err(PyValueError::new_err)?;
        relational_rows_dict(py, rows)
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
fn gts_relational_rows_from_bytes<'py>(
    py: Python<'py>,
    data: &[u8],
) -> PyResult<Bound<'py, PyDict>> {
    let rows = py
        .detach(|| {
            let graph = purrdf_gts::reader::read(data, true, None);
            crate::gts_view::relational_rows(&graph)
        })
        .map_err(PyValueError::new_err)?;
    relational_rows_dict(py, rows)
}

#[pyfunction]
fn gts_to_sqlite(data: &[u8], path: &str) -> PyResult<String> {
    let _ = (data, path);
    Err(PyValueError::new_err(
        "gts_to_sqlite is pending reimplementation on purrdf primitives",
    ))
}

#[pyfunction]
fn gts_to_duckdb(data: &[u8], path: &str) -> PyResult<String> {
    let _ = (data, path);
    Err(PyValueError::new_err(
        "gts_to_duckdb is pending reimplementation on purrdf primitives",
    ))
}

#[pyfunction]
fn gts_to_parquet(data: &[u8], out_dir: &str) -> PyResult<Vec<String>> {
    let _ = (data, out_dir);
    Err(PyValueError::new_err(
        "gts_to_parquet is pending reimplementation on purrdf primitives",
    ))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGtsFoldView>()?;
    m.add("GTS_ALL_SCOPE", ALL_SCOPE)?;
    m.add_function(wrap_pyfunction!(gts_relational_rows_from_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(gts_to_sqlite, m)?)?;
    m.add_function(wrap_pyfunction!(gts_to_duckdb, m)?)?;
    m.add_function(wrap_pyfunction!(gts_to_parquet, m)?)?;
    Ok(())
}

fn graph_from_parts(
    terms: Vec<PyTermRow>,
    quads: Vec<(usize, usize, usize, Option<usize>)>,
    reifiers: Vec<(usize, (usize, usize, usize))>,
    annotations: Vec<(usize, usize, usize)>,
) -> PyResult<Graph> {
    let term_count = terms.len();
    validate_terms(&terms, term_count)?;
    validate_quads(&quads, term_count)?;
    validate_reifiers(&reifiers, term_count)?;
    validate_annotations(&annotations, term_count)?;
    Ok(Graph {
        terms: terms
            .into_iter()
            .map(|(kind, value, datatype, lang, direction, reifier)| {
                Ok(Term {
                    kind: term_kind(kind)?,
                    value,
                    datatype,
                    lang,
                    direction,
                    reifier,
                })
            })
            .collect::<PyResult<Vec<_>>>()?,
        quads,
        // Widen the narrow Python rows to the 0.9.11 row-array; purrdf rows are
        // never graph-scoped, so the graph slot is `None`.
        reifiers: reifiers
            .into_iter()
            .map(|(rid, spo)| (rid, spo, None))
            .collect(),
        annotations: annotations
            .into_iter()
            .map(|(r, p, o)| (r, p, o, None))
            .collect(),
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
    for (idx, (_, _value, datatype, _lang, _direction, reifier)) in terms.iter().enumerate() {
        validate_optional_term_id(*datatype, term_count, &format!("terms[{idx}].datatype"))?;
        validate_optional_term_id(*reifier, term_count, &format!("terms[{idx}].reifier"))?;
    }
    Ok(())
}

fn validate_quads(
    quads: &[(usize, usize, usize, Option<usize>)],
    term_count: usize,
) -> PyResult<()> {
    for (idx, (s, p, o, g)) in quads.iter().enumerate() {
        validate_term_id(*s, term_count, &format!("quads[{idx}].s"))?;
        validate_term_id(*p, term_count, &format!("quads[{idx}].p"))?;
        validate_term_id(*o, term_count, &format!("quads[{idx}].o"))?;
        validate_optional_term_id(*g, term_count, &format!("quads[{idx}].g"))?;
    }
    Ok(())
}

fn validate_reifiers(
    reifiers: &[(usize, (usize, usize, usize))],
    term_count: usize,
) -> PyResult<()> {
    for (idx, (r, (s, p, o))) in reifiers.iter().enumerate() {
        validate_term_id(*r, term_count, &format!("reifiers[{idx}].reifier"))?;
        validate_term_id(*s, term_count, &format!("reifiers[{idx}].s"))?;
        validate_term_id(*p, term_count, &format!("reifiers[{idx}].p"))?;
        validate_term_id(*o, term_count, &format!("reifiers[{idx}].o"))?;
    }
    Ok(())
}

fn validate_annotations(annotations: &[(usize, usize, usize)], term_count: usize) -> PyResult<()> {
    for (idx, (r, p, v)) in annotations.iter().enumerate() {
        validate_term_id(*r, term_count, &format!("annotations[{idx}].reifier"))?;
        validate_term_id(*p, term_count, &format!("annotations[{idx}].predicate"))?;
        validate_term_id(*v, term_count, &format!("annotations[{idx}].value"))?;
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

fn term_kind_int(kind: TermKind) -> u8 {
    match kind {
        TermKind::Iri => 0,
        TermKind::Literal => 1,
        TermKind::Bnode => 2,
        TermKind::Triple => 3,
    }
}

fn relational_rows_dict(py: Python<'_>, rows: RelationalRows) -> PyResult<Bound<'_, PyDict>> {
    let out = PyDict::new(py);
    out.set_item("terms", rows.terms)?;
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
