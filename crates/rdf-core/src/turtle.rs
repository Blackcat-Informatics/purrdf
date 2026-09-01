// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF 1.2 Turtle emitter for [`crate::store`] stores.
//!
//! This is a hand-written, full-IRI Turtle serializer over the purrdf model
//! ([`RdfQuad`] / [`RdfReifier`] / [`RdfAnnotation`] / [`RdfTerm`]). It exists
//! because oxigraph's `Store::dump` rewrites the RDF 1.2 reifier shorthand
//! `<< s p o >>` into an extra `rdf:reifies` indirection node with opaque blank
//! labels — changing the *structure* of the document. The native reasoning lane
//! commits artifacts whose structure (`[] rdf:reifies <<( … )>>`, triple-term
//! objects via `purrdf:concludes <<( … )>>`, etc.) must be preserved, so this
//! emitter writes the clean full-IRI form the committed artifacts use.
//!
//! The emitter is intentionally *cosmetic-agnostic*: it emits FULL `<iri>` forms
//! everywhere (no prefix compaction). Banners / `@prefix` blocks are not the
//! emitter's concern — a caller may prepend a literal header. The drift gate
//! that guards the artifacts compares RDFC-1.0 canonical quad sets (graph
//! isomorphism), so prefix compaction and comment banners are immaterial; the
//! triple/reifier/annotation *structure* is what must round-trip.
//!
//! ## Term forms
//!
//! - IRI: `<iri>`
//! - Blank node: `_:label` (or `[]` for an empty/anonymous reifier subject —
//!   see [`emit_reifier`] / [`emit_annotation`])
//! - Literal: `"lex"`, `"lex"@lang`, `"lex"@lang--ltr`/`"lex"@lang--rtl`, `"lex"^^<datatype>` (escaped)
//! - Triple term (RDF 1.2): `<<( <s> <p> <o> )>>` (non-asserting; distinct from
//!   the bare `<< s p o >>` reifier shorthand, which asserts the triple)
//!
//! ## Blank-node labels
//!
//! Every `_:` term this module writes goes through one encode helper, which
//! passes an unscoped label already legal under the exact W3C
//! `BLANK_NODE_LABEL` production straight through (byte-identical) and
//! otherwise rewrites it as the deterministic, injective envelope in
//! [`crate::blank_label`]. Emission is therefore **total** — every dataset
//! serializes — and the emitted document always re-lexes. Because a blank-node
//! label carries no meaning (RDF identifies blank nodes only up to renaming)
//! and the encoding is injective, the emitted document is isomorphic to the
//! input: co-reference is preserved and distinct blank nodes stay distinct.
//! Callers wanting labels of their own choosing rewrite the dataset first with
//! the explicit recourse operations (`canonical_relabel` / `skolemize` /
//! `deskolemize`).

use crate::{
    QuadIds, RdfAnnotation, RdfDataset, RdfLiteral, RdfQuad, RdfReifier, RdfTerm, RdfTriple,
    TermId, TermRef,
    blank_label::{LabelAlphabet, encode_blank_label, retarget_owned_label},
};
use std::borrow::Cow;
use std::fmt::Write as _;

/// The blank-node label this emitter writes after `_:` for an OWNED-model term:
/// the caller's label when it is already legal under the exact W3C
/// Turtle/SPARQL `BLANK_NODE_LABEL` production, otherwise the deterministic,
/// injective envelope ([`retarget_owned_label`]).
///
/// The input is an [`RdfTerm::BlankNode`](crate::RdfTerm::BlankNode) slot, which
/// already carries a `(label, scope)` pair encoded under the owned model's
/// unconstrained alphabet — so this RE-TARGETS that encoding into the Turtle
/// alphabet rather than escaping it a second time (which would envelope an
/// envelope and stop the round trip restoring label identity).
///
/// Encoding rather than refusing keeps serialization total: a blank-node label
/// carries no meaning (RDF identifies blank nodes only up to renaming), so a
/// rewritten label preserves the graph up to isomorphism, while an emitted
/// out-of-alphabet label would produce a document no conforming parser —
/// including PurRDF's own — could read back.
fn emit_blank_label(label: &str) -> Cow<'_, str> {
    retarget_owned_label(label, LabelAlphabet::BlankNodeLabel)
}

