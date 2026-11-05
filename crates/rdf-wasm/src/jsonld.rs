// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reusable configured JSON-LD contexts for the WebAssembly surface.

use std::sync::Arc;

use crate::{host, operation};
use purrdf::{CompiledJsonLdContext as EngineContext, JsonLdSerializeMode, JsonLdSerializeOptions};
use purrdf_validate::document::{self, Operation};
use wasm_bindgen::prelude::*;

fn capture_error(error: host::ByteCaptureError, operation: Operation) -> JsValue {
    match error {
        host::ByteCaptureError::Host(error) => error,
        host::ByteCaptureError::Length => operation::coded_error(
            &format!(
                "document {} buffer byte length exceeds the portable unsigned range",
                operation.name()
            ),
            operation.byte_length_code(),
        ),
        host::ByteCaptureError::Limit { maximum } => {
            operation::native_diagnostic_to_js(purrdf_core::RdfDiagnostic::error(
                operation.input_limit_code(),
                format!(
                    "document {} input exceeds {maximum} bytes",
                    operation.name()
                ),
            ))
        }
        host::ByteCaptureError::Storage(error) => operation::native_diagnostic_to_js(error.into()),
    }
}

/// Expand original byte buffers to JSON document text through the shared request.
///
/// Request capture/admission and compilation precede source capture. Host arrays,
/// admitted bridge copies, native phases and returned JS string storage retain
/// separate authorities. Shared or detached backing buffers are refused.
/// # Errors
/// Throws original host buffer errors or native diagnostic/cause records.
#[wasm_bindgen(js_name = expandDocument)]
pub fn expand_document(source: &JsValue, request_json: &JsValue) -> Result<String, JsValue> {
    run(source, request_json, Operation::Expand)
}

/// Compact source/request byte buffers under an explicit output context.
/// Request and admitted byte capture use the same owners as document expansion.
/// # Errors
/// Throws original host buffer errors or native diagnostic/cause records.
#[wasm_bindgen(js_name = compactDocument)]
pub fn compact_document(source: &JsValue, request_json: &JsValue) -> Result<String, JsValue> {
    run(source, request_json, Operation::Compact)
}

/// Flatten source/request byte buffers, optionally compacting under an output context.
/// Request and admitted byte capture use the same owners as document expansion.
/// # Errors
/// Throws original host buffer errors or native diagnostic/cause records.
#[wasm_bindgen(js_name = flattenDocument)]
pub fn flatten_document(source: &JsValue, request_json: &JsValue) -> Result<String, JsValue> {
    run(source, request_json, Operation::Flatten)
}

fn run(source: &JsValue, request_json: &JsValue, operation: Operation) -> Result<String, JsValue> {
    let bytes =
        host::CapturedBytes::capture(request_json, purrdf_validate::rdf_input::MAX_OPTIONS_BYTES)
            .map_err(|error| capture_error(error, operation))?;
    let request =
        document::request(&bytes.bytes, operation).map_err(operation::native_diagnostic_to_js)?;
    let source = host::CapturedBytes::capture(source, request.max_input_bytes())
        .map_err(|error| capture_error(error, operation))?;
    document::execute_decoded(&source.bytes, &request).map_err(operation::native_diagnostic_to_js)
}

/// An immutable JSON-LD 1.1 context compiled once and reusable across datasets.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct CompiledJsonLdContext {
    pub(crate) inner: Arc<EngineContext>,
}

#[wasm_bindgen]
impl CompiledJsonLdContext {
    /// Compile the context branch of a versioned JSON-LD options document.
    #[wasm_bindgen(constructor)]
    pub fn new(options_json: &str) -> Result<Self, JsError> {
        let options = decode_options(options_json)?;
        let JsonLdSerializeMode::Context(context) = options.mode() else {
            return Err(JsError::new(
                "CompiledJsonLdContext requires JSON-LD options with mode `context`",
            ));
        };
        Ok(Self {
            inner: Arc::clone(context),
        })
    }

    /// Return the recursively canonical context document as JSON.
    #[wasm_bindgen(js_name = canonicalContextJson)]
    pub fn canonical_context_json(&self) -> String {
        self.inner.canonical_json()
    }
}

pub(crate) fn decode_options(json: &str) -> Result<JsonLdSerializeOptions, JsError> {
    JsonLdSerializeOptions::from_json(json.as_bytes())
        .map_err(|error| JsError::new(&error.to_string()))
}

pub(crate) fn context_options(context: &CompiledJsonLdContext) -> JsonLdSerializeOptions {
    JsonLdSerializeOptions::compiled(Arc::clone(&context.inner))
}
