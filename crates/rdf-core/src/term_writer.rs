// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`write_term_value`]: an owned [`TermValue`] in the RDF 1.2 canonical term
//! syntax.
//!
//! Every spelling decision is [`purrdf_lex::term_syntax`]'s — the escaped
//! `IRIREF`, the canonical literal with `xsd:string` elided, the `<<( … )>>`
//! delimiters — and every blank-node label is made legal (and injective over
//! blank-node scopes) by [`crate::blank_label::encode_blank_label`]. What this adds is the one
//! thing the lexical layer cannot know: the shape of a [`TermValue`], a triple
//! term nesting other terms. It walks that nesting over an explicit work list,
//! so a deeply nested triple term costs heap, not call frames.

use purrdf_lex::term_syntax::{
    TRIPLE_TERM_CLOSE, TRIPLE_TERM_OPEN, write_blank, write_iri, write_literal,
};

use crate::blank_label::LabelAlphabet;
use crate::ir::TermValue;
use crate::sink::TextOut;

/// Append `term` in the RDF 1.2 canonical term syntax.
///
/// * An IRI is `<…>`, escaped by [`purrdf_lex::iri_escape`].
/// * A blank node is `_:` and its label under [`crate::blank_label::encode_blank_label`]'s
///   `BLANK_NODE_LABEL` alphabet, so a label from any scope re-parses to the
///   same `(label, scope)` pair.
/// * A literal is its canonical form ([`purrdf_lex::term_syntax::write_literal`]):
///   `xsd:string` elided, a language tag and base direction as `@lang--dir`.
/// * A triple term is `<<( s p o )>>`.
///
/// ```
/// use purrdf_core::{TermBox, TermValue, write_term_value};
///
/// let term = TermValue::Triple {
///     s: TermBox::new(TermValue::iri("http://example.org/s")),
///     p: TermBox::new(TermValue::iri("http://example.org/p")),
///     o: TermBox::new(TermValue::simple_literal("o")),
/// };
/// let mut out = String::new();
/// write_term_value(&term, &mut out);
/// assert_eq!(out, "<<( <http://example.org/s> <http://example.org/p> \"o\" )>>");
/// ```
pub fn write_term_value<W: TextOut + ?Sized>(term: &TermValue, out: &mut W) {
    let mut resident = purrdf_lex::allocation::Resident;
    let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
    write_term_value_with_memory(term, out, &mut memory).expect("resident RDF term spelling");
}

/// Append the same canonical RDF 1.2 term under original native working admission.
///
/// The destination keeps its own physical owner. This memory covers the actual
/// nested-term work list and temporary canonical blank label; each is destroyed
/// before its grant is released. Destination failure stops the same emitter.
///
/// # Errors
/// Returns the first working-layout, admission or allocator failure.
pub fn write_term_value_with_memory<
    W: TextOut + ?Sized,
    S: purrdf_lex::allocation::Admission + ?Sized,
