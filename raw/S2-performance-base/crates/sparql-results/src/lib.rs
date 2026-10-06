// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

//! Native, wasm-clean serializer for the SPARQL result model.
//!
//! This crate is the canonical authority for turning a `purrdf-core`
//! [`SparqlResult`] (SELECT solutions, ASK boolean, or CONSTRUCT graph) into the
//! four W3C SPARQL Results formats — JSON (SRJ), XML, CSV, and TSV — plus an
//! additive, provenance-carrying `purrdf` extension.
//!
//! It depends **only** on `purrdf-core` (with `default-features = false`) so
//! it stays wasm-clean. Term and N-Triples syntax are produced
//! exclusively by the rdf-core kernel `emit_*` primitives (see `term`,
//! `graph`); this crate adds no term-syntax of its own.
//!
//! Scope: the shared infrastructure (error type, provenance carrier, term
//! lexicalization bridge, CONSTRUCT-graph N-Triples writer), the four per-format
//! document writers (JSON/XML/CSV/TSV), and the [`serialize`] dispatcher that
//! selects among them.
//!
//! # Examples
//!
//! Serialize a one-row `SELECT` result to SPARQL Results JSON and parse it
//! back:
//!
//! ```
//! use purrdf_core::{RdfDatasetBuilder, TermValue};
//! use purrdf_sparql_results::{
//!     ResultProvenance, SparqlResult, SparqlResultsFormat, from_json, serialize,
//! };
//!
//! let result = SparqlResult::Solutions {
//!     variables: vec!["s".to_string()],
//!     rows: vec![vec![Some(TermValue::Iri("http://example.org/alice".to_string()))]],
//!     aux: RdfDatasetBuilder::new().freeze().expect("empty aux dataset"),
//! };
//!
//! let outcome = serialize(
//!     &result,
//!     SparqlResultsFormat::Json,
//!     &ResultProvenance::default(),
//!     None,
//! )
//! .expect("SELECT serializes to JSON");
//! assert!(!outcome.provenance_dropped);
//!
//! let parsed = from_json(&outcome.bytes).expect("emitted SRJ parses back");
//! assert_eq!(parsed.variables, ["s"]);
//! assert_eq!(
//!     parsed.rows,
//!     [vec![Some(TermValue::Iri("http://example.org/alice".to_string()))]],
//! );
//! ```

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
pub use json_read::{
    BoundedParsedSolutions, ParsedSolutions, from_json, from_json_boolean, from_json_bounded,
    provenance_from_json,
};
pub use model::{ProvenanceNamespace, ResultProvenance, SolutionProvenance};
pub use tsv::to_tsv;
pub use xml::to_xml;
pub use xml_read::{from_xml, from_xml_boolean, provenance_from_xml};

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

impl SparqlResultsFormat {
    /// Every format, in the order a server prefers them when a client accepts all four:
    /// JSON first, the default.
    pub const ALL: [Self; 4] = [Self::Json, Self::Xml, Self::Csv, Self::Tsv];

