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
//! additive, provenance-carrying `purrdf` extension. It replaces the
//! oxigraph-family `sparesults` on the results path.
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
///
/// The variant order is the SPARQL Protocol's server-preference order — JSON first, the
/// default a host answers with when a client states no preference — and [`Self::ALL`]
/// carries it, so a negotiating host iterates this crate's order rather than its own.
///
/// # One name table
///
/// Every host boundary (the CLI, the C ABI, the WebAssembly package, the Python extension)
/// takes a results format by NAME, and each once carried its own spelling table. They now
/// share this one: [`Self::from_name`] resolves the canonical token ([`Self::token`]), the
/// accepted aliases, and the media type ([`Self::media_type`]), ASCII-case-insensitively
/// and ignoring surrounding whitespace. A spelling this method does not resolve is one no
/// host accepts.
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

/// Every spelling [`SparqlResultsFormat::from_name`] resolves, beside the format it names.
///
/// The canonical token first, then the aliases, then the media type — the order is
/// documentation only, since the lookup is a full scan over a table this small.
const FORMAT_NAMES: [(&str, SparqlResultsFormat); 11] = [
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
];

impl SparqlResultsFormat {
    /// Every format, in server-preference order (JSON first).
    pub const ALL: [Self; 4] = [Self::Json, Self::Xml, Self::Csv, Self::Tsv];

    /// The canonical short token naming this format: `json`, `xml`, `csv` or `tsv`.
    ///
    /// This is the spelling the SPARQL Protocol negotiation returns and the one every
    /// host's diagnostics use; [`Self::from_name`] resolves it back.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Csv => "csv",
            Self::Tsv => "tsv",
        }
    }

    /// The registered media type of this format's documents, for a `Content-Type`.
    #[must_use]
    pub const fn media_type(self) -> &'static str {
        match self {
            Self::Json => "application/sparql-results+json",
            Self::Xml => "application/sparql-results+xml",
            Self::Csv => "text/csv",
            Self::Tsv => "text/tab-separated-values",
        }
    }

    /// Resolve a caller-supplied format name, or `None` when no format is spelled that way.
    ///
    /// Accepted, ASCII-case-insensitively and with surrounding whitespace ignored:
    ///
    /// | Format | Token | Aliases | Media type |
    /// |---|---|---|---|
    /// | JSON | `json` | `srj`, `sparql-json` | `application/sparql-results+json` |
    /// | XML | `xml` | `sparql-xml` | `application/sparql-results+xml` |
    /// | CSV | `csv` | — | `text/csv` |
    /// | TSV | `tsv` | — | `text/tab-separated-values` |
    ///
    /// A media type with parameters (`text/csv; charset=utf-8`) is not a name and is not
    /// resolved: a caller holding an `Accept` or `Content-Type` header negotiates it
    /// through the protocol module, which understands parameters and weights, rather than
    /// through a name lookup that would have to guess which parameters are harmless.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim();
        FORMAT_NAMES
            .iter()
            .find(|(spelling, _)| spelling.eq_ignore_ascii_case(name))
            .map(|&(_, format)| format)
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
use purrdf_core::term_fixture as test_terms;

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

    /// Every token and media type round-trips through `from_name`, in either case and
    /// with surrounding whitespace, and every alias lands on the format it abbreviates.
    #[test]
    fn from_name_resolves_tokens_aliases_and_media_types() {
        for format in SparqlResultsFormat::ALL {
            assert_eq!(SparqlResultsFormat::from_name(format.token()), Some(format));
            assert_eq!(
                SparqlResultsFormat::from_name(format.media_type()),
                Some(format)
            );
            assert_eq!(
                SparqlResultsFormat::from_name(&format.token().to_ascii_uppercase()),
                Some(format)
            );
            assert_eq!(
                SparqlResultsFormat::from_name(&format!("  {}\t", format.media_type())),
                Some(format)
            );
        }
        assert_eq!(
            SparqlResultsFormat::from_name("srj"),
            Some(SparqlResultsFormat::Json)
        );
        assert_eq!(
            SparqlResultsFormat::from_name("sparql-json"),
            Some(SparqlResultsFormat::Json)
        );
        assert_eq!(
            SparqlResultsFormat::from_name("SPARQL-XML"),
            Some(SparqlResultsFormat::Xml)
        );
        assert_eq!(
            SparqlResultsFormat::from_name("Application/Sparql-Results+JSON"),
            Some(SparqlResultsFormat::Json)
        );
    }

    /// A spelling no host accepts stays refused: the empty name, rdflib's `txt` table
    /// format, a media type carrying parameters, and a token with interior whitespace.
    /// Each has a resolved neighbour, so the refusal is of the spelling and not of the
    /// format.
    #[test]
    fn from_name_refuses_spellings_no_host_accepts() {
        for unknown in [
            "",
            " ",
            "txt",
            "text/csv; charset=utf-8",
            "text/csv;charset=utf-8",
            "sparql json",
            "json/",
            "application/json",
            "ntriples",
        ] {
            assert_eq!(
                SparqlResultsFormat::from_name(unknown),
                None,
                "{unknown:?} must not resolve"
            );
        }
        // The neighbouring valid spellings of the refused ones.
        assert_eq!(
            SparqlResultsFormat::from_name("text/csv"),
            Some(SparqlResultsFormat::Csv)
        );
        assert_eq!(
            SparqlResultsFormat::from_name("sparql-json"),
            Some(SparqlResultsFormat::Json)
        );
        assert_eq!(
            SparqlResultsFormat::from_name("json"),
            Some(SparqlResultsFormat::Json)
        );
    }

    /// `ALL` is the server-preference order the protocol negotiates in, and the token and
    /// media type of each entry are distinct from every other entry's.
    #[test]
    fn all_is_ordered_and_names_are_distinct() {
        assert_eq!(
            SparqlResultsFormat::ALL.map(SparqlResultsFormat::token),
            ["json", "xml", "csv", "tsv"]
        );
        let mut names: Vec<&str> = FORMAT_NAMES.iter().map(|&(name, _)| name).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            before,
            "every spelling names exactly one format"
        );
        for format in SparqlResultsFormat::ALL {
            assert!(
                FORMAT_NAMES
                    .iter()
                    .any(|&(name, f)| f == format && name == format.token())
            );
            assert!(
                FORMAT_NAMES
                    .iter()
                    .any(|&(name, f)| f == format && name == format.media_type())
            );
        }
    }
}
