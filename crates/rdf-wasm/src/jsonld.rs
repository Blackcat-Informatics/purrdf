// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reusable configured JSON-LD contexts for the WebAssembly surface.

use std::sync::Arc;

use purrdf::{CompiledJsonLdContext as EngineContext, JsonLdSerializeMode, JsonLdSerializeOptions};
use wasm_bindgen::prelude::*;

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
    pub fn canonical_context_json(&self) -> Result<String, JsError> {
        serde_json::to_string(self.inner.canonical_context())
            .map_err(|error| JsError::new(&error.to_string()))
    }
}

pub(crate) fn decode_options(json: &str) -> Result<JsonLdSerializeOptions, JsError> {
    JsonLdSerializeOptions::from_json(json.as_bytes())
        .map_err(|error| JsError::new(&error.to_string()))
}

pub(crate) fn context_options(context: &CompiledJsonLdContext) -> JsonLdSerializeOptions {
    JsonLdSerializeOptions::compiled(Arc::clone(&context.inner))
}
