// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parse / serialize surface for the `purrdf` Python extension: the
//! `RdfFormat` pyclass, the `parse` / `serialize` module functions, and the
//! pure-Rust `parse_quads` / `serialize_triples` cores plus the `read_input`
//! helper the store seam shares.
//!
//! Native backing: the parse/serialize cores produce and consume the
//! oxigraph-free owned model (`RdfQuad` / `RdfTriple`) via the native codecs — no
//! `oxigraph::model::*` and no `flat_oxigraph_quads_from_dataset` bridge.

use std::sync::Arc;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

use super::query::{PyQueryQuads, PyQueryTriples};
use super::term::PyQuad;
use crate::{
    NativeRdfFormat, RdfDataset, RdfQuad, RdfTriple, SerializeGraph, SerializeOptions,
    StatementLayer, flat_dataset_from_quads, flat_rdf_quads_from_dataset, parse_dataset,
    serialize_dataset_to_format, serialize_dataset_with,
};

// ── RDF serialization format enum ───────────────────────────────────────────────

/// The RDF serialization formats the codebase loads/parses/serializes.
///
/// Mirrors the oxigraph Python `RdfFormat`; the members keep the SCREAMING_SNAKE Python
/// spelling (`RdfFormat.TURTLE`).
#[pyclass(name = "RdfFormat", eq, eq_int, from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
#[allow(
    clippy::upper_case_acronyms,
    reason = "the SCREAMING_SNAKE variant spellings ARE the Python-visible enum members (`RdfFormat.TURTLE`), so they must not be renamed"
)]
pub(crate) enum PyRdfFormat {
    TURTLE,
    N_TRIPLES,
    N_QUADS,
    TRIG,
    TRIX,
    HEXTUPLES,
    JSON_LD,
    YAML_LD,
}

impl PyRdfFormat {
    /// The native codec format selector: the always-on replacement for the
    /// oxigraph `RdfFormat` router on the parse/serialize path.
    pub(crate) fn to_native(self) -> NativeRdfFormat {
        match self {
            Self::TURTLE => NativeRdfFormat::Turtle,
            Self::N_TRIPLES => NativeRdfFormat::NTriples,
            Self::N_QUADS => NativeRdfFormat::NQuads,
            Self::TRIG => NativeRdfFormat::TriG,
            Self::TRIX => NativeRdfFormat::TriX,
            Self::HEXTUPLES => NativeRdfFormat::HexTuples,
            Self::JSON_LD => NativeRdfFormat::JsonLd,
            Self::YAML_LD => NativeRdfFormat::YamlLd,
        }
    }

    /// The Python-visible spelling of this member (`RdfFormat.N_QUADS`), for diagnostics.
    ///
    /// A Python caller never sees the native `NativeRdfFormat` name or a media type, so
    /// a message that names either would be asking them to re-derive which argument they
    /// passed. Written out rather than derived from `Debug` so the strings cannot drift
    /// away from the members if the Rust variant spellings ever change.
    pub(crate) fn member_name(self) -> &'static str {
        match self {
            Self::TURTLE => "RdfFormat.TURTLE",
            Self::N_TRIPLES => "RdfFormat.N_TRIPLES",
            Self::N_QUADS => "RdfFormat.N_QUADS",
            Self::TRIG => "RdfFormat.TRIG",
            Self::TRIX => "RdfFormat.TRIX",
            Self::HEXTUPLES => "RdfFormat.HEXTUPLES",
            Self::JSON_LD => "RdfFormat.JSON_LD",
            Self::YAML_LD => "RdfFormat.YAML_LD",
        }
    }
}

// ── The realized serialization loss ─────────────────────────────────────────────

