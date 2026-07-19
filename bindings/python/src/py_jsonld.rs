// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configured JSON-LD/YAML-LD Python surface over the shared Rust context engine.

use std::collections::BTreeMap;
use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::py_store::PyRdfFormat;
use crate::{
    CompiledJsonLdContext as EngineContext, JsonLdSerializeMode, JsonLdSerializeOptions,
    RdfDataset, classify, parse_dataset, serialize_dataset_to_format_with_jsonld_options,
};

/// An immutable compiled JSON-LD 1.1 context reusable across datasets.
#[pyclass(name = "CompiledJsonLdContext", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub(crate) struct PyCompiledJsonLdContext {
    pub(crate) inner: Arc<EngineContext>,
}

#[pymethods]
impl PyCompiledJsonLdContext {
    /// Compile the context-mode branch of a versioned JSON-LD options document.
    #[new]
    fn new(options_json: &str) -> PyResult<Self> {
        let options = decode_options(options_json)?;
        let JsonLdSerializeMode::Context(context) = options.mode() else {
            return Err(PyValueError::new_err(
                "CompiledJsonLdContext requires JSON-LD options with mode `context`",
            ));
        };
        Ok(Self {
            inner: Arc::clone(context),
        })
    }

    /// Compile a deterministic prefix map without constructing JSON in Python.
    #[staticmethod]
    fn from_prefixes(prefixes: BTreeMap<String, String>) -> PyResult<Self> {
        let context = EngineContext::from_prefixes(prefixes)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(Self {
            inner: Arc::new(context),
        })
    }

    /// Return the recursively canonicalized context document as JSON.
    fn canonical_context_json(&self) -> PyResult<String> {
        serde_json::to_string(self.inner.canonical_context())
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }
}

/// Format-selecting configured JSON-LD/YAML-LD serializer.
#[pyfunction]
#[pyo3(signature = (data, *, format, output_format, options_json=None, context=None, yaml_schema_url=None))]
#[allow(
    clippy::too_many_arguments,
    reason = "the Python API names each explicit serialization input"
)]
fn serialize_jsonld(
    py: Python<'_>,
    data: &Bound<'_, PyBytes>,
    format: PyRdfFormat,
    output_format: &str,
    options_json: Option<&str>,
    context: Option<&PyCompiledJsonLdContext>,
    yaml_schema_url: Option<&str>,
) -> PyResult<String> {
    let input = data.as_bytes().to_vec();
    let options = options_from_inputs(options_json, context, yaml_schema_url)?;
    py.detach(|| {
        let dataset = parse_dataset(&input, format.to_native().media_type(), None)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        serialize_frozen(&dataset, output_format, &options)
    })
}

pub(crate) fn decode_options(options_json: &str) -> PyResult<JsonLdSerializeOptions> {
    JsonLdSerializeOptions::from_json(options_json.as_bytes())
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

pub(crate) fn options_from_inputs(
    options_json: Option<&str>,
    context: Option<&PyCompiledJsonLdContext>,
    yaml_schema_url: Option<&str>,
) -> PyResult<JsonLdSerializeOptions> {
    let mut options = match (options_json, context) {
        (Some(_), Some(_)) => {
            return Err(PyValueError::new_err(
                "provide exactly one of options_json or context",
            ));
        }
        (Some(json), None) => decode_options(json)?,
        (None, Some(context)) => JsonLdSerializeOptions::compiled(Arc::clone(&context.inner)),
        (None, None) => {
            return Err(PyValueError::new_err(
                "configured JSON-LD serialization requires options_json or context",
            ));
        }
    };
    if let Some(schema_url) = yaml_schema_url {
        options = options
            .with_yaml_schema_url(schema_url)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
    }
    Ok(options)
}

pub(crate) fn serialize_frozen(
    dataset: &RdfDataset,
    output_format: &str,
    options: &JsonLdSerializeOptions,
) -> PyResult<String> {
    let format =
        classify(output_format).map_err(|error| PyValueError::new_err(error.to_string()))?;
    let outcome = serialize_dataset_to_format_with_jsonld_options(dataset, format, None, options)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    String::from_utf8(outcome.bytes).map_err(|error| {
        PyValueError::new_err(format!("serialization produced non-UTF-8: {error}"))
    })
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyCompiledJsonLdContext>()?;
    module.add_function(wrap_pyfunction!(serialize_jsonld, module)?)?;
    Ok(())
}
