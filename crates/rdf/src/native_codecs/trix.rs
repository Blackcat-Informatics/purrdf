// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! First-party TriX codec ("Triples in XML", W3C member submission).
//!
//! TriX is a quads/named-graph RDF serialization in XML: a `<TriX>` root holds one or
//! more `<graph>` elements, each optionally named by a leading `<uri>`/`<id>` and
//! carrying `<triple>` elements of three term children. The term vocabulary is `<uri>`
//! (IRI), `<id>` (blank node), `<plainLiteral>` (plain / language-tagged), and
//! `<typedLiteral datatype="…">` (typed).
//!
//! Like [`rdfxml`](super::rdfxml), the reader runs on the workspace's one XML reader
//! ([`purrdf_lex::xml`]) and the writer hand-rolls deterministic XML string
//! emission (stable graph/term order, canonical escaping) — no new dependency, so the
//! crate stays wasm-clean. TriX is a CLASSIC quad syntax with no RDF-1.2 triple-term
//! surface: a triple term in a serialize request is a HARD error rather than silent
//! loss.

use super::syntax::{
    ClassicTerm, CodecName, check_language_tag, element_children, element_text, freeze_classic_rows,
};
use purrdf_core::sink::{TextOut, TextSink};
use purrdf_core::xml_escape::Context;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use purrdf_lex::xml::Node;

use super::codec::RdfCodec;
use super::media_type::NativeRdfFormat;
use super::ser_model::{SerGraph, SerTerm, SerTermKind};
use super::text_parse::LineParseMode;
use crate::nesting::{XmlReadError, parse_xml};
use crate::{RdfDataset, RdfDiagnostic, RdfLiteral};
use purrdf_core::blank_label::{LabelAlphabet, is_valid_label};

/// This codec's name, which its diagnostics lead with.
const TRIX: CodecName = CodecName("TriX");

/// The TriX codec: a standalone (non-line-family) [`RdfCodec`] over the "Triples in XML"
/// quads syntax. A classic quad syntax with no RDF-1.2 triple-term surface, so it is
/// star-INcapable, and its XML-DOM parser carries no span-recording tokenizer.
pub(super) struct TriXCodec;

impl RdfCodec for TriXCodec {
    fn parse(
        &self,
        text: &str,
        // TriX has no base directive, so the scope is left EXACTLY as handed in: the base
        // in force at the end of a TriX document is the caller's, and this codec says so
        // by touching nothing.
        base: &mut purrdf_iri::BaseScope,
        _mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        super::parse::catch_codec_panic(NativeRdfFormat::TriX, || parse_trix_to_dataset(text, base))
    }

    fn serialize_into(
        &self,
        graph: &SerGraph,
        out: &mut TextSink<'_>,
    ) -> Result<(), RdfDiagnostic> {
        // Emitted element by element after a grouping pre-pass. TriX's own shape is
        // flat — `<graph>` blocks of `<triple>` elements — so the writer never needs
        // to know what follows; what it needs first is which rows share a graph slot,
        // and that index holds row IDENTIFIERS, not document text. Removing the output
        // buffer is therefore independent of it: the index stays, the document does
        // not accumulate.
        write_trix(graph, out)
    }
}

/// The TriX namespace (W3C member submission `trix-1`).
const TRIX_NS: &str = "http://www.w3.org/2004/03/trix/trix-1/";

// ───────────────────────────────────────────────────────────────────────────────
// Parse: TriX XML → frozen RdfDataset IR (via the shared statement-layer fold)
// ───────────────────────────────────────────────────────────────────────────────

