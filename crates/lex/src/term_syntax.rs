// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The RDF 1.2 term syntax every text writer spells a term with: an IRI as an
//! escaped `IRIREF`, a literal in its canonical N-Triples form, a blank node as
//! a `BLANK_NODE_LABEL`, and the delimiters of a triple term.
//!
//! # The contract
//!
//! This is the canonical term form of RDF 1.2 N-Triples (N-Triples 1.2 §4
//! "Canonical N-Triples"), which Turtle, TriG, N-Quads and SPARQL all accept
//! for a term:
//!
//! * an IRI is `<` + its body through [`crate::iri_escape`] + `>`;
//! * a literal is `"` + its lexical form through the
//!   [`Canonical`](Carrier::Canonical) carrier of [`crate::literal_escape`] +
//!   `"`, then `@lang`, `@lang--dir` or `^^<datatype>`. A literal whose
//!   datatype is `xsd:string` is written with no suffix at all (the canonical
//!   form omits it), and a language-tagged literal's datatype is implied by
//!   its tag and never written;
//! * a blank node is `_:` + its label;
//! * a triple term is [`TRIPLE_TERM_OPEN`], its subject, predicate and
//!   object separated by single spaces, and [`TRIPLE_TERM_CLOSE`], with one
//!   space inside each delimiter: `<<( s p o )>>`.
//!
//! There are no knobs. A writer that needs a different spelling is writing a
//! different syntax (a CURIE-compacting Turtle writer, an XML carrier), and
//! composes these pieces with its own rather than asking for a variant.

use crate::literal_escape::{self, Carrier};
use crate::text_out::TextOut;

/// The token that opens an RDF 1.2 triple term (N-Triples 1.2 `[6]`
/// `tripleTerm ::= '<<(' subject predicate object ')>>'`).
pub const TRIPLE_TERM_OPEN: &str = "<<(";

/// The token that closes an RDF 1.2 triple term.
pub const TRIPLE_TERM_CLOSE: &str = ")>>";

/// `xsd:string`, the datatype the canonical form leaves unwritten (XML Schema
/// 1.1 Part 2 §3.3.1, the RDF 1.2 simple-literal datatype). Spelled here
/// because this crate sits below `purrdf-xsd` and `purrdf-iri`, whose
/// vocabulary constants name it for every crate above.
pub(crate) const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Append `iri` as an `IRIREF`: `<`, the body escaped by
/// [`crate::iri_escape::push_escaped`], `>`.
///
/// ```
/// use purrdf_lex::term_syntax::write_iri;
///
/// let mut out = String::new();
/// write_iri("http://example.org/a b", &mut out);
/// assert_eq!(out, "<http://example.org/a\\u0020b>");
/// ```
pub fn write_iri<W: TextOut + ?Sized>(iri: &str, out: &mut W) {
    out.push('<');
    crate::iri_escape::push_escaped(iri, out);
    out.push('>');
}

/// Append a literal in its canonical form.
///
/// `lang` is the language tag and `dir` the RDF 1.2 base direction token
/// (`ltr` or `rtl`), written `@lang--dir`. When `lang` is present the literal
/// is `rdf:langString` or `rdf:dirLangString` by its tag, so `datatype` is not
/// written. A base direction exists only on a language-tagged literal
/// (RDF 1.2 Concepts §3.3), so `dir` is read only beside a `lang`; a
/// direction without a tag is not an RDF 1.2 literal and is a caller bug,
/// caught in debug builds. With no tag, an `xsd:string` literal is written
/// bare and every other datatype as `^^` and its `IRIREF`.
///
/// ```
/// use purrdf_lex::term_syntax::write_literal;
///
/// let xsd = |local: &str| format!("http://www.w3.org/2001/XMLSchema#{local}");
/// let mut out = String::new();
/// write_literal("a", &xsd("string"), None, None, &mut out);
/// write_literal("1", &xsd("integer"), None, None, &mut out);
/// write_literal("chat", "ignored", Some("fr"), None, &mut out);
/// write_literal("x", "ignored", Some("ar"), Some("rtl"), &mut out);
/// assert_eq!(
///     out,
///     "\"a\"\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>\"chat\"@fr\"x\"@ar--rtl"
/// );
/// ```
pub fn write_literal<W: TextOut + ?Sized>(
    lexical_form: &str,
    datatype: &str,
    lang: Option<&str>,
    dir: Option<&str>,
    out: &mut W,
) {
    debug_assert!(
        dir.is_none() || lang.is_some(),
        "a base direction exists only on a language-tagged literal"
    );
    out.push('"');
    literal_escape::write(lexical_form, Carrier::Canonical, out);
    out.push('"');
    if let Some(lang) = lang {
        out.push('@');
        out.push_str(lang);
        if let Some(dir) = dir {
            out.push_str("--");
            out.push_str(dir);
        }
    } else if datatype != XSD_STRING {
        out.push_str("^^");
        write_iri(datatype, out);
    }
}

