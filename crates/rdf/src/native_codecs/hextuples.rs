// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party HexTuples codec (NDJSON quads serialization).
//!
//! HexTuples is a line-oriented RDF serialization: one JSON array per line, six string
//! fields —
//! `[subject, predicate, value, datatype, language, graph]` — where
//!
//! * `subject` is an IRI or a `_:`-prefixed blank node,
//! * `predicate` is an IRI,
//! * `value` is the object's lexical value (an IRI / `_:` blank for node objects),
//! * `datatype` is the literal datatype IRI, or the sentinel `globalId` (IRI object) /
//!   `localId` (blank-node object),
//! * `language` is the BCP-47 language tag for a language-tagged literal (else empty),
//! * `graph` is the named-graph IRI / `_:` blank (empty for the default graph).
//!
//! Encoding and decoding reuse `serde_json` (already a dep — no new dependency, so the
//! crate stays wasm-clean). Emission is byte-deterministic: quads are written in dataset
//! order, one canonical JSON array per line. HexTuples is a CLASSIC quad syntax with no
//! RDF-1.2 triple-term surface: a triple term in a serialize request is a HARD error.

use std::sync::Arc;

use super::codec::RdfCodec;
use super::media_type::NativeRdfFormat;
use super::parse::{FoldNode, FoldRow, RDF_REIFIES, fold_statement_layer};
use super::ser_model::{SerGraph, SerTerm, SerTermKind};
use super::text_parse::LineParseMode;
use crate::{RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, TermId};
use purrdf_core::blank_label::{LabelAlphabet, is_valid_label};

/// The HexTuples codec: a standalone (non-line-family) [`RdfCodec`] over the
/// line-oriented NDJSON quads syntax. A classic quad syntax with no RDF-1.2 triple-term
/// surface, so it is star-INcapable, and its NDJSON parser carries no span-recording
/// tokenizer.
pub(super) struct HexTuplesCodec;

impl RdfCodec for HexTuplesCodec {
    fn parse(
        &self,
        text: &str,
        // HexTuples has no base directive, so the scope is left EXACTLY as handed in: the
        // base in force at the end of a HexTuples document is the caller's, and this codec
        // says so by touching nothing.
        base: &mut purrdf_iri::BaseScope,
        _mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        super::parse::catch_codec_panic(NativeRdfFormat::HexTuples, || {
            parse_hextuples_to_dataset(text, base)
        })
    }

    fn serialize_into(&self, graph: &SerGraph, out: &mut String) -> Result<(), RdfDiagnostic> {
        // Built whole, then appended. Unlike the four text formats, this one's document
        // is assembled as a TREE — XML nesting, or a `serde_json` value — so its writer
        // cannot emit a prefix before it knows what follows, and appending would mean
        // rebuilding the construction itself rather than redirecting its output. The
        // sink still earns its place here: the caller's buffer is the only one that
        // outlives the call, and this is the seam a streaming writer replaces.
        out.push_str(&serialize_ser_graph_to_hextuples(graph)?);
        Ok(())
    }
}

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
/// HexTuples datatype sentinel for an IRI object.
const GLOBAL_ID: &str = "globalId";
/// HexTuples datatype sentinel for a blank-node object.
const LOCAL_ID: &str = "localId";

fn parse_err(detail: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error(
        "native-codec-parse",
        format!("HexTuples: {}", detail.into()),
    )
}

fn serialize_err(detail: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error(
        "native-codec-serialize",
        format!("HexTuples: {}", detail.into()),
    )
}

// ───────────────────────────────────────────────────────────────────────────────
// Parse: HexTuples NDJSON → frozen RdfDataset IR (via the shared statement fold)
// ───────────────────────────────────────────────────────────────────────────────

/// A first-party HexTuples term the parser accumulates before interning. HexTuples is
/// classic (no triple terms).
#[derive(Clone, Debug)]
enum HexTerm {
    Iri(String),
    Blank(String),
    Literal(RdfLiteral),
}