/// Parse TriX `text` into a frozen [`RdfDataset`].
pub(super) fn parse_trix_to_dataset(
    text: &str,
    base: &purrdf_iri::BaseScope,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let document = parse_xml(text).map_err(|error| match error {
        XmlReadError::TooDeep(depth) => TRIX.parse_err(format!(
            "element nesting reaches {depth} levels, past the parser limit"
        )),
        XmlReadError::Malformed(error) => TRIX.parse_err(error.to_string()),
    })?;
    let root = document.root_element();
    if !is_trix(root, "TriX") {
        return Err(TRIX.parse_err("document root is not a <TriX> element"));
    }

    // Accumulate (subject, predicate, object, graph) rows, then intern + fold once.
    let mut rows: Vec<(ClassicTerm, String, ClassicTerm, Option<ClassicTerm>)> = Vec::new();
    for graph in element_children(root) {
        if !is_trix(graph, "graph") {
            return Err(TRIX.parse_err(format!(
                "unexpected element <{}> under <TriX>",
                graph.tag_name().name()
            )));
        }
        let mut graph_name: Option<ClassicTerm> = None;
        let mut seen_triple = false;
        for child in element_children(graph) {
            if is_trix(child, "triple") {
                seen_triple = true;
                let (subject, predicate, object) = parse_triple(child, base)?;
                rows.push((subject, predicate, object, graph_name.clone()));
            } else if matches!(local_of(child), Some("uri" | "id")) {
                if seen_triple || graph_name.is_some() {
                    return Err(TRIX.parse_err(
                        "a <graph> name (<uri>/<id>) must precede its <triple> elements",
                    ));
                }
                graph_name = Some(node_term(child, base)?);
            } else {
                return Err(TRIX.parse_err(format!(
                    "unexpected element <{}> under <graph>",
                    child.tag_name().name()
                )));
            }
        }
    }

    freeze_classic_rows(rows, LabelAlphabet::XmlText)
}

/// Parse a `<triple>` element's three term children.
fn parse_triple(
    element: Node<'_, '_>,
    base: &purrdf_iri::BaseScope,
) -> Result<(ClassicTerm, String, ClassicTerm), RdfDiagnostic> {
    let terms: Vec<Node<'_, '_>> = element_children(element).collect();
    if terms.len() != 3 {
        return Err(TRIX.parse_err(format!(
            "<triple> must have exactly three term children, found {}",
            terms.len()
        )));
    }
    let subject = node_term(terms[0], base)?;
    let ClassicTerm::Iri(predicate) = term_element(terms[1], base)? else {
        return Err(TRIX.parse_err("a predicate must be a <uri>"));
    };
    let object = term_element(terms[2], base)?;
    Ok((subject, predicate, object))
}

/// A subject / graph-name node term: only `<uri>` or `<id>` are valid here.
fn node_term(
    element: Node<'_, '_>,
    base: &purrdf_iri::BaseScope,
) -> Result<ClassicTerm, RdfDiagnostic> {
    match term_element(element, base)? {
        term @ (ClassicTerm::Iri(_) | ClassicTerm::Blank(_)) => Ok(term),
        ClassicTerm::Literal(_) => {
            Err(TRIX.parse_err("a subject or graph name must be a <uri> or <id>"))
        }
    }
}

/// Map a TriX term element to a [`ClassicTerm`].
fn term_element(
    element: Node<'_, '_>,
    base: &purrdf_iri::BaseScope,
) -> Result<ClassicTerm, RdfDiagnostic> {
    match local_of(element) {
        Some("uri") => {
            let iri = trimmed_text(element);
            validate_iri(&iri, base)?;
            Ok(ClassicTerm::Iri(iri))
        }
        Some("id") => {
            let label = trimmed_text(element);
            validate_blank_label(&label)?;
            Ok(ClassicTerm::Blank(label))
        }
        Some("plainLiteral") => {
            let lexical = element_text(element);
            match attr_xml_lang(element) {
                // The empty-attribute guard stays where it was: `xml:lang=""`
                // means "no language in scope" in XML, not "a language that is
                // the empty string", and still yields a plain literal.
                // Validating it would refuse documents that carry no language.
                Some(lang) if !lang.is_empty() => {
                    validate_language_tag(lang)?;
                    Ok(ClassicTerm::Literal(RdfLiteral {
                        lexical_form: lexical,
                        datatype: None,
                        language: Some(lang.to_owned()),
                        direction: None,
                    }))
                }
                _ => Ok(ClassicTerm::Literal(RdfLiteral::simple(lexical))),
            }
        }
        Some("typedLiteral") => {
            let datatype = attr_local(element, "datatype")
                .ok_or_else(|| TRIX.parse_err("<typedLiteral> requires a datatype attribute"))?;
            validate_iri(datatype, base)?;
            Ok(ClassicTerm::Literal(RdfLiteral::typed(
                element_text(element),
                datatype,
            )))
        }
        other => Err(TRIX.parse_err(format!(
            "unexpected term element <{}>",
            other.unwrap_or("?")
        ))),
    }
}

