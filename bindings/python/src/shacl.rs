// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 Python bindings for `purrdf-shapes`.
//!
//! # Platform note
//!
//! This module belongs to the separate Python binding crate because pyo3 exposes
//! CPython C-API symbols and those are intentionally unavailable in the main
//! wasm-clean Rust crates. There are zero degraded fallbacks and zero feature
//! flags controlling this.
//!
//! # Engine core separation
//!
//! Only this file imports pyo3. All engine modules (`engine`, `shapes`,
//! `constraints`, `path`, `report`, `model`) are PyO3-free so the rlib links
//! into the future Rust compiler without any Python dependency.

use std::sync::Arc;

use ::purrdf::RdfDataset;
use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyCapsule, PyCapsuleMethods, PyDict, PyList, PyTuple};

use purrdf_shapes::engine;
use purrdf_shapes::report::ValidationReport;
use purrdf_validate::ShapesProductRefusal;

/// Validate a data graph (N-Triples) against a shapes graph (Turtle).
///
/// Returns a dict with keys:
/// - `"conforms"` — bool
/// - `"results"` — list of dicts, each with keys:
///   `"focus"`, `"path"`, `"value"`, `"severity"`, `"component"`,
///   `"source_shape"`, `"message"`.
///
/// `shapes_base` is the base IRI the SHAPES document's relative IRI references resolve
/// against. This binding is handed a string and so has no retrieval IRI of its own;
/// PurRDF will not invent one, so a caller who read the shapes from a file or a URL and
/// wants `<PersonShape>` to mean something should pass that document's IRI. Left `None`,
/// a relative reference is a hard `ValueError` naming the remedy — never a validation
/// that quietly conforms because the constraint term was never resolved. `data_nt` needs
/// no counterpart: N-Triples admits no relative IRI by grammar.
#[pyfunction]
#[pyo3(signature = (shapes_ttl, data_nt, *, shapes_base=None))]
fn validate(
    py: Python<'_>,
    shapes_ttl: &str,
    data_nt: &str,
    shapes_base: Option<&str>,
) -> PyResult<Py<PyAny>> {
    // Parse + validation run detached (GIL released); the result dicts are
    // built after the GIL is reacquired.
    let report = py
        .detach(|| engine::validate_graphs(data_nt, shapes_ttl, shapes_base))
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    let out = PyDict::new(py);
    out.set_item("conforms", report.conforms)?;

    let results = PyList::empty(py);
    for r in &report.results {
        let d = PyDict::new(py);
        d.set_item("focus", r.focus_node.to_string())?;
        d.set_item("path", r.result_path.as_ref().map(ToString::to_string))?;
        d.set_item("value", r.value.as_ref().map(ToString::to_string))?;
        d.set_item("severity", r.severity.iri())?;
        d.set_item("component", r.source_constraint_component.as_str())?;
        d.set_item("source_shape", r.source_shape.to_string())?;
        d.set_item("message", r.message.clone())?;
        if !r.source_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .source_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("source_box_roles", roles)?;
        }
        if !r.path_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .path_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("path_box_roles", roles)?;
        }
        if !r.result_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .result_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("result_box_roles", roles)?;
        }
        results.append(d)?;
    }
    out.set_item("results", results)?;

    Ok(out.into_any().unbind())
}