/// Append `iri` as the `IRIREF` of a canonical N-Quads or N-Triples **line**:
/// `<`, the body escaped by [`crate::iri_escape::push_escaped_for`] with the
/// [`Canonical`](crate::iri_escape::Carrier::Canonical) carrier, `>`.
///
/// It escapes exactly the scalars the `IRIREF` production excludes. [`write_iri`]
/// is unchanged and also escapes DEL and the C1 block, for the writers whose text
/// can be carried in XML.
///
/// ```
/// use purrdf_lex::term_syntax::write_canonical_iri;
///
/// let mut out = String::new();
/// write_canonical_iri("urn:ex:a b\u{7f}", &mut out);
/// assert_eq!(out, "<urn:ex:a\\u0020b\u{7f}>");
/// ```
pub fn write_canonical_iri<W: TextOut + ?Sized>(iri: &str, out: &mut W) {
    out.push('<');
    crate::iri_escape::push_escaped_for(iri, crate::iri_escape::Carrier::Canonical, out);
    out.push('>');
}

/// Append a literal as it is spelled in a canonical N-Quads or N-Triples
/// **line** (RDF 1.2 N-Quads section 3, RDF 1.2 N-Triples section 3).
///
/// [`write_literal`] with three rules of the canonical form added, and used by
/// the canonical line writer only: the lexical form goes through
/// [`literal_escape::write_canonical_line`], so U+FFFE and U+FFFF are `UCHAR`;
/// the language tag and the base direction are written in ASCII lower case
/// ("Alphabetic characters in LANG_DIR MUST use only the lowercase letters");
/// and a datatype is written by [`write_canonical_iri`]. A tag that is already
/// in lower case is written as it is.
///
/// ```
/// use purrdf_lex::term_syntax::write_canonical_literal;
///
/// let mut out = String::new();
/// write_canonical_literal("y", "ignored", Some("EN-us"), Some("LTR"), &mut out);
/// write_canonical_literal("a\u{fffe}", "http://www.w3.org/2001/XMLSchema#string", None, None, &mut out);
/// assert_eq!(out, "\"y\"@en-us--ltr\"a\\uFFFE\"");
/// ```
pub fn write_canonical_literal<W: TextOut + ?Sized>(
    lexical_form: &str,
    datatype: &str,
    lang: Option<&str>,
    dir: Option<&str>,
    out: &mut W,
) {
    debug_assert!(
        dir.is_none() || lang.is_some(),
        "a base direction exists only on a language-tagged literal"
    );
    out.push('"');
    literal_escape::write_canonical_line(lexical_form, out);
    out.push('"');
    match lang {
        Some(lang) => {
            out.push('@');
            push_ascii_lower(lang, out);
            if let Some(dir) = dir {
                out.push_str("--");
                push_ascii_lower(dir, out);
            }
        }
        None if datatype != XSD_STRING => {
            out.push_str("^^");
            write_canonical_iri(datatype, out);
        }
        None => {}
    }
}

/// Append `text` with every ASCII letter in lower case, whole when it holds no
/// upper-case letter.
fn push_ascii_lower<W: TextOut + ?Sized>(text: &str, out: &mut W) {
    let Some(first) = text.bytes().position(|b| b.is_ascii_uppercase()) else {
        out.push_str(text);
        return;
    };
    out.push_str(&text[..first]);
    for c in text[first..].chars() {
        out.push(c.to_ascii_lowercase());
    }
}

/// Append a blank node: `_:` and `label`.
///
/// The label is written as given; making a label legal under the
/// `BLANK_NODE_LABEL` production (and injective over blank-node scopes) is
/// the job of the caller's label encoder.
///
/// ```
/// use purrdf_lex::term_syntax::write_blank;
///
/// let mut out = String::new();
/// write_blank("b0", &mut out);
/// assert_eq!(out, "_:b0");
/// ```
pub fn write_blank<W: TextOut + ?Sized>(label: &str, out: &mut W) {
    out.push_str("_:");
    out.push_str(label);
}

#[cfg(test)]
mod tests {
    use super::{
        TRIPLE_TERM_CLOSE, TRIPLE_TERM_OPEN, write_blank, write_canonical_iri,
        write_canonical_literal, write_iri, write_literal,
    };

    fn canonical_literal(lex: &str, dt: &str, lang: Option<&str>, dir: Option<&str>) -> String {
        let mut out = String::new();
        write_canonical_literal(lex, dt, lang, dir, &mut out);
        out
    }

