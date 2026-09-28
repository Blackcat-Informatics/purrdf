// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared RDF 1.2 term spelling for text surfaces that hold owned term values.

use core::fmt;

use purrdf_iri::literal_escape::{LiteralEscapes, escape_body};

use crate::blank_label::{LabelAlphabet, encode_blank_label};
use crate::ir::TermValue;
use crate::iri_escape::push_escaped;
use crate::sink::TextOut;

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Whether to spell a simple literal's `xsd:string` datatype explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringDatatype {
    /// Write `"text"`.
    Elide,
    /// Write `"text"^^<http://www.w3.org/2001/XMLSchema#string>`.
    Explicit,
}

/// Whether bracketed IRIs use the shared `IRIREF` escape law.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IriSpelling {
    /// Escape forbidden scalars and the XML-carrier control range.
    Escaped,
    /// Write the IRI body verbatim.
    Raw,
}

/// Whether blank labels retain the scope and arbitrary label characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlankSpelling {
    /// Encode the `(label, scope)` pair into a parseable token.
    Encoded,
    /// Write only the label verbatim.
    Raw,
}

/// Whether a root IRI is a bare endpoint identity or a bracketed RDF term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootIriSpelling {
    /// Write a root IRI as a bare endpoint URI.
    Bare,
    /// Write a root IRI as `<iri>`.
    Bracketed,
}

/// Choices that determine the byte spelling of a term. Raw blank labels can lose
/// scope, so callers that need a round trip select [`BlankSpelling::Encoded`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TermWriteStyle {
    /// The lexical form's escape profile.
    pub literal_escapes: LiteralEscapes,
    /// Omit the explicit `^^xsd:string` suffix for a simple literal.
    pub string_datatype: StringDatatype,
    /// Escape forbidden `IRIREF` scalars in bracketed IRIs.
    pub iri: IriSpelling,
    /// Encode the blank node's `(label, scope)` pair into a parseable token.
    pub blank: BlankSpelling,
    /// Write a top-level IRI without brackets (for an endpoint identity).
    pub root_iri: RootIriSpelling,
}

impl TermWriteStyle {
    /// SPARQL term syntax for a SERVICE endpoint record.
    pub const SERVICE_ENDPOINT: Self = Self {
        literal_escapes: LiteralEscapes::ShortOnly,
        string_datatype: StringDatatype::Elide,
        iri: IriSpelling::Escaped,
        blank: BlankSpelling::Encoded,
        root_iri: RootIriSpelling::Bare,
    };

    /// RDF 1.2 term syntax in a ShEx result shape map.
    pub const SHAPE_MAP: Self = Self {
        literal_escapes: LiteralEscapes::ShortOnly,
        string_datatype: StringDatatype::Elide,
        iri: IriSpelling::Escaped,
        blank: BlankSpelling::Encoded,
        root_iri: RootIriSpelling::Bracketed,
    };
}

/// Write an owned term, including arbitrarily nested RDF 1.2 triple terms.
///
/// # Errors
///
/// Returns a formatting error if the output rejects a literal body.
pub fn write_term_value<W: TextOut + ?Sized>(
    value: &TermValue,
    style: TermWriteStyle,
    out: &mut W,
) -> fmt::Result {
    if style.root_iri == RootIriSpelling::Bare
        && let TermValue::Iri(iri) = value
    {
        out.push_str(iri);
        return Ok(());
    }
    value.try_write_nested(
        out,
        "<<( ",
        " ",
        " )>>",
        |out, leaf| write_leaf(leaf, style, out),
        |out, text| {
            out.push_str(text);
            Ok(())
        },
    )
}

fn write_leaf<W: TextOut + ?Sized>(
    value: &TermValue,
    style: TermWriteStyle,
    out: &mut W,
) -> fmt::Result {
    match value {
        TermValue::Iri(iri) => {
            out.push('<');
            if style.iri == IriSpelling::Escaped {
                push_escaped(iri, out);
            } else {
                out.push_str(iri);
            }
            out.push('>');
        }
        TermValue::Blank { label, scope } => {
            out.push_str("_:");
            if style.blank == BlankSpelling::Encoded {
                out.push_str(&encode_blank_label(
                    label,
                    *scope,
                    LabelAlphabet::BlankNodeLabel,
                ));
            } else {
                out.push_str(label);
            }
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            out.push('"');
            escape_body(out, lexical_form, style.literal_escapes)?;
            out.push('"');
            if let Some(language) = language {
                out.push('@');
                out.push_str(language);
                if let Some(direction) = direction {
                    out.push_str("--");
                    out.push_str(direction.as_str());
                }
            } else if style.string_datatype == StringDatatype::Explicit || datatype != XSD_STRING {
                out.push_str("^^<");
                if style.iri == IriSpelling::Escaped {
                    push_escaped(datatype, out);
                } else {
                    out.push_str(datatype);
                }
                out.push('>');
            }
        }
        TermValue::Triple { .. } => unreachable!("triple components are written by the walk"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{BlankScope, RdfTextDirection, TermBox, TermValue};

    use super::{TermWriteStyle, write_term_value};

    #[test]
    fn directional_and_escaped_literal_keeps_its_value() {
        let value = TermValue::Literal {
            lexical_form: "a\"b\\c\n".into(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".into(),
            language: Some("ar".into()),
            direction: Some(RdfTextDirection::Rtl),
        };
        let mut out = String::new();
        write_term_value(&value, TermWriteStyle::SHAPE_MAP, &mut out).unwrap();
        assert_eq!(out, "\"a\\\"b\\\\c\\n\"@ar--rtl");
    }

    #[test]
    fn triple_and_scoped_blank_use_rdf12_syntax() {
        let value = TermValue::Triple {
            s: TermBox::new(TermValue::Blank {
                label: "a".into(),
                scope: BlankScope(7),
            }),
            p: TermBox::new(TermValue::iri("https://example.org/p")),
            o: TermBox::new(TermValue::simple_literal("x")),
        };
        let mut out = String::new();
        write_term_value(&value, TermWriteStyle::SHAPE_MAP, &mut out).unwrap();
        assert_eq!(out, "<<( _:purrdfesc7_a <https://example.org/p> \"x\" )>>");
    }
}
