// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The emitted seam's term codec: one escape table, read in both directions.
//!
//! [`sparql_term`] renders a [`TermValue`] as a SPARQL constant, and
//! [`decode_term`] reads a caller's canonical term lexical back into a
//! [`TermValue`]. They are inverses over the same table, and they live together
//! for exactly that reason: a decoder that accepted an escape the renderer does
//! not write (or the reverse) would let a seed round-trip into text that names a
//! different term.
//!
//! # Why the table is reproduced here rather than imported
//!
//! The kernel's canonical N-Quads writer owns the authoritative escape set, and
//! its literal half is private. Reproducing it — rather than approximating it —
//! is what makes an emitted constant and a canonicalized dataset agree character
//! for character: `"a\nb"` written here is byte-identical to `"a\nb"` written
//! there, so a needle that matches in one matches in the other. The IRI half is
//! not reproduced at all: it is
//! [`purrdf_core::iri_escape::is_iriref_escape_required`], read directly.
//!
//! # No serializer is pulled in
//!
//! This layer mints no vocabulary and parses no RDF documents; it needs exactly
//! one term's worth of text in each direction, over an escape table the kernel
//! already fixes. A full serializer or parser dependency would buy nothing here
//! and would put a document-level codec inside the composition layer.
//!
//! # A blank node has no constant form
//!
//! [`RenderError::BlankNotGround`] is not squeamishness about blank nodes. In a
//! property-function argument position a blank node is a **non-distinguished
//! variable** — the evaluator's own `term_is_bound` reads it as free — so
//! emitting one would silently unbind a position that
//! [`place`](crate::matching::place) just proved bound, and the invocation would
//! be admitted against a mode it does not actually have.

use core::fmt::Write as _;

use purrdf_core::iri_escape::is_iriref_escape_required;
use purrdf_core::{RdfTextDirection, TermValue};

/// `xsd:string`, the datatype a plain literal carries in the kernel's term
/// model. Spelled here because the kernel's own constant is crate-private; it is
/// the RDF specification's IRI, not vocabulary this layer mints.
pub(crate) const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// `rdf:langString`, the datatype a language-tagged literal carries.
pub(crate) const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// `rdf:dirLangString`, the datatype a directional language-tagged literal
/// carries (RDF 1.2).
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

/// A term value that has no SPARQL constant form.
///
/// Every variant is a refusal to write text that would mean something other than
/// the value it was handed. None of them is a policy choice.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RenderError {
    /// A blank node in an argument position is a non-distinguished variable, so
    /// it is free rather than ground and cannot be written as a constant.
    #[error(
        "blank node _:{label} is a non-distinguished variable in a property-function \
         argument position, so it has no ground constant form"
    )]
    BlankNotGround {
        /// The blank node's label, without the `_:` prefix.
        label: String,
    },

    /// A base direction without a language tag is not expressible: the concrete
    /// syntax writes the direction as a suffix of the tag.
    #[error(
        "literal {lexical_form:?} carries a base direction with no language tag, \
         which no concrete syntax can spell"
    )]
    DirectionWithoutLanguage {
        /// The literal's lexical form.
        lexical_form: String,
    },

    /// A language tag that is not a `LANGTAG` cannot be written after `@`
    /// without changing what the text parses as.
    #[error("language tag {tag:?} is not a well-formed LANGTAG")]
    MalformedLanguageTag {
        /// The rejected tag.
        tag: String,
    },
}

/// Render `value` as a SPARQL constant.
///
/// The output is a pure function of the value: no locale, no float formatting,
/// no hash iteration and no case folding is involved. A language tag is written
/// exactly as the value carries it — tags are normalized where terms are built,
/// and re-casing one here would make the emitted text depend on this function
/// rather than on the plan.
///
/// # The plain-string spelling is pinned
///
/// A literal whose datatype is `xsd:string` with no language tag is written as
/// the short form `"lexical"`, never `"lexical"^^<…#string>`. Both parse to the
/// same term, so one must be chosen for the emitted text to be a pure function
/// of the plan; the short form is that choice.
pub(crate) fn sparql_term(value: &TermValue) -> Result<String, RenderError> {
    let mut out = String::with_capacity(lexical_size_hint(value));
    write_term(value, &mut out)?;
    Ok(out)
}