/// Percent-encode a string the way Python's `urllib.parse.quote(value, safe="")`
/// does: every byte that is not an *unreserved* URI character
/// (`A-Z a-z 0-9 - . _ ~`) is replaced by its uppercase `%XX` form.
///
/// Used to mint rule IRIs (`<base>rule/<encoded-name>`) byte-identically to the
/// retired Python `_rule_iri` so the inferred-closure / explanations artifacts
/// stay RDF-isomorphic to the committed files.
fn percent_encode(value: &str) -> String {
    fn is_unreserved(b: u8) -> bool {
        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
    }
    let mut out = String::with_capacity(value.len());
    for &byte in value.as_bytes() {
        if is_unreserved(byte) {
            out.push(byte as char);
        } else {
            out.push('%');
            let _ = write!(out, "{byte:02X}");
        }
    }
    out
}

/// Mint the namespaced, percent-encoded rule IRI for a rule label.
///
/// `base` is the caller-supplied rule-IRI base (e.g. `https://example.org/vocab/rule/`)
/// and `rule_name` the firing rule's name. The result is `<base + encoded-name>`,
/// matching the retired Python `_rule_iri` byte-for-byte.
pub fn rule_iri(base: &str, rule_name: &str) -> String {
    format!("{base}{}", percent_encode(rule_name))
}

/// Escape a string for embedding in a double-quoted Turtle literal.
///
/// Backslash first (so later escapes are not doubled), then the quote and the
/// readable ECHAR forms (`\n \r \t`). The remaining C0 control characters and
/// DEL (`0x7F`) are escaped as `\uXXXX` — the N-Triples/N-Quads literal grammar
/// forbids them raw. The C1 block (`0x80`-`0x9F`) is left **raw**: the
/// N-Triples/N-Quads literal grammar permits it and the W3C RDFC-1.0 fixtures
/// pin it passing through unescaped. Mirrors
/// [`crate::ir::canon::write_literal_escaped`] exactly.
fn escape_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    write_literal_escaped(value, &mut out);
    out
}

fn write_literal_escaped(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
}

/// Render an [`RdfLiteral`] as an N-Triples/Turtle literal token.
///
/// Forms produced:
/// - `"lex"@lang` — plain language-tagged string
/// - `"lex"@lang--ltr` / `"lex"@lang--rtl` — RDF 1.2 directional language-tagged string
/// - `"lex"^^<datatype>` — datatype literal
/// - `"lex"` — plain literal (no lang, no datatype)
///
/// Direction without a language tag is malformed RDF and is silently ignored.
/// This function is infallible.
fn emit_literal(literal: &RdfLiteral) -> String {
    let lex = escape_literal(&literal.lexical_form);
    if let Some(lang) = &literal.language {
        match literal.direction {
            Some(dir) => format!("\"{lex}\"@{lang}--{}", dir.as_str()),
            None => format!("\"{lex}\"@{lang}"),
        }
    } else if let Some(datatype) = &literal.datatype {
        format!("\"{lex}\"^^<{datatype}>")
    } else {
        format!("\"{lex}\"")
    }
}

/// Escape a string for embedding in an IRIREF (`<…>`).
///
/// The IRIREF grammar forbids the reserved delimiter set (`< > " { } | ^ \``
/// plus `\`) and the *entire* control range raw, so each of those — plus the
/// space character — is escaped as `\uXXXX`. Unlike literals, the C1 block
/// (`0x80`-`0x9F`) is escaped here too, since IRIREF has no carve-out for it.
/// Mirrors [`crate`]'s sibling `escape_iri` in
/// `purrdf::native_codecs::ser_model` exactly.
fn escape_iri(iri: &str) -> String {
    let mut out = String::with_capacity(iri.len());
    write_iri_escaped(iri, &mut out);
    out
}