/// Entail a data graph (N-Triples) under a shapes graph (Turtle), returning the
/// materialized dataset as a canonical N-Triples string.
///
/// The entailment twin of [`validate`]: it applies every active SHACL-AF
/// `sh:rule` (`sh:TripleRule` / `sh:SPARQLRule`) to a fixpoint and returns the
/// base graph plus every inferred triple, serialized as deterministic N-Triples.
///
/// Raises `ValueError` if either graph fails to parse or if rule application
/// fails (an illegal head term, an unresolvable `sh:condition`, or a rule set that
/// does not reach a fixpoint).
///
/// # One boundary, three bindings
///
/// The parse→entail→serialize sequence is NOT reimplemented here: it is
/// [`purrdf_validate::entail_to_ntriples_string`], the same function the C-ABI
/// (`purrdf_shacl_entail_to_ntriples`) and WASM (`shacl_entail`) bindings call.
/// This binding used to inline the two-line body, which made that crate's
/// "the shared boundary the language bindings all route through" claim false and
/// left a third copy free to drift. Everything Python-specific — releasing the
/// GIL, mapping the error string to `ValueError` — stays here; the RDF work does
/// not.
#[pyfunction]
#[pyo3(signature = (shapes_ttl, data_nt, *, shapes_base=None))]
fn entail(
    py: Python<'_>,
    shapes_ttl: &str,
    data_nt: &str,
    shapes_base: Option<&str>,
) -> PyResult<String> {
    // Parse + entailment + serialization run detached (GIL released).
    py.detach(|| purrdf_validate::entail_to_ntriples_string(shapes_ttl, shapes_base, data_nt))
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// Parsed SHACL shapes that can be reused to validate multiple data graphs.
///
/// Construct from a Turtle shapes graph with `PyShapes(shapes_ttl)`, then call
/// `validate_nt(data_nt)` for each data graph. The Rust orchestration path in
/// `purrdf-validate` borrows the parsed shapes via [`Self::validate_against_dataset`].
#[pyclass(name = "Shapes")]
#[derive(Debug)]
pub struct PyShapes {
    inner: purrdf_shapes::shapes::Shapes,
}

impl PyShapes {
    /// Validate a borrowed native [`RdfDataset`] against these parsed shapes.
    ///
    /// This is the Rust-side primitive used by `purrdf-validate::PyValidationStore`
    /// so the data store does not have to be re-serialized to N-Triples.
    pub fn validate_against_dataset(&self, data: &RdfDataset) -> ValidationReport {
        engine::validate_dataset(data, &self.inner)
            .expect("validation over a frozen dataset is infallible")
    }
}

#[pymethods]
impl PyShapes {
    /// `Shapes(shapes_ttl, *, base=None)`.
    ///
    /// `base` is the shapes document's own base IRI, used to resolve its relative IRI
    /// references (RFC-3986 §5.1.2). Omitted, only an in-document `@base` can establish
    /// one and a relative reference otherwise raises `ValueError`.
    #[new]
    #[pyo3(signature = (shapes_ttl, *, base=None))]
    fn new(py: Python<'_>, shapes_ttl: &str, base: Option<&str>) -> PyResult<Self> {
        // Shapes-graph parsing runs detached (GIL released).
        let inner = py
            .detach(|| engine::parse_shapes(shapes_ttl, base))
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(Self { inner })
    }

    /// Validate an N-Triples data graph against these parsed shapes.
    fn validate_nt(&self, py: Python<'_>, data_nt: &str) -> PyResult<PyValidationReport> {
        // Native codec ingest: lenient on private-use language tags, every
        // malformed line reported in one pass. The engine runs over the frozen IR.
        // Both the ingest and the validation run detached (GIL released).
        let shapes = &self.inner;
        let report = py.detach(|| {
            let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(data_nt)
                .map_err(|errors| pyo3::exceptions::PyValueError::new_err(errors.join("\n")))?;
            engine::validate_dataset(data.as_ref(), shapes)
                .map_err(pyo3::exceptions::PyValueError::new_err)
        })?;
        Ok(PyValidationReport::new(report))
    }

    /// Analyze this shape tree once, without inspecting any data, returning a
    /// reusable `PreparedShapes`.
    ///
    /// The step that makes a prepared PRODUCT possible: a `PreparedShapes` is what
    /// `to_product()` writes out, and what admitting a product hands back.
    fn prepare(&self, py: Python<'_>) -> PyPreparedShapes {
        let shapes = self.inner.clone();
        PyPreparedShapes {
            inner: py.detach(|| engine::PreparedShapes::new(Arc::new(shapes))),
        }
    }

    /// Validate a borrowed native dataset against these parsed shapes.
    ///
    /// `data` must be an object (typically `purrdf_validate.ValidationStore`) that
    /// exposes an internal `_store_capsule()` method returning a capsule borrowing a
    /// frozen `Arc<RdfDataset>` snapshot. This avoids serialising the store to
    /// N-Triples for each validation phase.
    ///
    /// # Errors
    ///
    /// Returns `AttributeError` if `data` has no `_store_capsule` method, and
    /// `ValueError` if the capsule cannot be read.
    fn validate_store(&self, data: &Bound<'_, PyAny>) -> PyResult<PyValidationReport> {
        let capsule = data.call_method0("_store_capsule")?;
        let capsule = capsule.cast::<PyCapsule>()?;
        let ptr = capsule
            .pointer_checked(Some(c"purrdf-validation-dataset"))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let addr = unsafe { *ptr.cast::<usize>().as_ptr() };
        // SAFETY: the capsule's value is the address of an `Arc<RdfDataset>` the
        // producer keeps alive (and at a stable address) for the capsule's lifetime.
        // We clone the `Arc` (extending the dataset's lifetime past the capsule
        // borrow) so validation can run detached (GIL released) without touching
        // any py-bound value.
        let dataset = Arc::clone(unsafe { &*(addr as *const Arc<RdfDataset>) });
        let report = data
            .py()
            .detach(|| self.validate_against_dataset(dataset.as_ref()));
        Ok(PyValidationReport::new(report))
    }
}

/// A SHACL validation report.
///
/// Wraps the Rust [`crate::report::ValidationReport`] and exposes `conforms`,
/// the list of result dicts, and a canonical N-Triples serialization.
#[pyclass(name = "ValidationReport")]
#[derive(Debug)]
pub struct PyValidationReport {
    inner: ValidationReport,
}

impl PyValidationReport {
    /// Construct from a Rust [`crate::report::ValidationReport`].
    pub fn new(inner: ValidationReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValidationReport {
    #[getter]
    fn conforms(&self) -> bool {
        self.inner.conforms
    }

    #[getter]
    fn results(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for r in &self.inner.results {
            let d = PyDict::new(py);
            d.set_item("focus", r.focus_node.to_string())?;
            d.set_item("path", r.result_path.as_ref().map(ToString::to_string))?;
            d.set_item("value", r.value.as_ref().map(ToString::to_string))?;
            d.set_item("severity", r.severity.iri())?;
            d.set_item("component", r.source_constraint_component.as_str())?;
            d.set_item("source_shape", r.source_shape.to_string())?;
            d.set_item("message", r.message.clone())?;
            if !r.source_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .source_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("source_box_roles", roles)?;
            }
            if !r.path_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .path_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("path_box_roles", roles)?;
            }
            if !r.result_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .result_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("result_box_roles", roles)?;
            }
            list.append(d)?;
        }
        Ok(list.into_any().unbind())
    }

    /// Serialize the report to canonical N-Triples (runs detached — GIL released).
    fn to_ntriples(&self, py: Python<'_>) -> String {
        let report = &self.inner;
        py.detach(|| report.to_ntriples())
    }

    /// Serialize the report to a SARIF 2.1.0 JSON string (runs detached — GIL
    /// released). The sibling of [`to_ntriples`](Self::to_ntriples): the RDF
    /// report graph stays canonical N-Triples, while SARIF is the source-traced,
    /// actionable surface for editors and CI. Logical locations are emitted here
    /// (focus node, result path, constraint component, source shape); physical
    /// source spans require a span-tracked parse.
    fn to_sarif(&self, py: Python<'_>) -> String {
        let report = &self.inner;
        py.detach(|| {
            purrdf_validate::report_to_sarif_string(
                report,
                &purrdf_validate::SarifOptions::default(),
            )
        })
    }
}

