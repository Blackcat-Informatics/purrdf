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

use crate::{
    QuadIds, RdfAnnotation, RdfDataset, RdfDiagnostic, RdfLiteral, RdfQuad, RdfReifier, RdfTerm,
    RdfTriple, TermId, TermRef, blank_label,
};
use std::fmt::Write as _;

/// Reject a blank-node label that is not legal under the exact W3C Turtle/SPARQL
/// `BLANK_NODE_LABEL` production ([`blank_label::is_valid_blank_node_label`]).
///
/// This emitter is label-preserving, so an out-of-alphabet label would otherwise
/// round-trip into a document no conforming parser (including PurRDF's own) can
/// read back. The failure is a hard error, never a silent remap: relabeling here
/// would silently change the caller's blank-node identity story.
fn check_blank_label(label: &str) -> Result<(), RdfDiagnostic> {
    if blank_label::is_valid_blank_node_label(label) {
        Ok(())
    } else {
        Err(RdfDiagnostic::error(
            "turtle-emit-blank-label",
            format!(
                "invalid blank-node label {label:?} for the Turtle/SPARQL BLANK_NODE_LABEL \
                 alphabet: the Turtle emitter refuses to write an unparsable `_:` term"
            ),
        ))
    }
}

/// Walk an owned term tree, rejecting any blank-node label that is not legal
/// Turtle `BLANK_NODE_LABEL` syntax (a triple term recurses into its subject and
/// object; its predicate is an IRI string and cannot carry a label).
fn check_term_labels(term: &RdfTerm) -> Result<(), RdfDiagnostic> {
    match term {
        RdfTerm::BlankNode(label) => check_blank_label(label),
        RdfTerm::Triple(triple) => {
            check_term_labels(&triple.subject)?;
            check_term_labels(&triple.object)
        }
        RdfTerm::Iri(_) | RdfTerm::Literal(_) => Ok(()),
    }
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
/// rendered string.
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when the scope-qualified blank-node label is not
/// legal Turtle `BLANK_NODE_LABEL` syntax — the buffer may then hold a partial
/// prefix of the term and must be discarded by the caller.
pub fn write_dataset_term(
    dataset: &RdfDataset,
    id: TermId,
    out: &mut String,
) -> Result<(), RdfDiagnostic> {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => {
            out.push('<');
            write_iri_escaped(iri, out);
            out.push('>');
        }
        TermRef::Blank { label, scope } => {
            let qualified = scope.qualify_label(label);
            check_blank_label(&qualified)?;
            out.push_str("_:");
            out.push_str(&qualified);
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
            write_dataset_term(dataset, s, out)?;
            out.push(' ');
            write_dataset_predicate(dataset, p, out);
            out.push(' ');
            write_dataset_term(dataset, o, out)?;
            out.push_str(" )>>");
        }
    }
    Ok(())
}

fn write_dataset_predicate(dataset: &RdfDataset, id: TermId, out: &mut String) {
    let TermRef::Iri(iri) = dataset.resolve(id) else {
        unreachable!("predicate must resolve to an IRI")
    };
    out.push('<');
    out.push_str(iri);
    out.push('>');
}

/// Append one ID-native quad as the same default-graph statement emitted by
/// [`emit_quad`]. The graph-name slot is intentionally ignored by this Turtle
/// projection, matching the owned emitter.
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when a blank-node label in the quad is not legal
/// Turtle `BLANK_NODE_LABEL` syntax; the buffer must then be discarded.
pub fn write_dataset_quad(
    dataset: &RdfDataset,
    quad: QuadIds,
    out: &mut String,
) -> Result<(), RdfDiagnostic> {
    write_dataset_term(dataset, quad.s, out)?;
    out.push(' ');
    write_dataset_predicate(dataset, quad.p, out);
    out.push(' ');
    write_dataset_term(dataset, quad.o, out)?;
    out.push_str(" .\n");
    Ok(())
}

/// Append one ID-native annotation row without materializing owned terms.
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when a blank-node label in the row is not legal
/// Turtle `BLANK_NODE_LABEL` syntax; the buffer must then be discarded.
pub fn write_dataset_annotation(
    dataset: &RdfDataset,
    reifier: TermId,
    predicate: TermId,
    object: TermId,
    out: &mut String,
) -> Result<(), RdfDiagnostic> {
    write_dataset_term(dataset, reifier, out)?;
    out.push(' ');
    write_dataset_predicate(dataset, predicate, out);
    out.push(' ');
    write_dataset_term(dataset, object, out)?;
    out.push_str(" .\n");
    Ok(())
}

