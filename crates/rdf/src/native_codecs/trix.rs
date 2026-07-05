// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party TriX codec ("Triples in XML", W3C member submission).
//!
//! TriX is a quads/named-graph RDF serialization in XML: a `<TriX>` root holds one or
//! more `<graph>` elements, each optionally named by a leading `<uri>`/`<id>` and
//! carrying `<triple>` elements of three term children. The term vocabulary is `<uri>`
//! (IRI), `<id>` (blank node), `<plainLiteral>` (plain / language-tagged), and
//! `<typedLiteral datatype="…">` (typed).
//!
//! Like [`rdfxml`](super::rdfxml), the reader runs on the pure-Rust XML DOM
//! (`roxmltree`, already a dep) and the writer hand-rolls deterministic XML string
//! emission (stable graph/term order, canonical escaping) — no new dependency, so the
//! crate stays wasm-clean. TriX is a CLASSIC quad syntax with no RDF-1.2 triple-term
//! surface: a triple term in a serialize request is a HARD error rather than silent
//! loss.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use roxmltree::{Document, Node};

use super::codec::RdfCodec;
use super::media_type::NativeRdfFormat;
use super::parse::{FoldNode, FoldRow, RDF_REIFIES, fold_statement_layer};
use super::ser_model::{SerGraph, SerTerm, SerTermKind};
use super::text_parse::LineParseMode;
use crate::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, TermId};

/// The TriX codec: a standalone (non-line-family) [`RdfCodec`] over the "Triples in XML"
/// quads syntax. A classic quad syntax with no RDF-1.2 triple-term surface, so it is
/// star-INcapable, and its XML-DOM parser carries no span-recording tokenizer.
pub(super) struct TriXCodec;

impl RdfCodec for TriXCodec {
    fn carries_star(&self) -> bool {
        false
    }

    fn tokenizer_carries_spans(&self) -> bool {
        false
    }

    fn parse(
        &self,
        text: &str,
        _base_iri: Option<&str>,
        _mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        super::parse::catch_codec_panic(NativeRdfFormat::TriX, || parse_trix_to_dataset(text))
    }

    fn serialize(&self, graph: &SerGraph) -> Result<String, RdfDiagnostic> {
        serialize_ser_graph_to_trix(graph)
    }
}

/// The TriX namespace (W3C member submission `trix-1`).
const TRIX_NS: &str = "http://www.w3.org/2004/03/trix/trix-1/";

fn parse_err(detail: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("native-codec-parse", format!("TriX: {}", detail.into()))
}

fn serialize_err(detail: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("native-codec-serialize", format!("TriX: {}", detail.into()))
}

// ───────────────────────────────────────────────────────────────────────────────
// Parse: TriX XML → frozen RdfDataset IR (via the shared statement-layer fold)
// ───────────────────────────────────────────────────────────────────────────────

/// A first-party TriX term the parser accumulates before interning into the IR. TriX is
/// classic (no triple terms), so subject/object never carry a quoted triple.
#[derive(Clone, Debug)]
enum TrixTerm {
    Iri(String),
    Blank(String),
    Literal(RdfLiteral),
}

/// Parse TriX `text` into a frozen [`RdfDataset`].
pub(super) fn parse_trix_to_dataset(text: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let document = Document::parse(text).map_err(|e| parse_err(e.to_string()))?;
    let root = document.root_element();
    if !is_trix(root, "TriX") {
        return Err(parse_err("document root is not a <TriX> element"));
    }

    // Accumulate (subject, predicate, object, graph) rows, then intern + fold once.
    let mut rows: Vec<(TrixTerm, String, TrixTerm, Option<TrixTerm>)> = Vec::new();
    for graph in element_children(root) {
        if !is_trix(graph, "graph") {
            return Err(parse_err(format!(
                "unexpected element <{}> under <TriX>",
                graph.tag_name().name()
            )));
        }
        let mut graph_name: Option<TrixTerm> = None;
        let mut seen_triple = false;
        for child in element_children(graph) {
            if is_trix(child, "triple") {
                seen_triple = true;
                let (subject, predicate, object) = parse_triple(child)?;
                rows.push((subject, predicate, object, graph_name.clone()));
            } else if matches!(local_of(child), Some("uri" | "id")) {
                if seen_triple || graph_name.is_some() {
                    return Err(parse_err(
                        "a <graph> name (<uri>/<id>) must precede its <triple> elements",
                    ));
                }
                graph_name = Some(node_term(child)?);
            } else {
                return Err(parse_err(format!(
                    "unexpected element <{}> under <graph>",
                    child.tag_name().name()
                )));
            }
        }
    }

    freeze_rows(rows)
}

