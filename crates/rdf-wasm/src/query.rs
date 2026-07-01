// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The offline, in-browser SPARQL query surface over the wasm [`Dataset`].
//!
//! Binds the native multiset SPARQL evaluator
//! ([`NativeSparqlEngine`](purrdf_sparql_eval::NativeSparqlEngine)) to JavaScript so a
//! page can run SELECT / ASK / CONSTRUCT / DESCRIBE entirely client-side, with no
//! server and no network. The engine is the same one the native query gate uses,
//! with no baked-in HTTP client.
//!
//! ## Federation is intentionally absent
//!
//! This binds the plain [`SparqlEngine::query`](purrdf_core::SparqlEngine::query)
//! entry — the one with **no** [`RemoteQuerySource`](purrdf_sparql_eval::remote)
//! installed. A `SERVICE` or `LOAD` clause therefore **hard-fails** with a JsError
//! rather than silently returning an empty or partial result: in a browser there is
//! no resolver to fetch a remote graph, and a false answer is worse than an error.
//!
//! ## Result encoding
//!
//! - SELECT / ASK → **SPARQL Results JSON** (the W3C SRJ format) via
//!   [`purrdf_sparql_results`].
//! - CONSTRUCT / DESCRIBE → **Turtle** via the `native_codecs` serializer (the one
//!   serialization seam; never `oxigraph::io`, never the `purrdf-gts` crate).

use purrdf::{serialize_dataset, SerializeGraph};
use purrdf_core::{SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::NativeSparqlEngine;
use purrdf_sparql_results::{
    serialize as serialize_results, ResultProvenance, SparqlResultsFormat,
};
use wasm_bindgen::prelude::*;

use crate::dataset::{diag_to_err, Dataset};

#[wasm_bindgen]
impl Dataset {
    /// `query(sparql, base?)` → run a SPARQL query against this dataset, offline.
    ///
    /// Returns **SPARQL Results JSON** for SELECT / ASK and **Turtle** for
    /// CONSTRUCT / DESCRIBE. A parse error, an evaluation error, or a `SERVICE` /
    /// `LOAD` clause (unresolvable in-browser) throws a JsError — never a silent
    /// empty result.
    #[wasm_bindgen(js_name = query)]
    pub fn query(&self, sparql: &str, base: Option<String>) -> Result<String, JsError> {
        // Compact the COW delta to a frozen, shareable base the evaluator reads.
        let frozen = self.inner.freeze().map_err(diag_to_err)?;
        let engine = NativeSparqlEngine::new();
        let request = SparqlRequest {
            query: sparql,
            base_iri: base.as_deref(),
            substitutions: &[],
        };
        let result = engine.query(&frozen, request).map_err(diag_to_err)?;
        match result {
            // CONSTRUCT / DESCRIBE: emit the result graph as Turtle through the one
            // native serialization seam.
            SparqlResult::Graph(graph) => {
                let bytes = serialize_dataset(&graph, "text/turtle", SerializeGraph::Dataset)
                    .map_err(diag_to_err)?;
                String::from_utf8(bytes)
                    .map_err(|e| JsError::new(&format!("CONSTRUCT result is not valid UTF-8: {e}")))
            }
            // SELECT / ASK: emit W3C SPARQL Results JSON.
            other => {
                let outcome = serialize_results(
                    &other,
                    SparqlResultsFormat::Json,
                    &ResultProvenance::default(),
                )
                .map_err(|e| JsError::new(&e.to_string()))?;
                String::from_utf8(outcome.bytes).map_err(|e| {
                    JsError::new(&format!("SPARQL Results JSON is not valid UTF-8: {e}"))
                })
            }
        }
    }
}