    /// Every format's spellings, in [`Self::ALL`] (declaration) order: its short
    /// token, the internet media type the W3C SPARQL 1.1 Query Results format
    /// specifications register for it (`text/csv` and `text/tab-separated-values`
    /// for CSV and TSV), and its spelled-out aliases.
    ///
    /// The one table every spelling is read from, so [`Self::token`],
    /// [`Self::media_type`] and [`Self::from_name`] cannot drift apart.
    const SPELLINGS: [(&'static str, &'static str, &'static [&'static str]); 4] = [
        (
            "json",
            "application/sparql-results+json",
            &["srj", "sparql-json"],
        ),
        ("xml", "application/sparql-results+xml", &["sparql-xml"]),
        ("csv", "text/csv", &[]),
        ("tsv", "text/tab-separated-values", &[]),
    ];

    /// The format a caller named, or `None` for a name that is not one of the four.
    ///
    /// The one reading of a results-format name for every host — the command line, the C
    /// ABI, the wasm package and the Python binding — so a name one host accepts is not
    /// refused by another. It accepts the short token ([`Self::token`]), the media type
    /// ([`Self::media_type`]), and the aliases `srj`, `sparql-json` and `sparql-xml`,
    /// ignoring ASCII case and surrounding whitespace. A media type with parameters, an
    /// RDF syntax name (`turtle`, `jsonld`) and anything else is `None`: a graph result
    /// is serialized as RDF, not through this type.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim();
        Self::ALL.into_iter().find(|&format| {
            let (token, media_type, aliases) = Self::SPELLINGS[format as usize];
            [token, media_type]
                .iter()
                .chain(aliases)
                .any(|candidate| candidate.eq_ignore_ascii_case(name))
        })
    }

    /// The format's short token — `json`, `xml`, `csv` or `tsv` — which
    /// [`Self::from_name`] reads back.
    #[must_use]
    pub const fn token(self) -> &'static str {
        Self::SPELLINGS[self as usize].0
    }

    /// The format's registered media type, for a `Content-Type` or an `Accept`
    /// negotiation; [`Self::from_name`] reads it back.
    #[must_use]
    pub const fn media_type(self) -> &'static str {
        Self::SPELLINGS[self as usize].1
    }
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
/// carrying the additive provenance extension where the format allows AND a
/// [`ProvenanceNamespace`] is supplied to anchor it under.
///
/// This is the single public entry point: it dispatches to the per-format
/// writer ([`to_json`], [`to_xml`], [`to_csv`], [`to_tsv`]). `namespace` is
/// consulted only by the JSON/XML writers — CSV/TSV have no extension point at
/// all and trim any non-empty `provenance` regardless. PurRDF mints no
/// vocabulary IRIs of its own: with `namespace: None`, JSON/XML emit no
/// provenance element/member either, however populated `provenance` is (see
/// [`ProvenanceNamespace`]).
///
/// # Examples
///
/// ```
/// use purrdf_core::{RdfDatasetBuilder, TermValue};
/// use purrdf_sparql_results::{
///     ResultProvenance, SparqlResult, SparqlResultsFormat, serialize,
/// };
///
/// let result = SparqlResult::Solutions {
///     variables: vec!["s".to_string()],
///     rows: vec![vec![Some(TermValue::Iri("http://example.org/s".to_string()))]],
///     aux: RdfDatasetBuilder::new().freeze().expect("empty aux dataset"),
/// };
///
/// let tsv = serialize(
///     &result,
///     SparqlResultsFormat::Tsv,
///     &ResultProvenance::default(),
///     None,
/// )
/// .expect("SELECT serializes to TSV");
/// assert_eq!(tsv.bytes, b"?s\n<http://example.org/s>\n");
/// ```
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
    namespace: Option<&ProvenanceNamespace>,
) -> Result<SerializeOutcome, Error> {
    match format {
        SparqlResultsFormat::Json => to_json(result, provenance, namespace),
        SparqlResultsFormat::Xml => to_xml(result, provenance, namespace),
        SparqlResultsFormat::Csv => to_csv(result, provenance),
        SparqlResultsFormat::Tsv => to_tsv(result, provenance),
    }
}

/// What a STREAMING serialization reports in place of [`SerializeOutcome`].
///
/// The same exit-gate flag, plus the byte count — the bytes themselves went to the
/// caller's writer and were never held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "provenance_dropped is an exit gate; discarding it discards the report"]
pub struct StreamOutcome {
    /// Bytes handed to the writer. Meaningful only on `Ok`.
    pub bytes_written: u64,
    /// Identical in meaning, and identically computed, to
    /// [`SerializeOutcome::provenance_dropped`].
    pub provenance_dropped: bool,
}

/// Serialize a result INTO `writer`: the same bytes as [`serialize`], written
/// incrementally rather than accumulated.
///
/// Peak residency on the output side is the sink's fixed staging window rather than
/// the document. What that does NOT bound is the result itself: a `Solutions` value
/// holds its rows by construction, and this writes them out — it does not make the
/// answer smaller.
///
/// # Errors
///
/// Every per-format [`Error`] [`serialize`] reports, plus [`Error::Write`] when
/// `writer` fails. Format- and kind-level refusals are all decided before the first
/// byte; a per-term failure or a write failure may leave a PREFIX of the document
/// already written, and the caller must discard the target.
pub fn serialize_into(
    result: &SparqlResult,
    format: SparqlResultsFormat,
    provenance: &ResultProvenance,
    namespace: Option<&ProvenanceNamespace>,
    writer: &mut dyn std::io::Write,
) -> Result<StreamOutcome, Error> {
    let mut drain = purrdf_core::sink::WriterDrain(writer);
    let mut out = purrdf_core::sink::TextSink::to_drain(&mut drain);
    match format {
        SparqlResultsFormat::Json => json::write_srj(result, provenance, namespace, &mut out)?,
        SparqlResultsFormat::Xml => xml::write_srx(result, provenance, namespace, &mut out)?,
        SparqlResultsFormat::Csv => csv::write_csv(result, provenance, &mut out)?,
        SparqlResultsFormat::Tsv => tsv::write_tsv(result, provenance, &mut out)?,
    }
    let finished = out.finish().map_err(|error| Error::Write {
        kind: error.kind(),
        message: error.message().to_owned(),
    })?;
    Ok(StreamOutcome {
        bytes_written: finished.written,
        provenance_dropped: provenance_dropped(format, provenance, namespace),
    })
}