// ── Prepared shapes products ────────────────────────────────────────────────

create_exception!(
    shacl,
    ShapesProductError,
    pyo3::exceptions::PyValueError,
    "A refusal from the prepared-shapes-product admission boundary.\n\
     \n\
     Carries a `.dimension` attribute: one of the codec's pinned kebab-case labels \
     (`magic`, `format-version`, `stage-id`, `profile`, `truncated`, `trailer`, \
     `section-digest`, `container-digest`, `dataset-identity`, `shapes-graph`, \
     `prefixes`, `base`, `vocabulary`, `function-registry`, `aggregate-registry`, \
     `property-function-registry`, `class-catalog`, `unsupported-capability`, \
     `depth-limit`, `malformed`), or `None` when the failure happened before any \
     product existed — a shapes or data document that did not parse was never \
     admitted, and naming a dimension for it would claim a product was inspected \
     when none was.\n\
     \n\
     Branch on `.dimension`, never on `str(exc)`: the label is a pinned contract and \
     the message is prose that may be reworded. A `stage-id` refusal means re-pack \
     or restore with `rebuild()`; `container-digest` means the bytes are corrupt in \
     place; `function-registry` means the caller's own configuration differs from \
     the one the product was prepared against, which re-packing will not fix.\n\
     \n\
     Subclasses `ValueError`, so code that already catches the SHACL surface's \
     `ValueError` keeps working."
);