/// One decoded HexTuples line: `(subject, predicate, object, graph)`.
type HexRow = (HexTerm, String, HexTerm, Option<HexTerm>);

/// Parse HexTuples `text` into a frozen [`RdfDataset`].
pub(super) fn parse_hextuples_to_dataset(
    text: &str,
    base: &purrdf_iri::BaseScope,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut parser = HexTuplesStreamParser::new(base.clone());
    for line in text.lines() {
        parser.push_line(line)?;
    }
    parser.finish()
}

/// Decode ONE physical HexTuples line, or `None` when the line is blank.
///
/// The one copy of the per-line grammar: both [`parse_hextuples_to_dataset`] (which
/// holds the whole document) and [`HexTuplesStreamParser`] (which never does) call it
/// with the same 1-based `lineno`, so a malformed line produces the same diagnostic
/// whichever way the document arrived.
fn parse_hextuples_line(
    line: &str,
    lineno: usize,
    base: &purrdf_iri::BaseScope,
) -> Result<Option<HexRow>, RdfDiagnostic> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    let fields: Vec<String> = serde_json::from_str(line)
        .map_err(|e| parse_err(format!("line {lineno}: invalid JSON array: {e}")))?;
    if fields.len() != 6 {
        return Err(parse_err(format!(
            "line {lineno}: expected 6 fields, found {}",
            fields.len()
        )));
    }
    let [subject, predicate, value, datatype, language, graph] =
        <[String; 6]>::try_from(fields).map_err(|_| parse_err("internal: field count mismatch"))?;
    let subject = node_term(&subject, base)?;
    validate_iri(&predicate, base)?;
    let object = object_term(&value, &datatype, &language, base)?;
    let graph = if graph.is_empty() {
        None
    } else {
        Some(node_term(&graph, base)?)
    };
    Ok(Some((subject, predicate, object, graph)))
}

/// The HexTuples parser driven ONE LINE AT A TIME, for a caller reading from a `Read`.
///
/// HexTuples is NDJSON: one self-contained JSON array per line, with no cross-line
/// state at all, so a streamed parse is the buffered parse with the source buffer
/// removed. Term interning still happens once, over the accumulated rows in document
/// order, in [`freeze_rows`] — that is the sequential point that fixes every term id,
/// and moving it would change the frozen IR, so it is left exactly where it was.
pub(super) struct HexTuplesStreamParser {
    rows: Vec<HexRow>,
    /// The 1-based document line number of the NEXT line to be pushed.
    lineno: usize,
    /// The base in scope, carried for the DIAGNOSTIC only — see [`validate_iri`].
    base: purrdf_iri::BaseScope,
}

impl HexTuplesStreamParser {
    pub(super) fn new(base: purrdf_iri::BaseScope) -> Self {
        Self {
            rows: Vec::new(),
            lineno: 1,
            base,
        }
    }

    /// Feed the next physical line, in document order (without its terminator).
    pub(super) fn push_line(&mut self, line: &str) -> Result<(), RdfDiagnostic> {
        if let Some(row) = parse_hextuples_line(line, self.lineno, &self.base)? {
            self.rows.push(row);
        }
        self.lineno += 1;
        Ok(())
    }

    /// Intern and freeze once the stream is exhausted.
    pub(super) fn finish(self) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        freeze_rows(self.rows)
    }
}

/// A subject / graph node: an IRI or a `_:`-prefixed blank node.
fn node_term(value: &str, base: &purrdf_iri::BaseScope) -> Result<HexTerm, RdfDiagnostic> {
    if let Some(label) = value.strip_prefix("_:") {
        validate_blank_label(label)?;
        Ok(HexTerm::Blank(label.to_owned()))
    } else {
        validate_iri(value, base)?;
        Ok(HexTerm::Iri(value.to_owned()))
    }
}