// ── XML tree helpers ───────────────────────────────────────────────────────────

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

/// `text` with its leading and trailing runs of XML whitespace removed.
///
/// > `S ::= (#x20 | #x9 | #xD | #xA)+`
///
/// — XML 1.0 (Fifth Edition) §2.3 *Common Syntactic Constructs*. Four code points, and the
/// same four Turtle's `WS` and JSON's `ws` name, so the single
/// [`purrdf_iri::terminals::is_ws`] transcription answers this production too.
///
/// Deliberately NOT [`str::trim`]. `str::trim` is defined over [`char::is_whitespace`],
/// the Unicode `White_Space` property — twenty-six scalars — and here that is not a
/// liberality, it is a SILENT REWRITE of the caller's data: a `<uri>` whose text carries
/// a leading or trailing U+00A0 NO-BREAK SPACE is a LAWFUL IRI (U+00A0 is `ucschar`, and
/// `IRIREF` excludes only `#x00-#x20` and nine delimiters), so trimming it produced a
/// DIFFERENT IRI than the document spelled, with no diagnostic and no loss entry. The
/// same applies to an `<id>` blank node label. The frozen W3C RDFC-1.0 corpus contains
/// exactly this term — `<urn:ex:\u{a0}>` — so it is not a hypothetical shape.
///
/// The bytes the XML layer has already normalized are still handled: XML line-end
/// normalization has turned every `#xD#xA` and lone `#xD` in the text into `#xA` before
/// the reader hands it over, and `#xA` is `S`, so pretty-printed indentation around a
/// `<uri>` is removed exactly as it always was.
fn trim_xml_s(text: &str) -> &str {
    purrdf_iri::terminals::trim_ws(text)
}

/// The direct text of `element` with its surrounding XML `S` removed — see
/// [`trim_xml_s`] for why that is not [`str::trim`].
fn trimmed_text(element: Node<'_, '_>) -> String {
    trim_xml_s(&element_text(element)).to_owned()
}

fn attr_local<'a>(element: Node<'a, '_>, local: &str) -> Option<&'a str> {
    element
        .attributes()
        .iter()
        .find(|attr| attr.name() == local && attr.namespace().is_none())
        .map(purrdf_lex::xml::Attribute::value)
}

fn attr_xml_lang<'a>(element: Node<'a, '_>) -> Option<&'a str> {
    element
        .attributes()
        .iter()
        .find(|attr| attr.name() == "lang" && attr.namespace() == Some(purrdf_iri::vocab::xml::NS))
        .map(purrdf_lex::xml::Attribute::value)
}