/// Raise a product refusal as [`ShapesProductError`], with `.dimension` always
/// present.
///
/// Always present — `None` rather than absent when there is no dimension — because an
/// attribute that sometimes exists forces every caller to write `getattr(exc,
/// "dimension", None)`, and the one who forgets gets an `AttributeError` from their
/// own error handler.
fn product_error(py: Python<'_>, refusal: &ShapesProductRefusal) -> PyErr {
    let error = ShapesProductError::new_err(refusal.to_string());
    let dimension = refusal.dimension_label().map_or_else(
        || py.None(),
        |label| {
            label
                .into_pyobject(py)
                .map_or_else(|_| py.None(), |bound| bound.into_any().unbind())
        },
    );
    // A failure to set the attribute would mean the exception object refused an
    // ordinary `setattr`, which cannot happen for a Python-level exception class;
    // it is ignored rather than replacing a precise refusal with a vaguer one.
    let _ = error.value(py).setattr("dimension", dimension);
    error
}

/// An immutable shape preparation: the parse-and-analyze work done once, reusable
/// across independent data graphs and writable as a prepared PRODUCT.
///
/// Obtain one with `Shapes(...).prepare()`, or by admitting a product with
/// `ShapesProduct.open(data).admit()`.
#[pyclass(name = "PreparedShapes")]
#[derive(Debug)]
pub struct PyPreparedShapes {
    inner: engine::PreparedShapes,
}

#[pymethods]
impl PyPreparedShapes {
    /// Write this preparation out as a prepared product, returning its bytes.
    ///
    /// Byte-deterministic: no clock, no randomness and no hash-iteration order reach
    /// the writer, so two calls over equal preparations return identical bytes and a
    /// content-addressed cache key over them is stable.
    ///
    /// The product carries the compiled model AND the shapes dataset it came from,
    /// under a per-section SHA-256 and a whole-container digest, plus the binding of
    /// every input it was compiled against.
    ///
    /// Raises `ShapesProductError` when the shapes graph declares something the
    /// product format cannot carry.
    fn to_product<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let prepared = &self.inner;
        let bytes = py
            .detach(|| purrdf_validate::prepared_to_product(prepared))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Where this preparation came from, as one deterministic token — `parsed`,
    /// `restored-admitted <identity_digest>` or `restored-rebuilt <identity_digest>`.
    ///
    /// TOTAL: every preparation has an answer and none of them means "we forgot to
    /// record it". An authenticated artifact whose output cannot be attributed to it
    /// answers only half the question — `admit()` establishes that this process MAY
    /// execute a product, and this establishes WHICH product a report came out of,
    /// which is the half a caller needs after the fact.
    ///
    /// The digest is the same 64 hexadecimal digits `identity_digest()` returns for
    /// the product, so a value read off a validation log can be handed straight back
    /// to `admit_expecting()` without editing. The two restore tokens are distinct
    /// because the identity means different things on each: admitted means the
    /// product's binding was checked against this process, rebuilt means it was
    /// recorded from the artifact and deliberately not checked (see `rebuild()`).
    fn provenance(&self) -> String {
        self.inner.provenance().to_string()
    }