>(
    term: &TermValue,
    out: &mut W,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<(), purrdf_lex::allocation::StorageError> {
    enum Step<'t> {
        Term(&'t TermValue),
        Text(&'static str),
    }
    memory.scope(|memory| {
        let mut held = Vec::new();
        let mut next = Some(Step::Term(term));
        while !out.failed() {
            let Some(step) = next.take().or_else(|| held.pop()) else {
                break;
            };
            let term = match step {
                Step::Text(text) => {
                    out.push_str(text);
                    continue;
                }
                Step::Term(term) => term,
            };
            match term {
                TermValue::Iri(iri) => write_iri(iri, out),
                TermValue::Blank { label, scope } => {
                    let encoded = crate::blank_label::encode_blank_label_with_memory(
                        label,
                        *scope,
                        LabelAlphabet::BlankNodeLabel,
                        memory,
                    )?;
                    write_blank(&encoded, out);
                    if let std::borrow::Cow::Owned(encoded) = encoded {
                        memory.release_string(encoded)?;
                    }
                }
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => {
                    write_literal(
                        lexical_form,
                        datatype,
                        language.as_deref(),
                        direction.map(crate::RdfTextDirection::as_str),
                        out,
                    );
                }
                TermValue::Triple { s, p, o } => {
                    out.push_str(TRIPLE_TERM_OPEN);
                    out.push(' ');
                    if out.failed() {
                        break;
                    }
                    for step in [
                        Step::Text(TRIPLE_TERM_CLOSE),
                        Step::Text(" "),
                        Step::Term(o),
                        Step::Text(" "),
                        Step::Term(p),
                        Step::Text(" "),
                    ] {
                        memory.push(&mut held, step)?;
                    }
                    next = Some(Step::Term(s));
                }
            }
        }
        memory.release_vec(held)
    })
}

#[cfg(test)]
mod tests {
    use super::write_term_value;
    use crate::{BlankScope, RdfTextDirection, TermBox, TermValue};

    fn written(term: &TermValue) -> String {
        let mut out = String::new();
        write_term_value(term, &mut out);
        out
    }

    fn literal(lex: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lex.to_owned(),
            datatype: datatype.to_owned(),
            language: None,
            direction: None,
        }
    }

    #[test]
    fn the_lexical_layers_xsd_string_is_the_xsd_crates() {
        // The canonical form elides exactly `purrdf_xsd`'s `xsd:string`.
        assert_eq!(
            written(&literal("a", purrdf_xsd::datatype::XSD_STRING)),
            "\"a\""
        );
        assert_eq!(
            written(&literal("1", purrdf_xsd::datatype::XSD_INTEGER)),
            "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn a_directional_literal_carries_tag_and_direction() {
        let term = TermValue::Literal {
            lexical_form: "\u{5e9}\"\\".to_owned(),
            datatype: purrdf_iri::vocab::rdf::DIR_LANG_STRING.to_owned(),
            language: Some("he".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        };
        assert_eq!(written(&term), "\"\u{5e9}\\\"\\\\\"@he--rtl");
    }

    #[test]
    fn iris_are_escaped_and_blank_labels_encoded() {
        assert_eq!(
            written(&TermValue::iri("http://example.org/a b")),
            "<http://example.org/a\\u0020b>"
        );
        assert_eq!(
            written(&TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope::DEFAULT,
            }),
            "_:b0"
        );
        let scoped = written(&TermValue::Blank {
            label: "b0".to_owned(),
            scope: BlankScope(3),
        });
        assert!(scoped.starts_with("_:") && scoped != "_:b0", "{scoped}");
    }

    #[test]
    fn a_deeply_nested_triple_term_is_written_without_recursion() {
        let depth = 50_000;
        let mut term = TermValue::iri("http://example.org/o");
        for _ in 0..depth {
            term = TermValue::Triple {
                s: TermBox::new(TermValue::iri("http://example.org/s")),
                p: TermBox::new(TermValue::iri("http://example.org/p")),
                o: TermBox::new(term),
            };
        }
        let mut expected = "<<( <http://example.org/s> <http://example.org/p> ".repeat(depth);
        expected.push_str("<http://example.org/o>");
        expected.push_str(&" )>>".repeat(depth));
        assert_eq!(written(&term), expected);
    }

    #[test]
    fn a_triple_term_in_subject_position_nests() {
        let inner = TermValue::Triple {
            s: TermBox::new(TermValue::iri("http://example.org/a")),
            p: TermBox::new(TermValue::iri("http://example.org/b")),
            o: TermBox::new(TermValue::iri("http://example.org/c")),
        };
        let outer = TermValue::Triple {
            s: TermBox::new(inner),
            p: TermBox::new(TermValue::iri("http://example.org/p")),
            o: TermBox::new(TermValue::simple_literal("x")),
        };
        assert_eq!(
            written(&outer),
            "<<( <<( <http://example.org/a> <http://example.org/b> <http://example.org/c> )>> \
             <http://example.org/p> \"x\" )>>"
        );
    }
}
