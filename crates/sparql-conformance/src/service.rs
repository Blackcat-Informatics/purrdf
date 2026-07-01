// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! In-memory `SERVICE` endpoint wiring for the conformance harness.
//!
//! A manifest's `qt:serviceData` declarations map endpoint IRIs to local data
//! files. This builds a [`LocalRemoteQuerySource`] from them — each endpoint
//! becomes an in-memory [`purrdf_core::RdfDataset`] that the native engine
//! queries when a `SERVICE <endpoint> { … }` clause is evaluated. **Fully offline and
//! deterministic**: there is no socket, the "remote" endpoint is just another
//! in-memory dataset answered by the same native engine (dog-fooding).

use purrdf_sparql_eval::LocalRemoteQuerySource;

use crate::manifest::SparqlTestCase;

/// The same sentinel base the manifest/data are parsed against, so IRIs in the
/// endpoint data align with those in the default-graph data.
const BASE: &str = "http://purrdf.test/manifest/";

/// Build an in-memory `SERVICE` source for `case`, if it declares any
/// `qt:serviceData`. Returns `Ok(None)` when the case is not federated.
///
/// # Errors
///
/// Returns a message if an endpoint's data file cannot be read or parsed.
pub fn build(case: &SparqlTestCase) -> Result<Option<LocalRemoteQuerySource>, String> {
    if case.service_data.is_empty() {
        return Ok(None);
    }
    let mut source = LocalRemoteQuerySource::new();
    for (endpoint, path) in &case.service_data {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("read service data {}: {e}", path.display()))?;
        let dataset = purrdf::parse_dataset(&bytes, "text/turtle", Some(BASE))
            .map_err(|e| format!("parse service data {}: {e}", path.display()))?;
        source = source.with_endpoint(endpoint.clone(), dataset);
    }
    Ok(Some(source))
}