/// The exact byte length of `value`'s canonical lexical when nothing in it needs
/// escaping, which is the overwhelmingly common case.
///
/// This is a capacity hint, not a bound: a lexical form carrying `"` or a
/// control character, or an IRI carrying a character the `IRIREF` production
/// forbids, renders longer and the string grows on its own. What the hint buys
/// is the repeated doubling that building a term one character at a time from an
/// empty `String` otherwise pays — a 60-byte IRI costs four allocations without
/// it and one with it, on every row a stratum emits.
fn lexical_size_hint(value: &TermValue) -> usize {
    match value {
        // `<` + text + `>`.
        TermValue::Iri(iri) => iri.len() + 2,
        // `_:` + label.
        TermValue::Blank { label, .. } => label.len() + 2,
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            // `"` + lexical + `"`.
            let mut size = lexical_form.len() + 2;
            if let Some(tag) = language {
                // `@` + tag, then `--` + `ltr`/`rtl`.
                size += tag.len() + 1;
                if direction.is_some() {
                    size += 5;
                }
            } else if datatype != XSD_STRING {
                // `^^` + `<` + datatype + `>`.
                size += datatype.len() + 4;
            }
            size
        }
        // `<<( ` + s + ` ` + p + ` ` + o + ` )>>`.
        TermValue::Triple { s, p, o } => {
            10 + lexical_size_hint(s) + lexical_size_hint(p) + lexical_size_hint(o)
        }
    }
}

/// Write `value` as the canonical term lexical naming a **result**.
///
/// This is [`sparql_term`]'s reading for the other direction of travel, and the
/// two differ in exactly one term kind. A blank node in an argument position is
/// a non-distinguished variable — free, not ground — so [`sparql_term`] refuses
/// it, because emitting one would silently unbind a position the caller proved
/// bound. Naming a row the evaluator already produced carries no such hazard: a
/// blank node is an ordinary answer, and `_:label` is its ordinary spelling.
///
/// Refusing it here instead would discard every other row in the same stratum
/// over one answer the layer simply declined to write down — and the label is
/// not lost, because [`decode_term`] reads `_:label` back. What a blank node
/// genuinely cannot do is seed a *later* request, since its label is
/// dataset-local; that is refused where it happens, at placement, rather than
/// pre-emptively here.
pub(crate) fn candidate_lexical(value: &TermValue) -> String {
    // Sized up front: this runs once per row every stratum emits.
    let mut out = String::with_capacity(lexical_size_hint(value));
    if let TermValue::Blank { label, .. } = value {
        out.push_str("_:");
        out.push_str(label);
        return out;
    }
    // Every non-blank arm is infallible, and a blank nested inside a triple term
    // is written by the same rule rather than refused.
    if write_term(value, &mut out).is_err() {
        out.clear();
        write_candidate(value, &mut out);
    }
    out
}

/// Append `value`'s result-naming form to `out`, spelling blank nodes.
fn write_candidate(value: &TermValue, out: &mut String) {
    match value {
        TermValue::Blank { label, .. } => {
            out.push_str("_:");
            out.push_str(label);
        }
        TermValue::Triple { s, p, o } => {
            out.push_str("<<( ");
            write_candidate(s, out);
            out.push(' ');
            write_candidate(p, out);
            out.push(' ');
            write_candidate(o, out);
            out.push_str(" )>>");
        }
        other => {
            // Infallible for IRIs and literals: `write_term` only refuses blanks.
            let _ = write_term(other, out);
        }
    }
}

/// Append `value`'s SPARQL constant form to `out`.
fn write_term(value: &TermValue, out: &mut String) -> Result<(), RenderError> {
    match value {
        TermValue::Iri(iri) => {
            write_iri(iri, out);
            Ok(())
        }
        TermValue::Blank { label, .. } => Err(RenderError::BlankNotGround {
            label: label.clone(),
        }),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => write_literal(lexical_form, datatype, language.as_deref(), *direction, out),
        TermValue::Triple { s, p, o } => {
            // RDF 1.2 triple term. `<<( s p o )>>` is the value form the SPARQL
            // parser reads as a term (`<< s p o >>` is a *reifying* triple, which
            // emits its own triples and is not a value).
            out.push_str("<<( ");
            write_term(s, out)?;
            out.push(' ');
            write_term(p, out)?;
            out.push(' ');
            write_term(o, out)?;
            out.push_str(" )>>");
            Ok(())
        }
    }
}

/// Append `iri`'s `<…>` form, escaping exactly what the `IRIREF` production
/// forbids.
fn write_iri(iri: &str, out: &mut String) {
    out.push('<');
    for ch in iri.chars() {
        if is_iriref_escape_required(ch) {
            write_u_escape(ch, out);
        } else {
            out.push(ch);
        }
    }
    out.push('>');
}

