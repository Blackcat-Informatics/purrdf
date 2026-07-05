// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
#![forbid(unsafe_code)]

//! Native, wasm-clean serializer for the SPARQL result model.
//!
//! This crate is the canonical authority for turning a `purrdf-core`
//! [`SparqlResult`] (SELECT solutions, ASK boolean, or CONSTRUCT graph) into the
//! four W3C SPARQL Results formats — JSON (SRJ), XML, CSV, and TSV — plus an
//! additive, provenance-carrying `purrdf` extension. It replaces the
//! oxigraph-family `sparesults` on the results path (purrdf S9).
//!
//! It depends **only** on `purrdf-core` (with `default-features = false`) so
//! it stays oxigraph-free and wasm-clean. Term and N-Triples syntax are produced
//! exclusively by the rdf-core kernel `emit_*` primitives (see `term`,
//! `graph`); this crate adds no term-syntax of its own.
//!
//! Scope: the shared infrastructure (error type, provenance carrier, term
//! lexicalization bridge, CONSTRUCT-graph N-Triples writer), the four per-format
//! document writers (JSON/XML/CSV/TSV), and the [`serialize`] dispatcher that
//! selects among them.

mod csv;
mod error;
mod graph;
mod json;
mod json_read;
mod model;
mod term;
mod tsv;
mod xml;
mod xml_read;

pub use csv::to_csv;
pub use error::Error;
pub use json::to_json;
pub use json_read::{ParsedSolutions, from_json, from_json_boolean};
pub use model::{ResultProvenance, SolutionProvenance};
pub use tsv::to_tsv;
pub use xml::to_xml;
pub use xml_read::{from_xml, from_xml_boolean};

/// Re-export of the egress result model this crate serializes, so consumers name
/// a single path (`purrdf_sparql_results::SparqlResult`).
pub use purrdf_core::SparqlResult;

/// The four W3C SPARQL Results serialization formats this crate targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlResultsFormat {
    /// SPARQL Results JSON (a.k.a. SRJ).
    Json,
    /// SPARQL Results XML.
    Xml,
    /// SPARQL Results CSV.
    Csv,
    /// SPARQL Results TSV.
    Tsv,
}

/// The result of a serialization: the encoded bytes plus an exit-gate flag.
#[derive(Debug, Clone)]
pub struct SerializeOutcome {
    /// The serialized result document.
    pub bytes: Vec<u8>,
    /// True when a non-empty [`ResultProvenance`] was requested but the chosen
    /// format could not carry it. CSV and TSV are pure-W3C value-only formats
    /// with no extension point, so a populated provenance is trimmed at the exit
    /// gate and this flag is set, letting the caller detect the lossy projection.
    pub provenance_dropped: bool,
}

/// Serialize a [`SparqlResult`] to the requested [`SparqlResultsFormat`],
/// carrying the additive `purrdf` provenance extension where the format allows.
///
/// This is the single public entry point: it dispatches to the per-format
/// writer ([`to_json`], [`to_xml`], [`to_csv`], [`to_tsv`]).
///
/// # Errors
///
/// Propagates the per-format [`Error`]. Notably, the result-kind support matrix
/// is enforced by the writers: XML rejects CONSTRUCT graphs, and CSV/TSV reject
/// both ASK booleans and CONSTRUCT graphs, all via [`Error::Format`].
pub fn serialize(
    result: &SparqlResult,
    format: SparqlResultsFormat,
    provenance: &ResultProvenance,
) -> Result<SerializeOutcome, Error> {
    match format {
        SparqlResultsFormat::Json => to_json(result, provenance),
        SparqlResultsFormat::Xml => to_xml(result, provenance),
        SparqlResultsFormat::Csv => to_csv(result, provenance),
        SparqlResultsFormat::Tsv => to_tsv(result, provenance),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{RdfDatasetBuilder, TermValue};

    fn select_one() -> SparqlResult {
        SparqlResult::Solutions {
            variables: vec!["s".to_string()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/s".to_string(),
            ))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        }
    }

    #[test]
    fn dispatch_routes_each_format() {
        let result = select_one();
        let prov = ResultProvenance::default();

        let json = serialize(&result, SparqlResultsFormat::Json, &prov).expect("json");
        assert!(
            String::from_utf8(json.bytes)
                .expect("utf8")
                .starts_with('{')
        );

        let xml = serialize(&result, SparqlResultsFormat::Xml, &prov).expect("xml");
        assert!(
            String::from_utf8(xml.bytes)
                .expect("utf8")
                .starts_with("<?xml")
        );

        let csv = serialize(&result, SparqlResultsFormat::Csv, &prov).expect("csv");
        assert_eq!(
            String::from_utf8(csv.bytes).expect("utf8"),
            "s\r\nhttp://example.org/s\r\n"
        );

        let tsv = serialize(&result, SparqlResultsFormat::Tsv, &prov).expect("tsv");
        assert_eq!(
            String::from_utf8(tsv.bytes).expect("utf8"),
            "?s\n<http://example.org/s>\n"
        );
    }
}