/// One serialized document plus the WHOLE realized loss of producing it, partitioned
/// by CAUSE — the return of `Store.dump_with_loss` / `MutableDataset.dump_with_loss`.
///
/// # Why counts rather than a refusal
///
/// A graph-carrying CONSTRUCT result REFUSES a single-graph syntax (`QueryQuads`),
/// because the graph name is in the query the caller wrote. Dumping a store is the
/// opposite situation: asking a multi-graph store for Turtle is a legitimate "give me
/// the default graph", so the honest answer is not to refuse but to say what was left
/// behind. `dump` answers with bytes alone, exactly as it always has; this is the same
/// serialization with the numbers attached.
///
/// # Why three counts rather than one flag
///
/// The causes are independent, so their sum is the total and no row is charged twice.
/// A star-capable single-graph target (`TURTLE`, `N_TRIPLES`) loses named graphs and no
/// statement rows; a dataset-capable star-incapable one (`TRIX`, `HEXTUPLES`) loses
/// statement rows and base directions and no named graphs. Reading one count alone
/// cannot distinguish "nothing was lost" from "the loss was charged to a cause I am not
/// reading" — which is exactly how a silent drop hides.
///
/// Every count is REALIZED — what this document actually discarded — not the static
/// pair contract `purrdf.loss_matrix_json()` describes.
///
/// # `statement_rows_dropped` on THIS lane
///
/// The Python quad model carries the RDF-1.2 statement layer as ordinary `rdf:reifies`
/// / annotation quads with triple-term objects rather than as a separate layer (see
/// [`dataset_from_quads_verbatim`]), so a star-incapable target does not silently drop
/// them here — it REFUSES the triple term outright. The count is reported anyway, and
/// with the same meaning as on every other host: it is the same
/// `SerializeOutcome` field the C ABI's `out_statement_rows_dropped` and the wasm
/// `statementRowsDropped` read, so a caller comparing hosts is comparing one number,
/// and a future statement-layer ingress on this lane needs no new surface.
#[pyclass(name = "SerializeLoss", frozen, skip_from_py_object)]
#[derive(Debug)]
pub(crate) struct PySerializeLoss {
    bytes: Vec<u8>,
    statement_rows_dropped: usize,
    directional_literals_dropped: usize,
    named_graph_rows_dropped: usize,
}

#[pymethods]
impl PySerializeLoss {
    /// The serialized document, identical to what `dump(format=…)` returns.
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.bytes)
    }

    /// RDF-1.2 statement-layer rows (reifier bindings + annotation triples) dropped
    /// because the target cannot represent quoted triples. `0` for star-capable
    /// formats. Rows dropped for being scoped to a named graph are counted by
    /// `named_graph_rows_dropped` instead, never here and never twice.
    #[getter]
    const fn statement_rows_dropped(&self) -> usize {
        self.statement_rows_dropped
    }

    /// Object literals whose RDF-1.2 base direction the target has no surface for
    /// (`TRIX` / `HEXTUPLES` keep the language tag but cannot carry the direction).
    /// `0` for every direction-capable format.
    #[getter]
    const fn directional_literals_dropped(&self) -> usize {
        self.directional_literals_dropped
    }

    /// Rows the single-graph flattening dropped because the target has no named-graph
    /// construct: base quads asserted in a named graph plus the statement-layer rows
    /// scoped to one. `0` for every dataset-capable format. The rows are DROPPED, not
    /// folded into the default graph.
    #[getter]
    const fn named_graph_rows_dropped(&self) -> usize {
        self.named_graph_rows_dropped
    }

    fn __repr__(&self) -> String {
        format!(
            "SerializeLoss(bytes={}, statement_rows_dropped={}, \
             directional_literals_dropped={}, named_graph_rows_dropped={})",
            self.bytes.len(),
            self.statement_rows_dropped,
            self.directional_literals_dropped,
            self.named_graph_rows_dropped
        )
    }
}

/// Serialize the whole quad set to `format` and report every realized loss.
///
/// The pure-Rust core behind both `dump_with_loss` methods, so the store and the
/// mutable dataset can never report the same document differently. `SerializeGraph`
/// is `Dataset` unconditionally — a graph SELECTION would make the named-graph count
/// meaningless, since the caller would already have chosen what to keep — matching the
/// C ABI's `purrdf_serialize`, which has no selection parameter for the same reason.
pub(super) fn dump_quads_with_loss(
    quads: &[RdfQuad],
    format: NativeRdfFormat,
) -> Result<PySerializeLoss, String> {
    let dataset = flat_dataset_from_quads(quads)?;
    let outcome = serialize_dataset_to_format(&dataset, format, None).map_err(|e| e.to_string())?;
    Ok(PySerializeLoss {
        bytes: outcome.bytes,
        statement_rows_dropped: outcome.statement_rows_dropped,
        directional_literals_dropped: outcome.directional_literals_dropped,
        named_graph_rows_dropped: outcome.named_graph_rows_dropped,
    })
}

// ── Module-level functions ──────────────────────────────────────────────────────