/// Parse a `<triple>` element's three term children.
fn parse_triple(element: Node<'_, '_>) -> Result<(TrixTerm, String, TrixTerm), RdfDiagnostic> {
    let terms: Vec<Node<'_, '_>> = element_children(element).collect();
    if terms.len() != 3 {
        return Err(parse_err(format!(
            "<triple> must have exactly three term children, found {}",
            terms.len()
        )));
    }
    let subject = node_term(terms[0])?;
    let TrixTerm::Iri(predicate) = term_element(terms[1])? else {
        return Err(parse_err("a predicate must be a <uri>"));
    };
    let object = term_element(terms[2])?;
    Ok((subject, predicate, object))
}

/// A subject / graph-name node term: only `<uri>` or `<id>` are valid here.
fn node_term(element: Node<'_, '_>) -> Result<TrixTerm, RdfDiagnostic> {
    match term_element(element)? {
        term @ (TrixTerm::Iri(_) | TrixTerm::Blank(_)) => Ok(term),
        TrixTerm::Literal(_) => Err(parse_err("a subject or graph name must be a <uri> or <id>")),
    }
}

/// Map a TriX term element to a [`TrixTerm`].
fn term_element(element: Node<'_, '_>) -> Result<TrixTerm, RdfDiagnostic> {
    match local_of(element) {
        Some("uri") => {
            let iri = trimmed_text(element);
            validate_iri(&iri)?;
            Ok(TrixTerm::Iri(iri))
        }
        Some("id") => {
            let label = trimmed_text(element);
            validate_blank_label(&label)?;
            Ok(TrixTerm::Blank(label))
        }
        Some("plainLiteral") => {
            let lexical = element_text(element);
            match attr_xml_lang(element) {
                Some(lang) if !lang.is_empty() => Ok(TrixTerm::Literal(RdfLiteral {
                    lexical_form: lexical,
                    datatype: None,
                    language: Some(lang.to_owned()),
                    direction: None,
                })),
                _ => Ok(TrixTerm::Literal(RdfLiteral::simple(lexical))),
            }
        }
        Some("typedLiteral") => {
            let datatype = attr_local(element, "datatype")
                .ok_or_else(|| parse_err("<typedLiteral> requires a datatype attribute"))?;
            validate_iri(datatype)?;
            Ok(TrixTerm::Literal(RdfLiteral::typed(
                element_text(element),
                datatype,
            )))
        }
        other => Err(parse_err(format!(
            "unexpected term element <{}>",
            other.unwrap_or("?")
        ))),
    }
}