fn write_iri_escaped(iri: &str, out: &mut String) {
    for ch in iri.chars() {
        match ch {
            '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => {
                let _ = write!(out, "\\u{:04X}", ch as u32);
            }
            c if c.is_control() || c == ' ' => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
}

/// Append one interned term to an existing Turtle/N-Triples output buffer.
///
/// This is the borrowed counterpart of [`emit_term`]: it resolves directly from
/// the frozen dataset and allocates neither an owned term tree nor an intermediate
/// rendered string. The blank node's `(label, scope)` pair is encoded into the
/// Turtle `BLANK_NODE_LABEL` alphabet in ONE step (via [`encode_blank_label`]),
/// so the buffer always holds a re-parsable term.
pub fn write_dataset_term(dataset: &RdfDataset, id: TermId, out: &mut String) {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => {
            out.push('<');
            write_iri_escaped(iri, out);
            out.push('>');
        }
        TermRef::Blank { label, scope } => {
            out.push_str("_:");
            out.push_str(&encode_blank_label(
                label,
                scope,
                LabelAlphabet::BlankNodeLabel,
            ));
        }
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            out.push('"');
            write_literal_escaped(lexical, out);
            out.push('"');
            if let Some(language) = language {
                out.push('@');
                out.push_str(language);
                if let Some(direction) = direction {
                    out.push_str("--");
                    out.push_str(direction.as_str());
                }
            } else {
                let TermRef::Iri(datatype) = dataset.resolve(datatype) else {
                    unreachable!("literal datatype must resolve to an IRI")
                };
                out.push_str("^^<");
                out.push_str(datatype);
                out.push('>');
            }
        }
        TermRef::Triple { s, p, o } => {
            out.push_str("<<( ");
            write_dataset_term(dataset, s, out);
            out.push(' ');
            write_dataset_predicate(dataset, p, out);
            out.push(' ');
            write_dataset_term(dataset, o, out);
            out.push_str(" )>>");
        }
    }
}

fn write_dataset_predicate(dataset: &RdfDataset, id: TermId, out: &mut String) {
    let TermRef::Iri(iri) = dataset.resolve(id) else {
        unreachable!("predicate must resolve to an IRI")
    };
    out.push('<');
    out.push_str(iri);
    out.push('>');
}

/// The `rdf:reifies` IRI every reifier binding is written under.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Append `s p o [g] .\n` — the one statement writer behind every `write_dataset_*`
/// entry point below, so the N-Triples and N-Quads spellings of a row cannot drift
/// apart in anything but the graph slot.
///
/// `graph` is `None` for the default graph AND for every triple-only projection;
/// `Some(id)` appends the fourth N-Quads term. A graph name is an IRI or a blank
/// node in the RDF 1.2 abstract syntax, but it is rendered through the same total
/// [`write_dataset_term`] as every other position rather than a partial match, for
/// the same reason that function is total.
fn write_dataset_statement(
    dataset: &RdfDataset,
    subject: TermId,
    predicate: TermId,
    object: TermId,
    graph: Option<TermId>,
    out: &mut String,
) {
    write_dataset_term(dataset, subject, out);
    out.push(' ');
    write_dataset_predicate(dataset, predicate, out);
    out.push(' ');
    write_dataset_term(dataset, object, out);
    if let Some(graph) = graph {
        out.push(' ');
        write_dataset_term(dataset, graph, out);
    }
    out.push_str(" .\n");
}

/// Append `<reifier> rdf:reifies <statement> [g] .\n`.
fn write_dataset_reifier_statement(
    dataset: &RdfDataset,
    reifier: TermId,
    statement: TermId,
    graph: Option<TermId>,
    out: &mut String,
) {
    write_dataset_term(dataset, reifier, out);
    out.push_str(" <");
    out.push_str(RDF_REIFIES);
    out.push_str("> ");
    write_dataset_term(dataset, statement, out);
    if let Some(graph) = graph {
        out.push(' ');
        write_dataset_term(dataset, graph, out);
    }
    out.push_str(" .\n");
}

/// Append one ID-native quad as the same default-graph statement emitted by
/// [`emit_quad`]. The graph-name slot is intentionally ignored by this Turtle
/// projection, matching the owned emitter.
///
/// Ignoring the graph is only honest for a caller that has already established it
/// has nowhere to put one. A caller rendering a graph-CARRYING dataset wants
/// [`write_dataset_nquad`], which spells the slot out instead of dropping it.
pub fn write_dataset_quad(dataset: &RdfDataset, quad: QuadIds, out: &mut String) {
    write_dataset_statement(dataset, quad.s, quad.p, quad.o, None, out);
}

/// Append one ID-native quad as an N-Quads statement, CARRYING its graph slot:
/// `s p o .` in the default graph and `s p o g .` in a named one.
///
/// The graph-preserving twin of [`write_dataset_quad`]. A default-graph-only
/// dataset renders byte-identically through either, because an N-Quads line with
/// no graph term IS the N-Triples line — which is why widening a triple-only
/// egress to this writer never changes an existing document, and only ever adds
/// the term that was being dropped.
pub fn write_dataset_nquad(dataset: &RdfDataset, quad: QuadIds, out: &mut String) {
    write_dataset_statement(dataset, quad.s, quad.p, quad.o, quad.g, out);
}