/// Parse RDF bytes/str into a list of `Quad`. Mirrors the oxigraph Python `parse`.
///
/// Unlike `Store.load`, blank-node labels are preserved verbatim (no renaming),
/// so canonicalization over the parsed quads is meaningful.
///
/// `base` is the document base relative IRI references resolve against, the same
/// parameter `Store.load` carries and the same one the WebAssembly and C surfaces
/// take. It is genuinely optional but never silently defaulted: PurRDF is handed
/// bytes and has no retrieval IRI, so omitting it means "no base in scope" and a
/// relative reference then hard-fails with `iri-relative-no-base` rather than being
/// resolved against a fabricated one. A document may still establish its own base
/// (Turtle `@base`, `xml:base`, JSON-LD `@context.@base`), which wins over this one.
#[pyfunction]
#[pyo3(signature = (input, format, *, base=None))]
pub(crate) fn parse(
    py: Python<'_>,
    input: &Bound<'_, PyAny>,
    format: PyRdfFormat,
    base: Option<String>,
) -> PyResult<Vec<Py<PyQuad>>> {
    let data = read_input(Some(input), None)?;
    // The native parse runs detached (GIL released); the Quad objects are
    // built after reacquiring.
    let quads = py
        .detach(move || parse_quads(&data, format.to_native(), base.as_deref()))
        .map_err(|e| PyValueError::new_err(format!("parse error: {e}")))?;
    quads
        .into_iter()
        .map(|inner| Py::new(py, PyQuad { inner }))
        .collect()
}

/// Serialize a `QueryTriples` or a `QueryQuads` in `format`. Mirrors the oxigraph
/// Python `serialize`: when `output` (a file-like with `.write`) is given the bytes are
/// written to it and `None` is returned; when `output` is omitted the serialized
/// `bytes` are returned directly.
///
/// Both CONSTRUCT result types are accepted, and each delegates to its OWN
/// `serialize` method rather than re-implementing it, so the module function and the
/// method can never disagree — in particular a `QueryQuads` refuses a single-graph
/// syntax here exactly as it does there.
///
/// `base` is the document base the output is written under — the egress mirror of
/// [`parse`]'s. A format that can express a base (Turtle, TriG, JSON-LD, YAML-LD)
/// writes it and relativizes its IRIs against it; one that cannot (N-Triples,
/// N-Quads, TriX, HexTuples) emits absolute IRIs, which is the only spelling those
/// grammars admit. Either way a base that is not an absolute IRI is a hard failure,
/// never silently ignored.
#[pyfunction]
#[pyo3(signature = (input, output=None, format=None, *, base=None))]
pub(crate) fn serialize(
    py: Python<'_>,
    input: &Bound<'_, PyAny>,
    output: Option<&Bound<'_, PyAny>>,
    format: Option<PyRdfFormat>,
    base: Option<String>,
) -> PyResult<Option<Py<PyBytes>>> {
    let format = format.ok_or_else(|| PyValueError::new_err("serialize: format is required"))?;
    let bytes = if let Ok(triples) = input.cast::<PyQueryTriples>() {
        triples.borrow().serialize_bytes(py, format, base)?
    } else if let Ok(quads) = input.cast::<PyQueryQuads>() {
        quads.borrow().serialize_bytes(py, format, base)?
    } else {
        return Err(PyTypeError::new_err(
            "serialize: input must be a QueryTriples or a QueryQuads",
        ));
    };
    match output {
        Some(output) => {
            output.call_method1("write", (PyBytes::new(py, &bytes),))?;
            Ok(None)
        }
        None => Ok(Some(PyBytes::new(py, &bytes).unbind())),
    }
}

// ── Pure-Rust cores (unit-tested without a Python interpreter) ───────────────────

/// Parse RDF bytes into owned quads via the native codec with no blank-node
/// renaming. Routes through [`parse_dataset`](crate::parse_dataset) → IR → the flat
/// quad un-fold ([`flat_rdf_quads_from_dataset`]) so the `rdf:reifies` / annotation
/// rows of the RDF 1.2 statement layer reappear in the quad stream exactly as a flat
/// parse would yield them. Private-use language tags such as `@x-purrdf-*` are valid
/// BCP-47 `x-…` privateuse tags and survive the native parse.
pub(crate) fn parse_quads(
    data: &[u8],
    format: NativeRdfFormat,
    base: Option<&str>,
) -> Result<Vec<RdfQuad>, String> {
    let dataset = parse_dataset(data, format.media_type(), base).map_err(|e| e.to_string())?;
    Ok(flat_rdf_quads_from_dataset(&dataset))
}

