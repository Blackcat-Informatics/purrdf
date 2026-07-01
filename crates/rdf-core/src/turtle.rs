// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF 1.2 Turtle emitter for [`crate::store`] stores.
//!
//! This is a hand-written, full-IRI Turtle serializer over the purrdf model
//! ([`RdfQuad`] / [`RdfReifier`] / [`RdfAnnotation`] / [`RdfTerm`]). It exists
//! because oxigraph's `Store::dump` rewrites the RDF 1.2 reifier shorthand
//! `<< s p o >>` into an extra `rdf:reifies` indirection node with opaque blank
//! labels — changing the *structure* of the document. The native reasoning lane
//! commits artifacts whose structure (`[] rdf:reifies << … >>`, triple-term
//! objects via `purrdf:concludes << … >>`, etc.) must be preserved, so this
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
//! - Triple term (RDF 1.2): `<< <s> <p> <o> >>` (the reifier-shorthand form)

use crate::{RdfAnnotation, RdfLiteral, RdfQuad, RdfReifier, RdfTerm, RdfTriple};
use std::fmt::Write as _;

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
/// `base` is the rule-IRI base (e.g. `https://blackcatinformatics.ca/purrdf/rule/`)
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
    out
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
    out
}

/// Serialize an [`RdfTerm`] to its Turtle form (full `<iri>`, `_:bnode`, literal,
/// or the RDF 1.2 triple-term shorthand `<< <s> <p> <o> >>`).
pub fn emit_term(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => format!("<{}>", escape_iri(iri)),
        RdfTerm::BlankNode(label) => format!("_:{label}"),
        RdfTerm::Literal(literal) => emit_literal(literal),
        RdfTerm::Triple(triple) => emit_triple_term(triple),
    }
}

/// Serialize an [`RdfTriple`] as an RDF 1.2 triple-term: `<< <s> <p> <o> >>`.
fn emit_triple_term(triple: &RdfTriple) -> String {
    format!(
        "<< {} <{}> {} >>",
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
pub fn emit_quad(quad: &RdfQuad) -> String {
    format!(
        "{} <{}> {} .\n",
        emit_term(&quad.subject),
        quad.predicate,
        emit_term(&quad.object)
    )
}

/// Emit a reifier binding as `<reifier> rdf:reifies << s p o >> ; <pred> <obj> ; … .`
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
pub fn emit_reifier(reifier: &RdfReifier, annotations: &[(String, String)]) -> String {
    const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
    let subject = match &reifier.reifier {
        RdfTerm::BlankNode(_) if !annotations.is_empty() => "[]".to_owned(),
        other => emit_term(other),
    };
    let statement = emit_triple_term(&reifier.statement);
    let mut out = format!("{subject} <{RDF_REIFIES}> {statement}");
    for (predicate, object) in annotations {
        let _ = write!(out, " ;\n   <{predicate}> {object}");
    }
    out.push_str(" .\n");
    out
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
                "https://blackcatinformatics.ca/purrdf/rule/",
                "el:subClassOf-transitive"
            ),
            "https://blackcatinformatics.ca/purrdf/rule/el%3AsubClassOf-transitive"
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
    fn emit_term_triple_term_is_reifier_shorthand() {
        let triple = RdfTriple::new(
            iri("http://example.org/s"),
            "http://example.org/p",
            iri("http://example.org/o"),
        );
        assert_eq!(
            emit_term(&RdfTerm::triple(triple)),
            "<< <http://example.org/s> <http://example.org/p> <http://example.org/o> >>"
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
                "https://purrdf.org/ontology#viaRule".to_owned(),
                "<https://purrdf.org/rule/x>".to_owned(),
            )],
        );
        // Anonymous reifier subject, rdf:reifies head, triple-term shorthand,
        // and the folded annotation — all in one statement.
        assert!(out.starts_with("[] <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> << "));
        assert!(out.contains("purrdf.org/ontology#viaRule> <https://purrdf.org/rule/x>"));
        assert!(out.trim_end().ends_with(" ."));
    }

    #[test]
    fn emit_reifier_blank_subject_keeps_label_without_annotations() {
        // With no folded annotations the reifier's annotations are emitted as
        // standalone triples that reference it by blank-node label, so the
        // reifier must keep that label (not collapse to an anonymous `[]`),
        // else the rdf:reifies binding is severed from its annotations. (#666)
        let triple = RdfTriple::new(
            iri("http://example.org/s"),
            "http://example.org/p",
            iri("http://example.org/o"),
        );
        let reifier = RdfReifier::new(RdfTerm::blank_node("r0"), triple);

        let out = emit_reifier(&reifier, &[]);
        assert!(
            out.starts_with("_:r0 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> << "),
            "blank reifier must keep its label when annotations ride standalone: {out}"
        );

        // A standalone annotation triple on the same reifier resolves to the
        // very same blank node, so the link survives serialization.
        let annotation = RdfAnnotation::new(
            RdfTerm::blank_node("r0"),
            "https://purrdf.org/ontology#viaRule",
            RdfTerm::iri("https://purrdf.org/rule/x"),
        );
        assert!(emit_annotation(&annotation).starts_with("_:r0 "));
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
