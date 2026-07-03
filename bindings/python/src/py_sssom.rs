// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 bindings for the native SSSOM codec — the `purrdf` SSSOM surface that
//! replaces the `sssom` PyPI package's parse + validate behaviour on the
//! `purrdf-dev regenerate` / mappings-generator path.
//!
//! # Kernel-clean separation
//!
//! Like [`crate::py`] and [`crate::py_store`], this module is compiled **only
//! under the `python` feature**. The SSSOM kernel ([`crate::sssom`]) stays
//! PyO3-free: the load-bearing parse / validate / serialize / `to_rdf` logic is
//! unit-tested without a Python interpreter, and these `#[pyfunction]`s are thin
//! wrappers over it. The Python caller therefore sees the *same* diagnostics the
//! Rust fidelity gate enforces, with no external RDF/SSSOM runtime.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::{sssom, RdfDiagnostic, RdfSeverity};

/// The uppercase severity string the SSSOM validation golden uses (`"ERROR"`),
/// matching `tests/fixtures/lint-golden/sssom_validation.json`. The Python caller
/// filters on `severity in ("ERROR", "FATAL")`, so the case must be exact.
fn severity_str(severity: RdfSeverity) -> String {
    severity.as_str().to_ascii_uppercase()
}

/// Build a `{severity, code, message, check, instance}` diagnostic dict — the
/// golden record shape the Python validation channel consumes.
fn diag_dict<'py>(
    py: Python<'py>,
    severity: &str,
    code: &str,
    message: &str,
    check: &str,
    instance: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("severity", severity)?;
    dict.set_item("code", code)?;
    dict.set_item("message", message)?;
    dict.set_item("check", check)?;
    dict.set_item("instance", instance)?;
    Ok(dict)
}

/// Map a structural parse failure ([`RdfDiagnostic`]) into the same diagnostic
/// channel as a semantic validation defect, tagged `severity="FATAL"` /
/// `check="parse"`. Unifying parse and validation failures in one list lets the
/// Python caller filter `severity in ("ERROR", "FATAL")` uniformly instead of
/// branching on a separate parse-error path.
fn parse_failure_dict<'py>(py: Python<'py>, diag: &RdfDiagnostic) -> PyResult<Bound<'py, PyDict>> {
    diag_dict(py, "FATAL", &diag.code, &diag.message, "parse", None)
}

/// Parse a PurRDF SSSOM TSV document, then validate it, returning one dict per
/// diagnostic with string keys `{severity, code, message, check, instance}`.
///
/// A structurally unparsable document yields a single-element list carrying the
/// parse failure as a `FATAL` / `check="parse"` diagnostic (see
/// [`parse_failure_dict`]); a clean corpus file yields `[]`. This is the native
/// replacement for the Python `_validate_sssom` shim — parse and validation
/// defects arrive in one channel the caller filters by severity.
#[pyfunction]
fn validate_sssom(py: Python<'_>, text: &str) -> PyResult<Vec<Py<PyDict>>> {
    // Parse + validation run detached (GIL released); the diagnostic dicts are
    // built after the GIL is reacquired.
    let outcome = py.detach(|| sssom::parse_tsv(text).map(|set| sssom::validate(&set)));
    let diagnostics = match outcome {
        Ok(diagnostics) => diagnostics,
        Err(diag) => return Ok(vec![parse_failure_dict(py, &diag)?.unbind()]),
    };
    diagnostics
        .into_iter()
        .map(|d| {
            diag_dict(
                py,
                &severity_str(d.severity),
                &d.code,
                &d.message,
                &d.check,
                d.instance.as_deref(),
            )
            .map(Bound::unbind)
        })
        .collect()
}

/// Parse a PurRDF SSSOM TSV document and render its [`sssom::to_rdf`] projection as
/// N-Triples text — the round-trip / ENHANCE surface the Python codec never had
/// (the `sssom` PyPI package validates but emits no RDF here). A structurally
/// unparsable document is a hard `ValueError`.
#[pyfunction]
fn sssom_to_rdf(py: Python<'_>, text: &str) -> PyResult<String> {
    // Parse + RDF projection run detached (GIL released).
    py.detach(|| {
        let set = sssom::parse_tsv(text).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(sssom::to_rdf(&set)
            .iter()
            .map(crate::turtle::emit_quad)
            .collect())
    })
}

/// Parse then re-serialize a PurRDF SSSOM TSV document, returning the canonical
/// TSV form ([`sssom::serialize_tsv`]). The byte-stable round-trip the compile
/// path uses to prove the codec is idempotent. A structurally unparsable document
/// is a hard `ValueError`.
#[pyfunction]
fn sssom_roundtrip_tsv(py: Python<'_>, text: &str) -> PyResult<String> {
    // Parse + canonical re-serialization run detached (GIL released).
    py.detach(|| {
        let set = sssom::parse_tsv(text).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(sssom::serialize_tsv(&set))
    })
}

/// The default check set the native validator replicates from sssom-py
/// ([`sssom::SSSOM_DEFAULT_VALIDATION_TYPES`]) — exposed so Python / tests can
/// assert the parity surface the codec covers.
#[pyfunction]
fn sssom_default_validation_types() -> Vec<String> {
    sssom::SSSOM_DEFAULT_VALIDATION_TYPES
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

/// Register the native SSSOM surface on the `purrdf` module.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate_sssom, m)?)?;
    m.add_function(wrap_pyfunction!(sssom_to_rdf, m)?)?;
    m.add_function(wrap_pyfunction!(sssom_roundtrip_tsv, m)?)?;
    m.add_function(wrap_pyfunction!(sssom_default_validation_types, m)?)?;
    Ok(())
}