/// Serialize triples through the native codec under an optional document `base`.
///
/// `base` is the egress mirror of [`parse_quads`]'s: a syntax that can express a base
/// writes it and relativizes against it, one that cannot emits absolute IRIs, and a
/// base that is not an absolute IRI is a hard failure on either. The decision is the
/// core format registry's, never respelled here.
///
/// The selection is `SerializeGraph::Dataset` — which
/// [`serialize_dataset_to_format`] applies — rather than the `DefaultGraph` this used
/// to name, and the two are the SAME output for this input: every triple is built
/// below as a default-graph quad, so there is no named-graph row for the dataset
/// selection to keep. Nor is there a statement layer to drop: the IR is built verbatim
/// (RDF 1.2 triple-term objects stay triple-term objects, no `rdf:reifies` fold), so
/// the reifier and annotation tables are empty and star-incapable targets report zero
/// dropped rows. One entry point carries the base rather than a second `…_with_base`
/// overload beside it, which is how a call site ends up silently not passing one.
pub(crate) fn serialize_triples(
    triples: &[RdfTriple],
    format: NativeRdfFormat,
    base: Option<&str>,
) -> Result<Vec<u8>, String> {
    // Build the IR verbatim — every triple is a default-graph quad, RDF 1.2
    // triple-term objects preserved as triple-term objects (no statement-layer fold)
    // — then serialize through the native codec.
    let quads: Vec<RdfQuad> = triples
        .iter()
        .map(|t| RdfQuad::new(t.subject.clone(), t.predicate.clone(), t.object.clone()))
        .collect();
    let dataset = flat_dataset_from_quads(&quads)?;
    serialize_dataset_to_format(&dataset, format, base)
        .map(|outcome| outcome.bytes)
        .map_err(|e| e.to_string())
}

/// Serialize a graph-carrying CONSTRUCT result's quads through the native codec,
/// emitting **every** graph.
///
/// The quad twin of [`serialize_triples`]: same verbatim freeze (RDF 1.2 triple-term
/// objects preserved as triple-term objects, no statement-layer fold), but
/// [`SerializeGraph::Dataset`] rather than `DefaultGraph`, because the graph slot is
/// the whole reason this stream is quads. `format` is the caller's, unchecked here: the
/// single-graph syntaxes are refused BEFORE this point (`QueryQuads::serialize`), so a
/// format that reaches this function can carry what it is handed.
///
/// `base` is the document base the output is written under, carrying exactly the meaning
/// it does on [`serialize_triples`]: a graph-carrying result is no less an egress
/// document than a triple-carrying one, so it is written under the same base through the
/// same seam.
pub(crate) fn serialize_quads(
    quads: &[RdfQuad],
    format: NativeRdfFormat,
    base: Option<&str>,
) -> Result<Vec<u8>, String> {
    let dataset = flat_dataset_from_quads(quads)?;
    serialize_dataset_with(
        &dataset,
        format,
        base,
        &SerializeOptions {
            selection: SerializeGraph::Dataset,
            // The verbatim freeze this function promises: the RDF 1.2 statement layer is
            // EMITTED, never folded away, and a format with no surface for it fails
            // closed rather than dropping rows behind the caller's back.
            statement_layer: StatementLayer::Emit,
            jsonld_options: None,
        },
    )
    .map(|outcome| outcome.bytes)
    .map_err(|e| e.to_string())
}

/// Freeze a flat native quad list into the IR verbatim — RDF 1.2 triple-term objects
/// preserved as triple-term objects (no statement-layer fold), named graphs kept —
/// for native serialization. Shared by the `Store`/`MutableDataset` dump
/// paths.
pub(super) fn dataset_from_quads_verbatim(quads: &[RdfQuad]) -> Result<Arc<RdfDataset>, String> {
    flat_dataset_from_quads(quads)
}

pub(crate) fn read_input(
    input: Option<&Bound<'_, PyAny>>,
    path: Option<String>,
) -> PyResult<Vec<u8>> {
    if let Some(path) = path {
        return std::fs::read(&path)
            .map_err(|e| PyValueError::new_err(format!("cannot read `{path}`: {e}")));
    }
    let Some(input) = input else {
        return Err(PyValueError::new_err(
            "either `input` data or the `path` keyword must be given",
        ));
    };
    if let Ok(bytes) = input.cast::<PyBytes>() {
        return Ok(bytes.as_bytes().to_vec());
    }
    if let Ok(text) = input.cast::<PyString>() {
        return Ok(text.to_str()?.as_bytes().to_vec());
    }
    Err(PyTypeError::new_err("input must be bytes or str"))
}