    /// Validate an N-Triples data graph against this preparation.
    ///
    /// The same verdict `Shapes.validate_nt` reaches, through the same engine entry
    /// point — which is the property a restored product is only useful if it has.
    fn validate_nt(&self, py: Python<'_>, data_nt: &str) -> PyResult<PyValidationReport> {
        let prepared = &self.inner;
        let report = py.detach(|| {
            let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(data_nt)
                .map_err(|errors| pyo3::exceptions::PyValueError::new_err(errors.join("\n")))?;
            engine::validate_dataset(data.as_ref(), prepared.shapes())
                .map_err(pyo3::exceptions::PyValueError::new_err)
        })?;
        Ok(PyValidationReport::new(report))
    }
}

/// A prepared product whose envelope has been verified and whose self-description has
/// been decoded — but which has NOT been admitted.
///
/// Holding one is a statement about framing and integrity, never about fitness: the
/// bytes are a well-formed product of this format, and nothing has yet claimed they
/// are a product this build may execute. That separation is what makes
/// `identity_components()` useful — read what a product says it was compiled from in
/// order to decide what to do about it, without any of it reaching a validator.
///
/// This value OWNS the bytes it was opened from. The Rust view borrows its input, and
/// a Python object cannot hold a borrow of a `bytes` it does not own, so each method
/// re-opens the owned buffer. Re-opening costs the container's cheap integrity tier —
/// the framing and the section digests — never the canonicalization `certify()`
/// performs.
#[pyclass(name = "ShapesProduct")]
#[derive(Debug)]
pub struct PyShapesProduct {
    bytes: Vec<u8>,
}

#[pymethods]
impl PyShapesProduct {
    /// Open `data` as a prepared product: verify the envelope and decode the
    /// product's self-description, admitting nothing.
    ///
    /// Raises `ShapesProductError` with a structural `.dimension` when the bytes are
    /// not a well-formed product of this format.
    #[staticmethod]
    fn open(py: Python<'_>, data: &[u8]) -> PyResult<Self> {
        // Opened here and discarded: this is the eager check that makes holding a
        // `ShapesProduct` mean something. Every later method re-opens the owned copy.
        py.detach(|| purrdf_validate::explain_shapes_product(data))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(Self {
            bytes: data.to_vec(),
        })
    }