/// Append a literal's `"…"` form with its tag, direction or datatype.
fn write_literal(
    lexical_form: &str,
    datatype: &str,
    language: Option<&str>,
    direction: Option<RdfTextDirection>,
    out: &mut String,
) -> Result<(), RenderError> {
    if language.is_none() && direction.is_some() {
        return Err(RenderError::DirectionWithoutLanguage {
            lexical_form: lexical_form.to_owned(),
        });
    }
    out.push('"');
    write_literal_escaped(lexical_form, out);
    out.push('"');
    if let Some(tag) = language {
        if !is_langtag(tag) {
            return Err(RenderError::MalformedLanguageTag {
                tag: tag.to_owned(),
            });
        }
        out.push('@');
        out.push_str(tag);
        if let Some(direction) = direction {
            out.push_str("--");
            out.push_str(direction.as_str());
        }
        return Ok(());
    }
    // The short form for a plain string; every other datatype is written out.
    if datatype != XSD_STRING {
        out.push_str("^^");
        write_iri(datatype, out);
    }
    Ok(())
}

/// Append `value` escaped for a `"…"` string, matching the canonical N-Quads
/// `ECHAR` set; other C0 controls and `U+007F` become `\uXXXX`, and everything
/// else (including all non-ASCII) rides verbatim as UTF-8.
fn write_literal_escaped(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => write_u_escape(c, out),
            c => out.push(c),
        }
    }
}

/// Write `\uXXXX`, or `\UXXXXXXXX` beyond the BMP, in upper-case hex.
fn write_u_escape(ch: char, out: &mut String) {
    let code_point = ch as u32;
    if code_point <= 0xFFFF {
        let _ = write!(out, "\\u{code_point:04X}");
    } else {
        let _ = write!(out, "\\U{code_point:08X}");
    }
}

/// Whether `tag` is a `LANGTAG` body: `[a-zA-Z]+ ('-' [a-zA-Z0-9]+)*`.
///
/// The base direction is carried separately in a [`TermValue`], so it is not
/// part of what this accepts; [`write_literal`] appends it after the tag.
fn is_langtag(tag: &str) -> bool {
    let mut subtags = tag.split('-');
    let Some(primary) = subtags.next() else {
        return false;
    };
    if primary.is_empty() || !primary.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    subtags.all(|subtag| !subtag.is_empty() && subtag.bytes().all(|b| b.is_ascii_alphanumeric()))
}

// ---------------------------------------------------------------------------
// The decoding direction
// ---------------------------------------------------------------------------

/// Read a caller's canonical term lexical into a term value.
///
/// The composition layer carries a seed term as the text the caller's own
/// canonical term codec produced (`<iri>`, `_:label`, `"lexical"@tag`,
/// `"lexical"^^<datatype>`, or the RDF 1.2 triple term `<<( s p o )>>`).
/// Decoding it into a typed value — rather than splicing the caller's bytes into
/// the emitted query — is what keeps a seed from writing query syntax: every
/// character that reaches the output goes back through [`sparql_term`].
///
/// # Errors
///
/// A human-readable reason when the text is not one canonical term. The reason
/// is a diagnostic, not a parser position: this is a refusal to guess, and a
/// caller repairs its own codec rather than reading a column number.
pub(crate) fn decode_term(text: &str) -> Result<TermValue, String> {
    let mut cursor = Cursor::new(text);
    let value = cursor.term()?;
    cursor.skip_whitespace();
    if !cursor.at_end() {
        return Err(format!(
            "trailing text after the term at byte {}",
            cursor.position
        ));
    }
    Ok(value)
}