/// Intern the accumulated rows into a fresh builder, fold the statement layer, freeze.
fn freeze_rows(
    rows: Vec<(TrixTerm, String, TrixTerm, Option<TrixTerm>)>,
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

fn intern_term(builder: &mut RdfDatasetBuilder, term: &TrixTerm) -> TermId {
    match term {
        TrixTerm::Iri(iri) => builder.intern_iri(iri),
        TrixTerm::Blank(label) => builder.intern_blank(label, BlankScope::DEFAULT),
        TrixTerm::Literal(literal) => builder.intern_literal(literal.clone()),
    }
}

// ── roxmltree helpers ───────────────────────────────────────────────────────────

/// Whether `element` is the TriX-namespace element `local`. A namespace-less document
/// (no `xmlns`) is accepted leniently by matching on the local name alone.
fn is_trix(element: Node<'_, '_>, local: &str) -> bool {
    element.tag_name().name() == local
        && matches!(element.tag_name().namespace(), None | Some(TRIX_NS))
}

fn local_of<'a>(element: Node<'a, '_>) -> Option<&'a str> {
    element
        .is_element()
        .then(|| element.tag_name().name())
        .filter(|_| matches!(element.tag_name().namespace(), None | Some(TRIX_NS)))
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(Node::is_element)
}

/// Concatenated direct text of an element (the literal lexical form, verbatim).
fn element_text(element: Node<'_, '_>) -> String {
    element
        .children()
        .filter(Node::is_text)
        .filter_map(|n| n.text())
        .collect()
}

fn trimmed_text(element: Node<'_, '_>) -> String {
    element_text(element).trim().to_owned()
}

fn attr_local<'a>(element: Node<'a, '_>, local: &str) -> Option<&'a str> {
    element
        .attributes()
        .find(|attr| attr.name() == local && attr.namespace().is_none())
        .map(|attr| attr.value())
}

fn attr_xml_lang<'a>(element: Node<'a, '_>) -> Option<&'a str> {
    element
        .attributes()
        .find(|attr| {
            attr.name() == "lang"
                && attr.namespace() == Some("http://www.w3.org/XML/1998/namespace")
        })
        .map(|attr| attr.value())
}

/// Minimal syntactic IRI validation (mirrors the `rdfxml` codec's contract).
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

/// Blank-node label contract (mirrors the `rdfxml` codec's contract).
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
// Serialize: SerGraph → TriX XML text (deterministic)
// ───────────────────────────────────────────────────────────────────────────────

/// Serialize a [`SerGraph`] to TriX XML text.
///
/// Quads are grouped into `<graph>` elements by their graph slot (default graph first,
/// then named graphs in first-appearance order) so the emission is deterministic.
/// Annotation rows are emitted as plain triples in the default graph. A quoted-triple
/// (RDF-1.2) term is a HARD error — TriX has no triple-term surface.
pub(super) fn serialize_ser_graph_to_trix(graph: &SerGraph) -> Result<String, RdfDiagnostic> {
    // Group triples by graph slot, preserving first-appearance order.
    let mut order: Vec<Option<usize>> = Vec::new();
    let mut groups: HashMap<Option<usize>, Vec<(usize, usize, usize)>> = HashMap::new();
    // A real reifier binding (`rid rdf:reifies <<triple>>`) is unrepresentable in TriX;
    // a self-reifier sentinel is an inline quoted-triple term already carried by its
    // parent quad, so it is skipped.
    for &(rid, _, _) in &graph.reifiers {
        if !is_self_reifier(graph, rid) {
            return Err(serialize_err(
                "cannot serialize an RDF-1.2 reifier binding (no triple-term surface)",
            ));
        }
    }
    let rows = graph
        .quads
        .iter()
        .map(|&(s, p, o, g)| (g, (s, p, o)))
        .chain(graph.annotations.iter().map(|&(r, p, v, g)| (g, (r, p, v))));
    for (slot, triple) in rows {
        if !groups.contains_key(&slot) {
            order.push(slot);
        }
        groups.entry(slot).or_default().push(triple);
    }

    // Ensure the default graph sorts before named graphs when both are present.
    order.sort_by_key(|slot| (slot.is_some(), *slot));

    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\">\n",
    );
    for slot in order {
        out.push_str("  <graph>\n");
        if let Some(gid) = slot {
            write_graph_name(&mut out, graph, gid)?;
        }
        for (s, p, o) in groups.remove(&slot).unwrap_or_default() {
            out.push_str("    <triple>\n");
            write_term(&mut out, graph, s)?;
            write_term(&mut out, graph, p)?;
            write_term(&mut out, graph, o)?;
            out.push_str("    </triple>\n");
        }
        out.push_str("  </graph>\n");
    }
    out.push_str("</TriX>\n");
    Ok(out)
}