/// Append one ID-native annotation row without materializing owned terms.
///
/// The annotation's own graph slot is dropped, exactly as [`write_dataset_quad`]
/// drops a base quad's; [`write_dataset_annotation_nquad`] keeps it.
pub fn write_dataset_annotation(
    dataset: &RdfDataset,
    reifier: TermId,
    predicate: TermId,
    object: TermId,
    out: &mut String,
) {
    write_dataset_statement(dataset, reifier, predicate, object, None, out);
}

/// Append one ID-native annotation row as an N-Quads statement, carrying the graph
/// slot the annotation was asserted in.
///
/// The RDF 1.2 statement layer is keyed PER GRAPH — one reifier id may be
/// annotated independently in two graphs — so an annotation's graph is content,
/// not decoration, and a graph-carrying egress that dropped it would silently
/// merge two graphs' annotations of the same reifier.
pub fn write_dataset_annotation_nquad(
    dataset: &RdfDataset,
    reifier: TermId,
    predicate: TermId,
    object: TermId,
    graph: Option<TermId>,
    out: &mut String,
) {
    write_dataset_statement(dataset, reifier, predicate, object, graph, out);
}

/// Append one ID-native reifier binding without materializing its statement tree.
///
/// The declaration's own graph slot is dropped;
/// [`write_dataset_reifier_nquad`] keeps it.
pub fn write_dataset_reifier(
    dataset: &RdfDataset,
    reifier: TermId,
    statement: TermId,
    out: &mut String,
) {
    write_dataset_reifier_statement(dataset, reifier, statement, None, out);
}

/// Append one ID-native reifier binding as an N-Quads statement, carrying the graph
/// slot the declaration was made in (see [`write_dataset_annotation_nquad`] for why
/// that slot is content).
pub fn write_dataset_reifier_nquad(
    dataset: &RdfDataset,
    reifier: TermId,
    statement: TermId,
    graph: Option<TermId>,
    out: &mut String,
) {
    write_dataset_reifier_statement(dataset, reifier, statement, graph, out);
}

/// Render an [`RdfTerm`] in Turtle term syntax WITHOUT applying the blank-node
/// label escape (full `<iri>`, `_:bnode`, literal, or the RDF 1.2 non-asserting
/// triple term `<<( <s> <p> <o> )>>`).
///
/// This is a DISPLAY surface for diagnostics, report identity strings and
/// `Display` impls — never document egress. A label outside the Turtle
/// `BLANK_NODE_LABEL` alphabet renders verbatim here, so a message can name the
/// caller's own label; [`emit_term`] is the egress form, which escapes it.
#[must_use]
pub fn display_term(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => format!("<{}>", escape_iri(iri)),
        RdfTerm::BlankNode(label) => format!("_:{label}"),
        RdfTerm::Literal(literal) => emit_literal(literal),
        RdfTerm::Triple(triple) => display_triple_term(triple),
    }
}

/// Serialize an [`RdfTerm`] to its Turtle form (full `<iri>`, `_:bnode`, literal,
/// or the RDF 1.2 non-asserting triple term `<<( <s> <p> <o> )>>`).
///
/// A blank-node label outside the Turtle `BLANK_NODE_LABEL` alphabet is encoded
/// (via [`retarget_owned_label`]) rather than refused, so the rendered term
/// always re-lexes; the encoding is deterministic and injective, so blank-node
/// co-reference is preserved exactly.
#[must_use]
pub fn emit_term(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => format!("<{}>", escape_iri(iri)),
        RdfTerm::BlankNode(label) => format!("_:{}", emit_blank_label(label)),
        RdfTerm::Literal(literal) => emit_literal(literal),
        RdfTerm::Triple(triple) => emit_triple_term(triple),
    }
}