#[cfg(test)]
mod tests {
    use crate::{RdfLiteral, RdfTerm};

    use super::*;

    const NQUADS_LANG: &str =
        "<https://example.org/s> <https://example.org/p> \"hallo\"@x-purrdf-afrikaans .";

    #[test]
    fn parse_quads_accepts_private_language_tag_in_nquads() {
        // The project's private-use language tags (`@x-purrdf-*`) must survive the
        // parse. The native N-Quads codec (purrdf-gts's own lenient tokenizer) accepts
        // them, including the >8-char subtag `afrikaans` that strict BCP-47 rejects.
        let quads = parse_quads(NQUADS_LANG.as_bytes(), NativeRdfFormat::NQuads, None)
            .expect("private-use language tags must parse via N-Quads");
        assert_eq!(quads.len(), 1);
        match &quads[0].object {
            RdfTerm::Literal(lit) => {
                assert_eq!(lit.lexical_form, "hallo");
                assert_eq!(lit.language.as_deref(), Some("x-purrdf-afrikaans"));
            }
            other => panic!("expected a literal, got {other:?}"),
        }
    }

    #[test]
    fn parse_quads_accepts_private_language_tag_in_turtle_and_ntriples() {
        // the project's `@x-purrdf-*` private-use tags exceed BCP-47's 8-char
        // subtag limit (`afrikaans` is 9 chars). purrdf-gts runs its Turtle / N-Triples
        // codecs in `lenient` mode — matching the prior oxigraph `RdfParser::lenient()`
        // path — so these tags parse in EVERY format, not just N-Quads.
        let ttl = concat!(
            "<https://example.org/s> <https://example.org/p> ",
            "\"hallo\"@x-purrdf-afrikaans ."
        );
        for format in [NativeRdfFormat::Turtle, NativeRdfFormat::NTriples] {
            let quads = parse_quads(ttl.as_bytes(), format, None)
                .unwrap_or_else(|e| panic!("{format:?} must accept the private-use tag: {e}"));
            assert_eq!(quads.len(), 1);
            match &quads[0].object {
                RdfTerm::Literal(lit) => {
                    assert_eq!(lit.lexical_form, "hallo");
                    assert_eq!(lit.language.as_deref(), Some("x-purrdf-afrikaans"));
                }
                other => panic!("expected a literal, got {other:?}"),
            }
        }
    }

    #[test]
    fn parse_quads_preserves_literal_lexical_form() {
        // A Store round-trip canonicalizes `+00:00` → `Z` and `0.70` → `0.7`;
        // the parse path must NOT, so the codec preserves the source lexical form.
        let ttl = concat!(
            "<https://example.org/s> <https://example.org/p> ",
            "\"2026-06-19T00:00:00+00:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime> ."
        );
        let quads = parse_quads(ttl.as_bytes(), NativeRdfFormat::Turtle, None).expect("parse");
        match &quads[0].object {
            RdfTerm::Literal(lit) => assert_eq!(lit.lexical_form, "2026-06-19T00:00:00+00:00"),
            other => panic!("expected a literal, got {other:?}"),
        }
    }

    #[test]
    fn parse_quads_reads_rdf12_quoted_triple() {
        // RDF 1.2 reifier: `<< s p o >>` as a quoted-triple object via rdf:reifies.
        let ttl = concat!(
            "<https://example.org/r> ",
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://example.org/s> <https://example.org/p> <https://example.org/o> )>> ."
        );
        let quads =
            parse_quads(ttl.as_bytes(), NativeRdfFormat::Turtle, None).expect("RDF 1.2 must parse");
        assert_eq!(quads.len(), 1);
        assert!(
            matches!(&quads[0].object, RdfTerm::Triple(_)),
            "object must be a quoted triple"
        );
    }

