// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed presentation a SPARQL failure's `ValueError` carries.
//!
//! An engine diagnostic becomes the same `ValueError`, with the same message and
//! arguments, that the Python surface has always raised. It also gets two attributes:
//!
//! * `message_id`: the presentation's stable identity (`"sparql-parse-syntax"`, …),
//!   or `None` when the diagnostic carries no presentation;
//! * `presentation`: a dict `{"message_id": str, "parameters": {name: {"kind": str,
//!   "value": …}}, "detail": dict | None}`, or `None`. `value` is a Python `int`
//!   for the `unsigned` and `signed` kinds (exact, of any width), a `bool` for
//!   `boolean`, and a `str` for `text` and `character`. `detail` is the nested
//!   condition of the same shape, such as an IRI refusal's `iri-*` cause.
//!
//! A host reads the condition and its typed arguments without parsing English.
//!
//! Every `ValueError` that leaves `query`, `query_governed`,
//! `query_entailment_governed`, `prepare`, `update` or `update_governed` carries both
//! attributes, whatever raised it: [`settled`] is those methods' one exit, and it
//! gives a `ValueError` without a presentation (an argument refusal, an unknown
//! regime, a rule document a regime refuses) `None` for both.
//!
//! A premise IRI the shared boundary refuses carries the same two attributes, from
//! every `purrdf.entail` function and from `Store.query_entailment_governed`:
//! `premise-iri-not-absolute`, whose `detail` is the IRI parser's `iri-*` condition
//! ([`presented_value_error`]).

use purrdf_core::{DiagnosticPresentation, DiagnosticValue, RdfDiagnostic};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// The `ValueError` reading `message` for `diagnostic`, carrying its typed
/// presentation as `message_id` and `presentation` (both `None` without one).
///
/// Callable with or without the GIL held: the attributes are set under
/// [`Python::attach`], which the detached engine calls reach only on this error path.
pub(super) fn value_error(message: String, diagnostic: &RdfDiagnostic) -> PyErr {
    presented_value_error(message, diagnostic.presentation())
}

/// The `ValueError` reading `message`, carrying `presentation` as `message_id` and
/// `presentation` (both `None` without one).
///
/// [`value_error`]'s body, for a refusal whose typed presentation is not carried by an
/// [`RdfDiagnostic`]: a premise IRI the shared boundary refuses
/// ([`purrdf_validate::PremiseIriError::presentation`]) reaches `purrdf.entail` and
/// `Store.query_entailment_governed` this way.
pub(crate) fn presented_value_error(
    message: String,
    presentation: Option<&DiagnosticPresentation>,
) -> PyErr {
    let error = PyValueError::new_err(message);
    Python::attach(|py| {
        let value = error.value(py);
        let attached = value
            .setattr(
                "message_id",
                presentation.map(DiagnosticPresentation::message_id),
            )
            .and_then(|()| {
                let record = presentation
                    .map(|presentation| presentation_dict(py, presentation))
                    .transpose()?;
                value.setattr("presentation", record)
            });
        // Setting an attribute on a fresh `ValueError` instance cannot fail short of
        // interpreter memory exhaustion; that failure, not the parse failure, is then
        // the exception the caller sees.
        attached.map_or_else(|failure| failure, |()| error)
    })
}

/// Run a SPARQL method's body, so that every `ValueError` it raises carries
/// `message_id` and `presentation`. One raised with them already set by
/// [`value_error`] keeps them, and any other gets `None` for both. Other exception
/// types (a `TypeError`, a `KeyboardInterrupt`) pass through untouched.
pub(super) fn settled<T>(body: impl FnOnce() -> PyResult<T>) -> PyResult<T> {
    body().map_err(|error| {
        Python::attach(|py| {
            if !error.is_instance_of::<PyValueError>(py) {
                return error;
            }
            let value = error.value(py);
            let attached = ["message_id", "presentation"]
                .into_iter()
                .try_for_each(|name| {
                    if value.hasattr(name)? {
                        Ok(())
                    } else {
                        value.setattr(name, py.None())
                    }
                });
            attached.map_or_else(|failure| failure, |()| error)
        })
    })
}

/// One presentation as the documented dict, its detail nested in the same shape.
fn presentation_dict<'py>(
    py: Python<'py>,
    presentation: &DiagnosticPresentation,
) -> PyResult<Bound<'py, PyDict>> {
    let record = PyDict::new(py);
    record.set_item("message_id", presentation.message_id())?;
    let parameters = PyDict::new(py);
    for parameter in presentation.parameters() {
        let typed = PyDict::new(py);
        match parameter.value() {
            DiagnosticValue::Text(text) => {
                typed.set_item("kind", "text")?;
                typed.set_item("value", text)?;
            }
            DiagnosticValue::Unsigned(number) => {
                typed.set_item("kind", "unsigned")?;
                typed.set_item("value", number)?;
            }
            DiagnosticValue::Signed(number) => {
                typed.set_item("kind", "signed")?;
                typed.set_item("value", number)?;
            }
            DiagnosticValue::Boolean(flag) => {
                typed.set_item("kind", "boolean")?;
                typed.set_item("value", flag)?;
            }
            DiagnosticValue::Character(character) => {
                typed.set_item("kind", "character")?;
                typed.set_item("value", character.to_string())?;
            }
            // A kind added later reaches Python as its JSON record's kind and value,
            // the spelling every host shares, rather than being dropped.
            _ => {
                let json = presentation.to_json();
                let entry = &json["parameters"][parameter.name()];
                typed.set_item("kind", entry["kind"].as_str())?;
                typed.set_item("value", entry["value"].to_string())?;
            }
        }
        parameters.set_item(parameter.name(), typed)?;
    }
    record.set_item("parameters", parameters)?;
    let detail = presentation
        .detail()
        .map(|detail| presentation_dict(py, detail))
        .transpose()?;
    record.set_item("detail", detail)?;
    Ok(record)
}