/// The `xml:lang` contract on `<plainLiteral>`: the concrete syntaxes' `LANGTAG`
/// terminal under the RFC 5646 §2.1 eight-character subtag ceiling, decided by
/// [`purrdf_iri::langtag`].
///
/// There was NO contract here before. A non-empty `xml:lang` was moved into the
/// literal unexamined, so this reader admitted `1`, `9-9`, `123-456`, `en-`, `-`
/// and `!!!` — and `en us`, whose embedded space is not expressible in `LANGTAG`
/// at all, which converted to N-Quads with exit 0 and produced a line no parser
/// can read back.
///
/// The profile is [`langtag::Profile::ConcreteSyntaxLangtagBounded`], the one
/// acceptance language every codec in this crate names — `text_parse`'s two
/// parsers, `rdfxml`, `jsonld`'s expander and the term projection in
/// `projections::term` all name the same one. It has to be shared, and TriX
/// makes the point twice over: its own writer emits a dataset's `@lang` into
/// `xml:lang` verbatim, so a tag this reader refuses but another accepts is a
/// TriX file this codec writes and cannot read back.
///
/// The failure reports the module's
/// [`langtag::LanguageTagError::diagnostic_code`], so the user learns which
/// production refused. TriX parse diagnostics carry no line/column — the
/// XML tree this walks is not a span-recording tokenizer, as this
/// module's header says — which this does not change.
fn validate_language_tag(language: &str) -> Result<(), RdfDiagnostic> {
    check_language_tag(language, |error| {
        format!("TriX: invalid language tag {language:?}: {error}")
    })
}

/// Validate a `<uri>` against the shared IRI layer.
///
/// TriX's row in `FORMATS` sets `admits_relative_iri: false`, so this routes through
/// [`BaseScope::resolve_absolute_only`](purrdf_iri::BaseScope::resolve_absolute_only):
/// a relative reference reports `iri-not-absolute-by-grammar` — the code that says
/// "supplying a base will not help" — and no base is ever applied, because none may be.
///
/// This replaces a hand-rolled check that tested only for the PRESENCE of a `:`, which
/// admitted a `path-noscheme` reference whose first segment merely contained one (RFC-3986
/// §4.2) as though it were absolute, and reported everything else as a generic parse
/// error. That is the same defect the RDF/XML codec carried, deleted the same way.
///
/// The scope handed in is the CALLER'S, not a locally minted empty one. It is still never
/// applied — `resolve_absolute_only` refuses a relative reference whatever is in scope —
/// but the refusal can now name the base in scope and say it is deliberately not applied
/// here. The empty stand-in could only say "no base IRI is in scope", which was false for
/// anyone who had passed `--base`.
fn validate_iri(value: &str, base: &purrdf_iri::BaseScope) -> Result<(), RdfDiagnostic> {
    base.resolve_absolute_only(value)
        .map(|_| ())
        .map_err(|error| RdfDiagnostic::error(error.diagnostic_code(), format!("TriX: {error}")))
}