    #[test]
    fn serialize_triples_round_trips_ntriples() {
        let triple = RdfTriple::new(
            RdfTerm::iri("https://example.org/s"),
            "https://example.org/p",
            RdfTerm::iri("https://example.org/o"),
        );
        let bytes = serialize_triples(
            std::slice::from_ref(&triple),
            NativeRdfFormat::NTriples,
            None,
        )
        .unwrap();
        let reparsed = parse_quads(&bytes, NativeRdfFormat::NTriples, None).unwrap();
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0].subject.to_string(), "<https://example.org/s>");
    }

    #[test]
    fn serialize_triples_emits_literal() {
        let triple = RdfTriple::new(
            RdfTerm::iri("https://example.org/s"),
            "https://example.org/p",
            RdfTerm::literal(RdfLiteral::simple("hi")),
        );
        let bytes = serialize_triples(
            std::slice::from_ref(&triple),
            NativeRdfFormat::NTriples,
            None,
        )
        .unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("\"hi\""));
    }

    #[test]
    fn serialize_triples_base_is_honored_per_the_format_registry() {
        // A triple whose IRIs sit under the base: Turtle can express a base, so it
        // declares one and relativizes; N-Triples cannot, so it answers with absolute
        // IRIs rather than erroring. Same parameter, two grammars, one registry.
        let triple = RdfTriple::new(
            RdfTerm::iri("https://example.org/base/s"),
            "https://example.org/base/p",
            RdfTerm::iri("https://example.org/base/o"),
        );
        let base = Some("https://example.org/base/");

        let turtle =
            serialize_triples(std::slice::from_ref(&triple), NativeRdfFormat::Turtle, base)
                .expect("turtle under a base");
        let turtle = String::from_utf8(turtle).expect("utf-8");
        assert!(
            turtle.contains("@base <https://example.org/base/> ."),
            "Turtle must declare the base, got: {turtle}"
        );

        let nt = serialize_triples(
            std::slice::from_ref(&triple),
            NativeRdfFormat::NTriples,
            base,
        )
        .expect("n-triples under a base");
        let nt = String::from_utf8(nt).expect("utf-8");
        assert!(
            nt.contains("<https://example.org/base/s>"),
            "N-Triples must stay absolute under a base, got: {nt}"
        );
    }

    #[test]
    fn serialize_triples_without_a_base_is_unchanged_by_the_dataset_selection() {
        // This core moved from `SerializeGraph::DefaultGraph` to the `Dataset`
        // selection `serialize_dataset_to_format` applies, so that it could carry a
        // base at all. The two must agree byte-for-byte on this input: every triple is
        // a default-graph quad and the verbatim IR build leaves the RDF 1.2 statement
        // layer empty, so neither the graph filter nor the star-layer drop can differ.
        let triple = RdfTriple::new(
            RdfTerm::iri("https://example.org/s"),
            "https://example.org/p",
            RdfTerm::literal(RdfLiteral::simple("v")),
        );
        for format in [
            NativeRdfFormat::Turtle,
            NativeRdfFormat::NTriples,
            NativeRdfFormat::NQuads,
            NativeRdfFormat::TriG,
            NativeRdfFormat::TriX,
            NativeRdfFormat::HexTuples,
            NativeRdfFormat::JsonLd,
            NativeRdfFormat::YamlLd,
        ] {
            let quads = vec![RdfQuad::new(
                triple.subject.clone(),
                triple.predicate.clone(),
                triple.object.clone(),
            )];
            let dataset = flat_dataset_from_quads(&quads).expect("freeze");
            let baseline = crate::serialize_dataset(
                &dataset,
                format.media_type(),
                SerializeGraph::DefaultGraph,
            )
            .expect("baseline serialize");
            let current = serialize_triples(std::slice::from_ref(&triple), format, None)
                .expect("current serialize");
            assert_eq!(
                current, baseline,
                "{format:?} must be byte-identical to the previous default-graph selection"
            );
        }
    }

    #[test]
    fn rdfformat_maps_to_native() {
        assert_eq!(PyRdfFormat::TURTLE.to_native(), NativeRdfFormat::Turtle);
        assert_eq!(
            PyRdfFormat::N_TRIPLES.to_native(),
            NativeRdfFormat::NTriples
        );
        assert_eq!(PyRdfFormat::N_QUADS.to_native(), NativeRdfFormat::NQuads);
        assert_eq!(PyRdfFormat::TRIG.to_native(), NativeRdfFormat::TriG);
        assert_eq!(PyRdfFormat::TRIX.to_native(), NativeRdfFormat::TriX);
        assert_eq!(
            PyRdfFormat::HEXTUPLES.to_native(),
            NativeRdfFormat::HexTuples
        );
        assert_eq!(PyRdfFormat::JSON_LD.to_native(), NativeRdfFormat::JsonLd);
        assert_eq!(PyRdfFormat::YAML_LD.to_native(), NativeRdfFormat::YamlLd);
    }
}