/// The object term, keyed by the `datatype` sentinel / IRI and the language field.
fn object_term(
    value: &str,
    datatype: &str,
    language: &str,
    base: &purrdf_iri::BaseScope,
) -> Result<HexTerm, RdfDiagnostic> {
    match datatype {
        GLOBAL_ID => {
            validate_iri(value, base)?;
            Ok(HexTerm::Iri(value.to_owned()))
        }
        LOCAL_ID => {
            let label = value.strip_prefix("_:").unwrap_or(value);
            validate_blank_label(label)?;
            Ok(HexTerm::Blank(label.to_owned()))
        }
        RDF_LANG_STRING if !language.is_empty() => Ok(HexTerm::Literal(RdfLiteral {
            lexical_form: value.to_owned(),
            datatype: None,
            language: Some(language.to_owned()),
            direction: None,
        })),
        "" | XSD_STRING => Ok(HexTerm::Literal(RdfLiteral::simple(value.to_owned()))),
        datatype => {
            validate_iri(datatype, base)?;
            Ok(HexTerm::Literal(RdfLiteral::typed(value, datatype)))
        }
    }
}

fn freeze_rows(rows: Vec<HexRow>) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut builder = RdfDatasetBuilder::new();
    let mut fold_rows: Vec<FoldRow> = Vec::with_capacity(rows.len());
    for (subject, predicate, object, graph) in rows {
        let subject = intern_term(&mut builder, &subject);
        let is_reifies = predicate == RDF_REIFIES;
        let predicate = builder.intern_iri(&predicate);
        let object = FoldNode::Term(intern_term(&mut builder, &object));
        let graph = graph.map(|g| intern_term(&mut builder, &g));
        fold_rows.push(FoldRow {
            subject,
            is_reifies,
            predicate,
            object,
            graph,
        });
    }
    fold_statement_layer(&mut builder, fold_rows)?;
    builder.freeze()
}

fn intern_term(builder: &mut RdfDatasetBuilder, term: &HexTerm) -> TermId {
    match term {
        HexTerm::Iri(iri) => builder.intern_iri(iri),
        // Text ingress: decode the `(label, scope)` encoding this codec's serializer
        // applied at egress, so a document it wrote re-parses to the very
        // `(label, scope)` pair it was written from. HexTuples types its blank ids
        // as `BLANK_NODE_LABEL`s, which is the alphabet the image test re-encodes
        // against.
        HexTerm::Blank(label) => builder.intern_text_blank(label, LabelAlphabet::BlankNodeLabel),
        HexTerm::Literal(literal) => builder.intern_literal(literal.clone()),
    }
}

/// Validate an IRI-position cell against the shared IRI layer.
///
/// HexTuples' row in `FORMATS` sets `admits_relative_iri: false`, so this routes through
/// [`BaseScope::resolve_absolute_only`](purrdf_iri::BaseScope::resolve_absolute_only):
/// a relative reference reports `iri-not-absolute-by-grammar` — the code that says
/// "supplying a base will not help" — and no base is ever applied, because none may be.
///
/// This replaces a hand-rolled check that tested only for the PRESENCE of a `:`, which
/// admitted a `path-noscheme` reference whose first segment merely contained one (RFC-3986
/// §4.2) as though it were absolute, and reported everything else as a generic parse
/// error. TriX and RDF/XML carried the byte-identical check; all three are gone.
///
/// The scope handed in is the CALLER'S, not a locally minted empty one. It is still never
/// applied — `resolve_absolute_only` refuses a relative reference whatever is in scope —
/// but the refusal can now say WHICH base is in scope and that it is deliberately not
/// applied here. The empty stand-in could only say "no base IRI is in scope", which was
/// false for anyone who had passed `--base`, and sent them looking for a dropped
/// parameter instead of at their document.
fn validate_iri(value: &str, base: &purrdf_iri::BaseScope) -> Result<(), RdfDiagnostic> {
    base.resolve_absolute_only(value)
        .map(|_| ())
        .map_err(|error| {
            RdfDiagnostic::error(error.diagnostic_code(), format!("HexTuples: {error}"))
        })
}