/// Whether the provenance extension was requested but could not be emitted.
///
/// Computed at ONE site, so the eager and streaming outcomes cannot disagree about
/// it. CSV and TSV have no extension point at all, so a requested provenance is
/// always dropped there.
fn provenance_dropped(
    format: SparqlResultsFormat,
    provenance: &ResultProvenance,
    namespace: Option<&ProvenanceNamespace>,
) -> bool {
    match format {
        SparqlResultsFormat::Json | SparqlResultsFormat::Xml => {
            !provenance.is_empty() && namespace.is_none()
        }
        SparqlResultsFormat::Csv | SparqlResultsFormat::Tsv => !provenance.is_empty(),
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

        let json = serialize(&result, SparqlResultsFormat::Json, &prov, None).expect("json");
        assert!(
            String::from_utf8(json.bytes)
                .expect("utf8")
                .starts_with('{')
        );

        let xml = serialize(&result, SparqlResultsFormat::Xml, &prov, None).expect("xml");
        assert!(
            String::from_utf8(xml.bytes)
                .expect("utf8")
                .starts_with("<?xml")
        );

        let csv = serialize(&result, SparqlResultsFormat::Csv, &prov, None).expect("csv");
        assert_eq!(
            String::from_utf8(csv.bytes).expect("utf8"),
            "s\r\nhttp://example.org/s\r\n"
        );

        let tsv = serialize(&result, SparqlResultsFormat::Tsv, &prov, None).expect("tsv");
        assert_eq!(
            String::from_utf8(tsv.bytes).expect("utf8"),
            "?s\n<http://example.org/s>\n"
        );
    }

    #[test]
    fn every_name_a_host_accepts_names_its_format() {
        for (name, format) in [
            ("json", SparqlResultsFormat::Json),
            ("srj", SparqlResultsFormat::Json),
            ("sparql-json", SparqlResultsFormat::Json),
            ("application/sparql-results+json", SparqlResultsFormat::Json),
            ("xml", SparqlResultsFormat::Xml),
            ("sparql-xml", SparqlResultsFormat::Xml),
            ("application/sparql-results+xml", SparqlResultsFormat::Xml),
            ("csv", SparqlResultsFormat::Csv),
            ("text/csv", SparqlResultsFormat::Csv),
            ("tsv", SparqlResultsFormat::Tsv),
            ("text/tab-separated-values", SparqlResultsFormat::Tsv),
        ] {
            assert_eq!(SparqlResultsFormat::from_name(name), Some(format), "{name}");
        }
    }

    #[test]
    fn a_name_is_read_without_regard_to_ascii_case_or_surrounding_whitespace() {
        for name in [
            "JSON",
            " Json ",
            "\tjson\n",
            "Application/SPARQL-Results+JSON",
            "SRJ",
        ] {
            assert_eq!(
                SparqlResultsFormat::from_name(name),
                Some(SparqlResultsFormat::Json),
                "{name:?}"
            );
        }
        assert_eq!(
            SparqlResultsFormat::from_name("TEXT/Tab-Separated-Values"),
            Some(SparqlResultsFormat::Tsv)
        );
    }

    #[test]
    fn a_name_outside_the_four_formats_is_refused() {
        for name in [
            "",
            " ",
            "turtle",
            "jsonld",
            "rdfxml",
            "jsonx",
            "js on",
            "application/json",
            "application/sparql-results+json; charset=utf-8",
            "text/csv2",
            "sparql-csv",
        ] {
            assert_eq!(SparqlResultsFormat::from_name(name), None, "{name:?}");
        }
    }

    #[test]
    fn the_token_and_the_media_type_read_back_to_their_format() {
        assert_eq!(
            SparqlResultsFormat::ALL.map(SparqlResultsFormat::token),
            ["json", "xml", "csv", "tsv"]
        );
        assert_eq!(
            SparqlResultsFormat::ALL.map(SparqlResultsFormat::media_type),
            [
                "application/sparql-results+json",
                "application/sparql-results+xml",
                "text/csv",
                "text/tab-separated-values",
            ]
        );
        for format in SparqlResultsFormat::ALL {
            assert_eq!(SparqlResultsFormat::from_name(format.token()), Some(format));
            assert_eq!(
                SparqlResultsFormat::from_name(format.media_type()),
                Some(format)
            );
        }
    }
}