/// Render an [`RdfTriple`] as an RDF 1.2 triple-term: `<<( <s> <p> <o> )>>`,
/// without the label escape (the display-layer twin of [`emit_triple_term`]).
///
/// The parens matter — the bare `<< s p o >>` form is a *reifying triple* that
/// ALSO asserts `s p o` (and mints a reifier), so re-parsing it would grow the
/// graph. A triple term denotes the triple without asserting it, which is what
/// every embedded position (a triple-term object, or the `rdf:reifies` object
/// via [`emit_reifier`]) requires.
fn display_triple_term(triple: &RdfTriple) -> String {
    format!(
        "<<( {} <{}> {} )>>",
        display_term(&triple.subject),
        triple.predicate,
        display_term(&triple.object)
    )
}

/// Serialize an [`RdfTriple`] as an RDF 1.2 triple-term (`<<( <s> <p> <o> )>>`),
/// escaping every blank-node label in the tree.
fn emit_triple_term(triple: &RdfTriple) -> String {
    format!(
        "<<( {} <{}> {} )>>",
        emit_term(&triple.subject),
        triple.predicate,
        emit_term(&triple.object)
    )
}

/// Emit a single quad as a Turtle statement line (`<s> <p> <o> .`).
///
/// The graph component (if any) is dropped — the emitter writes a single default
/// graph Turtle document, matching the native-lane artifacts (worlds are carried
/// as `purrdf:inWorld` annotations, not Turtle named graphs).
#[must_use]
pub fn emit_quad(quad: &RdfQuad) -> String {
    format!(
        "{} <{}> {} .\n",
        emit_term(&quad.subject),
        quad.predicate,
        emit_term(&quad.object)
    )
}

/// Emit a reifier binding as `<reifier> rdf:reifies <<( s p o )>> ; <pred> <obj> ; … .`
///
/// A blank-node reifier is emitted as the anonymous `[]` form **only when
/// annotations are folded onto it** — then the whole binding is one
/// self-contained Turtle statement, and `[]` correctly mints a fresh, distinct
/// node per call (the derived-axiom builder reuses the same blank-node *label*
/// for every reifier, so anonymising is what keeps them apart).
///
/// When `annotations` is empty the reifier's annotations are emitted as
/// *standalone* triples elsewhere (e.g. `asserted_turtle`), which reference
/// the reifier by its blank-node label. Emitting `[]` here would mint a new
/// anonymous node disconnected from those triples, silently severing the
/// reifier↔annotation link — so the blank node is emitted by its label instead.
/// A named reifier is always emitted as its term.
///
/// Each `(predicate, object)` pair takes a bare-IRI predicate (matching the
/// sibling [`RdfTriple`] / [`RdfAnnotation`] field convention) and a
/// structured [`RdfTerm`] object, rendered through [`emit_term`] — so a
/// blank-node object gets its label escaped and a literal object gets proper
/// quoting, the same guarantee every other position in this module carries.
/// A caller can no longer hand this function an already-rendered token that
/// bypasses that escaping.
#[must_use]
pub fn emit_reifier(reifier: &RdfReifier, annotations: &[(String, RdfTerm)]) -> String {
    let subject = match &reifier.reifier {
        RdfTerm::BlankNode(_) if !annotations.is_empty() => "[]".to_owned(),
        other => emit_term(other),
    };
    let statement = emit_triple_term(&reifier.statement);
    let mut out = format!("{subject} <{RDF_REIFIES}> {statement}");
    for (predicate, object) in annotations {
        let _ = write!(out, " ;\n   <{predicate}> {}", emit_term(object));
    }
    out.push_str(" .\n");
    out
}

/// Emit a free-standing resource: `<subject> a <type> ; <pred> <obj> ; … .`
///
/// Each `(predicate, object)` pair takes a bare-IRI predicate (matching the
/// sibling [`RdfTriple`] / [`RdfAnnotation`] field convention) and a
/// structured [`RdfTerm`] object, rendered through [`emit_term`] — the
/// generic "subject with a property list" writer the ledger / explanation
/// builders use. Routing the object through [`emit_term`] means a blank-node
/// object is label-escaped and a literal object is properly quoted, rather
/// than trusting the caller to have pre-rendered a safe token.
pub fn emit_resource(subject: &str, properties: &[(String, RdfTerm)]) -> String {
    let mut out = format!("<{subject}>");
    let mut first = true;
    for (predicate, object) in properties {
        let object = emit_term(object);
        if first {
            let _ = write!(out, " <{predicate}> {object}");
            first = false;
        } else {
            let _ = write!(out, " ;\n   <{predicate}> {object}");
        }
    }
    out.push_str(" .\n");
    out
}