/// Blank-node label contract for a `_:`-prefixed HexTuples identifier: the same
/// [`LabelAlphabet::BlankNodeLabel`] alphabet this codec EMITS, so every document
/// the HexTuples serializer writes re-parses here.
fn validate_blank_label(label: &str) -> Result<(), RdfDiagnostic> {
    if is_valid_label(label, LabelAlphabet::BlankNodeLabel) {
        Ok(())
    } else {
        Err(parse_err(format!(
            "invalid blank-node identifier {label:?}"
        )))
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// Serialize: SerGraph → HexTuples NDJSON text (deterministic)
// ───────────────────────────────────────────────────────────────────────────────

/// Serialize a [`SerGraph`] to HexTuples NDJSON text. Quads are emitted in dataset
/// order (one canonical JSON array per line); annotation rows follow as plain triples.
/// A quoted-triple (RDF-1.2) term is a HARD error — HexTuples has no triple-term
/// surface.
pub(super) fn serialize_ser_graph_to_hextuples(graph: &SerGraph) -> Result<String, RdfDiagnostic> {
    let mut out = String::new();
    for &(s, p, o, g) in &graph.quads {
        write_line(&mut out, graph, s, p, o, g)?;
    }
    for &(rid, _, _) in &graph.reifiers {
        if is_self_reifier(graph, rid) {
            continue;
        }
        return Err(serialize_err(
            "cannot serialize an RDF-1.2 reifier binding (no triple-term surface)",
        ));
    }
    for &(r, p, v, g) in &graph.annotations {
        write_line(&mut out, graph, r, p, v, g)?;
    }
    Ok(out)
}

fn is_self_reifier(graph: &SerGraph, rid: usize) -> bool {
    graph
        .terms
        .get(rid)
        .is_some_and(|t| t.kind == SerTermKind::Triple && t.reifier == Some(rid))
}

fn write_line(
    out: &mut String,
    graph: &SerGraph,
    s: usize,
    p: usize,
    o: usize,
    g: Option<usize>,
) -> Result<(), RdfDiagnostic> {
    let subject = node_string(graph, s)?;
    let predicate = iri_string(graph, p)?;
    let (value, datatype, language) = object_fields(graph, o)?;
    let graph_field = match g {
        Some(gid) => node_string(graph, gid)?,
        None => String::new(),
    };
    let line = serde_json::to_string(&[subject, predicate, value, datatype, language, graph_field])
        .map_err(|e| serialize_err(format!("JSON encode failed: {e}")))?;
    out.push_str(&line);
    out.push('\n');
    Ok(())
}

/// A node term's HexTuples string: an IRI verbatim, or a `_:`-prefixed blank label.
fn node_string(graph: &SerGraph, tid: usize) -> Result<String, RdfDiagnostic> {
    let term = ser_term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => Ok(ser_value(term)?.to_owned()),
        SerTermKind::Bnode => Ok(format!("_:{}", ser_value(term)?)),
        other => Err(serialize_err(format!(
            "a subject / graph node must be an IRI or blank node, got {other:?}"
        ))),
    }
}

fn iri_string(graph: &SerGraph, tid: usize) -> Result<String, RdfDiagnostic> {
    let term = ser_term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => Ok(ser_value(term)?.to_owned()),
        other => Err(serialize_err(format!(
            "a predicate must be an IRI, got {other:?}"
        ))),
    }
}