/// A byte cursor over one canonical term lexical.
struct Cursor<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Cursor<'a> {
    /// A cursor at the start of `text`.
    const fn new(text: &'a str) -> Self {
        Self { text, position: 0 }
    }

    /// The text the cursor has not consumed.
    ///
    /// Slicing at `position` is sound because the cursor only ever advances by
    /// whole `char` widths or by the byte length of a matched ASCII prefix, so
    /// it never lands inside a multi-byte sequence.
    fn rest(&self) -> &'a str {
        &self.text[self.position..]
    }

    /// Whether every byte has been consumed.
    fn at_end(&self) -> bool {
        self.position >= self.text.len()
    }

    /// Advance past ASCII whitespace.
    ///
    /// ASCII only, deliberately. The lexicals this scanner reads are the ones
    /// [`candidate_lexical`] writes, whose separators are all ASCII, so treating
    /// a Unicode space as a separator would accept a spelling this layer never
    /// emits and cannot round-trip.
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.rest().chars().next() {
            if ch.is_ascii_whitespace() {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    /// Consume `prefix` if it is next, reporting whether it was.
    ///
    /// The cursor is left untouched when the prefix does not match, so a caller
    /// can try alternatives in order without saving and restoring a position.
    fn eat(&mut self, prefix: &str) -> bool {
        if self.rest().starts_with(prefix) {
            self.position += prefix.len();
            true
        } else {
            false
        }
    }

    /// Decode one term at the cursor.
    fn term(&mut self) -> Result<TermValue, String> {
        self.skip_whitespace();
        if self.eat("<<(") {
            return self.triple_term();
        }
        if self.rest().starts_with("<<") {
            return Err(
                "a reifying triple `<< s p o >>` is not a term value; a triple term \
                 is spelled `<<( s p o )>>`"
                    .to_owned(),
            );
        }
        if self.rest().starts_with('<') {
            return self.iri().map(TermValue::Iri);
        }
        if self.eat("_:") {
            return Ok(TermValue::blank(self.blank_label()));
        }
        if self.rest().starts_with('"') {
            return self.literal();
        }
        Err(format!(
            "expected a canonical term (`<iri>`, `_:label`, `\"literal\"` or \
             `<<( s p o )>>`) at byte {}",
            self.position
        ))
    }

    /// Decode `<<( s p o )>>`, the opening delimiter already consumed.
    fn triple_term(&mut self) -> Result<TermValue, String> {
        let subject = self.term()?;
        let predicate = self.term()?;
        let object = self.term()?;
        self.skip_whitespace();
        if !self.eat(")>>") {
            return Err(format!(
                "unterminated triple term: expected `)>>` at byte {}",
                self.position
            ));
        }
        Ok(TermValue::Triple {
            s: Box::new(subject),
            p: Box::new(predicate),
            o: Box::new(object),
        })
    }

    /// Decode `<…>`, resolving `UCHAR` escapes.
    fn iri(&mut self) -> Result<String, String> {
        self.position += 1;
        let mut out = String::new();
        loop {
            let Some(ch) = self.rest().chars().next() else {
                return Err("unterminated IRI: no closing `>`".to_owned());
            };
            if ch == '>' {
                self.position += 1;
                return Ok(out);
            }
            if ch == '\\' {
                out.push(self.uchar()?);
                continue;
            }
            if ch == '<' {
                return Err(format!(
                    "unescaped `<` inside an IRI at byte {}",
                    self.position
                ));
            }
            self.position += ch.len_utf8();
            out.push(ch);
        }
    }

    /// Read a blank-node label: everything up to whitespace or a closing
    /// delimiter.
    fn blank_label(&mut self) -> String {
        let start = self.position;
        while let Some(ch) = self.rest().chars().next() {
            if ch.is_ascii_whitespace() || ch == ')' || ch == '>' {
                break;
            }
            self.position += ch.len_utf8();
        }
        self.text[start..self.position].to_owned()
    }

    /// Decode `"…"` with its optional tag, direction or datatype.
    fn literal(&mut self) -> Result<TermValue, String> {
        self.position += 1;
        let mut lexical_form = String::new();
        loop {
            let Some(ch) = self.rest().chars().next() else {
                return Err("unterminated literal: no closing `\"`".to_owned());
            };
            if ch == '"' {
                self.position += 1;
                break;
            }
            if ch == '\\' {
                lexical_form.push(self.echar()?);
                continue;
            }
            self.position += ch.len_utf8();
            lexical_form.push(ch);
        }
        if self.eat("^^") {
            let datatype = self.iri()?;
            return Ok(TermValue::Literal {
                lexical_form,
                datatype,
                language: None,
                direction: None,
            });
        }
        if self.eat("@") {
            let (language, direction) = self.language_tag()?;
            let datatype = if direction.is_some() {
                RDF_DIR_LANG_STRING
            } else {
                RDF_LANG_STRING
            };
            return Ok(TermValue::Literal {
                lexical_form,
                datatype: datatype.to_owned(),
                language: Some(language),
                direction,
            });
        }
        Ok(TermValue::Literal {
            lexical_form,
            datatype: XSD_STRING.to_owned(),
            language: None,
            direction: None,
        })
    }

    /// Read a language tag and its optional `--ltr` / `--rtl` base direction.
    fn language_tag(&mut self) -> Result<(String, Option<RdfTextDirection>), String> {
        let start = self.position;
        while let Some(ch) = self.rest().chars().next() {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }
        let raw = &self.text[start..self.position];
        let (tag, direction) = match raw.rsplit_once("--") {
            Some((tag, "ltr")) => (tag, Some(RdfTextDirection::Ltr)),
            Some((tag, "rtl")) => (tag, Some(RdfTextDirection::Rtl)),
            Some((_, other)) => {
                return Err(format!(
                    "invalid base direction `--{other}`: it must be exactly `ltr` or `rtl`"
                ));
            }
            None => (raw, None),
        };
        if !is_langtag(tag) {
            return Err(format!("{tag:?} is not a well-formed LANGTAG"));
        }
        Ok((tag.to_owned(), direction))
    }

    /// Resolve one `\…` escape inside a literal.
    fn echar(&mut self) -> Result<char, String> {
        let escape = self.rest().as_bytes().get(1).copied();
        let simple = match escape {
            Some(b't') => Some('\t'),
            Some(b'b') => Some('\u{08}'),
            Some(b'n') => Some('\n'),
            Some(b'r') => Some('\r'),
            Some(b'f') => Some('\u{0c}'),
            Some(b'"') => Some('"'),
            Some(b'\'') => Some('\''),
            Some(b'\\') => Some('\\'),
            _ => None,
        };
        if let Some(resolved) = simple {
            self.position += 2;
            return Ok(resolved);
        }
        self.uchar()
    }

    /// Resolve one `\uXXXX` / `\UXXXXXXXX` escape.
    fn uchar(&mut self) -> Result<char, String> {
        let width = match self.rest().as_bytes().get(1).copied() {
            Some(b'u') => 4,
            Some(b'U') => 8,
            _ => {
                return Err(format!(
                    "unrecognized escape at byte {}: only \\t \\b \\n \\r \\f \\\" \\' \
                     \\\\ \\uXXXX and \\UXXXXXXXX are canonical",
                    self.position
                ));
            }
        };
        let digits = self
            .rest()
            .get(2..2 + width)
            .ok_or_else(|| format!("truncated escape at byte {}", self.position))?;
        let code_point = u32::from_str_radix(digits, 16)
            .map_err(|_| format!("`{digits}` is not hexadecimal at byte {}", self.position))?;
        let resolved = char::from_u32(code_point)
            .ok_or_else(|| format!("`{digits}` is not a Unicode scalar value"))?;
        self.position += 2 + width;
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderError, decode_term, sparql_term};
    use purrdf_core::{RdfTextDirection, TermValue};

    fn rendered(value: &TermValue) -> String {
        sparql_term(value).expect("the fixture value renders")
    }

    #[test]
    fn plain_string_uses_the_short_form() {
        assert_eq!(rendered(&TermValue::simple_literal("fox")), "\"fox\"");
    }

    #[test]
    fn a_typed_literal_carries_its_datatype() {
        assert_eq!(
            rendered(&TermValue::typed_literal(
                "3",
                "http://www.w3.org/2001/XMLSchema#integer"
            )),
            "\"3\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn a_language_tag_is_written_verbatim() {
        assert_eq!(
            rendered(&TermValue::Literal {
                lexical_form: "fox".to_owned(),
                datatype: super::RDF_LANG_STRING.to_owned(),
                language: Some("en-GB".to_owned()),
                direction: None,
            }),
            "\"fox\"@en-GB"
        );
    }

    #[test]
    fn a_base_direction_rides_after_the_tag() {
        assert_eq!(
            rendered(&TermValue::Literal {
                lexical_form: "نص".to_owned(),
                datatype: super::RDF_DIR_LANG_STRING.to_owned(),
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            }),
            "\"نص\"@ar--rtl"
        );
    }

    #[test]
    fn a_direction_without_a_language_is_refused_but_with_one_renders() {
        let error = sparql_term(&TermValue::Literal {
            lexical_form: "x".to_owned(),
            datatype: super::RDF_DIR_LANG_STRING.to_owned(),
            language: None,
            direction: Some(RdfTextDirection::Ltr),
        })
        .expect_err("a direction with no tag has no spelling");
        assert!(matches!(
            error,
            RenderError::DirectionWithoutLanguage { .. }
        ));
        // The neighbouring valid case still succeeds.
        assert_eq!(
            rendered(&TermValue::Literal {
                lexical_form: "x".to_owned(),
                datatype: super::RDF_DIR_LANG_STRING.to_owned(),
                language: Some("he".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            }),
            "\"x\"@he--ltr"
        );
    }

    #[test]
    fn a_malformed_tag_is_refused_but_a_well_formed_one_renders() {
        let error = sparql_term(&TermValue::Literal {
            lexical_form: "x".to_owned(),
            datatype: super::RDF_LANG_STRING.to_owned(),
            language: Some("en\" . ?x <p> ?y # ".to_owned()),
            direction: None,
        })
        .expect_err("a tag that is not a LANGTAG cannot be written");
        assert!(matches!(error, RenderError::MalformedLanguageTag { .. }));
        assert_eq!(
            rendered(&TermValue::Literal {
                lexical_form: "x".to_owned(),
                datatype: super::RDF_LANG_STRING.to_owned(),
                language: Some("en".to_owned()),
                direction: None,
            }),
            "\"x\"@en"
        );
    }

    #[test]
    fn a_blank_node_has_no_constant_form_but_an_iri_does() {
        let error = sparql_term(&TermValue::blank("b0")).expect_err("a blank node is not ground");
        assert!(matches!(error, RenderError::BlankNotGround { .. }));
        assert_eq!(
            rendered(&TermValue::iri("http://example.org/s")),
            "<http://example.org/s>"
        );
    }

    #[test]
    fn control_characters_and_quotes_are_escaped() {
        assert_eq!(
            rendered(&TermValue::simple_literal("a\"b\\c\nd\u{1}e\u{7f}f")),
            "\"a\\\"b\\\\c\\nd\\u0001e\\u007Ff\""
        );
    }

    #[test]
    fn an_iri_escapes_only_what_the_production_forbids() {
        assert_eq!(
            rendered(&TermValue::iri("http://example.org/a b>c")),
            "<http://example.org/a\\u0020b\\u003Ec>"
        );
        // Non-ASCII is lawful in an IRIREF and rides verbatim.
        assert_eq!(
            rendered(&TermValue::iri("http://example.org/é")),
            "<http://example.org/é>"
        );
    }

    #[test]
    fn a_triple_term_renders_in_its_value_form() {
        let value = TermValue::Triple {
            s: Box::new(TermValue::iri("http://example.org/s")),
            p: Box::new(TermValue::iri("http://example.org/p")),
            o: Box::new(TermValue::simple_literal("o")),
        };
        assert_eq!(
            rendered(&value),
            "<<( <http://example.org/s> <http://example.org/p> \"o\" )>>"
        );
    }

    #[test]
    fn decoding_inverts_rendering() {
        for value in [
            TermValue::iri("http://example.org/s"),
            TermValue::simple_literal("quick brown fox"),
            TermValue::simple_literal("a\"b\\c\nd"),
            TermValue::typed_literal("3", "http://www.w3.org/2001/XMLSchema#integer"),
            TermValue::Literal {
                lexical_form: "fox".to_owned(),
                datatype: super::RDF_LANG_STRING.to_owned(),
                language: Some("en".to_owned()),
                direction: None,
            },
            TermValue::Literal {
                lexical_form: "نص".to_owned(),
                datatype: super::RDF_DIR_LANG_STRING.to_owned(),
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            },
            TermValue::Triple {
                s: Box::new(TermValue::iri("http://example.org/s")),
                p: Box::new(TermValue::iri("http://example.org/p")),
                o: Box::new(TermValue::simple_literal("o")),
            },
        ] {
            let text = rendered(&value);
            assert_eq!(decode_term(&text), Ok(value), "round trip of {text}");
        }
    }

    #[test]
    fn decoding_refuses_text_that_is_not_one_term() {
        for text in [
            "",
            "seed",
            "<unterminated",
            "\"unterminated",
            "<a> <b>",
            "<< <a> <b> <c> >>",
            "\"x\"@en--upside-down",
            "\"x\"\\q",
        ] {
            assert!(
                decode_term(text).is_err(),
                "{text:?} is not one canonical term"
            );
        }
        // The neighbouring valid cases still decode.
        for text in ["<http://example.org/s>", "_:b0", "\"x\"@en", "\"x\""] {
            assert!(decode_term(text).is_ok(), "{text:?} is one canonical term");
        }
    }
}