    /// The product's own bytes, exactly as opened.
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.bytes)
    }

    /// Everything the product says about itself, as deterministic `key value` lines:
    /// the container format version, the preparation stage id and whether this build
    /// knows it, the identity digest and every labelled identity component, then the
    /// recorded base, `sh:shapesGraph` IRI and prefix map.
    ///
    /// This is what makes a named refusal actionable: a restore refused on
    /// `prefixes` is answered by reading which prefix map the product actually
    /// carries, not by guessing.
    fn explain(&self, py: Python<'_>) -> PyResult<String> {
        let bytes = &self.bytes;
        py.detach(|| purrdf_validate::explain_shapes_product(bytes))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))
    }

    /// The container format version these bytes were written under.
    fn format_version(&self, py: Python<'_>) -> PyResult<u32> {
        self.explain_field(py, "format-version")?
            .parse()
            .map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(
                    "this build rendered a non-numeric product format version",
                )
            })
    }

    /// The 32-byte preparation stage id the product declares, as lowercase hex.
    fn stage_id(&self, py: Python<'_>) -> PyResult<String> {
        self.explain_field(py, "stage-id")
    }

    /// Whether this build knows the product's preparation stage.
    ///
    /// `False` says `admit()` will refuse these bytes and `rebuild()` is the path
    /// that still restores them — the memo was written against a model this build no
    /// longer has, and the shapes dataset the product carries is what rescues it.
    fn stage_known(&self, py: Python<'_>) -> PyResult<bool> {
        Ok(self.explain_field(py, "stage-known")? == "true")
    }

    /// The SHA-256 digest of the product's input binding, as lowercase hex.
    fn identity_digest(&self, py: Python<'_>) -> PyResult<String> {
        self.explain_field(py, "identity-digest")
    }

    /// The ordered, labelled components of the product's input binding as
    /// `(label, rendered_value)` pairs — which inputs it was compiled from.
    ///
    /// The ORDER is the identity, not the labels: two components may share a label
    /// and still be different components, so this is a list of pairs rather than a
    /// dict.
    fn identity_components<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let rendered = self.explain(py)?;
        let out = PyList::empty(py);
        for line in rendered.lines() {
            let Some(rest) = line.strip_prefix("identity ") else {
                continue;
            };
            // `splitn(2, …)` rather than a split on every space: an identity label is a
            // kebab-case constant and never contains one, but a rendered VALUE may, so
            // everything after the first space is the value.
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() != 2 {
                continue;
            }
            out.append(PyTuple::new(py, parts)?)?;
        }
        Ok(out)
    }

    /// **The common path.** Restore the preparation from the product's memo,
    /// re-deriving nothing expensive.
    ///
    /// The product's stage id, profile and complete input binding are checked before
    /// any of it reaches a validator. Raises `ShapesProductError` — read
    /// `.dimension` to decide what to do: `stage-id` means try `rebuild()`, an
    /// identity dimension means this process's configuration is not the one the
    /// product was prepared against.
    fn admit(&self, py: Python<'_>) -> PyResult<PyPreparedShapes> {
        let bytes = &self.bytes;
        let inner = py
            .detach(|| purrdf_validate::admit_shapes_product(bytes))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyPreparedShapes { inner })
    }

    /// **The common path, bound to the product you MEANT.** Restore exactly as
    /// `admit()` does, but only after confirming the product's input binding is
    /// `expected_identity`.
    ///
    /// Everything `admit()` checks is a question about this PROCESS — its build, its
    /// registries, its class analysis. None of them asks whether these are the bytes
    /// the caller wanted, because nothing in a product states which product was
    /// meant. Admitting a cache entry, a downloaded artifact or a path built from a
    /// configuration string without saying which one it must be is how a validator
    /// returns a well-formed report about a shapes graph nobody asked about.
    ///
    /// `expected_identity` is the 64 hexadecimal digits `identity_digest()` returns
    /// for the product you intend — one spelling, readable off the artifact itself,
    /// so the selector can be pinned in a test or a deployment manifest.
    ///
    /// Raises `ShapesProductError` with `.dimension == "shapes-graph"` when the
    /// product carries a different binding, and `ValueError` when
    /// `expected_identity` is not 64 hexadecimal digits — no product was inspected in
    /// that case, so no dimension names it.
    fn admit_expecting(
        &self,
        py: Python<'_>,
        expected_identity: &str,
    ) -> PyResult<PyPreparedShapes> {
        let expected = purrdf_validate::parse_identity_digest(expected_identity)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        let bytes = &self.bytes;
        let inner = py
            .detach(|| purrdf_validate::admit_shapes_product_expecting(bytes, &expected))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyPreparedShapes { inner })
    }

    /// **The forward-compatibility path.** Ignore the memo and re-derive the
    /// preparation from the shapes dataset the product carries.
    ///
    /// Not a Turtle fallback: no RDF text is parsed and no file is read. The dataset
    /// travels inside the product under the envelope's own digests, and this
    /// re-derives the shapes from it under the product's recorded parse inputs.
    fn rebuild(&self, py: Python<'_>) -> PyResult<PyPreparedShapes> {
        let bytes = &self.bytes;
        let inner = py
            .detach(|| purrdf_validate::rebuild_shapes_product(bytes))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyPreparedShapes { inner })
    }

    /// **The forward-compatibility path, bound to the product you MEANT.**
    /// Re-derive the preparation exactly as `rebuild()` does, but only after
    /// confirming the product's input binding is `expected_identity`.
    ///
    /// The rescue `rebuild()` performs is not a reason to stop asking whether
    /// this is the artifact the caller wanted: a cache entry from another
    /// build, or a product a deployment placed on disk under a stage id this
    /// build does not recognize, is still just a file that could be the wrong
    /// one. The same 32-byte comparison `admit_expecting()` runs FIRST also runs
    /// first here, ahead of the re-derivation, for the identical reason.
    ///
    /// `expected_identity` carries the same meaning it does on
    /// `admit_expecting()` — the 64 hexadecimal digits `identity_digest()`
    /// returns for the product you intend.
    ///
    /// Raises `ShapesProductError` with `.dimension == "shapes-graph"` when the
    /// product carries a different binding, and `ValueError` when
    /// `expected_identity` is not 64 hexadecimal digits — no product was
    /// inspected in that case, so no dimension names it.
    fn rebuild_expecting(
        &self,
        py: Python<'_>,
        expected_identity: &str,
    ) -> PyResult<PyPreparedShapes> {
        let expected = purrdf_validate::parse_identity_digest(expected_identity)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        let bytes = &self.bytes;
        let inner = py
            .detach(|| purrdf_validate::rebuild_shapes_product_expecting(bytes, &expected))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyPreparedShapes { inner })
    }

    /// **The cold path.** Independently corroborate the shapes dataset's canonical
    /// identity against the one this product's binding claims.
    ///
    /// Canonicalization is a graph-isomorphism computation over the shapes graph's
    /// blank nodes and can cost more than the shapes parse a product exists to
    /// eliminate, so it is never on a restore path. Call it from a build step, a
    /// conformance harness or a test — not before every validation.
    fn certify(&self, py: Python<'_>) -> PyResult<()> {
        let bytes = &self.bytes;
        py.detach(|| purrdf_validate::certify_shapes_product(bytes))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))
    }
}