/// The `(value, datatype, language)` triplet for an object term.
fn object_fields(graph: &SerGraph, tid: usize) -> Result<(String, String, String), RdfDiagnostic> {
    let term = ser_term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => Ok((
            ser_value(term)?.to_owned(),
            GLOBAL_ID.to_owned(),
            String::new(),
        )),
        SerTermKind::Bnode => Ok((
            format!("_:{}", ser_value(term)?),
            LOCAL_ID.to_owned(),
            String::new(),
        )),
        SerTermKind::Literal => {
            let value = ser_value(term)?.to_owned();
            if let Some(language) = &term.lang {
                Ok((value, RDF_LANG_STRING.to_owned(), language.clone()))
            } else if let Some(datatype) = term.datatype {
                let datatype_iri = ser_value(ser_term(graph, datatype)?)?.to_owned();
                Ok((value, datatype_iri, String::new()))
            } else {
                Ok((value, XSD_STRING.to_owned(), String::new()))
            }
        }
        SerTermKind::Triple => Err(serialize_err(
            "cannot serialize an RDF-1.2 triple term (no triple-term surface)",
        )),
    }
}

fn ser_term(graph: &SerGraph, tid: usize) -> Result<&SerTerm, RdfDiagnostic> {
    graph
        .terms
        .get(tid)
        .ok_or_else(|| serialize_err(format!("term id {tid} is out of range")))
}

fn ser_value(term: &SerTerm) -> Result<&str, RdfDiagnostic> {
    term.value
        .as_deref()
        .ok_or_else(|| serialize_err("term is missing its value"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_codecs::{parse_dataset, serialize_dataset};
    use crate::{SerializeGraph, datasets_isomorphic};

    fn round_trip_isomorphic(nq: &str) {
        let ds = parse_dataset(nq.as_bytes(), "application/n-quads", None).expect("parse nq");
        let hext = serialize_dataset(&ds, "application/x-hextuples", SerializeGraph::Dataset)
            .expect("serialize hext");
        let reparsed =
            parse_dataset(&hext, "application/x-hextuples", None).expect("re-parse hext");
        assert!(
            datasets_isomorphic(&ds, &reparsed),
            "HexTuples round-trip must be isomorphic; produced:\n{}",
            String::from_utf8_lossy(&hext)
        );
    }

    #[test]
    fn iri_bnode_literal_round_trip() {
        round_trip_isomorphic(concat!(
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n",
            "<https://example.org/s> <https://example.org/lit> \"plain\" .\n",
            "<https://example.org/s> <https://example.org/typed> ",
            "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
            "<https://example.org/s> <https://example.org/lang> \"hi\"@en .\n",
            "_:b0 <https://example.org/p> \"v\" .\n",
        ));
    }

    #[test]
    fn named_graph_round_trip() {
        round_trip_isomorphic(concat!(
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n",
            "<https://example.org/s2> <https://example.org/p> <https://example.org/o2> ",
            "<https://example.org/g> .\n",
        ));
    }

    #[test]
    fn each_line_is_a_six_element_json_array() {
        let nq = "<https://example.org/s> <https://example.org/p> \"v\"@en .\n";
        let ds = parse_dataset(nq.as_bytes(), "application/n-quads", None).expect("parse");
        let bytes =
            serialize_dataset(&ds, "application/x-hextuples", SerializeGraph::Dataset).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let line = text.lines().next().expect("one line");
        let fields: Vec<String> = serde_json::from_str(line).expect("json array");
        assert_eq!(fields.len(), 6);
        assert_eq!(fields[2], "v");
        assert_eq!(fields[3], RDF_LANG_STRING);
        assert_eq!(fields[4], "en");
        assert_eq!(fields[5], "");
    }

    #[test]
    fn output_is_deterministic() {
        let nq = concat!(
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> ",
            "<https://example.org/g> .\n",
            "<https://example.org/a> <https://example.org/b> \"c\" .\n",
        );
        let ds = parse_dataset(nq.as_bytes(), "application/n-quads", None).expect("parse");
        let first =
            serialize_dataset(&ds, "application/x-hextuples", SerializeGraph::Dataset).unwrap();
        let second =
            serialize_dataset(&ds, "application/x-hextuples", SerializeGraph::Dataset).unwrap();
        assert_eq!(
            first, second,
            "HexTuples emission must be byte-deterministic"
        );
    }
}