    /// The canonical line lowers a tag and a direction (N-Quads 1.2 section 3:
    /// "Alphabetic characters in LANG_DIR MUST use only the lowercase letters");
    /// the shared writer every serializer uses writes them as given.
    #[test]
    fn a_canonical_line_lowers_tag_and_direction_and_the_shared_writer_does_not() {
        assert_eq!(
            canonical_literal("hi", "unused", Some("EN-us"), Some("LTR")),
            "\"hi\"@en-us--ltr"
        );
        assert_eq!(
            canonical_literal("hi", "unused", Some("zh-Hant-TW"), None),
            "\"hi\"@zh-hant-tw"
        );
        assert_eq!(
            canonical_literal("hi", "unused", Some("en-us"), None),
            "\"hi\"@en-us"
        );
        assert_eq!(
            literal("hi", "unused", Some("EN-us"), Some("LTR")),
            "\"hi\"@EN-us--LTR"
        );
    }

    /// The canonical line writes U+FFFE and U+FFFF as `UCHAR` (a character outside
    /// XML 1.1 `Char`); the shared writer keeps both raw.
    #[test]
    fn a_canonical_line_escapes_the_noncharacters_and_the_shared_writer_does_not() {
        let string = format!("{XSD}string");
        assert_eq!(
            canonical_literal("a\u{fffe}\u{ffff}b", &string, None, None),
            "\"a\\uFFFE\\uFFFFb\""
        );
        assert_eq!(
            literal("a\u{fffe}\u{ffff}b", &string, None, None),
            "\"a\u{fffe}\u{ffff}b\""
        );
    }

    /// The canonical line escapes in an IRI exactly what `IRIREF` excludes; the
    /// shared writer also escapes DEL and the C1 block, for XML transport. A
    /// datatype IRI follows the writer it is under.
    #[test]
    fn a_canonical_line_keeps_del_and_c1_raw_in_an_iri_and_the_shared_writer_escapes_them() {
        let iri = "urn:ex:a\u{7f}\u{85}\u{a0}b c";
        let mut canonical = String::new();
        write_canonical_iri(iri, &mut canonical);
        assert_eq!(canonical, "<urn:ex:a\u{7f}\u{85}\u{a0}b\\u0020c>");
        let mut shared = String::new();
        write_iri(iri, &mut shared);
        assert_eq!(shared, "<urn:ex:a\\u007F\\u0085\u{a0}b\\u0020c>");
        assert_eq!(
            canonical_literal("1", "urn:ex:\u{7f}", None, None),
            "\"1\"^^<urn:ex:\u{7f}>"
        );
        assert_eq!(
            literal("1", "urn:ex:\u{7f}", None, None),
            "\"1\"^^<urn:ex:\\u007F>"
        );
    }

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    fn literal(lex: &str, dt: &str, lang: Option<&str>, dir: Option<&str>) -> String {
        let mut out = String::new();
        write_literal(lex, dt, lang, dir, &mut out);
        out
    }

    #[test]
    fn xsd_string_is_elided_and_every_other_datatype_is_kept() {
        assert_eq!(literal("a", &format!("{XSD}string"), None, None), "\"a\"");
        assert_eq!(
            literal("1", &format!("{XSD}integer"), None, None),
            "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
        // A near miss of the xsd:string IRI is a different datatype.
        assert_eq!(
            literal("a", &format!("{XSD}String"), None, None),
            "\"a\"^^<http://www.w3.org/2001/XMLSchema#String>"
        );
    }

    #[test]
    fn a_datatype_iri_is_escaped_like_any_iri() {
        assert_eq!(
            literal("v", "http://example.org/d t>", None, None),
            "\"v\"^^<http://example.org/d\\u0020t\\u003E>"
        );
        assert_eq!(
            literal("v", "http://example.org/dt", None, None),
            "\"v\"^^<http://example.org/dt>"
        );
    }

    #[test]
    fn language_and_direction_are_written_and_the_datatype_is_implied() {
        assert_eq!(literal("hi", "unused", Some("en"), None), "\"hi\"@en");
        assert_eq!(
            literal("hi", "unused", Some("en"), Some("ltr")),
            "\"hi\"@en--ltr"
        );
    }

    #[test]
    fn the_lexical_form_uses_the_canonical_carrier() {
        assert_eq!(
            literal("a\"b\\\u{8}\u{85}", &format!("{XSD}string"), None, None),
            "\"a\\\"b\\\\\\b\u{85}\""
        );
    }

    #[test]
    fn iris_blanks_and_the_triple_term_delimiters() {
        let mut out = String::new();
        out.push_str(TRIPLE_TERM_OPEN);
        out.push(' ');
        write_iri("http://example.org/s", &mut out);
        out.push(' ');
        write_iri("http://example.org/p", &mut out);
        out.push(' ');
        write_blank("o", &mut out);
        out.push(' ');
        out.push_str(TRIPLE_TERM_CLOSE);
        assert_eq!(
            out,
            "<<( <http://example.org/s> <http://example.org/p> _:o )>>"
        );
    }
}