/// Blank-node label contract for `<id>` element text: the same
/// [`LabelAlphabet::XmlText`] alphabet this codec EMITS, so every document the
/// TriX serializer writes re-parses here (ingress and egress agree on one
/// alphabet per syntax).
fn validate_blank_label(label: &str) -> Result<(), RdfDiagnostic> {
    if is_valid_label(label, LabelAlphabet::XmlText) {
        Ok(())
    } else {
        Err(TRIX.parse_err(format!("invalid blank-node identifier {label:?}")))
    }
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
fn write_trix<W: TextOut + ?Sized>(graph: &SerGraph, out: &mut W) -> Result<(), RdfDiagnostic> {
    // Group triples by graph slot, preserving first-appearance order.
    let mut order: Vec<Option<usize>> = Vec::new();
    let mut groups: crate::FastMap<Option<usize>, Vec<(usize, usize, usize)>> =
        crate::FastMap::default();
    // A real reifier binding (`rid rdf:reifies <<triple>>`) is unrepresentable in TriX;
    // a self-reifier sentinel is an inline quoted-triple term already carried by its
    // parent quad, so it is skipped.
    for &(rid, _, _) in &graph.reifiers {
        if !graph.is_self_reifier(rid) {
            return Err(TRIX.serialize_err(
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
        // One hash probe per row instead of two (`contains_key` then `entry`); a slot's
        // first appearance still lands in `order` at the same point.
        match groups.entry(slot) {
            Entry::Vacant(vacant) => {
                order.push(slot);
                vacant.insert(vec![triple]);
            }
            Entry::Occupied(mut occupied) => occupied.get_mut().push(triple),
        }
    }

    // Ensure the default graph sorts before named graphs when both are present.
    order.sort_by_key(|slot| (slot.is_some(), *slot));

    out.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\">\n",
    );
    for slot in order {
        if out.failed() {
            return Ok(());
        }
        out.push_str("  <graph>\n");
        if let Some(gid) = slot {
            write_graph_name(out, graph, gid)?;
        }
        for (s, p, o) in groups.remove(&slot).unwrap_or_default() {
            if out.failed() {
                return Ok(());
            }
            out.push_str("    <triple>\n");
            write_term(out, graph, s)?;
            write_term(out, graph, p)?;
            write_term(out, graph, o)?;
            out.push_str("    </triple>\n");
        }
        out.push_str("  </graph>\n");
    }
    out.push_str("</TriX>\n");
    Ok(())
}

/// Write a graph-name element (`<uri>` / `<id>`).
fn write_graph_name<W: TextOut + ?Sized>(
    out: &mut W,
    graph: &SerGraph,
    tid: usize,
) -> Result<(), RdfDiagnostic> {
    let term = TRIX.term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => {
            out.push_str("    <uri>");
            TRIX.push_xml(TRIX.value(term)?, Context::Text, out)?;
            out.push_str("</uri>\n");
        }
        SerTermKind::Bnode => {
            out.push_str("    <id>");
            TRIX.push_xml(TRIX.value(term)?, Context::Text, out)?;
            out.push_str("</id>\n");
        }
        other => {
            return Err(TRIX.serialize_err(format!(
                "a graph name must be an IRI or blank node, got {other:?}"
            )));
        }
    }
    Ok(())
}

/// Write a single term as a `<uri>` / `<id>` / `<plainLiteral>` / `<typedLiteral>`.
fn write_term<W: TextOut + ?Sized>(
    out: &mut W,
    graph: &SerGraph,
    tid: usize,
) -> Result<(), RdfDiagnostic> {
    let term = TRIX.term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => {
            out.push_str("      <uri>");
            TRIX.push_xml(TRIX.value(term)?, Context::Text, out)?;
            out.push_str("</uri>\n");
        }
        SerTermKind::Bnode => {
            out.push_str("      <id>");
            TRIX.push_xml(TRIX.value(term)?, Context::Text, out)?;
            out.push_str("</id>\n");
        }
        SerTermKind::Literal => write_literal(out, graph, term)?,
        SerTermKind::Triple => {
            return Err(TRIX.serialize_err(
                "cannot serialize an RDF-1.2 triple term (no triple-term surface)",
            ));
        }
    }
    Ok(())
}

fn write_literal<W: TextOut + ?Sized>(
    out: &mut W,
    graph: &SerGraph,
    term: &SerTerm,
) -> Result<(), RdfDiagnostic> {
    let lexical = TRIX.value(term)?;
    if let Some(language) = &term.lang {
        out.push_str("      <plainLiteral xml:lang=\"");
        TRIX.push_xml(language, Context::Attribute, out)?;
        out.push_str("\">");
        TRIX.push_xml(lexical, Context::Text, out)?;
        out.push_str("</plainLiteral>\n");
    } else if let Some(datatype) = term.datatype {
        let datatype_iri = TRIX.value(TRIX.term(graph, datatype)?)?;
        out.push_str("      <typedLiteral datatype=\"");
        TRIX.push_xml(datatype_iri, Context::Attribute, out)?;
        out.push_str("\">");
        TRIX.push_xml(lexical, Context::Text, out)?;
        out.push_str("</typedLiteral>\n");
    } else {
        out.push_str("      <plainLiteral>");
        TRIX.push_xml(lexical, Context::Text, out)?;
        out.push_str("</plainLiteral>\n");
    }
    Ok(())
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

    /// A TriX document with `terms` as the three children of its single `<triple>`.
    fn document(terms: &str) -> String {
        format!(
            "<TriX xmlns=\"{}\"><graph><triple>{terms}</triple></graph></TriX>",
            super::TRIX_NS
        )
    }

    /// The subject IRI of a one-triple document.
    fn subject_of(trix: &str) -> String {
        let dataset = parse_dataset(trix.as_bytes(), "application/trix", None)
            .unwrap_or_else(|e| panic!("parse TriX: {e}"));
        let quad = dataset.quads().next().expect("one quad");
        match dataset.term_value(quad.s) {
            crate::TermValue::Iri(iri) => iri,
            other => panic!("subject is not an IRI: {other:?}"),
        }
    }

    /// A U+00A0 at the edge of a `<uri>` is CONTENT: it is a lawful `ucschar`, so
    /// trimming it silently minted a DIFFERENT IRI than the document spelled.
    ///
    /// XML's whitespace is `S ::= (#x20 | #x9 | #xD | #xA)+` (XML 1.0 §2.3) — four code
    /// points — while `str::trim` answers `char::is_whitespace`, twenty-six. The frozen
    /// W3C RDFC-1.0 corpus contains exactly this term, so the rewrite was reachable from
    /// data this repository already ships.
    #[test]
    fn a_no_break_space_at_the_edge_of_a_uri_is_content_not_whitespace() {
        let marked = document("<uri>urn:ex:s\u{a0}</uri><uri>urn:ex:p</uri><uri>urn:ex:o</uri>");
        assert_eq!(
            subject_of(&marked),
            "urn:ex:s\u{a0}",
            "the NO-BREAK SPACE must survive verbatim into the IRI"
        );
        // A leading one too, on a term whose IRI is still absolute.
        let inner = document("<uri>urn:ex:a\u{a0}b</uri><uri>urn:ex:p</uri><uri>urn:ex:o</uri>");
        assert_eq!(subject_of(&inner), "urn:ex:a\u{a0}b");

        // The over-refusal side, executed: real XML `S` around the text — the
        // indentation every pretty-printer emits — is still removed, in all four
        // members, so an ordinary formatted document reads exactly as before.
        for padding in [" ", "\t", "\n", "\r\n", " \t\n  "] {
            let padded = document(&format!(
                "<uri>{padding}urn:ex:s{padding}</uri><uri>urn:ex:p</uri><uri>urn:ex:o</uri>"
            ));
            assert_eq!(
                subject_of(&padded),
                "urn:ex:s",
                "XML `S` padding {padding:?} must still be trimmed"
            );
        }
    }

    /// The same rule for `<id>`: a blank node label is what the document says it is.
    ///
    /// A label carrying a U+00A0 is not a lawful `BLANK_NODE_LABEL`, so this is the
    /// refusal side — but the refusal must come from the LABEL validator saying so, not
    /// from a trim silently rewriting the label into a different, lawful one.
    #[test]
    fn a_no_break_space_in_an_id_is_part_of_the_label() {
        let marked = document("<id>b0\u{a0}</id><uri>urn:ex:p</uri><uri>urn:ex:o</uri>");
        parse_dataset(marked.as_bytes(), "application/trix", None)
            .expect_err("U+00A0 is in no PN_CHARS class, so the label is refused as written");
        // The neighbour: the same label surrounded by real XML `S` is trimmed and
        // parses, exactly as it always did.
        let padded = document("<id>\n  b0\n  </id><uri>urn:ex:p</uri><uri>urn:ex:o</uri>");
        let dataset = parse_dataset(padded.as_bytes(), "application/trix", None)
            .expect("XML `S` around a label is still whitespace");
        assert_eq!(dataset.quads().count(), 1);
    }
}
