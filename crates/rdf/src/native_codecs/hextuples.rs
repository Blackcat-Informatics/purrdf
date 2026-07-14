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
use crate::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, TermId};

/// The HexTuples codec: a standalone (non-line-family) [`RdfCodec`] over the
/// line-oriented NDJSON quads syntax. A classic quad syntax with no RDF-1.2 triple-term
/// surface, so it is star-INcapable, and its NDJSON parser carries no span-recording
/// tokenizer.
pub(super) struct HexTuplesCodec;

impl RdfCodec for HexTuplesCodec {
    fn parse(
        &self,
        text: &str,
        _base_iri: Option<&str>,
        _mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        super::parse::catch_codec_panic(NativeRdfFormat::HexTuples, || {
            parse_hextuples_to_dataset(text)
        })
    }

    fn serialize(&self, graph: &SerGraph) -> Result<String, RdfDiagnostic> {
        serialize_ser_graph_to_hextuples(graph)
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

/// Parse HexTuples `text` into a frozen [`RdfDataset`].
pub(super) fn parse_hextuples_to_dataset(text: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut rows: Vec<(HexTerm, String, HexTerm, Option<HexTerm>)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<String> = serde_json::from_str(line)
            .map_err(|e| parse_err(format!("line {}: invalid JSON array: {e}", index + 1)))?;
        if fields.len() != 6 {
            return Err(parse_err(format!(
                "line {}: expected 6 fields, found {}",
                index + 1,
                fields.len()
            )));
        }
        let [subject, predicate, value, datatype, language, graph] =
            <[String; 6]>::try_from(fields)
                .map_err(|_| parse_err("internal: field count mismatch"))?;
        let subject = node_term(&subject)?;
        validate_iri(&predicate)?;
        let object = object_term(&value, &datatype, &language)?;
        let graph = if graph.is_empty() {
            None
        } else {
            Some(node_term(&graph)?)
        };
        rows.push((subject, predicate, object, graph));
    }
    freeze_rows(rows)
}

/// A subject / graph node: an IRI or a `_:`-prefixed blank node.
fn node_term(value: &str) -> Result<HexTerm, RdfDiagnostic> {
    if let Some(label) = value.strip_prefix("_:") {
        validate_blank_label(label)?;
        Ok(HexTerm::Blank(label.to_owned()))
    } else {
        validate_iri(value)?;
        Ok(HexTerm::Iri(value.to_owned()))
    }
}

/// The object term, keyed by the `datatype` sentinel / IRI and the language field.
fn object_term(value: &str, datatype: &str, language: &str) -> Result<HexTerm, RdfDiagnostic> {
    match datatype {
        GLOBAL_ID => {
            validate_iri(value)?;
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
            validate_iri(datatype)?;
            Ok(HexTerm::Literal(RdfLiteral::typed(value, datatype)))
        }
    }
}

fn freeze_rows(
    rows: Vec<(HexTerm, String, HexTerm, Option<HexTerm>)>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
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
        HexTerm::Blank(label) => builder.intern_blank(label, BlankScope::DEFAULT),
        HexTerm::Literal(literal) => builder.intern_literal(literal.clone()),
    }
}

fn validate_iri(value: &str) -> Result<(), RdfDiagnostic> {
    if value.is_empty()
        || !value.contains(':')
        || value
            .chars()
            .any(|ch| ch.is_ascii_control() || ch.is_ascii_whitespace() || ch == '<' || ch == '>')
    {
        return Err(parse_err(format!("invalid IRI {value:?}")));
    }
    Ok(())
}

fn validate_blank_label(label: &str) -> Result<(), RdfDiagnostic> {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return Err(parse_err("empty blank-node identifier"));
    };
    if !first.is_ascii_alphanumeric() && first != '_' {
        return Err(parse_err(format!(
            "invalid blank-node identifier {label:?}"
        )));
    }
    let mut last = first;
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' && ch != '.' {
            return Err(parse_err(format!(
                "invalid blank-node identifier {label:?}"
            )));
        }
        last = ch;
    }
    if last == '.' {
        return Err(parse_err(format!(
            "invalid blank-node identifier {label:?}"
        )));
    }
    Ok(())
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