/// Append one ID-native reifier binding without materializing its statement tree.
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when a blank-node label in the binding is not
/// legal Turtle `BLANK_NODE_LABEL` syntax; the buffer must then be discarded.
pub fn write_dataset_reifier(
    dataset: &RdfDataset,
    reifier: TermId,
    statement: TermId,
    out: &mut String,
) -> Result<(), RdfDiagnostic> {
    const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
    write_dataset_term(dataset, reifier, out)?;
    out.push_str(" <");
    out.push_str(RDF_REIFIES);
    out.push_str("> ");
    write_dataset_term(dataset, statement, out)?;
    out.push_str(" .\n");
    Ok(())
}

/// Render an [`RdfTerm`] in Turtle term syntax WITHOUT enforcing blank-node
/// label alphabets (full `<iri>`, `_:bnode`, literal, or the RDF 1.2
/// non-asserting triple term `<<( <s> <p> <o> )>>`).
///
/// This is a DISPLAY surface for diagnostics, report identity strings and
/// `Display` impls — never document egress. A label outside the Turtle
/// `BLANK_NODE_LABEL` alphabet renders verbatim here (an error message must be
/// able to name the offending term); [`emit_term`] is the validated egress form
/// that refuses such a label.
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
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when a blank-node label anywhere in the term
/// tree is not legal Turtle `BLANK_NODE_LABEL` syntax — the emitter refuses to
/// write a document no conforming parser could read back, and it never silently
/// remaps a label.
pub fn emit_term(term: &RdfTerm) -> Result<String, RdfDiagnostic> {
    check_term_labels(term)?;
    Ok(display_term(term))
}

/// Render an [`RdfTriple`] as an RDF 1.2 triple-term: `<<( <s> <p> <o> )>>`,
/// without label validation (the display-layer twin of [`emit_triple_term`]).
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
/// validating every blank-node label in the tree first.
fn emit_triple_term(triple: &RdfTriple) -> Result<String, RdfDiagnostic> {
    check_term_labels(&triple.subject)?;
    check_term_labels(&triple.object)?;
    Ok(display_triple_term(triple))
}

