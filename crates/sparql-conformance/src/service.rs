// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! In-memory `SERVICE` endpoint wiring for the conformance harness.
//!
//! A manifest's `qt:serviceData` declarations map endpoint IRIs to local data
//! files. This builds an [`InProcessServiceResolver`] from them — each endpoint
//! becomes an in-memory [`purrdf_core::RdfDataset`] that the native engine
//! queries when a `SERVICE <endpoint> { … }` clause is evaluated. **Fully offline and
//! deterministic**: there is no socket, the "remote" endpoint is just another
//! in-memory dataset answered by the same native engine (dog-fooding).

use purrdf_sparql_eval::InProcessServiceResolver;

use crate::manifest::SparqlTestCase;

/// Build the in-memory `SERVICE` source for `case`: each `qt:serviceData` endpoint
/// answered from its data file, and every other endpoint unreachable.
///
/// Every case gets one, federated or not, because it stands in for the network the
/// suite assumes. An endpoint the manifest declares no data for is one the test
/// expects not to answer — `service7` sends `SERVICE SILENT` to
/// `<http://invalid.endpoint.org/sparql>` and expects the join identity — so the source
/// fails it at the transport layer ([`purrdf_sparql_eval::RemoteError::Transport`]),
/// the endpoint failure `SILENT` tolerates. Running a case with no source at all would
/// be a different claim: that the engine was given nowhere to send the request, which
/// is a configuration fault `SILENT` does not swallow.
///
/// Endpoint data is parsed against the case's OWN sentinel base
/// ([`SparqlTestCase::base`]) — the same one the default-graph data and the query
/// use — so a relative IRI in the endpoint data denotes the same term it denotes
/// in the rest of the case. A harness-wide constant base would make endpoint data
/// from two different groups collide on one IRI space.
///
/// # Errors
///
/// Returns a message if an endpoint's data file cannot be read or parsed.
pub fn build(case: &SparqlTestCase) -> Result<InProcessServiceResolver, String> {
    let mut source = InProcessServiceResolver::new();
    for (endpoint, path) in &case.service_data {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("read service data {}: {e}", path.display()))?;
        let dataset = purrdf::parse_dataset(&bytes, "text/turtle", Some(&case.base))
            .map_err(|e| format!("parse service data {}: {e}", path.display()))?;
        source = source.with_endpoint(endpoint.clone(), dataset);
    }
    Ok(source)
}