impl PyShapesProduct {
    /// The value of the first `key value` line in the shared rendering whose key is
    /// `key`.
    ///
    /// Every scalar accessor reads the ONE rendering rather than opening the product
    /// a second way, so this class cannot report two independently-derived answers
    /// about one product.
    fn explain_field(&self, py: Python<'_>, key: &str) -> PyResult<String> {
        let rendered = self.explain(py)?;
        rendered
            .lines()
            .find_map(|line| {
                line.strip_prefix(key)
                    .and_then(|rest| rest.strip_prefix(' '))
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "this product's rendering carries no `{key}` line; that is a defect in \
                     this build rather than in the product"
                ))
            })
    }
}

/// Compile a Turtle shapes graph into a prepared product in one call.
///
/// The composition of `Shapes(shapes_ttl, base=shapes_base).prepare().to_product()`,
/// offered because packing a document is the common case and the three-step spelling
/// is a preparation nobody keeps.
///
/// `shapes_base` is the base IRI the shapes document's relative IRI references
/// resolve against, RECORDED in the product so a restore resolves them identically
/// without the document.
///
/// Raises `ShapesProductError`; `.dimension` is `None` when the shapes document
/// itself did not parse.
#[pyfunction]
#[pyo3(signature = (shapes_ttl, *, shapes_base=None))]
fn pack_product<'py>(
    py: Python<'py>,
    shapes_ttl: &str,
    shapes_base: Option<&str>,
) -> PyResult<Bound<'py, PyBytes>> {
    let bytes = py
        .detach(|| purrdf_validate::pack_shapes_product(shapes_ttl, shapes_base))
        .map_err(|refusal| product_error(py, &refusal))?;
    Ok(PyBytes::new(py, &bytes))
}

/// Register the `purrdf-shapes` surface on a Python module.
///
/// Exposes the legacy `validate(shapes_ttl, data_nt)` function, the SHACL-AF
/// `entail(shapes_ttl, data_nt)` rule-entailment function, and the reusable
/// `Shapes` / `ValidationReport` wrappers used by the Rust-native orchestration
/// in `purrdf-validate`. Called by the unified `purrdf_native` cdylib to
/// populate the `purrdf_native.shacl` submodule.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(entail, m)?)?;
    m.add_function(wrap_pyfunction!(pack_product, m)?)?;
    m.add_class::<PyShapes>()?;
    m.add_class::<PyValidationReport>()?;
    m.add_class::<PyPreparedShapes>()?;
    m.add_class::<PyShapesProduct>()?;
    m.add(
        "ShapesProductError",
        m.py().get_type::<ShapesProductError>(),
    )?;
    Ok(())
}