/// Emit a single quad as a Turtle statement line (`<s> <p> <o> .`).
///
/// The graph component (if any) is dropped — the emitter writes a single default
/// graph Turtle document, matching the native-lane artifacts (worlds are carried
/// as `purrdf:inWorld` annotations, not Turtle named graphs).
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when a blank-node label in the quad is not legal
/// Turtle `BLANK_NODE_LABEL` syntax.
pub fn emit_quad(quad: &RdfQuad) -> Result<String, RdfDiagnostic> {
    Ok(format!(
        "{} <{}> {} .\n",
        emit_term(&quad.subject)?,
        quad.predicate,
        emit_term(&quad.object)?
    ))
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
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when a blank-node label in the binding is not
/// legal Turtle `BLANK_NODE_LABEL` syntax (the anonymised `[]` subject form
/// carries no label and cannot fail on the subject).
pub fn emit_reifier(
    reifier: &RdfReifier,
    annotations: &[(String, String)],
) -> Result<String, RdfDiagnostic> {
    const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
    let subject = match &reifier.reifier {
        RdfTerm::BlankNode(_) if !annotations.is_empty() => "[]".to_owned(),
        other => emit_term(other)?,
    };
    let statement = emit_triple_term(&reifier.statement)?;
    let mut out = format!("{subject} <{RDF_REIFIES}> {statement}");
    for (predicate, object) in annotations {
        let _ = write!(out, " ;\n   <{predicate}> {object}");
    }
    out.push_str(" .\n");
    Ok(out)
}

/// Emit a free-standing resource: `<subject> a <type> ; <pred> <obj> ; … .`
///
/// Each `(predicate, object)` pair is already serialized (predicate is a bare
/// IRI string, object an already-emitted term string), so this is the generic
/// "subject with a property list" writer the ledger / explanation builders use.
pub fn emit_resource(subject: &str, properties: &[(String, String)]) -> String {
    let mut out = format!("<{subject}>");
    let mut first = true;
    for (predicate, object) in properties {
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
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when a blank-node label in the annotation is not
/// legal Turtle `BLANK_NODE_LABEL` syntax.
pub fn emit_annotation(annotation: &RdfAnnotation) -> Result<String, RdfDiagnostic> {
    Ok(format!(
        "{} <{}> {} .\n",
        emit_term(&annotation.reifier)?,
        annotation.predicate,
        emit_term(&annotation.object)?
    ))
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
            emit_term(&iri("http://example.org/a")).expect("legal IRI term emits"),
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
            emit_term(&RdfTerm::triple(triple)).expect("legal triple term emits"),
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
            write_dataset_quad(&dataset, quad, &mut out).expect("legal labels write");
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
        assert_eq!(
            emit_term(&term).expect("literal term emits"),
            "\"hello\"@ar--rtl"
        );
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
        assert_eq!(emit_term(&term).expect("literal term emits"), "\"x\"@en");
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
                "https://purrdf.org/ontology#viaRule".to_owned(),
                "<https://purrdf.org/rule/x>".to_owned(),
            )],
        )
        .expect("legal labels emit");
        // Anonymous reifier subject, rdf:reifies head, non-asserting triple term,
        // and the folded annotation — all in one statement.
        assert!(out.starts_with("[] <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( "));
        assert!(out.contains("purrdf.org/ontology#viaRule> <https://purrdf.org/rule/x>"));
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

        let out = emit_reifier(&reifier, &[]).expect("legal labels emit");
        assert!(
            out.starts_with("_:r0 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( "),
            "blank reifier must keep its label when annotations ride standalone: {out}"
        );

        // A standalone annotation triple on the same reifier resolves to the
        // very same blank node, so the link survives serialization.
        let annotation = RdfAnnotation::new(
            RdfTerm::blank_node("r0"),
            "https://purrdf.org/ontology#viaRule",
            RdfTerm::iri("https://purrdf.org/rule/x"),
        );
        assert!(
            emit_annotation(&annotation)
                .expect("legal labels emit")
                .starts_with("_:r0 ")
        );
    }

    #[test]
    fn emit_term_rejects_illegal_blank_label_loudly() {
        // A label outside the Turtle BLANK_NODE_LABEL alphabet is a hard error,
        // never a silent remap — in every emitting position, including nested
        // inside a triple term.
        for label in ["a b", "<urn:x>", "trailing.", "-lead", "\u{D7}y"] {
            let err = emit_term(&RdfTerm::blank_node(label)).expect_err("illegal label rejected");
            assert!(
                err.message.contains("BLANK_NODE_LABEL"),
                "error names the alphabet: {err:?}"
            );
            assert!(
                err.message.contains(label),
                "error names the label: {err:?}"
            );

            let triple = RdfTriple::new(
                RdfTerm::blank_node(label),
                "http://example.org/p",
                iri("http://example.org/o"),
            );
            emit_term(&RdfTerm::triple(triple.clone()))
                .expect_err("illegal label inside a triple term rejected");
            emit_quad(&RdfQuad {
                subject: RdfTerm::blank_node(label),
                predicate: "http://example.org/p".to_string(),
                object: iri("http://example.org/o"),
                graph_name: None,
                location: None,
            })
            .expect_err("illegal quad subject rejected");
            emit_reifier(&RdfReifier::new(iri("http://example.org/r"), triple), &[])
                .expect_err("illegal label inside a reifier statement rejected");
        }
        // display_term stays total over the same labels: it is the diagnostic
        // surface an error message renders through, not document egress.
        assert_eq!(display_term(&RdfTerm::blank_node("a b")), "_:a b");
    }

    #[test]
    fn write_dataset_term_validates_the_qualified_label() {
        // The borrowed writer validates the SCOPE-QUALIFIED label: a legal raw
        // label stays legal after qualification (dots double, suffix appended),
        // while an illegal raw label is refused.
        let mut builder = crate::RdfDatasetBuilder::new();
        let good = builder.intern_blank("a.b", crate::BlankScope(4));
        let bad = builder.intern_blank("a b", crate::BlankScope::DEFAULT);
        let p = builder.intern_iri("http://example.org/p");
        builder.push_quad(good, p, bad, None);
        let dataset = builder.freeze().expect("dataset freezes");

        let mut out = String::new();
        write_dataset_term(&dataset, good, &mut out).expect("legal label writes");
        assert_eq!(out, "_:a..b.s4", "raw dots double before the scope suffix");

        let mut out = String::new();
        let err =
            write_dataset_term(&dataset, bad, &mut out).expect_err("illegal label is refused");
        assert!(err.message.contains("BLANK_NODE_LABEL"), "{err:?}");
        for quad in dataset.quads() {
            let mut line = String::new();
            write_dataset_quad(&dataset, quad, &mut line)
                .expect_err("quad carrying the illegal label is refused");
        }
    }

    #[test]
    fn emit_resource_property_list() {
        let out = emit_resource(
            "https://purrdf.org/ontology#dl-el-crosscheck",
            &[
                (
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_owned(),
                    "<https://purrdf.org/ontology#CrosscheckLedger>".to_owned(),
                ),
                (
                    "https://purrdf.org/ontology#consistent".to_owned(),
                    "true".to_owned(),
                ),
            ],
        );
        assert!(out.contains("<https://purrdf.org/ontology#dl-el-crosscheck>"));
        assert!(out.contains("#type> <https://purrdf.org/ontology#CrosscheckLedger> ;"));
        assert!(out.contains("#consistent> true ."));
    }
}