/// Emit a standalone annotation triple `<reifier> <predicate> <object> .`.
///
/// Mostly used in tests; the production builders fold annotations onto a reifier
/// head via [`emit_reifier`].
#[must_use]
pub fn emit_annotation(annotation: &RdfAnnotation) -> String {
    format!(
        "{} <{}> {} .\n",
        emit_term(&annotation.reifier),
        annotation.predicate,
        emit_term(&annotation.object)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iri(value: &str) -> RdfTerm {
        RdfTerm::iri(value)
    }

    #[test]
    fn percent_encode_matches_urllib_quote_safe_empty() {
        // colon → %3A, hyphen kept, alnum kept (matches the committed rule IRIs).
        assert_eq!(
            percent_encode("el:subPropertyOf-transitive"),
            "el%3AsubPropertyOf-transitive"
        );
        // space → %20, slash → %2F, unreserved kept.
        assert_eq!(percent_encode("a b/c.d_e~f"), "a%20b%2Fc.d_e~f");
    }

    #[test]
    fn rule_iri_is_base_plus_encoded_name() {
        assert_eq!(
            rule_iri(
                "https://example.org/vocab/rule/",
                "el:subClassOf-transitive"
            ),
            "https://example.org/vocab/rule/el%3AsubClassOf-transitive"
        );
    }

    #[test]
    fn emit_term_iri_is_angle_bracketed() {
        assert_eq!(
            emit_term(&iri("http://example.org/a")),
            "<http://example.org/a>"
        );
    }

    #[test]
    fn emit_term_triple_term_uses_non_asserting_parens() {
        let triple = RdfTriple::new(
            iri("http://example.org/s"),
            "http://example.org/p",
            iri("http://example.org/o"),
        );
        assert_eq!(
            emit_term(&RdfTerm::triple(triple)),
            "<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>>"
        );
    }

    #[test]
    fn write_dataset_term_triple_arm_uses_non_asserting_parens() {
        // Coverage for the ID-native (borrowed) writer's `TermRef::Triple` arm
        // directly: a triple-term OBJECT of an ordinary quad (not an
        // `rdf:reifies` statement) must serialize with the `<<( … )>>`
        // delimiter. Spelling it bare `<< … >>` would re-parse as a *reifying,
        // asserting* triple — a different, larger graph — so this asserts the
        // exact delimiter rather than only the component IRIs.
        let mut builder = crate::RdfDatasetBuilder::new();
        let s = builder.intern_iri("http://example.org/s");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_iri("http://example.org/o");
        let statement = builder.intern_triple(s, p, o);
        let outer_s = builder.intern_iri("http://example.org/outer");
        let outer_p = builder.intern_iri("http://example.org/concludes");
        builder.push_quad(outer_s, outer_p, statement, None);
        let dataset = builder.freeze().expect("dataset freezes");

        let mut out = String::new();
        for quad in dataset.quads() {
            write_dataset_quad(&dataset, quad, &mut out);
        }
        assert_eq!(
            out,
            "<http://example.org/outer> <http://example.org/concludes> \
<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>> .\n"
        );
    }

    #[test]
    fn emit_literal_lang_and_datatype() {
        assert_eq!(
            emit_literal(&RdfLiteral::language_tagged("hello \"x\"", "en")),
            "\"hello \\\"x\\\"\"@en"
        );
        assert_eq!(
            emit_literal(&RdfLiteral::typed(
                "42",
                "http://www.w3.org/2001/XMLSchema#integer"
            )),
            "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn emit_directional_literal_rtl() {
        use crate::model::RdfTextDirection;
        let lit = RdfLiteral {
            lexical_form: "hello".to_string(),
            datatype: None,
            language: Some("ar".to_string()),
            direction: Some(RdfTextDirection::Rtl),
        };
        let term = RdfTerm::Literal(lit);
        assert_eq!(emit_term(&term), "\"hello\"@ar--rtl");
    }

    #[test]
    fn emit_lang_literal_no_direction() {
        let lit = RdfLiteral {
            lexical_form: "x".to_string(),
            datatype: None,
            language: Some("en".to_string()),
            direction: None,
        };
        let term = RdfTerm::Literal(lit);
        assert_eq!(emit_term(&term), "\"x\"@en");
    }

    #[test]
    fn emit_reifier_blank_subject_is_anonymous_with_annotations() {
        let triple = RdfTriple::new(
            iri("http://example.org/s"),
            "http://example.org/p",
            iri("http://example.org/o"),
        );
        let reifier = RdfReifier::new(RdfTerm::blank_node("r0"), triple);
        let out = emit_reifier(
            &reifier,
            &[(
                "https://example.org/ontology#viaRule".to_owned(),
                iri("https://example.org/rule/x"),
            )],
        );
        // Anonymous reifier subject, rdf:reifies head, non-asserting triple term,
        // and the folded annotation — all in one statement.
        assert!(out.starts_with("[] <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( "));
        assert!(out.contains("example.org/ontology#viaRule> <https://example.org/rule/x>"));
        assert!(out.trim_end().ends_with(" ."));
    }

    #[test]
    fn emit_reifier_blank_subject_keeps_label_without_annotations() {
        // With no folded annotations the reifier's annotations are emitted as
        // standalone triples that reference it by blank-node label, so the
        // reifier must keep that label (not collapse to an anonymous `[]`),
        // else the rdf:reifies binding is severed from its annotations.
        let triple = RdfTriple::new(
            iri("http://example.org/s"),
            "http://example.org/p",
            iri("http://example.org/o"),
        );
        let reifier = RdfReifier::new(RdfTerm::blank_node("r0"), triple);

        let out = emit_reifier(&reifier, &[]);
        assert!(
            out.starts_with("_:r0 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( "),
            "blank reifier must keep its label when annotations ride standalone: {out}"
        );

        // A standalone annotation triple on the same reifier resolves to the
        // very same blank node, so the link survives serialization.
        let annotation = RdfAnnotation::new(
            RdfTerm::blank_node("r0"),
            "https://example.org/ontology#viaRule",
            RdfTerm::iri("https://example.org/rule/x"),
        );
        assert!(emit_annotation(&annotation).starts_with("_:r0 "));
    }

    #[test]
    fn emit_term_escapes_an_illegal_blank_label_in_every_position() {
        use crate::blank_label::is_valid_blank_node_label;

        // A label outside the Turtle BLANK_NODE_LABEL alphabet is escaped, never
        // refused — in every emitting position, including nested inside a triple
        // term — so emission stays total and the output re-lexes.
        for label in ["a b", "<urn:x>", "trailing.", "-lead", "\u{D7}y"] {
            let token = emit_term(&RdfTerm::blank_node(label));
            let emitted = token
                .strip_prefix("_:")
                .expect("a blank term emits the `_:` prefix");
            assert!(
                is_valid_blank_node_label(emitted),
                "{label:?} emitted as {emitted:?}, which is not a legal label"
            );
            assert_ne!(emitted, label, "an illegal label must be rewritten");

            let triple = RdfTriple::new(
                RdfTerm::blank_node(label),
                "http://example.org/p",
                iri("http://example.org/o"),
            );
            for rendered in [
                emit_term(&RdfTerm::triple(triple.clone())),
                emit_quad(&RdfQuad {
                    subject: RdfTerm::blank_node(label),
                    predicate: "http://example.org/p".to_string(),
                    object: iri("http://example.org/o"),
                    graph_name: None,
                    location: None,
                }),
                emit_reifier(&RdfReifier::new(iri("http://example.org/r"), triple), &[]),
            ] {
                assert!(
                    rendered.contains(emitted),
                    "every emitting position writes the escaped label: {rendered}"
                );
                assert!(
                    !rendered.contains(&format!("_:{label}")),
                    "the raw label must never reach the document: {rendered}"
                );
            }
        }
        // display_term stays verbatim over the same labels: it is the diagnostic
        // surface an error message renders through, not document egress.
        assert_eq!(display_term(&RdfTerm::blank_node("a b")), "_:a b");
    }

    #[test]
    fn emit_term_escape_preserves_distinctness_of_blank_nodes() {
        // Two distinct labels — one legal, one whose escape could collide with
        // it if the escape image were unreserved — stay distinct on egress.
        let illegal = "a b";
        let twin = "purrdfesc_a_000020b";
        assert!(crate::blank_label::is_valid_blank_node_label(twin));
        assert_ne!(
            emit_term(&RdfTerm::blank_node(illegal)),
            emit_term(&RdfTerm::blank_node(twin))
        );
    }

    #[test]
    fn write_dataset_term_encodes_the_label_and_scope_together() {
        // The borrowed writer encodes the `(label, scope)` pair in ONE step: a
        // legal raw label at the default scope is written verbatim, while a
        // scoped pair or an illegal raw label becomes the envelope.
        let mut builder = crate::RdfDatasetBuilder::new();
        let good = builder.intern_blank("a.b", crate::BlankScope(4));
        let bad = builder.intern_blank("a b", crate::BlankScope::DEFAULT);
        let p = builder.intern_iri("http://example.org/p");
        builder.push_quad(good, p, bad, None);
        let dataset = builder.freeze().expect("dataset freezes");

        let mut out = String::new();
        write_dataset_term(&dataset, good, &mut out);
        assert_eq!(
            out, "_:purrdfesc4_a_00002Eb",
            "a scoped pair is written as its envelope"
        );

        let mut out = String::new();
        write_dataset_term(&dataset, bad, &mut out);
        assert_eq!(out, "_:purrdfesc_a_000020b");

        let mut lines = String::new();
        for quad in dataset.quads() {
            write_dataset_quad(&dataset, quad, &mut lines);
        }
        assert_eq!(
            lines,
            "_:purrdfesc4_a_00002Eb <http://example.org/p> _:purrdfesc_a_000020b .\n"
        );
    }

    #[test]
    fn emit_resource_property_list() {
        let out = emit_resource(
            "https://example.org/ontology#dl-el-crosscheck",
            &[
                (
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_owned(),
                    iri("https://example.org/ontology#CrosscheckLedger"),
                ),
                (
                    "https://example.org/ontology#consistent".to_owned(),
                    RdfTerm::Literal(RdfLiteral::typed(
                        "true",
                        "http://www.w3.org/2001/XMLSchema#boolean",
                    )),
                ),
            ],
        );
        assert!(out.contains("<https://example.org/ontology#dl-el-crosscheck>"));
        assert!(out.contains("#type> <https://example.org/ontology#CrosscheckLedger> ;"));
        assert!(
            out.contains("#consistent> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .")
        );
    }

    #[test]
    fn emit_reifier_annotation_object_escapes_hostile_blank_label() {
        // A structured annotation object whose blank label is illegal under
        // BLANK_NODE_LABEL must be escaped, exactly like every other emitting
        // position in this module — the caller can no longer hand this
        // function an already-rendered token that bypasses that guarantee.
        // (`crates/rdf/tests` carries the companion full-document round-trip
        // through the native Turtle parser.)
        use crate::blank_label::is_valid_blank_node_label;

        let triple = RdfTriple::new(
            iri("http://example.org/s"),
            "http://example.org/p",
            iri("http://example.org/o"),
        );
        let reifier = RdfReifier::new(iri("http://example.org/r"), triple);
        for label in ["bad label", "a\u{d7}b"] {
            let out = emit_reifier(
                &reifier,
                &[(
                    "http://example.org/annotates".to_owned(),
                    RdfTerm::blank_node(label),
                )],
            );
            assert!(
                !out.contains(&format!("_:{label}")),
                "the raw hostile label must never reach the document: {out}"
            );
            let token = out
                .rsplit("_:")
                .next()
                .expect("emitted annotation carries a blank-node token")
                .trim_end_matches(" .\n");
            assert!(
                is_valid_blank_node_label(token),
                "{label:?} emitted as {token:?}, which is not a legal label: {out}"
            );
            assert_ne!(token, label, "an illegal label must be rewritten");
        }
    }

    #[test]
    fn emit_resource_property_escapes_hostile_blank_label() {
        // Same guarantee as emit_reifier, for the sibling property-list writer:
        // a hostile blank-node object label must be escaped, not written
        // verbatim.
        use crate::blank_label::is_valid_blank_node_label;

        for label in ["bad label", "a\u{d7}b"] {
            let out = emit_resource(
                "http://example.org/subject",
                &[(
                    "http://example.org/annotates".to_owned(),
                    RdfTerm::blank_node(label),
                )],
            );
            assert!(
                !out.contains(&format!("_:{label}")),
                "the raw hostile label must never reach the document: {out}"
            );
            let token = out
                .rsplit("_:")
                .next()
                .expect("emitted resource carries a blank-node token")
                .trim_end_matches(" .\n");
            assert!(
                is_valid_blank_node_label(token),
                "hostile label emitted as {token:?}, which is not a legal label: {out}"
            );
            assert_ne!(token, label, "an illegal label must be rewritten");
        }
    }
}