fn is_self_reifier(graph: &SerGraph, rid: usize) -> bool {
    graph
        .terms
        .get(rid)
        .is_some_and(|t| t.kind == SerTermKind::Triple && t.reifier == Some(rid))
}

/// Write a graph-name element (`<uri>` / `<id>`).
fn write_graph_name(out: &mut String, graph: &SerGraph, tid: usize) -> Result<(), RdfDiagnostic> {
    let term = ser_term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => {
            let _ = writeln!(out, "    <uri>{}</uri>", escape_text(ser_value(term)?));
        }
        SerTermKind::Bnode => {
            let _ = writeln!(out, "    <id>{}</id>", escape_text(ser_value(term)?));
        }
        other => {
            return Err(serialize_err(format!(
                "a graph name must be an IRI or blank node, got {other:?}"
            )));
        }
    }
    Ok(())
}

/// Write a single term as a `<uri>` / `<id>` / `<plainLiteral>` / `<typedLiteral>`.
fn write_term(out: &mut String, graph: &SerGraph, tid: usize) -> Result<(), RdfDiagnostic> {
    let term = ser_term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => {
            let _ = writeln!(out, "      <uri>{}</uri>", escape_text(ser_value(term)?));
        }
        SerTermKind::Bnode => {
            let _ = writeln!(out, "      <id>{}</id>", escape_text(ser_value(term)?));
        }
        SerTermKind::Literal => write_literal(out, graph, term)?,
        SerTermKind::Triple => {
            return Err(serialize_err(
                "cannot serialize an RDF-1.2 triple term (no triple-term surface)",
            ));
        }
    }
    Ok(())
}

fn write_literal(out: &mut String, graph: &SerGraph, term: &SerTerm) -> Result<(), RdfDiagnostic> {
    let lexical = escape_text(ser_value(term)?);
    if let Some(language) = &term.lang {
        let _ = writeln!(
            out,
            "      <plainLiteral xml:lang=\"{}\">{lexical}</plainLiteral>",
            escape_attr(language)
        );
    } else if let Some(datatype) = term.datatype {
        let datatype_iri = ser_value(ser_term(graph, datatype)?)?;
        let _ = writeln!(
            out,
            "      <typedLiteral datatype=\"{}\">{lexical}</typedLiteral>",
            escape_attr(datatype_iri)
        );
    } else {
        let _ = writeln!(out, "      <plainLiteral>{lexical}</plainLiteral>");
    }
    Ok(())
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

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use crate::native_codecs::{parse_dataset, serialize_dataset};
    use crate::{SerializeGraph, datasets_isomorphic};

    fn round_trip_isomorphic(nq: &str) {
        let ds = parse_dataset(nq.as_bytes(), "application/n-quads", None).expect("parse nq");
        let trix = serialize_dataset(&ds, "application/trix", SerializeGraph::Dataset)
            .expect("serialize trix");
        let reparsed = parse_dataset(&trix, "application/trix", None).expect("re-parse trix");
        assert!(
            datasets_isomorphic(&ds, &reparsed),
            "TriX round-trip must be isomorphic; produced:\n{}",
            String::from_utf8_lossy(&trix)
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
    fn output_is_deterministic() {
        let nq = concat!(
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> ",
            "<https://example.org/g> .\n",
            "<https://example.org/a> <https://example.org/b> \"c\" .\n",
        );
        let ds = parse_dataset(nq.as_bytes(), "application/n-quads", None).expect("parse");
        let first = serialize_dataset(&ds, "application/trix", SerializeGraph::Dataset).unwrap();
        let second = serialize_dataset(&ds, "application/trix", SerializeGraph::Dataset).unwrap();
        assert_eq!(first, second, "TriX emission must be byte-deterministic");
        assert!(String::from_utf8_lossy(&first).contains("<TriX"));
    }

    #[test]
    fn special_characters_escape_and_round_trip() {
        round_trip_isomorphic(concat!(
            "<https://example.org/s> <https://example.org/p> ",
            "\"a & b < c > d\" .\n",
        ));
    }
}
